# Guarded backend lifetime

Hunter owns the synchronous sequence used by the coordinator and workers:

```text
enter host-selected guard
prepare host-selected backend
run body (all scoped work must join here)
drop owned backend
drop guard / restore same-thread policy
```

`with_guarded_backend` returns the original opaque `Guard` or `Backend` setup error.
Guard failure skips backend/body; backend failure releases guard and skips body.
Factories/body stay on the caller thread and may borrow non-Send state. Ordinary
unwind drops owned backend before guard, including a single backend destructor
panic. Guard Drop must not panic. Abort/double panic do not guarantee restoration.
The host must not extract handles needing the guard into a return value/external
state; externally borrowed state and unused callback captures are not owned by the
backend cleanup-order guarantee.

| Integration | Protected lifetime | Failure interpretation |
| --- | --- | --- |
| Coordinator | Borrowed Probes attachment, setup, workers/sampler join, final profiles and Finished | Existing Warning then Finished(Failed) for guard failure |
| Worker | Owned WorkerAdapter, listing, prepared metadata use/drop, result publication | Existing SetupFailed message for guard failure |
| Prepared files | Owned state inside synchronous measurement call | Existing per-entry errors/fallbacks |

Actual product backend factories are infallible. Generic backend setup errors are
an upstream API branch, not a new product status. Product guards choose the upstream
`NonMaterializingIoGuard`: on macOS it stores the previous thread I/O policy, disables
materialization and restores it best effort on Drop. PhantomData<Rc<()>> keeps it
non-Send/non-Sync. Other OS use the original no-op. There is no universal provider
or race-free/no-follow guarantee; native metadata, marker exception and bulk
algorithms are unchanged. BuildHunter keeps cloud path rejection and dataless
error messages; its existing public guard name is a source-compatible alias.

Five generic lifetime tests, a real Darwin nested/unwind test, two host worker
failure/panic tests and existing guarded probe/cancel/callback-panic tests cover
this boundary. Draft claims/receipts distinguish these witnesses from native UI,
home/live-provider acceptance and pending whole-process resource budgets.
