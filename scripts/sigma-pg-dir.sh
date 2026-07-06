#!/usr/bin/env bash
# Resolve the sigma-pg checkout (migrations + migrate binary).
set -euo pipefail

sigma_pg_dir() {
  if [[ -n "${SIGMA_PG_DIR:-}" && -f "${SIGMA_PG_DIR}/migrations/001_sigma_init.sql" ]]; then
    printf '%s\n' "$(cd "${SIGMA_PG_DIR}" && pwd)"
    return 0
  fi

  local script_dir root candidates candidate
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  root="$(cd "${script_dir}/.." && pwd)"
  candidates=(
    "${root}/sigma-pg"
    "${root}/../sigma-pg"
    "${root}/../../sigma-pg"
  )

  for candidate in "${candidates[@]}"; do
    if [[ -f "${candidate}/migrations/001_sigma_init.sql" ]]; then
      printf '%s\n' "$(cd "${candidate}" && pwd)"
      return 0
    fi
  done

  echo "error: sigma-pg not found. Clone https://github.com/sigmatactical-org/sigma-pg" >&2
  echo "       next to this repo or set SIGMA_PG_DIR to the checkout path." >&2
  return 1
}

platform_dir() {
  if [[ -n "${SIGMA_PLATFORM_DIR:-}" && -f "${SIGMA_PLATFORM_DIR}/scripts/postgres-dev.sh" ]]; then
    printf '%s\n' "$(cd "${SIGMA_PLATFORM_DIR}" && pwd)"
    return 0
  fi

  local script_dir root candidates candidate
  script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  root="$(cd "${script_dir}/.." && pwd)"
  candidates=(
    "${root}/platform"
    "${root}/../platform"
    "${root}/../../platform"
  )

  for candidate in "${candidates[@]}"; do
    if [[ -f "${candidate}/scripts/postgres-dev.sh" ]]; then
      printf '%s\n' "$(cd "${candidate}" && pwd)"
      return 0
    fi
  done

  return 1
}

ensure_postgres() {
  local platform
  if platform="$(platform_dir)"; then
    "${platform}/scripts/postgres-dev.sh" port-forward-bg
    return 0
  fi

  echo "error: PostgreSQL not reachable and platform/scripts/postgres-dev.sh not found." >&2
  echo "       Deploy kind (kubectl apply -k platform/environments/dev) and port-forward," >&2
  echo "       or set SIGMA_PLATFORM_DIR to the platform checkout." >&2
  return 1
}
