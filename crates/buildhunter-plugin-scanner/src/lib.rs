//! Opt-in filesystem integration. The pure policy crate and default clients stay separate.
#[cfg(any(target_os = "macos", target_os = "linux"))]
use build_hunter::{ArtifactClassificationCode, CandidateAction, CandidateNodeKind, ScanEvent};
use build_hunter::{ScanControl, ScanOptions, ScanProfileSample, ScanStatus, ScanWarning};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use buildhunter_plugin_policy::{CandidateFacts, Decision, MarkerFact, MarkerProbe, NodeKind};
use buildhunter_plugin_policy::{Classification, PluginCatalog, TargetDescriptor};
#[cfg(any(target_os = "macos", target_os = "linux"))]
use std::{cell::RefCell, collections::BTreeMap};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod scope;

#[derive(Clone, Debug)]
pub struct PluginArtifact {
    pub id: u64,
    pub relative_path: PathBuf,
    pub target: Arc<TargetDescriptor>,
    pub bytes: u64,
    pub nested: bool,
    pub partial: bool,
}
#[derive(Clone, Debug)]
pub struct CandidateDiagnostic {
    pub relative_path: PathBuf,
    pub classification: Classification,
}
#[derive(Clone, Debug)]
pub enum PluginScanEvent {
    ArtifactDiscovered(PluginArtifact),
    ArtifactCompleted { id: u64, bytes: u64, partial: bool },
    Diagnostic(CandidateDiagnostic),
    Warning(ScanWarning),
    Profile(ScanProfileSample),
    Finished(ScanStatus),
}
#[derive(Debug)]
pub struct PluginScanReport {
    /// Full validated catalog identity, independent of exclusions.
    pub catalog_digest: String,
    pub exclusions: std::collections::BTreeSet<String>,
    pub artifacts: Vec<PluginArtifact>,
    pub warnings: Vec<ScanWarning>,
    pub diagnostics: Vec<CandidateDiagnostic>,
    pub status: ScanStatus,
    /// Actual no-follow marker metadata observations, excluding synthetic unavailable facts.
    pub marker_observations: u64,
}
impl PluginScanReport {
    pub fn total_bytes(&self) -> u64 {
        self.artifacts
            .iter()
            .filter(|a| !a.nested)
            .map(|a| a.bytes)
            .sum()
    }
}
#[derive(Debug)]
pub enum ScanError {
    Io(std::io::Error),
    Policy(PolicyError),
    UnsupportedPlatform,
}
// Preserve public policy errors without creating another direct Rifle dependency.
#[derive(Debug)]
pub struct PolicyError {
    pub code: String,
    pub location: String,
    pub message: String,
}
impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "scan preflight failed: {e}"),
            Self::Policy(e) => write!(f, "{} at {}: {}", e.code, e.location, e.message),
            Self::UnsupportedPlatform => {
                write!(
                    f,
                    "no verified no-follow acquisition backend on this platform"
                )
            }
        }
    }
}
impl std::error::Error for ScanError {}

