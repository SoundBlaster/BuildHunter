import Darwin
import Foundation
import Testing
@testable import BuildHunter

@Suite("File-backed user templates")
@MainActor
struct UserTemplateTests {
    @Test("Templates extend the real catalog and package versions survive report export")
    func catalogAndExport() throws {
        let document = Data(TemplateCatalog.example().utf8)
        let metadata = try TemplateCatalog.metadata(documents: [document])
        #expect(metadata.targets.count == 14)
        #expect(metadata.packages?.count == 4)
        let package = try JSONDecoder().decode(TemplateDocumentInfo.self, from: document)
        #expect(metadata.packages?.contains { $0.id == package.id && $0.version == package.version } == true)
        #expect(metadata.digest != ScanCatalogMetadata.basic()?.digest)
        let excluded = try TemplateCatalog.metadata(documents: [document], excluded: [metadata.targets.last!.id])
        #expect(metadata.digest == excluded.digest)
        let report = ScanReportExport(rootName: "Fixture", rootURL: nil, status: .complete,
            rows: [], order: .table, tableOrder: [], warnings: [], excludedFilterIDs: [], elapsedSeconds: nil,
            appVersion: "test", catalogMetadata: metadata, diagnostics: [])
        let object = try #require(JSONSerialization.jsonObject(with: report.data(as: .json)) as? [String: Any])
        let catalog = try #require(object["catalog"] as? [String: Any])
        #expect((catalog["packages"] as? [[String: String]])?.count == 4)
        let settings = SearchFilterSettings(catalog: [], userDefaults: UserDefaults(suiteName: UUID().uuidString)!)
        settings.useCatalog(metadata)
        #expect(settings.groups.contains("Example"))
    }

    @Test("Invalid input is diagnosed without partial activation")
    func malformedPackages() throws {
        let valid = Data(TemplateCatalog.example().utf8)
        let invalid: [Data] = [Data(), Data("{".utf8), Data([0xff]), Data("{\"id\":1,\"id\":2}".utf8),
            Data(repeating: 91, count: 100), Data(repeating: 32, count: TemplateCatalog.maximumFileBytes + 1),
            Data(String(decoding: valid, as: UTF8.self).replacingOccurrences(of: "1-draft.1", with: "999").utf8),
            Data(String(decoding: valid, as: UTF8.self).replacingOccurrences(of: "\"eq\"", with: "\"execute\"").utf8)]
        for data in invalid {
            let problem = captureProblem { _ = try TemplateCatalog.metadata(documents: [valid, data]) }
            #expect(problem != nil)
            #expect(problem?.message.isEmpty == false)
        }
        let duplicate = captureProblem { _ = try TemplateCatalog.metadata(documents: [valid, valid]) }
        #expect(duplicate?.code == "duplicate_id")
    }

    @Test("Bounded reads reject nonregular, oversized, missing and moved files")
    func fileAdmission() throws {
        let directory = try fixture()
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appending(path: "template.json")
        let valid = Data(TemplateCatalog.example().utf8)
        try valid.write(to: url)
        let reference = try TemplateReference.connect(url)
        #expect(try TemplateFiles.read(reference) == valid)
        let oversized = directory.appending(path: "oversized.json")
        try Data(repeating: 32, count: TemplateCatalog.maximumFileBytes + 1).write(to: oversized)
        #expect(captureProblem { _ = try TemplateFiles.readURL(oversized) }?.code == "limit_exceeded")
        #expect(captureProblem { _ = try TemplateFiles.readURL(directory) }?.code == "invalid_file")
        let fifo = directory.appending(path: "pipe.json")
        #expect(fifo.withUnsafeFileSystemRepresentation { Darwin.mkfifo($0!, 0o600) } == 0)
        #expect(captureProblem { _ = try TemplateFiles.readURL(fifo) }?.code == "invalid_file")
        try FileManager.default.moveItem(at: url, to: directory.appending(path: "moved.json"))
        #expect(captureProblem { _ = try TemplateFiles.read(reference) } != nil)
        #expect(captureProblem { _ = try TemplateFiles.readURL(url) }?.code == "file_unavailable")
    }

    @Test("Save refuses observed external changes and never replaces them with a draft")
    func coordinatedSave() throws {
        let directory = try fixture()
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appending(path: "template.json")
        let original = Data(TemplateCatalog.example().utf8)
        try original.write(to: url)
        let reference = try TemplateReference.connect(url)
        let draft = Data(String(decoding: original, as: UTF8.self).replacingOccurrences(of: "1.0.0", with: "1.0.1").utf8)
        try TemplateFiles.save(reference, bytes: draft, expected: original)
        #expect(try TemplateFiles.read(reference) == draft, "Atomic replacement must remain readable through the registration")
        let external = Data(String(decoding: draft, as: UTF8.self).replacingOccurrences(of: "1.0.1", with: "1.0.2").utf8)
        try external.write(to: url, options: .atomic)
        #expect(captureProblem { try TemplateFiles.save(reference, bytes: original, expected: draft) }?.code == "file_changed")
        #expect(try Data(contentsOf: url) == external)
        #expect(captureProblem { try TemplateFiles.save(reference, bytes: Data("{".utf8), expected: external) } != nil)
        #expect(try Data(contentsOf: url) == external)
    }

