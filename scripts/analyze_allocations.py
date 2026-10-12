#!/usr/bin/env python3
"""Summarize live allocations exported from Instruments without loading all XML.

Input is the Allocations List detail table, not xctrace's data/table schemas.
Raw traces stay local; the JSON contains categories and symbol names only.
"""
import argparse
from collections import defaultdict
import json
import xml.etree.ElementTree as ET


def analyze(path):
    frames = {}
    backtraces = {}
    categories = defaultdict(lambda: [0, 0])
    stacks = defaultdict(lambda: [0, 0])
    large = []
    container = None
    for event, row in ET.iterparse(path, events=("start", "end")):
        if event == "start":
            if row.tag == "node":
                container = row
            continue
        if row.tag != "row":
            continue
        names = []
        backtrace = row.find("backtrace")
        if backtrace is not None:
            if "ref" in backtrace.attrib:
                names = backtraces[backtrace.attrib["ref"]]
            else:
                for frame in backtrace.findall("frame"):
                    if "id" in frame.attrib:
                        frames[frame.attrib["id"]] = frame.attrib.get("name", "?")
                    names.append(frames.get(frame.attrib.get("ref"), frame.attrib.get("name", "?")))
            if "id" in backtrace.attrib:
                backtraces[backtrace.attrib["id"]] = names
        if row.attrib.get("live") == "true":
            category = row.attrib.get("category", "?")
            size = int(row.attrib.get("size", "0"))
            categories[category][0] += size
            categories[category][1] += 1
            if category == "_ContiguousArrayStorage<ScanRow>":
                key = tuple(name for name in names
                            if any(part in name for part in ("ScanTable", "WindowScan", "BuildHunter")))
                stacks[key][0] += size
                stacks[key][1] += 1
            if size >= 32 * 1024 * 1024:
                large.append(dict(bytes=size, category=category, stack=names,
                                  timestamp=row.attrib.get("timestamp")))
        row.clear()
        if container is not None:
            container.remove(row)

    def ranked(groups, label):
        return [{label: key, "bytes": value[0], "count": value[1]}
                for key, value in sorted(groups.items(), key=lambda item: -item[1][0])]
    return dict(live_bytes=sum(value[0] for value in categories.values()),
                categories=ranked(categories, "category")[:30],
                scanrow_stacks=ranked(stacks, "stack"), large_allocations=large)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("xml")
    parser.add_argument("--output", required=True)
    parser.add_argument("--statistics", help="Statistics XML to cross-check total live bytes")
    args = parser.parse_args()
    result = analyze(args.xml)
    if args.statistics:
        statistics = ET.parse(args.statistics)
        expected = next(int(row.attrib["persistent-bytes"]) for row in statistics.iter("row")
                        if row.attrib.get("category") == "All Heap & Anonymous VM")
        if result["live_bytes"] != expected:
            raise SystemExit(f"Live bytes differ: list={result['live_bytes']}, statistics={expected}")
    with open(args.output, "w") as output:
        json.dump(result, output, indent=2)
        output.write("\n")
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
