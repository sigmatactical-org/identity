#!/usr/bin/env bash
# Start/stop the devcontainer E2E stack (Traefik TLS, Keycloak, echo).
# PostgreSQL comes from the kind stack (platform/scripts/postgres-dev.sh).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# shellcheck source=scripts/sigma-pg-dir.sh
source "$ROOT/scripts/sigma-pg-dir.sh"

COMPOSE=(docker compose -f .devcontainer/docker-compose.yml -f .devcontainer/docker-compose.e2e.yml)
IDENTITY_BIN="$ROOT/target/release/sigma-identity"
IDENTITY_LOG="/tmp/sigma-identity.log"

hosts_keycloak() {
  if ! grep -q '[[:space:]]keycloak\.localhost' /etc/hosts 2>/dev/null; then
    echo "127.0.0.1 keycloak.localhost" | sudo tee -a /etc/hosts >/dev/null
  fi
}

keycloak_theme_marker() {
  printf '%s\n' "$1/assets/keycloak/sigma/login/theme.properties"
}

prepare_theme_layout() {
  local compose_theme expected
  compose_theme="$(cd "$ROOT/.devcontainer/../.." && pwd)/theme"
  expected="$(keycloak_theme_marker "$compose_theme")"
  if [[ -f "$expected" ]]; then
    return 0
  fi

  local repo_theme
  repo_theme="$(keycloak_theme_marker "$ROOT/theme")"
  if [[ -f "$repo_theme" ]]; then
    mkdir -p "$(dirname "$compose_theme")"
    ln -sfn "$ROOT/theme" "$compose_theme"
  fi
}

identity_running() {
  "${COMPOSE[@]}" ps --status running identity 2>/dev/null | grep -q identity
}

ensure_binary() {
  if identity_running; then
    if [[ -x "$IDENTITY_BIN" ]]; then
      echo "binary ready: $IDENTITY_BIN (host)"
      return
    fi
    "${COMPOSE[@]}" exec -T identity bash -lc \
      'cd /workspace && cargo build --release -p sigma-identity'
    "${COMPOSE[@]}" exec -T identity bash -lc \
      'test -x /workspace/target/release/sigma-identity'
    echo "binary ready: /workspace/target/release/sigma-identity (container)"
    return
  fi

  mkdir -p target/release
  cargo build --release -p sigma-identity
  test -x "$IDENTITY_BIN"
  echo "binary ready: $IDENTITY_BIN (host)"
}

stop_identity() {
  "${COMPOSE[@]}" exec -T identity bash -lc \
    'pkill -x sigma-identity 2>/dev/null || true; for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done'
}

start_identity() {
  stop_identity
  "${COMPOSE[@]}" exec -d identity bash -lc \
    "cd /workspace && cp .env.e2e-ci .env && ./target/release/sigma-identity >>${IDENTITY_LOG} 2>&1"
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

cmd="${1:-}"
case "$cmd" in
  up)
    hosts_keycloak
    prepare_theme_layout
    if ! PGPASSWORD=sigma psql -h 127.0.0.1 -p 5432 -U sigma -d sigma -c 'select 1' >/dev/null 2>&1; then
      echo "Starting PostgreSQL port-forward from kind..."
      ensure_postgres
    fi
    "${COMPOSE[@]}" up -d --build
    ;;
  down)
    prepare_theme_layout
    "${COMPOSE[@]}" down -v
    ;;
  build)
    ensure_binary
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
    for _ in $(seq 1 90); do
      if curl -skf https://localhost:3000/app/up >/dev/null \
        && curl -skf https://keycloak.localhost:8101/realms/multcorp/.well-known/openid-configuration >/dev/null; then
        echo "identity and keycloak ready"
        exit 0
      fi
      sleep 2
    done
    echo "error: identity/keycloak did not become ready" >&2
    dump_identity_debug
    "${COMPOSE[@]}" logs identity traefik keycloak
    exit 1
    ;;
  *)
    echo "usage: $0 {up|down|build|run|wait}" >&2
    exit 1
    ;;
esac
