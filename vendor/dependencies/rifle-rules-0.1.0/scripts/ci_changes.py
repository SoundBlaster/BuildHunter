"""Conservative heavy-job gate. A failed/unknown diff always requests builds."""
import os
from pathlib import PurePosixPath
import subprocess

LIGHT_FILES = {"specpm.yaml"}


def is_light(path):
    return (path.startswith(("docs/", "specs/")) or path in LIGHT_FILES
            or (len(PurePosixPath(path).parts) == 1 and path.endswith(".md")))


def requires_heavy(paths):
    return any(not is_light(path) for path in paths)


def changed_paths(event, base, head, run=subprocess.run):
    if event not in ("pull_request", "push") or not base or not head or set(base) == {"0"}:
        return None
    # PR: cumulative changes relative to merge base, not merely the latest commit.
    revision = base + ("..." if event == "pull_request" else "..") + head
    try:
        result = run(["git", "diff", "--name-only", "--no-renames", "-z", revision, "--"],
                     capture_output=True, check=True)
    except (OSError, subprocess.CalledProcessError):
        return None
    # --no-renames includes BOTH old and new paths, including code moved to docs.
    return [os.fsdecode(path) for path in result.stdout.split(b"\0") if path]


def main():
    paths = changed_paths(os.environ.get("EVENT_NAME"), os.environ.get("BASE_SHA"),
                          os.environ.get("HEAD_SHA"))
    heavy = paths is None or requires_heavy(paths)
    value = str(heavy).lower()
    print(f"heavy={value}; changed paths={len(paths) if paths is not None else 'unknown/manual'}")
    with open(os.environ["GITHUB_OUTPUT"], "a", encoding="utf-8") as output:
        output.write(f"heavy={value}\n")


if __name__ == "__main__":
    main()
