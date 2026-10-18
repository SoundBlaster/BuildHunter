"""Offline adversarial fixtures for CI applicability, identity and merge evidence."""
import copy
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch, Mock
import os
import urllib.request
import urllib.parse
import zipfile

from stack_ci import (API, NotReady, StripRedirectAuth, WORKFLOWS, evidence, main,
                      plan, publish, receipt, required_jobs, verify, verify_workflow)
from stack_ci_ruleset import proposal


BASE, LOWER, TIP, OTHER = (letter * 40 for letter in "abcd")
REPO = "example/project"


def archive(value):
    data = io.BytesIO()
    with zipfile.ZipFile(data, "w") as zipped:
        zipped.writestr("ci-plan.json", json.dumps(value))
    return data.getvalue()


class Fixture:
    repository = REPO

    def __init__(self, stacked=True, heavy=True):
        self.base = BASE
        self.heavy = heavy
        self.prs = {}
        self.calls = []
        self.runs, self.jobs, self.artifacts, self.zipfiles = {}, {}, {}, {}
        self.bad_reviews = set()
        for number, head, ref, parent, base in [(1, LOWER, "lower", "main", BASE), (2, TIP, "tip", "lower", LOWER)]:
            self.prs[number] = {"number": number, "state": "open", "draft": False,
                "head": {"ref": ref, "sha": head, "repo": {"full_name": REPO}},
                "base": {"ref": parent, "sha": base},
                "stack": {"number": 7, "position": number, "size": 2} if stacked else None}
        if not stacked:
            self.prs = {1: self.prs[1]}

    def request(self, path, data=None, binary=False):
        self.calls.append((path, data))
        if path.startswith("statuses/"):
            return {}
        if path.startswith("pulls/"):
            return copy.deepcopy(self.prs[int(path.split("/")[1])])
        if path == "stacks/7":
            return {"open": True, "base": {"ref": "main"}, "pull_requests": list(self.prs.values())}
        if path.startswith("branches/"):
            return {"commit": {"sha": self.base}}
        if path.startswith("compare/"):
            base = path.split("/")[1].split("...")[0]
            return {"merge_base_commit": {"sha": base},
                    "files": [{"filename": "src/lib.rs" if self.heavy else "docs/a.md"}]}
        if path.endswith("/zip"):
            return self.zipfiles[int(path.split("/")[2])]
        raise AssertionError("Unexpected request: " + path)

    def pages(self, path, key=None):
        self.calls.append((path, None))
        if path.startswith("actions/workflows/"):
            workflow = path.split("/")[2]
            head = path.split("head_sha=")[1].split("&")[0]
            return copy.deepcopy(self.runs.get((workflow, head), []))
        if path.startswith("actions/runs/"):
            run = int(path.split("/")[2])
            return copy.deepcopy(self.jobs[run] if "/jobs" in path else self.artifacts[run])
        if path == "pulls?state=open":
            return copy.deepcopy(list(self.prs.values()))
        raise AssertionError("Unexpected pages: " + path)

    def reviews(self, number):
        if number in self.bad_reviews:
            raise NotReady("Unresolved review threads")

    def green(self, enabled=True):
        self.runs, self.jobs, self.artifacts, self.zipfiles = {}, {}, {}, {}
        for number in self.prs:
            expected = plan(self, number, enabled)
            for index, (workflow, name) in enumerate(WORKFLOWS.items(), start=1):
                run = number * 10 + index
                self.runs[workflow, expected["head"]] = [{"id": run, "head_sha": expected["head"],
                    "event": "pull_request", "path": ".github/workflows/" + workflow,
                    "pull_requests": [{"number": number}], "status": "completed", "conclusion": "success"}]
                self.jobs[run] = [{"name": n, "status": "completed", "conclusion": "success"}
                                  for n in required_jobs(workflow, expected)]
                self.artifacts[run] = [{"id": run, "name": "ci-plan-" + name, "expired": False}]
                self.zipfiles[run] = archive(evidence(expected))


