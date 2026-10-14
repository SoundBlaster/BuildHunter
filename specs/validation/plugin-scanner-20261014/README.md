# Plugin scanner integration validation — 2026-10-14

Baseline 6bab3c3 plus independently hashed changed inputs. Historical recovery,
policy and PR-review snapshots remain unchanged, not evidence for this revision.

- [Source/map/tool identity](source-identity.json).
- [Full commands and actual outputs](commands.json).
- [Negative control](negative-control.json): enabling the legacy name gate in
  the new entrypoint makes the arbitrary Forge integration test fail. Restoring
  the observer makes it pass. The temporary mutation is not in the delivered source.
- [Frozen maps](../../evidence/plugin-scanner-20261014/maps/).
- [SpecPM diagnostics](specpm.json): 0 errors, 15 unresolved-capability warnings.

All final commands passed on local macOS ARM64. Root suites: 52 library / 4 CLI
(default), 56 library / 4 CLI (all features). Independent policy: 15 tests in
Debug and Release. Integration: 5 marker-scope tests / 11 scan tests in both
Debug and Release. Both clippy checks, CLI integration, fresh-Cargo-home offline
check/install/launch and plugin schemas passed. HC parsing and all four profiles
passed. The Linux invalid-name and Windows unsupported-backend tests await CI;
APFS rejects the invalid filename used by the Linux fixture. Permission witness
ran as a non-root user; root execution would bypass that test's mode checks.

Synthetic dataless metadata establishes decoding only, not live iCloud behavior.
Marker replacement tests do not establish race-free shared traversal/measurement.
No Swift/UI/FFI migration, home scan, Store build or throughput/RSS measurement
was run. Hosted CI is separate evidence, pending at PR publication.
