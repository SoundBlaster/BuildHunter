#!/usr/bin/env python3
"""List or export XCTest attachments from an .xcresult bundle without Xcode.

Works on Linux and macOS. Two sources of truth, tried in order:
  * database.sqlite3, which links every attachment to its activity, test case
    run and test case. xcresulttool creates it the first time it reads the
    bundle, so a bundle straight from a failed `xcodebuild test` lacks it.
  * the object graph in Data/: ActionTestSummary objects in Xcode's
    length-prefixed encoding, which every bundle has.
Payloads live in Data/data.<id>, sometimes zstd-compressed.

Usage:
  xcresult_attachments.py BUNDLE.xcresult --list
  xcresult_attachments.py BUNDLE.xcresult --out DIR [--test SUBSTR] [--type png,mp4,txt]
                          [--failures-only] [--video-fps 2]

Export writes DIR/<TestClass>/<testName>/<NN>_<name>.<ext> plus DIR/manifest.json.
"""
import argparse
import datetime
import json
import os
import re
import shutil
import sqlite3
import subprocess
import sys

APPLE_EPOCH = 978307200  # 2001-01-01 in Unix time
ZSTD_MAGIC = b"\x28\xb5\x2f\xfd"
UTI_EXT = {
    "public.png": "png",
    "public.jpeg": "jpg",
    "public.heic": "heic",
    "public.mpeg-4": "mp4",
    "com.apple.quicktime-movie": "mov",
    "public.plain-text": "txt",
    "public.utf8-plain-text": "txt",
    "public.json": "json",
    "public.data": "bin",
}
MAGIC_EXT = [
    (b"\x89PNG", "png"),
    (b"\xff\xd8\xff", "jpg"),
    (b"{", "json"),
    (b"[", "json"),
]

QUERY = """
SELECT a.rowid, a.name, a.filenameOverride, a.uniformTypeIdentifier, a.timestamp,
       a.xcResultKitPayloadRefId, a.testIssue_fk, a.activity_fk
FROM Attachments a ORDER BY a.timestamp
"""


def open_db(bundle):
    path = os.path.join(bundle, "database.sqlite3")
    if not os.path.exists(path):
        sys.exit(f"{path} not found; is this an Xcode 16+ .xcresult bundle?")
    db = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    return db


def test_run_for(db, activity_fk, issue_fk):
    """Walk up nested activities (or the test issue) to the owning test case run."""
    run = None
    failure = issue_fk is not None
    activity = None
    seen = set()
    fk = activity_fk
    while fk is not None and fk not in seen:
        seen.add(fk)
        row = db.execute(
            "SELECT title, testCaseRun_fk, parent_fk, failureIDs FROM Activities WHERE rowid=?", (fk,)
        ).fetchone()
        if row is None:
            break
        activity = activity or row["title"]
        failure = failure or bool(row["failureIDs"])
        if row["testCaseRun_fk"] is not None:
            run = row["testCaseRun_fk"]
            break
        fk = row["parent_fk"]
    if run is None and issue_fk is not None:
        row = db.execute("SELECT testCaseRun_fk FROM TestIssues WHERE rowid=?", (issue_fk,)).fetchone()
        run = row["testCaseRun_fk"] if row else None
    if run is None:
        return None, None, activity, failure
    row = db.execute(
        """SELECT tc.identifier, tcr.result FROM TestCaseRuns tcr
           JOIN TestCases tc ON tc.rowid = tcr.testCase_fk WHERE tcr.rowid=?""",
        (run,),
    ).fetchone()
    return (row["identifier"], row["result"], activity, failure) if row else (None, None, activity, failure)


def read_payload(bundle, ref):
    path = os.path.join(bundle, "Data", f"data.{ref}")
    with open(path, "rb") as fh:
        raw = fh.read()
    if raw.startswith(ZSTD_MAGIC):
        try:
            import zstandard
        except ImportError:
            sys.exit("payload is zstd-compressed: pip install zstandard")
        # Frames may omit the content size, so stream instead of decompress().
        return zstandard.ZstdDecompressor().stream_reader(raw).read()
    return raw


