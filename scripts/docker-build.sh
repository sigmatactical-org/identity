#!/usr/bin/env bash
# Build release binary, populate build/image/, and build the runtime container.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

./scripts/prepare-local.sh
cargo build --release
./scripts/prepare-image-context.sh

case "${1:-build}" in
  build)
    docker build -f Dockerfile build/image "${@:2}"
    ;;
  compose)
    docker compose -f docker-compose-build.yaml build "${@:2}"
    ;;
  *)
    echo "usage: $0 [build|compose] [docker args...]" >&2
    exit 1
    ;;
esac
