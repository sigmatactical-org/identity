# OpenID Connect RP conformance

Automated [OpenID Foundation conformance suite](https://gitlab.com/openid/conformance-suite) runs against **sigma-identity** as the relying party (OAuth/OIDC client). The suite provides a fake OpenID Provider; identity uses authorization code + PKCE, `client_secret_post` token auth, and server-side sessions.

## Prerequisites

- Docker Compose v2
- Python 3.11+
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

| Plan | Config | Notes |
|------|--------|-------|
| `oidcc-client-test-plan` | `config/oidcc-client-test.json` | Development smoke (CI default on dispatch) |
| `oidcc-client-basic-certification-test-plan` | `config/oidcc-client-basic.json` | Basic RP certification profile |
| `oidcc-client-refreshtoken-test-plan` | `config/oidcc-client-refreshtoken.json` | Refresh token behaviour |
| `oidcc-rp-initiated-logout-certification-test-plan` | `config/oidcc-rp-initiated-logout.json` | RP-initiated logout |

Run one plan:

```bash
python3 conformance/scripts/run.py --plan oidcc-client-test-plan
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
| `IDENTITY_CONFORMANCE_MODE` | Enables conformance harness under `/conformance/` |

## Architecture

```
Browser / suite automation
    → https://localhost:3000 (Traefik TLS → identity BFF)
    → https://localhost.emobix.co.uk:8443/test/a/sigma-identity/ (fake OP)
```

Bootstrap **must not** run identity before a test module: the fake OP exists only while a module is active. `test-smoke` starts the module first, then identity.

## BFF caveats

sigma-identity is a BFF (custom `/auth/login`, allowlists, Redis sessions), not a minimal OIDC client library demo. Some certification modules may fail until the harness or auth routes are extended. Use results to find spec gaps; Keycloak remains the certified OP in production.

Formal certification requires the hosted suite at [certification.openid.net](https://www.certification.openid.net/) and published artifacts with client logs.

## Results

Last run state: `conformance/.last-run.json`  
Bootstrap issuer: `conformance/.bootstrap-plan.json`
