import AppKit
import SwiftUI
import NestedA11yIDs
import UniformTypeIdentifiers

enum FinderRevealOutcome {
    case opened
    case openedParent(URL)
    case unavailable
}

@MainActor
enum ArtifactFolderActions {
    static func copyPath(_ url: URL) {
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(url.path, forType: .string)
    }

    static func reveal(_ url: URL, accessRoot: URL?) -> FinderRevealOutcome {
        let startedAccess = accessRoot?.startAccessingSecurityScopedResource() ?? false
        defer {
            if startedAccess { accessRoot?.stopAccessingSecurityScopedResource() }
        }

        let requested = url.standardizedFileURL
        guard let existingFolder = nearestExistingFolder(to: requested) else { return .unavailable }
        NSWorkspace.shared.activateFileViewerSelecting([existingFolder])
        return existingFolder.path == requested.path ? .opened : .openedParent(existingFolder)
    }

    private static func nearestExistingFolder(to url: URL) -> URL? {
        let fileManager = FileManager.default
        var candidate = url
        while true {
            var isDirectory: ObjCBool = false
            if fileManager.fileExists(atPath: candidate.path, isDirectory: &isDirectory), isDirectory.boolValue {
                return candidate
            }
            let parent = candidate.deletingLastPathComponent()
            guard parent.path != candidate.path else { return nil }
            candidate = parent
        }
    }
}

extension View {
    func artifactFolderContextMenu(
        url: URL?,
        onReveal: @escaping () -> Void
    ) -> some View {
        contextMenu {
            Button("Show in Finder", systemImage: "folder", action: onReveal)
                .disabled(url == nil)
            Button("Copy Path", systemImage: "document.on.document") {
                if let url { ArtifactFolderActions.copyPath(url) }
            }
            .disabled(url == nil)
        }
    }
}

struct BuildHunterWindow: View {
    @State private var model: WindowScanModel
    @Environment(\.openWindow) private var openWindow
    private let windowStore: ScanWindowStore?

    init(model: WindowScanModel = WindowScanModel(), windowStore: ScanWindowStore? = nil) {
        _model = State(initialValue: model)
        self.windowStore = windowStore
    }

    var body: some View {
        VStack(spacing: 0) {
            readOnlyBanner
            if let targetName = model.targetName {
                report(targetName: targetName)
                ScanReportFooter(profile: model.profile, isScanning: model.isScanning)
                    .a11yRoot("buildhunter.report")
            } else {
                emptyState
            }
        }
        .frame(minWidth: 760, minHeight: 460)
        .toolbar {
            ToolbarItem(placement: .automatic) {
                VStack {
                    Button("Diagram", systemImage: "chart.pie.fill") { showDiagram() }
                        .disabled(windowStore == nil)
                        .help("Show a live diagram of this scan")
                        .nestedAccessibilityIdentifier("openDiagram")
                }
                .a11yRoot("buildhunter.toolbar")
            }
#if DEBUG
            ToolbarItem(placement: .automatic) {
                VStack {
                    Menu {
                        ForEach(MockScanState.allCases) { mockState in
                            Button(mockState.title) { model.showMockState(mockState) }
                                .nestedAccessibilityIdentifier("scenario.\(mockState.id.rawValue)")
                        }
                    } label: {
                        Label("Mock State", systemImage: "testtube.2")
                    }
                    .nestedAccessibilityIdentifier("mockState")
                }
                .a11yRoot("buildhunter.toolbar")
            }
#endif
        }
        .fileImporter(isPresented: $model.isChoosingFolder, allowedContentTypes: [.folder]) { result in
            if case .success(let url) = result {
                model.accept(target: url)
            }
        }
        .dropDestination(for: URL.self) { urls, _ in
            guard let folder = urls.first, folder.hasDirectoryPath else { return false }
            model.accept(target: folder)
            return true
        }
        .onAppear { windowStore?.register(model) }
        .onDisappear {
            model.stop()
            windowStore?.scanWindowClosed(model.id)
        }
        .focusedSceneValue(\.buildHunterOpenFolder, OpenFolderRequest {
            model.isChoosingFolder = true
        })
        .focusedSceneValue(\.buildHunterShowDiagram, ArtifactDiagramRequest {
            showDiagram()
        })
    }

    private func showDiagram() {
        guard let windowStore else { return }
        windowStore.prepareDiagram(for: model)
        openWindow(id: "artifact-diagram", value: model.id)
    }

    private var readOnlyBanner: some View {
        Label {
            Text("Read-only scan · artifacts are measured as the folder is traversed.")
                .font(.callout.weight(.medium))
        } icon: {
            Image(systemName: "eye.fill")
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 18)
        .padding(.vertical, 11)
        .background(.yellow.opacity(0.16))
    }

