import Foundation

@MainActor
protocol ScanEventSource: Sendable {
    func events(for generation: UInt64) -> AsyncStream<ScanEvent>
}

/// Emits fabricated fixture data. It never inspects the selected folder.
struct DemoScanSource: ScanEventSource {
    func events(for generation: UInt64) -> AsyncStream<ScanEvent> {
        AsyncStream { continuation in
            let policy = ClassifyArtifactRoot()
            let fixtures: [(String, String, Int64, ArtifactPolicyContext)] = [
                ("B0000000-0000-4000-8000-000000000001", "DemoFixture/App/.build", 1_572_864,
                 ArtifactPolicyContext(nodeName: ".build", isDirectory: true, isSymbolicLink: false,
                                       ownMarkerFiles: [], parentMarkerFiles: [])),
                ("B0000000-0000-4000-8000-000000000002", "DemoFixture/Service/target", 8_388_608,
                 ArtifactPolicyContext(nodeName: "target", isDirectory: true, isSymbolicLink: false,
                                       ownMarkerFiles: [], parentMarkerFiles: ["Cargo.toml"])),
                ("B0000000-0000-4000-8000-000000000003", "DemoFixture/tools/.venv", 4_194_304,
                 ArtifactPolicyContext(nodeName: ".venv", isDirectory: true, isSymbolicLink: false,
                                       ownMarkerFiles: ["pyvenv.cfg"], parentMarkerFiles: []))
            ]
            let artifacts = fixtures.compactMap { id, path, bytes, facts -> DemoArtifact? in
                guard let classification = policy.decide(facts),
                      IsOuterArtifactRoot().isSatisfiedBy(ArtifactRootContext(hasArtifactAncestor: false)),
                      let uuid = UUID(uuidString: id) else { return nil }
                return DemoArtifact(id: uuid, relativePath: path, simulatedBytes: bytes,
                                    language: classification.language, kind: classification.kind)
            }
            let producer = Task {
                for artifact in artifacts {
                    guard !Task.isCancelled else { break }
                    continuation.yield(.discovered(generation: generation, artifact: artifact))
                    await Task.yield()
                    continuation.yield(.completed(generation: generation, artifactID: artifact.id, simulatedBytes: artifact.simulatedBytes))
                    await Task.yield()
                }
                if Task.isCancelled {
                    continuation.yield(.finished(generation: generation, result: .stopped))
                } else {
                    continuation.yield(.finished(generation: generation, result: .completed))
                }
                continuation.finish()
            }
            continuation.onTermination = { @Sendable _ in producer.cancel() }
        }
    }
}
