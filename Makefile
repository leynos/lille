.PHONY: all clean build fmt check-fmt test test-observers-v1 lint build-support-run \
	markdownlint nixie typecheck spelling

.ONESHELL:
SHELL := bash
# .ONESHELL feeds each recipe to one shell, so without -e only the last line's
# exit status reaches make and an earlier failure passes silently. pipefail
# covers the other half: a pipeline reports only its last command's status, so
# without it a recipe such as `git ls-files | xargs ...` succeeds when the git
# side fails. -c is make's own default and must be kept when SHELLFLAGS is
# overridden.
.SHELLFLAGS := -eo pipefail -c

RUSTFLAGS_STRICT := -D warnings
RUST_FLAGS ?= $(RUSTFLAGS_STRICT)
RUST_FLAGS_ENV := RUSTFLAGS="$(RUST_FLAGS)"
WHITAKER ?= whitaker
WORKSPACE_PACKAGES := --package lille --package build_support --package test_utils
MARKDOWNLINT := $(shell which markdownlint-cli2)
MD_FILES := $(shell git ls-files -co --exclude-standard '*.md')
UV ?= uv
UV_ENV = UV_CACHE_DIR=.uv-cache UV_TOOL_DIR=.uv-tools
TYPOS_CONFIG_BUILDER_VERSION ?= v0.1.1
TYPOS_CONFIG_BUILDER = $(UV_ENV) $(UV) tool run --from \
	"git+https://github.com/leynos/typos-config-builder.git@$(TYPOS_CONFIG_BUILDER_VERSION)" \
	typos-config-builder

all: lint test build spelling

clean:
	cargo clean

build:
	$(RUST_FLAGS_ENV) cargo build

test:
	$(RUST_FLAGS_ENV) cargo test --features test-support

test-observers-v1:
	$(RUST_FLAGS_ENV) cargo test --features "test-support observers-v1-spike"

typecheck:
	$(RUST_FLAGS_ENV) cargo check $(WORKSPACE_PACKAGES)

fmt:
	cargo fmt $(WORKSPACE_PACKAGES)
	mdformat-all

check-fmt:
	cargo fmt $(WORKSPACE_PACKAGES) -- --check

build-support-run:
	./scripts/build_support_runner.sh

RUSTDOC_FLAGS ?= --cfg docsrs -D warnings

lint:
	set -euo pipefail
	RUSTDOCFLAGS="$(RUSTDOC_FLAGS)" cargo doc --workspace --no-deps
	cargo clippy --all-targets --all-features -- $(RUST_FLAGS)
	$(RUST_FLAGS_ENV) $(WHITAKER) --all -- --all-targets --all-features

markdownlint: spelling
	if [[ -n "$(MD_FILES)" && -n "$(MARKDOWNLINT)" ]]; then $(MARKDOWNLINT) $(MD_FILES); fi

spelling:
	$(TYPOS_CONFIG_BUILDER) gate --repository .

nixie:
	nixie --no-sandbox
