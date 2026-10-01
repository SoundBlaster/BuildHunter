import AppKit
import SwiftUI

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
        .onChange(of: model.isChoosingFolder) { _, shouldChoose in
            guard shouldChoose else { return }
            chooseFolder()
        }
        .dropDestination(for: URL.self) { urls, _ in
            guard let folder = urls.first, folder.hasDirectoryPath else { return false }
            model.acceptDemoTarget(named: folder.lastPathComponent)
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
            Text("DEMO SCANNER — paths and sizes below are simulated; no files are read.")
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
            Text("Drop a folder here or use File → Open Folder… to preview the demo scanner.")
        } actions: {
            Button("Open Folder…") { model.isChoosingFolder = true }
                .keyboardShortcut("o", modifiers: .command)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private func report(targetName: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                VStack(alignment: .leading, spacing: 3) {
                    Text("Demo for \(targetName)").font(.title2.weight(.semibold))
                    Text(statusDescription)
                        .foregroundStyle(.secondary)
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
                    ProgressView("Waiting for simulated events…")
                        .padding()
                        .background(.regularMaterial, in: .rect(cornerRadius: 10))
                }
            }
            Text("Paths are fixture labels, not detected locations. Finder and Copy Path are unavailable in this skeleton.")
                .font(.caption)
                .foregroundStyle(.secondary)
        }
        .padding(18)
    }

    private func sizeDescription(_ state: SizeState) -> String {
        switch state {
        case .measuring: "Simulating…"
        case .simulated(let bytes): "~\(binarySize(bytes)) (demo)"
        case .partial(let bytes): bytes.map { "~\(binarySize($0)) partial demo" } ?? "Partial · size unknown"
        }
    }

    private var statusDescription: String {
        switch model.phase {
        case .idle: ""
        case .scanning: "Showing simulated streaming results"
        case .completed: "Demo complete"
        case .stopped: "Demo stopped · partial results"
        case .incomplete: "Demo incomplete · review warnings"
        }
    }

    private func binarySize(_ bytes: Int64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .binary
        return formatter.string(fromByteCount: bytes)
    }

    private func chooseFolder() {
        defer { model.isChoosingFolder = false }
        let panel = NSOpenPanel()
        panel.canChooseFiles = false
        panel.canChooseDirectories = true
        panel.allowsMultipleSelection = false
        panel.prompt = "Preview Demo"
        if panel.runModal() == .OK, let url = panel.url {
            model.acceptDemoTarget(named: url.lastPathComponent)
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
                             artifact: DemoArtifact(id: UUID(uuidString: "A0000000-0000-4000-8000-000000000002")!,
                                                    relativePath: "DemoFixture/App/.build", simulatedBytes: 0,
                                                    language: "Swift", kind: .buildOutput)))
    return BuildHunterWindow(model: model)
}

#Preview("Stopped with unknown size") {
    let model = WindowScanModel(source: PreviewIdleScanSource())
    model.acceptDemoTarget(named: "Sample project")
    model.apply(.discovered(generation: model.generation,
                             artifact: DemoArtifact(id: UUID(uuidString: "A0000000-0000-4000-8000-000000000003")!,
                                                    relativePath: "DemoFixture/App/.build", simulatedBytes: 0,
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
    func events(for generation: UInt64) -> AsyncStream<ScanEvent> {
        AsyncStream { _ in }
    }
}
