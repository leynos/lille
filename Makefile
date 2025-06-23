.PHONY: all clean build test fmt build-support-run targets/ddlog/debug/lille test-ddlog lint markdownlint nixie

.ONESHELL:
SHELL := bash

all: build

clean:
	cargo clean

build:
	RUSTFLAGS="-D warnings" cargo build

test:
	RUSTFLAGS="-D warnings" cargo test

fmt:
	@if [ ! -f generated/ddlog_lille/lib.rs ]; then \
	mkdir -p generated/ddlog_lille; \
	echo "// Placeholder for generated ddlog crate" > generated/ddlog_lille/lib.rs; \
	fi
	cargo fmt --all
	mdformat-all

build-support-run:
	./scripts/build_support_runner.sh -- --ddlog-dir generated/ddlog_lille

generated/ddlog_lille/lib.rs: build-support-run

targets/ddlog/debug/lille: generated/ddlog_lille/lib.rs
	RUSTFLAGS="-D warnings" cargo build --features ddlog --target-dir targets/ddlog

test-ddlog: generated/ddlog_lille/lib.rs
	RUSTFLAGS="-D warnings" cargo test --features ddlog --target-dir targets/ddlog

lint:
	cargo clippy --all-targets --all-features -- -D warnings

markdownlint:
	find . -name '*.md' -print0 | xargs -0 markdownlint

nixie:
	find . -name '*.md' -print0 | xargs -0 nixie
