//! Product-owned mapping from Hunter metadata to plugin marker facts.
use buildhunter_plugin_policy::{
    MARKER_SUFFIX_ENTRY_LIMIT, MarkerFact, MarkerMatch, MarkerProbe, Scope as MarkerScope,
};
use hunter_fs::{FileFact, Location, ScopedFilesystem, ScopedMetadata, Unavailable};
use hunter_fs_core::{DirectoryEvent, DirectoryVisit, ScanAccess};
use std::{
    cell::Cell,
    ffi::OsStr,
    io,
    path::{Component, Path},
};

pub(super) struct Scope<'a> {
    filesystem: &'a ScopedFilesystem,
    root: &'a Path,
    metadata: Option<ScopedMetadata>,
}
impl<'a> Scope<'a> {
    pub(super) fn new(
        filesystem: &'a ScopedFilesystem,
        root: &'a Path,
        needs_markers: bool,
    ) -> io::Result<Self> {
        Ok(Self {
            filesystem,
            root,
            metadata: if needs_markers {
                Some(filesystem.marker_scope()?)
            } else {
                None
            },
        })
    }
    pub(super) fn observe(
        &self,
        candidate: &Path,
        probe: &MarkerProbe,
        cancelled: &dyn Fn() -> bool,
    ) -> (MarkerFact<'static>, u64) {
        let Some(scope) = &self.metadata else {
            return (MarkerFact::Unavailable("out_of_scope"), 0);
        };
        let Ok(relative) = candidate.strip_prefix(self.root) else {
            return (MarkerFact::Unavailable("out_of_scope"), 0);
        };
        if relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
        {
            return (MarkerFact::Unavailable("out_of_scope"), 0);
        }
        let location = match probe.scope {
            MarkerScope::Own => Location::Own,
            MarkerScope::Parent => Location::Parent,
        };
        // Hunter's directory handle is rooted and no-follow for every component.
        // It reports the terminal entry type without resolving a symlink.
        let directory = match location {
            Location::Own => Some(candidate),
            Location::Parent if candidate != self.root => candidate.parent(),
            Location::Parent => None,
        };
        if probe.matching == MarkerMatch::Suffix {
            let Some(directory) = directory else {
                return (MarkerFact::Unavailable("out_of_scope"), 0);
            };
            let count = Cell::new(0usize);
            let found = Cell::new(false);
            let error = Cell::new(false);
            let visit = self.filesystem.visit_directory(
                directory,
                &mut || cancelled() || found.get() || count.get() >= MARKER_SUFFIX_ENTRY_LIMIT,
                &mut |event| {
                    count.set(count.get() + 1);
                    match event {
                        DirectoryEvent::Entry { path, file_type } => {
                            if (file_type.is_file() || file_type.is_symlink())
                                && path
                                    .file_name()
                                    .is_some_and(|name| probe.matches_name(name))
                            {
                                found.set(true);
                            }
                        }
                        DirectoryEvent::Error { .. } => error.set(true),
                    }
                },
            );
            let fact = if found.get() {
                MarkerFact::Known(true)
            } else if cancelled() {
                MarkerFact::Unavailable("cancelled")
            } else if visit == DirectoryVisit::Cancelled && count.get() >= MARKER_SUFFIX_ENTRY_LIMIT
            {
                MarkerFact::Unavailable("marker_entry_limit")
            } else if error.get() || visit != DirectoryVisit::Completed {
                MarkerFact::Unavailable("io_error")
            } else {
                MarkerFact::Known(false)
            };
            return (fact, 1);
        }
        let mut attempted = 0;
        if let Some(directory) = directory
            && let Ok(handle) = self.filesystem.prepare_directory(directory)
        {
            attempted += 1;
            match handle.entry_metadata(OsStr::new(&probe.name)) {
                Ok(entry) => {
                    return (
                        MarkerFact::Known(
                            entry.file_type.is_file() || entry.file_type.is_symlink(),
                        ),
                        attempted,
                    );
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return (MarkerFact::Known(false), attempted);
                }
                Err(_) => {}
            }
        }
        // Preserve Hunter's typed diagnostics for inaccessible or unsafe paths.
        let observation = scope.inspect_regular_file(candidate, location, OsStr::new(&probe.name));
        let fact = match observation.fact {
            FileFact::Known(value) => MarkerFact::Known(value),
            FileFact::Unavailable(reason) => MarkerFact::Unavailable(match reason {
                Unavailable::OutOfScope => "out_of_scope",
                Unavailable::AccessDenied => "access_denied",
                Unavailable::CloudSkipped => "cloud_skipped",
                Unavailable::IoError | Unavailable::InvalidName => "io_error",
            }),
        };
        (fact, attempted + u64::from(observation.metadata_attempted))
    }
}
