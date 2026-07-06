#!/usr/bin/env bash
# OpenID Connect conformance stack: suite fake OP + sigma-identity RP + Traefik TLS.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# shellcheck source=scripts/sigma-pg-dir.sh
source "$ROOT/scripts/sigma-pg-dir.sh"

COMPOSE=(docker compose -f conformance/docker-compose.yml)
CONFORMANCE_SERVER="${CONFORMANCE_SERVER:-}"
CLIENT_ID="${CLIENT_ID:-sigma-identity-conformance}"
CLIENT_SECRET="${CLIENT_SECRET:-conformance-client-secret-not-for-production}"
IDENTITY_BIN="$ROOT/target/release/sigma-identity"
CONFORMANCE_BIN="$ROOT/target/release/sigma-conformance"
IDENTITY_LOG="/tmp/sigma-identity.log"

identity_running() {
  "${COMPOSE[@]}" ps --status running identity 2>/dev/null | grep -q identity
}

stop_identity() {
  "${COMPOSE[@]}" exec -T identity bash -lc \
    'pkill -x sigma-identity 2>/dev/null || true; for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done'
}

start_identity() {
  stop_identity
  # Run sigma-identity as the detached exec main process (not backgrounded under bash).
  "${COMPOSE[@]}" exec -d identity bash -lc \
    "cd /workspace && cp .env.conformance-run .env 2>/dev/null || cp .env.conformance-ci .env; ./target/release/sigma-identity >>${IDENTITY_LOG} 2>&1"
}

dump_identity_debug() {
  echo "--- identity process ---" >&2
  "${COMPOSE[@]}" exec -T identity bash -lc \
    'pgrep -a sigma-identity || echo "(sigma-identity not running)"' >&2 || true
  echo "--- identity binary ---" >&2
  "${COMPOSE[@]}" exec -T identity bash -lc \
    'ls -la /workspace/target/release/sigma-identity 2>/dev/null || echo "(binary missing)"' >&2 || true
  echo "--- sigma-identity log ---" >&2
  "${COMPOSE[@]}" exec -T identity bash -lc \
    "cat ${IDENTITY_LOG} 2>/dev/null || echo '(no log at ${IDENTITY_LOG})'" >&2 || true
}

wait_identity_process() {
  for _ in $(seq 1 15); do
    if "${COMPOSE[@]}" exec -T identity bash -lc 'pgrep -x sigma-identity >/dev/null'; then
      return 0
    fi
    sleep 1
  done
  return 1
}

# Used by sigma-conformance when modules reach WAITING.
IDENTITY_START_CMD="docker compose -f conformance/docker-compose.yml exec -d identity bash -lc 'pkill -x sigma-identity 2>/dev/null || true; for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done; cd /workspace && cp .env.conformance-run .env 2>/dev/null || cp .env.conformance-ci .env; ./target/release/sigma-identity >>/tmp/sigma-identity.log 2>&1'"
IDENTITY_DEBUG_CMD="docker compose -f conformance/docker-compose.yml exec -T identity bash -lc 'pgrep -a sigma-identity || echo not-running; ls -la /workspace/target/release/sigma-identity 2>/dev/null; cat /tmp/sigma-identity.log 2>/dev/null | tail -80'"

export CONFORMANCE_ENSURE_IDENTITY=1
export CONFORMANCE_IDENTITY_START_CMD="$IDENTITY_START_CMD"
export CONFORMANCE_IDENTITY_DEBUG_CMD="$IDENTITY_DEBUG_CMD"

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
  if identity_running; then
    if [[ -x "$IDENTITY_BIN" && -x "$CONFORMANCE_BIN" ]]; then
      echo "binaries ready: $IDENTITY_BIN, $CONFORMANCE_BIN (host)"
      return
    fi
    "${COMPOSE[@]}" exec -T identity bash -lc \
      'cd /workspace && cargo build --release -p sigma-identity -p sigma-conformance'
    "${COMPOSE[@]}" exec -T identity bash -lc \
      'test -x /workspace/target/release/sigma-identity && test -x /workspace/target/release/sigma-conformance'
    echo "binaries ready: /workspace/target/release/{sigma-identity,sigma-conformance} (container)"
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
  test -x "$IDENTITY_BIN" && test -x "$CONFORMANCE_BIN"
}

cmd="${1:-}"
case "$cmd" in
  up)
    hosts_entries
    if ! pg_isready -h 127.0.0.1 -p 5432 -U sigma -d sigma >/dev/null 2>&1; then
      echo "Starting PostgreSQL port-forward from kind..."
      ensure_postgres
    fi
    "${COMPOSE[@]}" up -d --build
    ;;
  down)
    "${COMPOSE[@]}" down -v
    ;;
  wait-suite)
    if [[ -z "${CONFORMANCE_SERVER:-}" ]]; then
      echo "error: CONFORMANCE_SERVER must point at a hosted OIDC conformance suite" >&2
      echo "Sigma runs PostgreSQL only; the vendor suite (MongoDB) is not started locally." >&2
      echo "Use certification.openid.net or set CONFORMANCE_SERVER to your suite URL." >&2
      exit 1
    fi
    for _ in $(seq 1 60); do
      if curl -skf "$CONFORMANCE_SERVER/" >/dev/null 2>&1; then
        echo "conformance suite ready"
        exit 0
      fi
      sleep 3
    done
    echo "error: conformance suite did not become ready at $CONFORMANCE_SERVER" >&2
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
    echo "binary ready: $IDENTITY_BIN"
    ;;
  run)
    ensure_binary
    start_identity
    if ! wait_identity_process; then
      echo "error: sigma-identity exited immediately after start" >&2
      dump_identity_debug
      exit 1
    fi
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
    dump_identity_debug
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
