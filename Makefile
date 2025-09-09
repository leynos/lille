.PHONY: all clean build fmt check-fmt test lint build-support-run markdownlint nixie

.ONESHELL:
SHELL := bash

RUSTFLAGS_STRICT := RUSTFLAGS="-D warnings"
WORKSPACE_PACKAGES := --package lille --package build_support --package test_utils

all: lint test build

clean:
	cargo clean

build:
	$(RUSTFLAGS_STRICT) cargo build

test:
	$(RUSTFLAGS_STRICT) cargo test

fmt:
	cargo fmt $(WORKSPACE_PACKAGES)
	mdformat-all

check-fmt:
	cargo fmt $(WORKSPACE_PACKAGES) -- --check

build-support-run:
	./scripts/build_support_runner.sh

lint:
	cargo clippy --all-targets --all-features -- -D warnings

markdownlint:
	find . -name '*.md' -print0 | xargs -0 markdownlint

nixie:
	find . -name '*.md' -print0 | xargs -0 nixie
