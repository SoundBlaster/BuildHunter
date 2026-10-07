import Testing
@testable import BuildHunter

@Suite("Warnings table")
struct ScanWarningsTests {
    @Test("Copy preserves report order, full messages and ignores stale row IDs")
    func selectedMessages() {
        let messages = ["First: /a/very/long/path", "Second\nwith details", "Third"]
        #expect(ScanWarningRow.selectedText(in: messages, selection: [2, 0, 99]) ==
                "First: /a/very/long/path\nThird")
        #expect(ScanWarningRow.selectedText(in: messages, selection: [1]) == "Second\nwith details")
        #expect(ScanWarningRow.selectedText(in: messages, selection: []) == nil)
        #expect(ScanWarningRow.selectedText(in: [], selection: [0]) == nil)
    }

    @Test("Warnings keep their own report alive until the last companion window closes")
    @MainActor
    func warningsLifetime() {
        let store = ScanWindowStore()
        let first = WindowScanModel()
        let second = WindowScanModel()
        store.register(first)
        store.register(second)
        store.prepareWarnings(for: first)
        store.prepareWarnings(for: first)
        store.prepareDiagram(for: first)
        store.scanWindowClosed(first.id)
        store.diagramWindowClosed(first.id)
        #expect(store.model(for: first.id) === first)
        store.warningsWindowClosed(first.id)
        #expect(store.model(for: first.id) == nil)
        #expect(store.model(for: second.id) === second)
        store.scanWindowClosed(second.id)
    }

    @Test("Closing warnings leaves its scan window registered")
    @MainActor
    func closingWarnings() {
        let store = ScanWindowStore()
        let scan = WindowScanModel()
        store.prepareWarnings(for: scan)
        store.warningsWindowClosed(scan.id)
        #expect(store.model(for: scan.id) === scan)
        store.scanWindowClosed(scan.id)
        #expect(store.model(for: scan.id) == nil)
    }
}
