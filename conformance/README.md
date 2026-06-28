# OpenID Connect RP conformance

Automated [OpenID Foundation conformance suite](https://gitlab.com/openid/conformance-suite) runs against **sigma-identity** as the relying party (OAuth/OIDC client). The suite provides a fake OpenID Provider; identity uses authorization code + PKCE, `client_secret_post` token auth, and server-side sessions.

## Prerequisites

- Docker Compose v2
- Rust toolchain (builds `sigma-conformance` and `sigma-identity`)
- `/etc/hosts`: `127.0.0.1 localhost.emobix.co.uk`
- Built theme (`./scripts/prepare-local.sh`)

## Quick start

```bash
./scripts/conformance-stack.sh up
./scripts/conformance-stack.sh wait-suite
./scripts/conformance-stack.sh build
./scripts/conformance-stack.sh test-smoke
```

`test-smoke` starts a suite module (fake OP), waits for `WAITING`, then starts identity and runs the module. Bootstrap is optional (records plan metadata only).

Suite UI: [https://localhost.emobix.co.uk:8443](https://localhost.emobix.co.uk:8443)  
RP harness UI: [https://localhost:3000/conformance/](https://localhost:3000/conformance/)

## Plans

## Plans

| Plan | Config | Notes |
|------|--------|-------|
| `oidcc-client-test-plan` | `config/oidcc-client-test.json` | Development plan (29 modules; CI on `main` push) |
| `oidcc-client-basic-certification-test-plan` | `config/oidcc-client-basic.json` | Basic RP certification (14 modules; `client_secret_basic`) |
| `oidcc-client-refreshtoken-test-plan` | `config/oidcc-client-refreshtoken.json` | Refresh token (`form_post`, 3 modules) |
| `oidcc-rp-initiated-logout-certification-test-plan` | `config/oidcc-rp-initiated-logout.json` | RP-initiated logout (11 modules; needs browser OP confirm — see below) |

Run one plan:

```bash
./target/release/sigma-conformance run --plan oidcc-client-test-plan
```

Run all plans in `plans.json`:

```bash
./scripts/conformance-stack.sh test
```

## Environment

Copy [`.env.conformance-ci`](../.env.conformance-ci) — key variables:

| Variable | Purpose |
|----------|---------|
| `IDENTITY_OIDC_ISSUER_URL` | Fake OP: `https://localhost.emobix.co.uk:8443/test/a/sigma-identity/` (trailing slash required) |
| `IDENTITY_OIDC_CLIENT_ID` / `SECRET` | Must match `conformance/config/*.json` |
| `IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD` | `client_secret_post` for most RP plans |
| `IDENTITY_OIDC_AUTHORIZE_RESPONSE_MODE` | Set to `form_post` for refresh-token plan (runner sets per plan) |
| `IDENTITY_LOGOUT_SEND_ID_TOKEN_HINT` | `1` for RP-initiated logout plan (runner sets per module) |
| `IDENTITY_CONFORMANCE_MODE` | Enables conformance harness under `/conformance/` |

## CI

`.github/workflows/conformance.yml`:

- **Push to `main`** (conformance-related paths): full `oidcc-client-test-plan`
- **Weekly schedule**: all plans in `plans.json`
- **Manual dispatch**: optional `plan` input (plan name or `all`)

Settle/gap env vars in the workflow: `CONFORMANCE_MODULE_SETTLE=20`, `CONFORMANCE_MODULE_GAP=25`, `CONFORMANCE_MODULE_TIMEOUT=360`.

### Local pass status (automated curl runner)

| Plan | Result |
|------|--------|
| `oidcc-client-test-plan` | 28 passed, 1 skipped |
| `oidcc-client-basic-certification-test-plan` | 13 passed, 1 skipped |
| `oidcc-client-refreshtoken-test-plan` | 3 passed |
| `oidcc-rp-initiated-logout-certification-test-plan` | Blocked — suite requires `server.discoveryUrl` at plan creation, but the fake OP only accepts discovery after the module reaches `WAITING`. Use the suite UI + `browser` automation in the plan config, or an always-on OP (see SimpleSAMLphp conformance configs). |

## Architecture

```
Browser / suite automation
    → https://localhost:3000 (Traefik TLS → identity BFF)
    → https://localhost.emobix.co.uk:8443/test/a/sigma-identity/ (fake OP)
```

Bootstrap **must not** run identity before a test module: the fake OP exists only while a module is active. `test-smoke` starts the module first, then identity.

## BFF caveats

sigma-identity is a BFF (custom `/auth/login`, allowlists, Redis sessions), not a minimal OIDC client library demo. The runner drives flows via curl against `/auth/login`, `/auth/refresh`, `/auth/logout`, and `/auth/conformance/discover`. Formal hosted certification uses [certification.openid.net](https://www.certification.openid.net/).

## Results

Last run state: `conformance/.last-run.json`  
Bootstrap issuer: `conformance/.bootstrap-plan.json`
