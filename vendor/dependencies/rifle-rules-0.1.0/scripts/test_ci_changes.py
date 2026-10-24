import subprocess
from pathlib import Path
import tempfile
import unittest
from ci_changes import changed_paths, requires_heavy


class ChangeGateTests(unittest.TestCase):
    def test_docs_and_contracts(self):
        self.assertFalse(requires_heavy(["README.md", "docs/a.md", "docs/image.png",
            "specs/a.spec.yaml", "specpm.yaml"]))

    def test_code_configuration_and_unknown_paths(self):
        for path in ("src/lib.rs", "macos/Package.swift", "Cargo.lock", "Cargo.toml",
                     ".github/workflows/ci.yml", "scripts/ci/changes.py", "new/build.input",
                     "src/embedded.md", "schemas/program.schema.json"):
            with self.subTest(path=path):
                self.assertTrue(requires_heavy(["docs/a.md", path]))

    def test_manual_missing_and_first_push(self):
        for event, base in (("workflow_dispatch", "abc"), ("push", "0" * 40),
                            ("pull_request", ""), ("unknown", "abc")):
            self.assertIsNone(changed_paths(event, base, "def"))

    def test_unavailable_diff_is_conservative(self):
        def fail(*args, **kwargs):
            raise subprocess.CalledProcessError(128, "git")
        self.assertIsNone(changed_paths("push", "abc", "def", fail))

    def test_full_pr_diff_and_rename_consider_old_path(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            def git(*args):
                return subprocess.check_output(["git", *args], cwd=root).decode().strip()
            git("init", "-q")
            git("config", "user.name", "Test")
            git("config", "user.email", "test@example.invalid")
            (root / "src").mkdir()
            (root / "src/code.rs").write_text("original")
            git("add", "."); git("commit", "-qm", "base")
            base = git("rev-parse", "HEAD")
            (root / "src/code.rs").write_text("changed")
            git("commit", "-qam", "code")
            (root / "docs").mkdir()
            (root / "docs/a.md").write_text("docs")
            git("add", "."); git("commit", "-qm", "docs")
            def run(args, **kwargs):
                return subprocess.run(args, cwd=root, **kwargs)
            paths = changed_paths("pull_request", base, git("rev-parse", "HEAD"), run)
            self.assertTrue(requires_heavy(paths))
            before = git("rev-parse", "HEAD")
            git("mv", "src/code.rs", "docs/code.rs")
            git("commit", "-qm", "rename")
            paths = changed_paths("push", before, git("rev-parse", "HEAD"), run)
            self.assertIn("src/code.rs", paths)
            self.assertTrue(requires_heavy(paths))
            before = git("rev-parse", "HEAD")
            git("rm", "docs/a.md"); git("commit", "-qm", "delete doc")
            self.assertFalse(requires_heavy(changed_paths("push", before, "HEAD", run)))


if __name__ == "__main__":
    unittest.main()
