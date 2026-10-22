"""Run the real CLI against the shared, reviewed migration golden corpus."""
import json
import os
from pathlib import Path
import stat
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
FIXTURE = ROOT / "tests/fixtures/migration-parity-v1.json"
BINARY = Path(sys.argv.pop(1)).resolve()
CORPUS = json.loads(FIXTURE.read_text())


def materialize(root, case):
    for directory in case["directories"]:
        (root / directory).mkdir(parents=True, exist_ok=True)
    for name, content in case["files"].items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content.encode("utf-8"))
    for name, target in case["symlinks"].items():
        path = root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.symlink_to(target, target_is_directory=(path.parent / target).is_dir())


def measured_bytes(path, apparent):
    """Independent filesystem oracle: include directory metadata, skip links."""
    info = path.lstat()
    if stat.S_ISLNK(info.st_mode):
        return 0
    size = info.st_size if apparent or os.name == "nt" else info.st_blocks * 512
    if stat.S_ISDIR(info.st_mode):
        size += sum(measured_bytes(child, apparent) for child in path.iterdir())
    return size


class MigrationParityTests(unittest.TestCase):
    def test_catalog_ids_are_frozen(self):
        result = subprocess.run([str(BINARY), "--list-filters", "--json"],
                                capture_output=True, text=True, encoding="utf-8", check=True, timeout=20)
        self.assertEqual([row["id"] for row in json.loads(result.stdout)], CORPUS["catalogIDs"])
        self.assertEqual(CORPUS["schemaVersion"], 1)

    def test_lifecycle_and_bridge_terminal_goldens_agree(self):
        mapping = {str(row["rustStatus"]): row["gui"] for row in CORPUS["terminalMappings"]}
        for line in (ROOT / "tests/fixtures/migration-lifecycle-v1.tsv").read_text().splitlines():
            if line and not line.startswith("#"):
                fields = line.split("\t")
                self.assertEqual(len(fields), 6)
                self.assertEqual(mapping[fields[1]], fields[5])

    def test_cli_golden_profiles_and_size_modes(self):
        for case in CORPUS["cases"]:
            if case.get("unixOnly", False) and os.name == "nt":
                continue
            for run in case["runs"]:
                if run["client"] != "cli":
                    continue
                for apparent in [False, True]:
                    with self.subTest(tree=case["id"], run=run["id"], apparent=apparent):
                        self.check_case(case, run, apparent)

    def check_case(self, case, run, apparent):
        with tempfile.TemporaryDirectory(prefix="BuildHunter parity ") as directory:
            scratch = Path(directory)
            root = scratch / "tree"
            root.mkdir()
            materialize(root, case)
            selected = root / case["scanRoot"]
            profile = scratch / "profile.json"  # never changes the measured tree
            command = [str(BINARY), str(selected), "--json", "--profile", str(profile),
                       "--language", run["language"]]
            if apparent:
                command.append("--apparent")
            if run["includeEnvironments"]:
                command.append("--include-envs")
            if run["excluded"]:
                command.extend(["--exclude", ",".join(run["excluded"])])
            result = subprocess.run(command, capture_output=True, text=True, encoding="utf-8", timeout=20)
            self.assertEqual(result.returncode, run["exitCode"], result.stderr)
            if run["status"] == "invalid-input":
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr.strip(), "PATH must be an accessible directory")
                self.assertFalse(profile.exists())
                self.assertEqual(run["expected"], [])
                return
            report = json.loads(result.stdout)
            accounting = json.loads(profile.read_text())
            self.assertEqual(accounting["status"], run["status"])
            self.assertEqual(len(report["errors"]), run["warnings"])
            expected = [case["rows"][key] for key in run["expected"]]
            actual = []
            for row in report["artifacts"]:
                # The CLI canonicalizes its input; macOS TMPDIR can start at /var,
                # an alias of /private/var. Normalize only this comparison base.
                relative = os.path.relpath(row["path"], selected.resolve()).replace(os.sep, "/")
                actual.append((relative, row["language"], row["kind"], row["nested"]))
                self.assertEqual(row["bytes"], measured_bytes(Path(row["path"]), apparent))
                self.assertFalse(row["partial"])
            self.assertEqual(sorted(actual), sorted((row["path"], row["language"], row["kind"], row["nested"])
                                                    for row in expected))
            total = sum(measured_bytes(selected / row["path"], apparent)
                        for row in expected if not row["nested"])
            self.assertEqual(report["total_bytes"], total)
            self.assertEqual(accounting["samples"][-1]["measured_bytes"], total)


if __name__ == "__main__":
    unittest.main()
