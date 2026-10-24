# Execution provenance review correction

PR #8 review correctly identified an unrelated environment-wide output/cache
inventory. Replace it with the verified volume identity and actual compiler/fixture
locations only. Keep the original command receipts, including Cargo metadata:
they prove the build routing required by the owner and contain toolchain facts,
not runtime search/classification vocabulary. AGENTS.md clarifies this distinction;
the prohibition on product/ecosystem classification in Hunter remains unchanged.

All ten canonical distribution files are byte-identical to 3af2a4d. BuildHunter
may keep that immutable source pin; no dependency or runtime update is needed.
Current roles/profiles, capabilities, contracts and parent assembly obligations
are unchanged. Initial Debug/Release/clippy receipts retain their exact runtime
scope. Fresh HC parse/validation and SpecPM (0 errors / 3 existing warnings) pass.
No runtime tests were rerun for this documentation/provenance correction.
