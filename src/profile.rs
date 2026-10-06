//! Cumulative coordinator progress, sampled on its own thread so stalls in the coordinator or
//! in the event consumer cannot thin the timeline. Bytes are metadata sizes, never disk reads.
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

pub const PROFILE_INTERVAL: Duration = Duration::from_millis(100);

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

/// Written only by the coordinator thread and read by the sampler.
#[derive(Default)]
pub(crate) struct ProfileCounters {
    pub entries: AtomicU64,
    pub directories: AtomicU64,
    pub measured_bytes: AtomicU64,
    pub artifacts: AtomicU64,
    pub warnings: AtomicU64,
    pub pending_tasks: AtomicU64,
}

/// Single-writer increment: a plain load and store, no read-modify-write on the hot path.
pub(crate) fn bump(counter: &AtomicU64, amount: u64) {
    counter.store(
        counter.load(Ordering::Relaxed).saturating_add(amount),
        Ordering::Relaxed,
    );
}

/// Samples waiting for the coordinator. Unbounded on purpose: dropping here would hide
/// losses from the report's truncation count, and a stall costs only 56 bytes per interval.
#[derive(Default)]
struct Pending {
    samples: VecDeque<ScanProfileSample>,
    last_elapsed_us: Option<u64>,
}

pub(crate) struct ProfileState {
    started: Instant,
    pub counters: ProfileCounters,
    pending: Mutex<Pending>,
    has_pending: AtomicBool,
}

impl ProfileState {
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            counters: ProfileCounters::default(),
            pending: Mutex::new(Pending::default()),
            has_pending: AtomicBool::new(false),
        }
    }

    /// Queues a snapshot of the counters taken now. The clock is read under the queue lock,
    /// so queued samples are strictly increasing in time whichever thread records them.
    pub fn record(&self) {
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        let now = u64::try_from(self.started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let elapsed_us = match pending.last_elapsed_us {
            Some(last) if now <= last => last.saturating_add(1),
            _ => now,
        };
        pending.last_elapsed_us = Some(elapsed_us);
        let counters = &self.counters;
        let sample = ScanProfileSample {
            elapsed_us,
            entries: counters.entries.load(Ordering::Relaxed),
            directories: counters.directories.load(Ordering::Relaxed),
            measured_bytes: counters.measured_bytes.load(Ordering::Relaxed),
            artifacts: counters.artifacts.load(Ordering::Relaxed),
            warnings: counters.warnings.load(Ordering::Relaxed),
            pending_tasks: counters.pending_tasks.load(Ordering::Relaxed),
        };
        pending.samples.push_back(sample);
        self.has_pending.store(true, Ordering::Release);
    }

    /// Takes queued samples in time order. Cheap when nothing is queued.
    pub fn drain(&self) -> VecDeque<ScanProfileSample> {
        if !self.has_pending.load(Ordering::Acquire) {
            return VecDeque::new();
        }
        let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
        self.has_pending.store(false, Ordering::Release);
        std::mem::take(&mut pending.samples)
    }

    /// Records one sample per interval boundary until `stop` is dropped or signalled.
    /// Ticks missed while the thread was descheduled are skipped, never replayed.
    pub fn run_sampler(&self, stop: &mpsc::Receiver<()>) {
        let mut tick: u32 = 1;
        loop {
            let deadline = PROFILE_INTERVAL.saturating_mul(tick);
            let wait = deadline.saturating_sub(self.started.elapsed());
            match stop.recv_timeout(wait) {
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => return,
            }
            self.record();
            let elapsed_ticks = self.started.elapsed().as_nanos() / PROFILE_INTERVAL.as_nanos();
            tick = u32::try_from(elapsed_ticks)
                .unwrap_or(u32::MAX - 1)
                .saturating_add(1)
                .max(tick.saturating_add(1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_from_any_thread_stay_strictly_ordered() {
        let state = ProfileState::new();
        bump(&state.counters.entries, 3);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    for _ in 0..50 {
                        state.record();
                    }
                });
            }
        });
        let samples = state.drain();
        assert_eq!(samples.len(), 200);
        assert!(
            samples
                .iter()
                .zip(samples.iter().skip(1))
                .all(|(a, b)| a.elapsed_us < b.elapsed_us)
        );
        assert!(samples.iter().all(|sample| sample.entries == 3));
        assert!(state.drain().is_empty());
    }

    #[test]
    fn a_long_stall_loses_no_samples() {
        let state = ProfileState::new();
        for _ in 0..10_000 {
            state.record();
        }
        let samples = state.drain();
        assert_eq!(samples.len(), 10_000);
        state.record();
        assert!(state.drain()[0].elapsed_us > samples.back().unwrap().elapsed_us);
    }

    #[test]
    fn sampler_ticks_on_interval_boundaries_until_stopped() {
        let state = ProfileState::new();
        let (stop, signal) = mpsc::channel();
        std::thread::scope(|scope| {
            let state = &state;
            scope.spawn(move || state.run_sampler(&signal));
            std::thread::sleep(PROFILE_INTERVAL * 3 + PROFILE_INTERVAL / 2);
            stop.send(()).unwrap();
        });
        let samples = state.drain();
        assert!(
            (2..=4).contains(&samples.len()),
            "expected about three ticks, got {}",
            samples.len()
        );
        for sample in &samples {
            assert!(
                sample.elapsed_us >= 100_000,
                "no tick before the first interval"
            );
        }
        for (a, b) in samples.iter().zip(samples.iter().skip(1)) {
            assert!(
                b.elapsed_us - a.elapsed_us >= 90_000,
                "ticks are not replayed in a burst"
            );
        }
    }
}
