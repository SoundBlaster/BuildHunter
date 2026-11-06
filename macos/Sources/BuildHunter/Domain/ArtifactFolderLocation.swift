import Foundation
import Darwin

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
        guard let root, root.isFileURL, !relativePath.hasPrefix("/"), !relativePath.utf8.contains(0),
              !relativePath.split(separator: "/").contains("..") else { return nil }
        let path = relativePath == "." ? "" : relativePath
        guard !unsafePrefixes.contains(where: { $0.isEmpty || path == $0 || path.hasPrefix($0 + "/") }) else { return nil }
        // Do not append a trailing slash: it can force a symlink to be resolved
        // as a directory before Finder sees the entry itself.
        return path.isEmpty ? root : root.appendingPathComponent(path, isDirectory: false)
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
            // Inspect the requested entry itself: even a dangling link can be
            // selected in Finder without resolving or opening its destination.
            if candidate.path == requested.path && isSymbolicLink(candidate) { return candidate }
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

    private static func isSymbolicLink(_ url: URL) -> Bool {
        var metadata = stat()
        return url.path.withCString { path in
            lstat(path, &metadata) == 0 && (metadata.st_mode & mode_t(S_IFMT)) == mode_t(S_IFLNK)
        }
    }
}
