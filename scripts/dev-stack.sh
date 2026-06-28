#!/usr/bin/env bash
# Dependencies for sigma-identity: Redis, Keycloak (dev realm), echo backend.
# Use with host-native `cargo run` and .env.host (see README).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/.devcontainer"

COMPOSE=(docker compose -f docker-compose.dev-deps.yml)

KEYCLOAK_URL="${KEYCLOAK_URL:-http://127.0.0.1:8101/realms/multcorp}"
REDIS_URL="${REDIS_URL:-redis://127.0.0.1:6379/}"
ECHO_URL="${ECHO_URL:-http://127.0.0.1:8088/}"

redis_on_host() {
  if command -v redis-cli >/dev/null 2>&1; then
    redis-cli -h 127.0.0.1 -p 6379 ping 2>/dev/null | grep -q PONG && return 0
  fi
  (echo -en "PING\r\n"; sleep 0.1) 2>/dev/null | nc -w 1 127.0.0.1 6379 2>/dev/null | grep -q PONG
}

cmd="${1:-}"
case "$cmd" in
  up)
    SCALE_REDIS=()
    if redis_on_host; then
      echo "Redis already listening on 127.0.0.1:6379 — using host Redis (not starting container)"
      SCALE_REDIS=(--scale redis=0)
    fi
    "${COMPOSE[@]}" up -d --build "${SCALE_REDIS[@]}"
    ;;
  down)
    "${COMPOSE[@]}" down
    ;;
  wait)
    if redis_on_host; then
      echo "Redis ready (host)"
    else
      echo "Waiting for Redis..."
      for _ in $(seq 1 30); do
        if "${COMPOSE[@]}" exec -T redis redis-cli ping 2>/dev/null | grep -q PONG; then
          echo "Redis ready"
          break
        fi
        sleep 1
      done
    fi
    echo "Waiting for Keycloak..."
    for _ in $(seq 1 90); do
      if curl -sf "${KEYCLOAK_URL}/.well-known/openid-configuration" >/dev/null 2>&1; then
        echo "Keycloak ready"
        break
      fi
      sleep 2
    done
    if ! curl -sf "${KEYCLOAK_URL}/.well-known/openid-configuration" >/dev/null 2>&1; then
      echo "error: Keycloak did not become ready at ${KEYCLOAK_URL}" >&2
      "${COMPOSE[@]}" logs keycloak
      exit 1
    fi
    echo "Waiting for echo at ${ECHO_URL}..."
    for _ in $(seq 1 30); do
      if curl -sf "${ECHO_URL}" >/dev/null 2>&1; then
        echo "Echo ready"
        exit 0
      fi
      sleep 1
    done
    echo "error: echo backend did not become ready" >&2
    "${COMPOSE[@]}" logs echo
    exit 1
    ;;
  env)
    cp "$ROOT/.env.host" "$ROOT/.env"
    echo "Wrote $ROOT/.env from .env.host"
    ;;
  logs)
    "${COMPOSE[@]}" logs -f "${2:-}"
    ;;
  *)
    echo "usage: $0 {up|down|wait|env|logs [service]}" >&2
    exit 1
    ;;
esac
