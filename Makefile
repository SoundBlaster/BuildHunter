SPECPM_REGISTRY ?= http://127.0.0.1:8081

.PHONY: specpm-validate
specpm-validate:
	@python3 scripts/check_specpm.py "$(SPECPM_REGISTRY)"
