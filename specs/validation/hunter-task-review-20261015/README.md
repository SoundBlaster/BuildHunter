# Hunter task contract wording review

Upstream Hunter #4 review replaces classification-specific vocabulary with opaque
host-policy language. All upstream/vendored Rust source files and BuildHunter
src/lib.rs are unchanged. Canonical README, UPSTREAM.json, current revision
bindings and upstream draft snapshot are refreshed; historical receipts remain
untouched. Affected roles: HunterPortable, HunterWork, HunterDirectoryTasks;
capabilities/interfaces/assembly duties unchanged.

commands.json records fresh vendor verification, four inventory/mutation tests,
HC parsing of both maps/four profile validations, and SpecPM validation. Structural
validation has zero errors; unresolved external capability warnings remain and
do not establish runtime conformance. No application, UI or performance tests
rerun locally for documentation-only changes; extraction receipts remain under
../hunter-tasks-20261015 with their original identities.
