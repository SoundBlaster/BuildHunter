import Foundation
import Darwin
import CryptoKit

struct TemplateProblem: Error, LocalizedError, Codable, Equatable, Sendable {
    let code: String
    let location: String
    let message: String
    var templateIndex: Int? = nil

    var errorDescription: String? {
        "\(message) [\(code)\(location.isEmpty ? "" : ": \(location)")]"
    }
}

struct TemplateReference: Codable, Equatable, Identifiable, Sendable {
    let id: UUID
    var path: String
    var bookmark: Data
    var enabled: Bool

    static func connect(_ url: URL, id: UUID = UUID()) throws -> Self {
        guard url.isFileURL else { throw TemplateProblem(code: "invalid_file", location: "", message: "Choose a local JSON file.") }
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        return Self(id: id, path: url.standardizedFileURL.path,
            bookmark: try url.bookmarkData(options: .withSecurityScope, includingResourceValuesForKeys: nil, relativeTo: nil),
            enabled: true)
    }
}

struct TemplateDocumentInfo: Decodable, Sendable {
    let id: String
    let version: String
    let title: String
}

private final class TemplateErrorReceiver {
    var problem: TemplateProblem?
}

private func receiveTemplateError(_ context: UnsafeMutableRawPointer?, _ bytes: UnsafePointer<UInt8>?, _ count: Int) {
    guard let context, let bytes, (1...65_536).contains(count) else { return }
    // Rust invokes this callback synchronously, on the caller's thread only.
    let receiver = Unmanaged<TemplateErrorReceiver>.fromOpaque(context).takeUnretainedValue()
    receiver.problem = try? JSONDecoder().decode(TemplateProblem.self, from: Data(bytes: bytes, count: count))
}


/// Edition resources are immutable application inputs, never user registrations.
enum BundledTemplateCatalog {
    private struct Manifest: Decodable {
        struct Entry: Decodable { let file: String; let sha256: String }
        let format: String
        let packages: [Entry]
    }

    static func documents(bundle: Bundle = .main) throws -> [Data] {
        guard let setting = bundle.object(forInfoDictionaryKey: "BuildHunterBundledCatalog") else { return [] }
        guard let name = setting as? String, let resources = bundle.resourceURL else {
            throw problem("Invalid bundled catalog setting.")
        }
        return try documents(resources: resources, manifestPath: name)
    }

    static func documents(resources: URL, manifestPath: String) throws -> [Data] {
        let parts = manifestPath.split(separator: "/", omittingEmptySubsequences: false)
        guard parts.count == 2, parts.allSatisfy({ safeName(String($0)) }) else {
            throw problem("The bundled catalog must name a resource folder and manifest.")
        }
        let root = resources.resolvingSymlinksInPath().standardizedFileURL
        let directory = root.appending(path: String(parts[0]), directoryHint: .isDirectory)
        let manifestURL = directory.appending(path: String(parts[1]))
        do {
            try checkLocation(manifestURL, root: root)
            let bytes = try TemplateFiles.readURL(manifestURL)
            guard bytes.count <= 65_536 else { throw problem("Bundled manifest exceeds 64 KiB.") }
            let manifest = try JSONDecoder().decode(Manifest.self, from: bytes)
            guard manifest.format == "buildhunter.bundled-catalog/1",
                  (1...TemplateCatalog.maximumTemplates).contains(manifest.packages.count),
                  Set(manifest.packages.map(\.file)).count == manifest.packages.count else {
                throw problem("Unsupported or invalid bundled catalog manifest.")
            }
            var result: [Data] = []
            var total = 0
            for entry in manifest.packages {
                guard safeName(entry.file) else { throw problem("Invalid bundled package filename.") }
                let url = directory.appending(path: entry.file)
                try checkLocation(url, root: root)
                let data = try TemplateFiles.readURL(url)
                total += data.count
                guard total <= TemplateCatalog.maximumCatalogBytes else { throw problem("Bundled catalog exceeds its byte budget.") }
                let hash = SHA256.hash(data: data).map { String(format: "%02x", $0) }.joined()
                guard hash == entry.sha256 else { throw problem("Bundled package integrity check failed: \(entry.file)") }
                result.append(data)
            }
            return result
        } catch {
            throw problem("Cannot load bundled catalog: \(error.localizedDescription)")
        }
    }

