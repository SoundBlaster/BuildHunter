//! Cumulative coordinator progress. Bytes are metadata sizes, never disk-read throughput.
use std::time::{Duration, Instant};

pub const PROFILE_INTERVAL: Duration = Duration::from_millis(250);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScanProfileSample {
    pub elapsed_us: u64,
    pub entries: u64,
    pub directories: u64,
    pub measured_bytes: u64,
    pub artifacts: u64,
    pub warnings: u64,
    pub pending_tasks: u64,
}

pub(crate) struct ProfileClock {
    started: Instant,
    last_emitted: Duration,
    pub counters: ScanProfileSample,
}

impl ProfileClock {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            last_emitted: Duration::ZERO,
            counters: ScanProfileSample::default(),
        }
    }

    pub fn sample(&mut self, force: bool, pending: usize) -> Option<ScanProfileSample> {
        self.sample_at(self.started.elapsed(), force, pending)
    }

    fn sample_at(
        &mut self,
        elapsed: Duration,
        force: bool,
        pending: usize,
    ) -> Option<ScanProfileSample> {
        if !force && elapsed.saturating_sub(self.last_emitted) < PROFILE_INTERVAL {
            return None;
        }
        self.last_emitted = elapsed;
        Some(ScanProfileSample {
            elapsed_us: u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX),
            pending_tasks: pending as u64,
            ..self.counters
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_and_forced_terminal_preserve_counters() {
        let mut clock = ProfileClock::new();
        clock.counters.entries = 9;
        assert!(
            clock
                .sample_at(Duration::from_millis(249), false, 3)
                .is_none()
        );
        let sample = clock
            .sample_at(Duration::from_millis(250), false, 3)
            .unwrap();
        assert_eq!(sample.entries, 9);
        assert_eq!(sample.pending_tasks, 3);
        assert!(
            clock
                .sample_at(Duration::from_millis(251), false, 2)
                .is_none()
        );
        let last = clock
            .sample_at(Duration::from_millis(251), true, 2)
            .unwrap();
        assert_eq!(last.elapsed_us, 251_000);
        assert_eq!(last.pending_tasks, 2);
    }
}
