import Foundation

/// Where "Show in Finder" ended up for a scanned folder.
enum FinderRevealOutcome: Equatable {
    case opened
    case openedParent(URL)
    case unavailable

    /// The alert text when Finder could not select the requested folder itself.
    var notice: String? {
        switch self {
        case .opened:
            nil
        case .openedParent(let parent):
            "The folder is no longer available. Opened \(parent.path) in Finder instead."
        case .unavailable:
            "The folder and its parents within the scanned folder no longer exist."
        }
    }
}

enum ArtifactFolderLocation {
    /// A displayed replacement character never becomes an action capability.
    /// Safe ancestors before the first invalid component remain usable.
    static func actionURL(root: URL?, relativePath: String, unsafePrefixes: Set<String>) -> URL? {
        guard let root, root.isFileURL, !relativePath.hasPrefix("/"),
              !relativePath.split(separator: "/").contains("..") else { return nil }
        let path = relativePath == "." ? "" : relativePath
        guard !unsafePrefixes.contains(where: { $0.isEmpty || path == $0 || path.hasPrefix($0 + "/") }) else { return nil }
        return path.isEmpty ? root : root.appendingPathComponent(path, isDirectory: true)
    }

    /// The folder itself if it still exists as a directory, otherwise its nearest existing
    /// ancestor, but never above `root`, the folder the user chose to scan. A sandboxed build
    /// can only see inside that folder, so climbing past it could report a misleading
    /// location, or the volume root. Without a root the walk may reach `/`.
    static func nearestExistingFolder(to url: URL, within root: URL?) -> URL? {
        let requested = url.standardizedFileURL
        let boundary = root?.standardizedFileURL.path
        var candidate = requested
        while true {
            if let boundary, !isPath(candidate.path, inside: boundary) { return nil }
            if isDirectory(candidate) { return candidate }
            if let boundary, candidate.path == boundary { return nil }
            let parent = candidate.deletingLastPathComponent()
            guard parent.path != candidate.path else { return nil }
            candidate = parent
        }
    }

    private static func isPath(_ path: String, inside root: String) -> Bool {
        path == root || path.hasPrefix(root.hasSuffix("/") ? root : root + "/")
    }

    private static func isDirectory(_ url: URL) -> Bool {
        var isDirectory: ObjCBool = false
        return FileManager.default.fileExists(atPath: url.path, isDirectory: &isDirectory) && isDirectory.boolValue
    }
}
