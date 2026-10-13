//! Process resource accounting. CPU 1.0 means one fully occupied logical core.
use crate::ScanProfileSample;

pub(crate) const CPU_AVAILABLE: u64 = 1;
pub(crate) const RSS_AVAILABLE: u64 = 2;

impl ScanProfileSample {
    /// Nullable process readings for an enclosing JSON sample object.
    pub fn resource_json_fields(&self) -> String {
        format!(
            "\"cpu_user_ns\":{},\"cpu_system_ns\":{},\"resident_bytes\":{}",
            json_value(cpu(*self).map(|value| value.0)),
            json_value(cpu(*self).map(|value| value.1)),
            json_value(rss(*self))
        )
    }
}

fn json_value(number: Option<impl std::fmt::Display>) -> String {
    number.map_or_else(|| "null".into(), |number| number.to_string())
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ResourceSummary {
    pub cpu_user_ns: Option<u64>,
    pub cpu_system_ns: Option<u64>,
    pub average_cpu_cores: Option<f64>,
    pub peak_cpu_cores: Option<f64>,
    pub initial_rss_bytes: Option<u64>,
    pub current_rss_bytes: Option<u64>,
    pub sampled_peak_rss_bytes: Option<u64>,
    pub average_rss_bytes: Option<f64>,
    pub rss_observed_us: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ResourceHistory {
    summary: ResourceSummary,
    previous: Option<ScanProfileSample>,
    rss_integral: f64,
}

impl ResourceHistory {
    pub fn record(&mut self, mut sample: ScanProfileSample) {
        if self
            .previous
            .is_some_and(|previous| sample.elapsed_us <= previous.elapsed_us)
        {
            return;
        }
        if let Some(previous) = self.previous {
            if cpu(previous).is_some_and(|(user, system)| {
                sample.cpu_user_ns < user || sample.cpu_system_ns < system
            }) {
                sample.resource_flags &= !CPU_AVAILABLE;
            }
            let elapsed_us = sample.elapsed_us - previous.elapsed_us;
            if let (Some((user, system)), Some((previous_user, previous_system))) =
                (cpu(sample), cpu(previous))
            {
                let cores = (user.saturating_sub(previous_user) as f64
                    + system.saturating_sub(previous_system) as f64)
                    / (elapsed_us as f64 * 1000.0);
                self.summary.peak_cpu_cores =
                    Some(self.summary.peak_cpu_cores.unwrap_or(0.0).max(cores));
            }
            if let (Some(rss), Some(previous_rss)) = (rss(sample), rss(previous)) {
                self.rss_integral +=
                    (rss as f64 / 2.0 + previous_rss as f64 / 2.0) * elapsed_us as f64;
                self.summary.rss_observed_us =
                    self.summary.rss_observed_us.saturating_add(elapsed_us);
                self.summary.average_rss_bytes =
                    Some(self.rss_integral / self.summary.rss_observed_us as f64);
            }
        } else {
            self.summary.initial_rss_bytes = rss(sample);
        }
        self.summary.cpu_user_ns = cpu(sample).map(|value| value.0);
        self.summary.cpu_system_ns = cpu(sample).map(|value| value.1);
        self.summary.average_cpu_cores = cpu(sample).and_then(|(user, system)| {
            (sample.elapsed_us > 0)
                .then(|| (user as f64 + system as f64) / (sample.elapsed_us as f64 * 1000.0))
        });
        self.summary.current_rss_bytes = rss(sample);
        if let Some(rss) = rss(sample) {
            self.summary.sampled_peak_rss_bytes =
                Some(self.summary.sampled_peak_rss_bytes.unwrap_or(0).max(rss));
        }
        self.previous = Some(sample);
    }
    pub fn summary(&self) -> ResourceSummary {
        self.summary
    }
}

fn cpu(sample: ScanProfileSample) -> Option<(u64, u64)> {
    (sample.resource_flags & CPU_AVAILABLE != 0)
        .then_some((sample.cpu_user_ns, sample.cpu_system_ns))
}
fn rss(sample: ScanProfileSample) -> Option<u64> {
    (sample.resource_flags & RSS_AVAILABLE != 0).then_some(sample.resident_bytes)
}

impl ResourceSummary {
    /// Resource-only JSON used by CLI profiles and benchmark artifacts.
    pub fn json(&self) -> String {
        format!(
            "{{\"scope\":\"process\",\"cpu_user_ns\":{},\"cpu_system_ns\":{},\"average_cpu_cores\":{},\"peak_cpu_cores\":{},\"initial_rss_bytes\":{},\"current_rss_bytes\":{},\"sampled_peak_rss_bytes\":{},\"average_rss_bytes\":{},\"rss_observed_us\":{}}}",
            json_value(self.cpu_user_ns),
            json_value(self.cpu_system_ns),
            json_value(self.average_cpu_cores),
            json_value(self.peak_cpu_cores),
            json_value(self.initial_rss_bytes),
            json_value(self.current_rss_bytes),
            json_value(self.sampled_peak_rss_bytes),
            json_value(self.average_rss_bytes),
            self.rss_observed_us
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(seconds: u64, user: u64, system: u64, rss: u64) -> ScanProfileSample {
        ScanProfileSample {
            elapsed_us: seconds * 1_000_000,
            cpu_user_ns: user * 1_000_000_000,
            cpu_system_ns: system * 1_000_000_000,
            resident_bytes: rss,
            resource_flags: CPU_AVAILABLE | RSS_AVAILABLE,
            ..Default::default()
        }
    }

    #[test]
    fn cpu_is_not_clamped_to_one_core_and_ram_is_time_weighted() {
        let mut history = ResourceHistory::default();
        history.record(sample(0, 0, 0, 100));
        history.record(sample(1, 3, 1, 300));
        history.record(sample(3, 4, 2, 100));
        let result = history.summary();
        assert_eq!(result.cpu_user_ns, Some(4_000_000_000));
        assert_eq!(result.cpu_system_ns, Some(2_000_000_000));
        assert_eq!(result.average_cpu_cores, Some(2.0));
        assert_eq!(result.peak_cpu_cores, Some(4.0));
        assert_eq!(result.initial_rss_bytes, Some(100));
        assert_eq!(result.current_rss_bytes, Some(100));
        assert_eq!(result.sampled_peak_rss_bytes, Some(300));
        assert_eq!(result.average_rss_bytes, Some(200.0));
        assert_eq!(result.rss_observed_us, 3_000_000);
    }

    #[test]
    fn unavailable_readings_are_gaps_and_not_zeroes() {
        let mut history = ResourceHistory::default();
        history.record(sample(0, 0, 0, 100));
        history.record(ScanProfileSample {
            elapsed_us: 1_000_000,
            ..Default::default()
        });
        assert_eq!(history.summary().current_rss_bytes, None);
        assert_eq!(history.summary().cpu_user_ns, None);
        history.record(sample(2, 2, 0, 200));
        assert_eq!(history.summary().average_rss_bytes, None);
        assert_eq!(history.summary().peak_cpu_cores, None);
        history.record(sample(3, 3, 0, 400));
        let before = history.summary();
        history.record(sample(3, 100, 100, 9999));
        history.record(sample(1, 100, 100, 9999));
        assert_eq!(history.summary().sampled_peak_rss_bytes, Some(400));
        assert_eq!(before.average_rss_bytes, Some(300.0));
        assert_eq!(before.rss_observed_us, 1_000_000);
        assert_eq!(before.peak_cpu_cores, Some(1.0));
    }

    #[test]
    fn decreasing_cpu_counters_are_unavailable_not_an_underflow() {
        let mut history = ResourceHistory::default();
        history.record(sample(0, 0, 0, 100));
        history.record(sample(1, 4, 1, 200));
        history.record(sample(2, 3, 2, 100));
        assert_eq!(history.summary().cpu_user_ns, None);
        assert_eq!(history.summary().peak_cpu_cores, Some(5.0));
        assert_eq!(history.summary().sampled_peak_rss_bytes, Some(200));
    }
}
