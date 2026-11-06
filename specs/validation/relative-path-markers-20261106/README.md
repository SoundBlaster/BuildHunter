# Selected-root-relative facts and terminal marker links

The public policy adapter accepts a typed, host-supplied `node.relative_path`.
The default CLI/GUI clients and opt-in scanner supply only a losslessly encoded
lexical path beneath the selected root; absence is unknown. The default clients
also reject a symbolic link as the final marker component. This intentionally
changes the prior compatibility profile: the old corpus is unchanged and the
one changed expectation is explicit in both Rust and Swift tests. The default
clients still use pathname probes and may inspect the selected root's parent;
the stricter descriptor-scoped scanner remains a separate API.

Checks on the source files identified in `source-identity.json`:

- Root Rust library: 98 passed, including both client profiles and link cases.
- Plugin policy: 21 passed; independent plugin scanner: 19 passed.
- CLI integration: 15 passed; CLI migration parity: 3 passed; JSON contract
  tests: 13 passed.
- Unsigned macOS XCTest `BuildHunterTests/MigrationParityTests`: 3 passed,
  exercising the native GUI event source and its migration corpus.
- Hypercode `current.hc`/`current.hcs`, CLI and GUI contexts: no issues.
- SpecPM: 0 errors and 54 unresolved-capability warnings.

Tests ran locally with build and test directories on TB5. These checks do not
establish signed-app behavior, descriptor confinement of the pathname clients,
or production/TestFlight delivery.
