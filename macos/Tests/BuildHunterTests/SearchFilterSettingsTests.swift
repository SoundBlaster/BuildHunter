import Foundation
import Testing
@testable import BuildHunter

@Suite("Search filter settings")
@MainActor
struct SearchFilterSettingsTests {
    @Test("Every catalog entry starts enabled")
    func defaultsEnabled() {
        let (settings, _) = makeSettings()

        #expect(catalog.allSatisfy { settings.isEnabled(for: $0.id) })
        #expect(settings.snapshot.excludedFilterIDs.isEmpty)
    }

    @Test("Selections persist and reload")
    func persistsSelection() {
        let (settings, defaults) = makeSettings()
        settings.setEnabled(false, for: "swift.build")
        let reloaded = SearchFilterSettings(catalog: catalog, userDefaults: defaults)

        #expect(!reloaded.isEnabled(for: "swift.build"))
        #expect(reloaded.isEnabled(for: "rust.target"))
    }

    @Test("Group selection updates only its catalog entries")
    func groupSelection() {
        let (settings, _) = makeSettings()
        settings.setEnabled(false, group: "Python")

        #expect(settings.filters(in: "Python").allSatisfy { !settings.isEnabled(for: $0.id) })
        #expect(settings.isEnabled(for: "swift.build"))
    }

    @Test("Unknown saved IDs survive catalog updates and new IDs default on")
    func preservesUnknownIDs() {
        let (settings, defaults) = makeSettings()
        settings.setEnabled(false, for: "future.filter")
        let expanded = SearchFilterSettings(catalog: catalog + [Self.descriptor("new.filter", group: "New")],
                                            userDefaults: defaults)

        #expect(expanded.excludedFilterIDs.contains("future.filter"))
        #expect(expanded.isEnabled(for: "new.filter"))
    }

    @Test("Reset enables filters and clears unknown saved IDs")
    func resetSelection() {
        let (settings, _) = makeSettings()
        settings.setEnabled(false, for: "swift.build")
        settings.setEnabled(false, for: "future.filter")
        settings.reset()

        #expect(settings.excludedFilterIDs.isEmpty)
        #expect(settings.isEnabled(for: "swift.build"))
    }

    @Test("Snapshots keep the exclusions captured when created")
    func snapshotIsImmutable() {
        let (settings, _) = makeSettings()
        let before = settings.snapshot
        settings.setEnabled(false, for: "swift.build")

        #expect(before.excludedFilterIDs.isEmpty)
        #expect(before.includes(candidate: facts(".build"), classification: build("Swift")))
        #expect(!settings.snapshot.includes(candidate: facts(".build"), classification: build("Swift")))
    }

    @Test("Filter matching uses the classified language and candidate facts")
    func matchesCandidate() {
        let snapshot = makeSettings().0.snapshot

        #expect(snapshot.descriptor(matching: facts("target", parent: ["Cargo.toml"]),
                                    classification: build("Rust"))?.id == "rust.target")
        #expect(snapshot.descriptor(matching: facts("output"), classification: build("Rust")) == nil)
        #expect(snapshot.includes(candidate: facts("worker.pyc", directory: false),
                                  classification: ArtifactClassification(kind: .cache, language: "Python")))
        #expect(snapshot.descriptor(matching: facts("target"), classification: build("Swift")) == nil)
    }

    @Test("The Rust catalog decodes and environment classification takes precedence")
    func canonicalCatalogAndEnvironmentPrecedence() {
        let descriptors = SearchFilterCatalog.load()
        #expect(descriptors.count == 13)
        #expect(Set(descriptors.map(\.id)).count == descriptors.count)
        let snapshot = SearchFilterSnapshot(excludedFilterIDs: ["swift.build"], catalog: descriptors)
        let candidate = facts(".build", own: ["pyvenv.cfg"])
        let classification = ArtifactClassification(kind: .environment, language: "Python")
        #expect(snapshot.descriptor(matching: candidate, classification: classification)?.id == "python.environment")
        #expect(snapshot.includes(candidate: candidate, classification: classification))
    }

    @Test("Each canonical filter excludes its matching candidate")
    func canonicalFiltersExcludeMatchingCandidates() {
        let descriptors = SearchFilterCatalog.load()
        let candidates: [String: (ArtifactPolicyContext, ArtifactClassification)] = [
            "swift.build": (facts(".build"), build("Swift")),
            "rust.target": (facts("target", parent: ["Cargo.toml"]), build("Rust")),
            "python.bytecode": (facts("module.pyc", directory: false),
                                ArtifactClassification(kind: .cache, language: "Python")),
            "python.pytest-cache": (facts(".pytest_cache"), cache("Python")),
            "python.mypy-cache": (facts(".mypy_cache"), cache("Python")),
            "python.ruff-cache": (facts(".ruff_cache"), cache("Python")),
            "python.pytype-cache": (facts(".pytype"), cache("Python")),
            "python.build": (facts("build", parent: ["pyproject.toml"]), build("Python")),
            "python.dist": (facts("dist", parent: ["setup.py"]), build("Python")),
            "python.egg-info": (facts("module.egg-info"), cache("Python")),
            "python.environment": (facts("venv", own: ["pyvenv.cfg"]),
                                   ArtifactClassification(kind: .environment, language: "Python")),
            "python.tox": (facts(".tox"), ArtifactClassification(kind: .testEnvironment, language: "Python")),
            "python.nox": (facts(".nox"), ArtifactClassification(kind: .testEnvironment, language: "Python"))
        ]

        // One disabled ID is present at a time, so this exercises all 13 canonical exclusions.
        for descriptor in descriptors {
            guard let (candidate, classification) = candidates[descriptor.id] else {
                Issue.record("Missing fixture for canonical filter \(descriptor.id)")
                continue
            }
            let snapshot = SearchFilterSnapshot(excludedFilterIDs: [descriptor.id], catalog: descriptors)
            #expect(snapshot.descriptor(matching: candidate, classification: classification)?.id == descriptor.id)
            #expect(!snapshot.includes(candidate: candidate, classification: classification))
        }
    }

    private let catalog = [
        Self.descriptor("swift.build", group: "Swift", nodes: [".build"]),
        Self.descriptor("rust.target", group: "Rust", nodes: ["target"]),
        Self.descriptor("python.bytecode", group: "Python", suffixes: [".pyc"]),
        Self.descriptor("python.environment", group: "Python", venv: true)
    ]

    private func makeSettings() -> (SearchFilterSettings, UserDefaults) {
        let suite = "BuildHunter.SearchFilterSettingsTests.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        return (SearchFilterSettings(catalog: catalog, userDefaults: defaults), defaults)
    }

    private static func descriptor(_ id: String, group: String,
                                   nodes: [String] = [], suffixes: [String] = [],
                                   venv: Bool = false) -> SearchFilterDescriptor {
        SearchFilterDescriptor(id: id, title: id, group: group, nodeNames: nodes,
                               suffixes: suffixes, requiresVenvMarker: venv)
    }

    private func build(_ language: String) -> ArtifactClassification {
        ArtifactClassification(kind: .buildOutput, language: language)
    }

    private func cache(_ language: String) -> ArtifactClassification {
        ArtifactClassification(kind: .cache, language: language)
    }

    private func facts(_ name: String, directory: Bool = true,
                       own: Set<String> = [], parent: Set<String> = []) -> ArtifactPolicyContext {
        ArtifactPolicyContext(nodeName: name, isDirectory: directory, isSymbolicLink: false,
                              ownMarkerFiles: own, parentMarkerFiles: parent)
    }
}
