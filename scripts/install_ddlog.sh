#!/usr/bin/env bash
set -euo pipefail

# Cleanup temporary directory on exit if it was created
trap 'if [ -n "${TMP_DIR:-}" ]; then rm -rf "$TMP_DIR"; fi' EXIT

# Install DDlog from the v1.2.3 release archive into ~/.local/ddlog.
# After installation, environment variables required for DDlog
# are written to a `.env` file in this repository so that the
# build script can load them with `dotenvy`.

ARCHIVE_URL="https://github.com/vmware-archive/differential-datalog/releases/download/v1.2.3/ddlog-v1.2.3-20211213235218-Linux.tar.gz"
INSTALL_DIR="$HOME/.local/ddlog"
#ENV_FILE="$HOME/.ddlog_env"
ENV_FILE=".env"

# --- Preflight checks -------------------------------------------------------
for tool in curl tar mktemp; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "Error: required tool '$tool' not found in PATH" >&2
        exit 1
    fi
done

case "$(uname -s)" in
    Linux) ;;
    *) echo "Error: this installer only supports Linux" >&2; exit 1 ;;
esac

TMP_DIR="$(mktemp -d)"

echo "Downloading DDlog archive..."
curl --fail -L "$ARCHIVE_URL" -o "$TMP_DIR/ddlog.tgz"

mkdir -p "${INSTALL_DIR%/*}"
rm -rf "$INSTALL_DIR"

echo "Extracting..."
tar -xzf "$TMP_DIR/ddlog.tgz" -C "$TMP_DIR"

# Determine the extracted directory name (e.g. ddlog-v1.2.3-...) and move it
EXTRACTED_DIR=$(find "$TMP_DIR" -maxdepth 1 -mindepth 1 -type d -name 'ddlog*' | head -n 1)
if [ -z "$EXTRACTED_DIR" ]; then
    echo "Error: failed to locate extracted ddlog directory" >&2
    exit 1
fi
mv "$EXTRACTED_DIR" "$INSTALL_DIR"

# Backup existing environment file if present
if [ -f "$ENV_FILE" ]; then
    BACKUP="${ENV_FILE}.bak"
    echo "Backing up existing $ENV_FILE to $BACKUP"
    cp "$ENV_FILE" "$BACKUP"
fi

cat > "$ENV_FILE" <<EOV
DDLOG_HOME=$INSTALL_DIR
PATH=$INSTALL_DIR/bin:$PATH
EOV

echo "DDlog installed to $INSTALL_DIR"
echo "Dotenv file created at $ENV_FILE"

