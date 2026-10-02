#!/usr/bin/env python3
"""Compare the same release-mode benchmark against two scanner revisions on one host."""
import argparse
import csv
import hashlib
import io
import json
import math
import platform
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile

SCENARIOS = {"source-tree": 16, "many-roots": 4096, "pruned-git": 0, "cancel": 1}
METRICS = ("elapsed_ns", "first_artifact_ns", "cancellation_latency_ns")


def parse_sample(text):
    rows = {}
    for row in csv.DictReader(io.StringIO(text)):
        name = row.pop("scenario")
        if name not in SCENARIOS or name in rows:
            raise ValueError(f"Unexpected or duplicate scenario: {name}")
        values = {key: int(value) for key, value in row.items()}
        if set(values) != {*METRICS, "artifacts", "policy_calls"}:
            raise ValueError(f"Invalid metric schema: {name}")
        if any(value < 0 for value in values.values()) or values["elapsed_ns"] <= 0:
            raise ValueError(f"Invalid measurement: {name}")
        if values["artifacts"] != SCENARIOS[name]:
            raise ValueError(f"Incorrect artifact count: {name}")
        if values["artifacts"] and not values["first_artifact_ns"]:
            raise ValueError(f"Missing discovery latency: {name}")
        if name == "cancel" and not values["cancellation_latency_ns"]:
            raise ValueError("Missing cancellation latency")
        rows[name] = values
    if set(rows) != set(SCENARIOS):
        raise ValueError("Benchmark did not execute every scenario")
    return rows


def summarize(samples):
    if len(samples) < 5:
        raise ValueError("At least five samples are required")
    result = {}
    for scenario in SCENARIOS:
        result[scenario] = {}
        for metric in METRICS:
            values = [sample[scenario][metric] for sample in samples]
            median = statistics.median(values)
            result[scenario][metric] = {
                "samples": values,
                "median": median,
                "p95": sorted(values)[math.ceil(len(values) * 0.95) - 1],
                "mad": statistics.median(abs(value - median) for value in values),
            }
    return result


def compare(baseline, candidate, ratio=1.5, allowance_ns=5_000_000):
    results = []
    for scenario in SCENARIOS:
        for metric in METRICS:
            old, new = baseline[scenario][metric], candidate[scenario][metric]
            if old["median"] == new["median"] == 0:
                continue
            limit = old["median"] * ratio + allowance_ns + 6 * (old["mad"] + new["mad"])
            results.append({
                "scenario": scenario, "metric": metric,
                "baseline_median_ns": old["median"], "candidate_median_ns": new["median"],
                "limit_ns": limit, "regression": new["median"] > limit,
            })
    return results


def run(command, **kwargs):
    try:
        return subprocess.run(command, check=True, **kwargs)
    except subprocess.CalledProcessError as error:
        if error.stdout:
            print(error.stdout, file=sys.stderr)
        if error.stderr:
            print(error.stderr, file=sys.stderr)
        raise


def compile_driver(repo, output, harness):
    target = output / "build"
    run(["cargo", "build", "--locked", "--release", "--lib", "--target-dir", str(target)], cwd=repo)
    binary = output / ("scanner.exe" if platform.system() == "Windows" else "scanner")
    run([
        "rustc", "--edition=2024", "-O", str(harness),
        "--extern", f"build_hunter={target / 'release/libbuild_hunter.rlib'}",
        "-L", f"dependency={target / 'release/deps'}", "-o", str(binary),
    ])
    return binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline-ref", required=True)
    parser.add_argument("--output-dir", type=Path, default=Path("target/performance"))
    parser.add_argument("--samples", type=int, default=7)
    args = parser.parse_args()
    if args.samples < 5:
        parser.error("--samples must be at least 5")
    repo = Path(__file__).resolve().parents[1]
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    harness = repo / "benches/scanner.rs"
    baseline_sha = run(["git", "rev-parse", args.baseline_ref], cwd=repo, capture_output=True, text=True).stdout.strip()
    candidate_sha = run(["git", "rev-parse", "HEAD"], cwd=repo, capture_output=True, text=True).stdout.strip()
    with tempfile.TemporaryDirectory(prefix="buildhunter-performance-") as temporary:
        baseline_repo = Path(temporary) / "baseline"
        fixture = Path(temporary) / "fixture"
        run(["git", "worktree", "add", "--detach", str(baseline_repo), baseline_sha], cwd=repo)
        try:
            binaries = {
                "baseline": compile_driver(baseline_repo, output / "baseline", harness),
                "candidate": compile_driver(repo, output / "candidate", harness),
            }
            run([str(binaries["candidate"]), "--prepare", str(fixture)])
            def sample(variant):
                process = run([str(binaries[variant]), "--fixture", str(fixture)], capture_output=True, text=True)
                return parse_sample(process.stdout)
            # Both revisions warm the same fixture, then alternate execution order.
            for variant in binaries:
                sample(variant)
            samples = {variant: [] for variant in binaries}
            for index in range(args.samples):
                order = ["baseline", "candidate"] if index % 2 == 0 else ["candidate", "baseline"]
                for variant in order:
                    samples[variant].append(sample(variant))
        finally:
            run(["git", "worktree", "remove", str(baseline_repo)], cwd=repo)
    summaries = {variant: summarize(values) for variant, values in samples.items()}
    comparisons = compare(summaries["baseline"], summaries["candidate"])
    report = {
        "baseline_sha": baseline_sha, "candidate_sha": candidate_sha,
        "candidate_dirty": bool(run(["git", "status", "--porcelain"], cwd=repo, capture_output=True, text=True).stdout),
        "harness_sha256": hashlib.sha256(harness.read_bytes()).hexdigest(),
        "host": {"system": platform.platform(), "architecture": platform.machine()},
        "rustc": run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "profile": "release", "cache": "warm, shared fixture, interleaved revisions",
        "threshold": {"ratio": 1.5, "allowance_ns": 5_000_000, "mad_multiplier": 6},
        "summaries": summaries, "comparisons": comparisons,
    }
    (output / "report.json").write_text(json.dumps(report, indent=2) + "\n")
    lines = ["| Scenario / metric | Base median (ms) | Candidate median (ms) | Result |", "|---|---:|---:|---|"]
    for result in comparisons:
        status = "REGRESSION" if result["regression"] else "PASS"
        lines.append(f"| {result['scenario']} / {result['metric']} | {result['baseline_median_ns'] / 1e6:.3f} | {result['candidate_median_ns'] / 1e6:.3f} | {status} |")
    markdown = "\n".join(lines) + "\n"
    (output / "report.md").write_text(markdown)
    print(markdown)
    if any(result["regression"] for result in comparisons):
        raise SystemExit("Performance regression exceeded the noise-adjusted budget; see report.json")


if __name__ == "__main__":
    main()
