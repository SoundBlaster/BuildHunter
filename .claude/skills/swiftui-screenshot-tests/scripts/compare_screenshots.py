#!/usr/bin/env python3
"""Compare current screenshots against baselines and write visual diffs.

Usage:
  compare_screenshots.py BASELINE CURRENT --diff-dir DIR [--tolerance 0.001]
                         [--pixel-threshold 16] [--ignore X,Y,W,H ...]

BASELINE and CURRENT are both files or both directories. Directories may be:
  * an export from xcresult_attachments.py (has manifest.json with "path"),
  * a raw `xcresulttool export attachments` folder (manifest.json with
    "suggestedHumanReadableName"), as CI uploads in ui-screenshots/,
  * any folder of PNGs.
Images are paired by test + attachment name (repeats become name#2, name#3 in
capture order), so UUID file names and ordinal
prefixes do not matter.

A pixel counts as changed when any channel differs by more than
--pixel-threshold (0-255). An image fails when the changed fraction exceeds
--tolerance, or when sizes differ. Exit status is 1 when anything failed, is
missing, or is new. DIR gets a baseline|current|diff triptych per changed image,
plus report.md and report.json. Triptych names start with the status
(fail__, size__, pass__ for changes under tolerance).
"""
import argparse
import json
import os
import re
import sys

from PIL import Image, ImageChops, ImageDraw

UUID = r"[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}"


def strip_noise(name):
    name = os.path.splitext(name)[0]
    name = re.sub(rf"_\d+_{UUID}$", "", name)  # xcresulttool suggested names: shot_0_<UUID>
    name = re.sub(rf"[_-]?{UUID}", "", name)
    name = re.sub(r"^\d+_", "", name)  # ordinal prefix from xcresult_attachments.py
    return name


def index(root):
    """Map a stable key -> image path.

    Keys are test/attachment-name. A name a test attaches more than once gets
    #2, #3, ... in capture order, so repeated checkpoints are compared one to
    one instead of collapsing into the first. Occurrences are counted even when
    a file is missing, so one lost capture doesn't shift the others.
    """
    if os.path.isfile(root):
        return {"image": root}
    keyed = {}
    seen = {}

    def add(key, path):
        seen[key] = seen.get(key, 0) + 1
        if seen[key] > 1:
            key = f"{key}#{seen[key]}"
        if os.path.exists(path):
            keyed[key] = path

    manifest_path = os.path.join(root, "manifest.json")
    if os.path.exists(manifest_path):
        with open(manifest_path) as fh:
            manifest = json.load(fh)
        # xcresult_attachments.py manifest: already in capture order.
        for entry in manifest:
            if entry.get("path", "").lower().endswith(".png"):
                add(f"{entry['test']}/{entry['name']}", os.path.join(root, entry["path"]))
        # `xcresulttool export attachments` manifest: one entry per test, unordered attachments.
        for entry in manifest:
            attachments = sorted(entry.get("attachments", []), key=lambda a: a.get("timestamp", 0))
            for attachment in attachments:
                file_name = attachment.get("exportedFileName", "")
                if file_name.lower().endswith(".png"):
                    name = strip_noise(attachment["suggestedHumanReadableName"])
                    add(f"{entry['testIdentifier']}/{name}", os.path.join(root, file_name))
        if seen:
            return keyed
    for dirpath, _, files in os.walk(root):
        for name in sorted(files):  # ordinal prefixes keep capture order
            if name.lower().endswith(".png"):
                rel = os.path.relpath(os.path.join(dirpath, name), root)
                parts = rel.split(os.sep)
                parts[-1] = strip_noise(parts[-1])
                add("/".join(parts), os.path.join(dirpath, name))
    return keyed


def parse_rect(text):
    x, y, w, h = (int(v) for v in text.split(","))
    return (x, y, x + w, y + h)


