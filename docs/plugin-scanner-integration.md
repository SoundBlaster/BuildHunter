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
and keep traversing unless the observer cancels; excluded/NoMatch parents keep
traversing too. Diagnostics are delivered synchronously while their candidate is
processed, without waiting for another core event. Cancellation from this callback
is observed before processing descendants, including diagnostic-only scans. `.git` is a
host-owned prune boundary. Nested findings are reported; totals count outer roots
once using the existing allocated/apparent-size and per-path hardlink semantics.

The opt-in API `scan_with_candidate_access` observes every
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

Windows and other platforms without the macOS/Linux backend fail closed with
UnsupportedPlatform; their no-follow acquisition remains pending.
Marker safety is stronger than the legacy symlink-marker behavior and is confined
to this opt-in path. The pilot now selects Hunter ScopedFilesystem for root observation, directory
listings, worker sizing and coordinator fallback. Markers share that exact root
anchor rather than independently reopening the selected path. Each prepared
measurement pass retains its directory, and failed scoped access never retries
through a normal path or the pathname bulk backend. A file slot changed into a
link/directory is not counted. Independent passes still observe a live tree: this
is no-follow acquisition, not stable inode identity between passes or a snapshot.
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

Next: explicit C2/C3/C4 migration decisions and versioned FFI/Settings catalog
routing (R4), with resource budgets (R5) kept separate.

## Hunter extraction — 2026-10-14

The opt-in integration now pins `hunter-fs` from public Hunter at
`5c8f3c9a1a018039b5aea9b731f2fb744f625b5a` (Hunter PR #1).
`scope.rs` owns only optional acquisition and product outcome mapping. Hunter owns
the selected-root descriptor, component opens, final stat, technical unavailable
outcomes and synthetic dataless detection. Existing marker observations count
only final stat attempts, as before. Catalog-validated names remain unchanged;
Hunter additionally validates arbitrary public names instead of panicking.

That scoped-metadata stage added no application/CLI dependency; the subsequent portable
extraction adds a std-only vendored path dependency without changing default behavior.
The caller still installs the non-materializing policy before Hunter opens the
root and keeps it until scanning ends. Metadata safety does not prove the shared
pathname traversal/measurement safe against replacement. Windows still fails
closed. Full Hunter extraction, default-client migration and scan performance
acceptance remain pending. Upstream source/contract snapshots identify ownership
and revision; fresh local receipts are in `specs/validation/hunter-metadata-20261014`.

Cross-repository merge order: Hunter #1 first, then this consumer PR. The revision
pin is intentional until release alignment; HTTPS dependencies allow keyless CI,
while developer Git remotes continue using SSH.

Portable entry enumeration/byte sizing are now shared with Hunter through the
root offline graph; see hunter-portable-extraction.md. Classifier/marker policy
and the root scanner/FFI public interfaces remain unchanged.

## Dependency alignment — 2026-10-22

The independent scanner now pins `hunter-fs` to
`c5354e973b5d1d90c438c94b78c06898ffe62500`, exactly the revision recorded by
`vendor/hunter-fs-core/UPSTREAM.json`. Its workspace-root `[patch]` redirects
Hunter's Git core dependency to `../../vendor/hunter-fs-core`, which is also
used by the application. Simply updating the facade pin would otherwise allow
Git and path copies of the same core version. Scoped marker implementation
bytes are unchanged from the former `5c8f3c9` pin.

Rifle remains pinned to `693a284123faaa1ad3192cd88b259ba97a7b5910`; the policy-only
package still has no Hunter dependency. The default CLI does not resolve Rifle
or the opt-in scanner. No package version/default feature/classification/ABI
changes are introduced. Only the scanner lockfile needs a Git revision update;
root and policy lockfiles retain their existing resolution.

`scripts/verify_plugin_dependencies.py` checks all three active host graphs via
locked/offline Cargo metadata after dependencies have been fetched by normal
builds. It rejects duplicate or unverified core sources and stale Hunter/Rifle
pins. CI runs it on Linux/macOS/Windows; Windows correctly excludes the Unix-only
facade. Fixture tests exercise both host shapes and bad resolution cases. This
is the graph of these repository workspaces, not a guarantee for arbitrary
downstream consumers: Cargo patches in a dependency manifest are ignored by an
external workspace root, which must own its own patch/resolution policy.

The earlier extraction snapshots/checkpoints above remain historical. Fresh
source/graph/build/test evidence is in `specs/validation/plugin-alignment-20261022`.
Default client migration, C2/C3/C4 decisions and whole-process resource budgets
remain separate; dependency alignment does not complete them or establish a speedup.

## R3 descriptor acquisition — 2026-10-23

The current pin is Hunter `bfcd75f8996b5a958c81238bb6d44d9fa1f88047` (PR #26).
Older pin/stage descriptions above remain historical. Root handle acquisition
happens under the caller's non-materializing guard even when no markers are
requested. Only macOS/Linux expose this backend; Windows/other Unix fail closed.
The default CLI/GUI still use their old policies/backend, as frozen by R2.

Deterministic event-callback fixtures replace the selected root, an intermediate
parent directory, and a listed file before measurement. They assert that foreign
bytes are never counted, root replacement stays on the original tree, refusal
marks partial/incomplete state, and a changed link produces a change notice. A
file without content-read permission remains measurable via fstatat. These are
actual plugin scan tests, separate from Hunter's lower-level prepared-pass and
parallel-offset tests and the application seam's std-backed routing spies.

No snapshot across independent passes, durable Finder/path identity, report-memory
bound, no-download proof on a live provider or scan-speed gain is claimed. New
source/run identities are in `specs/validation/plugin-scoped-scan-20261023`.

The scoped-error review additionally removes native pathname diagnostics after
backend errors. Original errno/kind/message are retained; an outside replacement
object cannot relabel a failure. Post-rebase plugin execution identities are in
`plugin-scoped-review-20261023`; the earlier plugin receipts are historical.

## R5 retention update — 2026-10-24

The default clients have since migrated to catalog profiles (R4); paragraphs above
about pre-migration defaults record their historical stage. The independent scoped
pilot keeps its stricter acquisition contract. It now uses `ScanControl` resource
limits for traversal/findings/warnings and the shared `DiagnosticRetention` for
count/estimated-payload admission before diagnostic clone/publication. Rejection
requests `Incomplete` and retains the reserved resource-limit warning. Ordinary
within-quota diagnostic-only pilot scans keep their existing `Completed` status.

`PluginArtifact.target` changes from `TargetDescriptor` to
`Arc<TargetDescriptor>` in this experimental API. Event/report artifacts share an
owned immutable descriptor per target, which remains usable after the original
catalog drops. Use `artifact.target.as_ref()` for a descriptor reference; use
`artifact.target.as_ref().clone()` only when an owned copy is specifically needed.
No serialized target identity or marker/no-follow semantics change. Quotas are
per-scan retained payload estimates, not a process-wide RSS ceiling or catalog size
limit. Fresh regressions/source identities: `specs/validation/r5-plugin-budgets-20261024`.
