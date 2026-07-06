:#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo "==> Running checks"
cargo fmt --check
cargo clippy -- -D warnings
cargo test

echo "==> Building release binary"
cargo build --release

echo "==> Release binary: target/release/vicount"
