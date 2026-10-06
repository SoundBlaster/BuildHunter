#!/usr/bin/env python3
"""Inspect Screener .vtrace bundles (manifest.json + timeline.jsonl + frames/).

Usage:
  vtrace.py list ROOT                      # find .vtrace bundles under ROOT, newest first
  vtrace.py timeline TRACE [--kinds marker,keyframe]
  vtrace.py sheet TRACE --out sheet.png [--after MARKER] [--window-ms 1500] [--every N] [--cols 5]

Times are milliseconds since sessionStarted, from monotonicNanoseconds.
Each frame is labeled with the latest marker before it, so the sheet reads like
"navigation.begin +120ms". `--after` keeps only frames within --window-ms after
each occurrence of a marker name (e.g. navigation.begin).
"""
import argparse
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from contact_sheet import sheet  # noqa: E402

FRAME_KINDS = ("keyframe", "thumbnail")


def load(trace):
    with open(os.path.join(trace, "manifest.json")) as fh:
        manifest = json.load(fh)
    records = []
    with open(os.path.join(trace, "timeline.jsonl")) as fh:
        for line in fh:
            line = line.strip()
            if not line:
                continue
            try:
                records.append(json.loads(line))
            except json.JSONDecodeError:
                break  # the writer may be mid-append on a live trace
    records.sort(key=lambda r: r["sequence"])
    origin = records[0]["monotonicNanoseconds"] if records else 0
    for record in records:
        record["ms"] = (record["monotonicNanoseconds"] - origin) / 1e6
    return manifest, records


def find_traces(root):
    found = []
    for dirpath, dirnames, _ in os.walk(root):
        for name in list(dirnames):
            if name.endswith(".vtrace"):
                path = os.path.join(dirpath, name)
                found.append(path)
                dirnames.remove(name)
    return sorted(found, key=os.path.getmtime, reverse=True)


def describe(record):
    meta = " ".join(f"{k}={v}" for k, v in sorted(record.get("metadata", {}).items()))
    blob = f" -> {record['blob']}" if record.get("blob") else ""
    return f"{record['ms']:9.1f}ms  {record['kind']:<14} {record['name']}{(' ' + meta) if meta else ''}{blob}"


def cmd_list(args):
    for trace in find_traces(args.root):
        try:
            manifest, records = load(trace)
        except (OSError, json.JSONDecodeError) as error:
            print(f"{trace}  unreadable: {error}")
            continue
        frames = sum(r["kind"] in FRAME_KINDS for r in records)
        markers = sum(r["kind"] == "marker" for r in records)
        span = records[-1]["ms"] if records else 0
        print(f"{trace}\n    {manifest.get('name')}  {manifest.get('startedAt')}  "
              f"{frames} frames, {markers} markers, {span:.0f}ms")


def cmd_timeline(args):
    manifest, records = load(args.trace)
    print(f"# {manifest.get('name')} ({manifest.get('appBundleID')}, {manifest.get('platform')}) "
          f"started {manifest.get('startedAt')}")
    kinds = set(args.kinds.split(",")) if args.kinds else None
    for record in records:
        if kinds is None or record["kind"] in kinds:
            print(describe(record))


def cmd_sheet(args):
    manifest, records = load(args.trace)
    last_marker = None
    windows = []
    picked = []
    for record in records:
        if record["kind"] == "marker":
            last_marker = record
            if args.after and record["name"] == args.after:
                windows.append(record["ms"])
            continue
        if record["kind"] not in FRAME_KINDS or not record.get("blob"):
            continue
        if args.after and not any(0 <= record["ms"] - start <= args.window_ms for start in windows):
            continue
        if last_marker:
            meta = ",".join(f"{k}={v}" for k, v in sorted(last_marker.get("metadata", {}).items()))
            label = f"{last_marker['name']} +{record['ms'] - last_marker['ms']:.0f}ms\n{meta}"
        else:
            label = f"{record['ms']:.0f}ms {record['name']}"
        picked.append((os.path.join(args.trace, record["blob"]), label))
    picked = picked[:: max(1, args.every)]
    if not picked:
        sys.exit("no frames matched")
    title = args.title or f"{manifest.get('name')} — {os.path.basename(args.trace)}"
    sheet(picked, args.out, args.cols, args.width, title)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("list")
    p.add_argument("root")
    p.set_defaults(fn=cmd_list)
    p = sub.add_parser("timeline")
    p.add_argument("trace")
    p.add_argument("--kinds")
    p.set_defaults(fn=cmd_timeline)
    p = sub.add_parser("sheet")
    p.add_argument("trace")
    p.add_argument("--out", required=True)
    p.add_argument("--after", help="marker name that opens a window of frames")
    p.add_argument("--window-ms", type=float, default=1500)
    p.add_argument("--every", type=int, default=1, help="keep every Nth frame")
    p.add_argument("--cols", type=int, default=5)
    p.add_argument("--width", type=int, default=280)
    p.add_argument("--title")
    p.set_defaults(fn=cmd_sheet)
    args = ap.parse_args()
    args.fn(args)


if __name__ == "__main__":
    main()
