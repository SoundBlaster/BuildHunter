#!/usr/bin/env python3
"""Run production catalog source and its six focused tests without an AppKit host.
This is auxiliary unit evidence, not application/sandbox/StoreKit acceptance.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

parser = argparse.ArgumentParser(__doc__)
parser.add_argument("--output", type=Path, required=True)
parser.add_argument("--rust-library", type=Path, required=True)
parser.add_argument("--cache", type=Path, required=True)
a = parser.parse_args()
repo = Path(__file__).resolve().parents[3]
root = a.output.resolve()
harness = root / "harness"
sources = harness / "Sources/BuildHunter"
headers = harness / "Sources/CBuildHunter"
tests = harness / "Tests/BuildHunterTests"
for folder in [sources, headers, tests]:
    folder.mkdir(parents=True, exist_ok=True)
inputs = ["macos/Sources/BuildHunter/Domain/UserTemplates.swift",
          "macos/Sources/BuildHunter/Domain/SearchFilters.swift",
          "macos/Sources/BuildHunter/Domain/ArtifactPolicyContext.swift",
          "macos/Sources/BuildHunter/Bridge/BuildHunterFFI.h",
          "macos/Tests/BuildHunterTests/UserTemplateTests.swift"]
(sources / "UserTemplates.swift").write_text("import CBuildHunter\n" + (repo / inputs[0]).read_text())
s = (repo / inputs[1]).read_text()
(sources / "Metadata.swift").write_text("import Foundation\nimport CBuildHunter\n" +
    s[s.index("struct ScanCatalogMetadata:"):s.index("\nstruct ScanDiagnostic:")])
s = (repo / inputs[2]).read_text()
(sources / "ArtifactKind.swift").write_text("import Foundation\n" +
    s[s.index("enum ArtifactKind:"):s.index("\nstruct ArtifactClassification:")])
(headers / "BuildHunterFFI.h").write_bytes((repo / inputs[3]).read_bytes())
(headers / "module.modulemap").write_text('module CBuildHunter { header "BuildHunterFFI.h" export * }\n')
s = (repo / inputs[4]).read_text()
(tests / "BundledCatalogTests.swift").write_text("import Foundation\nimport Testing\nimport CryptoKit\nimport CBuildHunter\n@testable import BuildHunter\n" +
    s[s.index('@Suite("Bundled catalog resources")'):])
# This local Rust archive was built for macOS 26. Match it in the auxiliary runner;
# do not mistake this receipt for verification on the app's oldest supported OS.
(harness / "Package.swift").write_text('// swift-tools-version: 6.0\nimport PackageDescription\n' +
    'let package = Package(name: "CatalogAdmissionHarness", platforms: [.macOS("26.0")], targets: [\n' +
    '.systemLibrary(name: "CBuildHunter"),\n' +
    '.target(name: "BuildHunter", dependencies: ["CBuildHunter"], linkerSettings: [.unsafeFlags([' +
    json.dumps(str(a.rust_library.resolve())) + ']), .linkedFramework("Security"), .linkedFramework("CoreFoundation"), .linkedFramework("IOKit")]),\n' +
    '.testTarget(name: "BuildHunterTests", dependencies: ["BuildHunter", "CBuildHunter"])])\n')
command = ["swift", "test", "--package-path", str(harness), "--scratch-path", str(root / "harness-build"),
           "--cache-path", str(a.cache.resolve())]
started = time.monotonic()
with (root / "harness.log").open("w") as log:
    result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=180)
receipt = {"command": command, "exit_code": result.returncode, "elapsed_s": round(time.monotonic() - started, 2),
           "source_sha256": {p: hashlib.sha256((repo / p).read_bytes()).hexdigest() for p in inputs},
           "generated_sha256": {str(p.relative_to(harness)): hashlib.sha256(p.read_bytes()).hexdigest()
                                for p in harness.rglob("*") if p.is_file() and ".swiftpm" not in p.parts},
           "rust_library_sha256": hashlib.sha256(a.rust_library.read_bytes()).hexdigest(),
           "log_sha256": hashlib.sha256((root / "harness.log").read_bytes()).hexdigest()}
(root / "harness-checks.json").write_text(json.dumps(receipt, indent=2) + "\n")
print((root / "harness.log").read_text()[-2500:])
raise SystemExit(result.returncode)
