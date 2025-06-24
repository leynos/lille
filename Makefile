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

fmt:
	cargo fmt --package lille --package build_support --package test_utils
	mdformat-all

fmt-check: generated/lille_ddlog/lib.rs.stub
	cargo fmt --package lille --package build_support --package test_utils -- --check

generated:
	mkdir -p generated

build-support-run: generated
	./scripts/build_support_runner.sh

# Create a stub lib.rs file for formatting and dependency resolution
generated/lille_ddlog/lib.rs.stub: generated
	mkdir -p generated/lille_ddlog
	echo '[package]' > generated/lille_ddlog/Cargo.toml
	echo 'name = "lille-ddlog"' >> generated/lille_ddlog/Cargo.toml
	echo 'version = "0.1.0"' >> generated/lille_ddlog/Cargo.toml
	echo 'edition = "2018"' >> generated/lille_ddlog/Cargo.toml
	echo '' >> generated/lille_ddlog/Cargo.toml
	echo '[lib]' >> generated/lille_ddlog/Cargo.toml
	echo 'path = "lib.rs"' >> generated/lille_ddlog/Cargo.toml
	echo '//! Stub file for lille-ddlog crate.' > generated/lille_ddlog/lib.rs
	echo '//! This file is replaced during the build process with generated DDlog code.' >> generated/lille_ddlog/lib.rs
	echo '//! It exists to satisfy Cargo'\''s dependency resolution during formatting and other operations.' >> generated/lille_ddlog/lib.rs
	echo '' >> generated/lille_ddlog/lib.rs
	echo '#![allow(dead_code)]' >> generated/lille_ddlog/lib.rs
	echo '' >> generated/lille_ddlog/lib.rs
	echo '// Minimal stub to make this a valid Rust library' >> generated/lille_ddlog/lib.rs
	> generated/lille_ddlog/lib.rs.stub

generated/lille_ddlog/lib.rs: build-support-run
	# Apply patches to fix static linking issues in generated DDlog code
	patch -N -p1 -d generated/lille_ddlog < patches/fix_static.patch
	# Rename the generated crate from "lille" to "lille-ddlog" to avoid conflicts
	sed -i 's/^name = "lille"/name = "lille-ddlog"/' generated/lille_ddlog/Cargo.toml
	# Remove workspace configuration from generated Cargo.toml (DDlog generates this incorrectly)
	sed -i '/^\[workspace\]/,$$d' generated/lille_ddlog/Cargo.toml
	# Suppress all clippy warnings on generated ddlog code (not worth fixing generated code)
	sed -i '1i#![allow(clippy::all)]' generated/lille_ddlog/ddlog_profiler/src/lib.rs
	sed -i '1i#![allow(clippy::all)]' generated/lille_ddlog/ddlog_derive/src/lib.rs

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
