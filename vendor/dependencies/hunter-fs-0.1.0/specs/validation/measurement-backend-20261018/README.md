# Typed measurement backend validation

Five new tests cover typed ordering, original-position slots, ID/errors, borrowed
non-Send prepared state, pre-cancelled directory error and preparation, cancellation
prefixes, empty/omitted slots and prepared-state release on success/cancel/panic.
Initial absent-API failure is retained. Existing closure API and worker/scope tests
remain gates. Workspace Debug/Release, clippy/fmt, HC and SpecPM outputs are recorded.

Host owns native/batch acquisition, fallbacks, timing, cancellation, guards and
actual workers/join/status. Structural checks and draft contracts do not establish
owner acceptance, UI/home/cloud behavior, performance gains or memory budgets.
SpecPM warnings are retained dependency references and historical metadata ID reuse.
