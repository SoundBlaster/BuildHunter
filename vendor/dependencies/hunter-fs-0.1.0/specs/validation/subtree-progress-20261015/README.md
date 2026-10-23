# Subtree progress extraction evidence

Eight tests first failed to compile before the public API existed; tdd-missing-api.log
preserves the red phase (API development, not an existing bug). They cover empty
roots/readiness, early/late descendant completion, separate nested states, saturated
sums, leaf replacement, partial flags, forced prefixes and one-shot completion.
Invalid counter retirement also has an explicit panic expectation.

macOS workspace: 9 scoped + 39 core tests pass in Debug/Release; clippy/fmt, HC and
SpecPM pass with zero errors and three existing warnings (two unresolved platform
requirements, one duplicate metadata section ID). Raw commands/tools/source hashes
are recorded. Initial YAML parse failure remains in initial-commands.json; only the
plain-scalar wording was fixed. Rust code/lint runs apply to the same Rust sources,
and corrected structural validation was repeated.

Contract stays draft. Membership, dependent completion order, unit choice, actual
worker lifetime and final status remain host duties. No full-tree or resource/
performance acceptance follows from these math tests.
