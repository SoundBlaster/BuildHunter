# Native backend/root observation validation

Five macOS core tests cover directory/file root facts, exact one-call/error/raw-path
preservation, borrowed backend state, listing paths/open failures and final symlinks
to files/directories/missing targets. Linux adds a non-UTF-8 root test. The initial
absent-API compile failure is retained in tdd-missing-api.log.
Workspace Debug/Release tests, clippy/fmt, HC parse/validation and SpecPM pass.
SpecPM has zero errors and four existing/platform dependency warnings.
Actual commands, source/tool identity and actual compiler/fixture output locations
are recorded; Cargo metadata confirms the external target. No unrelated cache inventory.

Only final-component metadata no-follow is provided. Path-based intermediate
components/read_dir can follow links and replacement races. Host owns access/cloud
admission, guard/iterator lifetime, intentional marker following, classification,
scheduling, cancellation and final status. No content reads, complete engine startup,
fixed memory budget, live cloud/UI or performance improvement is inferred.
Contracts remain draft; historical evidence is untouched.
