#!/usr/bin/env python3
"""Tile screenshots into one labeled PNG so a whole run can be reviewed at a glance.

Usage:
  contact_sheet.py INPUT... --out sheet.png [--cols 4] [--width 360] [--title TEXT]

INPUT may be image files or directories (searched recursively, sorted by path).
Each tile is labeled with its path relative to the directory it came from.
"""
import argparse
import math
import os
import sys

from PIL import Image, ImageDraw, ImageFont

IMAGE_EXTS = (".png", ".jpg", ".jpeg", ".heic")
LABEL_H = 30


def font(size=13):
    for path in ("/System/Library/Fonts/SFNSMono.ttf", "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"):
        if os.path.exists(path):
            return ImageFont.truetype(path, size)
    return ImageFont.load_default()


def gather(inputs):
    """Return (path, label) pairs: directories sorted by path, files in the order given."""
    items = []
    for inp in inputs:
        if os.path.isdir(inp):
            found = []
            for root, _, files in os.walk(inp):
                for name in files:
                    if name.lower().endswith(IMAGE_EXTS):
                        path = os.path.join(root, name)
                        found.append((path, os.path.relpath(path, inp)))
            items.extend(sorted(found, key=lambda item: item[1]))
        else:
            items.append((inp, os.path.basename(inp)))
    return items


def load(path):
    try:
        return Image.open(path).convert("RGB")
    except Exception as error:  # HEIC without pillow-heif, truncated PNG, ...
        print(f"skipped {path}: {error}", file=sys.stderr)
        return None


def fit_label(draw, text, fnt, width, keep_end):
    """Ellipsize to width; keep the end of paths (file names), the start of everything else."""
    if draw.textlength(text, font=fnt) <= width:
        return text
    while text and draw.textlength(text + "…", font=fnt) > width:
        text = text[1:] if keep_end else text[:-1]
    return "…" + text if keep_end else text + "…"


def sheet(items, out, cols=4, width=360, title=None):
    """items: (path, label) pairs, or (path, label, highlight) to outline a tile in red."""
    tiles = []
    for item in items:
        img = load(item[0])
        if img is None:
            continue
        height = max(1, round(img.height * width / img.width))
        tiles.append((img.resize((width, height), Image.LANCZOS), item[1], len(item) > 2 and item[2]))
    if not tiles:
        sys.exit("no images to tile")
    cols = max(1, min(cols, len(tiles)))
    rows = math.ceil(len(tiles) / cols)
    cell_h = max(t[0].height for t in tiles) + LABEL_H
    top = 34 if title else 0
    pad = 8
    canvas = Image.new("RGB", (cols * (width + pad) + pad, top + rows * (cell_h + pad) + pad), (245, 245, 247))
    draw = ImageDraw.Draw(canvas)
    fnt = font()
    if title:
        draw.text((pad, 9), title, fill=(20, 20, 20), font=font(16))
    for index, (img, label, highlight) in enumerate(tiles):
        x = pad + (index % cols) * (width + pad)
        y = top + pad + (index // cols) * (cell_h + pad)
        canvas.paste(img, (x, y))
        draw.rectangle([x - 1, y - 1, x + width, y + img.height], outline=(220, 40, 40) if highlight else (200, 200, 205),
                       width=3 if highlight else 1)
        for line_no, line in enumerate(label.split("\n")[:2]):
            draw.text((x + 2, y + img.height + 2 + line_no * 14), fit_label(draw, line, fnt, width - 4, keep_end=line_no == 0),
                      fill=(30, 30, 30), font=fnt)
    canvas.save(out)
    print(f"{out}: {len(tiles)} tiles, {canvas.width}x{canvas.height}")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("inputs", nargs="+")
    ap.add_argument("--out", required=True)
    ap.add_argument("--cols", type=int, default=4)
    ap.add_argument("--width", type=int, default=360, help="tile width in pixels")
    ap.add_argument("--title")
    args = ap.parse_args()
    sheet(gather(args.inputs), args.out, args.cols, args.width, args.title)


if __name__ == "__main__":
    main()
