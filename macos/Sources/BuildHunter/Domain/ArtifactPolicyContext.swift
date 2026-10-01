import Foundation

struct ArtifactPolicyContext: Sendable {
    let nodeName: String
    let isDirectory: Bool
    let isSymbolicLink: Bool
    let ownMarkerFiles: Set<String>
    let parentMarkerFiles: Set<String>
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
