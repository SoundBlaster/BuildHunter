import AppKit
import SwiftUI

@main
struct BuildHunterApp: App {
    var body: some Scene {
        WindowGroup("BuildHunter") {
            BuildHunterWindow()
        }
        .restorationBehavior(.disabled)
        .commands {
            OpenFolderCommands()
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
