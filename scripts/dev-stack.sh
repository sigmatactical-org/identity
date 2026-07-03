#!/usr/bin/env bash
# Dependencies for sigma-identity: PostgreSQL (sigma-pg), Keycloak (dev realm), echo.
# Use with host-native `cargo run` and .env.host (see README).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/.devcontainer"

# shellcheck source=scripts/sigma-pg-dir.sh
source "$ROOT/scripts/sigma-pg-dir.sh"

DEPS_COMPOSE=(docker compose -f docker-compose.dev-deps.yml)
PG_COMPOSE=(docker compose -f "$(sigma_pg_compose)")

KEYCLOAK_URL="${KEYCLOAK_URL:-http://127.0.0.1:8101/realms/multcorp}"
DATABASE_URL="${DATABASE_URL:-postgres://sigma:sigma@127.0.0.1:5432/sigma}"
ECHO_URL="${ECHO_URL:-http://127.0.0.1:8088/}"

postgres_sigma_ready() {
  if ! command -v psql >/dev/null 2>&1; then
    return 1
  fi
  for port in 5432 5433; do
    if PGPASSWORD=sigma psql -h 127.0.0.1 -p "$port" -U sigma -d sigma -c 'select 1' >/dev/null 2>&1; then
      return 0
    fi
  done
  return 1
}

sigma_pg_container_running() {
  docker inspect sigma-pg-postgres >/dev/null 2>&1 \
    && [[ "$(docker inspect sigma-pg-postgres --format '{{.State.Running}}')" == "true" ]]
}

ensure_sigma_dev_network() {
  docker network create sigma-dev >/dev/null 2>&1 || true
  if sigma_pg_container_running; then
    if ! docker inspect sigma-pg-postgres --format '{{range $k, $v := .NetworkSettings.Networks}}{{$k}} {{end}}' \
      | grep -qw sigma-dev; then
      docker network connect sigma-dev sigma-pg-postgres >/dev/null 2>&1 || true
    fi
  fi
}

cmd="${1:-}"
case "$cmd" in
  up)
    ensure_sigma_dev_network
    if postgres_sigma_ready; then
      echo "PostgreSQL (sigma) ready on host"
    elif sigma_pg_container_running; then
      echo "Using existing sigma-pg-postgres container"
    else
      echo "Starting PostgreSQL from sigma-pg..."
      (cd "$(sigma_pg_dir)" && "${PG_COMPOSE[@]}" up -d)
      ensure_sigma_dev_network
    fi
    "${DEPS_COMPOSE[@]}" up -d --build
    ;;
  down)
    "${DEPS_COMPOSE[@]}" down
    (cd "$(sigma_pg_dir)" && "${PG_COMPOSE[@]}" down) || true
    ;;
  wait)
    if postgres_sigma_ready; then
      echo "PostgreSQL ready (sigma@127.0.0.1:5432)"
    else
      echo "Waiting for PostgreSQL..."
      for _ in $(seq 1 30); do
        if (cd "$(sigma_pg_dir)" && "${PG_COMPOSE[@]}" exec -T postgres pg_isready -U sigma -d sigma >/dev/null 2>&1); then
          echo "PostgreSQL ready"
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
      "${DEPS_COMPOSE[@]}" logs keycloak
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
    "${DEPS_COMPOSE[@]}" logs echo
    exit 1
    ;;
  env)
    cp "$ROOT/.env.host" "$ROOT/.env"
    if command -v psql >/dev/null 2>&1; then
      for port in 5432 5433; do
        if PGPASSWORD=sigma psql -h 127.0.0.1 -p "$port" -U sigma -d sigma -c 'select 1' >/dev/null 2>&1; then
          sed -i "s|@127.0.0.1:543[23]/sigma|@127.0.0.1:${port}/sigma|" "$ROOT/.env"
          break
        fi
      done
    fi
    echo "Wrote $ROOT/.env from .env.host"
    ;;
  logs)
    service="${2:-}"
    if [[ -n "$service" && "$service" == "postgres" ]]; then
      (cd "$(sigma_pg_dir)" && "${PG_COMPOSE[@]}" logs -f postgres)
    else
      "${DEPS_COMPOSE[@]}" logs -f "${service:-}"
    fi
    ;;
  *)
    echo "usage: $0 {up|down|wait|env|logs [service]}" >&2
    exit 1
    ;;
esac
