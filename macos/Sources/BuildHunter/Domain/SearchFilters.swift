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

enum SearchFilterCatalog {
    static func load() -> [SearchFilterDescriptor] {
        guard let pointer = bh_search_filter_catalog_json() else { return [] }
        let json = String(cString: pointer)
        guard let data = json.data(using: .utf8),
              let descriptors = try? JSONDecoder().decode([SearchFilterDescriptor].self, from: data) else {
            return []
        }
        return descriptors
    }
}

@MainActor
@Observable
final class SearchFilterSettings {
    static let shared = SearchFilterSettings(userDefaults: defaultUserDefaults)

    private static var defaultUserDefaults: UserDefaults {
#if DEBUG
        if let suite = ProcessInfo.processInfo.environment["BUILDHUNTER_SETTINGS_SUITE"],
           let defaults = UserDefaults(suiteName: suite) {
            return defaults
        }
#endif
        return .standard
    }
    static let persistenceKey = "searchFilterExcludedIDs"

    private(set) var excludedFilterIDs: Set<String>

    @ObservationIgnored private let defaults: UserDefaults
    @ObservationIgnored private let catalog: [SearchFilterDescriptor]

    init(
        catalog: [SearchFilterDescriptor] = SearchFilterCatalog.load(),
        userDefaults: UserDefaults = .standard
    ) {
        self.catalog = catalog
        defaults = userDefaults
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
