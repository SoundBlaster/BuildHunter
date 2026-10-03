import AppKit
import SwiftUI

@main
struct BuildHunterApp: App {
    @State private var windows = ScanWindowStore()

    var body: some Scene {
        WindowGroup("BuildHunter") {
            BuildHunterWindow(windowStore: windows)
        }
        .restorationBehavior(.disabled)
        .commands {
            OpenFolderCommands()
            ArtifactDiagramCommands()
        }

        Settings {
            SearchSettingsWindow()
        }

        WindowGroup("Artifact Diagram", id: "artifact-diagram", for: UUID.self) { $scanID in
            if let scanID, let model = windows.model(for: scanID) {
                ArtifactDiagramWindow(scan: model)
                    .onDisappear { windows.diagramWindowClosed(scanID) }
            } else {
                ContentUnavailableView("Report unavailable", systemImage: "chart.pie",
                                       description: Text("Open a diagram from a scan window."))
            }
        }
        .defaultSize(width: 1_000, height: 700)
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
