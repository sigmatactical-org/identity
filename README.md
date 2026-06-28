# sigma-identity

Backend-for-frontend identity service for Sigma Tactical Group. Handles OpenID Connect login, server-side JWT sessions (http-only cookie), CSRF-protected API proxying, and static/SPA file serving.

Forked from [ErikWegner/rust-identity-service](https://github.com/ErikWegner/rust-identity-service) (MIT).

Repository: https://github.com/sigmatactical-org/identity

Shared site chrome comes from [sigma-theme](https://github.com/sigmatactical-org/sigma-theme).

## Features

- **Authentication** — OIDC login (e.g. Keycloak), JWT stored in a Redis-backed server session, exposed to browsers via an http-only cookie
- **API proxy** — forward authenticated requests to backends with JWT attached; CSRF on all methods (including GET); path-based routing to multiple targets; unauthenticated requests receive 401
- **File serving** — serve a local directory; directories with `index.html` behave as single-page apps

## Configuration

Environment variables use the `IDENTITY_` prefix. The deprecated `RIDSER_*` names from upstream are still accepted as a fallback. Copy `.env.default` to `.env` for local development.

Required:

| Variable | Purpose |
|---|---|
| `IDENTITY_BIND_PORT` | Listen port (default `3000`) |
| `IDENTITY_REDIS_URL` | Redis session store |
| `IDENTITY_SESSION_SECRET` | Session encryption key (minimum 32 bytes) |
| `IDENTITY_OIDC_ISSUER_URL` | OIDC issuer |
| `IDENTITY_OIDC_CLIENT_ID` | OIDC client id |
| `IDENTITY_OIDC_CLIENT_SECRET` | OIDC client secret |
| `IDENTITY_PROXY_TARGET` | Default authenticated proxy target |
| `IDENTITY_SESSION_REFRESH_THRESHOLD` | Seconds before token refresh |
| `IDENTITY_LOGIN_REDIRECT_APP_URIS` | Allowed post-login app redirect URIs (exact or trailing `*`) |
| `IDENTITY_OIDC_REDIRECT_URIS` | Allowed OAuth `redirect_uri` values sent to the IdP |
| `IDENTITY_LOGOUT_SSO_URI` | OIDC logout endpoint |
| `IDENTITY_LOGOUT_REDIRECT_APP_URIS` | Allowed post-logout app redirect URIs |
| `IDENTITY_LOGOUT_OIDC_REDIRECT_URIS` | Allowed `post_logout_redirect_uri` values sent to the IdP |

Optional:

| Variable | Purpose |
|---|---|
| `IDENTITY_BIND_ADDRESS` | Bind address (default `::`) |
| `IDENTITY_SESSION_COOKIE_NAME` | Session cookie name (default `identity.sid`) |
| `IDENTITY_SESSION_COOKIE_SAMESITE` | `none`, `lax` (default), or `strict` |
| `IDENTITY_ENV` | Set to `production` to block insecure TLS bypass |
| `IDENTITY_DANGER_ACCEPT_INVALID_CERTS` | Accept invalid TLS certs when proxying (dev only; blocked in production) |

See `.env.default` for the full list including proxy routing rules.

## Security notes

- Login validates both `app_uri` and `redirect_uri` against configured allowlists.
- Logout validates `app_uri` and `redirect_uri` before redirecting to the IdP.
- Proxy routes require an authenticated session and a valid CSRF token on every method except `OPTIONS`.
- Response security headers are set on all routes.

## What identity needs to run

sigma-identity is a stateless BFF except for **Redis sessions**. It does not store users or tokens in a database — the OIDC provider (Keycloak in dev) holds identity; Redis holds session cookies → tokens.

| Dependency | Dev (host) | Dev (container) | Production |
|------------|------------|-----------------|------------|
| **Redis** | `./scripts/dev-stack.sh up` → `127.0.0.1:6379` | compose `redis` service | Memorystore / managed Redis |
| **OIDC IdP** | Keycloak via dev-stack → `127.0.0.1:8101` | Keycloak in devcontainer | Keycloak (or other certified OP) |
| **Backend to proxy** | Echo via dev-stack → `127.0.0.1:8088` | compose `echo` service | Your API services |
| **sigma-theme** | `./scripts/prepare-local.sh` | same | baked into release image |
| **Config** | `.env.host` (see below) | `.env.default` in container | Secret Manager / K8s secrets |

### Quick start (host — recommended for debugging)

```bash
./scripts/prepare-local.sh
./scripts/dev-stack.sh up
./scripts/dev-stack.sh wait
./scripts/dev-stack.sh env      # writes .env from .env.host
cargo run
```

Open http://localhost:3000/exampleapp/ — login via Keycloak (`user1` / `user1` in dev realm).

Stop dependencies: `./scripts/dev-stack.sh down`

### Devcontainer (all-in-container)

VS Code Dev Containers, or manually:

```bash
cd .devcontainer
docker compose up -d
docker compose exec -u "${UID}:${GID}" -it identity /bin/bash
rustup update stable
export RUST_LOG=sigma_identity=debug,info
cp .env.default .env
cargo run
```

Keycloak, Redis, Traefik, and an echo backend are included in the compose stack. The OIDC client id in `dev_realm.json` is `identity`.

### Local (Rust only, bring your own Redis + IdP)

Requires Redis (`redis-server` on `127.0.0.1:6379`), an OIDC provider, and a built [`sigma-theme`](https://github.com/sigmatactical-org/sigma-theme) checkout:

```bash
./scripts/prepare-local.sh   # clone/link sigma-theme, build TS, patch cargo
cp .env.host .env            # or .env.default if IdP/echo hostnames match
cargo run
```

## Testing

Unit tests (requires Redis):

```bash
IDENTITY_TEST_REDIS_URL=redis://127.0.0.1:6379/ cargo test -- --test-threads=1
```

Example app frontend — TypeScript in `ts/src/`, gitignored bundle at `files/exampleapp/sample.js`:

```bash
cd ts && npm ci && npm run check && npm run build
```

Browser E2E (Playwright, optional):

```bash
./scripts/prepare-local.sh
./scripts/e2e-stack.sh up
./scripts/e2e-stack.sh build && ./scripts/e2e-stack.sh run && ./scripts/e2e-stack.sh wait
cd tests && npm ci && npx playwright install --with-deps && npx playwright test
./scripts/e2e-stack.sh down
```

CI (`.github/workflows/playwright.yml`) runs **Chromium only**; locally you can add `--project=firefox` or `--project=webkit`.

### OpenID Connect conformance (RP)

Runs the [OpenID Foundation conformance suite](https://gitlab.com/openid/conformance-suite) locally with **sigma-identity as the relying party** and the suite’s fake OP. See [`conformance/README.md`](conformance/README.md) for details.

```bash
# /etc/hosts: 127.0.0.1 localhost.emobix.co.uk
./scripts/prepare-local.sh
./scripts/conformance-stack.sh up
./scripts/conformance-stack.sh wait-suite
./scripts/conformance-stack.sh build
./scripts/conformance-stack.sh test-smoke    # one dev-plan module
./scripts/conformance-stack.sh test --plan oidcc-client-test-plan
./scripts/conformance-stack.sh test            # all configured RP plans
./scripts/conformance-stack.sh down
```

CI (`.github/workflows/conformance.yml`): full dev plan on `main` pushes; all plans weekly; manual dispatch with optional `plan` input.

## File serving

App-specific static pages (e.g. `files/exampleapp/`) live under `IDENTITY_FILES_DIR`. Shared chrome — CSS, JS, home page, favicon — comes from the [sigma-theme](https://github.com/sigmatactical-org/sigma-theme) crate (embedded at compile time via `./scripts/prepare-local.sh`).

## Docker

Release is entirely in **`.github/workflows/release.yml`**: GitHub Actions builds the Rust binary, `prepare-image-context.sh` copies artifacts into `build/image/`, then `docker build` (COPY-only Dockerfile) pushes to GHCR and Artifact Registry.

The runtime image is Debian 13 distroless (`gcr.io/distroless/cc-debian13:nonroot`). Production images omit the demo SPA under `files/exampleapp/` unless `IDENTITY_IMAGE_INCLUDE_DEMO=1`.

To stage and build the image locally:

```bash
./scripts/docker-build.sh              # stage build/image/ locally
docker build -f Dockerfile build/image
```

Local dev and E2E dependencies use Debian bookworm where possible (Redis, echo, Traefik). Keycloak is upstream UBI (dev-only).

## Upstream

Track upstream changes:

```bash
git fetch upstream
git merge upstream/main
```

Remote `upstream` points at [ErikWegner/rust-identity-service](https://github.com/ErikWegner/rust-identity-service).

## License

Licensed **MIT OR Apache-2.0** (see `LICENSE-MIT` and `LICENSE-APACHE` in this directory).
