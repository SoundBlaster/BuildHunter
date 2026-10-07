import AppKit
import UniformTypeIdentifiers

/// The Save panel behind "Export Results…". Like Preview's Export, the format and the row
/// order are chosen in an accessory view under the file name; the last choices are kept.
@MainActor
final class ScanResultsExportPanel: NSObject {
    private static let formatKey = "exportResultsFormat"
    private static let orderKey = "exportResultsOrder"

    private let panel = NSSavePanel()
    private let formatPopUp = NSPopUpButton(frame: .zero, pullsDown: false)
    private let orderPopUp = NSPopUpButton(frame: .zero, pullsDown: false)
    private let defaults = SearchFilterSettings.defaultUserDefaults
    private let makeReport: (ScanReportExport.Order) -> ScanReportExport

    private var format: ScanReportExport.Format {
        ScanReportExport.Format.allCases[max(0, formatPopUp.indexOfSelectedItem)]
    }
    private var order: ScanReportExport.Order {
        ScanReportExport.Order.allCases[max(0, orderPopUp.indexOfSelectedItem)]
    }

    /// `makeReport` builds the export from a snapshot taken when the command ran.
    init(makeReport: @escaping (ScanReportExport.Order) -> ScanReportExport) {
        self.makeReport = makeReport
        super.init()
        formatPopUp.addItems(withTitles: ScanReportExport.Format.allCases.map(\.title))
        orderPopUp.addItems(withTitles: ScanReportExport.Order.allCases.map(\.title))
        let savedFormat = defaults.string(forKey: Self.formatKey).flatMap(ScanReportExport.Format.init(rawValue:))
        let savedOrder = defaults.string(forKey: Self.orderKey).flatMap(ScanReportExport.Order.init(rawValue:))
        formatPopUp.selectItem(at: ScanReportExport.Format.allCases.firstIndex(of: savedFormat ?? .csv) ?? 0)
        orderPopUp.selectItem(at: ScanReportExport.Order.allCases.firstIndex(of: savedOrder ?? .table) ?? 0)
        formatPopUp.target = self
        formatPopUp.action = #selector(formatChanged)
        formatPopUp.setAccessibilityIdentifier("buildhunter.export.format")
        orderPopUp.setAccessibilityIdentifier("buildhunter.export.order")

        panel.title = "Export Results"
        panel.prompt = "Export"
        panel.canCreateDirectories = true
        panel.isExtensionHidden = false
        panel.accessoryView = accessoryView()
        panel.nameFieldStringValue = makeReport(.table).suggestedFileName(for: format)
        applyFormat()
#if DEBUG
        if let directory = ProcessInfo.processInfo.environment["BUILDHUNTER_EXPORT_DIRECTORY"] {
            panel.directoryURL = URL(fileURLWithPath: directory, isDirectory: true)
        }
#endif
    }

    /// Shows the panel as a sheet on `window` (or on its own) and writes the chosen file.
    /// Returns a message when writing failed, nil when it succeeded or was cancelled.
    func run(on window: NSWindow?) async -> String? {
        let response: NSApplication.ModalResponse
        if let window {
            response = await panel.beginSheetModal(for: window)
        } else {
            response = await panel.begin()
        }
        guard response == .OK, let url = panel.url else { return nil }
        defaults.set(format.rawValue, forKey: Self.formatKey)
        defaults.set(order.rawValue, forKey: Self.orderKey)
        do {
            try makeReport(order).data(as: format).write(to: url, options: .atomic)
            return nil
        } catch {
            return "\(url.lastPathComponent) could not be saved. \(error.localizedDescription)"
        }
    }

    @objc private func formatChanged() {
        applyFormat()
    }

    /// The extension follows the format; a name the user typed keeps its stem.
    private func applyFormat() {
        panel.allowedContentTypes = [format == .csv ? .commaSeparatedText : .json]
        let stem = (panel.nameFieldStringValue as NSString).deletingPathExtension
        panel.nameFieldStringValue = "\(stem).\(format.fileExtension)"
    }

    private func accessoryView() -> NSView {
        func label(_ text: String) -> NSTextField {
            let field = NSTextField(labelWithString: text)
            field.alignment = .right
            return field
        }
        let grid = NSGridView(views: [
            [label("Format:"), formatPopUp],
            [label("Order:"), orderPopUp],
        ])
        grid.rowSpacing = 8
        grid.columnSpacing = 8
        grid.column(at: 0).xPlacement = .trailing
        grid.rowAlignment = .firstBaseline
        let size = grid.fittingSize
        let container = NSView(frame: NSRect(x: 0, y: 0, width: size.width + 40, height: size.height + 24))
        grid.frame = NSRect(x: 20, y: 12, width: size.width, height: size.height)
        container.addSubview(grid)
        return container
    }
}
