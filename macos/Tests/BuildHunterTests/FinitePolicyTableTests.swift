import Foundation
import Testing
@testable import BuildHunter

@Suite("Finite scan policy table")
struct FinitePolicyTableTests {
    @Test("Every Rust vocabulary cell matches the shared Swift policy decision")
    func exhaustiveCellParity() throws {
        let catalog = SearchFilterCatalog.load()
        #expect(!catalog.isEmpty, "Parity needs the canonical Rust filter catalog.")
        let exclusionSets = [Set<String>()] + catalog.map { Set([$0.id]) } + [Set(catalog.map(\.id))]
        for excludedIDs in exclusionSets {
            let filters = SearchFilterSnapshot(excludedFilterIDs: excludedIDs, catalog: catalog)
            let table = try #require(RustCompiledPolicyTable.compile(filters: filters))
            #expect(table.entries.count == Int(BH_POLICY_TABLE_CELL_COUNT))
            #expect(table.version == UInt32(BH_POLICY_TABLE_VERSION))

            let policy = BuildHunterScanPolicy(filters: filters)
            for index in table.entries.indices {
                var facts = emptyFacts()
                #expect(bh_policy_table_cell_facts(index, &facts) != 0)
                expectEqual(table.entries[index], policy.decision(for: facts), at: index)
            }
            #expect(table.entries.last?.action == 0,
                    "The invalid UTF-8 cell must traverse with exclusions \(excludedIDs).")
        }
    }

    @Test("The shared helper covers representative and nonrepresentative matching details")
    func directPolicyEdges() {
        let policy = BuildHunterScanPolicy(filters: SearchFilterSnapshot(
            excludedFilterIDs: [], catalog: SearchFilterCatalog.load()))

        #expect(decision(policy, name: "custom.egg-info", directory: true).action == 2)
        #expect(decision(policy, name: "target.egg-info", directory: true).kind == 2)
        #expect(decision(policy, name: "custom.pyc", directory: false).kind == 2)
        #expect(decision(policy, name: "custom.pyo", directory: false).kind == 2)
        #expect(decision(policy, name: "prefix.target", directory: true,
                         parent: ArtifactMarkerFacts.cargoManifest).action == 0)
        #expect(decision(policy, name: ".git", directory: true).action == 1)
        #expect(decision(policy, name: ".git", directory: true, ancestor: true).action == 0)

        // Environment classification precedes every other matching rule, even when the
        // directory name resembles another artifact and only unrelated markers are set.
        let venvPriority = decision(policy, name: ".build", directory: true,
                                    own: [.pythonVirtualEnvironment],
                                    parent: [.cargoManifest, .pythonProject, .pythonSetupScript,
                                             .pythonSetupConfig, ArtifactMarkerFacts(rawValue: 1 << 20)])
        #expect(venvPriority.action == 2)
        #expect(venvPriority.language == 3)

        let unknownMarkers = decision(policy, name: "target", directory: true,
                                      parent: ArtifactMarkerFacts(rawValue: (1 << 0) | (1 << 20)))
        #expect(unknownMarkers.action == 2)
        #expect(unknownMarkers.language == 2)
    }

    @Test("Invalid UTF-8 traverses before candidate pruning, including symlinks")
    func invalidUTF8Traverses() {
        let policy = BuildHunterScanPolicy(filters: SearchFilterSnapshot(
            excludedFilterIDs: [], catalog: SearchFilterCatalog.load()))
        let invalidName: [UInt8] = [0xFF]
        let result = invalidName.withUnsafeBufferPointer { buffer in
            let facts = BHCandidateFacts(node_name: buffer.baseAddress, node_name_len: buffer.count,
                                        is_directory: 1, is_symbolic_link: 1,
                                        has_artifact_ancestor: 0, own_marker_files: 0,
                                        parent_marker_files: 0)
            return policy.decision(for: facts)
        }
        #expect(result.action == 0)
    }

    @Test("Outer-root and filter exclusions retain traversal semantics")
    func outerAndExcludedTraversal() {
        let filters = SearchFilterSnapshot(excludedFilterIDs: ["swift.build"],
                                           catalog: SearchFilterCatalog.load())
        let policy = BuildHunterScanPolicy(filters: filters)
        #expect(decision(policy, name: ".build", directory: true).action == 0)
        #expect(decision(policy, name: ".build", directory: true, ancestor: true).action == 0)
        #expect(decision(policy, name: "target", directory: true,
                         ancestor: true, parent: [.cargoManifest]).action == 0)

        let selected = BuildHunterScanPolicy(filters: SearchFilterSnapshot(
            excludedFilterIDs: [], catalog: SearchFilterCatalog.load()))
        #expect(decision(selected, name: "target", directory: true,
                         ancestor: true, parent: [.cargoManifest]).action == 0)
    }

    @Test("Custom exact, suffix, and Unicode descriptors use the callback fallback")
    func unsupportedDescriptorsFallBack() {
        for descriptor in [
            makeDescriptor(nodeNames: ["future-cache"]),
            makeDescriptor(suffixes: [".custom"]),
            makeDescriptor(nodeNames: ["缓存"])
        ] {
            let filters = SearchFilterSnapshot(excludedFilterIDs: [], catalog: [descriptor])
            #expect(RustCompiledPolicyTable.compile(filters: filters) == nil)
        }

        let unicode = makeDescriptor(nodeNames: ["缓存"])
        let excludedPolicy = BuildHunterScanPolicy(filters: SearchFilterSnapshot(
            excludedFilterIDs: [unicode.id], catalog: [unicode]))
        let includedPolicy = BuildHunterScanPolicy(filters: SearchFilterSnapshot(
            excludedFilterIDs: [], catalog: [unicode]))
        let facts = ArtifactPolicyContext(nodeName: "缓存", isDirectory: true, isSymbolicLink: false,
                                         ownMarkerFacts: [.pythonVirtualEnvironment], parentMarkerFacts: [])
        #expect(excludedPolicy.decision(for: facts).action == 0,
                "The legacy decision must apply custom Unicode matches to a classified candidate.")
        #expect(includedPolicy.decision(for: facts).action == 2)
    }

    @Test("A venv-marker descriptor ignores its otherwise custom match lists")
    func venvDescriptorListsAreIgnored() throws {
        let descriptor = SearchFilterDescriptor(id: "python.environment", title: "Environment",
                                               group: "Python", nodeNames: ["future-name"],
                                               suffixes: [".future"], requiresVenvMarker: true)
        let filters = SearchFilterSnapshot(excludedFilterIDs: [], catalog: [descriptor])
        let table = try #require(RustCompiledPolicyTable.compile(filters: filters))
        let policy = BuildHunterScanPolicy(filters: filters)
        for index in table.entries.indices {
            var facts = emptyFacts()
            #expect(bh_policy_table_cell_facts(index, &facts) != 0)
            expectEqual(table.entries[index], policy.decision(for: facts), at: index)
        }
    }

    @Test("A captured table is unaffected by a later snapshot")
    func capturedSnapshot() throws {
        let catalog = SearchFilterCatalog.load()
        let enabled = SearchFilterSnapshot(excludedFilterIDs: [], catalog: catalog)
        let disabled = SearchFilterSnapshot(excludedFilterIDs: ["swift.build"], catalog: catalog)
        let first = try #require(RustCompiledPolicyTable.compile(filters: enabled))
        let second = try #require(RustCompiledPolicyTable.compile(filters: disabled))
        #expect(first.entries.count == second.entries.count)
        #expect(first.entries[2 * 256 + 1].action == 2) // .build directory
        #expect(second.entries[2 * 256 + 1].action == 0)
    }

    private func decision(_ policy: BuildHunterScanPolicy, name: String, directory: Bool,
                         ancestor: Bool = false, own: ArtifactMarkerFacts = [],
                         parent: ArtifactMarkerFacts = []) -> BHCandidateDecision {
        let bytes = Array(name.utf8)
        return bytes.withUnsafeBufferPointer { buffer in
            let facts = BHCandidateFacts(node_name: buffer.baseAddress, node_name_len: buffer.count,
                                         is_directory: directory ? 1 : 0, is_symbolic_link: 0,
                                         has_artifact_ancestor: ancestor ? 1 : 0,
                                         own_marker_files: own.rawValue, parent_marker_files: parent.rawValue)
            return policy.decision(for: facts)
        }
    }

    private func emptyFacts() -> BHCandidateFacts {
        BHCandidateFacts(node_name: nil, node_name_len: 0, is_directory: 0, is_symbolic_link: 0,
                         has_artifact_ancestor: 0, own_marker_files: 0, parent_marker_files: 0)
    }

    private func makeDescriptor(nodeNames: [String] = [], suffixes: [String] = []) -> SearchFilterDescriptor {
        SearchFilterDescriptor(id: "custom", title: "Custom", group: "Python", nodeNames: nodeNames,
                               suffixes: suffixes, requiresVenvMarker: false)
    }

    private func expectEqual(_ lhs: BHCandidateDecision, _ rhs: BHCandidateDecision, at index: Int,
                             sourceLocation: SourceLocation = #_sourceLocation) {
        #expect(lhs.action == rhs.action, "Action differs at cell \(index)", sourceLocation: sourceLocation)
        #expect(lhs.language == rhs.language, "Language differs at cell \(index)", sourceLocation: sourceLocation)
        #expect(lhs.kind == rhs.kind, "Kind differs at cell \(index)", sourceLocation: sourceLocation)
    }
}
