# Code quality and duplication

Prefer shared components and implementations when behavior and responsibilities
match. Keep meaningful differences explicit and avoid extracting unrelated code
solely to improve a metric.

The advisory **Code quality report** workflow runs on Linux for source changes
and on manual dispatch. It uploads a seven-day artifact containing the analyzed
Git revision, a human-readable summary and a JSON report with clone locations.
Run it locally from the repository root with Node.js 22 and npm installed:

```sh
bash scripts/ci/code_quality.sh /path/to/report-directory
```

The script uses `jscpd` 5.2.0 (MIT) to scan Swift and Rust files under `src`,
`crates`, and `macos/Sources/BuildHunter`; vendored code and separate test
directories are outside this scope. Inline tests in source files are included.
It reports exact token clones of at least 8 lines and 50 tokens, source size and
the tool's per-file complexity estimate. These figures depend on the tool,
thresholds and scope: compare like with like and inspect the indicated fragments.
Repetition in tests or independently owned behavior can be intentional. Complexity
is a prompt for review, not a measurement of correctness or maintainability.

No duplication or complexity threshold blocks a change. Tool failures still fail
the report job; the workflow does not replace existing tests, Rust Clippy or
other CI checks and is not a required stack gate.
