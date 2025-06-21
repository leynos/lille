.PHONY: build test fmt build-support-run lint markdownlint nixie

build:
	. ./.env
	export DDLOG_HOME
	RUSTFLAGS="-D warnings" cargo build

test:
	. ./.env
	export DDLOG_HOME
	RUSTFLAGS="-D warnings" cargo test

fmt:
	cargo fmt --all
	mdformat-all

build-support-run:
	. ./.env
	export DDLOG_HOME
	./scripts/build_support_runner.sh

lint:
	. ./.env
	export DDLOG_HOME
	cargo clippy --all-targets --all-features -- -D warnings

markdownlint:
	markdownlint *.md **/*.md

nixie:
	nixie *.md **/*.md
