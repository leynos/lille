.PHONY: all clean build fmt test lint \
        build-support-run \
        build-ddlog test-ddlog build-inferencer \
        markdownlint nixie

.ONESHELL:
SHELL := bash

all: build

clean:
	cargo clean

build:
	RUSTFLAGS="-D warnings" cargo build

build-ddlog: targets/ddlog/debug/lille

test:
	RUSTFLAGS="-D warnings" cargo test

fmt:
	cargo fmt --all
	mdformat-all

build-support-run:
	./scripts/build_support_runner.sh

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

# Generate, patch, and compile the DDlog inferencer
build-inferencer: generated/ddlog_lille/lib.rs generated/ddlog_lille/patches/fix_static.patch
	patch -N -p1 -d generated/ddlog_lille/lille_ddlog < generated/ddlog_lille/patches/fix_static.patch
	RUSTFLAGS="-D warnings" cargo build --manifest-path generated/ddlog_lille/lille_ddlog/Cargo.toml --target-dir targets/ddlog
