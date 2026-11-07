#!/usr/bin/env python3
"""Validate specifications and reject new or repeated diagnostics."""
import collections
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASELINE = ROOT / "specs/specpm-warning-baseline.json"
KEYS = ("code", "file", "field", "message")


def fingerprint(warning):
    return tuple(warning.get(key, "") for key in KEYS)


def check(report, baseline):
    if not isinstance(report, dict) or not isinstance(baseline, list):
        return ["Invalid SpecPM report or baseline"]
    if not isinstance(report.get("errors"), list) or not isinstance(report.get("warnings"), list):
        return ["Invalid SpecPM report: missing errors or warnings"]
    if not all(isinstance(item, dict) for item in report["errors"] + report["warnings"] + baseline):
        return ["Invalid SpecPM diagnostic"]
    if report["errors"] or report.get("error_count") != 0:
        return ["SpecPM reported errors or an invalid error count"]
    if report.get("warning_count") != len(report["warnings"]):
        return ["Invalid SpecPM warning count"]
    unexpected = collections.Counter(map(fingerprint, report["warnings"])) - collections.Counter(
        map(fingerprint, baseline)
    )
    return [f"Unexpected SpecPM warning ({count}): {item}" for item, count in unexpected.items()]


def main():
    if len(sys.argv) != 2:
        print("Usage: check_specpm.py REGISTRY_URL", file=sys.stderr)
        return 2
    try:
        result = subprocess.run(
            ["specpm", "validate", str(ROOT), "--registry", sys.argv[1], "--json"],
            capture_output=True, text=True, check=False,
        )
        sys.stdout.write(result.stdout)
        sys.stderr.write(result.stderr)
        if result.returncode:
            return result.returncode
        report = json.loads(result.stdout)
        baseline = json.loads(BASELINE.read_text(encoding="utf-8"))
        problems = check(report, baseline)
    except (OSError, ValueError, TypeError) as error:
        print(f"SpecPM check failed: {error}", file=sys.stderr)
        return 1
    for problem in problems:
        print(problem, file=sys.stderr)
    return int(bool(problems))


if __name__ == "__main__":
    sys.exit(main())
