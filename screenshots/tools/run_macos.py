#!/usr/bin/env python3
"""Run native macOS window checkpoint batches with paired AppKit/XCTest captures."""
import argparse
import fcntl
import json
import plistlib
import shutil
import subprocess
import time
from datetime import datetime, timezone
from pathlib import Path
from run import Runner, collect, digest, prepare, test_targets, write_json

HERE = Path(__file__).resolve().parent


def matches_plan(plan, config):
    return any(t.get("BlueprintName") == config["testTarget"]
               and "/" + config["buildConfiguration"] + "/" in t.get("TestHostPath", "")
               for t in test_targets(plan))


def validate(config):
    if config.get("platform") != "macOS" or config.get("orientations") != ["window"]:
        raise ValueError('macOS requires platform macOS and orientations ["window"]')
    for key in ["locales", "checkpoints"]:
        values = config[key]
        if not values or len(values) != len(set(values)) or any(not isinstance(v, str) or not v or '/' in v or '..' in v for v in values):
            raise ValueError("Choose unique safe names for " + key)
    if len(config["checkpoints"]) > 16:
        raise ValueError("Harness accepts at most 16 checkpoints")
    for key in ["windowPoints", "nativeWindowPixels"]:
        value = config[key]
        if len(value) != 2 or any(type(v) is not int or v <= 0 for v in value):
            raise ValueError("Choose two positive integers for " + key)
    scales = [p / q for p, q in zip(config["nativeWindowPixels"], config["windowPoints"])]
    if scales[0] != scales[1] or scales[0] not in [1, 2]:
        raise ValueError("Choose native 1x or 2x dimensions; never resample")
    if config.get("appearance", "light") not in ["light", "dark"]:
        raise ValueError("Choose light or dark appearance")


