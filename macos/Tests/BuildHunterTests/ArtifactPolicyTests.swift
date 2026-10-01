import Foundation
import SpecificationCore
import Testing
@testable import BuildHunter

@Suite("Artifact policy")
struct ArtifactPolicyTests {
    private let policy = ClassifyArtifactRoot()

    @Test("Classifies the supported language and cache roots")
    func supportedArtifactRoots() {
        let cases: [(ArtifactPolicyContext, ArtifactClassification)] = [
            (facts(".build"), ArtifactClassification(kind: .buildOutput, language: "Swift")),
            (facts("target", parent: ["Cargo.toml"]), ArtifactClassification(kind: .buildOutput, language: "Rust")),
            (facts("build", parent: ["pyproject.toml"]), ArtifactClassification(kind: .buildOutput, language: "Python")),
            (facts("dist", parent: ["setup.cfg"]), ArtifactClassification(kind: .buildOutput, language: "Python")),
            (facts("__pycache__"), ArtifactClassification(kind: .cache, language: "Python")),
            (facts("pkg.egg-info"), ArtifactClassification(kind: .cache, language: "Python")),
            (facts(".ruff_cache"), ArtifactClassification(kind: .cache, language: "Python")),
            (facts(".tox"), ArtifactClassification(kind: .testEnvironment, language: "Python"))
        ]

        for (input, expected) in cases {
            #expect(policy.decide(input) == expected)
        }
    }

    @Test("Environment marker belongs to the candidate directory")
    func environmentMarkerIsLocal() {
        #expect(policy.decide(facts("custom-env", own: ["pyvenv.cfg"])) ==
                ArtifactClassification(kind: .environment, language: "Python"))
        #expect(policy.decide(facts("custom-env", parent: ["pyvenv.cfg"])) == nil)
    }

    @Test("Python bytecode files match independently of filename")
    func standaloneBytecodeFiles() {
        #expect(policy.decide(facts("worker.pyc", directory: false)) ==
                ArtifactClassification(kind: .cache, language: "Python"))
        #expect(policy.decide(facts("worker.pyo", directory: false)) ==
                ArtifactClassification(kind: .cache, language: "Python"))
    }

    @Test("Parent markers are required for Rust and Python build directories")
    func parentMarkersAreRequired() {
        #expect(policy.decide(facts("target")) == nil)
        #expect(policy.decide(facts("build")) == nil)
    }

    @Test("No-match cases include arbitrary directories and unsupported files")
    func noMatchIsExplicit() {
        #expect(policy.decide(facts("output")) == nil)
        #expect(policy.decide(facts("source.py", directory: false)) == nil)
    }

    @Test("Git metadata, symlinks, and candidates nested under an artifact are excluded")
    func exclusionsAndNestedRoots() {
        #expect(policy.decide(facts(".git")) == nil)
        #expect(policy.decide(facts(".build", symbolicLink: true)) == nil)
        #expect(policy.decide(facts(".git")) == nil)
        #expect(policy.decide(facts(".build", symbolicLink: true)) == nil)
        #expect(!IsOuterArtifactRoot().isSatisfiedBy(ArtifactRootContext(hasArtifactAncestor: true)))
        #expect(IsOuterArtifactRoot().isSatisfiedBy(ArtifactRootContext(hasArtifactAncestor: false)))
    }

    private func facts(
        _ name: String,
        directory: Bool = true,
        symbolicLink: Bool = false,
        own: Set<String> = [],
        parent: Set<String> = [],
    ) -> ArtifactPolicyContext {
        ArtifactPolicyContext(nodeName: name, isDirectory: directory, isSymbolicLink: symbolicLink,
                              ownMarkerFiles: own, parentMarkerFiles: parent)
    }
}
