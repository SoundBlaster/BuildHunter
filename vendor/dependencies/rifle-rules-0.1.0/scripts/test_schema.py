"""Verify copied program schema and public fixtures; not a Rust evaluator test."""
import json
from pathlib import Path
from jsonschema import Draft202012Validator

root = Path(__file__).resolve().parents[1]
schema = json.loads((root / "schemas/program.schema.json").read_text(encoding="utf-8"))
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema)
for path in sorted((root / "fixtures").glob("*-program.json")):
    validator.validate(json.loads(path.read_text(encoding="utf-8")))
    print("Schema validated:", path.name)