/// Scan with one immutable catalog/exclusion snapshot. Every visited non-link node
/// is eligible; `.git` remains a host prune boundary. Nested findings are reported,
/// with outer-root-only totals. Conflicts/unknowns emit diagnostics and keep walking.
/// Caller-thread callbacks are synchronous, serial and protected against cloud
/// materialization. Unsupported platforms fail closed before starting the scan.
/// This pilot does not change the CLI, GUI, or their existing numeric FFI.
pub fn scan(
    root: &Path,
    catalog: &PluginCatalog,
    options: ScanOptions,
    control: &ScanControl,
    emit: impl FnMut(&PluginScanEvent),
) -> Result<PluginScanReport, ScanError> {
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (root, catalog, options, control, emit);
        Err(ScanError::UnsupportedPlatform)
    }
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        // Both core events and policy diagnostics use the same serial observer.
        let emit = RefCell::new(emit);
        let _guard = build_hunter::NonMaterializingIoGuard::enter().map_err(ScanError::Io)?;
        let root = if root.is_absolute() {
            root.to_owned()
        } else {
            std::env::current_dir().map_err(ScanError::Io)?.join(root)
        };
        let requests: Vec<MarkerProbe> = catalog
            .marker_requests(NodeKind::Directory)
            .into_iter()
            .cloned()
            .collect();
        let filesystem = hunter_fs::ScopedFilesystem::open(&root).map_err(ScanError::Io)?;
        let scope = scope::Scope::new(&filesystem, !requests.is_empty()).map_err(ScanError::Io)?;
        struct State {
            selected: Option<Arc<TargetDescriptor>>,
            targets: BTreeMap<u64, Arc<TargetDescriptor>>,
            diagnostics: Vec<CandidateDiagnostic>,
            failure: Option<PolicyError>,
            observations: u64,
            diagnostic_budget: build_hunter::DiagnosticRetention,
        }
        // Each descriptor is owned once per catalog; finding/event/report clones
        // share it, so a large description is not multiplied by finding count.
        let descriptors: BTreeMap<_, _> = catalog
            .targets()
            .iter()
            .map(|target| (target.id.as_str(), Arc::new(target.clone())))
            .collect();
        let state = RefCell::new(State {
            selected: None,
            targets: BTreeMap::new(),
            diagnostics: Vec::new(),
            failure: None,
            observations: 0,
            diagnostic_budget: build_hunter::DiagnosticRetention::default(),
        });
        let report = build_hunter::scan_with_candidate_access(
            &root,
            options,
            control,
            &filesystem,
            |path, legacy, kind| {
                let mut state = state.borrow_mut();
                state.selected = None;
                if legacy.node_name == b".git" {
                    return CandidateAction::Prune;
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
                let active = catalog.marker_requests(kind);
                // Unknown placeholders can prove NoMatch without filesystem I/O.
                // They must never be interpreted as false or retained as final diagnostics.
                let mut markers: BTreeMap<_, _> = active
                    .iter()
                    .map(|p| ((*p).clone(), MarkerFact::Unavailable("not_observed")))
                    .collect();
                let classify = |markers: &BTreeMap<MarkerProbe, MarkerFact>| {
                    catalog.classify(CandidateFacts {
                        name: legacy.name(),
                        kind,
                        markers,
                    })
                };
                let result = classify(&markers).and_then(|preliminary| {
                    if preliminary.unknown.is_empty() {
                        return Ok(preliminary);
                    }
                    for probe in active {
                        let (fact, observed) = if control.is_cancelled() {
                            (MarkerFact::Unavailable("cancelled"), false)
                        } else {
                            scope.observe(path, probe)
                        };
                        state.observations += u64::from(observed);
                        markers.insert(probe.clone(), fact);
                    }
                    classify(&markers)
                });
                match result {
                    Err(e) => {
                        state.failure = Some(PolicyError {
                            code: e.code.into(),
                            location: e.location,
                            message: e.message,
                        });
                        control.cancel();
                        CandidateAction::Traverse
                    }
                    Ok(classification) => match &classification.decision {
                        Decision::Found(id) => {
                            state.selected = Some(
                                descriptors
                                    .get(id.as_str())
                                    .expect("validated target exists")
                                    .clone(),
                            );
                            // Numeric labels stay transport placeholders inside this wrapper;
                            // emitted/report artifacts always carry the real TargetDescriptor.
                            CandidateAction::Classify(ArtifactClassificationCode {
                                language: 0,
                                kind: 0,
                            })
                        }
                        Decision::NoMatch => CandidateAction::Traverse,
                        Decision::Ambiguous | Decision::Indeterminate => {
                            let diagnostic = CandidateDiagnostic {
                                relative_path: path.strip_prefix(&root).unwrap_or(path).to_owned(),
                                classification,
                            };
                            if !state.diagnostic_budget.admit(
                                &diagnostic.relative_path,
                                &diagnostic.classification,
                                control,
                            ) {
                                return CandidateAction::Traverse;
                            }
                            state.diagnostics.push(diagnostic.clone());
                            // Deliver while processing this candidate so the observer can
                            // cancel before traversal continues, even without core events.
                            // Never hold our state borrow across a consumer callback.
                            drop(state);
                            (emit.borrow_mut())(&PluginScanEvent::Diagnostic(diagnostic));
                            CandidateAction::Traverse
                        }
                    },
                }
            },
            |event| {
                let event = {
                    let mut state = state.borrow_mut();
                    match event {
                        ScanEvent::ArtifactDiscovered(a) => {
                            let target = state
                                .selected
                                .take()
                                .expect("discovery immediately follows classification");
                            state.targets.insert(a.id, target.clone());
                            PluginScanEvent::ArtifactDiscovered(PluginArtifact {
                                id: a.id,
                                relative_path: a.relative_path.clone(),
                                target,
                                bytes: a.bytes,
                                nested: a.nested,
                                partial: a.partial,
                            })
                        }
                        ScanEvent::ArtifactCompleted { id, bytes, partial } => {
                            PluginScanEvent::ArtifactCompleted {
                                id: *id,
                                bytes: *bytes,
                                partial: *partial,
                            }
                        }
                        ScanEvent::Warning(w) => PluginScanEvent::Warning(w.clone()),
                        ScanEvent::Profile(p) => PluginScanEvent::Profile(*p),
                        ScanEvent::Finished(s) => {
                            PluginScanEvent::Finished(if state.failure.is_some() {
                                ScanStatus::Failed
                            } else {
                                *s
                            })
                        }
                    }
                };
                (emit.borrow_mut())(&event);
            },
        );
        let mut state = state.into_inner();
        if let Some(error) = state.failure {
            return Err(ScanError::Policy(error));
        }
        let artifacts = report
            .artifacts
            .into_iter()
            .map(|a| PluginArtifact {
                id: a.id,
                relative_path: a.relative_path,
                target: state.targets.remove(&a.id).expect("discovered target"),
                bytes: a.bytes,
                nested: a.nested,
                partial: a.partial,
            })
            .collect();
        Ok(PluginScanReport {
            catalog_digest: catalog.catalog_digest().to_string(),
            exclusions: catalog.exclusions().clone(),
            artifacts,
            warnings: report.warnings,
            diagnostics: state.diagnostics,
            status: report.status,
            marker_observations: state.observations,
        })
    }
}