class PlanTests(unittest.TestCase):
    def test_ready_stack_only_tip_gets_all_five_macos_jobs(self):
        api = Fixture()
        lower, tip = plan(api, 1, True), plan(api, 2, True)
        self.assertTrue(lower["heavy"] and tip["heavy"])
        self.assertFalse(lower["macos"])
        self.assertTrue(tip["macos"])
        self.assertIn("Rust and CLI (ubuntu-latest)", required_jobs("ci.yml", lower))
        self.assertNotIn("Rust and CLI (macos-26)", required_jobs("ci.yml", lower))
        self.assertEqual(len(required_jobs("macos.yml", tip)), 4)

    def test_disabled_rollout_preserves_full_validation_on_each_layer(self):
        api = Fixture()
        for number in api.prs:
            self.assertTrue(plan(api, number, False)["macos"])

    def test_ordinary_pr_is_full_and_draft_waits_only_after_activation(self):
        api = Fixture(stacked=False)
        self.assertTrue(plan(api, 1, True)["macos"])
        api.prs[1]["draft"] = True
        self.assertFalse(plan(api, 1, True)["macos"])
        self.assertTrue(plan(api, 1, False)["macos"])

    def test_lower_drafts_hold_tip_until_whole_batch_ready(self):
        api = Fixture()
        api.prs[1]["draft"] = True
        self.assertFalse(plan(api, 2, True)["macos"])

    def test_docs_only_skips_expensive_jobs_without_claiming_test_pass(self):
        api = Fixture(heavy=False)
        for number in api.prs:
            expected = plan(api, number, True)
            self.assertFalse(expected["heavy"] or expected["macos"])
            self.assertEqual(required_jobs("macos.yml", expected), ["changes / classify"])

    def test_tip_uses_trunk_cumulative_changes_not_its_direct_parent_delta(self):
        api = Fixture()
        plan(api, 2, True)
        self.assertIn((f"compare/{BASE}...{TIP}", None), api.calls)

    def test_unknown_or_truncated_diff_is_heavy_and_renamed_source_is_not_hidden(self):
        for files in [None, [{"filename": "docs/a.md"}] * 300,
                      [{"filename": "docs/code.rs", "previous_filename": "src/code.rs"}]]:
            api = Fixture()
            original = api.request
            def request(path, data=None, binary=False):
                value = original(path, data, binary)
                if path.startswith("compare/"):
                    value["files"] = files
                return value
            api.request = request
            self.assertTrue(plan(api, 2, True)["heavy"])

    def test_foreign_member_invalid_base_and_non_linear_history_fail_closed(self):
        for change in [lambda a: a.prs[1]["head"]["repo"].update(full_name="fork/repo"),
                       lambda a: a.prs[2]["base"].update(ref="wrong"),
                       lambda a: a.prs[2]["base"].update(sha=OTHER),
                       lambda a: a.prs[2]["stack"].update(position=1)]:
            api = Fixture()
            change(api)
            with self.assertRaises(NotReady):
                plan(api, 2, True)

    def test_diverged_stack_needs_rebase(self):
        api = Fixture()
        original = api.request
        def request(path, data=None, binary=False):
            value = original(path, data, binary)
            if path.startswith("compare/"):
                value["merge_base_commit"]["sha"] = OTHER
            return value
        api.request = request
        with self.assertRaises(NotReady):
            plan(api, 2, True)

    def test_unregistered_dependent_pr_cannot_silently_take_tip_only_policy(self):
        api = Fixture(stacked=False)
        api.prs[1]["base"]["ref"] = "other-branch"
        with self.assertRaises(NotReady):
            plan(api, 1, True)
        self.assertTrue(plan(api, 1, False)["macos"])


