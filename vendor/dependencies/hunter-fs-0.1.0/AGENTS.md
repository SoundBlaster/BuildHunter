# Repository instructions

- Hunter must not depend on BuildHunter or Rifle or contain build-artifact classification vocabulary (ecosystem names, target/filter IDs or artifact categories).
  Generic documentation schema fields such as SpecPM `foreignArtifacts` do not denote build artifacts.
  This restriction concerns runtime search/classification policy. Normal Rust toolchain names and build/test provenance (including Cargo output metadata) are not classification vocabulary.
- Preserve raw paths, no-follow scope and observable error semantics.
- Maintain maps and draft specs alongside code; distinguish tests from guarantees.
- Prefix shell commands with `rtk`; Git remotes use SSH.
- Use no-wait for long work. Preserve unrelated changes.
