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

    @Test("Bytecode file classification never touches the file system")
    func bytecodeClassificationIsPureStringWork() {
        // URL(fileURLWithPath:) stats a relative path to infer a directory hint, about
        // 17 µs per name; a scan policy callback must stay pure string work.
        let candidates = (0..<20_000).map { facts("module\($0).cpython-312.pyc", directory: false) }

        let start = ProcessInfo.processInfo.systemUptime
        let classified = candidates.filter { policy.decide($0) != nil }.count
        let elapsed = ProcessInfo.processInfo.systemUptime - start

        #expect(classified == candidates.count)
        #expect(elapsed < 0.15)
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

    @Test("Git is skipped during discovery but included in accepted artifact measurements")
    func exclusionsAndNestedRoots() {
        #expect(policy.decide(facts(".git")) == nil)
        #expect(policy.decide(facts(".build", symbolicLink: true)) == nil)
        #expect(!IsScannableArtifactCandidate().isSatisfiedBy(facts(".git")))
        #expect(IsScannableArtifactCandidate().isSatisfiedBy(facts(".git", hasArtifactAncestor: true)))
        #expect(!IsOuterArtifactRoot().isSatisfiedBy(ArtifactRootContext(hasArtifactAncestor: true)))
        #expect(IsOuterArtifactRoot().isSatisfiedBy(ArtifactRootContext(hasArtifactAncestor: false)))
    }

    private func facts(
        _ name: String,
        directory: Bool = true,
        symbolicLink: Bool = false,
        hasArtifactAncestor: Bool = false,
        own: Set<String> = [],
        parent: Set<String> = [],
    ) -> ArtifactPolicyContext {
        ArtifactPolicyContext(nodeName: name, isDirectory: directory, isSymbolicLink: symbolicLink,
                              hasArtifactAncestor: hasArtifactAncestor,
                              ownMarkerFiles: own, parentMarkerFiles: parent)
    }
}
