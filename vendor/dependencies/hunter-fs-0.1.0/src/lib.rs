//! Reusable filesystem primitives, independent of product classification.
#[cfg(unix)]
mod scoped_metadata;
#[cfg(unix)]
pub use scoped_metadata::{FileFact, Location, Observation, ScopedMetadata, Unavailable};

#[cfg(any(target_os = "macos", target_os = "linux"))]
mod scoped_filesystem;
#[cfg(any(target_os = "macos", target_os = "linux"))]
pub use scoped_filesystem::ScopedFilesystem;

pub use hunter_fs_core::NonMaterializingIoGuard;
