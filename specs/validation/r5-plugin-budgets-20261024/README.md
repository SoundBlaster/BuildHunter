# R5 scoped plugin resource parity — 2026-10-24

Both default catalog and independent scoped adapters now use DiagnosticRetention
before cloning/storing/emitting diagnostics. Count or estimated-payload rejection
sets the control's first resource stop; core finalization preserves an explicit
Incomplete result and reserved warning. Normal within-quota diagnostic-only pilot
status remains Completed. Marker/no-follow behavior and the numeric ABI are unchanged.

Experimental PluginArtifact.target changes to Arc<TargetDescriptor>, shared across
events/findings/reports and owned after catalog release. Regressions cover diagnostic
count and payload rejection plus pointer identity and lifetime of shared descriptors.

Fresh root all-feature Debug/Release suites: 88 library + 5 CLI/profile each;
independent scoped integration: 18 tests in Debug and Release. Formatting and
all-target/all-feature Clippy passed. CLI 14, migration parity 3, dependency 10,
vendor 4 and schema 12 tests passed; fresh-Cargo-home offline check/install/launch
passed. Execution commands and actual log hashes are recorded separately. Cache
and output paths are external TB5 locations; schemas use the isolated jsonschema
venv. HC profiles and SpecPM are structural checks only, with unresolved external
capability warnings retained. Native test builds do not claim XCTest execution.

Earlier three-run CLI and controlled native GUI measurements are historical
source snapshots. The final-source home attempt and its limits are recorded in a
separate receipt. Do not infer equal result digests or completed scans from a timeout.
Raw home paths, stderr, samples and memory graphs remain local; only aggregate
receipts and hashes are published. Catalog/temporary allocations, opaque syscalls,
UI/export copies and concurrent scans remain outside a global RSS guarantee. No
new expensive CI job is added. Hosted native XCTest is requested once at the final
ready stack tip. These drafts are not owner acceptance of C5.

Final-source controlled fixtures completed: traversal 0 findings, burst 1 finding,
wide listing explicitly Incomplete with its resource warning (0 findings retained).
The home attempt timed out at 120 seconds with kernel peak RSS 40.34 MiB; two
workers were sampled in open$NOCANCEL. No final profile/report was emitted, so this
is neither a completed-home result nor equal-work latency/digest evidence. The
underlying open stall remains a separate platform/provider investigation.

Final-source Release app build and Debug build-for-testing both passed; their
exact commands/log hashes are included. Full hosted XCTest execution is pending.
