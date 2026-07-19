.PHONY: all clean build fmt check-fmt test test-observers-v1 lint build-support-run \
	markdownlint nixie typecheck spelling spelling-helper-test

.ONESHELL:
SHELL := bash

RUSTFLAGS_STRICT := -D warnings
RUST_FLAGS ?= $(RUSTFLAGS_STRICT)
RUST_FLAGS_ENV := RUSTFLAGS="$(RUST_FLAGS)"
WHITAKER ?= whitaker
WORKSPACE_PACKAGES := --package lille --package build_support --package test_utils
MARKDOWNLINT := $(shell which markdownlint-cli2)
MDTABLEFIX := $(shell which mdtablefix)
MD_FILES := $(shell git ls-files -co --exclude-standard '*.md')
UV ?= uv
UV_ENV = UV_CACHE_DIR=.uv-cache UV_TOOL_DIR=.uv-tools
RUFF_VERSION ?= 0.15.12
TYPOS_VERSION ?= 1.48.0

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
	$(RUST_FLAGS_ENV) $(WHITAKER) --all -- --all-targets --all-features

markdownlint: spelling
	if [[ -n "$(MD_FILES)" && -n "$(MARKDOWNLINT)" ]]; then $(MARKDOWNLINT) $(MD_FILES); fi

spelling: spelling-helper-test
	@$(UV_ENV) $(UV) run scripts/generate_typos_config.py
	@git ls-files -z '*.md' | \
		xargs -0 -r env $(UV_ENV) $(UV) tool run typos@$(TYPOS_VERSION) \
		--config typos.toml --force-exclude

spelling-helper-test:
	@$(UV_ENV) $(UV) tool run ruff@$(RUFF_VERSION) format --isolated \
		--target-version py313 --check scripts/generate_typos_config.py \
		scripts/typos_rollout.py scripts/typos_rollout_cache.py \
		scripts/tests/test_typos_rollout.py
	@$(UV_ENV) $(UV) tool run ruff@$(RUFF_VERSION) check --isolated \
		--target-version py313 scripts/generate_typos_config.py \
		scripts/typos_rollout.py scripts/typos_rollout_cache.py \
		scripts/tests/test_typos_rollout.py
	@PYTHONPATH=scripts $(UV_ENV) $(UV) run --no-project --python 3.13 \
		--with pytest==9.0.2 --with pytest-cov==7.0.0 \
		python -m pytest scripts/tests/test_typos_rollout.py \
		-c /dev/null --rootdir=. -p no:cacheprovider \
		--cov=generate_typos_config --cov=typos_rollout \
		--cov=typos_rollout_cache --cov-fail-under=90

nixie:
	nixie --no-sandbox
