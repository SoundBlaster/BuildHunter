#!/usr/bin/env python3
"""Process CPU/RSS traces and paired profiling overhead on an owned, warm fixture."""
import argparse
import hashlib
import json
from pathlib import Path
import platform
import subprocess
import tempfile

from compare_backends import resource_metrics, summary


def run(command, **kwargs):
    return subprocess.run(command, check=True, text=True, timeout=120, **kwargs)


def validate_pair(enabled, disabled):
    if enabled["report"] != disabled["report"] or enabled["status"] != "Completed" or enabled["warnings"]:
        raise ValueError("Profiling must preserve the complete artifact report")
    if disabled["resource_samples"] or disabled["resources"]["cpu_user_ns"] is not None:
        raise ValueError("Disabled profiling must not emit resource readings")
    resource_metrics(enabled)
    if len(enabled["resource_samples"]) < 2:
        raise ValueError("Initial and terminal resource readings are required")
    for previous, current in zip(enabled["resource_samples"], enabled["resource_samples"][1:]):
        if current["elapsed_us"] <= previous["elapsed_us"]:
            raise ValueError("Resource timestamps must increase")
        for key in ("cpu_user_ns", "cpu_system_ns"):
            if current[key] is None or previous[key] is None or current[key] < previous[key]:
                raise ValueError("Process CPU accounting must be available and monotonic")
        if current["resident_bytes"] is None or current["resident_bytes"] <= 0:
            raise ValueError("Process RSS must be available")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=Path("target/performance/resources"))
    parser.add_argument("--samples", type=int, default=7)
    args = parser.parse_args()
    if args.samples < 5:
        parser.error("At least five paired samples are required")
    repo = Path(__file__).resolve().parents[1]
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    run(["cargo", "build", "--locked", "--release", "--example", "profile_tree"], cwd=repo)
    binary = repo / "target/release/examples/profile_tree"
    pairs = []
    with tempfile.TemporaryDirectory(prefix="buildhunter-resources-") as directory:
        root = Path(directory)
        for module in range(128):
            build = root / ".build" / f"module-{module}"
            build.mkdir(parents=True)
            for index in range(1024):
                (build / f"object-{index}").write_bytes(b"fixture" * 128)

        def sample(enabled):
            command = [str(binary), str(root), "--report"]
            if not enabled:
                command.append("--no-profile")
            return json.loads(run(command, cwd=repo, capture_output=True).stdout)

        validate_pair(sample(True), sample(False))  # Warm both paths before measuring.
        for index in range(args.samples):
            pair = {}
            for enabled in ((True, False) if index % 2 == 0 else (False, True)):
                pair["enabled" if enabled else "disabled"] = sample(enabled)
            validate_pair(pair["enabled"], pair["disabled"])
            pairs.append(pair)
    elapsed = {key: summary([pair[key]["elapsed_ns"] for pair in pairs]) for key in ("enabled", "disabled")}
    overhead = summary([pair["enabled"]["elapsed_ns"] / pair["disabled"]["elapsed_ns"] * 100
                        for pair in pairs])
    # summary() accepts nonnegative measurements; store ratios as a percentage of baseline.
    resources = {key: summary([resource_metrics(pair["enabled"])[key] for pair in pairs])
                 for key in resource_metrics(pairs[0]["enabled"])}
    for pair in pairs:
        for data in pair.values():
            data.pop("report")
    report = {"sha": run(["git", "rev-parse", "HEAD"], cwd=repo, capture_output=True).stdout.strip(),
              "dirty": bool(run(["git", "status", "--porcelain"], cwd=repo, capture_output=True).stdout),
              "host": platform.platform(), "architecture": platform.machine(),
              "rustc": run(["rustc", "--version"], capture_output=True).stdout.strip(),
              "harness_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
              "fixture": "128 folders, 131072 files, 896 bytes/file, warm cache, alternating order",
              "reports_equal": True, "samples": pairs, "elapsed_ns": elapsed,
              "enabled_percent_of_disabled": overhead, "resources": resources}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    text = (f"Profiling on/off (same binary, {args.samples} warm alternating pairs, all reports equal): "
            f"{elapsed['enabled']['median']/1e6:.2f}/{elapsed['disabled']['median']/1e6:.2f} ms. "
            f"Median paired overhead {overhead['median']-100:+.1f}% (MAD {overhead['mad']:.1f} percentage points).\n\n"
            f"Process CPU time {resources['cpu_total_ns']['median']/1e6:.2f} ms; "
            f"mean CPU {resources['average_cpu_cores']['median']*100:.0f}% (100% = one core). "
            f"Mean RSS {resources['average_rss_bytes']['median']/2**20:.2f} MiB; "
            f"sampled peak RSS {resources['sampled_peak_rss_bytes']['median']/2**20:.2f} MiB.\n\n"
            "Per-run CPU/RSS timelines are in report.json. RSS sampling can miss short spikes. "
            "These are diagnostics on this host, not portable thresholds or battery/energy measurements.\n")
    (output / "report.md").write_text(text)
    print(text)


if __name__ == "__main__":
    main()
