# Stack-aware CI budget

The implementation is opt-in. `STACK_MACOS_CI` is absent/false by default, so
applicable existing macOS checks continue on each PR. No repository setting or
ruleset is changed by this PR. Hunter keeps its inexpensive platform tests.

| Validation | Ordinary ready PR | Ready native stack |
| --- | --- | --- |
| Linux/Windows Rust and CLI, schemas, Linux scanner comparison | Applicable cumulative diff | Applicable on every layer |
| macOS Rust/CLI, scanner comparison, Xcode UI/performance/Release | Applicable cumulative diff | Complete cumulative diff on tip only |
| Documentation only | Classifier/policy/schema checks | Same; never reported as a macOS test pass |
| Draft batch | Full legacy behavior until activation | After activation, no expensive macOS work until all members are ready |
| Push to main / manual run | Existing full applicable validation | Retained; concurrency cancels obsolete runs for the same ref |

Prepare bounded stacks of about 3–5 focused PRs. Mark lower layers ready first,
then the final tip ready. Explicit ready-for-review and stacked lifecycle triggers
start the applicable validation. Ordinary PRs without stack metadata never get the
lower-layer exemption. Unregistered dependent branches are rejected in optimized
mode. Stale event heads/bases, unknown API shapes and non-linear/outdated stacks
block validation; rebase onto the current trunk and rerun.

## Plans and the required gate

The shared classifier reads current native stack membership and trunk SHA from
GitHub. Compare against the trunk for the full cumulative diff; do not classify
just a tip's delta against its parent. Preserve both rename paths; uncertain or
300-file/truncated comparisons request heavy validation. Record bounded JSON plan
receipts per workflow. A receipt binds PR number, exact head/base, heavy applicability
and budget mode. Draft state and future upper layers are not test inputs for a
lower layer, so unchanged lower cheap results need not be rerun when a tip is added.
An old full run can supply stronger lower-layer evidence; a skipped lower run can
never supply a new tip's macOS evidence.

The publisher checks all current members, latest matching pull_request runs for
three fixed workflow paths, every applicable successful job, and each run's exact
plan artifact. Expired/duplicate/stale artifacts, cancelled/failed/skipped jobs,
wrong PR/workflow/event and pending newer runs cannot pass. It also queries all
GraphQL review-thread pages and review decisions. Status is `pending` until the
current combined evidence is valid; failure details remain in the original CI jobs.
Publisher first invalidates observed head statuses, rechecks live identities before
publication and uses the constant `Stack CI gate` context. Root pushes refresh all
open PRs; strict up-to-date rules prevent old-base validation from permitting merge.

The write-token publisher runs only trusted default-branch code. It never executes
PR code or extracts archives. Artifacts are bounded JSON data, and successful job
checks are independent of artifact assertions. Cross-host HTTP redirects strip the
GitHub token. Unknown/missing/paginated API state cannot silently become success.
Ordinary GitHub API/event delivery races still exist; perform a fresh preflight
immediately before requesting a merge. API outages leave the gate pending.

## Safe rollout after this stack merges

1. Keep `STACK_MACOS_CI=false` while merging the implementation. The publisher must
   exist on `main`; workflow_run is not activated merely by adding it to a PR.
2. Read the current main ruleset and actual GitHub Actions app ID. Generate and
   review an **additive** payload with `scripts/ci/stack_ci_ruleset.py --input
   /tmp/ruleset-before.json --actions-app-id <id>`. It preserves all existing rules,
   bypass actors and required checks, adds the producer-bound `Stack CI gate`, and
   enables strict up-to-date checks. The script has no mutation mode.
3. Apply the reviewed ruleset payload **before** enabling the variable. This blocks
   merges until gate evidence is available, rather than creating an unguarded window.
   Then set repository variable `STACK_MACOS_CI=true` and manually dispatch
   `stack-gate.yml` from main. Do not substitute this publisher for actual test runs.
4. Rebase a small native canary onto current main. Verify cheap checks on all layers,
   five macOS jobs on its ready tip, and a successful producer-bound gate on every
   member. Check that changing the tip/base or leaving a lower review thread open
   blocks the gate. Stale old runs without new receipts must not be accepted.
5. Run the authenticated read-only preflight immediately before a whole-stack merge:
   `STACK_MACOS_CI=true python3 scripts/ci/stack_ci.py verify-merge --pr <tip>`.
   Supply an authenticated `GH_TOKEN`/`GITHUB_TOKEN` without storing it in the repo.
   Only after it passes use `gh stack merge <tip>`. GitHub still enforces the current
   native-stack approval/thread/ruleset requirements at merge time.

**Enforcement boundary:** commit statuses cannot distinguish a whole-stack merge
request from a prefix request when the same lower commits participate in both.
The provided preflight rejects a prefix; agents/operators must use it. Enforcing
that choice against a maintainer intentionally bypassing the preflight needs a
restricted merge bot/GitHub App; this implementation does not claim that restriction.
Never merge an intermediate state solely because a higher combined state passed.

Rollback is explicit: keep merges blocked while disabling optimization and restoring
full applicable runs; inspect fresh evidence before adjusting the gate requirement.
Do not silently remove requirements or re-label skipped jobs as passing tests.

## Evidence and limits

Offline adversarial fixtures cover scheduling, source/base/configuration identity,
actual jobs, receipts, reruns, readiness, reviews, prefix rejection, archive/token
handling and additive ruleset planning. A live read-only standalone docs-PR plan
and current/proposed ruleset snapshots are recorded. Actionlint v1.7.12 runs per file;
only its outdated `stacked` enum diagnostic is narrowly excluded, and the existing
custom `xcode-27` runner label is declared. Shellcheck/pyflakes are not run by this
lint command. Hosted parser/CI results are separate from local validation.

Required-gate activation, native canary, actual privileged status publication and
measured runner-minute savings remain pending until the post-merge rollout above.
Runtime scan/bridge/plugin contracts are unchanged. CI has a separate HC map and
BoundarySpec; it is intentionally outside the application's runtime composition.

References: [native CI metadata](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/optimizing-ci-for-stacked-pull-requests),
[stack API](https://docs.github.com/en/rest/pulls/stacks),
[workflow_run](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run).
