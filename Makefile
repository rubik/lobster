.PHONY: test
test:
	cargo test

.PHONY: f
f:
	rustfmt $(shell find src benches examples -name "*.rs" -type f)

.PHONY: bench
bench:
	cargo bench
