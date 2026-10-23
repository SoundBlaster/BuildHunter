# R3 acquisition injection — 2026-10-23

Hunter facade and verified std-only core snapshot are pinned to
bfcd75f8996b5a958c81238bb6d44d9fa1f88047 (Hunter #26).

Scope: optional acquisition on the existing product engine; no backend selection
change in default CLI/GUI or the existing plugin entrypoint in this layer.
Scanner/Measurement and HunterFilesystemBackend/ScanScopedAccess are the affected
roles. The new interface is folded into buildhunter.current.scan; the concrete
backend contract is owned by Hunter. Target requirements/acceptance stay draft.

Local arm64 macOS results (commands.json): 70 root Rust tests plus 4 CLI tests in
Debug/Release, all-features clippy with warnings denied, vendor integrity and its
4 regressions, dependency graph validation and its 8 regressions, fresh-cache
offline CLI check/install/help, explicit Release CLI build, 13 CLI checks and
3 migration checks (30 fixture configurations, each in both size modes). Existing
plugin integration tests also passed at the aligned pin before backend selection.
HC parse/validation passed for current default/CLI/GUI and target public/store
profiles. SpecPM returned zero errors; unresolved dependency warnings are retained.

Two spy tests demonstrate worker/coordinator backend routing, one prepared pass
per directory, closure before Finished and no native fallback after denial. The
spy deliberately uses ordinary paths; it is not descriptor safety evidence.
Upstream no-follow fixtures and the following plugin layer have separate evidence.
No local Xcode UI run, whole-home benchmark, new resource bound or live cloud
acceptance is implied. CI is an independent gate.

Initial CLI invocations were made before an explicit source-pinned binary build;
they are marked superseded in the ledger. Current CLI claims use only the final
explicit Release build and repeated checks with recorded binary SHA256.
Exact source/map/input hashes are in source-identity.json. Raw logs are on TB5
and their digests are in commands.json. Historical receipts remain unchanged.

Initial receipts at eafb6ff are preserved under initial/. The current pin includes
the reviewed relative-root correction; core bytes are identical and current
validation refreshes the graph/pin and runtime checks.
