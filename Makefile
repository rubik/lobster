.PHONY: test
test:
	cargo test

.PHONY: f
f:
	rustfmt $(shell find src tests benches examples -name "*.rs" -type f)
