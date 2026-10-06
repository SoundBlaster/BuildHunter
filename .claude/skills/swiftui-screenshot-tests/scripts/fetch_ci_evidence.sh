#!/usr/bin/env bash
# Download the macOS UI test evidence (xcresult, ui-screenshots, Screener traces)
# from GitHub Actions into a fresh directory.
#
#   fetch_ci_evidence.sh OUT_DIR [--branch NAME | --run RUN_ID] [--artifact NAME] [--repo OWNER/REPO]
#
# Defaults: the latest completed macos.yml run on the current git branch, artifact
# macos-test-evidence, repo from the origin remote. Prints the run it used.
set -euo pipefail

out="${1:?usage: fetch_ci_evidence.sh OUT_DIR [--branch NAME | --run RUN_ID] [--artifact NAME] [--repo OWNER/REPO]}"
shift
branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo main)"
run=""
artifact="macos-test-evidence"
repo="$(git remote get-url origin 2>/dev/null | sed -E 's#.*github\.com[:/]([^/]+/[^/.]+)(\.git)?$#\1#; s#.*/git/([^/]+/[^/.]+)$#\1#')"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --branch) branch="$2"; shift 2 ;;
    --run) run="$2"; shift 2 ;;
    --artifact) artifact="$2"; shift 2 ;;
    --repo) repo="$2"; shift 2 ;;
    *) echo "unknown option $1" >&2; exit 2 ;;
  esac
done

if [[ -e "$out" && -n "$(ls -A "$out" 2>/dev/null)" ]]; then
  echo "$out is not empty; pick a fresh directory so runs never mix" >&2
  exit 1
fi

if [[ -z "$run" ]]; then
  run="$(gh run list --repo "$repo" --workflow macos.yml --branch "$branch" --status completed \
    --limit 1 --json databaseId --jq '.[0].databaseId')"
  [[ -n "$run" && "$run" != "null" ]] || { echo "no completed macos.yml run on $branch" >&2; exit 1; }
fi

gh run view "$run" --repo "$repo" --json databaseId,headSha,conclusion,displayTitle \
  --jq '"run \(.databaseId) \(.conclusion) \(.headSha[0:7]) \(.displayTitle)"'
mkdir -p "$out"
gh run download "$run" --repo "$repo" --name "$artifact" --dir "$out"
find "$out" -maxdepth 2 -mindepth 1 | sort
