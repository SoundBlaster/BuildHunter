# Resource budget work — 2026-10-24

Goal: provide host-selected limits without blocking the coordinator behind response
producers, and release completed registry storage. This is descriptive draft work;
application defaults and whole-process RSS acceptance are separate integration.

| HC role | Consumer | Contract / status | Evidence | Next obligation |
| --- | --- | --- | --- | --- |
| WorkCoordination/TaskQueue + QueueAdmission | Recursive host coordinator | hunter.fs.work; opt-in count/weight limits and exact ownership-preserving rejection | resource-foundations-20261024 | Consumer must stop after rejection; weight units are host-owned |
| TaskLedger/TaskIdentity | Long-running serial traversal | hunter.fs.task_ledger; sparse live slots with monotonic IDs | 100,000 sequential task regression | Bound peak concurrent admission separately |
| ScanAssembly/ResponseChannel | Worker producers and caller-thread engine | hunter.fs.scan_assembly; optional bounded/rendezvous responses | Full-buffer and factory-panic producer-release tests | Consumer must release receiver before join; limit individual payloads |

| DirectoryAcquisition/ListingRetention | Portable and scoped event consumers | hunter.fs.directory_listing; ordered retained prefix with explicit typed limit | bounded-listings-20261024 | Host must stop/report incomplete; opaque iterator/context allocations remain host-owned |

| DarwinBatchMetadata/BatchRetention | Optional macOS metadata preparation | hunter.fs.darwin_bulk_metadata; retained observation/weight caps | bounded-metadata-20261024 | Host must report limit; fixed batch buffer and table overhead excluded |

Next layer: BuildHunter adoption of bounded owned listing payloads.
Then BuildHunter must adopt limits, bound report/diagnostic/profile storage, preserve
partial/terminal semantics and measure end-to-end CPU/RSS/latency. This foundation
alone does not establish a whole-process memory bound or close R5/C5.
