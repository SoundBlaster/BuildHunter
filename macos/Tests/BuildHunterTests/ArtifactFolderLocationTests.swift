import Foundation
import Testing
@testable import BuildHunter

@Suite("Show in Finder location")
struct ArtifactFolderLocationTests {
    /// A scanned root with `Package/.build` inside, removed when the test ends.
    private func makeRoot() throws -> URL {
        let root = FileManager.default.temporaryDirectory
            .appendingPathComponent("BuildHunter Finder \(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: root.appendingPathComponent("Package/.build"),
                                                withIntermediateDirectories: true)
        return root
    }

    @Test("An existing folder is revealed itself")
    func existingFolder() throws {
        let root = try makeRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let build = root.appendingPathComponent("Package/.build", isDirectory: true)
        #expect(ArtifactFolderLocation.nearestExistingFolder(to: build, within: root)?.path
                == build.standardizedFileURL.path)
    }

    @Test("A deleted folder falls back to its nearest existing parent")
    func deletedFolder() throws {
        let root = try makeRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let gone = root.appendingPathComponent("Package/.build/debug/deps", isDirectory: true)
        #expect(ArtifactFolderLocation.nearestExistingFolder(to: gone, within: root)?.path
                == root.appendingPathComponent("Package/.build").standardizedFileURL.path)
    }

    @Test("The walk stops at the scanned folder instead of climbing past it")
    func boundedByRoot() throws {
        let root = try makeRoot()
        let gone = root.appendingPathComponent("Package/.build", isDirectory: true)
        try FileManager.default.removeItem(at: root)
        // The temporary directory above the root still exists, but it was never granted.
        #expect(ArtifactFolderLocation.nearestExistingFolder(to: gone, within: root) == nil)
        #expect(ArtifactFolderLocation.nearestExistingFolder(to: gone, within: nil) != nil)
    }

    @Test("A path outside the scanned folder is never revealed")
    func outsideRoot() throws {
        let root = try makeRoot()
        defer { try? FileManager.default.removeItem(at: root) }
        let sibling = root.deletingLastPathComponent()
        #expect(ArtifactFolderLocation.nearestExistingFolder(to: sibling, within: root) == nil)
        // An existing folder whose name merely starts with the root's name is a sibling too.
        let lookalike = URL(fileURLWithPath: root.path + "-other", isDirectory: true)
        try FileManager.default.createDirectory(at: lookalike, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: lookalike) }
        #expect(ArtifactFolderLocation.nearestExistingFolder(to: lookalike, within: root) == nil)
    }

    @Test("Only a fallback produces an alert")
    func notices() {
        #expect(FinderRevealOutcome.opened.notice == nil)
        let parent = URL(fileURLWithPath: "/tmp/Package", isDirectory: true)
        #expect(FinderRevealOutcome.openedParent(parent).notice?.contains("/tmp/Package") == true)
        #expect(FinderRevealOutcome.unavailable.notice != nil)
    }
}
