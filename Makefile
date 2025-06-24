.PHONY: all clean build fmt fmt-check test lint \
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

fmt: generated/lille_ddlog/lib.rs
	cargo fmt --package lille --package build_support --package test_utils
	mdformat-all

fmt-check: generated/lille_ddlog/lib.rs
	cargo fmt --package lille --package build_support --package test_utils -- --check

generated:
	mkdir -p generated

build-support-run: generated
	./scripts/build_support_runner.sh

generated/lille_ddlog/lib.rs: build-support-run
	patch -N -p1 -d generated/lille_ddlog < patches/fix_static.patch

targets/ddlog/debug/lille: generated/lille_ddlog/lib.rs
	RUSTFLAGS="-D warnings" cargo build --features ddlog --target-dir targets/ddlog

test-ddlog: generated/lille_ddlog/lib.rs
	RUSTFLAGS="-D warnings" cargo test --features ddlog --target-dir targets/ddlog

lint:
	cargo clippy --all-targets --all-features -- -D warnings

markdownlint:
	find . -name '*.md' -print0 | xargs -0 markdownlint

nixie:
	find . -name '*.md' -print0 | xargs -0 nixie

# Generate, patch, and compile the DDlog inferencer
build-inferencer: generated/lille_ddlog/lib.rs patches/fix_static.patch
	RUSTFLAGS="-D warnings" cargo build --manifest-path generated/ddlog_lille/lille_ddlog/Cargo.toml --target-dir targets/ddlog