class EvidenceTests(unittest.TestCase):
    def test_full_ready_batch_and_current_reviews_pass(self):
        api = Fixture()
        api.green()
        self.assertEqual(verify(api, 2, True, require_tip=True)["tip"], 2)

    def test_lower_prefix_is_rejected_by_merge_preflight(self):
        api = Fixture()
        api.green()
        with self.assertRaisesRegex(NotReady, "tip"):
            verify(api, 1, True, require_tip=True)

    def test_failed_pending_cancelled_skipped_or_missing_macos_job_never_passes(self):
        for conclusion in ["failure", None, "cancelled", "skipped", "missing"]:
            api = Fixture()
            api.green()
            jobs = api.jobs[22]
            job = next(j for j in jobs if j["name"] == "macOS unit and UI tests")
            if conclusion == "missing":
                jobs.remove(job)
            else:
                job["conclusion"] = conclusion
            with self.assertRaises(NotReady):
                verify(api, 2, True)

    def test_stale_base_head_budget_configuration_or_receipt_never_pass(self):
        for change in [lambda a: setattr(a, "base", OTHER),
                       lambda a: a.prs[2]["head"].update(sha=OTHER),
                       lambda a: a.zipfiles.update({22: archive({})}),
                       lambda a: a.zipfiles.update({22: archive({**evidence(plan(a, 2, True)), "enabled": False})})]:
            api = Fixture()
            api.green()
            change(api)
            with self.assertRaises(NotReady):
                verify(api, 2, True)

    def test_wrong_workflow_pr_or_event_cannot_supply_evidence(self):
        for key, value in [("path", ".github/workflows/evil.yml"), ("event", "workflow_dispatch"),
                           ("pull_requests", [{"number": 99}]), ("head_sha", OTHER)]:
            api = Fixture()
            api.green()
            api.runs["macos.yml", TIP][0][key] = value
            with self.assertRaises(NotReady):
                verify(api, 2, True)

    def test_newer_run_pending_cannot_reuse_old_green_run(self):
        api = Fixture()
        api.green()
        old = api.runs["macos.yml", TIP][0]
        api.runs["macos.yml", TIP].append({**old, "id": 999, "status": "in_progress", "conclusion": None})
        with self.assertRaises(NotReady):
            verify(api, 2, True)

    def test_inconsistent_success_conclusion_with_running_job_is_not_green(self):
        api = Fixture()
        api.green()
        api.jobs[21][0]["status"] = "in_progress"
        with self.assertRaises(NotReady):
            verify(api, 2, True)

    def test_ready_changes_do_not_invalidate_lower_cheap_evidence(self):
        api = Fixture()
        api.prs[2]["draft"] = True
        api.green()
        lower_receipts = {n: api.zipfiles[n] for n in [11, 12, 13]}
        api.prs[2]["draft"] = False
        api.green()
        api.zipfiles.update(lower_receipts)
        verify(api, 2, True)

    def test_full_lower_run_is_acceptable_but_skipped_tip_is_not(self):
        api = Fixture()
        api.green()
        expected = plan(api, 1, True)
        api.zipfiles[12] = archive({**evidence(expected), "macos": True})
        verify_workflow(api, "macos.yml", expected)
        api.zipfiles[22] = archive({**evidence(plan(api, 2, True)), "macos": False})
        with self.assertRaises(NotReady):
            verify(api, 2, True)

    def test_expired_or_duplicate_artifact_does_not_pass(self):
        for duplicate in [True, False]:
            api = Fixture()
            api.green()
            if duplicate:
                api.artifacts[22] *= 2
            else:
                api.artifacts[22][0]["expired"] = True
            with self.assertRaises(NotReady):
                verify(api, 2, True)

    def test_unresolved_review_blocks_whole_batch(self):
        api = Fixture()
        api.green()
        api.bad_reviews.add(1)
        with self.assertRaisesRegex(NotReady, "review"):
            verify(api, 2, True)

    def test_publish_pending_and_success_use_live_member_heads(self):
        for green in [False, True]:
            api = Fixture()
            if green:
                api.green()
            state, _ = publish(api, 2, True)
            self.assertEqual(state, "success" if green else "pending")
            posts = [(p, d) for p, d in api.calls if p.startswith("statuses/")]
            self.assertEqual({p for p, _ in posts}, {"statuses/" + LOWER, "statuses/" + TIP})
            self.assertTrue(all(d["state"] == state for _, d in posts))


