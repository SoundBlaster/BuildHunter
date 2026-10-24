# Backend/guard lifetime validation

Five generic tests cover guard/backend/body order and reverse same-thread cleanup,
non-Send state/results, guard and backend setup failures with original error identity,
factory/body/destructor panic, and worker join before backend/guard release. A sixth
Darwin test exercises nested native guards and real thread-policy restoration on
unwind. Missing-API TDD failure is retained. Workspace Debug/Release, clippy/fmt,
HC and SpecPM receipts include current source/tool/output identities.

No live cloud provider, GUI/home scan or speedup/RAM bound is claimed. Host chooses
guards/backends and owns cloud paths/status/terminal. Cleanup assumes terminating
callbacks, no guard-destructor panic and ordinary unwind; abort/double panic are
excluded. Original Darwin best-effort restoration and no-op other platforms remain.
Historical receipts are historical; draft structure validation is not acceptance.
