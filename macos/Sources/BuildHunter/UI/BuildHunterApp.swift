import AppKit
import SwiftUI
#if BUILDHUNTER_COMMERCE
import BuildHunterEditionUI
#endif

@main
struct BuildHunterApp: App {
    @NSApplicationDelegateAdaptor(TemplateApplicationDelegate.self) private var templateDelegate
    @State private var windows = ScanWindowStore()
    @State private var settingsTab = "rules"
#if BUILDHUNTER_COMMERCE
    @State private var commerce = CommerceSession(
        productID: Bundle.main.object(forInfoDictionaryKey: "BuildHunterCommerceProductID") as? String,
        onAccessChange: { allowed, expiresAt in
            BundledCatalogAuthorization.shared.update(isAllowed: allowed, expiresAt: expiresAt)
        })
    @Environment(\.scenePhase) private var scenePhase
#endif

    var body: some Scene {
        WindowGroup("BuildHunter") {
            BuildHunterWindow(windowStore: windows)
#if BUILDHUNTER_COMMERCE
                .task { await commerce.start() }
                .onChange(of: scenePhase) { _, phase in
                    if phase == .active { Task { await commerce.refresh() } }
                }
#endif
#if SCREENER_SCREENSHOTS
                .preferredColorScheme(IsDarkScreenshotAppearance().isSatisfiedBy(
                    ProcessInfo.processInfo.environment["SCREENER_APPEARANCE"] ?? "light") ? .dark : .light)
#endif
        }
        .restorationBehavior(.disabled)
        .commands {
            OpenFolderCommands()
            ArtifactDiagramCommands()
            ExportResultsCommands()
        }

        Settings {
            TabView(selection: $settingsTab) {
                SearchSettingsWindow().tabItem { Label("Scan Rules", systemImage: "magnifyingglass") }.tag("rules")
                UserTemplatesWindow(workspace: .shared).tabItem { Label("Templates", systemImage: "curlybraces") }.tag("templates")
#if BUILDHUNTER_COMMERCE
                ArsenalSettingsView(session: commerce, openScanRules: { settingsTab = "rules" }).tabItem { Label("Catalog", systemImage: "square.stack.3d.up") }.tag("catalog")
#endif
            }
            .frame(minWidth: 760, minHeight: 540)
        }

        WindowGroup("Artifact Diagram", id: "artifact-diagram", for: UUID.self) { $scanID in
            if let scanID, let model = windows.model(for: scanID) {
                ArtifactDiagramWindow(scan: model)
#if SCREENER_SCREENSHOTS
                    .preferredColorScheme(IsDarkScreenshotAppearance().isSatisfiedBy(
                        ProcessInfo.processInfo.environment["SCREENER_APPEARANCE"] ?? "light") ? .dark : .light)
#endif
                    .onDisappear { windows.diagramWindowClosed(scanID) }
            } else {
                ContentUnavailableView("Report unavailable", systemImage: "chart.pie",
                                       description: Text("Open a diagram from a scan window."))
            }
        }
        .defaultSize(width: 1_000, height: 700)
        .restorationBehavior(.disabled)
        .commandsRemoved()

        WindowGroup("Rule Profile", id: "rule-profile", for: ScanRuleProfile.self) { $profile in
            if let profile {
                RuleProfileWindow(profile: profile)
            }
        }
        .defaultSize(width: 960, height: 520)
        .restorationBehavior(.disabled)
        .commandsRemoved()

        WindowGroup("Scan Warnings", id: "scan-warnings", for: UUID.self) { $scanID in
            if let scanID, let model = windows.model(for: scanID) {
                ScanWarningsWindow(scan: model)
                    .onDisappear { windows.warningsWindowClosed(scanID) }
            } else {
                ContentUnavailableView("Report unavailable", systemImage: "exclamationmark.triangle",
                                       description: Text("Open warnings from a scan window."))
            }
        }
        .defaultSize(width: 760, height: 420)
        .restorationBehavior(.disabled)
        .commandsRemoved()
    }
}

struct ArtifactDiagramRequest {
    let perform: @MainActor () -> Void
}

private struct ArtifactDiagramFocusedValueKey: FocusedValueKey {
    typealias Value = ArtifactDiagramRequest
}

extension FocusedValues {
    var buildHunterShowDiagram: ArtifactDiagramRequest? {
        get { self[ArtifactDiagramFocusedValueKey.self] }
        set { self[ArtifactDiagramFocusedValueKey.self] = newValue }
    }
}

struct ArtifactDiagramCommands: Commands {
    @FocusedValue(\.buildHunterShowDiagram) private var request

    var body: some Commands {
        CommandGroup(after: .sidebar) {
            Button("Show Artifact Diagram") { request?.perform() }
                .keyboardShortcut("d", modifiers: [.command, .shift])
                .disabled(request == nil)
        }
    }
}

struct ExportResultsRequest {
    let perform: @MainActor () -> Void
}

private struct ExportResultsFocusedValueKey: FocusedValueKey {
    typealias Value = ExportResultsRequest
}

extension FocusedValues {
    var buildHunterExportResults: ExportResultsRequest? {
        get { self[ExportResultsFocusedValueKey.self] }
        set { self[ExportResultsFocusedValueKey.self] = newValue }
    }
}

struct ExportResultsCommands: Commands {
    @FocusedValue(\.buildHunterExportResults) private var request

    var body: some Commands {
        CommandGroup(replacing: .importExport) {
            Button("Export Results…") { request?.perform() }
                .keyboardShortcut("e", modifiers: [.command, .shift])
                .disabled(request == nil)
        }
    }
}

struct OpenFolderRequest {
    let perform: @MainActor () -> Void
}

private struct OpenFolderFocusedValueKey: FocusedValueKey {
    typealias Value = OpenFolderRequest
}

extension FocusedValues {
    var buildHunterOpenFolder: OpenFolderRequest? {
        get { self[OpenFolderFocusedValueKey.self] }
        set { self[OpenFolderFocusedValueKey.self] = newValue }
    }
}

struct OpenFolderCommands: Commands {
    @FocusedValue(\.buildHunterOpenFolder) private var request

    var body: some Commands {
        CommandGroup(after: .newItem) {
            Button("Open Folder…") { request?.perform() }
                .keyboardShortcut("o", modifiers: .command)
        }
    }
}