class TransportTests(unittest.TestCase):
    def test_review_required_and_unresolved_threads_are_rejected(self):
        for decision, unresolved in [("REVIEW_REQUIRED", False), ("CHANGES_REQUESTED", False), ("APPROVED", True)]:
            api = API(REPO, "test-token")
            data = {"data": {"repository": {"pullRequest": {"reviewDecision": decision,
                "reviewThreads": {"nodes": [{"isResolved": not unresolved}],
                                  "pageInfo": {"hasNextPage": False, "endCursor": None}}}}}}
            api.opener = Mock()
            api.opener.open.return_value = io.BytesIO(json.dumps(data).encode())
            with self.assertRaises(NotReady):
                api.reviews(1)

    def test_archive_is_read_as_bounded_json_without_extracting_paths(self):
        self.assertEqual(receipt(archive({"schema": 1})), {"schema": 1})
        for contents in [b"not zip", b"x" * (1024 * 1024 + 1)]:
            with self.assertRaises(NotReady):
                receipt(contents)
        stream = io.BytesIO()
        with zipfile.ZipFile(stream, "w") as zipped:
            zipped.writestr("../ci-plan.json", "{}")
        with self.assertRaises(NotReady):
            receipt(stream.getvalue())

    def test_redirect_drops_token_on_other_host(self):
        req = urllib.request.Request("https://api.github.com/a", headers={"Authorization": "Bearer secret"})
        redirected = StripRedirectAuth().redirect_request(req, None, 302, "", {}, "https://blob.example/a")
        self.assertIsNone(redirected.get_header("Authorization"))

    def test_pagination_does_not_silently_stop_after_first_page(self):
        api = API(REPO)
        api.request = lambda path: ([1] * 100 if urllib.parse.parse_qs(path.split("?", 1)[1])["page"] == ["1"] else [2])
        self.assertEqual(api.pages("pulls"), [1] * 100 + [2])
        api.request = lambda path: [1] * 100
        with self.assertRaises(NotReady):
            api.pages("pulls")

    def test_manual_plan_keeps_full_matrix_without_live_api(self):
        with tempfile.TemporaryDirectory() as folder:
            output = Path(folder, "output")
            env = {"GITHUB_EVENT_NAME": "workflow_dispatch", "GITHUB_OUTPUT": str(output)}
            with patch.dict("os.environ", env, clear=True), patch("sys.argv", ["stack_ci.py", "plan"]):
                main()
            result = output.read_text()
            self.assertIn("heavy=true", result)
            self.assertIn("macos=true", result)
            self.assertIn("macos-26", result)

    def test_event_entrypoint_rejects_stale_head_or_base_without_receipt(self):
        for key in ["head", "base"]:
            api = Fixture(stacked=False)
            event = {"pull_request": copy.deepcopy(api.prs[1])}
            event["pull_request"][key]["sha"] = OTHER
            with tempfile.TemporaryDirectory() as folder:
                event_file = Path(folder, "event.json")
                event_file.write_text(json.dumps(event))
                env = {"GITHUB_EVENT_NAME": "pull_request", "GITHUB_REPOSITORY": REPO,
                       "GITHUB_EVENT_PATH": str(event_file), "RUNNER_TEMP": folder,
                       "GITHUB_OUTPUT": str(Path(folder, "output")), "STACK_MACOS_CI": "true"}
                with patch.dict(os.environ, env, clear=True), patch("sys.argv", ["stack_ci.py", "plan"]), patch("stack_ci.API", return_value=api):
                    with self.assertRaises(NotReady):
                        main()
                self.assertFalse(Path(folder, "ci-plan.json").exists())

    def test_event_entrypoint_outputs_linux_only_matrix_for_lower_layer(self):
        api = Fixture()
        event = {"pull_request": copy.deepcopy(api.prs[1])}
        event["pull_request"]["stack"]["base"] = {"sha": BASE}
        with tempfile.TemporaryDirectory() as folder:
            event_file = Path(folder, "event.json")
            event_file.write_text(json.dumps(event))
            env = {"GITHUB_EVENT_NAME": "pull_request", "GITHUB_REPOSITORY": REPO,
                   "GITHUB_EVENT_PATH": str(event_file), "RUNNER_TEMP": folder,
                   "GITHUB_OUTPUT": str(Path(folder, "output")), "STACK_MACOS_CI": "true"}
            with patch.dict(os.environ, env, clear=True), patch("sys.argv", ["stack_ci.py", "plan"]), patch("stack_ci.API", return_value=api):
                main()
            self.assertNotIn("macos-26", Path(folder, "output").read_text())
            self.assertEqual(json.loads(Path(folder, "ci-plan.json").read_text()), evidence(plan(api, 1, True)))

    def test_disabled_publisher_does_not_write_any_status(self):
        api = Fixture()
        with patch.dict(os.environ, {}, clear=True), patch("sys.argv", ["stack_ci.py", "publish"]), patch("stack_ci.API", return_value=api):
            main()
        self.assertFalse(api.calls)

    def test_publisher_invalidates_old_success_before_plan_schema_failure(self):
        api = Fixture()
        del api.prs[1]["base"]
        with patch.dict(os.environ, {"STACK_MACOS_CI": "true"}, clear=True), patch("sys.argv", ["stack_ci.py", "publish"]), patch("stack_ci.API", return_value=api):
            main()
        posts = [(p, d) for p, d in api.calls if p.startswith("statuses/")]
        self.assertEqual({p for p, _ in posts}, {"statuses/" + LOWER, "statuses/" + TIP})
        self.assertTrue(all(d["state"] == "pending" for _, d in posts))


