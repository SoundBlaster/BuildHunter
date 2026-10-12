import copy
import unittest
from compare_backends import summary, validate_pair


class BackendComparisonTests(unittest.TestCase):
    def fixture(self):
        return {"report": {"rows": [["path", "swift", "build", 4096, False, False]],
                           "warnings": [], "status": "Completed"},
                "status": "Completed", "warnings": 0, "artifacts": 1,
                "first_artifact_ns": 1, "cancellation_latency_ns": 0,
                "bulk_directories": 1, "bulk_fallbacks": 0}

    def test_exact_report_equality_includes_sizes_partial_flags_and_paths(self):
        portable = self.fixture()
        validate_pair(portable, copy.deepcopy(portable))
        for column, value in ((0, "wrong-path"), (3, 8192), (5, True)):
            bulk = copy.deepcopy(portable)
            bulk["report"]["rows"][0][column] = value
            with self.assertRaises(ValueError):
                validate_pair(portable, bulk)

    def test_fallback_or_empty_scan_cannot_pass_as_batch_work(self):
        portable = self.fixture()
        for key, value in (("bulk_directories", 0), ("bulk_fallbacks", 1)):
            bulk = copy.deepcopy(portable)
            bulk[key] = value
            with self.assertRaises(ValueError):
                validate_pair(portable, bulk)
        portable["artifacts"] = 0
        with self.assertRaises(ValueError):
            validate_pair(portable, portable)

    def test_cancel_requires_matching_status_and_latency(self):
        portable = self.fixture()
        portable["status"] = portable["report"]["status"] = "Cancelled"
        portable["cancellation_latency_ns"] = 1
        validate_pair(portable, copy.deepcopy(portable), cancelled=True)
        portable["cancellation_latency_ns"] = 0
        with self.assertRaises(ValueError):
            validate_pair(portable, portable, cancelled=True)

    def test_statistics_preserve_raw_samples_and_noise(self):
        result = summary([1, 3, 3, 4, 9])
        self.assertEqual(result["median"], 3)
        self.assertEqual(result["p95"], 9)
        self.assertEqual(result["mad"], 1)
        with self.assertRaises(ValueError):
            summary([1, 2])


if __name__ == "__main__":
    unittest.main()
