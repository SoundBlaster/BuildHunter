# Directory worker protocol validation

Seven new tests cover listing/measurement publication order, intent and host
prefetch, listing-only tasks, rejection at both results, cancellation after listing,
closed queues, continued LIFO work and callback panic. Borrowed non-Send host
state/context and non-Copy IDs are exercised. Initial absent-API failure is
retained in tdd-missing-api.log. Existing measurement-slot and scoped join tests
remain gates. Workspace Debug/Release, clippy/fmt, HC and SpecPM commands are recorded.

No platform/backend policy, worker join, full-engine lifetime or memory budget
is implemented here. Callbacks retain guard/cancellation/error/timing policy.
No app UI/home/cloud or performance improvement is inferred. Contracts stay draft.
