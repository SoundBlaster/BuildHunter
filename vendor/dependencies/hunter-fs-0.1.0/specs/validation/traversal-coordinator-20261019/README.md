# Traversal coordinator validation

Baseline main 7fb8b0827cf777ce852ecb6ca94bdcad017ce6fb; immutable runtime/core pin
10268f69c41890bb23ab4cdc33d25f3aac8e46d1. Workspace Debug/Release: 108 unit tests
(99 core + 9 Unix), plus one compile-fail doctest. Seven new coordinator witnesses
cover recursive task/measurement receipts, borrowed root skip without observation,
root error preservation, cancellation before dispatch/after timeout, disconnection,
queue rejection and callback panic/deferred-state release. Fmt, all-target clippy,
HC parse/validation and SpecPM pass; command outputs and tool/source hashes retained.

These are local macOS Rust checks on TB5 outputs, not Windows/Linux, live cloud,
home-directory or GUI acceptance, general speedup or whole-process resource bounds.
The coordinator owns admission/task accounting and drives opaque host interpretation;
child-routing policy, selected-subtree membership, measurements/report and statuses
remain host responsibilities. New BoundarySpec remains draft. Old admission/ledger/
result-loop/lifecycle requirements are composed without broadening their scope.
