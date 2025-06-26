.PHONY: all clean build fmt fmt-check test lint \
            build-support-run ddlog-stubs \
            build-ddlog test-ddlog build-inferencer \
            markdownlint nixie

.ONESHELL:
SHELL := bash

# Variables to reduce repetition
RUSTFLAGS_STRICT := RUSTFLAGS="-D warnings"
WORKSPACE_PACKAGES := --package lille --package build_support --package test_utils
DDLOG_TARGET_DIR := targets/ddlog
GENERATED_SRC := generated/lille_ddlog

# Portable sed in-place editing (GNU sed vs BSD/macOS sed compatibility)
UNAME_S := $(shell uname -s 2>/dev/null || echo "Unknown")
ifeq ($(UNAME_S),Darwin)
	SED_INPLACE := sed -i ''
else
	SED_INPLACE := sed -i
endif

all: lint test build

clean:
	cargo clean

build: ddlog-stubs
	$(RUSTFLAGS_STRICT) cargo build

build-ddlog: $(DDLOG_TARGET_DIR)/debug/lille

build-inferencer: $(DDLOG_TARGET_DIR)/debug/liblille_ddlog.rlib

test: ddlog-stubs
	$(RUSTFLAGS_STRICT) cargo test

fmt: ddlog-stubs
	cargo fmt $(WORKSPACE_PACKAGES)
	mdformat-all

fmt-check: ddlog-stubs
	cargo fmt $(WORKSPACE_PACKAGES) -- --check

$(GENERATED_SRC):
	mkdir -p '$(GENERATED_SRC)'

build-support-run: ddlog-stubs
	./scripts/build_support_runner.sh

# Copy prebuilt DDlog stubs into the generated directory
ddlog-stubs: $(GENERATED_SRC)
	cp stubs/lille_ddlog/Cargo.toml '$(GENERATED_SRC)/Cargo.toml'
	cp stubs/lille_ddlog/lib.rs '$(GENERATED_SRC)/src/lib.rs'
	mkdir -p '$(GENERATED_SRC)/differential_datalog/src'
	cp stubs/lille_ddlog/differential_datalog/Cargo.toml '$(GENERATED_SRC)/differential_datalog/Cargo.toml'
	cp stubs/lille_ddlog/differential_datalog/lib.rs '$(GENERATED_SRC)/differential_datalog/src/lib.rs'


$(GENERATED_SRC)/src/lib.rs: build-support-run patches/fix_static.patch
	# Apply patches to fix static linking issues in generated DDlog code
	patch -N -p1 -d '$(GENERATED_SRC)' < patches/fix_static.patch
	# Rename the generated crate from "lille" to "lille-ddlog" to avoid conflicts
	$(SED_INPLACE) 's/^name = "lille"/name = "lille-ddlog"/' '$(GENERATED_SRC)/Cargo.toml'
	# Remove workspace configuration from generated Cargo.toml (DDlog generates this incorrectly)
	$(SED_INPLACE) '/^\[workspace\]/,$$d' '$(GENERATED_SRC)/Cargo.toml'
	# Suppress all warnings and clippy on generated ddlog code (not worth fixing generated code)
	find '$(GENERATED_SRC)' -name "*.rs" -type f -print0 | \
	           while IFS= read -r -d $$'\0' file; do \
	                   if ! head -2 "$$file" | grep -q "^#!\[allow(clippy::all)\]"; then \
                               $(SED_INPLACE) '1s;^;#![allow(warnings)]\n#![allow(clippy::all)]\n;' "$$file"; \
	                   fi; \
	           done

$(DDLOG_TARGET_DIR)/debug/lille: $(DDLOG_TARGET_DIR)/debug/liblille_ddlog.rlib
	$(RUSTFLAGS_STRICT) cargo build --features ddlog --target-dir '$(DDLOG_TARGET_DIR)'

test-ddlog: $(DDLOG_TARGET_DIR)/debug/liblille_ddlog.rlib
	$(RUSTFLAGS_STRICT) cargo test --features ddlog --target-dir '$(DDLOG_TARGET_DIR)'
	
lint: ddlog-stubs
	cargo clippy --all-targets --all-features -- -D warnings

markdownlint:
	find . -name '*.md' -print0 | xargs -0 markdownlint

nixie:
	find . -name '*.md' -print0 | xargs -0 nixie

# Generate, patch, and compile the DDlog inferencer
$(DDLOG_TARGET_DIR)/debug/liblille_ddlog.rlib: $(GENERATED_SRC)/src/lib.rs
	mkdir -p '$(DDLOG_TARGET_DIR)'
	$(RUSTFLAGS_STRICT) cargo build --manifest-path generated/lille_ddlog/Cargo.toml --target-dir '$(DDLOG_TARGET_DIR)'