    private var emptyState: some View {
        ContentUnavailableView {
            Label("Choose a project folder", systemImage: "folder.badge.questionmark")
        } description: {
            Text("Drop a folder here or use File → Open Folder… to scan local build artifacts.")
        } actions: {
            Button("Open Folder…", systemImage: "folder.fill") { model.isChoosingFolder = true }
                .keyboardShortcut("o", modifiers: .command)
                .nestedAccessibilityIdentifier("openFolder")
        }
        .a11yRoot("buildhunter.empty")
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func report(targetName: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    HStack(alignment: .firstTextBaseline, spacing: 7) {
                        Image(systemName: "folder.fill").accessibilityHidden(true)
                        Text("\(targetName)")
                            .nestedAccessibilityIdentifier("target")
                    }
                    .font(.title2.weight(.semibold))
                    Label {
                        Text(statusDescription)
                            .nestedAccessibilityIdentifier("status")
                    } icon: {
                        Image(systemName: statusSymbolName).accessibilityHidden(true)
                    }
                    .foregroundStyle(.secondary)
                }
                Spacer()
                if model.isScanning {
                    Button("Stop", systemImage: "stop.circle.fill") { model.stop() }
                } else if model.phase != .idle {
                    Button("Rescan", systemImage: "arrow.clockwise") { model.rescan() }
                }
            }
            if !model.warnings.isEmpty {
                DisclosureGroup("Warnings and details (\(model.warnings.count))") {
                    ForEach(Array(model.warnings.enumerated()), id: \.offset) { _, warning in
                        Text(warning).font(.callout).foregroundStyle(.secondary)
                    }
                }
                .foregroundStyle(.orange)
                .nestedAccessibilityIdentifier("warnings")
            }
            ArtifactReportTable(scan: model)
        }
        .padding(18)
        .a11yRoot("buildhunter.report")
    }

    private var statusDescription: String {
        switch model.phase {
        case .idle: ""
        case .scanning: "Searching…"
        case .completed: "Scan complete"
        case .stopped: "Scan stopped · partial results"
        case .incomplete: "Scan incomplete · review warnings"
        }
    }

    private var statusSymbolName: String {
        switch model.phase {
        case .idle: "folder.fill"
        case .scanning: "hourglass"
        case .completed: "checkmark.circle.fill"
        case .stopped: "ellipsis.circle"
        case .incomplete: "exclamationmark.circle.fill"
        }
    }

}

private struct ArtifactReportTable: View {
    let scan: WindowScanModel
    @State private var model = ScanTableModel()
    @State private var finderNotice: String?

    var body: some View {
        Table(model.rows, sortOrder: $model.sortOrder) {
            TableColumn("Path", sortUsing: ScanRowComparator(column: .path)) { row in
                ArtifactReportCell(text: row.relativePath, folderURL: artifactURL(for: row)) {
                    reveal(row)
                }
            }
            .width(min: 240, ideal: 360)
            TableColumn("Size", sortUsing: ScanRowComparator(column: .size)) { row in
                ArtifactReportCell(text: sizeDescription(row.size), folderURL: artifactURL(for: row)) {
                    reveal(row)
                }
                .monospacedDigit()
            }
            .width(min: 100, ideal: 125)
            TableColumn("Language", sortUsing: ScanRowComparator(column: .language)) { row in
                ArtifactReportCell(text: row.language, language: ArtifactLanguage(rawValue: row.language),
                                   folderURL: artifactURL(for: row)) { reveal(row) }
            }
            .width(min: 90, ideal: 120)
            TableColumn("Kind", sortUsing: ScanRowComparator(column: .kind)) { row in
                ArtifactReportCell(text: row.kind.rawValue, folderURL: artifactURL(for: row)) {
                    reveal(row)
                }
            }
            .width(min: 110, ideal: 150)
        }
        .nestedAccessibilityIdentifier("table")
        .alert("Folder location", isPresented: Binding(
            get: { finderNotice != nil },
            set: { if !$0 { finderNotice = nil } }
        )) {
            Button("OK", role: .cancel) { finderNotice = nil }
        } message: {
            Text(finderNotice ?? "")
        }
        .overlay {
            if scan.rows.isEmpty && scan.isScanning {
                ProgressView("Searching for build artifacts…")
                    .padding()
                    .background(.regularMaterial, in: .rect(cornerRadius: 10))
            }
        }
        .task { await model.follow(scan) }
    }

    private func artifactURL(for row: ScanRow) -> URL? {
        scan.targetURL?.appendingPathComponent(row.relativePath, isDirectory: true)
    }

    private func reveal(_ row: ScanRow) {
        guard let url = artifactURL(for: row) else { return }
        switch ArtifactFolderActions.reveal(url, accessRoot: scan.targetURL) {
        case .opened:
            finderNotice = nil
        case .openedParent(let parent):
            finderNotice = "The folder is no longer available. Opened \(parent.path) in Finder instead."
        case .unavailable:
            finderNotice = "No existing parent folder is available to open in Finder."
        }
    }

