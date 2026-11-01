#!/usr/bin/env python3
"""Run the pinned String Catalog Doctor and reject unconfigured catalogs."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

DOCTOR_REVISION = "2c50f14ea08edc1281e1883c72f1b1b4f1f2a77c"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, required=True)
    parser.add_argument("--doctor", type=Path, required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    doctor = args.doctor.resolve()
    try:
        revision = subprocess.check_output(["git", "-C", str(doctor), "rev-parse", "HEAD"], text=True).strip()
        if revision != DOCTOR_REVISION:
            raise ValueError(f"doctor must be at {DOCTOR_REVISION}, got {revision}")
        if subprocess.run(["git", "-C", str(doctor), "diff", "--quiet", "HEAD", "--", "string_catalog_doctor"], check=False).returncode:
            raise ValueError("doctor source has uncommitted changes")
        config = json.loads((root / ".string-catalog-doctor.json").read_text())
        configured = {item["path"] for item in config["catalogs"]}
        paths = subprocess.check_output(["git", "-C", str(root), "ls-files", "-z", "--cached", "--others", "--exclude-standard"])
        found = {os.fsdecode(path) for path in paths.split(b"\0") if path.endswith(b".xcstrings")}
        missing = found - configured
        if missing:
            raise ValueError("unconfigured catalogs: " + ", ".join(sorted(missing)))
        result = subprocess.run([sys.executable, str(doctor / "string_catalog_doctor"), "--root", str(root)], check=False)
        return result.returncode
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        print(f"string catalog gate: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
