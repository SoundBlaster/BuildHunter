//! Optional filesystem acquisition for hosts using the existing scan engine.
use crate::{DirectoryEvent, DirectoryVisit, EntryType, MeasuredSize};
use std::{ffi::OsStr, fs, io, path::Path};

/// Metadata from one opened directory. Implementations own the acquisition handle;
/// child names are single components, never complete paths. The caller retains
/// access and thread materialization guards through use and drop.
pub trait DirectoryAccess {
    fn metadata(&self) -> io::Result<fs::Metadata>;
    fn entry_metadata(&self, name: &OsStr) -> io::Result<EntryMetadata>;
}

/// Metadata needed by enumeration and sizing, without requiring an open file or
/// exposing the private layout of std::fs::Metadata. Backends retain raw names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntryMetadata {
    pub file_type: EntryType,
    pub size: MeasuredSize,
}

/// A scan-scoped acquisition boundary, shared by coordinator and workers. This
/// interface does not itself promise no-follow behavior: the selected backend
/// documents its platform and scope. Paths are observation/report identities.
///
/// `visit_directory` opens before checking cancellation, reports open/iteration
/// errors through `emit`, and checks stop before each entry's type observation.
/// Callbacks are synchronous and are not retained. No callbacks run after return.
/// `prepare_directory` retains one directory through a complete measurement pass;
/// it must not silently fall back to a pathname when opening that directory fails.
/// The host owns policy, error interpretation, workers and terminal publication.
pub trait ScanAccess: Sync {
    /// Only the selected root; non-root identities must be rejected. Directory
    /// metadata/file sizing after admission use the prepared directory instead.
    fn root_metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
    fn visit_directory(
        &self,
        path: &Path,
        should_stop: &mut dyn FnMut() -> bool,
        emit: &mut dyn FnMut(DirectoryEvent),
    ) -> DirectoryVisit;
    fn prepare_directory(&self, path: &Path) -> io::Result<Box<dyn DirectoryAccess + '_>>;
}
