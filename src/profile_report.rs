use build_hunter::{PROFILE_INTERVAL, ResourceHistory, ScanProfileSample};
use std::collections::VecDeque;

const CAPACITY: usize = 4096;

#[derive(Default)]
pub struct ProfileReport {
    samples: VecDeque<ScanProfileSample>,
    dropped: u64,
    peak_entries: f64,
    peak_bytes: f64,
    resources: ResourceHistory,
    resource_usage: Option<serde_json::Value>,
    first_finding_us: Option<u64>,
}

impl ProfileReport {
    pub fn set_resource_usage(&mut self, usage: serde_json::Value) {
        self.resource_usage = Some(usage);
    }

    pub fn record_first_finding(&mut self, elapsed: std::time::Duration) {
        self.first_finding_us
            .get_or_insert(u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX));
    }

    pub fn record(&mut self, sample: ScanProfileSample) {
        if let Some(previous) = self.samples.back() {
            if sample.elapsed_us <= previous.elapsed_us {
                return;
            }
            let seconds = (sample.elapsed_us - previous.elapsed_us) as f64 / 1_000_000.0;
            self.peak_entries = self
                .peak_entries
                .max(sample.entries.saturating_sub(previous.entries) as f64 / seconds);
            self.peak_bytes = self.peak_bytes.max(
                sample
                    .measured_bytes
                    .saturating_sub(previous.measured_bytes) as f64
                    / seconds,
            );
        }
        self.resources.record(sample);
        if self.samples.len() == CAPACITY {
            self.samples.pop_front();
            self.dropped += 1;
        }
        self.samples.push_back(sample);
    }

    pub fn json(&self, root: &str, status: &str, size_mode: &str) -> String {
        let last = self.samples.back().copied().unwrap_or_default();
        let seconds = last.elapsed_us as f64 / 1_000_000.0;
        let rate = |count: u64| {
            if seconds > 0.0 {
                count as f64 / seconds
            } else {
                0.0
            }
        };
        let samples = self.samples.iter().map(|s| format!(
            "{{\"elapsed_us\":{},\"entries\":{},\"directories\":{},\"measured_bytes\":{},\"artifacts\":{},\"warnings\":{},\"pending_tasks\":{},{}}}",
            s.elapsed_us, s.entries, s.directories, s.measured_bytes, s.artifacts, s.warnings, s.pending_tasks, s.resource_json_fields()
        )).collect::<Vec<_>>().join(",");
        let pending_dropped = self
            .resource_usage
            .as_ref()
            .and_then(|usage| usage["droppedProfileSamples"].as_u64())
            .unwrap_or(0);
        let budget_json = self
            .resource_usage
            .as_ref()
            .map_or_else(|| "null".to_owned(), serde_json::Value::to_string);
        format!(
            "{{\"schema_version\":1,\"root\":{},\"status\":{},\"size_mode\":{},\"sample_interval_ms\":{},\"first_finding_elapsed_us\":{},\"truncated_samples\":{},\"pending_truncated_samples\":{},\"scan_budgets\":{},\"average_entries_per_second\":{},\"average_measured_bytes_per_second\":{},\"peak_entries_per_second\":{},\"peak_measured_bytes_per_second\":{},\"resources\":{},\"samples\":[{}]}}\n",
            crate::quoted(root),
            crate::quoted(status),
            crate::quoted(size_mode),
            PROFILE_INTERVAL.as_millis(),
            self.first_finding_us
                .map_or_else(|| "null".to_owned(), |value| value.to_string()),
            self.dropped.saturating_add(pending_dropped),
            pending_dropped,
            budget_json,
            rate(last.entries),
            rate(last.measured_bytes),
            self.peak_entries,
            self.peak_bytes,
            self.resources.summary().json(),
            samples
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pending_losses_and_display_evictions_are_accounted_separately() {
        let mut profile = ProfileReport::default();
        for i in 0..5000 {
            profile.record(ScanProfileSample {
                elapsed_us: i + 1,
                ..Default::default()
            });
        }
        profile.set_resource_usage(serde_json::json!({"droppedProfileSamples":17}));
        let value: serde_json::Value =
            serde_json::from_str(&profile.json("root", "completed", "apparent")).unwrap();
        assert_eq!(value["truncated_samples"], 921);
        assert_eq!(value["pending_truncated_samples"], 17);
        assert_eq!(value["scan_budgets"]["droppedProfileSamples"], 17);
        assert_eq!(value["samples"].as_array().unwrap().len(), CAPACITY);
    }

    #[test]
    fn zero_elapsed_and_duplicate_samples_never_emit_nonfinite_rates() {
        let mut profile = ProfileReport::default();
        profile.record(ScanProfileSample::default());
        profile.record(ScanProfileSample::default());
        let json = profile.json("a\n\"b", "cancelled", "apparent");
        assert_eq!(profile.samples.len(), 1);
        assert!(json.contains("\"average_entries_per_second\":0"));
        assert!(!json.contains("NaN"));
        assert!(!json.contains("inf"));
        assert!(json.contains("a\\u000a\\\"b"));
        assert!(json.contains("\"resident_bytes\":null"));
        assert!(json.contains("\"average_cpu_cores\":null"));
    }

    #[test]
    fn bounded_history_keeps_whole_scan_mean_and_peak() {
        let mut profile = ProfileReport::default();
        for i in 0..5000 {
            profile.record(ScanProfileSample {
                elapsed_us: i * 1_000_000,
                entries: i * 10,
                measured_bytes: i * 100,
                cpu_user_ns: i * 1_000_000_000,
                resident_bytes: if i == 1 { 9999 } else { 100 },
                resource_flags: 3,
                ..Default::default()
            });
        }
        assert_eq!(profile.samples.len(), CAPACITY);
        assert_eq!(profile.dropped, 904);
        assert_eq!(profile.peak_entries, 10.0);
        assert_eq!(
            profile.resources.summary().sampled_peak_rss_bytes,
            Some(9999)
        );
        assert_eq!(profile.resources.summary().average_cpu_cores, Some(1.0));
        assert!(
            profile
                .json("root", "completed", "allocated")
                .contains("\"average_entries_per_second\":10")
        );
    }
}
