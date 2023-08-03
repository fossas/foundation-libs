
test:
	@cargo nextest run
	@cargo test --doc

.PHONY: test
