"""Verify the pinned Hunter distribution snapshot without network access."""
import hashlib
import json
from pathlib import Path
import re

FILES = {"Cargo.toml", "LICENSE", "README.md", "src/lib.rs", "src/work.rs", "src/task.rs", "src/children.rs", "src/ledger.rs", "src/subtree.rs", "src/execution.rs", "src/backend.rs", "src/worker.rs", "src/listing.rs", "src/measurement.rs", "src/session.rs", "src/admission.rs", "src/lifetime.rs", "src/materialization.rs", "src/assembly.rs"}


def verify(directory):
    directory = Path(directory)
    if directory.is_symlink():
        raise ValueError("Hunter snapshot root must not be a symlink")
    provenance = directory / "UPSTREAM.json"
    if provenance.is_symlink() or not provenance.is_file():
        raise ValueError("Hunter provenance must be a regular file, not a symlink")
    record = json.loads(provenance.read_text())
    if (record["repository"] != "https://github.com/SoundBlaster/Hunter"
            or record["source_directory"] != "crates/hunter-fs-core"
            or not re.fullmatch(r"[0-9a-f]{40}", record["revision"])
            or set(record["files"]) != FILES):
        raise ValueError("Invalid Hunter snapshot provenance")
    allowed_files = FILES | {"UPSTREAM.json"}
    allowed_directories = {"src"}
    actual_files = set()
    for entry in directory.rglob("*"):
        relative = entry.relative_to(directory).as_posix()
        if entry.is_symlink():
            raise ValueError(f"Hunter snapshot contains a symlink: {relative}")
        if entry.is_dir():
            if relative not in allowed_directories:
                raise ValueError(f"Unexpected Hunter snapshot directory: {relative}")
        elif entry.is_file():
            actual_files.add(relative)
        else:
            raise ValueError(f"Unexpected Hunter snapshot entry type: {relative}")
    if actual_files != allowed_files:
        raise ValueError(f"Unexpected Hunter snapshot file inventory: {sorted(actual_files ^ allowed_files)}")
    for path, expected in record["files"].items():
        actual = hashlib.sha256((directory / path).read_bytes()).hexdigest()
        if actual != expected:
            raise ValueError(f"Hunter snapshot differs from pinned content: {path}")
    return record["revision"]


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[1]
    print("Hunter vendored snapshot verified:", verify(root / "vendor/hunter-fs-core"))
