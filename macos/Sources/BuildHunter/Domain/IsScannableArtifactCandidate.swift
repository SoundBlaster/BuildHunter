import SpecificationCore

struct IsScannableArtifactCandidate: Specification {
    private let rule = PredicateSpec<ArtifactPolicyContext>(description: "artifact.candidate.scannable") {
        !$0.isSymbolicLink && !($0.isDirectory && $0.nodeName == ".git")
    }

    func isSatisfiedBy(_ context: ArtifactPolicyContext) -> Bool {
        rule.isSatisfiedBy(context)
    }
}
