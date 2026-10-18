"""Prepare an additive ruleset payload; never mutate GitHub from this script."""
import argparse
import copy
import json
from pathlib import Path


def proposal(original, actions_app_id):
    if original.get("target") != "branch" or original.get("enforcement") != "active":
        raise ValueError("An active branch ruleset is required")
    if not isinstance(actions_app_id, int) or actions_app_id <= 0:
        raise ValueError("Read the actual GitHub Actions app ID from a repository check")
    result = {key: copy.deepcopy(original[key]) for key in
              ["name", "target", "enforcement", "conditions", "rules", "bypass_actors"]}
    matches = [r for r in result["rules"] if r["type"] == "required_status_checks"]
    if len(matches) > 1:
        raise ValueError("Ambiguous existing required-check rules")
    if not matches:
        matches = [{"type": "required_status_checks", "parameters": {"required_status_checks": []}}]
        result["rules"].extend(matches)
    params = matches[0]["parameters"]
    checks = params["required_status_checks"]
    gate = [c for c in checks if c["context"] == "Stack CI gate"]
    if gate and (len(gate) != 1 or gate[0].get("integration_id") != actions_app_id):
        raise ValueError("Existing gate has a different or ambiguous producer")
    if not gate:
        checks.append({"context": "Stack CI gate", "integration_id": actions_app_id})
    params["strict_required_status_checks_policy"] = True
    return result


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--input", required=True)
    parser.add_argument("--actions-app-id", type=int, required=True)
    args = parser.parse_args()
    print(json.dumps(proposal(json.loads(Path(args.input).read_text()), args.actions_app_id), indent=2))
