#!/usr/bin/env python3
"""Validate the performance report and regression gate without running benchmarks."""
import unittest
from compare_performance import METRICS, SCENARIOS, compare, parse_sample, summarize


def samples(elapsed):
    return [
        {name: {metric: value if metric == "elapsed_ns" else 0 for metric in METRICS} for name in SCENARIOS}
        for value in elapsed
    ]


class PerformanceReportTests(unittest.TestCase):
    def test_large_stable_regression_is_rejected(self):
        result = compare(summarize(samples([100_000_000] * 7)), summarize(samples([200_000_000] * 7)))
        self.assertTrue(all(row["regression"] for row in result))

    def test_equal_or_faster_runs_pass(self):
        baseline = summarize(samples([100_000_000] * 7))
        for value in [100_000_000, 50_000_000]:
            self.assertFalse(any(row["regression"] for row in compare(baseline, summarize(samples([value] * 7)))))

    def test_sub_millisecond_noise_does_not_fail(self):
        result = compare(summarize(samples([100_000] * 7)), summarize(samples([300_000] * 7)))
        self.assertFalse(any(row["regression"] for row in result))

    def test_summary_preserves_samples_and_reports_median_p95_and_mad(self):
        summary = summarize(samples([1, 2, 3, 4, 5, 6, 7]))["source-tree"]["elapsed_ns"]
        self.assertEqual(summary, {"samples": [1, 2, 3, 4, 5, 6, 7], "median": 4, "p95": 7, "mad": 2})
        with self.assertRaises(ValueError):
            summarize(samples([1, 2]))

    def test_missing_scenarios_and_wrong_artifact_counts_are_rejected(self):
        header = "scenario,elapsed_ns,first_artifact_ns,cancellation_latency_ns,artifacts,policy_calls\n"
        with self.assertRaises(ValueError):
            parse_sample(header + "source-tree,100,20,0,16,16\n")
        with self.assertRaises(ValueError):
            parse_sample(header + "many-roots,100,20,0,1,1\n")


if __name__ == "__main__":
    unittest.main()
