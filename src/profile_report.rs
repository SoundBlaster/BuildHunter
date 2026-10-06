use build_hunter::ScanProfileSample;
use std::collections::VecDeque;

const CAPACITY: usize = 4096;

#[derive(Default)]
pub struct ProfileReport {
    samples: VecDeque<ScanProfileSample>,
    dropped: u64,
    peak_entries: f64,
    peak_bytes: f64,
}

impl ProfileReport {
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
            "{{\"elapsed_us\":{},\"entries\":{},\"directories\":{},\"measured_bytes\":{},\"artifacts\":{},\"warnings\":{},\"pending_tasks\":{}}}",
            s.elapsed_us, s.entries, s.directories, s.measured_bytes, s.artifacts, s.warnings, s.pending_tasks
        )).collect::<Vec<_>>().join(",");
        format!(
            "{{\"schema_version\":1,\"root\":{},\"status\":{},\"size_mode\":{},\"sample_interval_ms\":250,\"truncated_samples\":{},\"average_entries_per_second\":{},\"average_measured_bytes_per_second\":{},\"peak_entries_per_second\":{},\"peak_measured_bytes_per_second\":{},\"samples\":[{}]}}\n",
            crate::quoted(root),
            crate::quoted(status),
            crate::quoted(size_mode),
            self.dropped,
            rate(last.entries),
            rate(last.measured_bytes),
            self.peak_entries,
            self.peak_bytes,
            samples
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
    }

    #[test]
    fn bounded_history_keeps_whole_scan_mean_and_peak() {
        let mut profile = ProfileReport::default();
        for i in 0..5000 {
            profile.record(ScanProfileSample {
                elapsed_us: i * 1_000_000,
                entries: i * 10,
                measured_bytes: i * 100,
                ..Default::default()
            });
        }
        assert_eq!(profile.samples.len(), CAPACITY);
        assert_eq!(profile.dropped, 904);
        assert_eq!(profile.peak_entries, 10.0);
        assert!(
            profile
                .json("root", "completed", "allocated")
                .contains("\"average_entries_per_second\":10")
        );
    }
}
