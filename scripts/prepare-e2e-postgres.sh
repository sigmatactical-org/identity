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

echo "Ensuring keycloak database exists..."
if ! "${psql_base[@]}" -d postgres -tAc "SELECT 1 FROM pg_database WHERE datname = 'keycloak'" | grep -q 1; then
  "${psql_base[@]}" -d postgres -c "CREATE DATABASE keycloak OWNER sigma;"
fi

pg_dir="$(sigma_pg_dir)"
echo "Running sigma-pg migrations from $pg_dir..."
DATABASE_URL="postgres://${PGUSER}:${PGPASSWORD}@${PGHOST}:${PGPORT}/sigma" \
  cargo run --manifest-path "$pg_dir/Cargo.toml" --bin sigma-pg-migrate

echo "PostgreSQL ready for identity E2E."
