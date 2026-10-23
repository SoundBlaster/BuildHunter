# Scoped session startup/join validation

Five new tests cover helper stop before queue closure, all-thread join before
return, caller-thread borrowed non-Send results, opaque setup failures, coordinator
receiver release on unwind, unused coordinator capture release on startup panic,
and worker-panic peer closure. Initial missing-API failure is retained. Existing
low-level work/directory/measurement gates remain. Debug/Release, clippy/fmt, HC and
SpecPM commands include tool/source/output identities. Dependency-reference and
historical metadata-ID warnings are retained; structural checks are not acceptance.

Host owns root access, guards, QoS, sampler/body, setup/status/terminal protocol.
Callbacks/destructors must terminate and helpers must stop on guard drop. No syscall
interruption, watchdog, UI/home/cloud acceptance or resource improvement is claimed.