    private func sizeDescription(_ state: SizeState) -> String {
        switch state {
        case .measuring: "Measuring…"
        case .measured(let bytes): ByteCountFormatter.string(fromByteCount: bytes, countStyle: .binary)
        case .partial(let bytes): bytes.map {
            "\(ByteCountFormatter.string(fromByteCount: $0, countStyle: .binary)) partial"
        } ?? "Partial · size unknown"
        }
    }
}

private struct ArtifactReportCell: View {
    let text: String
    var language: ArtifactLanguage? = nil
    var folderURL: URL? = nil
    var onReveal: () -> Void = {}
    @State private var isHovered = false

    var body: some View {
        HStack {
            if let language { ArtifactLanguageIcon(language: language) }
            Text(text)
        }
            .frame(maxWidth: .infinity, minHeight: 24, alignment: .leading)
            .background(Color.accentColor.opacity(isHovered ? 0.16 : 0), in: .rect(cornerRadius: 4))
            .contentShape(Rectangle())
            .onHover { isHovered = $0 }
            .artifactFolderContextMenu(url: folderURL, onReveal: onReveal)
            .onTapGesture(count: 2) {
                guard folderURL != nil else { return }
                onReveal()
            }
    }
}

/// Shared bottom chrome keeps both scan and diagram windows aligned.
struct WindowStatusBar<Content: View>: View {
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(spacing: 0) {
            Divider()
            HStack(alignment: .firstTextBaseline, spacing: 0) {
                content()
                    .font(.caption)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 12)
            .frame(maxWidth: .infinity, minHeight: 38, maxHeight: 38, alignment: .leading)
        }
    }
}

private struct ScanReportFooter: View {
    let profile: ScanProfileHistory
    let isScanning: Bool

    var body: some View {
        WindowStatusBar {
            // Fill the bar so the profile chart sits at the trailing edge.
            HStack(alignment: .center, spacing: 12) {
                Label {
                    Text("BuildHunter only reads files and folders. It never deletes artifacts.")
                        .nestedAccessibilityIdentifier("readOnlyStatus")
                } icon: {
                    Image(systemName: "eye.fill").accessibilityHidden(true)
                }
                Spacer(minLength: 0)
                ScanProfileStatusItem(profile: profile, isScanning: isScanning)
            }
            .frame(maxWidth: .infinity)
        }
    }
}

#Preview("Empty window") {
    BuildHunterWindow()
}

#Preview("Streaming results") {
    let model = WindowScanModel(source: PreviewIdleScanSource())
    model.acceptDemoTarget(named: "Sample project")
    model.apply(.discovered(generation: model.generation,
                             artifact: ScanArtifact(id: UUID(uuidString: "A0000000-0000-4000-8000-000000000002")!,
                                                    relativePath: "DemoFixture/App/.build",
                                                    language: "Swift", kind: .buildOutput)))
    return BuildHunterWindow(model: model)
}

#Preview("Stopped with unknown size") {
    let model = WindowScanModel(source: PreviewIdleScanSource())
    model.acceptDemoTarget(named: "Sample project")
    model.apply(.discovered(generation: model.generation,
                             artifact: ScanArtifact(id: UUID(uuidString: "A0000000-0000-4000-8000-000000000003")!,
                                                    relativePath: "DemoFixture/App/.build",
                                                    language: "Swift", kind: .buildOutput)))
    model.stop()
    return BuildHunterWindow(model: model)
}

#Preview("Incomplete with warning") {
    let model = WindowScanModel(source: PreviewIdleScanSource())
    model.acceptDemoTarget(named: "Sample project")
    model.apply(.warning(generation: model.generation, message: "Demo warning detail"))
    model.apply(.finished(generation: model.generation, result: .completed))
    return BuildHunterWindow(model: model)
}

private struct PreviewIdleScanSource: ScanEventSource {
    func events(for generation: UInt64, target: URL?) -> AsyncStream<ScanEvent> {
        AsyncStream { _ in }
    }

    func cancel(generation: UInt64) {}
}

/// Bundled assets keep badge rendering local and independent of filesystem scanning.
struct ArtifactLanguageIcon: View {
    let language: ArtifactLanguage

    var body: some View {
        Image(language.assetName)
            .renderingMode(language == .rust ? .template : .original)
            .resizable()
            .scaledToFit()
            .frame(width: 16, height: 16)
            .foregroundStyle(.primary)
            .help(language.rawValue)
            .accessibilityHidden(true)
    }
}

struct ArtifactLanguageBadges: View {
    let languages: Set<ArtifactLanguage>

    private var ordered: [ArtifactLanguage] {
        ArtifactLanguage.allCases.filter { languages.contains($0) }
    }

    var body: some View {
        HStack(spacing: 4) {
            ForEach(ordered, id: \.self) { language in
                ArtifactLanguageIcon(language: language)
            }
        }
        .fixedSize()
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Artifact languages")
        .accessibilityValue(ordered.map(\.rawValue).joined(separator: ", "))
    }
}
