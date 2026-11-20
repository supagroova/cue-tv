.PHONY: help doctor check staged-check harness-test apple-build apple-test apple-ui-test apple-lab-test apple-ci-test apple-long-test hooks
help:
	@echo "doctor | check (portable harness) | staged-check | harness-test | apple-build | apple-test DESTINATION=... | apple-ui-test DESTINATION=... | apple-ci-test DESTINATION=... | hooks"
doctor:
	python3 scripts/agent/doctor.py
check:
	python3 scripts/agent/check.py
	$(MAKE) harness-test
staged-check:
	python3 scripts/agent/check.py --staged
harness-test:
	python3 -m unittest discover -s tests/agent -v
apple-build:
	bash scripts/agent/apple.sh build
apple-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh test
apple-ui-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh ui-test
hooks:
	bash scripts/agent/install-hooks.sh
apple-lab-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh lab-test
apple-ci-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh ci-test
apple-long-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh long-test
