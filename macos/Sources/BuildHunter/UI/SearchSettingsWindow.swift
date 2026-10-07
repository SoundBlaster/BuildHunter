import SwiftUI
import NestedA11yIDs

struct SearchSettingsWindow: View {
    @Bindable private var settings: SearchFilterSettings
    @AppStorage(ScanWarningsPresentation.storageKey, store: SearchFilterSettings.defaultUserDefaults)
    private var warningsPresentation = ScanWarningsPresentation.window

    init(settings: SearchFilterSettings = .shared) {
        self.settings = settings
    }

    var body: some View {
        Form {
            Section {
                Text("Choose which supported build outputs, caches, and environments appear in new scans.")
                    .foregroundStyle(.secondary)
                ForEach(settings.groups, id: \.self) { group in
                    VStack(alignment: .leading, spacing: 8) {
                        Toggle(group, isOn: Binding(
                            get: { settings.filters(in: group).allSatisfy { settings.isEnabled(for: $0.id) } },
                            set: { settings.setEnabled($0, group: group) }
                        ))
                        .font(.headline)
                        .nestedAccessibilityIdentifier("group.\(group)")
                        ForEach(settings.filters(in: group)) { filter in
                            Toggle(filter.title, isOn: Binding(
                                get: { settings.isEnabled(for: filter.id) },
                                set: { settings.setEnabled($0, for: filter.id) }
                            ))
                            .nestedAccessibilityIdentifier("filter.\(filter.id)")
                        }
                    }
                    .padding(.vertical, 4)
                }
            } footer: {
                Text("Changes apply to the next scan or Rescan. Open reports keep their current results.")
            }

            Section {
                Button("Enable All") { settings.enableAll() }
                    .nestedAccessibilityIdentifier("enableAll")
            }

            Section {
                Picker("Show scan warnings in", selection: $warningsPresentation) {
                    ForEach(ScanWarningsPresentation.allCases) { Text($0.title).tag($0) }
                }
                .nestedAccessibilityIdentifier("warningsPresentation")
            } header: {
                Text("Experiments")
            } footer: {
                Text("Right-click the warnings button in a report to open the other one.")
            }
        }
        .formStyle(.grouped)
        .frame(width: 420, height: 540)
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
