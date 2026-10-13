# CI change policy

All three workflows start for PRs and pushes to main. A reusable Linux job
classifies the complete event diff before scheduling expensive jobs. Job-level
conditions preserve a completed/skipped check rather than suppressing the entire
workflow with path filters. See [GitHub job conditions](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-jobs-with-conditions).

Light-only changes are `docs/**`, `specs/**`, root Markdown files, `specpm.yaml`,
`scripts/test_plugin_contract.py`, and its pinned requirements file. These are
currently design/documentation inputs, not application build inputs. If schemas
or specs become runtime resources, remove them from this allowlist in that same PR.
Every other path, including source, assets, dependencies, build scripts, CI files
and unknown paths, requests all existing builds/tests/benchmarks. Mixed changes
also run everything. Plugin schema checks remain unconditional and lightweight.

PRs use base...head (merge-base cumulative diff); pushes use before..after.
`--no-renames` includes both sides of renames and prevents code moved into docs
from being treated as documentation-only. Null-separated paths avoid line-splitting
errors. Manual runs, initial pushes or an unavailable comparison request full CI.
No GitHub API file-list cap is involved. Failure of the classifier job itself is
visible as a failed workflow; its dependent heavy jobs cannot run on missing output.

The new CI-configuration PR itself intentionally runs full CI. Later docs-only
changes skip Rust/CLI matrices, macOS unit/UI/performance/Release jobs, and scanner
comparison matrices. Classifier tests run in the reusable job. Native skip behavior
is supplied by GitHub; local tests verify the path/event policy and actual Git diffs.
