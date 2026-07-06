:#!/usr/bin/env bash
set -euo pipefail

REPO="scotia/vicount"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

say() {
    printf 'vicount-install: %s\n' "$1"
}

err() {
    printf 'vicount-install: %s\n' "$1" >&2
    exit 1
}

command -v cargo >/dev/null 2>&1 || err "cargo is required but not installed"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

say "Cloning ${REPO}..."
git clone --depth 1 "https://github.com/${REPO}.git" "$TMP_DIR/vicount"

cd "$TMP_DIR/vicount"

say "Building release binary..."
cargo build --release

mkdir -p "$INSTALL_DIR"
cp target/release/vicount "$INSTALL_DIR/vicount"
chmod +x "$INSTALL_DIR/vicount"

say "Installed vicount to ${INSTALL_DIR}/vicount"

if [[ ":$PATH:" != *":${INSTALL_DIR}:"* ]]; then
    say "Warning: ${INSTALL_DIR} is not on your PATH."
fi
