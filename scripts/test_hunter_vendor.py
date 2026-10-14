"""Regression: modified upstream code cannot pass the offline identity check."""
from pathlib import Path
import os
import shutil
import tempfile
import unittest
from verify_hunter_vendor import verify

ROOT = Path(__file__).resolve().parents[1] / "vendor/hunter-fs-core"


class VendorTests(unittest.TestCase):
    def test_real_snapshot_passes(self):
        self.assertEqual(len(verify(ROOT)), 40)

    def test_modified_source_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / "snapshot"
            shutil.copytree(ROOT, copy)
            source = copy / "src/lib.rs"
            source.write_bytes(source.read_bytes() + b"\n// changed\n")
            with self.assertRaisesRegex(ValueError, "differs from pinned content"):
                verify(copy)

    def test_extra_cargo_inputs_are_rejected(self):
        for name in ["build.rs", "src/hidden.rs", "examples/hidden.rs"]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as directory:
                copy = Path(directory) / "snapshot"
                shutil.copytree(ROOT, copy)
                extra = copy / name
                extra.parent.mkdir(parents=True, exist_ok=True)
                extra.write_text("fn main() {}\n")
                with self.assertRaisesRegex(ValueError, "Unexpected Hunter snapshot"):
                    verify(copy)

    @unittest.skipIf(os.name == "nt", "Windows symlink permissions vary")
    def test_symlinked_source_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            copy = Path(directory) / "snapshot"
            shutil.copytree(ROOT, copy)
            source = copy / "src/lib.rs"
            outside = Path(directory) / "outside.rs"
            outside.write_bytes(source.read_bytes())
            source.unlink()
            source.symlink_to(outside)
            with self.assertRaisesRegex(ValueError, "symlink"):
                verify(copy)


if __name__ == "__main__":
    unittest.main()
