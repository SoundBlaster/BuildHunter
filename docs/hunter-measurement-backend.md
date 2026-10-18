# Typed measurement preparation — 2026-10-18

Hunter DirectoryMeasurementBackend observes directory metadata and prepares owned
state implementing FileMeasurementBackend. DirectoryMeasurementPlan::measure_with
and the existing closure API share one original-position file loop. Directory
metadata then preparation still run on directory failure, pre-cancellation and
empty/omitted slots; cancellation retains an aligned prefix. Prepared state may
borrow external resources without Send/Sync/boxing requirements and drops before
return or on callback/stop unwind. The host retains effective guards through drop.

BuildHunter src/measurement.rs owns native MetadataBackend and PreparedFiles. It
retains optional bulk reads, test injections, per-entry consumption, Unsupported/
missing native fallback, profiling counters and materialization error decoration.
Other bulk errors retain their original code/message and do not trigger fallback.
WorkerAdapter only times/delegates the call. Its outer thread guard, Listed-first
publication, cancelled Measured result, queue/receiver lifetime, join and terminal
status retain their existing boundaries. No metadata/batch implementation moved
into Hunter; that separate platform ownership decision remains pending.

Five new upstream tests cover typed preparation/measurement and state release.
Two new host witnesses check pre-cancelled directory error still prepares and
synthetic batch consumption/fallback/error provenance under the active guard.
Existing closure API, marker/cloud, bulk, profile/cancel/panic, worker/join and
plugin tests remain gates. Paired Release fixtures compare against dec3ad60.
No speedup, RAM budget, local native UI/home scan or live cloud acceptance is claimed.

The exact fourteen-file std-only snapshot pins Hunter 1eb300054c24bbf56a936287fbd586c4fb39079c. Runtime source binds
to BuildHunter c2cd2b857d73ff98b41f9653276918c23018cc5d; the independent scoped-metadata Git pin and fresh-cache
offline CLI remain supported. Draft contracts/maps and source/tool/command/performance
receipts are under specs/evidence/hunter-measurement-20261018 and
specs/validation/hunter-measurement-20261018. Merge Hunter preserving the pinned
commit before this consumer. Full startup/guard assembly, descriptor recursion,
default plugin migration and C5 resource budgets remain outstanding.
