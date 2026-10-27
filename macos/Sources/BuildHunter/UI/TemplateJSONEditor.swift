import AppKit
import SwiftUI

/// Bounded plain text editing. Highlight only the changed paragraph; large files
/// keep plain text to avoid turning each keystroke into a full-document rendering job.
struct TemplateJSONEditor: NSViewRepresentable {
    @Binding var text: String
    var editable: Bool
    var rejected: (String) -> Void

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let view = NSTextView.scrollableTextView()
        guard let editor = view.documentView as? NSTextView else { return view }
        editor.isRichText = false
        editor.isAutomaticQuoteSubstitutionEnabled = false
        editor.isAutomaticDashSubstitutionEnabled = false
        editor.isAutomaticTextReplacementEnabled = false
        editor.isAutomaticSpellingCorrectionEnabled = false
        editor.isContinuousSpellCheckingEnabled = false
        editor.font = .monospacedSystemFont(ofSize: NSFont.systemFontSize, weight: .regular)
        editor.textColor = .textColor
        editor.allowsUndo = true
        editor.delegate = context.coordinator
        editor.setAccessibilityIdentifier("buildhunter.templates.editor")
        return view
    }

    func updateNSView(_ view: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let editor = view.documentView as? NSTextView else { return }
        editor.isEditable = editable
        if editor.string != text {
            editor.string = text
            editor.undoManager?.removeAllActions()
            context.coordinator.highlight(editor, range: NSRange(location: 0, length: (text as NSString).length))
        }
    }

    @MainActor
    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: TemplateJSONEditor
        private let strings = try? NSRegularExpression(pattern: #""(?:\\.|[^"\\])*""#)
        private let values = try? NSRegularExpression(pattern: #"\b(?:true|false|null|-?[0-9]+(?:\.[0-9]+)?)\b"#)
        private var editedRange: NSRange?

        init(_ parent: TemplateJSONEditor) { self.parent = parent }

        func textView(_ textView: NSTextView, shouldChangeTextIn affectedCharRange: NSRange, replacementString: String?) -> Bool {
            guard let replacementString else { return true }
            let current = textView.string as NSString
            guard affectedCharRange.location <= current.length,
                  affectedCharRange.length <= current.length - affectedCharRange.location else { return false }
            guard replacementString.utf8.count <= TemplateCatalog.maximumFileBytes else {
                parent.rejected("A template is limited to 1 MiB.")
                return false
            }
            let next = current.replacingCharacters(in: affectedCharRange, with: replacementString)
            guard next.utf8.count <= TemplateCatalog.maximumFileBytes else {
                parent.rejected("A template is limited to 1 MiB.")
                return false
            }
            editedRange = NSRange(location: affectedCharRange.location, length: (replacementString as NSString).length)
            return true
        }

        func textDidChange(_ notification: Notification) {
            guard let editor = notification.object as? NSTextView else { return }
            parent.text = editor.string
            let value = editor.string as NSString
            let caret = min(editor.selectedRange().location, value.length)
            let changed = editedRange ?? NSRange(location: caret, length: 0)
            editedRange = nil
            let safeRange = NSIntersectionRange(changed, NSRange(location: 0, length: value.length))
            highlight(editor, range: value.paragraphRange(for: safeRange))
        }

        func highlight(_ editor: NSTextView, range: NSRange) {
            guard let storage = editor.textStorage else { return }
            storage.beginEditing()
            defer { storage.endEditing() }
            storage.addAttribute(.foregroundColor, value: NSColor.textColor, range: range)
            guard editor.string.utf8.count <= 131_072 else { return }
            for (expression, color) in [(values, NSColor.systemPurple), (strings, NSColor.systemBlue)] {
                expression?.enumerateMatches(in: editor.string, range: range) { match, _, _ in
                    if let match { storage.addAttribute(.foregroundColor, value: color, range: match.range) }
                }
            }
        }
    }
}
