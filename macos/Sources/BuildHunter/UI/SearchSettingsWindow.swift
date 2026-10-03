import SwiftUI
import NestedA11yIDs

struct SearchSettingsWindow: View {
    @Bindable private var settings: SearchFilterSettings

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
        }
        .formStyle(.grouped)
        .frame(width: 420, height: 460)
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
