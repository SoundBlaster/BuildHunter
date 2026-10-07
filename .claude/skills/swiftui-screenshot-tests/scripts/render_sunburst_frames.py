#!/usr/bin/env python3
"""Render ring-sector animation frames (JSON) into a contact sheet, no Mac needed.

Use it with a Linux SwiftPM harness that runs the app's pure geometry code
(see references/linux-harness.md) and prints frames as JSON:

  [ {"label": "zoom 0.3",            # or any scalar keys, e.g. phase/fade/zoom
     "frames": [ {"id": "Projects/App", "start": 0.0, "end": 0.25,
                  "inner": 0.22, "outer": 0.46, "opacity": 1.0,
                  "color": "#4c8bf5"  (optional)} ] } ]

start/end are fractions of a full turn clockwise from 12 o'clock; inner/outer
are fractions of the chart radius. Without "color", ids are hashed so every
sector keeps its color across frames, and children share their branch hue
(--branch-depth path components). Ids starting with "other:" are gray.

Usage:
  render_sunburst_frames.py frames.json --out sheet.png [--cols 5] [--size 220]
                            [--center-radius 0.21] [--branch-depth 2]
"""
import argparse
import colorsys
import hashlib
import json
import math

from PIL import Image, ImageDraw


def color(frame, depth, alpha):
    if "color" in frame:
        hexa = frame["color"].lstrip("#")
        r, g, b = (int(hexa[i:i + 2], 16) for i in (0, 2, 4))
    elif str(frame["id"]).startswith("other:"):
        r, g, b = 140, 140, 140
    else:
        key = "/".join(str(frame["id"]).split("/")[:depth])
        hue = int(hashlib.md5(key.encode()).hexdigest()[:6], 16) / 0xFFFFFF
        r, g, b = (int(255 * c) for c in colorsys.hsv_to_rgb(hue, 0.6, 0.9))
    return (r, g, b, int(255 * max(0.0, min(1.0, alpha))))


def label_for(state, index):
    if "label" in state:
        return str(state["label"])
    parts = [f"{k}={v:.2f}" if isinstance(v, float) else f"{k}={v}"
             for k, v in state.items() if k != "frames" and not isinstance(v, (list, dict))]
    return " ".join(parts) or f"#{index}"


def tile(state, index, size, center_radius, depth):
    radius = size * 0.46
    c = size / 2
    layer = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    for frame in state["frames"]:
        opacity = frame.get("opacity", 1.0)
        if opacity <= 0.01 or frame["outer"] - frame["inner"] < 1e-4:
            continue
        a0 = frame["start"] * 2 * math.pi - math.pi / 2
        a1 = frame["end"] * 2 * math.pi - math.pi / 2
        steps = max(2, int((a1 - a0) / 0.03))
        outer = [(c + math.cos(a0 + (a1 - a0) * i / steps) * frame["outer"] * radius,
                  c + math.sin(a0 + (a1 - a0) * i / steps) * frame["outer"] * radius) for i in range(steps + 1)]
        inner = [(c + math.cos(a1 - (a1 - a0) * i / steps) * frame["inner"] * radius,
                  c + math.sin(a1 - (a1 - a0) * i / steps) * frame["inner"] * radius) for i in range(steps + 1)]
        sector = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        ImageDraw.Draw(sector).polygon(outer + inner, fill=color(frame, depth, opacity),
                                       outline=(255, 255, 255, int(255 * opacity)))
        layer = Image.alpha_composite(layer, sector)
    if center_radius > 0:
        r = center_radius * radius
        ImageDraw.Draw(layer).ellipse([c - r, c - r, c + r, c + r], fill=(236, 236, 240, 255))
    out = Image.new("RGBA", (size, size + 18), (255, 255, 255, 255))
    out.alpha_composite(layer, (0, 0))
    ImageDraw.Draw(out).text((6, size + 2), label_for(state, index), fill=(0, 0, 0, 255))
    return out.convert("RGB")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("frames")
    ap.add_argument("--out", required=True)
    ap.add_argument("--cols", type=int, default=5)
    ap.add_argument("--size", type=int, default=220)
    ap.add_argument("--center-radius", type=float, default=0.0, help="draw a center disc on top, like a button")
    ap.add_argument("--branch-depth", type=int, default=2)
    args = ap.parse_args()
    with open(args.frames) as fh:
        states = json.load(fh)
    tiles = [tile(s, i, args.size, args.center_radius, args.branch_depth) for i, s in enumerate(states)]
    cols = max(1, min(args.cols, len(tiles)))
    rows = math.ceil(len(tiles) / cols)
    sheet = Image.new("RGB", (cols * args.size, rows * (args.size + 18)), (255, 255, 255))
    for i, t in enumerate(tiles):
        sheet.paste(t, ((i % cols) * args.size, (i // cols) * (args.size + 18)))
    sheet.save(args.out)
    print(f"{args.out}: {len(tiles)} frames")


if __name__ == "__main__":
    main()
