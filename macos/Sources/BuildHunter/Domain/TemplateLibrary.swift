import Foundation
import Observation

@MainActor
@Observable
final class TemplateLibrary {
    static let shared = TemplateLibrary(userDefaults: SearchFilterSettings.defaultUserDefaults)
    static let persistenceKey = "userTemplateFiles.v1"
    private(set) var references: [TemplateReference] = []
    private(set) var storageProblem: String?
    @ObservationIgnored private let defaults: UserDefaults

    init(userDefaults: UserDefaults) {
        defaults = userDefaults
        if let data = defaults.data(forKey: Self.persistenceKey) {
            do {
                guard data.count <= 8 * 1_048_576 else {
                    throw TemplateProblem(code: "invalid_registry", location: "", message: "Template registrations exceed their byte limit.")
                }
                let references = try JSONDecoder().decode([TemplateReference].self, from: data)
                guard references.count <= TemplateCatalog.maximumTemplates,
                      Set(references.map(\.id)).count == references.count,
                      Set(references.map(\.path)).count == references.count,
                      references.allSatisfy({ !$0.bookmark.isEmpty && $0.bookmark.count <= 65_536 && $0.path.hasPrefix("/") }) else {
                    throw TemplateProblem(code: "invalid_registry", location: "", message: "Invalid template registrations.")
                }
                self.references = references
            } catch { storageProblem = error.localizedDescription }
        }
    }

    func connect(_ reference: TemplateReference) throws {
        try validateConnection(path: reference.path, replacingID: reference.id)
        guard !reference.bookmark.isEmpty, reference.bookmark.count <= 65_536, reference.path.hasPrefix("/") else {
            throw TemplateProblem(code: "invalid_registry", location: reference.path, message: "Invalid template file reference.")
        }
        if let index = references.firstIndex(where: { $0.id == reference.id || $0.path == reference.path }) {
            let replacement = TemplateReference(id: references[index].id, path: reference.path,
                bookmark: reference.bookmark, enabled: references[index].enabled)
            references[index] = replacement
        } else {
            references.append(reference)
        }
        persist()
    }

    func validateConnection(path: String, replacingID: UUID? = nil) throws {
        guard storageProblem == nil else {
            throw TemplateProblem(code: "invalid_registry", location: "", message: "Reset the invalid template registrations before adding files.")
        }
        if let replacingID, references.contains(where: { $0.id == replacingID }),
           references.contains(where: { $0.path == path && $0.id != replacingID }) {
            throw TemplateProblem(code: "file_already_connected", location: path,
                message: "This destination belongs to another connected template. Choose another file.")
        }
        guard references.contains(where: { $0.path == path || $0.id == replacingID })
                || references.count < TemplateCatalog.maximumTemplates else {
            throw TemplateProblem(code: "limit_exceeded", location: "", message: "At most 61 template files may be connected.")
        }
    }

    func setEnabled(_ enabled: Bool, id: UUID) {
        guard let index = references.firstIndex(where: { $0.id == id }) else { return }
        references[index].enabled = enabled
        persist()
    }

    func disconnect(id: UUID) {
        references.removeAll { $0.id == id }
        persist()
    }

    func resetInvalidRegistrations() {
        references = []
        storageProblem = nil
        persist()
    }

    private func persist() {
        do { defaults.set(try JSONEncoder().encode(references), forKey: Self.persistenceKey) }
        catch { storageProblem = error.localizedDescription }
    }
}
