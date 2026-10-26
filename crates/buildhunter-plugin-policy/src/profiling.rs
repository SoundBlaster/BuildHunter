//! Bounded, scan-owned aggregate metrics. No clock calls or mutation in a catalog.
use rifle_rules::{Sha256Digest, Truth};
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RulePhase {
    Preliminary,
    Resolved,
}
impl RulePhase {
    fn label(self) -> &'static str {
        match self {
            Self::Preliminary => "preliminary",
            Self::Resolved => "resolved",
        }
    }
}

#[derive(Default, Debug)]
struct Counts {
    evaluations: u64,
    matched: u64,
    no_match: u64,
    unknown: u64,
    total_ns: u64,
    max_ns: u64,
}

/// At most two rows per enabled rule; excludes candidate/event history and paths.
#[derive(Debug)]
pub struct RuleProfile {
    digest: Sha256Digest,
    exclusions: BTreeSet<String>,
    calls: [u64; 2],
    errors: u64,
    rows: Vec<((String, String, String), [Counts; 2])>,
}
impl RuleProfile {
    pub(crate) fn new(digest: Sha256Digest, exclusions: BTreeSet<String>) -> Self {
        Self {
            digest,
            exclusions,
            calls: [0, 0],
            errors: 0,
            rows: Vec::new(),
        }
    }
    pub(crate) fn matches_catalog(
        &self,
        digest: &Sha256Digest,
        exclusions: &BTreeSet<String>,
    ) -> bool {
        &self.digest == digest && &self.exclusions == exclusions
    }
    pub(crate) fn register(&mut self, package: &str, target: &str, rule: &str) {
        self.rows.push((
            (package.into(), target.into(), rule.into()),
            [Counts::default(), Counts::default()],
        ));
    }
    pub(crate) fn record_call(&mut self, phase: RulePhase) {
        let calls = &mut self.calls[phase as usize];
        *calls = calls.saturating_add(1);
    }
    pub(crate) fn record_error(&mut self) {
        self.errors = self.errors.saturating_add(1);
    }
    pub(crate) fn record(
        &mut self,
        index: usize,
        phase: RulePhase,
        truth: Truth,
        elapsed: std::time::Duration,
    ) {
        let counts = &mut self.rows[index].1[phase as usize];
        counts.evaluations = counts.evaluations.saturating_add(1);
        let counter = match truth {
            Truth::True => &mut counts.matched,
            Truth::False => &mut counts.no_match,
            Truth::Unknown => &mut counts.unknown,
        };
        *counter = counter.saturating_add(1);
        let ns = u64::try_from(elapsed.as_nanos()).unwrap_or(u64::MAX);
        counts.total_ns = counts.total_ns.saturating_add(ns);
        counts.max_ns = counts.max_ns.max(ns);
    }
    pub fn json(&self) -> Value {
        let rows = self.rows.iter().flat_map(|((package, target, rule), counts)| [RulePhase::Preliminary, RulePhase::Resolved].into_iter().map(move |phase| {
            let c = &counts[phase as usize];
            json!({
            "package_id":package, "target_id":target, "rule_id":rule, "phase":phase.label(),
            "evaluations":c.evaluations, "skipped_by_dispatch":self.calls[phase as usize].saturating_sub(c.evaluations),
            "true":c.matched, "false":c.no_match, "unknown":c.unknown,
            "total_ns":c.total_ns, "max_ns":c.max_ns,
            "average_ns": if c.evaluations == 0 { None } else { Some(c.total_ns as f64 / c.evaluations as f64) },
        })})).collect::<Vec<_>>();
        json!({"format":"buildhunter.rule-profile/1", "catalog_digest":self.digest.to_string(), "excluded_target_ids":self.exclusions,
            "timing_scope":"rule_evaluation_and_outcome_wall_time",
            "preliminary_calls":self.calls[0], "resolved_calls":self.calls[1],
            "classification_errors":self.errors, "rules":rows})
    }
}