def capture(config, args, root, out, runner):
    derived = (root / config["derivedData"]).resolve()
    if not args.skip_build:
        if config.get("generate"):
            runner.run(*config["generate"])
        # Xcode copies and modifies a signed runner app. Disabling signing leaves
        # its resource seal invalid; local ad hoc signing needs no team/certificate.
        runner.run("xcodebuild", "build-for-testing", "-project", config["project"], "-scheme", config["scheme"],
                   "-configuration", config["buildConfiguration"], "-destination", "platform=macOS,arch=arm64",
                   "-derivedDataPath", derived, "-clonedSourcePackagesDirPath", derived / "SourcePackages",
                   "CODE_SIGNING_ALLOWED=YES", "CODE_SIGN_IDENTITY=-", "DEVELOPMENT_TEAM=",
                   "SCREENER_OUTPUT_ROOT=" + str(out.parent),
                   "SCREENER_APP_BUNDLE_ID=" + config["bundleID"],
                   "ONLY_ACTIVE_ARCH=YES", log="build.log", timeout=1800)
    products = derived / "Build/Products"
    app = products / config["buildConfiguration"] / config["appProduct"]
    info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
    if info["CFBundleIdentifier"] != config["bundleID"]:
        raise ValueError("Wrong app bundle; use an isolated screenshot configuration")
    plans = []
    for path in products.glob("*.xctestrun"):
        if path.name.startswith("Screener-"):
            continue
        plan = plistlib.loads(path.read_bytes())
        if matches_plan(plan, config):
            plans.append((path, plan))
    if len(plans) != 1:
        raise ValueError("Expected one matching xctestrun plan")
    plan_path, base = plans[0]
    manifest = {"configuration": config, "platform": "macOS", "captureArea": "window-frame", "state": "capturing",
                "collectionComplete": False, "assets": [],
                "executableSHA256": digest(app / "Contents/MacOS" / info["CFBundleExecutable"]),
                "gitHead": runner.run("git", "rev-parse", "HEAD", output=True).strip(),
                "gitStatus": runner.run("git", "status", "--porcelain", output=True),
                "host": runner.run("sw_vers", output=True),
                "toolSHA256": {name: digest(HERE / name) for name in ["run.py", "run_macos.py", "prepare.swift"]}}
    resolved = root / config["project"] / "project.xcworkspace/xcshareddata/swiftpm/Package.resolved"
    if resolved.exists():
        manifest["packageResolution"] = json.loads(resolved.read_text())
        manifest["packageResolutionSHA256"] = digest(resolved)
    write_json(out / "manifest.json", manifest)
    for locale in config["locales"]:
        exported = out / "collected" / locale
        exported.mkdir(parents=True)
        plan = plistlib.loads(plistlib.dumps(base))
        for target in test_targets(plan):
            target.setdefault("TestingEnvironmentVariables", {}).update({
                "SCREENER_TEST": "1", "SCREENER_EXPORT_ROOT": str(exported),
                "SCREENER_LANGUAGE": locale, "SCREENER_LOCALE": locale.replace("-", "_"),
                "SCREENER_WINDOW_POINTS": json.dumps(config["windowPoints"]),
                "SCREENER_APPEARANCE": config.get("appearance", "light"),
                "SCREENER_APP_ARGUMENTS": json.dumps(config.get("launchArguments", [])),
                "SCREENER_APP_ENVIRONMENT": json.dumps(config.get("launchEnvironment", {}))})
        generated = plan_path.parent / ("Screener-macOS-" + locale + ".xctestrun")
        generated.write_bytes(plistlib.dumps(plan))
        runner.run("xcodebuild", "test-without-building", "-xctestrun", generated,
                   "-destination", "platform=macOS,arch=arm64", "-parallel-testing-enabled", "NO",
                   "-collect-test-diagnostics", "never", "-test-timeouts-enabled", "YES",
                   "-maximum-test-execution-time-allowance", "240",
                   "-only-testing:" + config["testTarget"] + "/" + config["testClass"],
                   "-resultBundlePath", out / (locale + ".xcresult"), log=locale + ".log", timeout=360)
        summary = json.loads(runner.run("xcrun", "xcresulttool", "get", "test-results", "summary",
                            "--path", out / (locale + ".xcresult"), "--format", "json", output=True))
        write_json(out / (locale + "-test-summary.json"), summary)
        if summary.get("result") != "Passed" or summary.get("failedTests", 0) or not summary.get("passedTests", 0) or summary.get("skippedTests", 0):
            raise ValueError("Expected executed, passing screenshot tests")
        manifest["assets"] += collect(exported, set(), out, locale, config)
        write_json(out / "manifest.json", manifest)
    manifest["collectionComplete"] = True
    write_json(out / "manifest.json", manifest)
    prepare(out, runner)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--config", type=Path)
    parser.add_argument("--skip-build", action="store_true")
    parser.add_argument("--prepare-output", type=Path)
    args = parser.parse_args()
    if args.prepare_output:
        out = args.prepare_output.resolve()
        config = json.loads((out / "manifest.json").read_text())["configuration"]
        root = Path(config["resolvedRoot"])
    else:
        if not args.config:
            parser.error("--config is required")
        config_path = args.config.resolve()
        config = json.loads(config_path.read_text())
        root = (config_path.parent / config.get("projectRoot", "..")).resolve()
        config["resolvedRoot"] = str(root)
        validate(config)
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
            if args.prepare_output:
                prepare(out, runner)
            else:
                if subprocess.run(config.get("commandPrefix", []) + ["pgrep", "-x", "xcodebuild"], stdout=subprocess.DEVNULL).returncode == 0:
                    raise ValueError("An Xcode build is active; serialize shared DerivedData")
                capture(config, args, root, out, runner)
    except Exception as error:
        write_json(out / "failure.json", {"state": "failed", "error": str(error)})
        raise SystemExit("macOS screenshot batch failed: " + str(error) + "; evidence: " + str(out)) from error
    finally:
        write_json(out / runner.metrics_name, {"elapsedSeconds": round(time.monotonic() - started, 3), "operations": runner.operations})


if __name__ == "__main__":
    main()
