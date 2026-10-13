#if SCREENER_SCREENSHOTS
import SpecificationCore

/// Fixture state is ready once product work and presentation transitions finish.
struct IsScreenshotReady: Specification {
    struct Context {
        let isScanning: Bool
        let isTransitioning: Bool
        let expectedArtifacts: Int
        let displayedArtifacts: Int
    }
    private let rule = PredicateSpec<Context>(description: "screenshots.presentation.ready") {
        !$0.isScanning && !$0.isTransitioning && $0.expectedArtifacts == $0.displayedArtifacts
    }
    func isSatisfiedBy(_ context: Context) -> Bool { rule.isSatisfiedBy(context) }
}
#endif
