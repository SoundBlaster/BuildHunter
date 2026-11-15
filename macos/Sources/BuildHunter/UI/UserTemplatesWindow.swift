import AppKit
import NestedA11yIDs
import SwiftUI
import UniformTypeIdentifiers

struct UserTemplatesWindow: View {
    @Bindable var workspace: TemplateWorkspace
    @State private var pending: PendingAction?
    @State private var showingHelp = false

    private enum PendingAction { case new, load(TemplateReference) }

    var body: some View {
        VStack(alignment: .leading) {
            HStack {
                Button("Import JSON…", action: connect)
                    .help("Connect JSON files in their current folder")
                    .nestedAccessibilityIdentifier("connect")
                Button("New…") { request(.new) }
                    .nestedAccessibilityIdentifier("new")
                Button("Template Help") { showingHelp = true }
                    .nestedAccessibilityIdentifier("help")
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
                .frame(minWidth: 140, idealWidth: 220, maxWidth: 350)
                .disabled(workspace.busy)
                VStack(alignment: .leading) {
                    Text(workspace.reference?.path ?? "New template")
                        .font(.caption).textSelection(.enabled).lineLimit(2)
                    if workspace.isDirty { Text("Unsaved changes").font(.caption).foregroundStyle(.secondary) }
                    TemplateJSONEditor(text: $workspace.text, editable: workspace.hasDocument && !workspace.busy,
                        rejected: workspace.explain)
                        .nestedAccessibilityIdentifier("editor")
                        .frame(minHeight: 200)
                    ScrollView(.horizontal) {
                        HStack {
                            Button("Check") { Task { await workspace.check() } }
                                .nestedAccessibilityIdentifier("check")
                            Button("Reload") {
                                if let reference = workspace.reference { request(.load(reference)) }
                            }.disabled(workspace.reference == nil)
                            Spacer()
                            Button("Export JSON…") { savePanel(exporting: true) }
                                .nestedAccessibilityIdentifier("export")
                            Button("Save As…", action: saveAs)
                            Button("Save") { Task { await workspace.save() } }
                                .disabled(workspace.reference == nil || !workspace.isDirty)
                        }
                    }
                    .fixedSize(horizontal: false, vertical: true)
                    .disabled(workspace.busy || !workspace.hasDocument)
                }
                .frame(minWidth: 300)
                .padding(.leading, 8)
            }
            Text(workspace.message)
                .font(.callout).foregroundStyle(workspace.isError ? Color.red : Color.secondary)
                .textSelection(.enabled).lineLimit(4)
                .accessibilityLabel("Template status")
                .accessibilityValue(workspace.message)
                .nestedAccessibilityIdentifier("status")
            Text("Files stay where you put them. Git is managed outside BuildHunter. Enabled files are reloaded for each new scan.")
                .font(.caption).foregroundStyle(.secondary)
        }
        .padding()
        .a11yRoot("buildhunter.templates")
        .sheet(isPresented: $showingHelp) { TemplateWritingHelp() }
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

private struct TemplateWritingHelp: View {
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("Write a scan template").font(.title2.bold())
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Text("Templates are JSON files that describe what to find. They do not run commands or delete files.")
                    helpStep("1. Start with an example", "Choose New… in Templates. The example finds a forge-out directory beside a forge.project marker. You can also import an existing JSON file.")
                    helpStep("2. Describe your artifact", "Replace the example package and target titles. Keep the generated IDs unique, or choose your own stable IDs. Avoid the reserved swift, rust and python namespaces.")
                    helpStep("3. Set matching conditions", "Change forge-out to the exact file or folder name you want to find and forge.project to a marker in its parent folder. Set the node kind to directory or regular_file. The marker is checked by name; its contents are not read.")
                    helpStep("4. Check and save", "Choose Check and fix any diagnostics. Choose Save As… and select where to keep the JSON file. Enable the saved template in the list.")
                    helpStep("5. Try a scan", "Create a small test folder with a matching artifact and marker, plus a similar folder without the marker. Start a new scan or Rescan and check that only the intended artifact appears.")
                    Text("Changes apply to new scans. Connected files stay in their chosen folder, including a Git repository. A broken enabled template prevents the next scan until you fix or disable it.")
                        .foregroundStyle(.secondary)
                    Link("Template language reference ↗", destination: URL(string: "https://github.com/SoundBlaster/BuildHunter/blob/main/docs/template-language.md")!)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            HStack { Spacer(); Button("Done") { dismiss() }.keyboardShortcut(.defaultAction) }
        }
        .padding(24)
        .frame(minWidth: 480, idealWidth: 600, minHeight: 420, idealHeight: 600)
    }

    private func helpStep(_ title: LocalizedStringKey, _ description: LocalizedStringKey) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(.headline)
            Text(description)
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
