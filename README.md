# identity

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

## Development

### Devcontainer (recommended)

VS Code with the Dev Containers extension, or manually:

```bash
cd .devcontainer
docker compose up -d
docker compose exec -u "${UID}:${GID}" -it identity /bin/bash
rustup update stable
export RUST_LOG=identity=debug,info
cp .env.default .env
cargo run
```

Keycloak, Redis, Traefik, and an echo backend are included in the compose stack. The OIDC client id in `dev_realm.json` is `identity`.

### Local (Rust only)

Requires Redis, an OIDC provider, and a built [`sigma-theme`](https://github.com/sigmatactical-org/sigma-theme) checkout:

```bash
./scripts/prepare-local.sh   # clone/link sigma-theme, build TS, patch cargo
cp .env.default .env
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

End-to-end:

```bash
cd tests && npm ci && npx playwright test
```

Playwright tests expect the full devcontainer stack (see `.github/workflows/playwright.yml`).

## File serving

App-specific static pages (e.g. `files/exampleapp/`) live under `IDENTITY_FILES_DIR`. Shared chrome — CSS, JS, home page, favicon — comes from the [`sigma-theme`](../sigma-theme) crate (embedded at compile time).

## Docker

```bash
docker compose -f docker-compose-build.yaml up --build
```

## Upstream

Track upstream changes:

```bash
git fetch upstream
git merge upstream/main
```

Remote `upstream` points at [ErikWegner/rust-identity-service](https://github.com/ErikWegner/rust-identity-service).
