# R5 resource foundations

Hunter adds optional count/weight admission limits and typed rejection while
retaining unbounded compatibility constructors. Rejection never blocks the serial
coordinator. The weighted enqueue keeps reservation/telemetry ordering; the host
must stop after a rejected publication. Counts and caller-defined weights do not
account for arbitrary payload allocations or RSS.

Completed listing records are removed immediately, separately from the checked
never-reused ID counter. The regression completes 100,000 sequential tasks while
checking constant live storage; map capacity still follows concurrent high water.

ScanChannel supports bounded/rendezvous responses using a generic sender type.
The existing engine releases unread receiver state before join. A worker blocked
on a rendezvous send during factory panic is released and joined. Full-channel
admission and payload ownership are covered independently.

Local arm64 macOS Debug/Release workspace checks passed: 144 core unit tests,
9 facade unit + 9 filesystem integration tests and 2 compile-fail documentation
tests. Clippy all targets/features passed. HC parse/validate passed; SpecPM reports
0 errors / 15 unresolved-capability warnings. Commands, tool/source identities and
log hashes are separate. One initial clippy ordering diagnostic was fixed by moving
the test module; current recorded commands describe the corrected source.

No BuildHunter defaults change in this layer. Listing payload limits, reports/
diagnostics/profile bounds and whole-process/resource measurements remain the next
layers. Source/maps/contracts stay drafts; tests do not accept new requirements.
