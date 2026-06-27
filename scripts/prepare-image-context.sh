#!/usr/bin/env bash
# Populate build/image/ for `docker build -f Dockerfile build/image`.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/identity"
if [[ ! -f "$BIN" && -f "$ROOT/../target/release/identity" ]]; then
  BIN="$ROOT/../target/release/identity"
fi
if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build --release" >&2
  exit 1
fi
mkdir -p "$ROOT/build/image"
cp "$BIN" "$ROOT/build/image/identity"
chmod 755 "$ROOT/build/image/identity"
rm -rf "$ROOT/build/image/files"
cp -a "$ROOT/files" "$ROOT/build/image/files"
