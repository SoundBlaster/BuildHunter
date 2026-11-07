import AppKit
import SwiftUI
#if BUILDHUNTER_COMMERCE
import BuildHunterEditionUI
#endif

#if BUILDHUNTER_MANAGED_ASSETS
@MainActor private let managedCatalogDelivery = CatalogAssetDelivery { root, expiry in
    BundledCatalogAuthorization.shared.update(isAllowed: false)
    if let root {
        _ = try BundledTemplateCatalog.documents(resources: root, manifestPath: "ArsenalCatalog/catalog.json")
        BundledTemplateCatalog.setManagedResources(root)
        BundledCatalogAuthorization.shared.update(isAllowed: true, expiresAt: expiry)
    }
}
#endif

@main
struct BuildHunterApp: App {
    @NSApplicationDelegateAdaptor(TemplateApplicationDelegate.self) private var templateDelegate
    @State private var windows = ScanWindowStore()
    @State private var settingsTab = "rules"
#if BUILDHUNTER_COMMERCE
    @State private var catalogNavigationError: String?
    @State private var commerce = CommerceSession(
        productID: Bundle.main.object(forInfoDictionaryKey: "BuildHunterCommerceProductID") as? String,
        onAccessChange: { allowed, expiresAt in
#if BUILDHUNTER_MANAGED_ASSETS
            managedCatalogDelivery.update(isAllowed: allowed, expiresAt: expiresAt)
#else
            BundledCatalogAuthorization.shared.update(isAllowed: allowed, expiresAt: expiresAt)
#endif
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
#if BUILDHUNTER_COMMERCE
                SearchSettingsWindow(catalogUnlocked: catalogUnlocked, openCatalog: { settingsTab = "catalog" })
                    .tabItem { Label("Scan Rules", systemImage: "magnifyingglass") }.tag("rules")
#else
                SearchSettingsWindow().tabItem { Label("Scan Rules", systemImage: "magnifyingglass") }.tag("rules")
#endif
                UserTemplatesWindow(workspace: .shared).tabItem { Label("Templates", systemImage: "curlybraces") }.tag("templates")
#if BUILDHUNTER_COMMERCE
#if BUILDHUNTER_MANAGED_ASSETS
                ArsenalSettingsView(session: commerce, openScanRules: openCatalogRules, delivery: managedCatalogDelivery, row: catalogPreviewRow).tabItem { Label("Catalog", systemImage: "square.stack.3d.up") }.tag("catalog")
#else
                ArsenalSettingsView(session: commerce, openScanRules: openCatalogRules, row: catalogPreviewRow).tabItem { Label("Catalog", systemImage: "square.stack.3d.up") }.tag("catalog")
#endif
#endif
            }
            .frame(minWidth: 760, minHeight: 540)
#if BUILDHUNTER_MANAGED_ASSETS
            .onChange(of: managedCatalogDelivery.state, initial: true) { _, state in
                if state == .ready { refreshCatalogRules(navigate: false) }
            }
#endif
#if BUILDHUNTER_COMMERCE
            .alert("Unable to load scan rules", isPresented: Binding(
                get: { catalogNavigationError != nil },
                set: { if !$0 { catalogNavigationError = nil } }
            )) { Button("OK") { catalogNavigationError = nil } } message: {
                Text(catalogNavigationError ?? "")
            }
#endif
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

#if BUILDHUNTER_COMMERCE
    private var catalogUnlocked: Bool {
        switch commerce.access {
        case .purchased, .trial: true
        case .locked, .disabled: false
        }
    }

    private func catalogPreviewRow(id: String, title: Text, kind: Text, expanded: Bool,
                                   action: @escaping () -> Void) -> some View {
        Button(action: action) {
            ScanRuleRow {
                HStack(spacing: 8) {
                    ScanRuleIcon(id: id, group: "", isUser: false)
                    title
                }
            } secondary: {
                kind.foregroundStyle(.secondary)
                Image(systemName: expanded ? "chevron.down" : "chevron.right")
                    .foregroundStyle(.secondary).frame(width: 18)
            }
        }
        .buttonStyle(.plain)
        .accessibilityValue(expanded ? "Expanded" : "Collapsed")
    }

    @MainActor
    private func openCatalogRules() {
        refreshCatalogRules(navigate: true)
    }

    @MainActor
    private func refreshCatalogRules(navigate: Bool) {
        let library = TemplateLibrary.shared
        if let problem = library.storageProblem {
            catalogNavigationError = problem
            return
        }
        let references = library.references.filter(\.enabled)
        let access = BundledCatalogAuthorization.shared.snapshot()
        guard access.isAllowed else {
            catalogNavigationError = String(localized: "Catalog access changed. Try again.")
            return
        }
        Task {
            do {
                let metadata = try await Task.detached {
                    let documents = try references.map { try TemplateFiles.read($0) }
                    return try TemplateCatalog.metadata(documents: documents, access: access)
                }.value
                guard access == BundledCatalogAuthorization.shared.snapshot() else {
                    throw TemplateProblem(code: "access_changed", location: "", message: String(localized: "Catalog access changed. Try again."))
                }
                guard library.storageProblem == nil, references == library.references.filter(\.enabled) else {
                    throw TemplateProblem(code: "templates_changed", location: "", message: String(localized: "Templates changed. Try again."))
                }
                SearchFilterSettings.shared.useCatalog(metadata)
                if navigate { settingsTab = "rules" }
            } catch {
                catalogNavigationError = error.localizedDescription
            }
        }
    }
#endif
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
