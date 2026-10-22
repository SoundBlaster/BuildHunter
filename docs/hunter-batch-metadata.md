# Hunter batch metadata integration — 2026-10-21

Runtime baseline: BuildHunter main `7ab00f16ae437fe88427c2acbe1b651b30b044f1`.
The branch also carries the documentation audit #96; its dated matrix remains a
historical comparison. Exact Hunter core pin: `7e8243c738613f985e1568278f1a69df6c5036b7`,
the top of Hunter #22 → #23. All 25 vendored files match that upstream snapshot.
Merge the Hunter stack before this consumer; Git remotes stay SSH and dependency
provenance stays HTTPS. Version/release alignment remains a later step.

## Ownership

Hunter `darwin_bulk::read_directory_sizes` owns the original final-component
no-follow open, aligned buffer, getattrlistbulk loop and checked records. It uses
a borrowed stop callback in place of ScanControl, at the same loop boundary.
Open still precedes the first stop check. Stop returns accumulated observations;
I/O/decode failure returns an error. Native errors and raw basename bytes remain.
The four decoder/filesystem tests moved upstream; the local duplicate is removed.

Hunter `PreparedDirectorySizes` owns already acquired map/error state. `take_file`
consumes a basename entry, including cached errors, once. Missing/consumed names
and Unsupported acquisition return None. Other batch-wide errors are reconstructed
for each requested file, retaining errno or kind/message. The helper does no I/O,
adds no cancellation checkpoint and does not validate directory membership.
BuildHunter supplies original same-directory slots. Keys use platform-native
basename encoding and are not portable serialized paths.

BuildHunter `MetadataBackend`/`PreparedFiles` retain:

- decision to acquire a batch only when at least one file slot exists;
- test metadata hooks and materialization-policy observations before acquisition;
- opt-in bulk directory/fallback counters and measurement timing;
- injected-error precedence before cached lookup;
- portable Probes fallback and product dataless error decoration;
- guard placement and all classification, membership, aggregation and final status.

Cached errors return before fallback/counter increments. Fallback errors are
already decorated by Probes and are not decorated again. Preparation still runs
after directory metadata failure and pre-cancellation, before the slot loop stops.
DirectoryMeasurementPlan, worker publication, receiver/join and terminal boundaries
are unchanged. Native core APIs do not acquire or restore a host thread's guard.

## Feature and validation

The existing `macos-bulk-metadata` feature forwards to Hunter's equally named
feature. It remains default-off; CLI/GUI defaults and the C ABI are unchanged.
The pure prepared lookup is available on all platforms; Darwin acquisition is
compiled only for macOS with the feature. Existing CI package-test commands now
exercise all core features, using the same jobs/runners.

Local source/command/map identities, tests and paired Release comparisons for
both feature modes are in
[`specs/validation/hunter-batch-20261021`](../specs/validation/hunter-batch-20261021/).
The pair uses the same fixture/harness and seven warm interleaved samples per
revision/mode. These are regression budgets, not a general speedup or RSS bound.
Runtime changes bind the immutable integration commit separately from later
documentation/CI receipts; historical launch/audit receipts remain historical.

Only the final opened directory component is no-follow. Path ancestors and shared
traversal/measurement remain pathname-based. No atomic view, live iCloud guarantee,
syscall interruption or whole-scan memory bound is introduced. No fresh home/UI/AX
acceptance is claimed. Current/target maps and draft scan provenance are updated;
default plugin migration and catalog semantics are not changed.

Next extraction: requested QoS and OS process CPU/RSS snapshots, keeping product
profile counters, event/JSON ABI and UI in BuildHunter. Descriptor-bound traversal,
C5 budgets, default plugin migration and commercial delivery remain separate work.
