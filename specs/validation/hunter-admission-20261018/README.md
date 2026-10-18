# Root admission validation

Four new upstream seam tests complement existing host selected-root, pre-cancel,
symlink, missing-root and synthetic cloud-path witnesses. Fresh Debug/Release,
CLI/offline/plugin/vendor and clippy/fmt receipts are in commands.json. Runtime
bindings point to the immutable source commit; vendor bytes match upstream Git
objects. Structural validation covers four HC profiles and draft SpecPM claims;
external-capability warnings do not prove implementation conformance/acceptance.

Performance reports compare this runtime to main c4a0c2f on a shared warm fixture,
seven interleaved samples/eight ratio and noise-adjusted gates. They are regression
budgets, not a speedup, peak RSS or home/live-provider acceptance claim. No native
UI, real home scan or live provider was run in this slice. Existing guard/backend
assembly remains product-owned; prior evidence is historical.

Results: upstream workspace 91 tests (82 core + 9 Unix) in each Debug/Release;
BuildHunter default 71, all-features 76 in each Debug/Release; exact vendored core
82 in each mode; independent plugin scanner 12 in each mode; CLI 13 and vendor 4.
All eight paired performance budgets pass: many-roots 75.046 -> 74.958 ms,
source-tree 3.107 -> 3.090 ms. SpecPM has zero errors and 29 unresolved external
capability/reference warnings; all four HC profiles pass. This does not establish
an improvement beyond noise or validate the pending whole-process resource budgets.