    private static func safeName(_ name: String) -> Bool {
        !name.isEmpty && name != "." && name != ".." && name.utf8.allSatisfy {
            (48...57).contains($0) || (65...90).contains($0) || (97...122).contains($0) || [45, 46, 95].contains($0)
        }
    }
    private static func checkLocation(_ url: URL, root: URL) throws {
        let values = try url.resourceValues(forKeys: [.isSymbolicLinkKey])
        guard values.isSymbolicLink != true,
              url.resolvingSymlinksInPath().path.hasPrefix(root.path + "/") else {
            throw problem("Bundled resources must remain inside the application resources.")
        }
    }
    private static func problem(_ message: String) -> TemplateProblem {
        TemplateProblem(code: "bundled_catalog_invalid", location: "BuildHunterBundledCatalog", message: message)
    }
}

enum TemplateCatalog {
    static let maximumFileBytes = 1_048_576
    static let maximumTemplates = 61
    static let maximumCatalogBytes = 16 * 1_048_576

    /// The immutable Rust handle owns its parsed data; all NSData backing lives through the call.
    static func create(documents userDocuments: [Data], excluded: Set<String> = [],
                       bundledDocuments: [Data]? = nil) throws -> UnsafeMutableRawPointer {
        let bundled = try bundledDocuments ?? BundledTemplateCatalog.documents()
        let documents = bundled + userDocuments
        guard documents.count <= maximumTemplates,
              documents.allSatisfy({ $0.count <= maximumFileBytes }),
              documents.reduce(0, { $0 + $1.count }) <= maximumCatalogBytes else {
            throw TemplateProblem(code: "limit_exceeded", location: "", message: "Template byte or package limit exceeded.")
        }
        let exclusions = try JSONEncoder().encode(excluded.sorted())
        let backing = documents.map { $0 as NSData }
        let buffers = backing.map { BHPluginDocument(bytes: $0.bytes.assumingMemoryBound(to: UInt8.self), len: $0.length) }
        let receiver = TemplateErrorReceiver()
        let handle = withExtendedLifetime(backing) {
            exclusions.withUnsafeBytes { exclusions in
                buffers.withUnsafeBufferPointer { buffers in
                    bh_catalog_create_with_templates_v1(2, exclusions.baseAddress?.assumingMemoryBound(to: UInt8.self),
                        exclusions.count, buffers.baseAddress, buffers.count, receiveTemplateError,
                        Unmanaged.passUnretained(receiver).toOpaque())
                }
            }
        }
        guard let handle else {
            var problem = receiver.problem ?? TemplateProblem(code: "catalog_failed", location: "", message: "Could not prepare the template catalog.")
            if let index = problem.templateIndex {
                if index < bundled.count {
                    problem = TemplateProblem(code: "bundled_catalog_invalid", location: "BuildHunterBundledCatalog", message: problem.message)
                } else {
                    problem.templateIndex = index - bundled.count
                }
            }
            throw problem
        }
        return handle
    }

    static func metadata(documents: [Data], excluded: Set<String> = [], bundledDocuments: [Data]? = nil) throws -> ScanCatalogMetadata {
        let handle = try create(documents: documents, excluded: excluded, bundledDocuments: bundledDocuments)
        defer { bh_catalog_destroy(handle) }
        guard let pointer = bh_catalog_json(handle),
              let data = String(cString: pointer).data(using: .utf8),
              let metadata = try? JSONDecoder().decode(ScanCatalogMetadata.self, from: data), metadata.isSupported else {
            throw TemplateProblem(code: "catalog_failed", location: "", message: "Invalid catalog metadata.")
        }
        return metadata
    }

    static func example() -> String {
        guard let pointer = bh_template_example_json() else { return "{}" }
        let namespace = "local.template." + UUID().uuidString.lowercased()
        return String(cString: pointer).replacingOccurrences(of: "example.forge", with: namespace)
    }
}

enum TemplateFiles {
    static func withAccess<T>(_ reference: TemplateReference, _ body: (URL) throws -> T) throws -> T {
        var stale = false
        let url = try URL(resolvingBookmarkData: reference.bookmark,
            options: [.withSecurityScope, .withoutUI, .withoutMounting], relativeTo: nil, bookmarkDataIsStale: &stale)
        guard url.standardizedFileURL.path == reference.path else {
            throw TemplateProblem(code: "file_moved", location: reference.path, message: "The template moved. Reconnect its file.")
        }
        let scoped = url.startAccessingSecurityScopedResource()
        defer { if scoped { url.stopAccessingSecurityScopedResource() } }
        return try body(url)
    }

    static func read(_ reference: TemplateReference) throws -> Data {
        do { return try withAccess(reference) { try readURL($0) } }
        catch let problem as TemplateProblem { throw problem }
        catch { throw TemplateProblem(code: "file_unavailable", location: reference.path, message: error.localizedDescription) }
    }

