#!/usr/bin/env python3
"""Build, run real UI checkpoints, and collect a native Screener/system PNG batch."""
import argparse
import fcntl
import hashlib
import html
import json
import os
import plistlib
import shutil
import signal
import struct
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png_info(path):
    header = path.read_bytes()[:29]
    if header[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("Not a PNG: " + str(path))
    return list(struct.unpack(">II", header[16:24])), header[24]


def confined(root, relative):
    target = (root / relative).resolve()
    if not target.is_relative_to(root.resolve()):
        raise ValueError("Artifact path escaped its root")
    return target


class Runner:
    def __init__(self, root, prefix, output, metrics_name="metrics.json"):
        self.root, self.prefix, self.output = root, prefix, output
        self.operations = []
        self.metrics_name = metrics_name

    def run(self, *args, output=False, log=None, timeout=300):
        command = self.prefix + [str(a) for a in args]
        print("+ " + " ".join(command), flush=True)
        started = time.monotonic()
        stream = (self.output / log).open("w") if log else None
        try:
            with subprocess.Popen(command, cwd=self.root, text=True, start_new_session=True,
                                  stdout=stream if stream else subprocess.PIPE if output else None,
                                  stderr=subprocess.STDOUT if stream else None) as process:
                try:
                    stdout, _ = process.communicate(timeout=timeout)
                except (subprocess.TimeoutExpired, KeyboardInterrupt):
                    os.killpg(process.pid, signal.SIGTERM)
                    try:
                        process.communicate(timeout=10)
                    except subprocess.TimeoutExpired:
                        os.killpg(process.pid, signal.SIGKILL)
                        process.communicate()
                    raise
                if process.returncode:
                    raise subprocess.CalledProcessError(process.returncode, command)
                return stdout if output else None
        finally:
            if stream:
                stream.close()
            self.operations.append({"command": [str(a) for a in args], "seconds": round(time.monotonic() - started, 3)})
            write_json(self.output / self.metrics_name, {"operations": self.operations})


def test_targets(value):
    if isinstance(value, dict):
        if "TestBundlePath" in value:
            yield value
        for child in value.values():
            yield from test_targets(child)
    elif isinstance(value, list):
        for child in value:
            yield from test_targets(child)


def read_timeline(trace):
    rows = [json.loads(line) for line in (trace / "timeline.jsonl").read_text().splitlines()]
    if not rows or rows[-1]["kind"] != "sessionEnded" or any(r["sequence"] != i for i, r in enumerate(rows)):
        raise ValueError("Trace is incomplete or reordered: " + str(trace))
    return rows


def collect(cache, previous, out, locale, config):
    runs = [p for p in cache.iterdir() if p.is_dir() and p.name not in previous]
    if len(runs) != len(config["orientations"]):
        raise ValueError("Unexpected number of screenshot sessions")
    found, assets = set(), []
    names = config["checkpoints"]
    for source in runs:
        if not (source / "stopped.json").exists() or (source / "failure.json").exists():
            raise ValueError("Session did not stop successfully")
        folder = out / "traces" / locale / source.name
        shutil.copytree(source, folder)
        traces = list(folder.glob("*.vtrace"))
        if len(traces) != 1:
            raise ValueError("Expected one trace bundle")
        trace = traces[0]
        rows = read_timeline(trace)
        orientation = next(r["metadata"]["orientation"] for r in rows if r["name"] == "screenshots.ready")
        if orientation not in config["orientations"] or orientation in found:
            raise ValueError("Unexpected or repeated orientation")
        found.add(orientation)
        expected = (config["nativeWindowPixels"] if config.get("platform") == "macOS" else
                    config["nativePortraitPixels"] if orientation == "portrait" else config["nativePortraitPixels"][::-1])
        frames = [r for r in rows if r["kind"] == "keyframe"]
        if [r["name"] for r in frames] != names:
            raise ValueError("Missing, duplicate or reordered checkpoints")
        if {p.stem for p in (folder / "system").glob("*.png")} != set(names):
            raise ValueError("System reference set differs from the checkpoint set")
        for frame in frames:
            checkpoint = next(r for r in rows if r["name"] == "screenshots.checkpoint" and r["metadata"]["name"] == frame["name"])
            scale = float(frame["metadata"]["scale"])
            points = [float(checkpoint["metadata"][key]) for key in ["pointWidth", "pointHeight"]]
            if [round(p * scale) for p in points] != expected:
                raise ValueError("Window point geometry times scale differs from native pixels")
            for backend, blob in [("screener", confined(trace, frame["blob"])), ("system", folder / "system" / (frame["name"] + ".png"))]:
                destination = out / "raw" / backend / locale / orientation / (frame["name"] + ".png")
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copyfile(blob, destination)
                encoded, depth = png_info(destination)
                if encoded != expected and not (config.get("platform") != "macOS" and backend == "system" and encoded == expected[::-1]):
                    raise ValueError("Non-native PNG dimensions")
                assets.append({"backend": backend, "locale": locale, "orientation": orientation, "screen": frame["name"],
                               "source": str(destination.relative_to(out)), "pixels": expected, "encodedPixels": encoded,
                               "bitDepth": depth, "scale": scale, "rawSHA256": digest(destination),
                               "trace": str(trace.relative_to(out)), "recordID": frame["id"]})
    return assets


def prepare(out, runner):
    manifest = json.loads((out / "manifest.json").read_text())
    if not manifest.get("collectionComplete"):
        raise ValueError("Collection is not complete; rerun the scenario")
    config = manifest["configuration"]
    expected = {(b, l, o, n) for b in ["screener", "system"] for l in config["locales"]
                for o in config["orientations"] for n in config["checkpoints"]}
    actual = [(a["backend"], a["locale"], a["orientation"], a["screen"]) for a in manifest["assets"]]
    if len(actual) != len(expected) or set(actual) != expected:
        raise ValueError("Incomplete or duplicate batch")
    for asset in manifest["assets"]:
        source = confined(out / "raw", str(Path(asset["source"]).relative_to("raw")))
        if digest(source) != asset["rawSHA256"]:
            raise ValueError("Raw capture changed since collection")
    for trace in {a["trace"] for a in manifest["assets"]}:
        read_timeline(confined(out, trace))
    if (out / "failure.json").exists():
        shutil.copyfile(out / "failure.json", out / ("previous-failure-" + str(time.time_ns()) + ".json"))
    manifest["state"] = "preparing"
    write_json(out / "manifest.json", manifest)
    preparations = [{"source": str(out / a["source"]), "output": str(out / a["source"].replace("raw/", "png/", 1)), "pixels": a["pixels"]}
                    for a in manifest["assets"]]
    write_json(out / "prepare.json", preparations)
    runner.run("xcrun", "swift", HERE / "prepare.swift", out / "prepare.json", log="prepare.log", timeout=600)
    for asset, item in zip(manifest["assets"], preparations):
        path = Path(item["output"])
        asset["png"], asset["sha256"] = str(path.relative_to(out)), digest(path)
        if png_info(path) != (asset["pixels"], asset["bitDepth"]):
            raise ValueError("Preparation changed dimensions or bit depth")
    cards = [f'<a href="{html.escape(a["png"], quote=True)}"><img src="{html.escape(a["png"], quote=True)}"><span>'
             + html.escape(" · ".join(a[k] for k in ["backend", "locale", "orientation", "screen"])) + '</span></a>' for a in manifest["assets"]]
    (out / "review.html").write_text('<!doctype html><meta charset="utf-8"><title>Screenshot batch</title>'
        '<style>body{font:14px system-ui;background:#eee}main{display:grid;grid-template-columns:repeat(4,1fr);gap:16px}'
        'img{width:100%;height:280px;object-fit:contain}span{display:block}</style><h1>Screener / system references</h1>'
        '<p>View-hierarchy and system captures can differ for glass and chrome. Review system PNGs for App Store use.</p><main>' + ''.join(cards) + '</main>')
    manifest["state"], manifest["preparedAt"] = "complete", datetime.now(timezone.utc).isoformat()
    write_json(out / "manifest.json", manifest)
    (out / "failure.json").unlink(missing_ok=True)
    print("Ready: " + str(out / "review.html"), flush=True)


def capture(config, args, root, out, runner):
    device_list = json.loads(runner.run("xcrun", "simctl", "list", "devices", "available", "--json", output=True))["devices"]
    available = device_list.get(config["runtime"], [])
    registry = root / ".build/screener/simulators.json"
    owned = json.loads(registry.read_text()) if registry.exists() else {}
    key = config["deviceType"] + ":" + config["runtime"]
    udid = args.simulator or owned.get(key)
    if not args.simulator and not any(d["udid"] == udid for d in available):
        udid = None  # A deleted owned device is a cache miss, not an explicit selection.
    if not udid:
        if shutil.disk_usage(root).free < 5 * 1024**3:
            raise ValueError("Need 5 GiB free to create a capture simulator; pass an existing --simulator")
        udid = runner.run("xcrun", "simctl", "create", "Screener screenshots", config["deviceType"], config["runtime"], output=True).strip()
        owned[key] = udid
        write_json(registry, owned)
        available.append({"udid": udid, "deviceTypeIdentifier": config["deviceType"], "state": "Shutdown"})
    device = next((d for d in available if d["udid"] == udid and d["deviceTypeIdentifier"] == config["deviceType"]), None)
    if not device:
        raise ValueError("Selected simulator does not match the configured runtime/device type")
    derived = root / config["derivedData"]
    if not args.skip_build:
        if config.get("generate"):
            runner.run(*config["generate"])
        runner.run("xcodebuild", "build-for-testing", "-project", config["project"], "-scheme", config["scheme"],
                   "-configuration", config["buildConfiguration"], "-destination", f"platform=iOS Simulator,id={udid}",
                   "-derivedDataPath", derived, "-clonedSourcePackagesDirPath", derived / "SourcePackages",
                   "CODE_SIGNING_ALLOWED=NO", log="build.log", timeout=1800)
    products = derived / "Build/Products"
    app = products / (config["buildConfiguration"] + "-iphonesimulator") / config["appProduct"]
    info = plistlib.loads((app / "Info.plist").read_bytes())
    if info["CFBundleIdentifier"] != config["bundleID"]:
        raise ValueError("Wrong app bundle; use an isolated screenshot build")
    candidates = []
    for path in products.glob("*.xctestrun"):
        if path.name.startswith("Screener-"):
            continue
        plan = plistlib.loads(path.read_bytes())
        if config["buildConfiguration"] + "-iphonesimulator" in str(plan):
            candidates.append((path, plan))
    if len(candidates) != 1:
        raise ValueError("Expected one matching xctestrun plan")
    plan_path, base = candidates[0]
    manifest = {"configuration": config, "simulator": udid, "state": "capturing", "collectionComplete": False,
                "executableSHA256": digest(app / info["CFBundleExecutable"]), "assets": [],
                "gitHead": runner.run("git", "rev-parse", "HEAD", output=True).strip(),
                "gitStatus": runner.run("git", "status", "--porcelain", output=True),
                "toolSHA256": {name: digest(HERE / name) for name in ["run.py", "prepare.swift"]}}
    resolved = root / config["project"] / "project.xcworkspace/xcshareddata/swiftpm/Package.resolved"
    if resolved.exists():
        manifest["packageResolution"] = json.loads(resolved.read_text())
        manifest["packageResolutionSHA256"] = digest(resolved)
    write_json(out / "manifest.json", manifest)
    was_booted = device["state"] == "Booted"
    if not was_booted:
        runner.run("xcrun", "simctl", "boot", udid)
    try:
        runner.run("xcrun", "simctl", "bootstatus", udid, "-b")
        runner.run("xcrun", "simctl", "ui", udid, "appearance", config.get("appearance", "light"))
        runner.run("xcrun", "simctl", "status_bar", udid, "override", "--time", "9:41", "--batteryState", "discharging", "--batteryLevel", "100")
        runner.run("xcrun", "simctl", "install", udid, app)
        def container():
            return Path(runner.run("xcrun", "simctl", "get_app_container", udid, config["bundleID"], "data", output=True).strip())
        data = container()
        for locale in config["locales"]:
            cache = data / "Library/Caches/Screener/Screenshots"
            previous = {p.name for p in cache.iterdir()} if cache.exists() else set()
            plan = plistlib.loads(plistlib.dumps(base))
            for target in test_targets(plan):
                target.setdefault("TestingEnvironmentVariables", {}).update({
                    "SCREENER_TEST": "1", "SCREENER_CONTAINERS_ROOT": str(data.parent), "SCREENER_BUNDLE_ID": config["bundleID"],
                    "SCREENER_ORIENTATIONS": ",".join(config["orientations"]), "SCREENER_LANGUAGE": locale,
                    "SCREENER_LOCALE": locale.replace("-", "_"),
                    "SCREENER_APP_ARGUMENTS": json.dumps(config.get("launchArguments", [])),
                    "SCREENER_APP_ENVIRONMENT": json.dumps(config.get("launchEnvironment", {}))})
            generated = plan_path.parent / ("Screener-" + locale + ".xctestrun")
            generated.write_bytes(plistlib.dumps(plan))
            try:
                runner.run("xcodebuild", "test-without-building", "-xctestrun", generated,
                           "-destination", f"platform=iOS Simulator,id={udid}", "-parallel-testing-enabled", "NO",
                           "-collect-test-diagnostics", "never", "-test-timeouts-enabled", "YES",
                           "-maximum-test-execution-time-allowance", "300", "-only-testing:" + config["testTarget"] + "/" + config["testClass"],
                           "-resultBundlePath", out / (locale + ".xcresult"), log=locale + ".log", timeout=420)
            finally:
                data = container()
                cache = data / "Library/Caches/Screener/Screenshots"
                if cache.exists():
                    for partial in cache.iterdir():
                        if partial.is_dir() and partial.name not in previous:
                            shutil.copytree(partial, out / "diagnostics" / locale / partial.name, dirs_exist_ok=True)
            manifest["assets"] += collect(cache, previous, out, locale, config)
            write_json(out / "manifest.json", manifest)
        manifest["collectionComplete"] = True
        write_json(out / "manifest.json", manifest)
        prepare(out, runner)
    finally:
        if not was_booted:
            runner.run("xcrun", "simctl", "shutdown", udid)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--simulator", help="Explicit existing simulator UDID; never erased")
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--prepare-output", type=Path)
    args = parser.parse_args()
    if args.prepare_output:
        out = args.prepare_output.resolve()
        config = json.loads((out / "manifest.json").read_text())["configuration"]
        root = Path(config["resolvedRoot"])
    else:
        if not args.config:
            parser.error("--config is required for capture")
        config_path = args.config.resolve()
        config = json.loads(config_path.read_text())
        root = (config_path.parent / config.get("projectRoot", "..")).resolve()
        config["resolvedRoot"] = str(root)
        for key in ["locales", "orientations", "checkpoints"]:
            values = config[key]
            if not values or len(values) != len(set(values)) or any(not isinstance(v, str) or not v or '/' in v or '..' in v for v in values):
                parser.error("Choose unique safe names for " + key)
        if any(o not in ["portrait", "landscapeLeft"] for o in config["orientations"]):
            parser.error("This adapter supports portrait and landscapeLeft")
        if len(config["checkpoints"]) > 16:
            parser.error("Harness accepts at most 16 checkpoints per orientation")
        if shutil.disk_usage(root).free < config.get("minimumFreeGiB", 3) * 1024**3:
            parser.error("Insufficient disk headroom; preserve artifacts and inspect rebuildable caches")
        out = root / config.get("output", "screenshots/output") / datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%fZ")
        out.mkdir(parents=True)
    runner = Runner(root, config.get("commandPrefix", []), out,
                    "preparation-metrics.json" if args.prepare_output else "metrics.json")
    lock_path = root / ".build/screener-build.lock"
    lock_path.parent.mkdir(parents=True, exist_ok=True)
    started = time.monotonic()
    try:
        with lock_path.open("w") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
            if not args.prepare_output:
                if subprocess.run(config.get("commandPrefix", []) + ["pgrep", "-x", "xcodebuild"], stdout=subprocess.DEVNULL).returncode == 0:
                    raise ValueError("An Xcode build is active; serialize the shared DerivedData")
                capture(config, args, root, out, runner)
            else:
                prepare(out, runner)
    except Exception as error:
        write_json(out / "failure.json", {"state": "failed", "error": str(error)})
        raise SystemExit("Screenshot batch failed: " + str(error) + "; evidence: " + str(out)) from error
    finally:
        write_json(out / ("preparation-metrics.json" if args.prepare_output else "metrics.json"),
                   {"elapsedSeconds": round(time.monotonic() - started, 3), "operations": runner.operations})


if __name__ == "__main__":
    main()
