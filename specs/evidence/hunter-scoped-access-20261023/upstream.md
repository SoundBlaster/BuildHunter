# Pinned Hunter acquisition excerpts

Original repository: SoundBlaster/Hunter at bfcd75f8996b5a958c81238bb6d44d9fa1f88047.
Full-file SHA256 identities are recorded in source-identity.json. These excerpts
identify public seams; the repository source and exact Git dependency remain the
authority. They are not an independent copy of the implementation or proof.

From src/scoped_filesystem.rs, impl ScopedFilesystem:
```rust
pub fn open(root: &Path) -> io::Result<Self>
pub fn marker_scope(&self) -> io::Result<ScopedMetadata>
```
From crates/hunter-fs-core/src/scan_access.rs:
```rust
pub trait ScanAccess: Sync {
    fn root_metadata(&self, path: &Path) -> io::Result<fs::Metadata>;
    fn visit_directory(&self, path: &Path, should_stop: &mut dyn FnMut() -> bool,
        emit: &mut dyn FnMut(DirectoryEvent)) -> DirectoryVisit;
    fn prepare_directory(&self, path: &Path) -> io::Result<Box<dyn DirectoryAccess + '_>>;
}
pub trait DirectoryAccess {
    fn metadata(&self) -> io::Result<fs::Metadata>;
    fn entry_metadata(&self, name: &OsStr) -> io::Result<EntryMetadata>;
}
```

Selected directory roots, component resolution and prepared-pass handles are
owned upstream. Selected non-directories retain initial metadata only. Product
selection, type-change warning semantics, lifetime assembly and guard placement
remain host-owned. Independent passes are not an atomic snapshot; Windows/other
Unix and durable path actions remain unsupported/unresolved.
