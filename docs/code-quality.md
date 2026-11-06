# Code quality and duplication

Prefer shared components and implementations when behavior and responsibilities
match. Keep meaningful differences explicit and avoid extracting unrelated code
solely to improve a metric.

The advisory **Code quality report** workflow runs on Linux for source changes
and on manual dispatch. It uploads a seven-day artifact containing the analyzed
Git revision, a human-readable summary and a JSON report with clone locations.
Run it locally from the repository root with Node.js 22, npm and Python installed:

```sh
python3 -m pip install tree-sitter==0.25.2 tree-sitter-rust==0.24.0
bash scripts/ci/code_quality.sh /path/to/report-directory
```

The script uses `jscpd` 5.2.0 (MIT) to scan Swift and Rust files under `src`,
`crates`, and `macos/Sources/BuildHunter`; vendored code and test directories
are outside this scope. The pinned Tree-sitter Rust parser also removes inline
test-only items and their external modules before scanning. Conditional code
which can run in production is retained. Staging preserves source line numbers.
It reports exact token clones of at least 8 lines and 50 tokens, source size and
the tool's per-file complexity estimate. These figures depend on the tool,
thresholds and scope: compare like with like and inspect the indicated fragments.
The reported byte size includes whitespace used to mask test-only code, so it
is not a production-only size metric. Clone locations retain repository paths.
Repetition in independently owned behavior can be intentional. Complexity
is a prompt for review, not a measurement of correctness or maintainability.

No duplication or complexity threshold blocks a change. Tool failures still fail
the report job; the workflow does not replace existing tests, Rust Clippy or
other CI checks and is not a required stack gate.

For deliberately independent code, a narrowly scoped `jscpd:ignore-start` /
`jscpd:ignore-end` comment pair suppresses the region. State the reason next to
the markers; do not exclude an entire file to silence one known match.
