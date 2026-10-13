"""Draft plugin wire-shape tests only; no evaluator or filesystem probes.

Run: python -m pip install -r scripts/plugin-contract-requirements.txt
     python scripts/test_plugin_contract.py
Semantic references, budgets, matching and JCS are pending runtime-pilot tests.
"""
import copy
import json
from pathlib import Path
import unittest

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

ROOT = Path(__file__).resolve().parents[1]
CONTRACT = ROOT / "docs/contracts/plugin-v1"
BASE = "https://buildhunter.invalid/contracts/"


def load(name):
    return json.loads((CONTRACT / name).read_text(encoding="utf-8"))


PROGRAM = load("rifle-program.schema.json")
PLUGIN = load("plugin.schema.json")
# Fixed in-memory resources: tests never fetch schemas over the network.
REGISTRY = Registry().with_resources([
    (BASE + "rifle-program.schema.json", Resource.from_contents(PROGRAM)),
    (BASE + "plugin.schema.json", Resource.from_contents(PLUGIN)),
])
VALIDATOR = Draft202012Validator(
    {"$ref": BASE + "plugin.schema.json"}, registry=REGISTRY
)


class PluginWireShapeTests(unittest.TestCase):
    def setUp(self):
        self.plugin = load("examples/rust.json")

    def reject(self, value):
        self.assertTrue(list(VALIDATOR.iter_errors(value)))

    def test_schemas_are_well_formed(self):
        Draft202012Validator.check_schema(PROGRAM)
        Draft202012Validator.check_schema(PLUGIN)

    def test_complete_examples(self):
        for path in sorted((CONTRACT / "examples").glob("*.json")):
            with self.subTest(package=path.name):
                VALIDATOR.validate(json.loads(path.read_text(encoding="utf-8")))

    def test_boolean_composition_and_suffix(self):
        self.plugin["program"]["rules"][0]["when"] = {"any": [
            {"not": {"fact": "project", "op": "eq", "value": False}},
            {"fact": "name", "op": "suffix", "value": ".build"},
            {"fact": "kind", "op": "in", "values": ["directory", "regular_file"]},
        ]}
        VALIDATOR.validate(self.plugin)

    def test_unknown_formats_are_rejected(self):
        for field in ("plugin", "program"):
            candidate = copy.deepcopy(self.plugin)
            node = candidate if field == "plugin" else candidate["program"]
            node["format"] = "future/2"
            with self.subTest(field=field):
                self.reject(candidate)

    def test_unknown_fields_in_each_layer_are_rejected(self):
        for path in ((), ("program",), ("program", "rules", 0),
                     ("program", "rules", 0, "when"), ("targets", 0),
                     ("bindings", "project")):
            candidate = copy.deepcopy(self.plugin)
            node = candidate
            for key in path:
                node = node[key]
            node["execute"] = "arbitrary command"
            with self.subTest(path=path):
                self.reject(candidate)

    def test_marker_must_be_single_safe_component(self):
        for name in (".", "..", "../Cargo.toml", "/tmp/file", "dir/file",
                     "dir\\file", "*", "", "Cargo.toml\n"):
            candidate = copy.deepcopy(self.plugin)
            candidate["bindings"]["project"]["name"] = name
            with self.subTest(name=name):
                self.reject(candidate)

    def test_empty_or_ambiguous_expressions_are_rejected(self):
        for condition in ({"all": []}, {"any": []}, {},
                          {"all": [{"fact": "project", "op": "eq", "value": True}],
                           "not": {"fact": "project", "op": "eq", "value": False}},
                          {"fact": "name", "op": "regex", "value": ".*"},
                          {"fact": "name", "op": "suffix", "value": True},
                          {"fact": "name", "op": "suffix", "value": ""},
                          {"fact": "name", "op": "in", "values": []}):
            candidate = copy.deepcopy(self.plugin)
            candidate["program"]["rules"][0]["when"] = condition
            with self.subTest(condition=condition):
                self.reject(candidate)

    def test_literal_is_not_an_executable_object_or_number(self):
        for value in (1, None, {"shell": "pwd"}, [True]):
            candidate = copy.deepcopy(self.plugin)
            candidate["program"]["rules"][0]["when"] = {
                "fact": "project", "op": "eq", "value": value}
            with self.subTest(value=value):
                self.reject(candidate)

    def test_local_array_limits(self):
        self.plugin["program"]["rules"] *= 257
        self.reject(self.plugin)

    def test_target_rule_references_are_nonempty_and_unique(self):
        for references in ([], ["output", "output"]):
            candidate = copy.deepcopy(self.plugin)
            candidate["targets"][0]["ruleIds"] = references
            with self.subTest(references=references):
                self.reject(candidate)

    def test_ids_and_versions(self):
        for field, value in (("id", "org.Example"), ("id", "../rust"),
                             ("id", "org.rust\n"), ("version", "01.0.0"),
                             ("version", "1.0"), ("version", "1.0.0\n")):
            candidate = copy.deepcopy(self.plugin)
            candidate[field] = value
            with self.subTest(field=field, value=value):
                self.reject(candidate)


if __name__ == "__main__":
    unittest.main()
