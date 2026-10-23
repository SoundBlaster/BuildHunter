# Directory listing acquisition validation

Seven new macOS tests cover ordered owned entries, non-Clone/non-Default borrowed
context, observer and iteration errors, observer cancellation, failed/empty opens,
link types, callback panic and host-supplied iterator paths. Linux adds raw names.
Initial missing-API failure is retained in tdd-missing-api.log. Workspace Debug/
Release tests, clippy/fmt, HC and SpecPM checks are recorded with native outputs.

Host owns directory opening, platform guards, observer I/O/context meaning,
post-list hooks and final status. Existing visitor/worker/measurement/join tests
remain gates. SpecPM warnings are unresolved dependency references and a historical
metadata ID duplicate; see complete diagnostics in commands.json. Structural
checks and draft contracts are distinct from owner/runtime acceptance. No UI/home,
live-cloud, performance improvement or whole-scan memory budget is inferred.
