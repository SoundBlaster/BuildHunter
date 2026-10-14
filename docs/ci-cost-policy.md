# Hosted CI cost policy

Automatic checks are Ubuntu-only. Expensive Xcode jobs, non-Linux Rust runners
and comparative benchmarks require explicit workflow_dispatch. This controls
what BuildHunter schedules; it is not an account billing/spending-limit control.

| Workflow | PR/main push | Manual default | Explicit extra checks |
|---|---|---|---|
| Rust CI | Ubuntu fmt/tests/clippy/CLI/offline/schema checks; docs-only classification can skip builds | Ubuntu only | full_platform_checks=true adds macOS and Windows |
| macOS CI | Never | Release only | suite=tests, performance or all |
| Scanner performance | Never | One Ubuntu benchmark job | runner=macos-26; baseline_ref selects comparison revision |

Superseded Rust runs cancel for the same PR/ref. Existing manual workflows retain
their cancellation, timeouts and evidence retention. Local macOS checks remain
available through `scripts/ci/macos.sh`, so turning off automatic jobs does not
remove the tests or lower their assertions. macOS/UI and performance regressions
must be checked locally or explicitly requested before relevant releases/merges.

`python scripts/ci/test_workflow_policy.py` guards automatic-event runner selection,
manual defaults, absence of PR/push triggers for expensive workflows, one-runner
benchmarks and cancellation. It runs in the Ubuntu schema job; its PyYAML version
is pinned in `scripts/plugin-contract-requirements.txt`.

## Emergency pause and reactivation

On 2026-10-14, Rust CI, macOS CI and Scanner performance were disabled through the
GitHub workflow API to immediately stop the old automatic policy while this
change awaits merge. No queued/in-progress BuildHunter runs remained. Re-enabling
Rust CI before this policy reaches main would restore the old multi-OS matrix.

After merging this policy into main, verify the default-branch workflow files,
then re-enable Rust CI. macOS CI and Scanner performance can stay disabled; enable
one only when an explicit manual run is wanted. Run controls and YAML triggers
are separate: disabled workflows do not accept manual dispatch until enabled.

```sh
gh workflow enable ci.yml --repo SoundBlaster/BuildHunter
# Only for an explicitly requested suite, after main has the manual-only policy:
gh workflow enable macos.yml --repo SoundBlaster/BuildHunter
gh workflow run macos.yml --repo SoundBlaster/BuildHunter -f suite=tests
```

No full macOS/benchmark dispatch or account billing change is part of this policy
validation. Historical green runs remain evidence for their original source only.
