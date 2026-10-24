# R5 bounded profile backlog and resource disclosure

Pending profile retention has a 4,096-sample cap and exact producer-loss count,
separate from CLI history eviction. Catalog profile metadata, CLI/profile JSON
and GUI JSON export preserve resource estimates and loss. GUI resource series
break across omission and missing RAM intervals are not integrated. The numeric
88-byte C sample remains unchanged. Final profile/terminal ordering is covered.

Debug/Release all-feature tests passed (88 library + 5 CLI/profile); all-target/
all-feature Clippy passed. Debug test build and Release application build passed
in Xcode. A native Swift smoke verifies metadata round trip, series
gaps, RAM integration and 600-point history. These build/smoke results do not
claim full local XCTest execution. CI must execute native tests at the ready tip.
HC current/target/CI profiles and SpecPM passed (0 errors/49 unresolved-capability
warnings). Correctly invoked CLI/golden/vendor/dependency checks are recorded
separately; an initial unittest discovery was inappropriate for argv-driven CLI
scripts and its failed output is not a passing receipt.

Three Release CLI runs per baseline/input and candidate/input preserve results
within budgets. Home: about 120 to 46 MiB median kernel peak RSS with equal
44,451 findings/658 warnings/digest. CPU comparable, wall time higher in this
small warm-tree sample. Oversized listing returns explicit incomplete; its shorter
time is not an equivalent-work speedup. Sanitized aggregate receipts and exact
candidate source/binary identity are included; raw home paths remain local.

A real GUI home run reached two blocked native opens and did not complete. Its
performance popover revealed a separate retained font-cache/rendering issue. Heap
reference evidence, not a successful xctrace trace, motivated the next stack layer.
Neither whole-app C5 acceptance nor a GUI RSS ceiling is claimed here.
