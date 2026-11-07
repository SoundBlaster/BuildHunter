import AppKit
import SwiftUI
import NestedA11yIDs
#if BUILDHUNTER_COMMERCE
import BuildHunterEditionUI
#endif

/// Shared layout for scan-rule selection and the optional catalog preview.
struct ScanRuleRow<Primary: View, Secondary: View>: View {
    @ViewBuilder let primary: Primary
    @ViewBuilder let secondary: Secondary

    var body: some View {
        HStack(spacing: 10) {
            primary
            Spacer(minLength: 8)
            secondary
        }
        .padding(.vertical, 4)
        .padding(.horizontal, 12)
        .contentShape(Rectangle())
    }
}

struct ScanRuleIcon: View {
    let id: String
    let group: String
    let isUser: Bool

    private var icon: Image {
        if isUser { return Image(systemName: "person.crop.circle") }
#if BUILDHUNTER_COMMERCE
        if let name = catalogRuleIconName(for: id) { return Image(name) }
#endif
        switch group.lowercased() {
        case "swift": return Image("LanguageSwift")
        case "rust": return Image("LanguageRust")
        case "python": return Image("LanguagePython")
        default: return Image(systemName: "square.stack.3d.up")
        }
    }

    var body: some View {
        icon.renderingMode(!isUser && group.lowercased() == "rust" ? .template : .original)
            .resizable().scaledToFit().frame(width: 20, height: 20)
            .foregroundStyle(.primary).accessibilityHidden(true)
    }
}

struct SearchSettingsWindow: View {
    @Bindable private var settings: SearchFilterSettings
    @AppStorage(ScanWarningsPresentation.storageKey, store: SearchFilterSettings.defaultUserDefaults)
    private var warningsPresentation = ScanWarningsPresentation.window

    init(settings: SearchFilterSettings = .shared) {
        self.settings = settings
    }

    @State private var query = ""
    @State private var selectionChanged = false
    @State private var expandedGroups: Set<String> = []
    @State private var expandedSections: Set<String> = []
    @State private var ruleOrigins: [String: SearchRuleOrigin] = [:]

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

    @ViewBuilder private func originBadge(_ origin: SearchRuleOrigin?) -> some View {
        if let origin {
            Group {
                switch origin {
                case .catalog: Text("Catalog")
                case .user: Text("User")
                }
            }
            .font(.caption.weight(.medium))
            .foregroundStyle(.secondary)
            .padding(.horizontal, 6).padding(.vertical, 2)
            .background(.quaternary, in: RoundedRectangle(cornerRadius: 4))
        }
    }

    private func checkbox(_ title: String, filters: [SearchFilterDescriptor]) -> some View {
        let selections = filters.map { filter in
            FilterSelection(isEnabled: Binding(
                get: { settings.isEnabled(for: filter.id) },
                set: { settings.setEnabled($0, for: filter.id); selectionChanged = true }
            ))
        }
        return Toggle(sources: selections, isOn: \.isEnabled) {
            HStack(spacing: 8) {
                ScanRuleIcon(id: "", group: filters.first?.group ?? title,
                             isUser: filters.allSatisfy { ruleOrigins[$0.id] == .user })
                Text(LocalizedStringKey(title)).fontWeight(.semibold)
            }
            .padding(.leading, 6)
        }
        .toggleStyle(.checkbox)
        .nestedAccessibilityIdentifier("group.\(title)")
    }

    private func row(_ filter: SearchFilterDescriptor) -> some View {
        ScanRuleRow {
            Toggle(isOn: Binding(get: { settings.isEnabled(for: filter.id) },
                set: { settings.setEnabled($0, for: filter.id); selectionChanged = true })) {
                HStack(spacing: 8) {
                    ScanRuleIcon(id: filter.id, group: filter.group, isUser: ruleOrigins[filter.id] == .user)
                    Text(filter.title)
                }
                .padding(.leading, 6)
            }
                .toggleStyle(.checkbox)
                .nestedAccessibilityIdentifier("filter.\(filter.id)")
            originBadge(ruleOrigins[filter.id])
        } secondary: {
            let names = filter.nodeNames + filter.suffixes
            if !names.isEmpty {
                Text(names.joined(separator: ", ")).font(.system(.caption, design: .monospaced))
                    .foregroundStyle(.secondary).lineLimit(1)
            }
        }
    }

