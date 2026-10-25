import AppKit
import SwiftUI
import UniformTypeIdentifiers

struct UserTemplatesWindow: View {
    @Bindable var workspace: TemplateWorkspace
    @State private var pending: PendingAction?

    private enum PendingAction { case new, load(TemplateReference) }

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Button("Import JSON…", action: connect)
                    .help("Connect JSON files in their current folder")
                    .accessibilityIdentifier("buildhunter.templates.connect")
                Button("New…") { request(.new) }
                    .accessibilityIdentifier("buildhunter.templates.new")
                Spacer()
                if workspace.busy { ProgressView().controlSize(.small) }
            }
            .disabled(workspace.busy)
            if let error = workspace.library.storageProblem {
                Text(error).foregroundStyle(.red)
                Button("Reset registrations") { workspace.library.resetInvalidRegistrations() }
            }
            HSplitView {
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 12) {
                        ForEach(workspace.library.references) { reference in
                            VStack(alignment: .leading) {
                                Toggle(isOn: Binding(get: { reference.enabled }, set: { workspace.library.setEnabled($0, id: reference.id) })) {
                                    Text(URL(filePath: reference.path).lastPathComponent).lineLimit(1)
                                }
                                Text(reference.path)
                                    .font(.caption).lineLimit(2).truncationMode(.middle)
                                    .multilineTextAlignment(.leading)
                                    .frame(maxWidth: .infinity, alignment: .leading)
                                    .textSelection(.enabled)
                                HStack {
                                    Button("Open") { request(.load(reference)) }
                                    Button("Disconnect") { workspace.library.disconnect(id: reference.id) }
                                }.controlSize(.small)
                            }
                            .padding(.vertical, 4)
                            .accessibilityElement(children: .contain)
                        }
                    }
                    .padding(8)
                }
                .frame(minWidth: 200, idealWidth: 250, maxWidth: 350)
                .disabled(workspace.busy)
                VStack(alignment: .leading) {
                    Text(workspace.reference?.path ?? "New template")
                        .font(.caption).textSelection(.enabled).lineLimit(2)
                    if workspace.isDirty { Text("Unsaved changes").font(.caption).foregroundStyle(.secondary) }
                    TemplateJSONEditor(text: $workspace.text, editable: workspace.hasDocument && !workspace.busy,
                        rejected: workspace.explain)
                        .frame(minHeight: 200)
                    HStack {
                        Button("Check") { Task { await workspace.check() } }
                            .accessibilityIdentifier("buildhunter.templates.check")
                        Button("Reload") {
                            if let reference = workspace.reference { request(.load(reference)) }
                        }.disabled(workspace.reference == nil)
                        Spacer()
                        Button("Export JSON…") { savePanel(exporting: true) }
                            .accessibilityIdentifier("buildhunter.templates.export")
                        Button("Save As…", action: saveAs)
                        Button("Save") { Task { await workspace.save() } }
                            .disabled(workspace.reference == nil || !workspace.isDirty)
                    }
                    .disabled(workspace.busy || !workspace.hasDocument)
                }
                .frame(minWidth: 360)
                .padding(.leading, 8)
            }
            Text(workspace.message)
                .font(.callout).foregroundStyle(workspace.isError ? Color.red : Color.secondary)
                .textSelection(.enabled).lineLimit(4)
                .accessibilityLabel("Template status")
                .accessibilityValue(workspace.message)
                .accessibilityIdentifier("buildhunter.templates.status")
            Text("Files stay where you put them. Git is managed outside BuildHunter. Enabled files are reloaded for each new scan.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding()
        .frame(minWidth: 760, minHeight: 500)
        .confirmationDialog("Discard the unsaved template changes?", isPresented: Binding(
            get: { pending != nil }, set: { if !$0 { pending = nil } })) {
            Button("Discard Changes", role: .destructive) {
                if let action = pending { pending = nil; perform(action) }
            }
            Button("Cancel", role: .cancel) { pending = nil }
        }
    }

    private func request(_ action: PendingAction) {
        if workspace.isDirty { pending = action } else { perform(action) }
    }

    private func perform(_ action: PendingAction) {
        switch action {
        case .new: workspace.newDocument()
        case .load(let reference): Task { await workspace.load(reference) }
        }
    }

    private func connect() {
        let panel = NSOpenPanel()
        panel.allowedContentTypes = [.json]
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = true
        panel.message = "Connect template files from any folder, including your own Git repository."
        panel.begin { response in
            if response == .OK { Task { await workspace.connect(panel.urls) } }
        }
    }

    private func saveAs() {
        savePanel(exporting: false)
    }

    private func savePanel(exporting: Bool) {
        let panel = NSSavePanel()
        panel.allowedContentTypes = [.json]
        panel.nameFieldStringValue = workspace.reference.map { URL(filePath: $0.path).lastPathComponent } ?? "Template.json"
        if exporting { panel.prompt = "Export" }
        panel.begin { response in
            if response == .OK, let url = panel.url {
                Task {
                    if exporting { await workspace.exportJSON(url) }
                    else { await workspace.saveAs(url) }
                }
            }
        }
    }
}

@MainActor
final class TemplateApplicationDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        guard TemplateWorkspace.shared.isDirty else { return .terminateNow }
        let alert = NSAlert()
        alert.messageText = "There are unsaved template changes."
        alert.informativeText = "Cancel to save the JSON file, or quit and discard the draft."
        alert.addButton(withTitle: "Cancel")
        alert.addButton(withTitle: "Quit Without Saving")
        return alert.runModal() == .alertFirstButtonReturn ? .terminateCancel : .terminateNow
    }
}
