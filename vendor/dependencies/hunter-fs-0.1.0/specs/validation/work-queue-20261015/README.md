# Scoped work extraction evidence

23 macOS workspace tests (9 scoped + 14 core) pass Debug/Release, plus clippy/fmt/HC.
SpecPM has no errors; unresolved host capability warnings remain. Recursive task
expansion covers seven tasks; panic test requires every peer to exit before the
coordinator guard drops, avoiding a false pass from coordinator timeout. An initial
test fixture exited the coordinator before starting the panic and was corrected;
that was test synchronization, not a production shutdown witness.
Queue releases pending payloads on close outside the mutex, rejects future jobs,
and keeps in-flight ownership with workers. Thread Scope, not spawn_workers return,
is the join boundary. Host owns guard placement, setup/QoS/cancellation/results.
No full recursive filesystem engine or pending-memory budget is claimed.
