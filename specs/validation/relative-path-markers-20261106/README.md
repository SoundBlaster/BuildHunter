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

## Architecture validation details

The actual Hypercode executable was
`/Users/egor/Development/GitHub/0AL/Hypercode/.build/release/hypercode`, SHA-256
`ed236f86551590d3b0539c66e572cdfcdcf0ce567fa3913e2f2dfa0097dfc78b`.
Its contemporaneous source checkout was clean for tracked files at
`cb577e1bf68db79da34e465f63656c3cd79a7d53`; the binary hash identifies the
executable actually run and does not claim how it was built. This executable
does not support `--version`.

SpecPM reported `specpm 0.2.0` via `specpm --version`. Its editable source
checkout was clean for tracked files at `b4c292427da193e660a41881a08c6620c395f035`;
`src/specpm/cli.py` SHA-256 was
`23c1e8163f1862c28d13bac14e6974181bc335982aa48d94c66e3b23a759986a`.
Python distribution metadata separately reported `0.1.0`; the CLI version and
source identity above describe what was invoked.

From the BuildHunter worktree, the actual validation commands were:

```sh
/Users/egor/Development/GitHub/0AL/Hypercode/.build/release/hypercode validate specs/hypercode/current.hc --hcs specs/hypercode/current.hcs --ctx client=cli
/Users/egor/Development/GitHub/0AL/Hypercode/.build/release/hypercode validate specs/hypercode/current.hc --hcs specs/hypercode/current.hcs --ctx client=gui
specpm validate . --json
```

Both Hypercode commands exited zero with `ok: no issues found`. SpecPM exited
zero with status `warning_only`, zero errors and 54 unresolved-capability
warnings; those warnings are not treated as proof that cross-package
capabilities resolve. Local Rust was 1.97.0, Cargo 1.97.0, Python 3.10.19 and
Xcode 27.1 (27A9275). Source hashes are recorded separately.

Tests ran locally with build and test directories on TB5. These checks do not
establish signed-app behavior, descriptor confinement of the pathname clients,
or production/TestFlight delivery.
