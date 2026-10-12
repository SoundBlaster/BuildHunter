import copy
import unittest
from compare_backends import summary, validate_pair, resource_metrics
from profile_resources import validate_pair as validate_profile_pair


class BackendComparisonTests(unittest.TestCase):
    def test_profile_overhead_requires_report_parity_and_real_resource_samples(self):
        enabled = self.fixture()
        enabled["resources"] = {"scope": "process", "cpu_user_ns": 300, "cpu_system_ns": 100,
                                "average_cpu_cores": 4, "peak_cpu_cores": 5,
                                "average_rss_bytes": 200, "sampled_peak_rss_bytes": 300}
        enabled["resource_samples"] = [
            {"elapsed_us": 0, "cpu_user_ns": 0, "cpu_system_ns": 0, "resident_bytes": 100},
            {"elapsed_us": 1, "cpu_user_ns": 300, "cpu_system_ns": 100, "resident_bytes": 300}]
        disabled = copy.deepcopy(enabled)
        disabled["resource_samples"] = []
        disabled["resources"]["cpu_user_ns"] = None
        validate_profile_pair(enabled, disabled)
        for key, value in (("elapsed_us", 0), ("cpu_user_ns", None), ("resident_bytes", None)):
            broken = copy.deepcopy(enabled)
            broken["resource_samples"][-1][key] = value
            with self.assertRaises(ValueError):
                validate_profile_pair(broken, disabled)
        broken = copy.deepcopy(disabled)
        broken["report"]["rows"][0][3] = 1
        with self.assertRaises(ValueError):
            validate_profile_pair(enabled, broken)
        with self.assertRaises(ValueError):
            validate_profile_pair(enabled, enabled)

    def test_process_metrics_preserve_parallel_cpu_and_reject_missing_readings(self):
        data = {"resources": {"scope": "process", "cpu_user_ns": 300, "cpu_system_ns": 100,
                              "average_cpu_cores": 4, "peak_cpu_cores": 5,
                              "average_rss_bytes": 200, "sampled_peak_rss_bytes": 300}}
        self.assertEqual(resource_metrics(data)["cpu_total_ns"], 400)
        self.assertEqual(resource_metrics(data)["average_cpu_cores"], 4)
        for invalid in (None, -1, float("nan"), float("inf"), True):
            broken = copy.deepcopy(data)
            broken["resources"]["average_rss_bytes"] = invalid
            with self.assertRaises(ValueError):
                resource_metrics(broken)

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
