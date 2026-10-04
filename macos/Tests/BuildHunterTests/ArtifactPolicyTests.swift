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

    @Test("Bytecode files are recognized by suffix, as the Rust CLI and filter catalog do")
    func bytecodeFilesMatchBySuffix() {
        let bytecode = ArtifactClassification(kind: .cache, language: "Python")
        #expect(policy.decide(facts("module.cpython-312.pyc", directory: false)) == bytecode)
        #expect(policy.decide(facts("module.opt-1.pyo", directory: false)) == bytecode)
        #expect(policy.decide(facts(".pyc", directory: false)) == bytecode)
        #expect(policy.decide(facts("module.pyc.bak", directory: false)) == nil)
        #expect(policy.decide(facts("notes.PYC", directory: false)) == nil)
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

    @Test("Typed marker masks preserve Set initializer policy for every marker combination")
    func markerMaskAndSetParity() {
        let names: [(UInt32, String)] = [
            (1, "Cargo.toml"), (2, "pyproject.toml"), (4, "setup.py"),
            (8, "setup.cfg"), (16, "pyvenv.cfg")
        ]
        for ownMask in UInt32(0)..<32 {
            for parentMask in UInt32(0)..<32 {
                let ownNames = Set(names.compactMap { ownMask & $0.0 == 0 ? nil : $0.1 })
                let parentNames = Set(names.compactMap { parentMask & $0.0 == 0 ? nil : $0.1 })
                for name in [".build", "target", "venv", ".tox", ".nox", "build", "dist",
                             "__pycache__", "package.egg-info", ".pytest_cache", "module.pyc", ".git", "other"] {
                    for directory in [false, true] {
                        for symbolicLink in [false, true] {
                            for hasAncestor in [false, true] {
                                let fromSets = ArtifactPolicyContext(
                                    nodeName: name, isDirectory: directory, isSymbolicLink: symbolicLink,
                                    hasArtifactAncestor: hasAncestor,
                                    ownMarkerFiles: ownNames, parentMarkerFiles: parentNames
                                )
                                let fromMasks = ArtifactPolicyContext(
                                    nodeName: name, isDirectory: directory, isSymbolicLink: symbolicLink,
                                    hasArtifactAncestor: hasAncestor,
                                    ownMarkerFacts: ArtifactMarkerFacts(rawValue: ownMask),
                                    parentMarkerFacts: ArtifactMarkerFacts(rawValue: parentMask)
                                )
                                #expect(fromMasks.ownMarkerFacts == fromSets.ownMarkerFacts)
                                #expect(fromMasks.parentMarkerFacts == fromSets.parentMarkerFacts)
                                #expect(policy.decide(fromMasks) == referenceClassification(
                                    fromSets, ownMarkers: ownNames, parentMarkers: parentNames
                                ))
                            }
                        }
                    }
                }
            }
        }
        // Unknown marker names and unknown ABI bits have never affected the policy.
        let unknownName = ArtifactPolicyContext(nodeName: "build", isDirectory: true, isSymbolicLink: false,
                                               ownMarkerFiles: ["unknown.marker"], parentMarkerFiles: ["unknown.marker"])
        let unknownBit = ArtifactPolicyContext(nodeName: "build", isDirectory: true, isSymbolicLink: false,
                                               ownMarkerFacts: ArtifactMarkerFacts(rawValue: 1 << 31),
                                               parentMarkerFacts: ArtifactMarkerFacts(rawValue: 1 << 31))
        #expect(policy.decide(unknownName) == policy.decide(unknownBit))
    }

    private func referenceClassification(
        _ context: ArtifactPolicyContext, ownMarkers: Set<String>, parentMarkers: Set<String>
    ) -> ArtifactClassification? {
        guard !context.isSymbolicLink,
              !(context.isDirectory && context.nodeName == ".git" && !context.hasArtifactAncestor) else { return nil }
        if context.isDirectory && ownMarkers.contains("pyvenv.cfg") {
            return ArtifactClassification(kind: .environment, language: "Python")
        }
        if context.isDirectory && (context.nodeName == ".tox" || context.nodeName == ".nox") {
            return ArtifactClassification(kind: .testEnvironment, language: "Python")
        }
        if context.isDirectory && context.nodeName == ".build" {
            return ArtifactClassification(kind: .buildOutput, language: "Swift")
        }
        if context.isDirectory && context.nodeName == "target"
            && parentMarkers.contains("Cargo.toml") {
            return ArtifactClassification(kind: .buildOutput, language: "Rust")
        }
        if context.isDirectory && (context.nodeName == "build" || context.nodeName == "dist")
            && !parentMarkers.isDisjoint(with: ["pyproject.toml", "setup.py", "setup.cfg"]) {
            return ArtifactClassification(kind: .buildOutput, language: "Python")
        }
        if context.isDirectory && context.nodeName == "__pycache__" {
            return ArtifactClassification(kind: .cache, language: "Python")
        }
        if context.isDirectory && context.nodeName.hasSuffix(".egg-info") {
            return ArtifactClassification(kind: .cache, language: "Python")
        }
        if context.isDirectory && [".pytest_cache", ".mypy_cache", ".ruff_cache", ".pytype"].contains(context.nodeName) {
            return ArtifactClassification(kind: .cache, language: "Python")
        }
        if !context.isDirectory && (context.nodeName.hasSuffix(".pyc") || context.nodeName.hasSuffix(".pyo")) {
            return ArtifactClassification(kind: .cache, language: "Python")
        }
        return nil
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