class RulesetTests(unittest.TestCase):
    def original(self):
        return {"id": 123, "name": "main", "target": "branch", "enforcement": "active",
                "conditions": {"ref_name": {"include": ["refs/heads/main"], "exclude": []}},
                "bypass_actors": [], "rules": [
                    {"type": "non_fast_forward"}, {"type": "deletion"},
                    {"type": "pull_request", "parameters": {"required_review_thread_resolution": True}}]}

    def test_proposal_preserves_all_existing_rules_and_is_idempotent(self):
        original = self.original()
        expected = copy.deepcopy(original)
        result = proposal(original, 15368)
        self.assertEqual(original, expected)
        self.assertEqual(result["rules"][:-1], original["rules"])
        self.assertEqual(result["conditions"], original["conditions"])
        self.assertNotIn("id", result)
        self.assertEqual(proposal(result, 15368), result)

    def test_existing_required_checks_are_not_removed(self):
        original = self.original()
        original["rules"].append({"type": "required_status_checks", "parameters": {
            "required_status_checks": [{"context": "Existing", "integration_id": 1}],
            "strict_required_status_checks_policy": False}})
        result = proposal(original, 15368)
        params = result["rules"][-1]["parameters"]
        self.assertIn({"context": "Existing", "integration_id": 1}, params["required_status_checks"])
        self.assertTrue(params["strict_required_status_checks_policy"])

    def test_inactive_rule_or_wrong_existing_gate_producer_are_rejected(self):
        original = self.original()
        original["enforcement"] = "evaluate"
        with self.assertRaises(ValueError):
            proposal(original, 15368)
        original = proposal(self.original(), 1)
        with self.assertRaises(ValueError):
            proposal(original, 15368)


if __name__ == "__main__":
    unittest.main()
