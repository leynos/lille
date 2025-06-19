#!/usr/bin/env bash
set -euo pipefail

# Run build_support_runner with environment variables normally set by Cargo.
# This allows executing the build pipeline without invoking `cargo build`.

# Create a temporary OUT_DIR and clean up on exit.
OUT_DIR="$(mktemp -d)"
trap 'rm -rf "$OUT_DIR"' EXIT

# Determine the manifest directory (repository root).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

export CARGO_MANIFEST_DIR="$MANIFEST_DIR"
export OUT_DIR="$OUT_DIR"
exec cargo run -p build_support --bin build_support_runner -- "$@"
