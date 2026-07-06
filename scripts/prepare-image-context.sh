#!/usr/bin/env bash
# Populate build/image/ for `docker build -f Dockerfile build/image`.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/target/release/sigma-identity"
if [[ ! -f "$BIN" && -f "$ROOT/../target/release/sigma-identity" ]]; then
  BIN="$ROOT/../target/release/sigma-identity"
fi
if [[ ! -f "$BIN" ]]; then
  echo "error: missing $BIN — run: cargo build --release" >&2
  exit 1
fi

normalize_tree() {
  local dir="$1"
  find "$dir" -type d -exec chmod 755 {} \;
  find "$dir" -type f -exec chmod 644 {} \;
}

mkdir -p "$ROOT/build/image"
rm -f "$ROOT/build/image/sigma-identity"
cp "$BIN" "$ROOT/build/image/sigma-identity"
chmod 555 "$ROOT/build/image/sigma-identity"
rm -rf "$ROOT/build/image/files"
mkdir -p "$ROOT/build/image/files"

# Production images omit the local demo SPA (still available via compose volume mount).
if [[ "${IDENTITY_IMAGE_INCLUDE_DEMO:-0}" == "1" ]]; then
  cp -a "$ROOT/files/." "$ROOT/build/image/files/"
else
  if command -v rsync >/dev/null 2>&1; then
    rsync -a --exclude='exampleapp' --exclude='conformance' "$ROOT/files/" "$ROOT/build/image/files/"
  else
    cp -a "$ROOT/files/." "$ROOT/build/image/files/"
    rm -rf "$ROOT/build/image/files/exampleapp" "$ROOT/build/image/files/conformance"
  fi
fi

if [[ -z "$(ls -A "$ROOT/build/image/files" 2>/dev/null || true)" ]]; then
  rmdir "$ROOT/build/image/files"
fi

if [[ -d "$ROOT/build/image/files" ]]; then
  normalize_tree "$ROOT/build/image/files"
fi
