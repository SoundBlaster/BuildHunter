#if SCREENER_SCREENSHOTS
import SpecificationCore

/// The capture profile owns appearance independently of the host's system theme.
struct IsDarkScreenshotAppearance: Specification {
    private let rule = PredicateSpec<String>(description: "screenshots.appearance.dark") { $0 == "dark" }
    func isSatisfiedBy(_ context: String) -> Bool { rule.isSatisfiedBy(context) }
}
#endif
