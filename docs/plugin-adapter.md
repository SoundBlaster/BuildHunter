# Pure BuildHunter plugin adapter

Available as the independent Cargo package `crates/buildhunter-plugin-policy`.
It remains independently testable with its own lockfile. R4 connects it to default
CLI/GUI catalog scanning; Cargo source replacement preserves fresh-cache
offline CLI builds. Rifle is pinned to merge commit
`693a284123faaa1ad3192cd88b259ba97a7b5910`. Cargo downloads the public dependency
through HTTPS so hosted CI does not need an SSH key; developer Git remotes remain
SSH. [Client migration](catalog-client-migration.md) describes profile compatibility,
versioned event FFI, Settings and the separate stricter scoped scanner.

## Validation and immutable settings

`PluginCatalog::load(inputs, exclusions)` first asks Rifle to validate all generic
packages/programs and full-content identities. It then enforces the closed
BuildHunter plugin schema: format, text/IDs, exact facts/bindings, binding types,
marker components, target kinds and complete one-to-one rule assignment. A target
may own several rules; a rule may not belong to two targets. All targets/markers
are validated before applying exclusions, including disabled or unknown-origin
packages. Duplicate target IDs across packages are errors regardless of settings.

The host supplies `Origin`, never plugin JSON. Reserved `swift.*`, `rust.*` and
`python.*` IDs require Builtin origin AND their corresponding
`org.buildhunter.swift/rust/python` package owner. Marking an imported package as
Builtin is a host trust bug, not something data can authorize. Other namespace
ownership/authenticity policy remains host-owned; this validator only enforces
these reserved core namespaces and globally unique target IDs.

Limits are the contract's per-program/package/catalog budgets from Rifle plus
2048 targets and 512 distinct declared scope/name marker probes. No partial
snapshot escapes errors. Unknown well-formed exclusion IDs are retained; exclusions
are immutable and separate from catalog digest. Settings can inspect all target
metadata, including disabled entries.

After full validation the adapter compiles only enabled rules, removes their
unused fact declarations and produces the corresponding active bindings.
All-disabled packages have no active Plan. Marker requests from disabled-only
rules disappear. The original catalog identity still covers complete packages;
it does not change merely because settings change. Original metadata JSON and
unfiltered Rifle plans are not retained in the final snapshot.

## Classification, facts and probes

`marker_requests(kind)` returns deduplicated active probes. Own markers are not
requested for non-directories; symlink candidates have no requests and never
classify. This is a complete conservative probe set for the current adapter,
not a candidate-index optimization or instruction to probe all markers eagerly.

`classify(CandidateFacts)` consumes observations; it does not obtain them.
Node kind is an enum, name is lossless Unicode or None, and each required marker
is Known(bool) or Unavailable(reason). None name yields `invalid_name` unknown.
A missing required marker observation is an integration error, not false.
Own marker on a non-directory is false without a supplied observation.
The future scanner adapter must enforce no-follow/scope/cloud policies and map
out-of-scope/errors to unavailable; these are not proven by in-memory tests.

Rifle evaluates each active program. Rule results for a target use three-valued
OR; a true alternative suppresses irrelevant unknowns. Different targets resolve:

| State | Decision |
|---|---|
| No true or unknown | NoMatch |
| One true, no unknown | Found(target ID) |
| At least two true | Ambiguous, with true/unknown IDs |
| Zero/one true and at least one unknown | Indeterminate |

Matched and unknown target IDs are sorted. Relevant unavailable facts include
package ID to distinguish local fact names. No measurement, deletion, Finder
handoff, traversal pruning or final scan status is performed. Excluded, found,
ambiguous and indeterminate parents do not authorize skipping descendants.

## Evidence and next integration

Tests use the public Rust and synthetic Forge fixtures and negative manifests.
They cover origin, cross-package IDs, exact references/types, exclusions/probe
elimination, target OR, conflicts/unknowns, lossless-name handling and budgets.
These are pure policy tests, not filesystem, UI, ABI or performance evidence.
The legacy scanner suites remain regression coverage with default/all features.

Production scanner logic is unchanged. `scripts/test_offline_cli.py` builds and
installs the CLI with a fresh Cargo home and offline mode; CI runs it separately
from the explicit adapter Debug/Release tests.

```sh
cargo test --manifest-path crates/buildhunter-plugin-policy/Cargo.toml --locked
cargo test --manifest-path crates/buildhunter-plugin-policy/Cargo.toml --locked --release
```

The opt-in [scanner integration](plugin-scanner-integration.md) now observes
arbitrary candidates and uses descriptor-bound acquisition on macOS/Linux.
Default CLI/GUI use the separate `legacy-file-markers/1` compatibility path with
frozen C2 profiles, catalog/diagnostics through versioned FFI/Settings and rejection
of lossy GUI path actions. No JIT, plugin commands or private dependency is required.
