#!/usr/bin/env bash
set -euo pipefail

# Run build_support_runner with environment variables normally set by Cargo.
# This allows executing the build pipeline without invoking `cargo build`.

# Determine the manifest directory (repository root).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default OUT_DIR writes into the src directory like build.rs.
OUT_DIR="$MANIFEST_DIR/src"
BUILD_ARGS=()
CLEAN_MODE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --tmp)
            OUT_DIR="$(mktemp -d)"
            CLEAN_MODE="remove"
            shift
            ;;
        --tmp-keep)
            OUT_DIR="$(mktemp -d)"
            CLEAN_MODE="keep"
            shift
            ;;
        --)
            shift
            BUILD_ARGS+=("$@")
            break
            ;;
        *)
            BUILD_ARGS+=("$1")
            shift
            ;;
    esac
done

export CARGO_MANIFEST_DIR="$MANIFEST_DIR"
export OUT_DIR="$OUT_DIR"

if [[ "$CLEAN_MODE" == "remove" ]]; then
    trap 'rm -rf "$OUT_DIR"' EXIT
elif [[ "$CLEAN_MODE" == "keep" ]]; then
    trap 'echo "Temporary output retained at: $OUT_DIR"' EXIT
fi

# Compile the helper binary if needed and run it directly so Cargo does not
# override `CARGO_MANIFEST_DIR`.
cargo build -p build_support --bin build_support_runner
exec "$MANIFEST_DIR/target/debug/build_support_runner" "${BUILD_ARGS[@]}"