    private func groupHeader(_ title: String, filters: [SearchFilterDescriptor], expanded: Binding<Bool>) -> some View {
        ScanRuleRow {
            checkbox(title, filters: filters)
            if let first = filters.first.flatMap({ ruleOrigins[$0.id] }),
               filters.allSatisfy({ ruleOrigins[$0.id] == first }) {
                originBadge(first).padding(.leading, 8)
            }
        } secondary: {
            Button {
                if (NSApp.currentEvent?.modifierFlags ?? NSEvent.modifierFlags).contains(.option) {
                    expandedGroups = Set(settings.groups.filter { settings.filters(in: $0).count > 1 })
                    expandedSections = Set(["Caches", "Build & packaging", "Environments", "Other"].map { "Python." + $0 })
                } else {
                    expanded.wrappedValue.toggle()
                }
            } label: {
                HStack {
                    Spacer(minLength: 8)
                    Text("\(filters.filter { settings.isEnabled(for: $0.id) }.count) / \(filters.count)")
                        .foregroundStyle(.secondary).monospacedDigit()
                    Image(systemName: expanded.wrappedValue ? "chevron.down" : "chevron.right")
                        .foregroundStyle(.secondary).frame(width: 18)
                }.contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel(Text(LocalizedStringKey(title)))
            .accessibilityValue(expanded.wrappedValue ? "Expanded" : "Collapsed")
            .nestedAccessibilityIdentifier("expand.\(title)")
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
                            let groupExpansion = expansion(group)
                            groupHeader(group, filters: filters, expanded: groupExpansion)
                            if groupExpansion.wrappedValue {
                                if group == "Python" {
                                    ForEach(["Caches", "Build & packaging", "Environments", "Other"], id: \.self) { name in
                                        let children = filters.filter { section($0) == name }
                                        let shown = children.filter(matches)
                                        if !shown.isEmpty {
                                            let sectionExpansion = expansion(group + "." + name, sections: true)
                                            groupHeader(name, filters: children, expanded: sectionExpansion).padding(.leading, 20)
                                            if sectionExpansion.wrappedValue {
                                                ForEach(shown) { row($0).padding(.leading, 40) }
                                            }
                                        }
                                    }
                                } else {
                                    ForEach(visible) { row($0).padding(.leading, 20) }
                                }
                            }
                        }
                    }
                }
            } footer: {
                if selectionChanged {
                    Text("Changes apply to the next scan or Rescan. Open reports keep their current results.")
                }
            }
            Section {
                Button("Enable All") { settings.enableAll(); selectionChanged = true }
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
        .task(id: settings.catalogRevision) {
            ruleOrigins = [:]
            guard let digest = settings.catalogDigest else { return }
            let revision = settings.catalogRevision
            let references = TemplateLibrary.shared.references.filter(\.enabled)
            let access = BundledCatalogAuthorization.shared.snapshot()
            let origins = try? await Task.detached {
                let user = try TemplateFiles.documents(references)
                let bundled: [Data]
                if !access.isAllowed { bundled = [] }
                else if let root = access.managedResources {
                    bundled = try BundledTemplateCatalog.documents(resources: root, manifestPath: "ArsenalCatalog/catalog.json")
                } else { bundled = try BundledTemplateCatalog.documents() }
                return try SearchRuleOrigins.resolve(userDocuments: user, bundledDocuments: bundled, access: access, digest: digest)
            }.value
            guard !Task.isCancelled, settings.catalogRevision == revision, settings.catalogDigest == digest,
                  references == TemplateLibrary.shared.references.filter(\.enabled),
                  access == BundledCatalogAuthorization.shared.snapshot() else { return }
            ruleOrigins = origins ?? [:]
        }
    }

}

#if DEBUG
#Preview("Search settings") {
    SearchSettingsWindow(settings: SearchFilterSettings(
        userDefaults: UserDefaults(suiteName: "BuildHunter.SettingsPreview")!
    ))
}
#endif
