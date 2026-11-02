import SwiftUI
import NestedA11yIDs

struct SearchSettingsWindow: View {
    @Bindable private var settings: SearchFilterSettings
    @AppStorage(ScanWarningsPresentation.storageKey, store: SearchFilterSettings.defaultUserDefaults)
    private var warningsPresentation = ScanWarningsPresentation.window

    init(settings: SearchFilterSettings = .shared) {
        self.settings = settings
    }

    @State private var query = ""
    @State private var expandedGroups: Set<String> = []
    @State private var expandedSections: Set<String> = []

    private var searching: Bool { !query.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
    private var allFilters: [SearchFilterDescriptor] { settings.groups.flatMap { settings.filters(in: $0) } }

    private func matches(_ filter: SearchFilterDescriptor) -> Bool {
        let term = query.trimmingCharacters(in: .whitespacesAndNewlines)
        return term.isEmpty || ([filter.title, filter.id, filter.group] + filter.nodeNames + filter.suffixes)
            .contains { $0.localizedCaseInsensitiveContains(term) }
    }

    private func section(_ filter: SearchFilterDescriptor) -> String {
        switch filter.id {
        case "python.bytecode", "python.pytest-cache", "python.mypy-cache", "python.ruff-cache", "python.pytype-cache": "Caches"
        case "python.build", "python.dist", "python.egg-info": "Build & packaging"
        case "python.environment", "python.tox", "python.nox": "Environments"
        default: "Other"
        }
    }

    private func expansion(_ name: String, sections: Bool = false) -> Binding<Bool> {
        Binding(get: { searching || (sections ? expandedSections : expandedGroups).contains(name) }, set: { value in
            guard !searching else { return }
            if sections {
                if value { expandedSections.insert(name) } else { expandedSections.remove(name) }
            } else {
                if value { expandedGroups.insert(name) } else { expandedGroups.remove(name) }
            }
        })
    }

    private struct FilterSelection {
        let isEnabled: Binding<Bool>
    }

    private func checkbox(_ title: String, filters: [SearchFilterDescriptor]) -> some View {
        let selections = filters.map { filter in
            FilterSelection(isEnabled: Binding(
                get: { settings.isEnabled(for: filter.id) },
                set: { settings.setEnabled($0, for: filter.id) }
            ))
        }
        return Toggle(sources: selections, isOn: \.isEnabled) {
            Text(LocalizedStringKey(title)).fontWeight(.semibold)
        }
        .toggleStyle(.checkbox)
        .nestedAccessibilityIdentifier("group.\(title)")
    }

    private func row(_ filter: SearchFilterDescriptor) -> some View {
        HStack(spacing: 10) {
            Toggle(filter.title, isOn: Binding(get: { settings.isEnabled(for: filter.id) },
                set: { settings.setEnabled($0, for: filter.id) }))
                .toggleStyle(.checkbox)
                .nestedAccessibilityIdentifier("filter.\(filter.id)")
            Spacer()
            let names = filter.nodeNames + filter.suffixes
            if !names.isEmpty {
                Text(names.joined(separator: ", ")).font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary).lineLimit(1)
            }
        }.padding(.vertical, 4)
    }

    private func groupHeader(_ title: String, filters: [SearchFilterDescriptor]) -> some View {
        HStack {
            checkbox(title, filters: filters)
            Spacer()
            Text("\(filters.filter { settings.isEnabled(for: $0.id) }.count) / \(filters.count)")
                .foregroundStyle(.secondary).monospacedDigit()
        }
    }

    var body: some View {
        Form {
            Section {
                HStack {
                    VStack(alignment: .leading) {
                        Text("Included in scans").font(.headline)
                        Text("\(allFilters.filter { settings.isEnabled(for: $0.id) }.count) of \(allFilters.count) rules enabled")
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    TextField("Filter rules", text: $query).textFieldStyle(.roundedBorder).frame(maxWidth: 220)
                        .nestedAccessibilityIdentifier("filterQuery")
                }
                ForEach(settings.groups, id: \.self) { group in
                    let filters = settings.filters(in: group)
                    let visible = filters.filter(matches)
                    if !visible.isEmpty {
                        if filters.count == 1, let filter = visible.first {
                            row(filter)
                        } else {
                            DisclosureGroup(isExpanded: expansion(group)) {
                                if group == "Python" {
                                    ForEach(["Caches", "Build & packaging", "Environments", "Other"], id: \.self) { name in
                                        let children = filters.filter { section($0) == name }
                                        let shown = children.filter(matches)
                                        if !shown.isEmpty {
                                            DisclosureGroup(isExpanded: expansion(group + "." + name, sections: true)) {
                                                ForEach(shown) { row($0) }
                                            } label: { groupHeader(name, filters: children) }
                                        }
                                    }
                                } else {
                                    ForEach(visible) { row($0) }
                                }
                            } label: { groupHeader(group, filters: filters) }
                        }
                    }
                }
            } footer: {
                Text("Changes apply to the next scan or Rescan. Open reports keep their current results.")
            }
            Section {
                Button("Enable All") { settings.enableAll() }
                    .nestedAccessibilityIdentifier("enableAll")
            }
            Section {
                Toggle("Profile individual rules", isOn: $settings.ruleProfilingEnabled)
                    .help("Adds per-rule counters and wall timings to the next scan. Instrumentation adds overhead.")
                    .nestedAccessibilityIdentifier("ruleProfiling")
                Picker("Show scan warnings in", selection: $warningsPresentation) {
                    ForEach(ScanWarningsPresentation.allCases) { Text($0.title).tag($0) }
                }.nestedAccessibilityIdentifier("warningsPresentation")
            } header: { Text("Experiments") } footer: {
                Text("Rule profiling applies to new scans and adds measurement overhead. Warning presentation applies to report windows.")
            }
        }
        .formStyle(.grouped)
        .frame(minWidth: 760, minHeight: 540)
        .a11yRoot("buildhunter.settings.search")
    }

}

#if DEBUG
#Preview("Search settings") {
    SearchSettingsWindow(settings: SearchFilterSettings(
        userDefaults: UserDefaults(suiteName: "BuildHunter.SettingsPreview")!
    ))
}
#endif
