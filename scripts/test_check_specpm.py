import json
import unittest
from pathlib import Path

from check_specpm import check


class SpecPMBaselineTests(unittest.TestCase):
    def test_regression_policy(self):
        baseline = json.loads((Path(__file__).resolve().parents[1] / "specs/specpm-warning-baseline.json").read_text())
        report = {"errors": [], "error_count": 0, "warnings": baseline, "warning_count": len(baseline)}
        self.assertEqual(check(report, baseline), [])
        self.assertEqual(check({**report, "warnings": [], "warning_count": 0}, baseline), [])
        self.assertTrue(check({**report, "errors": [{"code": "error"}], "error_count": 1}, baseline))
        changed = [*baseline[:-1], {"code": "new-warning"}]
        self.assertTrue(check({**report, "warnings": changed}, baseline))
        duplicate = [*baseline, baseline[0]]
        self.assertTrue(check({**report, "warnings": duplicate, "warning_count": len(duplicate)}, baseline))
        unavailable = [{"code": "dependency_registry_unavailable"}]
        self.assertTrue(check({**report, "warnings": unavailable, "warning_count": 1}, baseline))
        self.assertTrue(check({"errors": [], "warnings": []}, baseline))


if __name__ == "__main__":
    unittest.main()
