//! Catalog decisions with the versioned, legacy-compatible CLI/GUI acquisition profile.
//! The descriptor-bound plugin pilot remains a separate stronger acquisition API.
use crate::{
    ArtifactClassificationCode, CandidateAction, CandidateNodeKind, ScanControl, ScanEvent,
    ScanOptions, ScanProfileSample, ScanStatus, ScanWarning,
    catalog::{CatalogSnapshot, ClientProfile},
};
use buildhunter_plugin_policy::{
    CandidateFacts, Classification, Decision, MarkerFact, MarkerProbe, NodeKind, Scope,
    TargetDescriptor,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug)]
pub struct CatalogArtifact {
    pub id: u64,
    pub relative_path: PathBuf,
    pub target: Arc<TargetDescriptor>,
    pub bytes: u64,
    pub nested: bool,
    pub partial: bool,
}

#[derive(Clone, Debug)]
pub struct CatalogDiagnostic {
    pub relative_path: PathBuf,
    pub classification: Classification,
}

#[derive(Clone, Debug)]
pub enum CatalogScanEvent {
    ArtifactDiscovered(CatalogArtifact),
    ArtifactCompleted { id: u64, bytes: u64, partial: bool },
    Diagnostic(CatalogDiagnostic),
    Warning(ScanWarning),
    Profile(ScanProfileSample),
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
}

impl CatalogScanReport {
    pub fn total_bytes(&self) -> u64 {
        self.artifacts
            .iter()
            .filter(|a| !a.nested)
            .fold(0u64, |sum, a| sum.saturating_add(a.bytes))
    }
}

/// Scan with one immutable snapshot. Dynamic marker requests and every-node
/// observation replace the fixed candidate-name/marker/prefetch vocabulary.
/// CLI admission retains environment pruning; GUI admission retains outer roots.
/// Marker file links and selected-root parent markers retain the R2 profile.
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
        diagnostic_budget: crate::scan_budget::Retention,
    }
    impl State {
        fn retain_diagnostic(
            &mut self,
            diagnostic: &CatalogDiagnostic,
            control: &ScanControl,
        ) -> bool {
            let classification = &diagnostic.classification;
            let mut weight =
                diagnostic
                    .relative_path
                    .capacity()
                    .saturating_add(std::mem::size_of::<CatalogDiagnostic>())
                    .saturating_add(
                        (classification.matched.capacity() + classification.unknown.capacity())
                            .saturating_mul(std::mem::size_of::<String>()),
                    )
                    .saturating_add(classification.unavailable.capacity().saturating_mul(
                        std::mem::size_of::<buildhunter_plugin_policy::FactNotice>(),
                    ));
            for id in classification.matched.iter().chain(&classification.unknown) {
                weight = weight.saturating_add(id.capacity());
            }
            for notice in &classification.unavailable {
                weight = weight
                    .saturating_add(notice.package_id.capacity())
                    .saturating_add(notice.fact_id.capacity())
                    .saturating_add(notice.reason.capacity());
            }
            let limits = control.limits();
            if let Err(reason) = self.diagnostic_budget.admit(
                weight,
                limits.diagnostics,
                limits.diagnostic_weight,
                crate::ScanLimit::Diagnostics,
                crate::ScanLimit::DiagnosticWeight,
            ) {
                control.exhaust(reason);
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
        diagnostic_budget: crate::scan_budget::Retention::default(),
    });
    let emit = RefCell::new(emit);
    let catalog = snapshot.catalog();
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
            let kind = match kind {
                CandidateNodeKind::Directory => NodeKind::Directory,
                CandidateNodeKind::RegularFile => NodeKind::RegularFile,
                CandidateNodeKind::Symlink => NodeKind::Symlink,
                CandidateNodeKind::Other => NodeKind::Other,
            };
            if kind == NodeKind::Symlink {
                return CandidateAction::Traverse;
            }
            let requests = catalog.candidate_marker_requests(legacy.name(), kind);
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
            let result = catalog
                .classify(CandidateFacts {
                    name: legacy.name(),
                    kind,
                    markers: &placeholders,
                })
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
                    catalog.classify(CandidateFacts {
                        name: legacy.name(),
                        kind,
                        markers: &markers,
                    })
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
                Ok(classification) => match &classification.decision {
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
                },
            }
        },
        |event| {
            let event = {
                let mut state = state_cell.borrow_mut();
                match event {
                    ScanEvent::ArtifactDiscovered(a) => {
                        let target = state
                            .selected
                            .take()
                            .expect("serial discovery follows classification");
                        state.targets.insert(a.id, target.clone());
                        CatalogScanEvent::ArtifactDiscovered(CatalogArtifact {
                            id: a.id,
                            relative_path: a.relative_path.clone(),
                            target,
                            bytes: a.bytes,
                            nested: a.nested,
                            partial: a.partial,
                        })
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
                bytes: a.bytes,
                nested: a.nested,
                partial: a.partial,
            })
            .collect(),
        warnings: report.warnings,
        status: diagnostic_status(report.status, !state.diagnostics.is_empty()),
        diagnostics: state.diagnostics,
        marker_observations: state.observations,
    }
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
    match std::fs::metadata(directory.join(&probe.name)) {
        Ok(metadata) => Observation::Known(metadata.is_file()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Observation::Known(false),
        Err(error) => Observation::Unavailable(error.to_string()),
    }
}
