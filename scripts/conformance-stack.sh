#!/usr/bin/env bash
# OpenID Connect conformance stack: suite fake OP + sigma-identity RP + Traefik TLS.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f conformance/docker-compose.yml)
CONFORMANCE_SERVER="${CONFORMANCE_SERVER:-https://127.0.0.1:8443}"
CLIENT_ID="${CLIENT_ID:-sigma-identity-conformance}"
CLIENT_SECRET="${CLIENT_SECRET:-conformance-client-secret-not-for-production}"
IDENTITY_START_CMD="docker compose -f conformance/docker-compose.yml exec -d identity bash -lc 'pkill -x sigma-identity 2>/dev/null || true; for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done; cd /workspace && cp .env.conformance-run .env 2>/dev/null || cp .env.conformance-ci .env; nohup ./target/release/sigma-identity >/tmp/sigma-identity.log 2>&1 &'"

export CONFORMANCE_ENSURE_IDENTITY=1
export CONFORMANCE_IDENTITY_START_CMD="$IDENTITY_START_CMD"

hosts_entries() {
  if grep -q '[[:space:]]localhost\.emobix\.co\.uk' /etc/hosts 2>/dev/null; then
    return 0
  fi
  if curl -skf "${CONFORMANCE_SERVER}/" >/dev/null 2>&1; then
    return 0
  fi
  if ! grep -q '[[:space:]]localhost\.emobix\.co\.uk' /etc/hosts 2>/dev/null; then
    echo "127.0.0.1 localhost.emobix.co.uk" | sudo tee -a /etc/hosts >/dev/null
  fi
}

ensure_binary() {
  if "${COMPOSE[@]}" ps --status running identity 2>/dev/null | grep -q identity; then
    "${COMPOSE[@]}" exec -T identity bash -lc \
      'pkill -x sigma-identity 2>/dev/null || true; for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done; cd /workspace && cargo build --release -p sigma-identity -p sigma-conformance'
    return
  fi
  mkdir -p target/release
  cargo build --release -p sigma-identity -p sigma-conformance
  if [[ -f ../target/release/sigma-identity ]]; then
    cp ../target/release/sigma-identity target/release/sigma-identity
    cp ../target/release/sigma-conformance target/release/sigma-conformance 2>/dev/null || true
  elif [[ ! -f target/release/sigma-identity ]]; then
    ./scripts/prepare-local.sh
    cargo build --release -p sigma-identity -p sigma-conformance
    cp ../target/release/sigma-identity target/release/sigma-identity
    cp ../target/release/sigma-conformance target/release/sigma-conformance 2>/dev/null || true
  fi
}

cmd="${1:-}"
case "$cmd" in
  up)
    hosts_entries
    "${COMPOSE[@]}" up -d --build
    ;;
  down)
    "${COMPOSE[@]}" down -v
    ;;
  wait-suite)
    for _ in $(seq 1 60); do
      if curl -skf "$CONFORMANCE_SERVER/" >/dev/null 2>&1; then
        echo "conformance suite ready"
        exit 0
      fi
      sleep 3
    done
    echo "error: conformance suite did not become ready at $CONFORMANCE_SERVER" >&2
    "${COMPOSE[@]}" logs server nginx
    exit 1
    ;;
  bootstrap)
    hosts_entries
    "$ROOT/target/release/sigma-conformance" bootstrap \
      --conformance-server "$CONFORMANCE_SERVER" \
      --client-id "$CLIENT_ID" \
      --client-secret "$CLIENT_SECRET"
    echo "note: fake OP issuer is active only while a test module is running (see test-smoke)" >&2
    ;;
  build)
    ensure_binary
    echo "binary ready: $ROOT/target/release/sigma-identity"
    ;;
  run)
    ensure_binary
    eval "$IDENTITY_START_CMD"
    ;;
  wait)
    for _ in $(seq 1 120); do
      if curl -skf https://localhost:3000/app/up >/dev/null; then
        echo "identity ready"
        exit 0
      fi
      sleep 2
    done
    echo "error: identity did not become ready" >&2
    "${COMPOSE[@]}" logs identity traefik
    exit 1
    ;;
  test)
    ensure_binary
    export CONFORMANCE_SERVER="$CONFORMANCE_SERVER" CLIENT_ID CLIENT_SECRET
    if [[ $# -lt 2 ]]; then
      "$ROOT/target/release/sigma-conformance" run --all
    else
      "$ROOT/target/release/sigma-conformance" run "${@:2}"
    fi
    ;;
  test-smoke)
    ensure_binary
    export CONFORMANCE_SERVER="$CONFORMANCE_SERVER" CLIENT_ID CLIENT_SECRET
    "$ROOT/target/release/sigma-conformance" run --plan oidcc-client-test-plan --module oidcc-client-test "${@:2}"
    ;;
  *)
    echo "usage: $0 {up|down|wait-suite|bootstrap|build|run|wait|test|test-smoke}" >&2
    exit 1
    ;;
esac
