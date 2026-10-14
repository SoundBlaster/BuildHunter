"""Verify the pinned Hunter distribution snapshot without network access."""
import hashlib
import json
from pathlib import Path
import re

FILES = {"Cargo.toml", "LICENSE", "README.md", "src/lib.rs"}


def verify(directory):
    directory = Path(directory)
    record = json.loads((directory / "UPSTREAM.json").read_text())
    if (record["repository"] != "https://github.com/SoundBlaster/Hunter"
            or record["source_directory"] != "crates/hunter-fs-core"
            or not re.fullmatch(r"[0-9a-f]{40}", record["revision"])
            or set(record["files"]) != FILES):
        raise ValueError("Invalid Hunter snapshot provenance")
    for path, expected in record["files"].items():
        actual = hashlib.sha256((directory / path).read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"Hunter snapshot differs from pinned content: {path}")
    return record["revision"]


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[1]
    print("Hunter vendored snapshot verified:", verify(root / "vendor/hunter-fs-core"))
