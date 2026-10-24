//! Host-selected per-scan retention budgets. Weights estimate owned payloads,
//! not allocator overhead, an RSS ceiling, filesystem snapshots or consumer memory.
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};

#[derive(Clone, Copy, Debug)]
pub struct ScanLimits {
    pub queued_tasks: usize,
    pub queued_weight: usize,
    pub response_capacity: usize,
    pub listing_entries: usize,
    pub listing_errors: usize,
    pub listing_weight: usize,
    pub findings: usize,
    pub findings_weight: usize,
    pub warnings: usize,
    pub warning_weight: usize,
    pub diagnostics: usize,
    pub diagnostic_weight: usize,
    pub profile_samples: usize,
}
impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            queued_tasks: 65_536,
            queued_weight: 32 * 1024 * 1024,
            response_capacity: 8,
            listing_entries: 65_536,
            listing_errors: 4096,
            listing_weight: 8 * 1024 * 1024,
            findings: 100_000,
            findings_weight: 32 * 1024 * 1024,
            warnings: 4096,
            warning_weight: 2 * 1024 * 1024,
            diagnostics: 4096,
            diagnostic_weight: 4 * 1024 * 1024,
            profile_samples: 4096,
        }
    }
}
impl ScanLimits {
    pub(crate) fn validate(self) {
        assert!(
            self.queued_tasks > 0 && self.queued_weight > 0 && self.profile_samples > 0,
            "queue and profile budgets must be positive"
        );
    }
    pub(crate) fn listing(self) -> hunter_fs_core::ListingLimits {
        hunter_fs_core::ListingLimits {
            max_entries: self.listing_entries,
            max_errors: self.listing_errors,
            max_weight: self.listing_weight,
        }
    }
    pub fn json(self) -> serde_json::Value {
        serde_json::json!({"queuedTasks":self.queued_tasks,"queuedWeight":self.queued_weight,
            "responseCapacity":self.response_capacity,"listingEntries":self.listing_entries,
            "listingErrors":self.listing_errors,"listingWeight":self.listing_weight,
            "findings":self.findings,"findingsWeight":self.findings_weight,
            "warnings":self.warnings,"warningWeight":self.warning_weight,
            "diagnostics":self.diagnostics,"diagnosticWeight":self.diagnostic_weight,
            "profileSamples":self.profile_samples})
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanLimit {
    QueuedTasks = 1,
    QueuedWeight,
    ListingEntries,
    ListingErrors,
    ListingWeight,
    Findings,
    FindingsWeight,
    Warnings,
    WarningWeight,
    Diagnostics,
    DiagnosticWeight,
    MetadataEntries,
    MetadataWeight,
}
impl ScanLimit {
    pub fn name(self) -> &'static str {
        match self {
            Self::QueuedTasks => "queued task count",
            Self::QueuedWeight => "queued task payload",
            Self::ListingEntries => "directory entry count",
            Self::ListingErrors => "directory error count",
            Self::ListingWeight => "directory listing payload",
            Self::Findings => "finding count",
            Self::FindingsWeight => "finding/progress payload",
            Self::Warnings => "warning count",
            Self::WarningWeight => "warning payload",
            Self::Diagnostics => "diagnostic count",
            Self::DiagnosticWeight => "diagnostic payload",
            Self::MetadataEntries => "batch metadata count",
            Self::MetadataWeight => "batch metadata payload",
        }
    }
    fn from_code(value: u8) -> Option<Self> {
        Some(match value {
            1 => Self::QueuedTasks,
            2 => Self::QueuedWeight,
            3 => Self::ListingEntries,
            4 => Self::ListingErrors,
            5 => Self::ListingWeight,
            6 => Self::Findings,
            7 => Self::FindingsWeight,
            8 => Self::Warnings,
            9 => Self::WarningWeight,
            10 => Self::Diagnostics,
            11 => Self::DiagnosticWeight,
            12 => Self::MetadataEntries,
            13 => Self::MetadataWeight,
            _ => return None,
        })
    }
}

#[derive(Default)]
pub(crate) struct Usage {
    exhausted: AtomicU8,
    pub peak_pending_tasks: AtomicU64,
    pub peak_listing_entries: AtomicU64,
    pub peak_listing_weight: AtomicU64,
    pub dropped_profile_samples: AtomicU64,
}
impl Usage {
    pub fn stop(&self, reason: ScanLimit) {
        let _ =
            self.exhausted
                .compare_exchange(0, reason as u8, Ordering::AcqRel, Ordering::Acquire);
    }
    pub fn limit(&self) -> Option<ScanLimit> {
        ScanLimit::from_code(self.exhausted.load(Ordering::Acquire))
    }
    pub fn json(&self, limits: ScanLimits) -> serde_json::Value {
        serde_json::json!({"limits":limits.json(),"limitReached":self.limit().map(ScanLimit::name),
            "peakPendingTasks":self.peak_pending_tasks.load(Ordering::Relaxed),
            "peakListingEntries":self.peak_listing_entries.load(Ordering::Relaxed),
            "peakListingWeight":self.peak_listing_weight.load(Ordering::Relaxed),
            "droppedProfileSamples":self.dropped_profile_samples.load(Ordering::Relaxed)})
    }
}

/// Serial retention gate. An over-budget payload is rejected before cloning/emitting.
#[derive(Default)]
pub(crate) struct Retention {
    pub count: usize,
    pub weight: usize,
}
impl Retention {
    pub fn admit(
        &mut self,
        weight: usize,
        count_limit: usize,
        weight_limit: usize,
        count_reason: ScanLimit,
        weight_reason: ScanLimit,
    ) -> Result<(), ScanLimit> {
        if self.count >= count_limit {
            return Err(count_reason);
        }
        if weight > weight_limit.saturating_sub(self.weight) {
            return Err(weight_reason);
        }
        self.count += 1;
        self.weight += weight;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejected_retention_never_consumes_or_overflows_budget() {
        let mut budget = Retention::default();
        assert_eq!(
            budget.admit(4, 2, 5, ScanLimit::Findings, ScanLimit::FindingsWeight),
            Ok(())
        );
        assert_eq!(
            budget.admit(
                usize::MAX,
                2,
                5,
                ScanLimit::Findings,
                ScanLimit::FindingsWeight
            ),
            Err(ScanLimit::FindingsWeight)
        );
        assert_eq!((budget.count, budget.weight), (1, 4));
        assert_eq!(
            budget.admit(1, 2, 5, ScanLimit::Findings, ScanLimit::FindingsWeight),
            Ok(())
        );
        assert_eq!(
            budget.admit(0, 2, 5, ScanLimit::Findings, ScanLimit::FindingsWeight),
            Err(ScanLimit::Findings)
        );
    }
    #[test]
    fn first_limit_reason_is_stable_across_workers() {
        let usage = Usage::default();
        usage.stop(ScanLimit::ListingWeight);
        usage.stop(ScanLimit::QueuedTasks);
        assert_eq!(usage.limit(), Some(ScanLimit::ListingWeight));
    }
}