def compare(base_path, cur_path, threshold, ignore):
    base = Image.open(base_path).convert("RGB")
    cur = Image.open(cur_path).convert("RGB")
    if base.size != cur.size:
        return {"status": "size", "baseSize": base.size, "currentSize": cur.size}, base, cur, None
    delta = ImageChops.difference(base, cur)
    r, g, b = delta.split()
    peak = ImageChops.lighter(ImageChops.lighter(r, g), b)
    if ignore:
        draw = ImageDraw.Draw(peak)
        for rect in ignore:
            draw.rectangle(rect, fill=0)
    mask = peak.point(lambda v: 255 if v > threshold else 0)
    changed = mask.histogram()[255]
    total = base.width * base.height
    return {
        "status": "ok",
        "changedPixels": changed,
        "changedFraction": changed / total,
        "maxDelta": peak.getextrema()[1],
        "bbox": mask.getbbox(),
    }, base, cur, mask


def triptych(base, cur, mask, bbox, out):
    width = max(base.width, cur.width)
    height = max(base.height, cur.height)
    canvas = Image.new("RGB", (width * 3 + 16, height), (255, 255, 255))
    canvas.paste(base, (0, 0))
    canvas.paste(cur, (width + 8, 0))
    if mask is not None:
        overlay = Image.blend(base, Image.new("RGB", base.size, (255, 255, 255)), 0.7)
        overlay.paste((230, 30, 30), mask=mask)
        canvas.paste(overlay, (2 * (width + 8), 0))
        if bbox:
            draw = ImageDraw.Draw(canvas)
            for offset in (0, width + 8, 2 * (width + 8)):
                x0, y0, x1, y1 = bbox
                draw.rectangle([x0 + offset - 2, y0 - 2, x1 + offset + 2, y1 + 2], outline=(255, 140, 0), width=2)
    canvas.save(out)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("baseline")
    ap.add_argument("current")
    ap.add_argument("--diff-dir", required=True)
    ap.add_argument("--tolerance", type=float, default=0.001, help="allowed changed-pixel fraction")
    ap.add_argument("--pixel-threshold", type=int, default=16, help="per-channel delta that counts as a change")
    ap.add_argument("--ignore", action="append", default=[], type=parse_rect, metavar="X,Y,W,H",
                    help="region to ignore, in pixels (repeatable)")
    args = ap.parse_args()

    base_index = index(args.baseline)
    cur_index = index(args.current)
    os.makedirs(args.diff_dir, exist_ok=True)
    results = []
    for key in sorted(set(base_index) | set(cur_index)):
        if key not in cur_index:
            results.append({"key": key, "status": "missing", "baseline": base_index[key]})
            continue
        if key not in base_index:
            results.append({"key": key, "status": "new", "current": cur_index[key]})
            continue
        info, base, cur, mask = compare(base_index[key], cur_index[key], args.pixel_threshold, args.ignore)
        info.update(key=key, baseline=base_index[key], current=cur_index[key])
        if info["status"] == "ok":
            info["status"] = "fail" if info["changedFraction"] > args.tolerance else "pass"
        if info["status"] != "pass" or info.get("changedPixels"):
            # Below-tolerance changes get a triptych too, prefixed so failures sort first.
            diff_path = os.path.join(args.diff_dir,
                                     f"{info['status']}__" + re.sub(r"[^A-Za-z0-9._-]+", "_", key) + ".png")
            triptych(base, cur, mask, info.get("bbox"), diff_path)
            info["diff"] = diff_path
        results.append(info)

    lines = ["| status | image | changed | max Δ | diff |", "|---|---|---|---|---|"]
    for r in results:
        changed = f"{r['changedFraction']:.4%}" if "changedFraction" in r else (
            f"{r['baseSize']} → {r['currentSize']}" if r["status"] == "size" else "")
        lines.append(f"| {r['status']} | `{r['key']}` | {changed} | {r.get('maxDelta', '')} | "
                     f"{os.path.basename(r['diff']) if 'diff' in r else ''} |")
    with open(os.path.join(args.diff_dir, "report.md"), "w") as fh:
        fh.write("\n".join(lines) + "\n")
    with open(os.path.join(args.diff_dir, "report.json"), "w") as fh:
        json.dump(results, fh, indent=2, default=list)
    print("\n".join(lines))
    bad = [r for r in results if r["status"] != "pass"]
    print(f"\n{len(results) - len(bad)} passed, {len(bad)} need attention; diffs in {args.diff_dir}")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
