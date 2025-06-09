#!/usr/bin/env bash
set -euo pipefail

# Install DDlog from the v1.2.3 release archive into ~/.local/ddlog.
# After installation, environment variables required for DDlog
# are written to ~/.ddlog_env in a form suitable for sourcing.

ARCHIVE_URL="https://github.com/vmware-archive/differential-datalog/releases/download/v1.2.3/ddlog-v1.2.3-20211213235218-Linux.tar.gz"
INSTALL_DIR="$HOME/.local/ddlog"
ENV_FILE="$HOME/.ddlog_env"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

curl -L "$ARCHIVE_URL" -o "$TMP_DIR/ddlog.tgz"

rm -rf "$INSTALL_DIR"
mkdir -p "$INSTALL_DIR"
tar -xzf "$TMP_DIR/ddlog.tgz" -C "$TMP_DIR"
mv "$TMP_DIR/ddlog" "$INSTALL_DIR"

cat > "$ENV_FILE" <<EOV
export DDLOG_HOME="$INSTALL_DIR"
export PATH="${INSTALL_DIR}/bin:\$PATH"
EOV

echo "DDlog installed to $INSTALL_DIR"
echo "Source $ENV_FILE to update your environment"

