# Catalog-driven scan integration pilot

`crates/buildhunter-plugin-scanner` connects the pure PluginCatalog to the existing
Rust traversal/measurement engine. It is an independent package, outside the root
CLI dependency graph. The pure policy package remains filesystem-free. CLI, GUI,
Settings defaults and numeric FFI still use their existing policies.

## API and ownership

```rust
let report = buildhunter_plugin_scanner::scan(
    root, &catalog, options, &control, |event| { /* serial live updates */ }
)?;
```

The catalog and exclusions are immutable for the entire call. Reports retain
the full catalog digest and exclusion snapshot, including unknown future IDs. Events and final
artifacts carry full TargetDescriptor metadata, including arbitrary ecosystem and
stable target IDs. Internal numeric scanner labels are transport placeholders;
they are never exposed as the plugin's classification. One Found target routes to
one measured artifact. Ambiguous/Indeterminate candidates emit typed diagnostics
and keep traversing; excluded/NoMatch parents keep traversing too. `.git` is a
host-owned prune boundary. Nested findings are reported; totals count outer roots
once using the existing allocated/apparent-size and per-path hardlink semantics.

The new dependency-free root API `scan_with_candidate_observer` observes every
visited node, bypassing the legacy candidate-name gate, marker resolution and
speculative artifact sizing. Its path is an observation identity, not an access
capability. Callbacks are serial on the calling thread. `ScanControl::is_cancelled`
reports a request, not worker completion. The scan call still owns worker join,
partial completion events, cloud guard restoration and callback lifetime.

## Fact acquisition

The integration converts every entry's lossless name and actual node kind to
plugin facts. Unsupported UTF-8 stays unavailable rather than matching a lossy
replacement. Unix entry paths remain PathBuf throughout this API; this does not
change the GUI's existing path actions or ABI.

Active bindings determine marker names and own/parent scopes. A preliminary pure
evaluation with unknown placeholders skips metadata observations only when no
unknown target remains; it never replaces an unavailable fact with false. All
necessary active probes are then observed, deduplicated by scope/name. Disabled
bindings cause no observations. This is conservative acquisition, not a compiled
candidate index, directory-listing cache or throughput optimization.

On macOS/Linux, a selected root directory descriptor anchors marker access.
Intermediate components are opened one at a time with openat/O_NOFOLLOW;
fstatat/AT_SYMLINK_NOFOLLOW checks the final marker's type without opening its
contents. Link/directory/non-file markers are false. Parent of selected root is
out_of_scope without reading outside. A replaced intermediate directory cannot
redirect a marker lookup. Missing final marker is false; access, cloud and boundary
failures are unavailable. Checks run under the non-materializing guard; macOS
SF_DATALESS metadata is cloud_skipped. Scope preflight failures return ScanError
before scan events; policy errors return ScanError after an explicit Failed event.

Windows fails closed with UnsupportedPlatform; its no-follow backend is pending.
Marker safety is stronger than the legacy symlink-marker behavior and is confined
to this opt-in path. The shared traversal still uses pathname directory listings
and measurement: descriptor-bound marker tests do not prove race-free traversal
or sizing under concurrent directory replacement. That separate Hunter/backend
migration remains necessary before claiming an end-to-end no-follow capability.
Observations are not an atomic filesystem snapshot. Missing markers observed
before a lookup cannot be distinguished from markers that never existed; no new
churn warning is fabricated. Current traversal/measurement churn notices remain.

## Diagnostics, limits and validation

Completed means the inherited traversal completed, not that all plugin facts were
known. Plugin diagnostics are separate from I/O warnings and must be surfaced by
future consumers. Reports retain artifacts and diagnostics, like existing scan
reports; no whole-scan RSS bound is claimed. marker_observations counts actual
marker metadata checks, excluding out-of-scope placeholders and directory opens.

```sh
cargo test --manifest-path crates/buildhunter-plugin-scanner/Cargo.toml --locked
cargo test --manifest-path crates/buildhunter-plugin-scanner/Cargo.toml --locked --release
```

Tests cover arbitrary directory/file names, own-marker-only rules, distinct enabled
children under excluded parents, ambiguity with continued traversal, cancellation,
root scope, no-follow marker links and directory replacement, permission failures,
nested totals/legacy sizing, and unchanged default/offline CLI. Linux additionally
creates invalid UTF-8 filenames; APFS rejects those filenames, so macOS uses the
pure policy None-name coverage. Synthetic dataless metadata is a decoder witness,
not a live iCloud test. No GUI, live provider or performance evidence is implied.

Next: descriptor-bound traversal/measurement in Hunter, then explicit C2/C3/C4
migration decisions and versioned FFI/Settings catalog routing.
