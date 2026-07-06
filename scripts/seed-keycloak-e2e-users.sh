#!/usr/bin/env bash
# Seed Keycloak test users for identity E2E / Playwright (user1 fixture).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT/.devcontainer"

COMPOSE=(docker compose -f docker-compose.yml -f docker-compose.e2e.yml)
REALM="${KEYCLOAK_REALM:-multcorp}"
KC_ADMIN_USER="${KEYCLOAK_ADMIN_USER:-admin}"
KC_ADMIN_PASSWORD="${KEYCLOAK_ADMIN_PASSWORD:-admin}"
FIXTURE_USERNAME="${E2E_FIXTURE_USERNAME:-user1}"
FIXTURE_PASSWORD="${E2E_FIXTURE_PASSWORD:-user1}"

kcadm() {
  "${COMPOSE[@]}" exec -T keycloak /opt/keycloak/bin/kcadm.sh "$@"
}

echo "Waiting for Keycloak admin API..."
for _ in $(seq 1 60); do
  if kcadm config credentials \
    --server http://localhost:8101 \
    --realm master \
    --user "$KC_ADMIN_USER" \
    --password "$KC_ADMIN_PASSWORD" >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

kcadm config credentials \
  --server http://localhost:8101 \
  --realm master \
  --user "$KC_ADMIN_USER" \
  --password "$KC_ADMIN_PASSWORD"

existing_id="$(kcadm get users -r "$REALM" -q "username=$FIXTURE_USERNAME" --fields id --format csv --noquotes 2>/dev/null | tail -1 || true)"
if [[ -z "$existing_id" || "$existing_id" == "id" ]]; then
  echo "Creating Keycloak user $FIXTURE_USERNAME..."
  kcadm create users -r "$REALM" \
    -s "username=$FIXTURE_USERNAME" \
    -s "email=${FIXTURE_USERNAME}@example.com" \
    -s emailVerified=true \
    -s enabled=true
  existing_id="$(kcadm get users -r "$REALM" -q "username=$FIXTURE_USERNAME" --fields id --format csv --noquotes | tail -1)"
else
  echo "Keycloak user $FIXTURE_USERNAME already exists ($existing_id)"
fi

echo "Setting password for $FIXTURE_USERNAME..."
kcadm set-password -r "$REALM" --userid "$existing_id" --new-password "$FIXTURE_PASSWORD"

echo "Granting sigma-admin to $FIXTURE_USERNAME..."
kcadm add-roles -r "$REALM" --userid "$existing_id" --rolename sigma-admin 2>/dev/null \
  || kcadm add-roles -r "$REALM" --uusername "$FIXTURE_USERNAME" --rolename sigma-admin

sa_username="service-account-identity"
for role in manage-users view-users; do
  kcadm add-roles -r "$REALM" --uusername "$sa_username" --cclientid realm-management --rolename "$role" 2>/dev/null \
    || true
done

echo "E2E Keycloak user ready: $FIXTURE_USERNAME / $FIXTURE_PASSWORD"
