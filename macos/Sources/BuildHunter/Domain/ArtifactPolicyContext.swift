import Foundation

/// Marker bits shared with the Rust scanner's stable `BHCandidateFacts` ABI.
struct ArtifactMarkerFacts: OptionSet, Sendable {
    let rawValue: UInt32

    init(rawValue: UInt32) {
        self.rawValue = rawValue
    }

    static let cargoManifest = Self(rawValue: 1 << 0)
    static let pythonProject = Self(rawValue: 1 << 1)
    static let pythonSetupScript = Self(rawValue: 1 << 2)
    static let pythonSetupConfig = Self(rawValue: 1 << 3)
    static let pythonVirtualEnvironment = Self(rawValue: 1 << 4)

    static let pythonBuildProject: Self = [pythonProject, pythonSetupScript, pythonSetupConfig]

    init(markerFiles: Set<String>) {
        var facts: Self = []
        if markerFiles.contains("Cargo.toml") { facts.insert(.cargoManifest) }
        if markerFiles.contains("pyproject.toml") { facts.insert(.pythonProject) }
        if markerFiles.contains("setup.py") { facts.insert(.pythonSetupScript) }
        if markerFiles.contains("setup.cfg") { facts.insert(.pythonSetupConfig) }
        if markerFiles.contains("pyvenv.cfg") { facts.insert(.pythonVirtualEnvironment) }
        self = facts
    }

    var markerFiles: Set<String> {
        var names = Set<String>()
        if contains(.cargoManifest) { names.insert("Cargo.toml") }
        if contains(.pythonProject) { names.insert("pyproject.toml") }
        if contains(.pythonSetupScript) { names.insert("setup.py") }
        if contains(.pythonSetupConfig) { names.insert("setup.cfg") }
        if contains(.pythonVirtualEnvironment) { names.insert("pyvenv.cfg") }
        return names
    }
}

struct ArtifactPolicyContext: Sendable {
    let nodeName: String
    let isDirectory: Bool
    let isSymbolicLink: Bool
    let hasArtifactAncestor: Bool
    let ownMarkerFacts: ArtifactMarkerFacts
    let parentMarkerFacts: ArtifactMarkerFacts

    /// Compatibility view for tests and non-hot-path callers using marker names.
    var ownMarkerFiles: Set<String> { ownMarkerFacts.markerFiles }
    var parentMarkerFiles: Set<String> { parentMarkerFacts.markerFiles }

    init(nodeName: String, isDirectory: Bool, isSymbolicLink: Bool,
         hasArtifactAncestor: Bool = false, ownMarkerFiles: Set<String>, parentMarkerFiles: Set<String>) {
        self.nodeName = nodeName
        self.isDirectory = isDirectory
        self.isSymbolicLink = isSymbolicLink
        self.hasArtifactAncestor = hasArtifactAncestor
        self.ownMarkerFacts = ArtifactMarkerFacts(markerFiles: ownMarkerFiles)
        self.parentMarkerFacts = ArtifactMarkerFacts(markerFiles: parentMarkerFiles)
    }

    init(nodeName: String, isDirectory: Bool, isSymbolicLink: Bool,
         hasArtifactAncestor: Bool = false,
         ownMarkerFacts: ArtifactMarkerFacts, parentMarkerFacts: ArtifactMarkerFacts) {
        self.nodeName = nodeName
        self.isDirectory = isDirectory
        self.isSymbolicLink = isSymbolicLink
        self.hasArtifactAncestor = hasArtifactAncestor
        self.ownMarkerFacts = ownMarkerFacts
        self.parentMarkerFacts = parentMarkerFacts
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