    @Test("Registry persists references and disabled state, disconnect never deletes the source")
    func registry() throws {
        let directory = try fixture()
        defer { try? FileManager.default.removeItem(at: directory) }
        let suite = UUID().uuidString
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let file = directory.appending(path: "template.json")
        try Data("{".utf8).write(to: file)
        let reference = try TemplateReference.connect(file)
        let library = TemplateLibrary(userDefaults: defaults)
        try library.connect(reference)
        library.setEnabled(false, id: reference.id)
        try library.connect(TemplateReference.connect(file))
        let restored = TemplateLibrary(userDefaults: defaults)
        #expect(restored.references.count == 1 && restored.references.first?.id == reference.id)
        #expect(restored.references.first?.enabled == false)
        #expect(try TemplateFiles.documents(restored.references).isEmpty)
        restored.disconnect(id: reference.id)
        #expect(FileManager.default.fileExists(atPath: file.path))
        defaults.set(Data("broken".utf8), forKey: TemplateLibrary.persistenceKey)
        let broken = TemplateLibrary(userDefaults: defaults)
        #expect(broken.storageProblem != nil)
        #expect(captureProblem { try broken.connect(reference) }?.code == "invalid_registry")
        broken.resetInvalidRegistrations()
        #expect(broken.storageProblem == nil && broken.references.isEmpty)
    }

    @Test("Bad edits stay in the draft and failed reload never discards them")
    func workspaceDraft() async throws {
        let directory = try fixture()
        defer { try? FileManager.default.removeItem(at: directory) }
        let suite = UUID().uuidString
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let workspace = TemplateWorkspace(library: TemplateLibrary(userDefaults: defaults))
        workspace.newDocument()
        await workspace.check()
        #expect(!workspace.isError && workspace.isDirty)
        let file = directory.appending(path: "draft.json")
        await workspace.saveAs(file)
        #expect(!workspace.isError && !workspace.isDirty)
        let reference = try #require(workspace.reference)
        workspace.text = "{"
        await workspace.save()
        #expect(workspace.isError && workspace.isDirty && workspace.text == "{")
        try FileManager.default.removeItem(at: file)
        await workspace.load(reference)
        #expect(workspace.isError && workspace.text == "{")
    }

    @Test("A real custom scan freezes its catalog; broken next scan fails until explicitly disabled")
    func scanIsolation() async throws {
        let directory = try fixture()
        defer { try? FileManager.default.removeItem(at: directory) }
        let scanRoot = directory.appending(path: "scan", directoryHint: .isDirectory)
        let output = scanRoot.appending(path: "forge-out", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        try Data().write(to: scanRoot.appending(path: "forge.project"))
        try Data(repeating: 1, count: 4096).write(to: output.appending(path: "object"))
        let file = directory.appending(path: "template.json")
        try Data(TemplateCatalog.example().utf8).write(to: file)
        let suite = UUID().uuidString
        let defaults = try #require(UserDefaults(suiteName: suite))
        defer { defaults.removePersistentDomain(forName: suite) }
        let library = TemplateLibrary(userDefaults: defaults)
        let reference = try TemplateReference.connect(file)
        try library.connect(reference)
        let source = RustScanEventSource(settings: SearchFilterSettings(userDefaults: defaults), templates: library)
        var paths: [String] = []
        var metadata: ScanCatalogMetadata?
        var terminal: ScanTerminalResult?
        for await event in source.events(for: 1, target: scanRoot) {
            switch event {
            case .catalog(_, let value, _):
                metadata = value
                try Data("{".utf8).write(to: file, options: .atomic)
            case .discovered(_, let artifact): paths.append(artifact.relativePath)
            case .finished(_, let result): terminal = result
            default: break
            }
        }
        #expect(terminal == .completed && paths == ["forge-out"] && metadata?.packages?.count == 4)
        var warnings: [String] = []
        for await event in source.events(for: 2, target: scanRoot) {
            switch event {
            case .catalog, .discovered: Issue.record("Invalid template must not activate a partial catalog")
            case .warning(_, let message): warnings.append(message)
            case .finished(_, let result): terminal = result
            default: break
            }
        }
        #expect(warnings.contains { $0.contains(file.path) && $0.contains("Settings") })
        if case .failed = terminal {} else { Issue.record("Invalid enabled file must fail the scan") }
        library.setEnabled(false, id: reference.id)
        for await event in source.events(for: 3, target: scanRoot) {
            if case .finished(_, let result) = event { terminal = result }
        }
        #expect(terminal == .completed)
    }

    private func captureProblem(_ operation: () throws -> Void) -> TemplateProblem? {
        do { try operation(); Issue.record("Expected a template diagnostic"); return nil }
        catch let problem as TemplateProblem { return problem }
        catch { Issue.record("Unexpected error: \(error)"); return nil }
    }

    private func fixture() throws -> URL {
        let url = FileManager.default.temporaryDirectory.appending(path: "UserTemplates-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
        return url
    }
}
