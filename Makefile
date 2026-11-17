.PHONY: apple-build apple-test apple-ui-test apple-lab-test apple-ci-test apple-long-test
apple-build:
	bash scripts/agent/apple.sh build
apple-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh test
apple-ui-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh ui-test
apple-lab-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh lab-test
apple-ci-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh ci-test
apple-long-test:
	DESTINATION='$(DESTINATION)' bash scripts/agent/apple.sh long-test
