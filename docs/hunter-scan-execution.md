# Serial scan result execution — 2026-10-15

Hunter `drive_scan` replaces the receive loop in `Scanner.run`. A synchronous
`ScanCoordinator` adapter exposes outstanding count, stop state, profile checkpoint,
receipt accounting and product-specific dispatch. No heap allocation, dynamic
dispatch or per-message task is introduced by the pump.

Before each wait the loop checks for zero outstanding work and then stop. It drains
profile samples at the checkpoint, receives a response, retires it in the task
ledger and synchronizes pending-task telemetry, then checks stop before dispatch.
Dispatch may enqueue more directory tasks or reserve/defer measurement responses.
An idle profile timeout repeats the loop/checkpoint without retiring a response.
No profiling selects the same blocking receive as before. A checkpoint-triggered
stop is observed after receipt or on the next timeout; the helper cannot interrupt
blocking syscalls/receive/callbacks. Disconnection breaks the loop without changing
outstanding count or inventing a new product status, matching the existing policy.

`Drained`, `Stopped` and `Disconnected` are loop exits only. BuildHunter still:

- enters the calling-thread cloud guard, admits/measures the selected root and
  selects classification/marker/cloud/error behavior;
- constructs the queue/channel, spawns workers/sampler with QoS/platform guards,
  and keeps shutdown guards inside the outer `thread::scope`;
- finalizes open results innermost first as partial, computes status precedence,
  resets terminal pending telemetry and emits `Finished` only after scope join.

The result receiver stays local to the scan call, borrowed by the pump rather than
stored in product state. It is moved into the scope closure and explicitly dropped
before scope join, also released there on unwind. Callback panic still unwinds through the existing shutdown
guards and scope join; worker panic still closes peers before unwinding. Nothing
is detached or moved to another callback thread. Public Rust/Swift/FFI shapes and
default/opt-in product policy remain unchanged.

Ten upstream tests cover execution boundaries. The application adds a deterministic
profile-callback cancellation test with both a queued setup error and an idle channel:
queued errors retire once without product dispatch; idle cancellation leaves one
unanswered task; profile pending counts agree in both cases. Existing nested sizes,
terminal events, cloud guards, ignored measurements, cancel/panic and independent
plugin scanner tests remain regression gates. Release fixtures compare the exact
runtime against main 10e8aeed on the same host; no speedup or RAM claim is inferred.

The exact ten-file std-only snapshot pins Hunter `3af2a4dd549f473e79364b45f248197ee4ec57ee` and preserves offline
root CLI builds. The independent plugin metadata Git pin remains unchanged.
Map/source/runtime receipts live in `specs/validation/hunter-execution-20261015`;
frozen maps and upstream draft contract are in `specs/evidence/hunter-execution-20261015`.
Contracts remain draft. Full root/backend execution assembly, descriptor-bound
recursion, channel/report budgets and live cloud/UI acceptance are still pending.
Merge Hunter first preserving the pinned commit in main ancestry, then BuildHunter.
