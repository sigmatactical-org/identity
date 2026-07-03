#!/usr/bin/env bash
# Start/stop the devcontainer E2E stack (Traefik TLS, Keycloak, Redis, echo).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f .devcontainer/docker-compose.yml -f .devcontainer/docker-compose.e2e.yml)

hosts_keycloak() {
  if ! grep -q '[[:space:]]keycloak\.localhost' /etc/hosts 2>/dev/null; then
    echo "127.0.0.1 keycloak.localhost" | sudo tee -a /etc/hosts >/dev/null
  fi
}

cmd="${1:-}"
case "$cmd" in
  up)
    hosts_keycloak
    "${COMPOSE[@]}" up -d --build
    ;;
  down)
    "${COMPOSE[@]}" down -v
    ;;
  build)
    "${COMPOSE[@]}" exec -T identity bash -lc \
      'cd /workspace && cargo build --release -p sigma-identity'
    ;;
  run)
    "${COMPOSE[@]}" exec -d identity bash -lc \
      'cd /workspace && cp .env.e2e-ci .env && nohup ./target/release/sigma-identity >/tmp/sigma-identity.log 2>&1 &'
    ;;
  wait)
    for _ in $(seq 1 90); do
      if curl -skf https://localhost:3000/app/up >/dev/null; then
        echo "identity ready"
        exit 0
      fi
      sleep 2
    done
    echo "error: identity did not become ready at https://localhost:3000/app/up" >&2
    "${COMPOSE[@]}" logs
    exit 1
    ;;
  *)
    echo "usage: $0 {up|down|build|run|wait}" >&2
    exit 1
    ;;
esac
