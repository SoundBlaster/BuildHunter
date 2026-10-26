import Foundation
import Observation
import SpecificationCore

struct SearchFilterDescriptor: Codable, Hashable, Identifiable, Sendable {
    let id: String
    let title: String
    let group: String
    let nodeNames: [String]
    let suffixes: [String]
    let requiresVenvMarker: Bool

    func matches(_ candidate: ArtifactPolicyContext, classification: ArtifactClassification) -> Bool {
        guard group == classification.language else { return false }
        if requiresVenvMarker {
            return candidate.ownMarkerFacts.contains(.pythonVirtualEnvironment)
        }
        return nodeNames.contains(candidate.nodeName)
            || suffixes.contains { candidate.nodeName.hasSuffix($0) }
    }
}

struct SearchFilterDecisionContext: Sendable {
    let descriptor: SearchFilterDescriptor
    let candidate: ArtifactPolicyContext
    let classification: ArtifactClassification
    let excludedFilterIDs: Set<String>
}

/// Applies the user's filter selection to a candidate already recognized by the scanner.
/// The cached predicate captures no state; it reads only the immutable Sendable context.
/// `PredicateSpec` itself lacks `Sendable`, so this wrapper records that narrower guarantee.
struct IsIncludedSearchArtifact: Specification, @unchecked Sendable {
    private let rule = PredicateSpec<SearchFilterDecisionContext>(description: "search.filter.candidate.enabled") {
        !$0.excludedFilterIDs.contains($0.descriptor.id)
            && $0.descriptor.matches($0.candidate, classification: $0.classification)
    }

    func isSatisfiedBy(_ context: SearchFilterDecisionContext) -> Bool {
        rule.isSatisfiedBy(context)
    }
}

/// One immutable preference view is captured at the start of each scan.
struct SearchFilterSnapshot: Sendable {
    let excludedFilterIDs: Set<String>
    let catalog: [SearchFilterDescriptor]
    private let inclusionPolicy: IsIncludedSearchArtifact

    init(excludedFilterIDs: Set<String>, catalog: [SearchFilterDescriptor]) {
        self.excludedFilterIDs = excludedFilterIDs
        self.catalog = catalog
        inclusionPolicy = IsIncludedSearchArtifact()
    }

    func descriptor(
        matching candidate: ArtifactPolicyContext,
        classification: ArtifactClassification
    ) -> SearchFilterDescriptor? {
        catalog.first { $0.matches(candidate, classification: classification) }
    }

    func includes(
        _ descriptor: SearchFilterDescriptor,
        candidate: ArtifactPolicyContext,
        classification: ArtifactClassification
    ) -> Bool {
        inclusionPolicy.isSatisfiedBy(
            SearchFilterDecisionContext(
                descriptor: descriptor,
                candidate: candidate,
                classification: classification,
                excludedFilterIDs: excludedFilterIDs
            )
        )
    }

    /// Unrecognized classifications remain eligible; a future scanner rule can be added
    /// before this app version learns how to filter its descriptor.
    func includes(candidate: ArtifactPolicyContext, classification: ArtifactClassification) -> Bool {
        guard let descriptor = descriptor(matching: candidate, classification: classification) else { return true }
        return includes(descriptor, candidate: candidate, classification: classification)
    }
}

/// Versioned scanner metadata; decoded once per scan before target IDs are used.
struct ScanCatalogMetadata: Codable, Equatable, Sendable {
    struct Package: Codable, Equatable, Sendable {
        let id: String
        let version: String
    }
    struct Target: Codable, Equatable, Sendable {
        let id: String
        let title: String
        let ecosystem: String
        let kind: String
        let presentationKind: String

        var language: String { ecosystem.prefix(1).uppercased() + ecosystem.dropFirst() }
        var artifactKind: ArtifactKind? {
            switch presentationKind {
            case "build": .buildOutput
            case "cache", "bytecode", "metadata": .cache
            case "environment": .environment
            case "test-environment": .testEnvironment
            default: nil
            }
        }
    }
    let format: String
    let profile: String
    let digest: String
    let markerPolicy: String
    let excludedTargetIDs: [String]
    let targets: [Target]
    let packages: [Package]?

    var isSupported: Bool {
        format == "buildhunter.catalog/1" && profile == "buildhunter.gui/1"
            && markerPolicy == "legacy-file-markers/1"
            && digest.utf8.count == 64 && digest.utf8.allSatisfy { (48...57).contains($0) || (97...102).contains($0) }
            && targets.count <= 2_048 && Set(targets.map(\.id)).count == targets.count
            && targets.allSatisfy { !$0.id.isEmpty && $0.artifactKind != nil }
            && (packages.map { $0.count <= 64 && Set($0.map(\.id)).count == $0.count
                && $0.allSatisfy { !$0.id.isEmpty && !$0.version.isEmpty } } ?? true)
    }

    static func basic() -> Self? {
        guard bh_catalog_abi_version() == 1, let handle = bh_catalog_create(2, nil, 0) else { return nil }
        defer { bh_catalog_destroy(handle) }
        guard let pointer = bh_catalog_json(handle), let data = String(cString: pointer).data(using: .utf8),
              let metadata = try? JSONDecoder().decode(Self.self, from: data), metadata.isSupported else { return nil }
        return metadata
    }
}

