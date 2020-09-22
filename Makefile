.PHONY: test
test:
	cargo test

.PHONY: f
f:
	rustfmt $(shell find src -name "*.rs" -type f) $(shell find tests -name "*.rs")