def extension(uti, data):
    if uti in UTI_EXT and UTI_EXT[uti] != "bin":
        return UTI_EXT[uti]
    for magic, ext in MAGIC_EXT:
        if data.startswith(magic):
            return ext
    if data[4:8] == b"ftyp":
        return "mp4"
    try:
        data[:4096].decode("utf-8")
        return "txt"
    except UnicodeDecodeError:
        return "bin"


def safe(text):
    return re.sub(r"[^A-Za-z0-9._-]+", "_", text).strip("_") or "unnamed"


def collect(bundle):
    if os.path.exists(os.path.join(bundle, "database.sqlite3")):
        return collect_db(bundle)
    return collect_object_graph(bundle)


def collect_db(bundle):
    db = open_db(bundle)
    items = []
    for row in db.execute(QUERY).fetchall():
        test, result, activity, failure = test_run_for(db, row["activity_fk"], row["testIssue_fk"])
        items.append({
            "test": test or "(unknown)",
            "result": result,
            "name": row["name"] or row["filenameOverride"] or "attachment",
            "uti": row["uniformTypeIdentifier"],
            "activity": activity,
            "associatedWithFailure": failure,
            "timestamp": (row["timestamp"] or 0) + APPLE_EPOCH,
            "payload": row["xcResultKitPayloadRefId"],
        })
    return items


# Object-graph fallback ------------------------------------------------------
# Encoding: an object is "[" + type + fields/items + "]". The type is either
# "T" followed by a descriptor object ([K2:_nV<n>:Name ...]) or "S<n>:Name" for
# a type seen before. A field is "K<n>:key" followed by an object or by a raw
# "V<n>:value". Array elements follow the type directly. Primitives are
# objects with a single _v field.

def _lp(blob, i):
    colon = blob.index(b":", i + 1)
    size = int(blob[i + 1:colon])
    return blob[colon + 1:colon + 1 + size], colon + 1 + size


def _object(blob, i):
    if blob[i:i + 1] != b"[":
        raise ValueError(f"expected [ at {i}")
    i += 1
    tag = blob[i:i + 1]
    if tag == b"T":
        descriptor, i = _object(blob, i + 1)
        name = descriptor.get("_n") if isinstance(descriptor, dict) else None
    elif tag == b"S":
        raw, i = _lp(blob, i)
        name = raw.decode()
    else:
        name = None  # a type descriptor itself
    obj = {"_type": name}
    items = []
    while True:
        tag = blob[i:i + 1]
        if tag == b"]":
            i += 1
            break
        if tag == b"K":
            key, i = _lp(blob, i)
            if blob[i:i + 1] == b"V":
                value, i = _lp(blob, i)
                obj[key.decode()] = value.decode("utf-8", "replace")
            else:
                obj[key.decode()], i = _object(blob, i)
        elif tag == b"[":
            item, i = _object(blob, i)
            items.append(item)
        else:
            raise ValueError(f"unexpected {tag!r} at {i}")
    if items:
        obj["_items"] = items
    if set(obj) == {"_type", "_v"}:
        return obj["_v"], i
    return obj, i


def _apple_date(text):
    try:
        return datetime.datetime.strptime(text, "%Y-%m-%dT%H:%M:%S.%f%z").timestamp()
    except (TypeError, ValueError):
        return 0


