# Backend/guard lifetime validation

Five new upstream lifetime tests, a relocated native Darwin nested/unwind test,
two product guard-failure/backend-panic witnesses and existing guarded probe/cancel/
callback-panic tests cover this slice. Workspace 97 tests (88 core + 9 Unix) pass
in Debug/Release; BuildHunter default 72, all-features 77 in each Debug/Release;
exact vendored core 88 in each mode; plugin scanner 12 in each mode; CLI 13 and
vendor 4, offline build and clippy/fmt pass. Results are in commands.json.

Runtime binding identifies the immutable source commit, and every vendored byte
matches the pinned Hunter Git object. Current HC also repairs the old unmatched
HunterChildTraversal selector and duplicate product-session/traversal declarations.
Four HC profiles and SpecPM are checked separately; source bindings/structural
success do not establish target acceptance. External-capability warnings remain.

The paired Release harness compares to main 86c3def with seven interleaved warm
fixture samples/eight regression budgets. This is not a speedup, RAM bound, live
provider or native UI/home acceptance claim. Native Rust Darwin policy witnesses
are distinct from a fresh UI run. Guard restoration remains best effort and does
not cover abort/double panic, broken guard Drop or escaping guard-dependent handles.
Historical receipts remain historical; full backend/engine assembly is pending.

All eight paired budgets pass: many-roots 77.509 -> 75.805 ms; source-tree
3.910 -> 3.149 ms. These fixture medians are not a general speedup claim.
SpecPM: zero errors, 30 external-capability/reference warnings; four HC profiles pass.
After execution, the upstream core README was corrected to describe the complete
snapshot (all src/*.rs); final integration pin 9c1e36bd7477c2ae5b73af4a5ef8ec589386fc8c changes only
that README and UPSTREAM provenance from the tested/performance candidate ee6ba0cc384d551f224a44c2a48109d48d0d24f1.
performance-inputs.json proves unchanged Rust/Cargo inputs and records both metadata
hash pairs. Fresh structural/vendor checks validate the final exact snapshot.
