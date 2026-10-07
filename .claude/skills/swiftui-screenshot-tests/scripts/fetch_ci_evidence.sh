#!/usr/bin/env bash
# Download the macOS UI test evidence (xcresult, ui-screenshots, Screener traces)
# from GitHub Actions into a fresh directory.
#
#   fetch_ci_evidence.sh OUT_DIR [--branch NAME | --commit SHA | --run RUN_ID]
#                        [--artifact NAME] [--repo OWNER/REPO]
#
# Defaults: the newest macos.yml run on the current git branch that finished
# (success or failure; cancelled runs have no evidence), artifact
# macos-test-evidence, repo from the origin remote. Prints the run and full SHA.
set -euo pipefail

usage='usage: fetch_ci_evidence.sh OUT_DIR [--branch NAME | --commit SHA | --run RUN_ID] [--artifact NAME] [--repo OWNER/REPO]'
out="${1:?$usage}"
shift
branch=""
commit=""
run=""
artifact="macos-test-evidence"
repo=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --branch) branch="$2"; shift 2 ;;
    --commit) commit="$2"; shift 2 ;;
    --run) run="$2"; shift 2 ;;
    --artifact) artifact="$2"; shift 2 ;;
    --repo) repo="$2"; shift 2 ;;
    *) echo "unknown option $1" >&2; echo "$usage" >&2; exit 2 ;;
  esac
done

if [[ -z "$repo" ]]; then
  remote="$(git remote get-url origin 2>/dev/null || true)"
  repo="$(sed -E 's#^.*github\.com[:/]##; s#^.*/git/##; s#\.git$##' <<<"$remote" | awk -F/ 'NF>=2 {print $(NF-1)"/"$NF}')"
fi
[[ -n "$repo" ]] || { echo "cannot tell the repository; pass --repo OWNER/REPO" >&2; exit 2; }

if [[ -e "$out" && -n "$(ls -A "$out" 2>/dev/null)" ]]; then
  echo "$out is not empty; pick a fresh directory so runs never mix" >&2
  exit 1
fi

if [[ -z "$run" ]]; then
  filter=(--branch "${branch:-$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo main)}")
  [[ -n "$commit" ]] && filter=(--commit "$commit")
  run="$(gh run list --repo "$repo" --workflow macos.yml "${filter[@]}" --status completed --limit 20 \
    --json databaseId,conclusion \
    --jq '[.[] | select(.conclusion == "success" or .conclusion == "failure")][0].databaseId')"
  [[ -n "$run" && "$run" != "null" ]] || { echo "no finished macos.yml run for ${filter[*]}" >&2; exit 1; }
fi

gh run view "$run" --repo "$repo" --json databaseId,headSha,headBranch,conclusion,displayTitle \
  --jq '"run \(.databaseId) \(.conclusion) \(.headBranch) \(.headSha)\n    \(.displayTitle)"'
mkdir -p "$out"
gh run download "$run" --repo "$repo" --name "$artifact" --dir "$out"
find "$out" -maxdepth 1 -mindepth 1 | sort
