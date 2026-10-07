import AppKit
import SwiftUI
import NestedA11yIDs

struct ScanWarningRow: Identifiable {
    let id: Int
    let message: String

    static func selectedText(in warnings: [String], selection: Set<Int>) -> String? {
        let messages = warnings.enumerated().compactMap { index, message in
            selection.contains(index) ? message : nil
        }
        return messages.isEmpty ? nil : messages.joined(separator: "\n")
    }
}

struct ScanWarningsWindow: View {
    let scan: WindowScanModel
    @State private var selection: Set<Int> = []
    @Environment(\.dismissWindow) private var dismissWindow

    private var rows: [ScanWarningRow] {
        scan.warnings.enumerated().map { ScanWarningRow(id: $0.offset, message: $0.element) }
    }

    var body: some View {
        VStack(spacing: 0) {
            warningsTable
        }
        .toolbar {
            Button("Copy selected warnings", systemImage: "document.on.document") { copy(selection) }
                .disabled(selection.isEmpty)
                .help("Copy selected warnings (⌘C)")
                .nestedAccessibilityIdentifier("copy")
        }
        .frame(minWidth: 420, minHeight: 240)
        .navigationTitle("\(scan.targetName ?? "BuildHunter") — Scan Warnings")
        .onChange(of: scan.reportID) { selection.removeAll() }
        .onExitCommand { dismissWindow(id: "scan-warnings", value: scan.id) }
        .environment(\.accessibilityPrefix, "buildhunter.warnings")
    }

    private var warningsTable: some View {
        Table(rows, selection: $selection) {
            TableColumn("#") { row in
                Text("\(row.id + 1)").foregroundStyle(.secondary).monospacedDigit()
            }
            .width(40)
            TableColumn("Warning") { row in
                Text(row.message)
                    .lineLimit(nil)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .nestedAccessibilityIdentifier("table")
        .contextMenu(forSelectionType: Int.self) { selectedRows in
            Button("Copy", systemImage: "document.on.document") { copy(selectedRows) }
                .disabled(ScanWarningRow.selectedText(in: scan.warnings, selection: selectedRows) == nil)
        }
        .onCopyCommand {
            guard let text = ScanWarningRow.selectedText(in: scan.warnings, selection: selection) else { return [] }
            return [NSItemProvider(object: text as NSString)]
        }
        .overlay {
            if rows.isEmpty {
                ContentUnavailableView("No warnings", systemImage: "checkmark.circle")
            }
        }
    }

    private func copy(_ selectedRows: Set<Int>) {
        guard let text = ScanWarningRow.selectedText(in: scan.warnings, selection: selectedRows) else { return }
        NSPasteboard.general.clearContents()
        NSPasteboard.general.setString(text, forType: .string)
    }
}
