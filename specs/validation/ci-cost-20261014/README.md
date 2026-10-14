# Hosted CI cost controls — 2026-10-14

Owner requested disabling expensive automatic BuildHunter runs after reporting
GitHub charges. This change makes automatic events Ubuntu-only and requires
explicit manual opt-in for Xcode, non-Linux Rust runners and benchmarks.

[Commands](commands.json): 7 cost-policy tests, 5 change-classification tests,
plugin schema tests, actionlint v1.7.12 (registered existing xcode-27 label),
HC maps/four profiles and SpecPM passed locally. Runtime maps/requirements are
unchanged; CI scheduling is an operational product policy outside runtime maps.
[Input identities](source-identity.json) and [GitHub state](github-state.json)
record source and external pause separately. Prior architecture evidence stays
historical. No expensive hosted workflow was dispatched for validation.

Rust CI, macOS CI and Scanner performance were disabled immediately on GitHub;
no queued/running BuildHunter runs remained. Merge the policy before re-enabling
Rust CI; the main branch currently has the old matrix. Expensive workflows may
stay disabled until an explicit manual run is needed. The inspected main ruleset
requires PR/thread resolution but no expensive required status checks.
No account billing setting or dollar-savings estimate is claimed.
