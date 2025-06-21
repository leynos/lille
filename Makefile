.PHONY: all clean build test fmt build-support-run lint markdownlint nixie

.ONESHELL:
SHELL := bash
ENV := . ./.env && export DDLOG_HOME

all: build

clean:
	cargo clean

build:
	$(ENV)
	RUSTFLAGS="-D warnings" cargo build

test:
	$(ENV)
	RUSTFLAGS="-D warnings" cargo test

fmt:
	cargo fmt --all
	mdformat-all

build-support-run:
	$(ENV)
	./scripts/build_support_runner.sh

lint:
	$(ENV)
	cargo clippy --all-targets --all-features -- -D warnings

markdownlint:
	find . -name '*.md' -print0 | xargs -0 markdownlint

nixie:
	find . -name '*.md' -print0 | xargs -0 nixie
