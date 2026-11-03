# CI thread-status refresh validation

The workflow adds a five-minute schedule to the existing trusted default-branch
publisher. The scheduled path checks review-thread state without rewriting an
unchanged commit status. When a pending batch becomes eligible, it invalidates
any still-green member before full batch verification. The existing event-driven
publisher and `buildhunter.delivery.ci` obligations remain unchanged. GitHub may delay scheduled jobs; their actual execution after
merge has not been observed. This triggers up to 288 scheduled runs per day.

- `python3 -B -m unittest discover -s scripts/ci -p test_stack_ci.py`: 45 passed.
- Hypercode compiler SHA-256 `6503784100372b8fe93d0a794caf80bfbfc47da880874b2b9177267d34a26b9f`:
  `validate specs/hypercode/ci.hc --hcs specs/hypercode/ci.hcs --ctx layer=tip`
  and `--ctx layer=lower`: no issues.
- `specpm 0.2.0 validate . --json`: 0 errors, 54 unresolved-capability warnings.

These checks validate the model and local CI policy, not a hosted scheduled run.
File hashes and the base commit are in `source-identity.json`.
