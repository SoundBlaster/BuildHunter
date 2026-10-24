#!/usr/bin/env python3
"""One read-only CLI scan with kernel child CPU/peak-RSS and preserved raw outputs.

This is a manual/local measurement helper, not a new hosted CI job. On Unix wait4
measures only this scanner process (including its threads), not GUI/other windows.
A mutable home tree is observational, not a semantic-equivalence fixture.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import subprocess
import sys
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--timeout", type=float, default=300)
    args = parser.parse_args()
    if not hasattr(os, "wait4"):
        parser.error("wait4 is required for per-child resource accounting")
    if args.timeout <= 0:
        parser.error("timeout must be positive")
    binary = args.binary.resolve(strict=True)
    root = args.root.resolve(strict=True)
    output = args.output.resolve()
    if output == root or root in output.parents:
        parser.error("measurement outputs must be outside the scanned tree")
    output.mkdir(parents=True, exist_ok=False)
    profile = output / "profile.json"
    command = [str(binary), str(root), "--json", "--profile", str(profile)]
    binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
    started = time.perf_counter()
    timed_out = False
    with (output / "report.json").open("wb") as stdout, (output / "stderr.log").open("wb") as stderr:
        process = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)
        # Runtime-local wait: keep one tool session, enforce a bounded scan, and
        # collect exact child rusage. Popen.wait/poll would consume that evidence.
        while True:
            pid, status, usage = os.wait4(process.pid, os.WNOHANG)
            if pid:
                break
            if time.perf_counter() - started >= args.timeout:
                timed_out = True
                os.killpg(process.pid, signal.SIGTERM)
                _, status, usage = os.wait4(process.pid, 0)
                break
            time.sleep(0.05)
        process.returncode = os.waitstatus_to_exitcode(status)
    receipt = {"revision": args.revision, "binary_sha256": binary_sha256,
        "command": command, "root": str(root), "timed_out": timed_out, "exit_code": process.returncode,
        "wall_seconds": time.perf_counter()-started, "user_seconds":usage.ru_utime,
        "system_seconds":usage.ru_stime, "peak_rss_bytes":usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024),
        "first_finding_seconds":None, "latency_scope":"CLI total time; first-finding latency is not exposed by this CLI",
        "measurement_scope":"kernel wait4 CPU and peak RSS for one CLI process; not GUI footprint or an RSS guarantee"}
    if profile.exists():
        data=json.loads(profile.read_text())
        receipt.update(status=data["status"], profile_truncated_samples=data["truncated_samples"],
            scan_budgets=data.get("scan_budgets"), final_sample=data["samples"][-1] if data["samples"] else None)
        first = data.get("first_finding_elapsed_us")
        if first is not None:
            receipt["first_finding_seconds"] = first / 1_000_000
            receipt["latency_scope"] = "CLI total time; first finding uses CLI timer after preflight, including catalog setup"
    if not timed_out:
        report=json.loads((output/"report.json").read_text())
        rows=sorted((row["relative_path"],row["target_id"],row["bytes"],row["nested"],row["partial"]) for row in report["artifacts"])
        receipt.update(findings=len(rows),total_bytes=report["total_bytes"],warnings=len(report["errors"]),
            semantic_sha256=hashlib.sha256(json.dumps(rows,ensure_ascii=True,separators=(",",":")).encode()).hexdigest())
    (output/"receipt.json").write_text(json.dumps(receipt,indent=2)+"\n")
    print(json.dumps(receipt,indent=2),flush=True)
    return 124 if timed_out else 0


if __name__ == "__main__":
    raise SystemExit(main())
