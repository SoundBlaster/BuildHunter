//! Catalog decisions with the versioned, legacy-compatible CLI/GUI acquisition profile.
//! The descriptor-bound plugin pilot remains a separate stronger acquisition API.
use crate::{
    ArtifactClassificationCode, CandidateAction, CandidateNodeKind, ScanControl, ScanEvent,
    ScanOptions, ScanProfileSample, ScanStatus, ScanWarning,
    catalog::{CatalogSnapshot, ClientProfile},
};
use buildhunter_plugin_policy::{
    CandidateFacts, Classification, Decision, MarkerFact, MarkerProbe, NodeKind, RulePhase,
    RuleProfile, Scope, TargetDescriptor,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

impl From<CandidateNodeKind> for NodeKind {
    fn from(kind: CandidateNodeKind) -> Self {
        match kind {
            CandidateNodeKind::Directory => Self::Directory,
            CandidateNodeKind::RegularFile => Self::RegularFile,
            CandidateNodeKind::Symlink => Self::Symlink,
            CandidateNodeKind::Other => Self::Other,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CatalogArtifact {
    pub id: u64,
    pub relative_path: PathBuf,
    pub target: Arc<TargetDescriptor>,
    /// None for a symlink whose target was not measured.
    pub bytes: Option<u64>,
    pub nested: bool,
    pub partial: bool,
    pub is_symlink: bool,
}

#[derive(Clone, Debug)]
pub struct CatalogDiagnostic {
    pub relative_path: PathBuf,
    pub classification: Classification,
}

#[derive(Clone, Debug)]
pub enum CatalogScanEvent {
    ArtifactDiscovered(CatalogArtifact),
    SymlinkDiscovered(CatalogArtifact),
    ArtifactCompleted {
        id: u64,
        bytes: u64,
        partial: bool,
    },
    Diagnostic(CatalogDiagnostic),
    Warning(ScanWarning),
    Profile(ScanProfileSample),
    /// One bounded aggregate, emitted before Finished; never a per-candidate stream.
    RuleProfile(serde_json::Value),
    Finished(ScanStatus),
}

pub struct CatalogScanReport {
    pub catalog_digest: String,
    pub profile: ClientProfile,
    pub exclusions: BTreeSet<String>,
    pub artifacts: Vec<CatalogArtifact>,
    pub warnings: Vec<ScanWarning>,
    pub diagnostics: Vec<CatalogDiagnostic>,
    pub status: ScanStatus,
    pub marker_observations: u64,
    pub rule_profile: Option<serde_json::Value>,
}

impl CatalogScanReport {
    pub fn total_bytes(&self) -> u64 {
        self.artifacts
            .iter()
            .filter(|a| !a.nested && !a.is_symlink)
            .filter_map(|a| a.bytes)
            .fold(0u64, u64::saturating_add)
    }
}

/// Scan with one immutable snapshot. Dynamic marker requests and every-node
/// observation replace the fixed candidate-name/marker/prefetch vocabulary.
/// CLI admission retains environment pruning; GUI admission retains outer roots.
/// A marker is a regular file or a symlink entry; marker links are never followed.
/// Selected-root parent markers retain the R2 profile.
/// This pathname compatibility entrypoint does not claim descriptor confinement.
pub fn scan_client(
    root: &Path,
    snapshot: &CatalogSnapshot,
    options: ScanOptions,
    include_environments: bool,
    control: &ScanControl,
    emit: impl FnMut(&CatalogScanEvent),
) -> CatalogScanReport {
    struct State {
        selected: Option<Arc<TargetDescriptor>>,
        targets: BTreeMap<u64, Arc<TargetDescriptor>>,
        diagnostics: Vec<CatalogDiagnostic>,
        observations: u64,
        diagnostic_budget: crate::DiagnosticRetention,
        rule_profile: Option<RuleProfile>,
    }
    impl State {
        fn retain_diagnostic(
            &mut self,
            diagnostic: &CatalogDiagnostic,
            control: &ScanControl,
        ) -> bool {
            if !self.diagnostic_budget.admit(
                &diagnostic.relative_path,
                &diagnostic.classification,
                control,
            ) {
                return false;
            }
            self.diagnostics.push(diagnostic.clone());
            true
        }
    }
    let state_cell = RefCell::new(State {
        selected: None,
        targets: BTreeMap::new(),
        diagnostics: Vec::new(),
        observations: 0,
        diagnostic_budget: crate::DiagnosticRetention::default(),
        rule_profile: control
            .rule_profiling
            .load(std::sync::atomic::Ordering::Acquire)
            .then(|| snapshot.catalog().rule_profile()),
    });
    let emit = RefCell::new(emit);
    let catalog = snapshot.catalog();
    let needs_relative_path = catalog.needs_relative_path();
    let descriptors: BTreeMap<_, _> = catalog
        .targets()
        .iter()
        .map(|target| (target.id.as_str(), Arc::new(target.clone())))
        .collect();
    let report = crate::scan_with_candidate_observer(
        root,
        options,
        control,
        |path, legacy, kind| {
            let mut state = state_cell.borrow_mut();
            state.selected = None;
            // The two versioned clients historically reject an unavailable node name
            // before classification. The generic plugin API has broader semantics.
            if legacy.name().is_none() {
                return CandidateAction::Traverse;
            }
            if legacy.is_directory && legacy.node_name == b".git" && !legacy.has_artifact_ancestor {
                return CandidateAction::Prune;
            }
            if snapshot.profile() == ClientProfile::GuiV1 && legacy.has_artifact_ancestor {
                return CandidateAction::Traverse;
            }
            let kind = NodeKind::from(kind);
            let relative_path = needs_relative_path
                .then(|| crate::plugin_relative_path(root, path))
                .flatten();
            let requests = catalog
                .candidate_marker_requests(
                    legacy.name(),
                    if kind == NodeKind::Symlink {
                        NodeKind::Directory
                    } else {
                        kind
                    },
                )
                .into_iter()
                .filter(|probe| kind != NodeKind::Symlink || probe.scope == Scope::Parent)
                .collect::<Vec<_>>();
            let mut observed = BTreeMap::new();
            // Environment admission belongs to the CLI profile, even if the target is
            // excluded or has an arbitrary directory name. Never hide it in a rule ID.
            if snapshot.profile() == ClientProfile::CliV1
                && !include_environments
                && kind == NodeKind::Directory
                && !legacy.has_artifact_ancestor
            {
                let probe = MarkerProbe {
                    scope: Scope::Own,
                    name: "pyvenv.cfg".into(),
                };
                let value = observe_marker(path, &probe);
                state.observations = state.observations.saturating_add(1);
                let prune = matches!(value, Observation::Known(true));
                if let Observation::Unavailable(reason) = &value {
                    let diagnostic = host_diagnostic(
                        crate::relative_path(root, path),
                        "environment_admission",
                        reason.clone(),
                    );
                    if !state.retain_diagnostic(&diagnostic, control) {
                        return CandidateAction::Traverse;
                    }
                    drop(state);
                    (emit.borrow_mut())(&CatalogScanEvent::Diagnostic(diagnostic));
                    state = state_cell.borrow_mut();
                    if control.should_stop() {
                        return CandidateAction::Traverse;
                    }
                }
                observed.insert(probe, value);
                if prune {
                    return CandidateAction::Prune;
                }
            }
            let placeholders: BTreeMap<_, _> = requests
                .iter()
                .map(|p| ((*p).clone(), MarkerFact::Unavailable("not_observed")))
                .collect();
            let mut classify = |facts, phase| {
                if kind == NodeKind::Symlink {
                    catalog.classify_symlink_candidate(
                        facts,
                        relative_path.as_deref(),
                        state.rule_profile.as_mut(),
                        phase,
                    )
                } else if let Some(profile) = state.rule_profile.as_mut() {
                    catalog.classify_profiled_with_path(
                        facts,
                        relative_path.as_deref(),
                        profile,
                        phase,
                    )
                } else {
                    catalog.classify_with_path(facts, relative_path.as_deref())
                }
            };
            let result = classify(
                CandidateFacts {
                    name: legacy.name(),
                    kind,
                    markers: &placeholders,
                },
                RulePhase::Preliminary,
            )
            .and_then(|preliminary| {
                if preliminary.unknown.is_empty() {
                    return Ok(preliminary);
                }
                for probe in requests {
                    if !observed.contains_key(probe) {
                        observed.insert(
                            probe.clone(),
                            if control.is_cancelled() {
                                Observation::Unavailable("cancelled".into())
                            } else {
                                state.observations = state.observations.saturating_add(1);
                                observe_marker(path, probe)
                            },
                        );
                    }
                }
                let markers = observed
                    .iter()
                    .map(|(probe, value)| (probe.clone(), value.fact()))
                    .collect();
                if kind == NodeKind::Symlink {
                    catalog.classify_symlink_candidate(
                        CandidateFacts {
                            name: legacy.name(),
                            kind,
                            markers: &markers,
                        },
                        relative_path.as_deref(),
                        state.rule_profile.as_mut(),
                        RulePhase::Resolved,
                    )
                } else if let Some(profile) = state.rule_profile.as_mut() {
                    catalog.classify_profiled_with_path(
                        CandidateFacts {
                            name: legacy.name(),
                            kind,
                            markers: &markers,
                        },
                        relative_path.as_deref(),
                        profile,
                        RulePhase::Resolved,
                    )
                } else {
                    catalog.classify_with_path(
                        CandidateFacts {
                            name: legacy.name(),
                            kind,
                            markers: &markers,
                        },
                        relative_path.as_deref(),
                    )
                }
            });
            match result {
                Err(error) => {
                    control.fail();
                    let diagnostic = host_diagnostic(
                        crate::relative_path(root, path),
                        "policy_error",
                        error.to_string(),
                    );
                    if !state.retain_diagnostic(&diagnostic, control) {
                        return CandidateAction::Traverse;
                    }
                    drop(state);
                    (emit.borrow_mut())(&CatalogScanEvent::Diagnostic(diagnostic));
                    CandidateAction::Prune
                }
                Ok(classification) => {
                    if kind == NodeKind::Symlink
                        && let Some(potential) = classification.potential_symlink_target()
                    {
                        state.selected = descriptors.get(potential).cloned();
                        return CandidateAction::ReportSymlink(ArtifactClassificationCode {
                            language: 0,
                            kind: 0,
                        });
                    }
                    match &classification.decision {
                        Decision::Found(id) => {
                            state.selected = descriptors.get(id.as_str()).cloned();
                            CandidateAction::Classify(ArtifactClassificationCode {
                                language: 0,
                                kind: 0,
                            })
                        }
                        Decision::NoMatch => CandidateAction::Traverse,
                        Decision::Ambiguous | Decision::Indeterminate => {
                            let diagnostic = CatalogDiagnostic {
                                relative_path: crate::relative_path(root, path),
                                classification,
                            };
                            if !state.retain_diagnostic(&diagnostic, control) {
                                return CandidateAction::Traverse;
                            }
                            drop(state);
                            (emit.borrow_mut())(&CatalogScanEvent::Diagnostic(diagnostic));
                            CandidateAction::Traverse
                        }
                    }
                }
            }
        },
        |event| {
            if let ScanEvent::Finished(status) = event
                && let Some(profile) = &state_cell.borrow().rule_profile
            {
                let status =
                    diagnostic_status(*status, !state_cell.borrow().diagnostics.is_empty());
                (emit.borrow_mut())(&CatalogScanEvent::RuleProfile(rule_profile_json(
                    profile, status,
                )));
            }
            let event = {
                let mut state = state_cell.borrow_mut();
                match event {
                    ScanEvent::ArtifactDiscovered(a) | ScanEvent::SymlinkDiscovered(a) => {
                        let target = state
                            .selected
                            .take()
                            .expect("serial discovery follows classification");
                        state.targets.insert(a.id, target.clone());
                        let artifact = CatalogArtifact {
                            id: a.id,
                            relative_path: a.relative_path.clone(),
                            target,
                            bytes: (!a.is_symlink).then_some(a.bytes),
                            nested: a.nested,
                            partial: a.partial,
                            is_symlink: a.is_symlink,
                        };
                        if a.is_symlink {
                            CatalogScanEvent::SymlinkDiscovered(artifact)
                        } else {
                            CatalogScanEvent::ArtifactDiscovered(artifact)
                        }
                    }
                    ScanEvent::ArtifactCompleted { id, bytes, partial } => {
                        CatalogScanEvent::ArtifactCompleted {
                            id: *id,
                            bytes: *bytes,
                            partial: *partial,
                        }
                    }
                    ScanEvent::Warning(warning) => CatalogScanEvent::Warning(warning.clone()),
                    ScanEvent::Profile(sample) => {
                        let mut sample = *sample;
                        sample.warnings = sample
                            .warnings
                            .saturating_add(state.diagnostics.len() as u64);
                        CatalogScanEvent::Profile(sample)
                    }
                    ScanEvent::Finished(status) => CatalogScanEvent::Finished(diagnostic_status(
                        *status,
                        !state.diagnostics.is_empty(),
                    )),
                }
            };
            (emit.borrow_mut())(&event);
        },
    );
    let mut state = state_cell.into_inner();
    let status = diagnostic_status(report.status, !state.diagnostics.is_empty());
    let rule_profile = state
        .rule_profile
        .map(|profile| rule_profile_json(&profile, status));
    CatalogScanReport {
        catalog_digest: catalog.catalog_digest().to_string(),
        profile: snapshot.profile(),
        exclusions: catalog.exclusions().clone(),
        artifacts: report
            .artifacts
            .into_iter()
            .map(|a| CatalogArtifact {
                id: a.id,
                relative_path: a.relative_path,
                target: state
                    .targets
                    .remove(&a.id)
                    .expect("discovered catalog target"),
                bytes: (!a.is_symlink).then_some(a.bytes),
                nested: a.nested,
                partial: a.partial,
                is_symlink: a.is_symlink,
            })
            .collect(),
        warnings: report.warnings,
        status,
        diagnostics: state.diagnostics,
        marker_observations: state.observations,
        rule_profile,
    }
}

fn rule_profile_json(profile: &RuleProfile, status: ScanStatus) -> serde_json::Value {
    let mut value = profile.json();
    value["scan_status"] = serde_json::json!(match status {
        ScanStatus::Completed => "completed",
        ScanStatus::CompletedWithChanges => "completed_with_changes",
        ScanStatus::Cancelled => "cancelled",
        ScanStatus::Incomplete => "incomplete",
        ScanStatus::Failed => "failed",
    });
    value
}

fn diagnostic_status(status: ScanStatus, diagnostics: bool) -> ScanStatus {
    if diagnostics
        && matches!(
            status,
            ScanStatus::Completed | ScanStatus::CompletedWithChanges
        )
    {
        ScanStatus::Incomplete
    } else {
        status
    }
}

fn host_diagnostic(relative_path: PathBuf, fact: &str, reason: String) -> CatalogDiagnostic {
    CatalogDiagnostic {
        relative_path,
        classification: Classification {
            decision: Decision::Indeterminate,
            matched: Vec::new(),
            unknown: Vec::new(),
            unavailable: vec![buildhunter_plugin_policy::FactNotice {
                package_id: "org.buildhunter.host".into(),
                fact_id: fact.into(),
                reason,
            }],
        },
    }
}

enum Observation {
    Known(bool),
    Unavailable(String),
}
impl Observation {
    fn fact(&self) -> MarkerFact<'_> {
        match self {
            Self::Known(value) => MarkerFact::Known(*value),
            Self::Unavailable(reason) => MarkerFact::Unavailable(reason),
        }
    }
}
fn observe_marker(candidate: &Path, probe: &MarkerProbe) -> Observation {
    let directory = match probe.scope {
        Scope::Own => Some(candidate),
        Scope::Parent => candidate.parent(),
    };
    let Some(directory) = directory else {
        return Observation::Known(false);
    };
    match std::fs::symlink_metadata(directory.join(&probe.name)) {
        Ok(metadata) => Observation::Known(metadata.is_file() || metadata.file_type().is_symlink()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Observation::Known(false),
        Err(error) => Observation::Unavailable(error.to_string()),
    }
}
