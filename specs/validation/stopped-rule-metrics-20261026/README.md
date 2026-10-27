# Stopped rule metrics — 2026-10-26

Review: PR #121, discussion_r4374598957. Base head: 4559cc1c07bce068e40a16665cfc7356dae7d06b.
Runtime/test revision: `00609364edd5168c9f8653a3d86147d2e11bdba4`.
Current documentation/spec/map inputs are pinned by source-identity.json. Earlier profiling
and test-host receipts retain their original source/outcome scope.

## Behavior and ownership

GUI Stop cancels traversal, closes/discards ordinary queued events, increments
its generation and marks measuring rows partial immediately. An independent
bufferingNewest(1) terminal stream retains one copied validated rule-profile
snapshot plus its catalog and root-path safety flag after the Rust call returns.
A completed job remains discoverable while its original event stream is owned,
covering Stop racing with delivery of a completed aggregate. Stream close removes
the source registration; the detached worker independently retains native
control/context until return. No unbounded event replay or MainActor worker wait
is introduced. Rust, Rifle, event-8 wire format and ordinary queue bounds do not change.

The stopped model checks generation, report ID, catalog digest and exclusions.
Early Stop restores captured catalog metadata if its event was discarded, including
the unsafe-root flag. Ordinary late artifacts, completions, warnings and Finished
remain rejected. Rescan/replacement cancels the stopped consumer; the same catalog
digest alone cannot admit old metrics into a new report. The engine's terminal
status is retained, including a completion race.

## Checks

The unit suite passes 158 tests in 15 suites. New tests cover:
- Stop before/after catalog delivery, frozen rows/warnings and cancelled JSON metrics;
- root-path action safety retained by the late catalog handoff;
- late profiles after Rescan and target replacement;
- mismatched catalog digest/exclusions;
- a real Rust scan stopped with a one-slot backpressured event queue;
- a completed scan whose profile remains available through original stream ownership.

Final unlocked UI smoke passes all 3 selected tests (folder path/copy/Finder menu,
profile popover and template invalid-draft handling). The real-folder test now
activates its owner through the native Window menu before opening the chooser;
a companion window had obscured the earlier attempt. A failed earlier smoke and
a later interrupted command whose suite assertions completed are retained as such.
The owner reported the Mac locked during those attempts, then explicitly unlocked
it; the final successful command ran after that confirmation. No causal claim is
made that either window activation or unlocking alone explains all prior failures.
No new removable-volume prompt/mismatch was observed in the final run's TCC logs.

Debug build-for-testing and final Release build pass. The first Release attempt
was interrupted at its 180-second limit while compiling SwiftSyntax; the later
600-second-bounded invocation completed successfully. Both results are retained.
Build/test commands, actual outcomes and raw-log hashes are recorded in commands.json.
Full Xcode logs/results stay in the per-worktree verified TB5 build directory
(UUID 04B9F65E-894C-4A7C-B517-FC542E8D8E36), not in Git. Structural results are in
structural-commands.json. Build success, unit execution, UI smoke and CI are separate
evidence; no Store acceptance, instantaneous metrics or whole-process memory
ceiling is inferred. OS reads may delay the final snapshot. A failed catalog
preparation produces no metrics. No privacy or signing settings are changed by this fix.

## Architectural scope

Existing bridge cancellation/payload ownership, application generation and
RuleProfileCollection/Presentation roles compose the terminal handoff. It is
folded into these existing boundaries rather than adding a module/capability.
Current HCS bindings and bridge/application/profile constraints are refreshed;
current/target HC topology and target ownership are unchanged. Recovered specs
remain draft; this fixes the implementation conflict with the already requested
partial-metrics behavior rather than changing owner acceptance.
