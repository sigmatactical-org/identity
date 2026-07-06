#!/usr/bin/env bash
# Prepare GitHub Actions / host Postgres for identity E2E (sigma DB + Keycloak DB + migrations).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=scripts/sigma-pg-dir.sh
source "$ROOT/scripts/sigma-pg-dir.sh"

PGHOST="${PGHOST:-127.0.0.1}"
PGPORT="${PGPORT:-5432}"
PGUSER="${PGUSER:-sigma}"
PGPASSWORD="${PGPASSWORD:-sigma}"
export PGPASSWORD

psql_base=(psql -h "$PGHOST" -p "$PGPORT" -U "$PGUSER" -v ON_ERROR_STOP=1)

echo "Ensuring keycloak schema exists in database sigma..."
"${psql_base[@]}" -d sigma -c "CREATE SCHEMA IF NOT EXISTS keycloak;"
"${psql_base[@]}" -d sigma -c "GRANT ALL ON SCHEMA keycloak TO sigma;"

pg_dir="$(sigma_pg_dir)"
echo "Running sigma-pg migrations from $pg_dir..."
(
  cd "$pg_dir"
  DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@${PGHOST}:${PGPORT}/sigma" \
    cargo run --bin sigma-pg-migrate
)

echo "PostgreSQL ready for identity E2E."
