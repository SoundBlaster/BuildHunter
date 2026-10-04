import Foundation
import SpecificationCore
import XCTest
@testable import BuildHunter

/// Compare the actual adopted policy with its pre-static implementation in one process.
@MainActor
final class StaticPolicyPerformanceTests: XCTestCase {
    func testStaticPolicyPreservesResultsAndAvoidsDynamicDispatchRegression() throws {
        let legacy = LegacyDynamicArtifactPolicy()
        let candidate = ClassifyArtifactRoot()
        let names = [".build", "target", "build", "dist", "custom-env", ".tox", ".nox",
                     "__pycache__", "package.egg-info", ".pytest_cache", ".mypy_cache",
                     ".ruff_cache", ".pytype", "module.pyc", "module.pyo", ".git", "source.py"]
        let inputs = (0..<32_768).map { index in
            ArtifactPolicyContext(
                nodeName: names[index % names.count],
                isDirectory: index % 3 != 0,
                isSymbolicLink: index % 29 == 0,
                hasArtifactAncestor: index % 2 == 0,
                ownMarkerFacts: ArtifactMarkerFacts(rawValue: UInt32(index & 31)),
                parentMarkerFacts: ArtifactMarkerFacts(rawValue: UInt32((index / 32) & 31))
            )
        }
        // Independent old-policy comparison is outside timed regions.
        for input in inputs {
            XCTAssertEqual(candidate.decide(input), legacy.decide(input))
        }
        let expected = inputs.reduce(0) { $0 + (legacy.decide($1) == nil ? 0 : 1) } * 8
        func sample(_ usesStatic: Bool) -> (elapsed: Double, count: Int) {
            var count = 0
            let start = ProcessInfo.processInfo.systemUptime
            if usesStatic {
                for _ in 0..<8 {
                    for input in inputs { if candidate.decide(input) != nil { count += 1 } }
                }
            } else {
                for _ in 0..<8 {
                    for input in inputs { if legacy.decide(input) != nil { count += 1 } }
                }
            }
            return (ProcessInfo.processInfo.systemUptime - start, count)
        }
        XCTAssertEqual(sample(false).count, expected)
        XCTAssertEqual(sample(true).count, expected)
        var baseline: [Double] = []
        var current: [Double] = []
        for round in 0..<7 {
            for usesStatic in round.isMultiple(of: 2) ? [false, true] : [true, false] {
                let result = sample(usesStatic)
                XCTAssertEqual(result.count, expected)
                if usesStatic { current.append(result.elapsed) } else { baseline.append(result.elapsed) }
            }
        }
        func median(_ values: [Double]) -> Double { values.sorted()[values.count / 2] }
        let oldMedian = median(baseline)
        let newMedian = median(current)
        let oldMAD = median(baseline.map { abs($0 - oldMedian) })
        let newMAD = median(current.map { abs($0 - newMedian) })
        // Broad paired-workload gate; no exact speedup or whole-scan claim.
        let limit = oldMedian * 1.5 + 0.002 + 6 * (oldMAD + newMAD)
        let report: [String: Any] = [
            "fixture": "32768 mixed names/types/markers, 8 passes per sample",
            "baseline_seconds": baseline, "candidate_seconds": current,
            "baseline_median_seconds": oldMedian, "candidate_median_seconds": newMedian,
            "baseline_mad_seconds": oldMAD, "candidate_mad_seconds": newMAD,
            "gate_limit_seconds": limit, "classification_count": expected
        ]
        let data = try JSONSerialization.data(withJSONObject: report, options: [.sortedKeys, .prettyPrinted])
        let attachment = XCTAttachment(data: data, uniformTypeIdentifier: "public.json")
        attachment.name = "static-artifact-policy.json"
        attachment.lifetime = .keepAlways
        add(attachment)
        XCTAssertLessThanOrEqual(newMedian, limit,
                                 "Static policy must not regress beyond 1.5x baseline + 2 ms + 6 combined MAD.")
    }
}


private struct LegacyDynamicArtifactPolicy: DecisionSpec {
    typealias Context = ArtifactPolicyContext
    typealias Result = ArtifactClassification

    private let decision: FirstMatchSpec<Context, Result>
    private let scannableCandidate: IsScannableArtifactCandidate

    init() {
        scannableCandidate = IsScannableArtifactCandidate()
        let swiftBuild = PredicateSpec<Context>(description: "artifact.swift.build") {
            $0.isDirectory && $0.nodeName == ".build"
        }
        let rustBuild = PredicateSpec<Context>(description: "artifact.rust.target") {
            $0.isDirectory && $0.nodeName == "target" && $0.parentMarkerFacts.contains(.cargoManifest)
        }
        let pythonEnvironment = PredicateSpec<Context>(description: "artifact.python.environment") {
            $0.isDirectory && $0.ownMarkerFacts.contains(.pythonVirtualEnvironment)
        }
        let toxEnvironment = PredicateSpec<Context>(description: "artifact.python.test-environment") {
            $0.isDirectory && ($0.nodeName == ".tox" || $0.nodeName == ".nox")
        }
        let pythonBytecode = PredicateSpec<Context>(description: "artifact.python.bytecode-root") {
            $0.isDirectory && $0.nodeName == "__pycache__"
        }
        let pythonMetadata = PredicateSpec<Context>(description: "artifact.python.package-metadata") {
            $0.isDirectory && $0.nodeName.hasSuffix(".egg-info")
        }
        let pythonBuild = PredicateSpec<Context>(description: "artifact.python.build-output") {
            $0.isDirectory && ($0.nodeName == "build" || $0.nodeName == "dist")
                && !$0.parentMarkerFacts.intersection(.pythonBuildProject).isEmpty
        }
        let pythonCache = PredicateSpec<Context>(description: "artifact.python.tool-cache") {
            $0.isDirectory && ($0.nodeName == ".pytest_cache" || $0.nodeName == ".mypy_cache"
                || $0.nodeName == ".ruff_cache" || $0.nodeName == ".pytype")
        }
        let pythonBytecodeFile = PredicateSpec<Context>(description: "artifact.python.bytecode-file") {
            // Pure string work: URL(fileURLWithPath:) would stat a relative path per call.
            !$0.isDirectory && ($0.nodeName.hasSuffix(".pyc") || $0.nodeName.hasSuffix(".pyo"))
        }

        decision = FirstMatchSpec([
            (pythonEnvironment, ArtifactClassification(kind: .environment, language: "Python")),
            (toxEnvironment, ArtifactClassification(kind: .testEnvironment, language: "Python")),
            (swiftBuild, ArtifactClassification(kind: .buildOutput, language: "Swift")),
            (rustBuild, ArtifactClassification(kind: .buildOutput, language: "Rust")),
            (pythonBuild, ArtifactClassification(kind: .buildOutput, language: "Python")),
            (pythonBytecode, ArtifactClassification(kind: .cache, language: "Python")),
            (pythonMetadata, ArtifactClassification(kind: .cache, language: "Python")),
            (pythonCache, ArtifactClassification(kind: .cache, language: "Python")),
            (pythonBytecodeFile, ArtifactClassification(kind: .cache, language: "Python"))
        ])
    }

    func decide(_ context: Context) -> Result? {
        guard scannableCandidate.isSatisfiedBy(context) else { return nil }
        return decision.decide(context)
    }
}
