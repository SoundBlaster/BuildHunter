# Catalog client migration (R4)

Baseline: BuildHunter main `ebcdcef981e27889a6e43c524c06cff63eceea14`.

## Layer 1: catalog preparation

All 13 public basic checks are declarative plugin documents in `plugins/basic`.
`ClientProfile::CliV1` and `GuiV1` preserve C2 classification differences. Their
full catalog digests differ; exclusions preserve each digest and retain unknown
future IDs. GUI environment precedence is represented in the rules, including
when the environment target is disabled. Host environment/nested admission is
separate from pure classification.

The name index uses only necessary positive conditions. `all` can use a necessary
child constraint; `any` requires constrained alternatives; negation stays in the
fallback. Unknown names evaluate every possible target. Selected target programs
are evaluated by Rifle, with candidate-specific marker requests. A 200-target
fixture and differential Rifle tests cover dispatch, not scan throughput.

`CatalogSnapshot` and catalog C ABI 1 provide an owned immutable handle and JSON
metadata with format/profile/digest/descriptors/exclusions. Input bytes are copied;
metadata is borrowed until destruction. The owner must retain the snapshot until
all scans/readers finish. Legacy scan entrypoints and defaults are unchanged in
this layer; the GUI has not switched to the new metadata or classification yet.

The host now resolves Rifle and the Hunter facade. Cargo source replacement ships
unmodified dependencies (with their license files) so a fresh Cargo home can
check/install/run the CLI offline. Hunter/Rifle remain on their reviewed revision
pins, with one verified `hunter-fs-core`; no library version is raised.
Xcode invokes Cargo on every build so Cargo can track nested Rust/plugin inputs.
Cargo still decides whether compilation is needed.

## Layers following preparation (now implemented below)

- Shared catalog-driven scanning, client admission and a versioned event ABI that
  carries target/catalog identity and typed diagnostics.
- CLI/GUI/Settings cutover, explicit C3 marker-link behavior, C4 invalid-byte path
  action handling, golden client parity and the 200-target end-to-end Release run.

These drafts do not silently accept changed marker/path semantics, claim Windows
scoped acquisition, close resource budgets (R5), or prove sandbox/Store acceptance.

## Layer 2: client scan and event transport

`scan_client` observes all candidates, acquires markers requested by enabled
rule bindings, and measures findings with stable descriptors shared through Arc.
The current `terminal-entry-file-or-symlink/2` policy treats a terminal symlink
marker as present without resolving its target, including a dangling link.
Selected-root parent context remains available. This pathname API does not
claim descriptor confinement; the opt-in scoped scanner remains stronger.

Named symlinks, or symlinks matched by rules explicitly testing their kind, can
appear as potential findings when only target kind or own
markers are unknown. Their destination is never probed, traversed, or measured;
event 9 is terminal without a completion event. Ambiguous candidates remain
diagnostics. CLI JSON schema 3 uses null bytes and a symlink entry type; totals
include measured bytes only and disclose the count of unmeasured links.

`bh_scan_catalog_v1` emits metadata before discovery, references targets by ID,
and surfaces typed ambiguity/unavailability diagnostics. Borrowed pointers last
only through callback return; handles/context last until scan return after join.
Diagnostics make completed scans incomplete, while cancellation/failure dominate.
Direct callback tests copy metadata/IDs and cancel on discovery. All R2 normal
CLI/GUI fixture runs execute against this new API in allocated/apparent modes,
including exclusions, byte totals, marker links, environment precedence and
completion order. CLI preflight remains outside this library API.

Default CLI/GUI entrypoints are still switched only in the final client layer.

## Layer 3: default CLI, GUI and Settings

Default CLI calls `scan_client` with the CLI catalog; GUI owns a GUI catalog handle
through `bh_scan_catalog_v1` return. Swift no longer classifies every candidate or
builds a finite decision table for production scans. The numeric APIs and legacy
classification tests remain available; the old descriptor JSON supplies ordering
and compatibility hints only. Settings and `--list-filters` enumerate validated
plugin target IDs, with the existing preference key and unknown valid exclusions
preserved. Catalog reports use JSON schema 3 and export the frozen catalog/profile/
marker policy, target IDs, diagnostics and unmeasured link count; legacy/mock
exports without links or catalog metadata keep schema 1.

C3 now counts marker symlinks by entry without resolving them and separately
reports eligible candidate links; the frozen historical corpus is preserved.
Selected-root parent behavior remains unchanged. C4 copies raw selected-root bytes
and disables Finder/Copy actions for lossy components and descendants; safe
ancestors remain usable. This is action rejection, not a new inode capability.

The full scanner regression creates 200 actual new target names with individual
marker bindings, 1,024 unrelated folders and an exclusion; it verifies independent
allocated bytes, one completion per measured finding and exactly 200/199 relevant marker
observations. `cargo bench --locked --bench catalog_scan` additionally compares
13 vs 200 targets on the same representative filesystem tree in seven interleaved
Release rounds, checking all rows/sizes each time. It records first finding, total
time, CPU and sampled process RSS. The provisional broad timing guard is
`expanded median <= 2 * baseline median + 20 ms + 3 * baseline MAD`; correctness
and marker counts are exact independent gates. It runs inside existing Rust/CLI
CI jobs, with no new macOS runner job.

Local measurements and command/source identities are in
`specs/validation/r4-client-cutover-20261023`. Xcode build-for-testing succeeds;
local XCTest-host startup stalled in dyld/open before test execution and is not a
passing test result. CI must execute the new GUI tests. Warm synthetic timings do
not establish home-directory performance, memory-budget acceptance (R5), live
cloud behavior, sandbox acceptance or plugin installation (R7).
