#!/usr/bin/env python3
"""Exercise the compiled CLI against disposable filesystem fixtures."""
import json
import errno
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

BINARY = Path(sys.argv.pop(1)).resolve()


class CLIIntegrationTests(unittest.TestCase):
    def test_profile_is_opt_in_and_counts_nested_bytes_once(self):
        self.write(".build/__pycache__/module.pyc", b"bytecode")
        normal = self.scan("--apparent")
        destination = self.root / "profile.json"
        profiled = self.scan("--apparent", "--profile", str(destination))
        self.assertEqual(normal["artifacts"], profiled["artifacts"])
        profile = json.loads(destination.read_text())
        self.assertEqual(profile["schema_version"], 1)
        self.assertEqual(profile["status"], "completed")
        self.assertEqual(profile["size_mode"], "apparent")
        self.assertEqual(profile["samples"][-1]["measured_bytes"], normal["total_bytes"])
        self.assertGreater(profile["samples"][-1]["entries"], 0)
        self.assertGreaterEqual(profile["peak_entries_per_second"], 0)

    def test_profile_output_inside_artifact_does_not_affect_measurement(self):
        self.write(".build/object.o", b"fixture")
        normal = self.scan("--apparent")
        destination = self.root / ".build" / "profile.json"
        profiled = self.scan("--apparent", "--profile", str(destination))
        self.assertEqual(profiled["total_bytes"], normal["total_bytes"])
        self.assertEqual(json.loads(destination.read_text())["samples"][-1]["measured_bytes"], normal["total_bytes"])

    def test_profile_respects_search_filters(self):
        self.write(".build/object.o")
        self.write("__pycache__/module.pyc")
        destination = self.root / "profile.json"
        report = self.scan("--apparent", "--exclude", "swift.build", "--profile", str(destination))
        profile = json.loads(destination.read_text())
        self.assertEqual(profile["samples"][-1]["measured_bytes"], report["total_bytes"])
        self.assertEqual(profile["samples"][-1]["artifacts"], len(report["artifacts"]))

    def test_profile_requires_path_and_preserves_existing_output(self):
        destination = self.root / "profile.json"
        destination.write_text("keep me")
        result = subprocess.run([str(BINARY), str(self.root), "--profile", str(destination)], capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(destination.read_text(), "keep me")
        result = subprocess.run([str(BINARY), str(self.root), "--profile"], capture_output=True)
        self.assertEqual(result.returncode, 2)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="build-hunter-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()

    def write(self, relative, content=b"fixture"):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        return path

    def scan(self, *flags):
        result = subprocess.run([str(BINARY), str(self.root), "--json", *flags], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, 0, result.stderr)
        report = json.loads(result.stdout)
        self.assertFalse(report["errors"])
        self.assertEqual(report["total_bytes"], sum(r["bytes"] for r in report["artifacts"] if not r["nested"]))
        return report

    def test_language_detection_and_negative_project_markers(self):
        self.write("swift/.build/object")
        self.write("rust/Cargo.toml")
        self.write("rust/target/object")
        self.write("python/pyproject.toml")
        self.write("python/dist/wheel")
        self.write("unrelated/target/source")
        self.write("unrelated/dist/source")
        report = self.scan()
        self.assertEqual({r["language"] for r in report["artifacts"]}, {"swift", "rust", "python"})
        self.assertEqual(len(report["artifacts"]), 3)
        self.assertEqual([r["language"] for r in self.scan("--language", "rust")["artifacts"]], ["rust"])

    def test_nested_artifacts_count_once_and_scan_does_not_modify_files(self):
        path = self.write("app/.build/__pycache__/module.pyc", b"bytecode")
        before = path.read_bytes()
        report = self.scan("--apparent")
        self.assertEqual(len(report["artifacts"]), 3)
        outer = next(r for r in report["artifacts"] if not r["nested"])
        self.assertTrue(outer["path"].endswith(".build"))
        self.assertEqual(report["total_bytes"], outer["bytes"])
        self.assertGreaterEqual(outer["bytes"], len(before))
        self.assertEqual(path.read_bytes(), before)

    def test_environment_opt_in_and_ignored_git_data(self):
        self.write(".venv/pyvenv.cfg")
        self.write(".venv/__pycache__/module.pyc")
        self.write(".git/.build/object")
        self.assertEqual(self.scan()["artifacts"], [])
        report = self.scan("--include-envs")
        self.assertEqual(sum(not r["nested"] for r in report["artifacts"]), 1)
        self.assertEqual(next(r for r in report["artifacts"] if not r["nested"])["kind"], "environment")

    def test_individual_and_language_exclusions_preserve_nested_discovery(self):
        self.write("rust/Cargo.toml")
        self.write("rust/target/object")
        self.write("python/pyproject.toml")
        self.write("python/.pytest_cache/store")
        self.write("python/.pytest_cache/nested/.build/object")
        self.write("swift/.build/__pycache__/module.pyc", b"bytecode")

        individual = self.scan("--exclude", "python.pytest-cache,python.bytecode")
        # The CLI reports paths under its canonical root, which has a \\?\ prefix on Windows.
        root = Path(individual["root"])
        paths = {Path(row["path"]).relative_to(root).as_posix() for row in individual["artifacts"]}
        self.assertIn("python/.pytest_cache/nested/.build", paths)
        self.assertNotIn("python/.pytest_cache", paths)
        self.assertNotIn("swift/.build/__pycache__", paths)
        self.assertTrue(any(path.endswith("swift/.build") for path in paths))

        python_only = self.scan("--exclude", "python")
        self.assertEqual({row["language"] for row in python_only["artifacts"]}, {"swift", "rust"})

        mixed_aliases = self.scan("--exclude", "swift,rust")
        self.assertEqual({row["language"] for row in mixed_aliases["artifacts"]}, {"python"})

    def test_list_filters_uses_canonical_json_catalog(self):
        result = subprocess.run([str(BINARY), "--list-filters", "--json"], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(result.returncode, 0, result.stderr)
        catalog = json.loads(result.stdout)
        self.assertEqual(catalog[0]["id"], "swift.build")
        self.assertEqual(catalog[1]["id"], "rust.target")
        bytecode = next(item for item in catalog if item["id"] == "python.bytecode")
        self.assertEqual(bytecode["nodeNames"], ["__pycache__"])
        self.assertEqual(bytecode["suffixes"], [".pyc", ".pyo"])
        environment = next(item for item in catalog if item["id"] == "python.environment")
        self.assertIs(environment["requiresVenvMarker"], True)
        plain = subprocess.run([str(BINARY), "--list-filters"], capture_output=True, text=True, encoding="utf-8")
        self.assertEqual(plain.returncode, 0, plain.stderr)
        self.assertIn("python.mypy-cache", plain.stdout)

    def test_accepted_artifact_includes_nested_environment_contents(self):
        self.write("app/.build/.venv/pyvenv.cfg")
        self.write("app/.build/.venv/lib/module.pyc", b"bytecode" * 1024)
        default_report = self.scan("--apparent")
        opted_in_report = self.scan("--apparent", "--include-envs")

        self.assertEqual(default_report["total_bytes"], opted_in_report["total_bytes"])
        self.assertGreaterEqual(default_report["total_bytes"], len(b"bytecode" * 1024))
        self.assertEqual(sum(not r["nested"] for r in default_report["artifacts"]), 1)

    def test_json_escaping_and_symlink_is_not_followed(self):
        # Quotes are illegal in Windows filenames; exercise Unicode everywhere.
        name = "project-é" if sys.platform == "win32" else 'project-"é\t'
        self.write(f"{name}/.build/object")
        rows = self.scan()["artifacts"]
        self.assertEqual(len(rows), 1)
        self.assertTrue(os.path.samefile(rows[0]["path"], self.root / name / ".build"))
        if sys.platform != "win32":
            (self.root / "linked").symlink_to(self.root / name, target_is_directory=True)
            self.assertEqual(len(self.scan()["artifacts"]), 1)

    def test_invalid_arguments_and_empty_directory(self):
        self.assertEqual(self.scan()["total_bytes"], 0)
        for args in (["--language", "unknown"], ["--unknown"], [str(self.root / "missing")]):
            result = subprocess.run([str(BINARY), *args], capture_output=True, text=True, encoding="utf-8")
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)
        for args in (["--exclude"], ["--exclude", ""], ["--exclude", ","], ["--exclude", "swift,"], ["--exclude", "unknown.id"], ["--exclude", "--json"]):
            result = subprocess.run([str(BINARY), *args], capture_output=True, text=True, encoding="utf-8")
            self.assertEqual(result.returncode, 2, result.stdout + result.stderr)

    @unittest.skipIf(sys.platform == "win32", "Raw byte paths are Unix-specific")
    def test_non_utf8_path_argument_does_not_panic(self):
        root = os.fsencode(self.root) + b"/project-\xff"
        try:
            os.makedirs(root + b"/.build")
            with open(root + b"/.build/object", "wb") as output:
                output.write(b"fixture")
            expected_exit = 0
        except OSError as error:
            if error.errno != errno.EILSEQ:
                raise
            # Some macOS filesystems reject these names; argument parsing must
            # still return validation failure instead of panicking.
            expected_exit = 2
        result = subprocess.run([os.fsencode(BINARY), root, b"--json"], capture_output=True)
        self.assertEqual(result.returncode, expected_exit, result.stderr)
        if expected_exit == 0:
            report = json.loads(result.stdout)
            self.assertEqual(len(report["artifacts"]), 1)
            self.assertEqual(report["artifacts"][0]["language"], "swift")
        invalid = subprocess.run([os.fsencode(BINARY), b"--language", b"\xff"], capture_output=True)
        self.assertEqual(invalid.returncode, 2, invalid.stderr)


if __name__ == "__main__":
    unittest.main()
