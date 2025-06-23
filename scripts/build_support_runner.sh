#!/usr/bin/env bash
set -euo pipefail

usage() {
    cat <<EOF
Usage: ${0##*/} [--tmp|--tmp-keep] [-- <build_support args>]

Run build_support_runner with environment variables normally set by Cargo.

Options:
  --tmp        Use a temporary output directory removed on exit.
  --tmp-keep   Use a temporary output directory retained after running.
  --           Pass all following arguments to build_support.
  -h, --help   Show this help message and exit.
EOF
}

# Run build_support_runner with environment variables normally set by Cargo.
# This allows executing the build pipeline without invoking `cargo build`.

# Determine the manifest directory (repository root).
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MANIFEST_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

# Default OUT_DIR writes into the src directory like build.rs.
OUT_DIR="$MANIFEST_DIR/generated"
BUILD_ARGS=()
CLEAN_MODE=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --tmp)
            if [[ -n "$CLEAN_MODE" ]]; then
                echo "Error: --tmp and --tmp-keep are mutually exclusive" >&2
                exit 1
            fi
            OUT_DIR="$(mktemp -d)"
            CLEAN_MODE="remove"
            shift
            ;;
        --tmp-keep)
            if [[ -n "$CLEAN_MODE" ]]; then
                echo "Error: --tmp and --tmp-keep are mutually exclusive" >&2
                exit 1
            fi
            OUT_DIR="$(mktemp -d)"
            CLEAN_MODE="keep"
            shift
            ;;
        -h|--help)
            usage
            exit 0
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

if [[ ! -d "$OUT_DIR" ]]; then
    echo "Error: OUT_DIR '$OUT_DIR' does not exist" >&2
    exit 1
fi

export CARGO_MANIFEST_DIR="$MANIFEST_DIR"
export OUT_DIR="$OUT_DIR"

if [[ "$CLEAN_MODE" == "remove" ]]; then
    trap 'rm -rf -- "$OUT_DIR"' EXIT INT TERM
elif [[ "$CLEAN_MODE" == "keep" ]]; then
    trap 'echo "Temporary output retained at: $OUT_DIR"' EXIT INT TERM
fi

# Compile the helper binary if needed and run it directly so Cargo does not
# override `CARGO_MANIFEST_DIR`.
cargo build -p build_support --bin build_support_runner
exec "$MANIFEST_DIR/target/debug/build_support_runner" "${BUILD_ARGS[@]}"
