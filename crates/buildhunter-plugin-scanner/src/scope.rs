//! Product-owned mapping from Hunter metadata to plugin marker facts.
use buildhunter_plugin_policy::{MarkerFact, MarkerProbe, Scope as MarkerScope};
use hunter_fs::{FileFact, Location, ScopedFilesystem, ScopedMetadata, Unavailable};
use std::{ffi::OsStr, io, path::Path};

pub(super) struct Scope(Option<ScopedMetadata>);
impl Scope {
    pub(super) fn new(filesystem: &ScopedFilesystem, needs_markers: bool) -> io::Result<Self> {
        Ok(Self(if needs_markers {
            Some(filesystem.marker_scope()?)
        } else {
            None
        }))
    }
    pub(super) fn observe(
        &self,
        candidate: &Path,
        probe: &MarkerProbe,
    ) -> (MarkerFact<'static>, bool) {
        let Some(scope) = &self.0 else {
            return (MarkerFact::Unavailable("out_of_scope"), false);
        };
        let location = match probe.scope {
            MarkerScope::Own => Location::Own,
            MarkerScope::Parent => Location::Parent,
        };
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
        (fact, observation.metadata_attempted)
    }
}
