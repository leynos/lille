.PHONY: all clean build fmt check-fmt test test-observers-v1 lint build-support-run markdownlint nixie

.ONESHELL:
SHELL := bash

RUSTFLAGS_STRICT := -D warnings
RUST_FLAGS ?= $(RUSTFLAGS_STRICT)
RUST_FLAGS_ENV := RUSTFLAGS="$(RUST_FLAGS)"
WORKSPACE_PACKAGES := --package lille --package build_support --package test_utils
MARKDOWNLINT := $(shell which markdownlint-cli2)
MDTABLEFIX := $(shell which mdtablefix)
MD_FILES := $(shell git ls-files -co --exclude-standard '*.md')

all: lint test build

clean:
	cargo clean

build:
	$(RUST_FLAGS_ENV) cargo build

test:
	$(RUST_FLAGS_ENV) cargo test --features test-support

test-observers-v1:
	$(RUST_FLAGS_ENV) cargo test --features "test-support observers-v1-spike"

fmt:
	cargo fmt $(WORKSPACE_PACKAGES)
	if [[ -n "$(MD_FILES)" ]]; then \
	  if [[ -n "$(MDTABLEFIX)" ]]; then \
	    $(MDTABLEFIX) --wrap --renumber --breaks --ellipsis --fences --headings --in-place $(MD_FILES); \
	  fi; \
	  if [[ -n "$(MARKDOWNLINT)" ]]; then $(MARKDOWNLINT) --fix $(MD_FILES); fi; \
	fi

check-fmt:
	cargo fmt $(WORKSPACE_PACKAGES) -- --check

build-support-run:
	./scripts/build_support_runner.sh

RUSTDOC_FLAGS ?= --cfg docsrs -D warnings

lint:
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" cargo doc --workspace --no-deps
	cargo clippy --all-targets --all-features -- $(RUST_FLAGS)

markdownlint:
	if [[ -n "$(MD_FILES)" && -n "$(MARKDOWNLINT)" ]]; then $(MARKDOWNLINT) $(MD_FILES); fi

nixie:
	nixie --no-sandbox
