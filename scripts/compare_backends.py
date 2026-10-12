#!/usr/bin/env python3
"""Compare opt-in Darwin batch measurement with the portable backend on owned fixtures."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import tempfile


RESOURCE_METRICS = ("cpu_total_ns", "average_cpu_cores", "peak_cpu_cores", "average_rss_bytes", "sampled_peak_rss_bytes")
METRICS = ("elapsed_ns", "first_artifact_ns", "cancellation_latency_ns", "listing_worker_ns", "measurement_worker_ns", "peak_rss_bytes") + RESOURCE_METRICS


def resource_metrics(data):
    """Fail explicitly on unavailable accounting, rather than reporting zero use."""
    resources = data["resources"]
    if resources.get("scope") != "process":
        raise ValueError("Resource accounting must be process-wide")
    keys = ("cpu_user_ns", "cpu_system_ns") + RESOURCE_METRICS[1:]
    for key in keys:
        value = resources.get(key)
        if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0:
            raise ValueError(f"Unavailable or invalid process resource metric: {key}")
    return {"cpu_total_ns": resources["cpu_user_ns"] + resources["cpu_system_ns"],
            **{key: resources[key] for key in RESOURCE_METRICS[1:]}}


def summary(values):
    if len(values) < 5 or any(value < 0 for value in values):
        raise ValueError("At least five nonnegative samples are required")
    median = statistics.median(values)
    return {"samples": values, "median": median,
            "p95": sorted(values)[math.ceil(len(values) * .95) - 1],
            "mad": statistics.median(abs(value - median) for value in values)}


def validate_pair(portable, bulk, *, cancelled=False, expected_artifacts=1):
    if portable["report"] != bulk["report"]:
        raise ValueError("Backend reports differ (paths, classifications, sizes, nesting, partial flags, warnings or status)")
    expected = "Cancelled" if cancelled else "Completed"
    if portable["status"] != expected or portable["warnings"]:
        raise ValueError("Fixture scan did not complete as expected")
    if portable["artifacts"] != expected_artifacts or not portable["first_artifact_ns"]:
        raise ValueError("Fixture artifacts must be discovered")
    if cancelled and not portable["cancellation_latency_ns"]:
        raise ValueError("Missing cancellation latency")
    if not cancelled and (not bulk["bulk_directories"] or bulk["bulk_fallbacks"]):
        raise ValueError("Batch fixture must exercise getattrlistbulk without portable fallbacks")


def run(command, **kwargs):
    return subprocess.run(command, check=True, text=True, **kwargs)


def write_files(path, count):
    path.mkdir(parents=True, exist_ok=True)
    for index in range(count):
        (path / f"object-{index}").write_bytes(b"fixture" * 128)


def prepare(root):
    write_files(root / "wide/.build", 8192)
    for index in range(128):
        write_files(root / f"deep/.build/module-{index}", 32)
    write_files(root / "mixed/Swift/.build", 64)
    write_files(root / "mixed/Rust/target/deps", 64)
    (root / "mixed/Rust/Cargo.toml").write_text("[package]\n")
    write_files(root / "mixed/Python/.venv/__pycache__", 64)
    (root / "mixed/Python/.venv/pyvenv.cfg").write_text("home = fixture\n")
    write_files(root / "mixed/.git", 64)
    build = root / "mixed/Swift/.build"
    os.link(build / "object-0", build / "hard-link")
    os.symlink("object-0", build / "symlink")
    with (build / "sparse").open("wb") as file:
        file.truncate(32 * 1024 * 1024)
    with (build / "object-0/..namedfork/rsrc").open("wb") as file:
        file.write(b"resource" * 16_384)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output-dir", type=Path, default=Path("target/performance/bulk"))
    parser.add_argument("--samples", type=int, default=7)
    args = parser.parse_args()
    if platform.system() != "Darwin":
        parser.error("This experimental backend comparison requires macOS")
    if args.samples < 5:
        parser.error("--samples must be at least 5")
    repo = Path(__file__).resolve().parents[1]
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    binaries = {}
    for variant in ("portable", "bulk"):
        target = output / variant
        command = ["cargo", "build", "--locked", "--release", "--example", "profile_tree", "--target-dir", str(target)]
        if variant == "bulk":
            command += ["--features", "macos-bulk-metadata"]
        run(command, cwd=repo)
        binaries[variant] = target / "release/examples/profile_tree"
    scenarios = [(name, mode) for name in ("wide", "deep", "mixed") for mode in ("allocated", "apparent")]
    scenarios.append(("cancel", "allocated"))
    samples = {f"{name}/{mode}": {variant: [] for variant in binaries} for name, mode in scenarios}
    with tempfile.TemporaryDirectory(prefix="buildhunter-backends-") as temporary:
        fixture = Path(temporary)
        prepare(fixture)

        def sample(variant, name, mode):
            command = ["/usr/bin/time", "-l", str(binaries[variant]), str(fixture / ("wide" if name == "cancel" else name)), "--report", "--environments"]
            if mode == "apparent":
                command.append("--apparent")
            if name == "cancel":
                command.append("--cancel-on-first")
            process = run(command, capture_output=True)
            data = json.loads(process.stdout)
            match = re.search(r"(\d+)\s+maximum resident set size", process.stderr)
            if not match:
                raise ValueError("Missing macOS peak RSS measurement")
            data["peak_rss_bytes"] = int(match[1])
            data.update(resource_metrics(data))
            return data

        for name, mode in scenarios:
            validate_pair(*(sample(variant, name, mode) for variant in binaries), cancelled=name == "cancel", expected_artifacts=4 if name == "mixed" else 1)
        for index in range(args.samples):
            for name, mode in scenarios:
                pair = {}
                for variant in (("portable", "bulk") if index % 2 == 0 else ("bulk", "portable")):
                    pair[variant] = sample(variant, name, mode)
                    samples[f"{name}/{mode}"][variant].append(pair[variant])
                validate_pair(pair["portable"], pair["bulk"], cancelled=name == "cancel", expected_artifacts=4 if name == "mixed" else 1)

    summaries = {scenario: {variant: {metric: summary([sample[metric] for sample in values])
                                      for metric in METRICS} for variant, values in variants.items()}
                 for scenario, variants in samples.items()}
    # Keep exact fixture reports without repeating them in every raw timing sample.
    reports = {scenario: variants["portable"][0]["report"] for scenario, variants in samples.items()}
    for variants in samples.values():
        for values in variants.values():
            for value in values:
                value.pop("report")
    report = {"sha": run(["git", "rev-parse", "HEAD"], cwd=repo, capture_output=True).stdout.strip(),
              "dirty": bool(run(["git", "status", "--porcelain"], cwd=repo, capture_output=True).stdout),
              "host": platform.platform(), "architecture": platform.machine(),
              "rustc": run(["rustc", "--version"], capture_output=True).stdout.strip(),
              "harness_sha256": hashlib.sha256((repo / "examples/profile_tree.rs").read_bytes()).hexdigest(),
              "cache": "warm, shared owned fixtures, interleaved backends", "reports_equal": True,
              "samples": samples, "summaries": summaries, "reports": reports}
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    lines = ["| Scenario | Portable ms | Bulk ms | Bulk / portable | Portable peak MiB | Bulk peak MiB |", "|---|---:|---:|---:|---:|---:|"]
    for scenario, variants in summaries.items():
        old, new = variants["portable"], variants["bulk"]
        lines.append(f"| {scenario} | {old['elapsed_ns']['median']/1e6:.3f} | {new['elapsed_ns']['median']/1e6:.3f} | {new['elapsed_ns']['median']/old['elapsed_ns']['median']:.2f} | {old['peak_rss_bytes']['median']/2**20:.2f} | {new['peak_rss_bytes']['median']/2**20:.2f} |")
    lines += ["", "| Scenario | Portable CPU ms | Bulk CPU ms | Portable mean CPU | Bulk mean CPU | Portable mean RSS MiB | Bulk mean RSS MiB |",
              "|---|---:|---:|---:|---:|---:|---:|"]
    for scenario, variants in summaries.items():
        old, new = variants["portable"], variants["bulk"]
        lines.append(f"| {scenario} | {old['cpu_total_ns']['median']/1e6:.3f} | {new['cpu_total_ns']['median']/1e6:.3f} | {old['average_cpu_cores']['median']*100:.0f}% | {new['average_cpu_cores']['median']*100:.0f}% | {old['average_rss_bytes']['median']/2**20:.2f} | {new['average_rss_bytes']['median']/2**20:.2f} |")
    lines += ["", "All paired reports match exactly. Experimental feature is disabled by default.",
              "Timing ratios are diagnostic; this job gates report equality and real batch usage.",
              "Raw samples include CPU/RSS timelines, sampled peaks, first discovery, cancellation-to-return latency and aggregate worker phase times.",
              "CPU 100% is one logical core. RSS is process-wide; /usr/bin/time peak includes startup/serialization, unlike the scan's sampled RSS."]
    markdown = "\n".join(lines) + "\n"
    (output / "report.md").write_text(markdown)
    print(markdown)


if __name__ == "__main__":
    main()
