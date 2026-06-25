TARGET := $(shell rustc -vV | sed -n 's/^host: //p')

.PHONY: test
test:
	cargo test

.PHONY: test-prebuilt
test-prebuilt:
	NODORA_BUILD_FROM_SOURCE=0 GO=/bin/false \
		CARGO_TARGET_DIR=$(CURDIR)/target/prebuilt-dl \
		cargo test

.PHONY: clean
clean:
	cargo clean
