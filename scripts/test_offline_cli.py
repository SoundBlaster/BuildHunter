"""Prove root CLI build/install needs no dependency cache or network."""
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    root = Path(__file__).resolve().parents[1]
    with tempfile.TemporaryDirectory(prefix="buildhunter-offline-") as directory:
        scratch = Path(directory)
        env = dict(os.environ, CARGO_HOME=str(scratch / "cargo-home"),
                   CARGO_TARGET_DIR=str(scratch / "target"))
        commands = [
            ["cargo", "check", "--locked", "--offline"],
            ["cargo", "install", "--path", ".", "--locked", "--offline",
             "--root", str(scratch / "install")],
        ]
        for command in commands:
            subprocess.run(command, cwd=root, env=env, check=True)
        executable = scratch / "install" / "bin" / (
            "build-hunter.exe" if os.name == "nt" else "build-hunter")
        subprocess.run([str(executable), "--help"], cwd=root, env=env,
                       stdout=subprocess.DEVNULL, check=True)
    print("Fresh Cargo home: offline CLI check, install and launch passed")


if __name__ == "__main__":
    main()
