"""Read-only CI planning and trusted status publication for native PR stacks.

The publisher runs from the default branch, never from PR code. Artifacts are
bounded JSON data; actual successful jobs are checked independently of receipts.
"""
import argparse
import io
import json
import os
from pathlib import Path
import re
import urllib.error
import urllib.parse
import urllib.request
import zipfile

from changes import changed_paths, requires_heavy

CONTEXT = "Stack CI gate"
WORKFLOWS = {"ci.yml": "rust", "macos.yml": "macos", "performance.yml": "performance"}
MAX_BODY = 8 * 1024 * 1024


class NotReady(RuntimeError):
    """Missing, stale, ambiguous or failed evidence must never permit merge."""


class StripRedirectAuth(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        redirected = super().redirect_request(req, fp, code, msg, headers, newurl)
        if redirected is not None and urllib.parse.urlsplit(req.full_url).netloc != urllib.parse.urlsplit(newurl).netloc:
            redirected.remove_header("Authorization")
        return redirected


class API:
    def __init__(self, repository, token=None):
        if not re.fullmatch(r"[\w.-]+/[\w.-]+", repository):
            raise ValueError("Invalid repository")
        self.repository = repository
        self.root = "https://api.github.com/repos/" + repository + "/"
        self.token = token or os.environ.get("GH_TOKEN") or os.environ.get("GITHUB_TOKEN")
        self.opener = urllib.request.build_opener(StripRedirectAuth())

    def request(self, path, data=None, binary=False):
        headers = {"Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2026-03-10"}
        if self.token:
            headers["Authorization"] = "Bearer " + self.token
        body = None if data is None else json.dumps(data).encode()
        req = urllib.request.Request(self.root + path, body, headers)
        try:
            with self.opener.open(req, timeout=30) as response:
                raw = response.read(MAX_BODY + 1)
        except urllib.error.HTTPError as error:
            raise NotReady(f"GitHub API HTTP {error.code}: {path.split('?')[0]}") from None
        if len(raw) > MAX_BODY:
            raise NotReady("Oversized GitHub response")
        return raw if binary else json.loads(raw)

    def pages(self, path, key=None):
        result = []
        for page in range(1, 21):
            value = self.request(path + ("&" if "?" in path else "?") + f"per_page=100&page={page}")
            rows = value[key] if key else value
            result.extend(rows)
            if len(rows) < 100:
                return result
        raise NotReady("Pagination limit reached")

    def reviews(self, number):
        owner, name = self.repository.split("/")
        cursor = None
        for _ in range(20):
            query = """query($owner:String!,$name:String!,$number:Int!,$cursor:String){
              repository(owner:$owner,name:$name){pullRequest(number:$number){
                reviewDecision reviewThreads(first:100,after:$cursor){
                  nodes{isResolved} pageInfo{hasNextPage endCursor}
                }}}}"""
            # Separate endpoint, still restricted to the fixed GitHub API host.
            payload = json.dumps({"query": query, "variables": {
                "owner": owner, "name": name, "number": number, "cursor": cursor}}).encode()
            req = urllib.request.Request("https://api.github.com/graphql", payload, {
                "Authorization": "Bearer " + (self.token or ""), "Content-Type": "application/json"})
            with self.opener.open(req, timeout=30) as response:
                raw = response.read(MAX_BODY + 1)
            if len(raw) > MAX_BODY:
                raise NotReady("Oversized review response")
            result = json.loads(raw)
            if result.get("errors"):
                raise NotReady("Review state unavailable")
            pr = result["data"]["repository"]["pullRequest"]
            if pr["reviewDecision"] not in (None, "", "APPROVED"):
                raise NotReady("Required review is incomplete or changes were requested")
            threads = pr["reviewThreads"]
            if any(not thread["isResolved"] for thread in threads["nodes"]):
                raise NotReady("Unresolved review threads")
            if not threads["pageInfo"]["hasNextPage"]:
                return
            cursor = threads["pageInfo"]["endCursor"]
        raise NotReady("Review pagination limit reached")


def sha(value):
    if not isinstance(value, str) or not re.fullmatch(r"[0-9a-f]{40}", value):
        raise NotReady("Invalid revision")
    return value


def same_repo(pr, repository):
    return pr["head"]["repo"]["full_name"].lower() == repository.lower()


def plan(api, number, enabled):
    pr = api.request(f"pulls/{number}")
    if pr["state"] != "open":
        raise NotReady("PR is not open")
    stack = pr.get("stack")
    members = [pr]
    base_ref = pr["base"]["ref"]
    stack_number = None
    if stack is not None:
        stack_number = int(stack["number"])
        record = api.request(f"stacks/{stack_number}")
        if not record["open"]:
            raise NotReady("Stack is closed")
        base_ref = record["base"]["ref"]
        members = [api.request(f"pulls/{int(p['number'])}") for p in record["pull_requests"]
                   if p["state"] == "open" and p.get("merged_at") is None]
        if not members or number not in [p["number"] for p in members]:
            raise NotReady("Inconsistent stack membership")
        previous = None
        for member in members:
            if (not same_repo(member, api.repository) or member.get("stack", {}).get("number") != stack_number
                    or member["state"] != "open"):
                raise NotReady("Unverified native stack member")
            expected_ref = base_ref if previous is None else previous["head"]["ref"]
            if member["base"]["ref"] != expected_ref:
                raise NotReady("Non-linear stack bases")
            if previous is not None:
                parent = sha(previous["head"]["sha"])
                comparison = api.request(f"compare/{parent}...{sha(member['head']['sha'])}")
                if (sha(member["base"]["sha"]) != parent
                        or comparison["merge_base_commit"]["sha"] != parent):
                    raise NotReady("Stack needs a cascading rebase")
            previous = member
        tip_metadata = members[-1]["stack"]
        if tip_metadata["position"] != tip_metadata["size"]:
            raise NotReady("Stack tip metadata disagrees")
    base_sha = sha(api.request("branches/" + urllib.parse.quote(base_ref, safe=""))["commit"]["sha"])
    if enabled and stack is None and base_ref != "main":
        raise NotReady("Link dependent PRs into a native stack before optimized validation")
    head = sha(pr["head"]["sha"])
    comparison = api.request(f"compare/{base_sha}...{head}")
    if stack is not None and comparison["merge_base_commit"]["sha"] != base_sha:
        raise NotReady("Rebase stack onto the current trunk before validation")
    files = comparison.get("files")
    # Compare returns at most 300 files; a potentially truncated result is heavy.
    heavy = files is None or len(files) >= 300
    if files is not None:
        paths = [f["filename"] for f in files]
        paths += [f["previous_filename"] for f in files if "previous_filename" in f]
        heavy = heavy or requires_heavy(paths)
    ready = all(not p["draft"] for p in members)
    tip = members[-1]["number"]
    return {
        "schema": 1, "repository": api.repository, "pr": number, "head": head,
        "base_ref": base_ref, "base": base_sha, "stack": stack_number,
        "members": [{"pr": p["number"], "head": sha(p["head"]["sha"]), "draft": p["draft"]}
                    for p in members],
        "tip": tip, "ready": ready, "enabled": enabled, "heavy": heavy,
        "macos": heavy and (not enabled or (ready and number == tip)),
    }


def evidence(expected):
    # A layer's tests bind its own immutable sources and trunk, not the draft
    # state/identity of future upper layers. A changed tip still needs new proof.
    return {key: expected[key] for key in [
        "schema", "repository", "pr", "head", "base_ref", "base", "enabled", "heavy", "macos"]}


def receipt(raw):
    if len(raw) > 1024 * 1024:
        raise NotReady("Oversized plan archive")
    try:
        with zipfile.ZipFile(io.BytesIO(raw)) as archive:
            if archive.namelist() != ["ci-plan.json"] or archive.getinfo("ci-plan.json").file_size > 65536:
                raise NotReady("Invalid plan archive contents")
            return json.loads(archive.read("ci-plan.json"))
    except (zipfile.BadZipFile, ValueError, KeyError) as error:
        raise NotReady("Unreadable plan receipt") from error


def required_jobs(workflow, expected):
    names = ["changes / classify"]
    if workflow == "ci.yml":
        names += ["Plugin contract schemas"]
        if expected["heavy"]:
            names += ["Rust and CLI (ubuntu-latest)", "Rust and CLI (windows-latest)"]
        if expected["macos"]:
            names += ["Rust and CLI (macos-26)"]
    elif workflow == "performance.yml":
        if expected["heavy"]:
            names += ["Scanner comparison (ubuntu-latest)"]
        if expected["macos"]:
            names += ["Scanner comparison (macos-26)"]
    elif workflow == "macos.yml" and expected["macos"]:
        names += ["macOS unit and UI tests", "macOS performance tests", "macOS Release build"]
    return names


def verify_workflow(api, workflow, expected):
    query = urllib.parse.urlencode({"head_sha": expected["head"], "event": "pull_request"})
    runs = api.pages(f"actions/workflows/{workflow}/runs?{query}", "workflow_runs")
    runs = [r for r in runs if r["head_sha"] == expected["head"] and r["event"] == "pull_request"
            and r["path"].split("@", 1)[0] == ".github/workflows/" + workflow
            and any(p["number"] == expected["pr"] for p in r["pull_requests"])]
    if not runs:
        raise NotReady(f"Missing {workflow} for PR {expected['pr']}")
    run = max(runs, key=lambda r: r["id"])
    if run["status"] != "completed" or run["conclusion"] != "success":
        raise NotReady(f"{workflow} is not successful for PR {expected['pr']}")
    jobs = api.pages(f"actions/runs/{run['id']}/jobs?filter=latest", "jobs")
    for name in required_jobs(workflow, expected):
        matching = [j for j in jobs if j["name"] == name]
        if len(matching) != 1 or matching[0]["status"] != "completed" or matching[0]["conclusion"] != "success":
            raise NotReady(f"Missing successful job: {name}")
    artifacts = api.pages(f"actions/runs/{run['id']}/artifacts", "artifacts")
    matching = [a for a in artifacts if a["name"] == "ci-plan-" + WORKFLOWS[workflow] and not a["expired"]]
    if len(matching) != 1:
        raise NotReady("Missing or ambiguous plan receipt")
    actual = receipt(api.request(f"actions/artifacts/{matching[0]['id']}/zip", binary=True))
    comparable = evidence(expected)
    # A full old run is stronger than a currently unnecessary lower-layer macOS
    # run; never accept the reverse for a newly selected tip.
    if not comparable["macos"] and actual.get("macos") is True:
        comparable["macos"] = True
    if actual != comparable:
        raise NotReady("Plan receipt is stale or inconsistent with current stack/base/configuration")


def verify(api, number, enabled, require_tip=False):
    expected = plan(api, number, enabled)
    if require_tip and number != expected["tip"]:
        raise NotReady("Only the tested stack tip may be selected for batch merge")
    if not expected["ready"]:
        raise NotReady("Mark lower layers ready, then mark the final tip ready")
    for member in expected["members"]:
        current = plan(api, member["pr"], enabled)
        if current["members"] != expected["members"] or current["base"] != expected["base"]:
            raise NotReady("Stack changed during verification")
        for workflow in WORKFLOWS:
            verify_workflow(api, workflow, current)
        api.reviews(member["pr"])
    if plan(api, number, enabled) != expected:
        raise NotReady("Stack changed before publication")
    return expected


def publish(api, number, enabled):
    # Only reset/attest live head revisions; never trust SHAs from event input.
    initial = plan(api, number, enabled)
    state, description = "success", "Current batch CI evidence verified; merge only the tested tip"
    try:
        verified = verify(api, number, enabled)
    except NotReady as error:
        verified = initial
        state, description = "pending", str(error)
    for member in verified["members"]:
        live = api.request(f"pulls/{member['pr']}")
        if live["head"]["sha"] != member["head"]:
            raise NotReady("Head changed before status publication")
        api.request(f"statuses/{member['head']}", {
            "state": state, "context": CONTEXT, "description": description[:140],
            "target_url": f"https://github.com/{api.repository}/pull/{verified['tip']}",
        })
    return state, description


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["plan", "publish", "verify-merge"])
    parser.add_argument("--pr", type=int)
    args = parser.parse_args()
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text()) if os.environ.get("GITHUB_EVENT_PATH") else {}
    repository = os.environ.get("GITHUB_REPOSITORY", "SoundBlaster/BuildHunter")
    api = API(repository)
    enabled = os.environ.get("STACK_MACOS_CI", "false") == "true"
    if args.mode == "plan":
        if os.environ.get("GITHUB_EVENT_NAME") == "pull_request":
            number = event["pull_request"]["number"]
            result = plan(api, number, enabled)
            if result["head"] != event["pull_request"]["head"]["sha"]:
                raise NotReady("Stale workflow head")
            event_base = (event["pull_request"].get("stack") or {}).get("base", {}).get("sha") or event["pull_request"]["base"]["sha"]
            if event_base != result["base"]:
                raise NotReady("Stale workflow base; rebase and rerun")
            Path(os.environ["RUNNER_TEMP"], "ci-plan.json").write_text(json.dumps(evidence(result), sort_keys=True) + "\n")
        else:
            paths = changed_paths(os.environ.get("GITHUB_EVENT_NAME"), os.environ.get("BASE_SHA"), os.environ.get("HEAD_SHA"))
            heavy = paths is None or requires_heavy(paths)
            result = {"heavy": heavy, "macos": heavy}
        include = [{"os": "ubuntu-latest", "suffix": ""}, {"os": "windows-latest", "suffix": ".exe"}]
        if result["macos"]:
            include.append({"os": "macos-26", "suffix": ""})
        outputs = {"heavy": str(result["heavy"]).lower(), "macos": str(result["macos"]).lower(),
                   "rust_matrix": json.dumps({"include": include}),
                   "performance_os": json.dumps(["ubuntu-latest"] + (["macos-26"] if result["macos"] else []))}
        with open(os.environ["GITHUB_OUTPUT"], "a") as output:
            for key, value in outputs.items():
                output.write(f"{key}={value}\n")
        print(json.dumps(result, sort_keys=True))
    elif args.mode == "verify-merge":
        if not args.pr:
            parser.error("--pr is required")
        print(json.dumps(verify(api, args.pr, enabled, require_tip=True), sort_keys=True))
    else:
        if not enabled:
            print("Stack macOS optimization inactive; existing CI policy is unchanged")
            return
        # Refresh all open PRs: trunk/membership changes can invalidate other layers.
        prs = api.pages("pulls?state=open")
        # Invalidate first: schema/API errors or a cancelled publisher must not
        # leave a previous green status on a head we have already observed.
        for pr in prs:
            api.request(f"statuses/{sha(pr['head']['sha'])}", {
                "state": "pending", "context": CONTEXT, "description": "Refreshing current batch CI evidence"})
        handled = set()
        for pr in prs:
            if pr["number"] in handled:
                continue
            try:
                current = plan(api, pr["number"], enabled)
                state, description = publish(api, pr["number"], enabled)
                handled.update(m["pr"] for m in current["members"])
            except (NotReady, KeyError, TypeError, ValueError, urllib.error.URLError) as error:
                # Planning errors also invalidate any previously green head status.
                live = api.request(f"pulls/{pr['number']}")
                api.request(f"statuses/{sha(live['head']['sha'])}", {
                    "state": "pending", "context": CONTEXT,
                    "description": ("Cannot verify current CI plan: " + type(error).__name__)[:140],
                })
                state, description = "pending", "Plan unavailable"
            print(pr["number"], state, description)


if __name__ == "__main__":
    main()
