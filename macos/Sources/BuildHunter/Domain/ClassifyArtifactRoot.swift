import Foundation
import SpecificationCore

struct ClassifyArtifactRoot: DecisionSpec {
    typealias Context = ArtifactPolicyContext
    typealias Result = ArtifactClassification

    // The nine fixed rules retain their concrete types through the library's
    // balanced builder. Runtime catalogs can continue using FirstMatchSpec.
    private typealias Rule = BooleanDecisionAdapter<PredicateSpec<Context>, Result>
    private typealias Pair = BinaryFirstMatch<Rule, Rule>
    private typealias Triple = BinaryFirstMatch<Rule, Pair>
    private typealias Four = BinaryFirstMatch<Pair, Pair>
    private typealias Five = BinaryFirstMatch<Pair, Triple>
    private let decision: StaticFirstMatch<BinaryFirstMatch<Four, Five>>
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

        decision = StaticFirstMatch {
            pythonEnvironment.returning(ArtifactClassification(kind: .environment, language: "Python"))
            toxEnvironment.returning(ArtifactClassification(kind: .testEnvironment, language: "Python"))
            swiftBuild.returning(ArtifactClassification(kind: .buildOutput, language: "Swift"))
            rustBuild.returning(ArtifactClassification(kind: .buildOutput, language: "Rust"))
            pythonBuild.returning(ArtifactClassification(kind: .buildOutput, language: "Python"))
            pythonBytecode.returning(ArtifactClassification(kind: .cache, language: "Python"))
            pythonMetadata.returning(ArtifactClassification(kind: .cache, language: "Python"))
            pythonCache.returning(ArtifactClassification(kind: .cache, language: "Python"))
            pythonBytecodeFile.returning(ArtifactClassification(kind: .cache, language: "Python"))
        }
    }

    func decide(_ context: Context) -> Result? {
        guard scannableCandidate.isSatisfiedBy(context) else { return nil }
        return decision.decide(context)
    }
}
