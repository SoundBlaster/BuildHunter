import SwiftUI
import NestedA11yIDs
import UniformTypeIdentifiers

struct BuildHunterWindow: View {
    @State private var model: WindowScanModel

    init(model: WindowScanModel = WindowScanModel()) {
        _model = State(initialValue: model)
    }

    var body: some View {
        VStack(spacing: 0) {
            simulationBanner
            if let targetName = model.targetName {
                report(targetName: targetName)
            } else {
                emptyState
            }
        }
        .frame(minWidth: 760, minHeight: 460)
        .toolbar {
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
        .onDisappear {
            model.stop()
        }
        .focusedSceneValue(\.buildHunterOpenFolder, OpenFolderRequest {
            model.isChoosingFolder = true
        })
    }

    private var simulationBanner: some View {
            Label {
            Text("Read-only scan · artifacts are measured as the folder is traversed.")
                .font(.callout.weight(.medium))
        } icon: {
            Image(systemName: "exclamationmark.triangle.fill")
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
            Button("Open Folder…") { model.isChoosingFolder = true }
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
                    Text("\(targetName)")
                        .font(.title2.weight(.semibold))
                        .nestedAccessibilityIdentifier("target")
                    Text(statusDescription)
                        .foregroundStyle(.secondary)
                        .nestedAccessibilityIdentifier("status")
                }
                Spacer()
                if model.isScanning {
                    Button("Stop") { model.stop() }
                } else if model.phase != .idle {
                    Button("Rescan") { model.rescan() }
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
            Table(model.rows) {
                TableColumn("Path", value: \.relativePath)
                    .width(min: 240, ideal: 360)
                TableColumn("Size") { row in Text(sizeDescription(row.size)) }
                    .width(min: 100, ideal: 125)
                TableColumn("Language", value: \.language)
                    .width(min: 90, ideal: 120)
                TableColumn("Kind") { row in Text(row.kind.rawValue) }
                    .width(min: 110, ideal: 150)
            }
            .overlay {
                if model.rows.isEmpty && model.isScanning {
                ProgressView("Searching for build artifacts…")
                        .padding()
                        .background(.regularMaterial, in: .rect(cornerRadius: 10))
                }
            }
            Text("BuildHunter only reads files and folders. It never deletes artifacts.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(18)
        .a11yRoot("buildhunter.report")
    }

    private func sizeDescription(_ state: SizeState) -> String {
        switch state {
        case .measuring: "Simulating…"
        case .measured(let bytes): binarySize(bytes)
        case .partial(let bytes): bytes.map { "\(binarySize($0)) partial" } ?? "Partial · size unknown"
        }
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

    private func binarySize(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .binary
        return formatter.string(fromByteCount: bytes)
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
