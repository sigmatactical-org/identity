#!/usr/bin/env bash
# Build sigma-theme and identity TypeScript, and patch the git dependency for local/CI builds.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

THEME_PATH="sigma-theme"
if [[ -d ../sigma-theme/ts ]]; then
  THEME_PATH="../sigma-theme"
elif [[ ! -d sigma-theme/ts ]]; then
  git clone --depth 1 https://github.com/sigmatactical-org/sigma-theme.git sigma-theme
fi

mkdir -p .cargo
cat > .cargo/config.toml <<EOF
[patch."https://github.com/sigmatactical-org/sigma-theme.git"]
sigma-theme = { path = "$THEME_PATH" }
EOF

(cd "$THEME_PATH/ts" && npm ci && npm run check && npm run build)
(cd ts && npm ci && npm run check && npm run build)

echo "sigma-theme ($THEME_PATH) and identity frontends ready for cargo build."
