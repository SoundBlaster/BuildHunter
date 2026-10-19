# Guarded scan assembly

Hunter now composes the synchronous guard/backend lifetime with an owned empty
queue and capped available parallelism. The host body receives its backend and a
single-use ScanSession token. Consuming `session.run` starts helpers/workers and
runs the coordinator on the caller; helpers stop and the queue closes before all
threads join. The host then drains profiles and publishes Finished inside the
same backend/guard lifetime. Backend releases before guard.

Root admission deliberately stays in Scanner::run, after worker startup, using
Hunter admit_root. Moving it before startup would change existing event/error and
cancellation ordering. BuildHunter retains guard factory/scope selection, root
policy, channels, QoS/profile setup, worker effects, classification, warnings,
status precedence and terminal events. Hunter does not gain SpecificationCore or
Rifle dependencies; its std-only core remains product-independent.

The launch token is neither Clone nor publicly constructible and run consumes it.
An unused token starts no threads. Guard/backend failures skip queue/cap selection
and body; zero cap panics after successful preparation. Hosts must move receivers
into the coordinator and release them before return/unwind join. Helpers/workers
must terminate on stop/closure. No guard-dependent handle may escape; external
backend borrows stay externally owned. Abort/double panic and broken destructors
are outside cleanup guarantees. There is no watchdog or memory bound.

See upstream specs/scan-assembly.spec.yaml (draft), current-scan session_assembly
and scoped_session claims, and specs/validation/hunter-assembly-20261019. Structural
validation and fixture tests do not establish live home/cloud or GUI acceptance.
