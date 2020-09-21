.PHONY: f
f:
	rustfmt $(shell find src -name "*.rs" -type f)