def collect_object_graph(bundle):
    data_dir = os.path.join(bundle, "Data")
    items = []
    for name in os.listdir(data_dir):
        if not name.startswith("data."):
            continue
        blob = read_payload(bundle, name[len("data."):])
        if not blob.startswith(b"[") or b"ActionTestAttachment" not in blob:
            continue
        try:
            summary, _ = _object(blob, 0)
        except (ValueError, IndexError):
            continue
        if not isinstance(summary, dict) or summary.get("_type") != "ActionTestSummary":
            continue
        test = summary.get("identifier", "(unknown)")
        result = summary.get("testStatus")

        def walk(node, activity, failure):
            if isinstance(node, list):
                for child in node:
                    walk(child, activity, failure)
                return
            if not isinstance(node, dict):
                return
            kind = node.get("_type")
            if kind == "ActionTestActivitySummary":
                activity = node.get("title", activity)
                ids = node.get("failureSummaryIDs")
                failure = failure or bool(isinstance(ids, dict) and ids.get("_items"))
            elif kind == "ActionTestFailureSummary":
                failure = True
            elif kind == "ActionTestAttachment":
                ref = node.get("payloadRef")
                ref = ref.get("id") if isinstance(ref, dict) else None
                if ref:
                    items.append({
                        "test": test,
                        "result": result,
                        "name": node.get("name") or node.get("filename") or "attachment",
                        "uti": node.get("uniformTypeIdentifier"),
                        "activity": activity,
                        # deleteOnSuccess attachments only survive because the test failed.
                        "associatedWithFailure": failure or node.get("lifetime") == "deleteOnSuccess",
                        "timestamp": _apple_date(node.get("timestamp")),
                        "payload": ref,
                    })
                return
            for key, value in node.items():
                if key != "_type":
                    walk(value, activity, failure)

        walk(summary, None, False)
    if not items:
        sys.exit("no test attachments found in the object graph")
    return sorted(items, key=lambda item: item["timestamp"])


def extract_frames(video, fps):
    if shutil.which("ffmpeg") is None:
        print(f"  ffmpeg not found; skipped frames for {video}", file=sys.stderr)
        return None
    frames_dir = os.path.splitext(video)[0] + "_frames"
    os.makedirs(frames_dir, exist_ok=True)
    subprocess.run(
        ["ffmpeg", "-loglevel", "error", "-y", "-i", video, "-vf", f"fps={fps}",
         os.path.join(frames_dir, "frame_%04d.png")],
        check=True,
    )
    return frames_dir


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("bundle")
    ap.add_argument("--list", action="store_true", help="print attachments, export nothing")
    ap.add_argument("--out", help="export directory")
    ap.add_argument("--test", help="only tests whose identifier contains this substring")
    ap.add_argument("--type", help="comma-separated extensions to keep, e.g. png,mp4,txt")
    ap.add_argument("--failures-only", action="store_true")
    ap.add_argument("--video-fps", type=float, default=0, help="also split videos into PNG frames")
    args = ap.parse_args()

    items = collect(args.bundle)
    if args.test:
        items = [i for i in items if args.test in i["test"]]
    if args.failures_only:
        items = [i for i in items if i["associatedWithFailure"] or i["result"] == "Failure"]

    if args.list or not args.out:
        for i in items:
            flag = " FAIL" if i["associatedWithFailure"] else ""
            print(f"{i['test']}\t{i['result']}\t{i['uti']}\t{i['name']}{flag}")
        print(f"{len(items)} attachments", file=sys.stderr)
        return

    keep = set(args.type.split(",")) if args.type else None
    os.makedirs(args.out, exist_ok=True)
    counters = {}
    exported = []
    for i in items:
        data = read_payload(args.bundle, i["payload"])
        ext = extension(i["uti"], data)
        if keep and ext not in keep:
            continue
        test_dir = os.path.join(args.out, *[safe(p) for p in i["test"].split("/")])
        os.makedirs(test_dir, exist_ok=True)
        n = counters.get(test_dir, 0)
        counters[test_dir] = n + 1
        path = os.path.join(test_dir, f"{n:02d}_{safe(i['name'])}.{ext}")
        with open(path, "wb") as fh:
            fh.write(data)
        entry = {k: v for k, v in i.items() if k != "payload"}
        entry["path"] = os.path.relpath(path, args.out)
        if ext in ("mp4", "mov") and args.video_fps > 0:
            frames = extract_frames(path, args.video_fps)
            if frames:
                entry["frames"] = os.path.relpath(frames, args.out)
        exported.append(entry)
    with open(os.path.join(args.out, "manifest.json"), "w") as fh:
        json.dump(exported, fh, indent=2)
    print(f"exported {len(exported)} attachments to {args.out}")


if __name__ == "__main__":
    main()
