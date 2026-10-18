# Native canary acceptance — 2026-10-19

Native #90 (#88 → #89) passed all applicable CI at tip 74b8c6b and trunk d01d4ed.
The timed pre-merge read-only receipt and status snapshots are preserved verbatim;
job placement was 0 successful macOS jobs on the lower layer and 5 on the tip.
The preflight was repeated immediately before the authorized native stack merge.
`merged-stack.json` is a later API read confirming both PRs merged, ending at main
1a09c34071c2a9e0a6dc5615bc99ee2d664f2531. No closed PR was retested or old receipts
relabeled as validation of a newer main/consumer. Measured money/time savings and
runtime scanner acceptance are outside this evidence. Commands: `gh pr view 88`
and `89 --repo SoundBlaster/BuildHunter --json number,state,headRefOid,mergedAt,mergeCommit,url`;
preflight argv/time are retained in verify-merge-final.json.
