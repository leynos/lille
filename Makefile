.PHONY: all clean build fmt check-fmt test lint build-support-run markdownlint nixie

.ONESHELL:
SHELL := bash

RUSTFLAGS_STRICT := -D warnings
RUST_FLAGS ?= $(RUSTFLAGS_STRICT)
RUST_FLAGS_ENV := RUSTFLAGS="$(RUST_FLAGS)"
WORKSPACE_PACKAGES := --package lille --package build_support --package test_utils
MARKDOWNLINT := $(shell which markdownlint-cli2)

all: lint test build

clean:
	cargo clean

build:
	$(RUST_FLAGS_ENV) cargo build

test:
	$(RUST_FLAGS_ENV) cargo test --features test-support

fmt:
	cargo fmt $(WORKSPACE_PACKAGES)
	mdformat-all

check-fmt:
	cargo fmt $(WORKSPACE_PACKAGES) -- --check

build-support-run:
	./scripts/build_support_runner.sh

RUSTDOC_FLAGS ?= --cfg docsrs -D warnings

lint:
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" cargo doc --workspace --no-deps
	cargo clippy --all-targets --all-features -- $(RUST_FLAGS)

markdownlint:
	$(MARKDOWNLINT) "**/*.md"

nixie:
	nixie --no-sandbox
