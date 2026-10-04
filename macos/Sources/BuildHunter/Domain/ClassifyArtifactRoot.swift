import Foundation
import SpecificationCore

struct ClassifyArtifactRoot: DecisionSpec {
    typealias Context = ArtifactPolicyContext
    typealias Result = ArtifactClassification

    private let decision: FirstMatchSpec<Context, Result>
    private let scannableCandidate: IsScannableArtifactCandidate

    init() {
        scannableCandidate = IsScannableArtifactCandidate()
        let swiftBuild = PredicateSpec<Context>(description: "artifact.swift.build") {
            $0.isDirectory && $0.nodeName == ".build"
        }
        let rustBuild = PredicateSpec<Context>(description: "artifact.rust.target") {
            $0.isDirectory && $0.nodeName == "target" && $0.parentMarkerFacts.contains(.cargoManifest)
        }
        let pythonEnvironment = PredicateSpec<Context>(description: "artifact.python.environment") {
            $0.isDirectory && $0.ownMarkerFacts.contains(.pythonVirtualEnvironment)
        }
        let toxEnvironment = PredicateSpec<Context>(description: "artifact.python.test-environment") {
            $0.isDirectory && ($0.nodeName == ".tox" || $0.nodeName == ".nox")
        }
        let pythonBytecode = PredicateSpec<Context>(description: "artifact.python.bytecode-root") {
            $0.isDirectory && $0.nodeName == "__pycache__"
        }
        let pythonMetadata = PredicateSpec<Context>(description: "artifact.python.package-metadata") {
            $0.isDirectory && $0.nodeName.hasSuffix(".egg-info")
        }
        let pythonBuild = PredicateSpec<Context>(description: "artifact.python.build-output") {
            $0.isDirectory && ($0.nodeName == "build" || $0.nodeName == "dist")
                && !$0.parentMarkerFacts.intersection(.pythonBuildProject).isEmpty
        }
        let pythonCache = PredicateSpec<Context>(description: "artifact.python.tool-cache") {
            $0.isDirectory && ($0.nodeName == ".pytest_cache" || $0.nodeName == ".mypy_cache"
                || $0.nodeName == ".ruff_cache" || $0.nodeName == ".pytype")
        }
        let pythonBytecodeFile = PredicateSpec<Context>(description: "artifact.python.bytecode-file") {
            // Pure string work: URL(fileURLWithPath:) would stat a relative path per call.
            !$0.isDirectory && ($0.nodeName.hasSuffix(".pyc") || $0.nodeName.hasSuffix(".pyo"))
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
        guard scannableCandidate.isSatisfiedBy(context) else { return nil }
        return decision.decide(context)
    }
}
