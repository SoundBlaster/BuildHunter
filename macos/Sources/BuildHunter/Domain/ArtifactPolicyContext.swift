import Foundation

struct ArtifactPolicyContext: Sendable {
    let nodeName: String
    let isDirectory: Bool
    let isSymbolicLink: Bool
    let hasArtifactAncestor: Bool
    let ownMarkerFiles: Set<String>
    let parentMarkerFiles: Set<String>

    init(nodeName: String, isDirectory: Bool, isSymbolicLink: Bool,
         hasArtifactAncestor: Bool = false, ownMarkerFiles: Set<String>, parentMarkerFiles: Set<String>) {
        self.nodeName = nodeName
        self.isDirectory = isDirectory
        self.isSymbolicLink = isSymbolicLink
        self.hasArtifactAncestor = hasArtifactAncestor
        self.ownMarkerFiles = ownMarkerFiles
        self.parentMarkerFiles = parentMarkerFiles
    }
}

struct ArtifactRootContext: Sendable {
    let hasArtifactAncestor: Bool
}

enum ArtifactKind: String, CaseIterable, Sendable {
    case buildOutput = "Build output"
    case cache = "Cache"
    case environment = "Environment"
    case testEnvironment = "Test environment"
}

struct ArtifactClassification: Equatable, Sendable {
    let kind: ArtifactKind
    let language: String
}
