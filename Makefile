SPECPM_REGISTRY ?= http://127.0.0.1:8081

.PHONY: specpm-validate
specpm-validate:
	@specpm validate . --registry "$(SPECPM_REGISTRY)" --json
