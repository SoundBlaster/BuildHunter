import SpecificationCore

struct IsOuterArtifactRoot: Specification {
    private let rule = PredicateSpec<ArtifactRootContext>(description: "artifact.root.outer") {
        !$0.hasArtifactAncestor
    }

    func isSatisfiedBy(_ context: ArtifactRootContext) -> Bool {
        rule.isSatisfiedBy(context)
    }
}