struct ScanDiagnostic: Codable, Equatable, Sendable {
    struct Classification: Codable, Equatable, Sendable {
        struct Notice: Codable, Equatable, Sendable { let packageID: String; let factID: String; let reason: String }
        let kind: String
        let matchedTargetIDs: [String]
        let unknownTargetIDs: [String]
        let unavailable: [Notice]
    }
    let relativePath: String
    let classification: Classification
    let message: String
    enum CodingKeys: String, CodingKey { case relativePath = "relative_path", classification, message }
}

enum SearchFilterCatalog {
    static func load() -> [SearchFilterDescriptor] {
        guard let metadata = ScanCatalogMetadata.basic() else { return [] }
        return descriptors(for: metadata)
    }

    static func descriptors(for metadata: ScanCatalogMetadata) -> [SearchFilterDescriptor] {
        // Hints support the retained legacy policy/table APIs. Runtime membership,
        // titles and scan decisions now come from the plugin catalog.
        let legacy: [SearchFilterDescriptor]
        if let pointer = bh_search_filter_catalog_json(), let data = String(cString: pointer).data(using: .utf8) {
            legacy = (try? JSONDecoder().decode([SearchFilterDescriptor].self, from: data)) ?? []
        } else { legacy = [] }
        let descriptors = metadata.targets.map { target in
            let hint = legacy.first { $0.id == target.id }
            return SearchFilterDescriptor(id: target.id, title: target.title, group: target.language,
                nodeNames: hint?.nodeNames ?? [], suffixes: hint?.suffixes ?? [],
                requiresVenvMarker: hint?.requiresVenvMarker ?? false)
        }
        return descriptors.sorted { left, right in
            let first = legacy.firstIndex { $0.id == left.id } ?? .max
            let second = legacy.firstIndex { $0.id == right.id } ?? .max
            return first == second ? left.id < right.id : first < second
        }
    }
}

@MainActor
@Observable
final class SearchFilterSettings {
    static let shared = SearchFilterSettings(userDefaults: defaultUserDefaults)

    /// The app's preferences store; Debug UI tests isolate it with BUILDHUNTER_SETTINGS_SUITE.
    nonisolated static var defaultUserDefaults: UserDefaults {
#if DEBUG
        if let suite = ProcessInfo.processInfo.environment["BUILDHUNTER_SETTINGS_SUITE"],
           let defaults = UserDefaults(suiteName: suite) {
            return defaults
        }
#endif
        return .standard
    }
    static let persistenceKey = "searchFilterExcludedIDs"
    static let ruleProfilingKey = "profileIndividualRules"
    var ruleProfilingEnabled: Bool {
        didSet { defaults.set(ruleProfilingEnabled, forKey: Self.ruleProfilingKey) }
    }

    private(set) var excludedFilterIDs: Set<String>

    @ObservationIgnored private let defaults: UserDefaults
    private var catalog: [SearchFilterDescriptor]

    init(
        catalog: [SearchFilterDescriptor] = SearchFilterCatalog.load(),
        userDefaults: UserDefaults = .standard
    ) {
        self.catalog = catalog
        defaults = userDefaults
        ruleProfilingEnabled = userDefaults.bool(forKey: Self.ruleProfilingKey)
        excludedFilterIDs = Set(userDefaults.stringArray(forKey: Self.persistenceKey) ?? [])
    }

    var groups: [String] {
        var seen = Set<String>()
        return catalog.map(\.group).filter { seen.insert($0).inserted }
    }

    func filters(in group: String) -> [SearchFilterDescriptor] {
        catalog.filter { $0.group == group }
    }

    var snapshot: SearchFilterSnapshot {
        SearchFilterSnapshot(excludedFilterIDs: excludedFilterIDs, catalog: catalog)
    }

    /// Metadata describes the catalog actually admitted for this scan. Unknown
    /// exclusions remain persisted when packages are disconnected and reconnected.
    func useCatalog(_ metadata: ScanCatalogMetadata) {
        guard metadata.isSupported else { return }
        catalog = SearchFilterCatalog.descriptors(for: metadata)
    }

    func isEnabled(for filterID: String) -> Bool {
        !excludedFilterIDs.contains(filterID)
    }

    func setEnabled(_ isEnabled: Bool, for filterID: String) {
        if isEnabled {
            excludedFilterIDs.remove(filterID)
        } else {
            excludedFilterIDs.insert(filterID)
        }
        persist()
    }

    func setEnabled(_ isEnabled: Bool, group: String) {
        let filterIDs = Set(filters(in: group).map(\.id))
        if isEnabled {
            excludedFilterIDs.subtract(filterIDs)
        } else {
            excludedFilterIDs.formUnion(filterIDs)
        }
        persist()
    }

    func enableAll() {
        excludedFilterIDs.removeAll()
        persist()
    }

    func reset() {
        enableAll()
    }

    private func persist() {
        defaults.set(excludedFilterIDs.sorted(), forKey: Self.persistenceKey)
    }
}
