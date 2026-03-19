#!/bin/bash
# Copy sidecar binaries into the Tauri binaries directory with target triple suffix.
# Usage: ./copy-sidecars.sh [--build]
#   --build: also build the binaries before copying (default: copy only)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../../../.." && pwd)"
TARGET_DIR="$PROJECT_ROOT/target/release"
BINARIES_DIR="$SCRIPT_DIR"

# Detect target triple
ARCH=$(uname -m)
OS=$(uname -s)
case "$OS" in
  Darwin)
    case "$ARCH" in
      arm64) TRIPLE="aarch64-apple-darwin" ;;
      x86_64) TRIPLE="x86_64-apple-darwin" ;;
    esac
    ;;
  Linux)
    TRIPLE="x86_64-unknown-linux-gnu"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    TRIPLE="x86_64-pc-windows-msvc"
    ;;
esac

if [ -z "${TRIPLE:-}" ]; then
  echo "ERROR: Unsupported platform: $OS $ARCH" >&2
  exit 1
fi

echo "Platform: $TRIPLE"

# Optionally build first
if [ "${1:-}" = "--build" ]; then
  echo "Building sidecar binaries..."
  cargo build --release -p nexus-cli -p nexus-mcp-server --manifest-path "$PROJECT_ROOT/Cargo.toml"
fi

# Copy with target triple suffix
for BIN in nexus nexus-mcp-server; do
  SRC="$TARGET_DIR/$BIN"
  DST="$BINARIES_DIR/${BIN}-${TRIPLE}"
  if [ -f "$SRC" ]; then
    cp "$SRC" "$DST"
    chmod +x "$DST"
    echo "Copied: $BIN -> $(basename "$DST")"
  else
    echo "WARNING: $SRC not found. Run with --build or build manually first." >&2
  fi
done

echo "Done."
