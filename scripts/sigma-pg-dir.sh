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

  fetch_pinned_sigma_pg "${root}"
}

# Cache a checkout of sigma-pg at the revision this repo pins, printing its path.
#
# The migrations have to match the crate the service links against, and the
# lockfile is the only place that says which revision that is. Reading it here
# means a machine with no sibling checkout -- CI, or a fresh clone -- prepares
# the same schema as one that has one, with nothing to keep in sync by hand.
fetch_pinned_sigma_pg() {
  local root="$1" rev cache
  rev="$(sed -n 's|.*sigma-pg\.git?rev=\([0-9a-f]\{40\}\)#.*|\1|p' "${root}/Cargo.lock" | head -1)"
  if [[ -z "${rev}" ]]; then
    echo "error: no sigma-pg git revision in ${root}/Cargo.lock, and no checkout nearby." >&2
    echo "       Clone https://github.com/sigmatactical-org/sigma-pg next to this repo" >&2
    echo "       or set SIGMA_PG_DIR to it." >&2
    return 1
  fi

  cache="${XDG_CACHE_HOME:-${HOME}/.cache}/sigma-pg-checkouts/${rev}"
  if [[ ! -f "${cache}/migrations/001_sigma_init.sql" ]]; then
    rm -rf "${cache}"
    mkdir -p "${cache}"
    git init --quiet "${cache}"
    git -C "${cache}" fetch --quiet --depth 1 \
      https://github.com/sigmatactical-org/sigma-pg.git "${rev}" >&2
    git -C "${cache}" checkout --quiet FETCH_HEAD
    echo "Fetched sigma-pg ${rev:0:12} to ${cache}" >&2
  fi
  printf '%s\n' "${cache}"
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
