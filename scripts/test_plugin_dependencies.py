"""Regression coverage for duplicate-core and stale-pin dependency graphs."""
import copy
from pathlib import Path
import unittest

from verify_plugin_dependencies import HUNTER, RIFLE, verify

ROOT = Path("/checkout").resolve()
H = "a" * 40
R = "b" * 40


def graph(kind="scanner", unix=True):
    packages = [
        {"id": "root", "name": kind, "source": None, "manifest_path": str(ROOT / "Cargo.toml")},
        {"id": "core", "name": "hunter-fs-core", "source": None,
         "manifest_path": str(ROOT / "vendor/hunter-fs-core/Cargo.toml")},
        {"id": "facade", "name": "hunter-fs", "source": f"git+{HUNTER}?rev={H}#{H}"},
        {"id": "rifle", "name": "rifle-rules", "source": f"git+{RIFLE}?rev={R}#{R}"},
    ]
    deps = {"default": ["core", "rifle"] + (["facade"] if unix else []), "policy": ["rifle"],
            "scanner": ["core", "rifle"] + (["facade"] if unix else [])}[kind]
    nodes = [{"id": "root", "deps": [{"pkg": p} for p in deps]}]
    nodes += [{"id": p, "deps": ([{"pkg": "core"}] if p == "facade" else [])}
              for p in ["core", "facade", "rifle"]]
    return {"packages": packages, "resolve": {"root": "root", "nodes": nodes}}


class GraphTests(unittest.TestCase):
    def verify(self, value, kind="scanner", unix=True):
        return verify(value, ROOT, kind, unix, H, R)

    def test_expected_default_policy_and_unix_scanner_graphs(self):
        for kind in ["default", "policy", "scanner"]:
            with self.subTest(kind=kind):
                self.verify(graph(kind), kind)

    def test_windows_scanner_needs_no_unix_facade(self):
        result = self.verify(graph(unix=False), unix=False)
        self.assertNotIn("hunter-fs", result)
        self.assertIn("hunter-fs-core", result)

    def test_windows_default_keeps_catalog_without_a_unix_facade(self):
        result = self.verify(graph("default", unix=False), "default", unix=False)
        self.assertIn("rifle-rules", result)
        self.assertNotIn("hunter-fs", result)

    def test_rejects_second_reachable_core(self):
        value = graph()
        duplicate = copy.deepcopy(value["packages"][1])
        duplicate.update(id="git-core", source=f"git+{HUNTER}?rev={H}#{H}")
        value["packages"].append(duplicate)
        value["resolve"]["nodes"].append({"id": "git-core", "deps": []})
        value["resolve"]["nodes"][2]["deps"] = [{"pkg": "git-core"}]
        with self.assertRaisesRegex(ValueError, "expected 1 active hunter-fs-core, got 2"):
            self.verify(value)

    def test_rejects_wrong_local_core_even_with_only_one_copy(self):
        value = graph()
        value["packages"][1]["manifest_path"] = str(ROOT / "unverified/Cargo.toml")
        with self.assertRaisesRegex(ValueError, "verified vendored"):
            self.verify(value)

    def test_rejects_unpatched_git_core(self):
        value = graph()
        value["packages"][1]["source"] = f"git+{HUNTER}?rev={H}#{H}"
        with self.assertRaisesRegex(ValueError, "verified vendored"):
            self.verify(value)

    def test_rejects_stale_facade_and_rifle(self):
        for index in [2, 3]:
            value = graph()
            value["packages"][index]["source"] += "stale"
            with self.subTest(index=index), self.assertRaisesRegex(ValueError, "unexpected Git"):
                self.verify(value)

    def test_policy_must_not_pull_in_hunter(self):
        value = graph("policy")
        value["resolve"]["nodes"][0]["deps"].append({"pkg": "core"})
        with self.assertRaisesRegex(ValueError, "expected 0 active hunter-fs-core"):
            self.verify(value, "policy")

    def test_missing_resolved_package_is_rejected(self):
        value = graph()
        value["resolve"]["nodes"][0]["deps"].append({"pkg": "missing"})
        with self.assertRaisesRegex(ValueError, "Incomplete Cargo"):
            self.verify(value)


if __name__ == "__main__":
    unittest.main()
