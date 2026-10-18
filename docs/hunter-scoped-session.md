# Scoped startup and terminal boundary — 2026-10-18

Hunter run_scoped_workers owns one thread scope: queue shutdown guard, host helper
startup, worker launch, caller-thread coordinator, helper-stop guard drop, queue
close and actual join. The result returns only after scoped threads finish. Host
startup/coordinator captures are owned within the scope so unused receivers can
release before join on startup unwind. Coordinator must move in and release unread
receivers before return/unwind and never return resources needed for worker exit.

BuildHunter scan_session::run keeps root guard admission, profile/QoS/channels,
coordinator, status precedence and event/report publication. Its helper callback
starts the existing sampler and returns its stop sender. The worker callback applies
QoS then calls the existing guarded worker. Failed worker guard remains SetupFailed
interpreted by the host pump; failed outer admission warns then finishes Failed
without launching work. After successful scope join the host drains final profile
samples and emits one Finished. Callback/worker panic unwinds without product
Finished. The FFI panic boundary and its own failure behavior are unchanged.

Five upstream tests cover startup/shutdown/join and capture release, including
non-Send caller state and blocked senders released by receiver drop. Two product
witnesses exercise the admission-failure handler and profile/terminal order. They
do not inject a real native guard-setup failure. Existing SetupFailed/cancel/panic,
worker/result, cloud/marker, bulk and plugin tests stay gates. Destructors must be
well-behaved and arbitrary host code must terminate for join; no watchdog/syscall
interruption or complete generic root/guard lifecycle is implemented.

The fifteen-file std-only snapshot pins Hunter 31fd0bad749cdac63bc42f36ffe72bcd81b0c1ad; runtime source binds to
BuildHunter b4647e5de8821db51a65b300922a1cb39c925e91. Independent metadata pin and offline CLI remain unchanged.
Draft contracts/maps and source/tool/runtime/paired Release receipts live under
specs/evidence/hunter-session-20261018 and specs/validation/hunter-session-20261018.
No speedup/RAM claim or local native UI/home/live-cloud acceptance is made. Merge
Hunter preserving the pinned ancestry before this consumer. Root admission/guards,
default plugin migration, descriptor recursion and resource budgets remain pending.
