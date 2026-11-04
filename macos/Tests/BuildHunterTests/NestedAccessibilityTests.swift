import AppKit
import NestedA11yIDs
import SwiftUI
import Testing
@testable import BuildHunter

@Suite("Native accessibility context")
@MainActor
struct NestedAccessibilityTests {
    @Test("The template editor follows its enclosing accessibility prefix")
    func editorMovesWithContext() throws {
        func content(_ prefix: String) -> some View {
            TemplateJSONEditor(text: .constant("{}"), editable: true, rejected: { _ in })
                .nestedAccessibilityIdentifier("editor")
                .environment(\.accessibilityPrefix, prefix)
        }

        let host = NSHostingView(rootView: content("first"))
        host.frame = NSRect(x: 0, y: 0, width: 400, height: 240)
        let window = NSWindow(contentRect: host.frame, styleMask: [.titled], backing: .buffered, defer: false)
        window.contentView = host
        host.layoutSubtreeIfNeeded()

        func textView(in view: NSView) -> NSTextView? {
            if let editor = view as? NSTextView { return editor }
            return view.subviews.lazy.compactMap { textView(in: $0) }.first
        }

        let editor = try #require(textView(in: host))
        #expect(editor.accessibilityIdentifier() == "first.editor")
        host.rootView = content("second")
        host.layoutSubtreeIfNeeded()
        let movedEditor = try #require(textView(in: host))
        #expect(movedEditor.accessibilityIdentifier() == "second.editor")
    }
}
