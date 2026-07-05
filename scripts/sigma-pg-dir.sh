#!/usr/bin/env bash
# Resolve the sigma-pg checkout (PostgreSQL compose + init scripts).
set -euo pipefail

sigma_pg_dir() {
  if [[ -n "${SIGMA_PG_DIR:-}" && -f "${SIGMA_PG_DIR}/docker-compose.deps.yml" ]]; then
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
    if [[ -f "${candidate}/docker-compose.deps.yml" ]]; then
      printf '%s\n' "$(cd "${candidate}" && pwd)"
      return 0
    fi
  done

  echo "error: sigma-pg not found. Clone https://github.com/sigmatactical-org/sigma-pg" >&2
  echo "       next to this repo or set SIGMA_PG_DIR to the checkout path." >&2
  return 1
}

sigma_pg_compose() {
  printf '%s\n' "$(sigma_pg_dir)/docker-compose.deps.yml"
}
