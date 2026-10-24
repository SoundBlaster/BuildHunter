#!/usr/bin/env python3
"""Create new local R5 fixtures; never overwrite existing trees or run on CI automatically."""
import argparse
import json
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("output", type=Path)
args = parser.parse_args()
root = args.output.resolve()
root.mkdir(parents=True, exist_ok=False)
for scenario, directories, children in [("traversal", 200, 100), ("burst", 256, 256)]:
    scan = root / scenario
    scan.mkdir()
    if scenario == "burst":
        scan = scan / ".build"
        scan.mkdir()
    for index in range(directories):
        directory = scan / f"branch-{index:04d}"
        directory.mkdir()
        for child in range(children):
            if scenario == "traversal":
                (directory / f"leaf-{child:03d}").mkdir()
            else:
                (directory / f"data-{child:03d}.txt").write_bytes(b"payload")
wide = root / "wide"
wide.mkdir()
for index in range(50_000):
    (wide / f"{index:06d}.pyc").write_bytes(b"payload")
(root / "fixture-identity.json").write_text(json.dumps({
    "traversal_directories": 20_201, "burst_files": 65_536, "wide_files": 50_000,
    "payload": "payload", "immutable_during_comparison": True}, indent=2) + "\n")
print(root)
