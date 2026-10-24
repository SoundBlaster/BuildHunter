//! Product scan admission, profile/QoS setup and terminal publication around Hunter scope assembly.
use crate::{
    ArtifactHandler, CandidateAction, CandidateFacts, CandidateNodeKind, MAX_SCAN_WORKERS, Probes,
    ScanControl, ScanEvent, ScanOptions, ScanReport, ScanStatus, ScanWarning, materialization,
    profile, run_worker, thread_qos,
};
use hunter_fs_core::{EngineLaunch, GuardedSetupError, ScanChannel, with_scan_session_limits};
use std::{
    path::{Path, PathBuf},
    sync::atomic::Ordering,
};

pub(super) fn run(
    root: &Path,
    options: ScanOptions,
    control: &ScanControl,
    mut policy: impl FnMut(&Path, &CandidateFacts, CandidateNodeKind) -> CandidateAction,
    mut emit: impl FnMut(&ScanEvent),
    probes: &Probes,
    legacy_policy: bool,
) -> ScanReport {
    let result = with_scan_session_limits(
        materialization::Guard::enter,
        || Ok::<_, std::convert::Infallible>(probes),
        MAX_SCAN_WORKERS,
        hunter_fs_core::QueueLimits {
            max_tasks: control.limits.queued_tasks,
            max_weight: control.limits.queued_weight,
        },
        |backend, session| {
            let probes = *backend;
            let channel = ScanChannel::bounded(control.limits.response_capacity);
            let worker_qos = thread_qos::capture();
            let profile_state = control
                .profiling
                .load(Ordering::Acquire)
                .then(profile::ProfileState::new);
            let profile = profile_state.as_ref();
            let sender = channel.sender();
            let (artifacts, mut warnings, had_errors, changed) = session.run_engine(
                EngineLaunch {
                    backend: probes,
                    root,
                    channel,
                    preflight: crate::cloud_skip_reason,
                    wake_interval: profile.map(|_| profile::PROFILE_INTERVAL),
                },
                move |queue| {
                    thread_qos::apply(worker_qos);
                    run_worker(queue, &sender, control, probes, legacy_policy)
                },
                |scope| {
                    // Stop helpers before the shared queue closes and all threads join.
                    profile.map(|state| {
                        let (stop, signal) = std::sync::mpsc::channel::<()>();
                        scope.spawn(move || {
                            thread_qos::apply(worker_qos);
                            state.run_sampler(&signal);
                        });
                        stop
                    })
                },
                || {
                    let mut scanner = ArtifactHandler {
                        root,
                        options,
                        control,
                        policy: &mut policy,
                        legacy_policy,
                        emit: &mut emit,
                        probes,
                        artifacts: Vec::new(),
                        progress: Vec::new(),
                        warnings: Vec::new(),
                        finding_budget: crate::scan_budget::Retention::default(),
                        warning_budget: crate::scan_budget::Retention::default(),
                        had_errors: false,
                        changed: false,
                        profile,
                    };
                    scanner.begin_profile();
                    scanner
                },
                |scanner, _outcome| {
                    // Collect before helper shutdown/join so product progress state
                    // releases at the same boundary as the original coordinator.
                    (
                        scanner.artifacts,
                        scanner.warnings,
                        scanner.had_errors,
                        scanner.changed,
                    )
                },
            );
            // A reserved constant-size summary is outside the normal warning quota.
            // Publish after workers join so a late worker limit is not missed.
            if let Some(reason) = control.resource_limit() {
                let warning = ScanWarning {
                    relative_path: PathBuf::from("."),
                    message: format!(
                        "Resource limit reached: {}. Scan stopped; retained results may be partial.",
                        reason.name()
                    ),
                };
                warnings.push(warning.clone());
                if let Some(profile) = profile {
                    profile::bump(&profile.counters.warnings, 1);
                }
                emit(&ScanEvent::Warning(warning));
            }
            let status = if control.is_failed() {
                ScanStatus::Failed
            } else if control.is_cancelled() {
                ScanStatus::Cancelled
            } else if had_errors || control.resource_limit().is_some() {
                ScanStatus::Incomplete
            } else if changed {
                ScanStatus::CompletedWithChanges
            } else {
                ScanStatus::Completed
            };
            if let Some(profile) = profile {
                profile.counters.pending_tasks.store(0, Ordering::Relaxed);
                profile.record();
                for sample in profile.drain() {
                    emit(&ScanEvent::Profile(sample));
                }
            }
            emit(&ScanEvent::Finished(status));
            ScanReport {
                artifacts,
                warnings,
                status,
                #[cfg(test)]
                probes: probes.counts(),
            }
        },
    );
    match result {
        Ok(report) => report,
        Err(GuardedSetupError::Guard(error)) => failed_start(error, &mut emit, probes),
        Err(GuardedSetupError::Backend(never)) => match never {},
    }
}

fn failed_start(
    error: std::io::Error,
    emit: &mut impl FnMut(&ScanEvent),
    _probes: &Probes,
) -> ScanReport {
    let warning = ScanWarning {
        relative_path: PathBuf::from("."),
        message: format!("Cannot prevent cloud downloads; scan was not started: {error}"),
    };
    emit(&ScanEvent::Warning(warning.clone()));
    emit(&ScanEvent::Finished(ScanStatus::Failed));
    ScanReport {
        artifacts: Vec::new(),
        warnings: vec![warning],
        status: ScanStatus::Failed,
        #[cfg(test)]
        probes: _probes.counts(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn failed_admission_warns_then_finishes_without_starting_work() {
        let probes = Probes::default();
        let mut events = Vec::new();
        let report = failed_start(
            std::io::Error::from_raw_os_error(13),
            &mut |e| events.push(e.clone()),
            &probes,
        );
        assert_eq!(report.status, ScanStatus::Failed);
        assert!(report.artifacts.is_empty());
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.warnings[0].relative_path, Path::new("."));
        assert!(
            report.warnings[0]
                .message
                .starts_with("Cannot prevent cloud downloads; scan was not started:")
        );
        assert!(
            matches!(events.as_slice(), [ScanEvent::Warning(w), ScanEvent::Finished(ScanStatus::Failed)] if *w == report.warnings[0])
        );
        assert_eq!(report.probes.metadata, 0);
        assert!(report.probes.listing_threads.is_empty());
    }

    #[test]
    fn final_profile_is_drained_before_one_caller_thread_terminal_event() {
        let tree = crate::tests::TempTree::new();
        std::fs::create_dir(tree.0.join(".build")).unwrap();
        std::fs::write(tree.0.join(".build/data"), b"data").unwrap();
        let control = ScanControl::new();
        control.set_profiling(true);
        let owner = std::thread::current().id();
        let mut events = Vec::new();
        let report = run(
            &tree.0,
            ScanOptions::default(),
            &control,
            |_, facts, _| crate::rust_cli_policy(facts, false),
            |event| {
                assert_eq!(std::thread::current().id(), owner);
                events.push(event.clone());
            },
            &Probes::default(),
            true,
        );
        assert_eq!(report.status, ScanStatus::Completed);
        assert_eq!(
            events
                .iter()
                .filter(|e| matches!(e, ScanEvent::Finished(_)))
                .count(),
            1
        );
        assert!(matches!(
            events.last(),
            Some(ScanEvent::Finished(ScanStatus::Completed))
        ));
        let final_sample = events
            .iter()
            .rev()
            .find_map(|e| {
                if let ScanEvent::Profile(sample) = e {
                    Some(sample)
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(final_sample.pending_tasks, 0);
        assert_eq!(report.artifacts.len(), 1);
    }
}