    static func renewed(_ reference: TemplateReference) throws -> TemplateReference {
        try withAccess(reference) { url in
            var replacement = try TemplateReference.connect(url, id: reference.id)
            replacement.enabled = reference.enabled
            return replacement
        }
    }

    /// O_NONBLOCK avoids blocking on a FIFO; fstat rejects devices/directories before reading.
    static func readURL(_ url: URL) throws -> Data {
        let descriptor = url.withUnsafeFileSystemRepresentation { path in
            path.map { Darwin.open($0, O_RDONLY | O_NONBLOCK | O_CLOEXEC) } ?? -1
        }
        guard descriptor >= 0 else { throw ioError(url) }
        defer { Darwin.close(descriptor) }
        var before = stat()
        guard fstat(descriptor, &before) == 0 else { throw ioError(url) }
        guard before.st_mode & S_IFMT == S_IFREG else {
            throw TemplateProblem(code: "invalid_file", location: url.path, message: "A template must be a regular JSON file.")
        }
        guard before.st_size <= TemplateCatalog.maximumFileBytes else { throw sizeError(url) }
        var data = Data()
        var buffer = [UInt8](repeating: 0, count: 16_384)
        while true {
            let count = buffer.withUnsafeMutableBytes { Darwin.read(descriptor, $0.baseAddress, $0.count) }
            if count < 0 && errno == EINTR { continue }
            guard count >= 0 else { throw ioError(url) }
            if count == 0 { break }
            guard data.count + count <= TemplateCatalog.maximumFileBytes else { throw sizeError(url) }
            data.append(contentsOf: buffer.prefix(count))
        }
        var after = stat()
        guard fstat(descriptor, &after) == 0 else { throw ioError(url) }
        guard before.st_size == after.st_size,
              before.st_mtimespec.tv_sec == after.st_mtimespec.tv_sec,
              before.st_mtimespec.tv_nsec == after.st_mtimespec.tv_nsec else {
            throw TemplateProblem(code: "file_changed", location: url.path, message: "The template changed while being read. Reload it.")
        }
        return data
    }

    static func documents(_ references: [TemplateReference]) throws -> [Data] {
        let enabled = references.filter(\.enabled)
        guard enabled.count <= TemplateCatalog.maximumTemplates else {
            throw TemplateProblem(code: "limit_exceeded", location: "", message: "Too many enabled templates.")
        }
        var documents: [Data] = []
        var total = 0
        for reference in enabled {
            let bytes = try read(reference)
            total += bytes.count
            guard total <= TemplateCatalog.maximumCatalogBytes else { throw sizeError(URL(filePath: reference.path)) }
            documents.append(bytes)
        }
        return documents
    }

    static func createCatalog(_ references: [TemplateReference], excluded: Set<String>) throws -> UnsafeMutableRawPointer {
        let enabled = references.filter(\.enabled)
        do { return try TemplateCatalog.create(documents: documents(enabled), excluded: excluded) }
        catch var problem as TemplateProblem {
            if let index = problem.templateIndex, enabled.indices.contains(index) {
                problem = TemplateProblem(code: problem.code, location: enabled[index].path + problem.location,
                    message: problem.message, templateIndex: index)
            }
            throw problem
        }
    }

    static func save(_ reference: TemplateReference, bytes: Data, expected: Data) throws {
        _ = try TemplateCatalog.metadata(documents: [bytes])
        try withAccess(reference) { url in
            var coordinationError: NSError?
            var writeError: Error?
            NSFileCoordinator().coordinate(writingItemAt: url, options: .forReplacing, error: &coordinationError) { coordinatedURL in
                do {
                    guard try readURL(coordinatedURL) == expected else {
                        throw TemplateProblem(code: "file_changed", location: url.path,
                            message: "The file changed outside BuildHunter. Reload before saving.")
                    }
                    try bytes.write(to: coordinatedURL, options: .atomic)
                } catch { writeError = error }
            }
            if let coordinationError { throw coordinationError }
            if let writeError { throw writeError }
        }
    }

    private static func sizeError(_ url: URL) -> TemplateProblem {
        TemplateProblem(code: "limit_exceeded", location: url.path, message: "Template byte limit exceeded (1 MiB per file, 16 MiB per catalog).")
    }
    private static func ioError(_ url: URL) -> TemplateProblem {
        TemplateProblem(code: "file_unavailable", location: url.path, message: String(cString: strerror(errno)))
    }
}
