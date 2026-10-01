import Foundation
import SpecificationCore

struct ClassifyArtifactRoot: DecisionSpec {
    typealias Context = ArtifactPolicyContext
    typealias Result = ArtifactClassification

    private let decision: FirstMatchSpec<Context, Result>

    init() {
        let swiftBuild = PredicateSpec<Context>(description: "artifact.swift.build") {
            $0.isDirectory && $0.nodeName == ".build"
        }
        let rustBuild = PredicateSpec<Context>(description: "artifact.rust.target") {
            $0.isDirectory && $0.nodeName == "target" && $0.parentMarkerFiles.contains("Cargo.toml")
        }
        let pythonEnvironment = PredicateSpec<Context>(description: "artifact.python.environment") {
            $0.isDirectory && $0.ownMarkerFiles.contains("pyvenv.cfg")
        }
        let toxEnvironment = PredicateSpec<Context>(description: "artifact.python.test-environment") {
            $0.isDirectory && [".tox", ".nox"].contains($0.nodeName)
        }
        let pythonBytecode = PredicateSpec<Context>(description: "artifact.python.bytecode-root") {
            $0.isDirectory && $0.nodeName == "__pycache__"
        }
        let pythonMetadata = PredicateSpec<Context>(description: "artifact.python.package-metadata") {
            $0.isDirectory && $0.nodeName.hasSuffix(".egg-info")
        }
        let pythonBuild = PredicateSpec<Context>(description: "artifact.python.build-output") {
            $0.isDirectory && ["build", "dist"].contains($0.nodeName)
                && !$0.parentMarkerFiles.isDisjoint(with: ["pyproject.toml", "setup.py", "setup.cfg"])
        }
        let pythonCache = PredicateSpec<Context>(description: "artifact.python.tool-cache") {
            $0.isDirectory && [".pytest_cache", ".mypy_cache", ".ruff_cache", ".pytype"].contains($0.nodeName)
        }
        let pythonBytecodeFile = PredicateSpec<Context>(description: "artifact.python.bytecode-file") {
            !$0.isDirectory && ["pyc", "pyo"].contains(URL(fileURLWithPath: $0.nodeName).pathExtension)
        }

        decision = FirstMatchSpec([
            (pythonEnvironment, ArtifactClassification(kind: .environment, language: "Python")),
            (toxEnvironment, ArtifactClassification(kind: .testEnvironment, language: "Python")),
            (swiftBuild, ArtifactClassification(kind: .buildOutput, language: "Swift")),
            (rustBuild, ArtifactClassification(kind: .buildOutput, language: "Rust")),
            (pythonBuild, ArtifactClassification(kind: .buildOutput, language: "Python")),
            (pythonBytecode, ArtifactClassification(kind: .cache, language: "Python")),
            (pythonMetadata, ArtifactClassification(kind: .cache, language: "Python")),
            (pythonCache, ArtifactClassification(kind: .cache, language: "Python")),
            (pythonBytecodeFile, ArtifactClassification(kind: .cache, language: "Python"))
        ])
    }

    func decide(_ context: Context) -> Result? {
        guard IsScannableArtifactCandidate().isSatisfiedBy(context) else { return nil }
        return decision.decide(context)
    }
}
