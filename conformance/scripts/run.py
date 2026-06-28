#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Run OpenID Connect RP conformance plans against sigma-identity."""

from __future__ import annotations

import argparse
import json
import logging
import os
import ssl
import subprocess
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
IDENTITY_ROOT = ROOT.parent
sys.path.insert(0, str(Path(__file__).resolve().parent))

from conformance import (  # noqa: E402
    PASSING_RESULTS,
    ConformanceClient,
    ConformanceError,
    format_module_log,
)

STATE_FILE = ROOT / ".last-run.json"
PLANS_FILE = ROOT / "plans.json"
ENV_BASE = IDENTITY_ROOT / ".env.conformance-ci"
ENV_RUN = IDENTITY_ROOT / ".env.conformance-run"
DEFAULT_IDENTITY_URL = "https://localhost:3000/app/up"
DEFAULT_LOGIN_APP_URI = "https://localhost:3000/app/up"
DEFAULT_ISSUER = "https://localhost.emobix.co.uk:8443/test/a/sigma-identity/"
DEFAULT_ALIAS = "sigma-identity"
SUITE_PUBLIC_ORIGIN = "https://localhost.emobix.co.uk:8443"
TERMINAL_STATES = {"INTERRUPTED", "FAILED", "WARNING", "REVIEW", "PASSED", "SKIPPED"}
DEFAULT_IDENTITY_START_CMD = (
    "docker compose -f conformance/docker-compose.yml exec -d identity bash -lc "
    "'pkill -x sigma-identity 2>/dev/null || true; "
    "for _ in 1 2 3 4 5 6 7 8 9 10; do pgrep -x sigma-identity >/dev/null || break; sleep 1; done; "
    "cd /workspace && cp .env.conformance-run .env 2>/dev/null || cp .env.conformance-ci .env; "
    "nohup ./target/release/sigma-identity >/tmp/sigma-identity.log 2>&1 &'"
)


def module_action(module_name: str) -> str:
    if "discovery" in module_name:
        return "discover"
    if "refreshtoken" in module_name or module_name.startswith("oidcc-client-test-refresh"):
        return "refresh"
    if "logout" in module_name:
        return "logout"
    return "login"


def module_env_overrides(module_name: str) -> dict[str, str]:
    overrides: dict[str, str] = {}

    if "client-secret-basic" in module_name:
        overrides["IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD"] = "client_secret_basic"
    elif module_action(module_name) == "login":
        overrides["IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD"] = "client_secret_post"

    if "userinfo-bearer-body" in module_name:
        overrides["IDENTITY_OIDC_USERINFO_METHOD"] = "body"
    else:
        overrides["IDENTITY_OIDC_USERINFO_METHOD"] = "header"

    overrides.pop("IDENTITY_OIDC_WEBFINGER_RESOURCE", None)
    if module_name == "oidcc-client-test-discovery-webfinger-acct":
        overrides["IDENTITY_OIDC_WEBFINGER_RESOURCE"] = (
            f"acct:{DEFAULT_ALIAS}.{module_name}@localhost.emobix.co.uk"
        )
    elif module_name == "oidcc-client-test-discovery-webfinger-url":
        overrides["IDENTITY_OIDC_WEBFINGER_RESOURCE"] = (
            f"{SUITE_PUBLIC_ORIGIN}/{DEFAULT_ALIAS}/{module_name}"
        )

    if "rotation" in module_name:
        overrides["IDENTITY_OIDC_JWKS_REFRESH_AFTER_USERINFO"] = "1"

    return overrides


def write_conformance_env(overrides: dict[str, str]) -> None:
    if not ENV_BASE.is_file():
        raise ConformanceError(f"Missing base env file: {ENV_BASE}")
    lines = ENV_BASE.read_text().splitlines()
    managed_keys = {
        "IDENTITY_OIDC_TOKEN_ENDPOINT_AUTH_METHOD",
        "IDENTITY_OIDC_USERINFO_METHOD",
        "IDENTITY_OIDC_WEBFINGER_RESOURCE",
        "IDENTITY_OIDC_JWKS_REFRESH_AFTER_USERINFO",
    }
    out: list[str] = []
    seen: set[str] = set()
    for line in lines:
        if not line.strip() or line.lstrip().startswith("#"):
            out.append(line)
            continue
        key = line.split("=", 1)[0]
        if key in managed_keys:
            if key in overrides:
                out.append(f"{key}={overrides[key]}")
                seen.add(key)
            continue
        out.append(line)
    for key, value in overrides.items():
        if key not in seen:
            out.append(f"{key}={value}")
    ENV_RUN.write_text("\n".join(out) + "\n")


def ensure_identity_running(*, restart: bool = False) -> None:
    """Start sigma-identity after the fake OP reaches WAITING."""
    identity_url = os.environ.get("CONFORMANCE_IDENTITY_URL", DEFAULT_IDENTITY_URL).rstrip("/")
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE

    if not restart:
        try:
            with urllib.request.urlopen(identity_url, context=ctx) as resp:
                if resp.status == 200:
                    logging.info("Identity already running at %s", identity_url)
                    return
        except (urllib.error.URLError, urllib.error.HTTPError):
            pass

    start_cmd = os.environ.get("CONFORMANCE_IDENTITY_START_CMD", DEFAULT_IDENTITY_START_CMD)
    logging.info("Starting identity: %s", start_cmd)
    subprocess.run(start_cmd, shell=True, check=True, cwd=IDENTITY_ROOT)
    time.sleep(float(os.environ.get("CONFORMANCE_IDENTITY_START_DELAY", "3")))

    deadline = time.monotonic() + float(os.environ.get("CONFORMANCE_IDENTITY_TIMEOUT", "120"))
    while time.monotonic() < deadline:
        try:
            with urllib.request.urlopen(identity_url, context=ctx) as resp:
                if resp.status == 200:
                    logging.info("Identity ready at %s", identity_url)
                    return
        except (urllib.error.URLError, urllib.error.HTTPError):
            time.sleep(2)
    raise ConformanceError(f"Identity did not become ready at {identity_url}")


def curl(args: list[str], *, fail_on_http_error: bool = True) -> None:
    flags = ["-sk"]
    if fail_on_http_error:
        flags.append("-f")
    subprocess.run(["curl", *flags, *args], check=True)


def trigger_discover() -> None:
    identity_origin = os.environ.get("CONFORMANCE_IDENTITY_ORIGIN", "https://localhost:3000")
    url = f"{identity_origin}/auth/conformance/discover"
    logging.info("Triggering RP discovery: %s", url)
    curl([url], fail_on_http_error=False)


def trigger_login_flow(*, scope: str | None = None, cookie_jar: str | None = None) -> None:
    """Drive the authorization code flow so the suite receives auth/token/userinfo."""
    identity_origin = os.environ.get("CONFORMANCE_IDENTITY_ORIGIN", "https://localhost:3000")
    cookie_jar = cookie_jar or os.environ.get(
        "CONFORMANCE_LOGIN_COOKIE_JAR", "/tmp/sigma-conformance-cookies.txt"
    )
    if os.path.isfile(cookie_jar):
        os.remove(cookie_jar)
    params = urllib.parse.urlencode(
        {
            "scope": scope
            or os.environ.get(
                "CONFORMANCE_LOGIN_SCOPE", "openid profile email offline_access"
            ),
            "redirect_uri": f"{identity_origin}/auth/callback",
            "app_uri": os.environ.get("CONFORMANCE_LOGIN_APP_URI", DEFAULT_LOGIN_APP_URI),
            "state": f"conformance-{int(time.time())}",
        }
    )
    login_url = f"{identity_origin}/auth/login?{params}"
    cmd = [
        "curl",
        "-skL",
        "-c",
        cookie_jar,
        "-b",
        cookie_jar,
        "-o",
        "/dev/null",
        login_url,
    ]
    logging.info("Triggering RP login flow")
    subprocess.run(cmd, check=True)


def trigger_refresh_flow() -> None:
    identity_origin = os.environ.get("CONFORMANCE_IDENTITY_ORIGIN", "https://localhost:3000")
    cookie_jar = os.environ.get(
        "CONFORMANCE_LOGIN_COOKIE_JAR", "/tmp/sigma-conformance-cookies.txt"
    )
    trigger_login_flow(scope="openid profile email offline_access")
    refresh_url = f"{identity_origin}/auth/refresh"
    logging.info("Triggering RP refresh: POST %s", refresh_url)
    subprocess.run(
        ["curl", "-sk", "-b", cookie_jar, "-c", cookie_jar, "-X", "POST", refresh_url],
        check=True,
    )


def trigger_logout_flow() -> None:
    identity_origin = os.environ.get("CONFORMANCE_IDENTITY_ORIGIN", "https://localhost:3000")
    cookie_jar = os.environ.get(
        "CONFORMANCE_LOGIN_COOKIE_JAR", "/tmp/sigma-conformance-cookies.txt"
    )
    trigger_login_flow(scope="openid profile email")
    params = urllib.parse.urlencode(
        {
            "app_uri": f"{identity_origin}/conformance/",
            "redirect_uri": f"{identity_origin}/auth/logoutcallback",
        }
    )
    logout_url = f"{identity_origin}/auth/logout?{params}"
    logging.info("Triggering RP logout flow")
    subprocess.run(
        ["curl", "-skL", "-b", cookie_jar, "-c", cookie_jar, "-o", "/dev/null", logout_url],
        check=True,
    )


def perform_module_action(action: str, module_name: str) -> None:
    cookie_jar = f"/tmp/sigma-conformance-{module_name}.cookies.txt"
    if action == "discover":
        trigger_discover()
    elif action == "refresh":
        trigger_refresh_flow()
    elif action == "logout":
        trigger_logout_flow()
    else:
        scope = None
        if "distributed-claims" in module_name:
            scope = "openid profile email"
        trigger_login_flow(scope=scope, cookie_jar=cookie_jar)


def load_config(
    config_path: Path,
    *,
    client_id: str,
    client_secret: str,
    version: str = "dev",
) -> tuple[dict, dict | None]:
    raw = config_path.read_text()
    substitutions = {
        "{CLIENT_ID}": json.dumps(client_id)[1:-1],
        "{CLIENT_SECRET}": json.dumps(client_secret)[1:-1],
        "{VERSION}": json.dumps(version)[1:-1],
    }
    for placeholder, value in substitutions.items():
        raw = raw.replace(placeholder, value)
    config = json.loads(raw)
    variant = config.pop("variant", None)
    return config, variant


def wait_for_module_settle(client: ConformanceClient, module_id: str) -> str:
    """Wait until the suite reports a final result and late HTTP has drained."""
    settle = float(os.environ.get("CONFORMANCE_MODULE_SETTLE", "20"))
    deadline = time.monotonic() + settle
    result = "UNKNOWN"
    while time.monotonic() < deadline:
        info = client.get_module_info(module_id)
        result = info.get("result", result) or result
        status = info.get("status", "")
        if result in PASSING_RESULTS.union({"FAILED", "INTERRUPTED"}):
            break
        if status in {"INTERRUPTED", "FAILED"}:
            break
        time.sleep(2)
    time.sleep(settle)
    info = client.get_module_info(module_id)
    return info.get("result", result) or result


def run_module(
    client: ConformanceClient,
    plan_id: str,
    module_name: str,
    *,
    module_timeout: int,
) -> dict:
    module_id = client.start_test_module(plan_id, module_name)
    action = module_action(module_name)
    overrides = module_env_overrides(module_name)
    write_conformance_env(overrides)

    deadline = time.monotonic() + module_timeout
    identity_started = False
    waiting_actions = 0
    result = "UNKNOWN"

    if os.environ.get("CONFORMANCE_ENSURE_IDENTITY"):
        try:
            client.wait_for_status(module_id, "WAITING", timeout=120)
        except ConformanceError:
            pass

    while time.monotonic() < deadline:
        info = client.get_module_info(module_id)
        status = info.get("status", "")
        result = info.get("result", result)

        if status in TERMINAL_STATES or result in PASSING_RESULTS.union(
            {"FAILED", "INTERRUPTED"}
        ):
            break

        if status == "FINISHED":
            time.sleep(3)
            info = client.get_module_info(module_id)
            result = info.get("result", result)
            if result in PASSING_RESULTS.union({"FAILED", "INTERRUPTED"}):
                break

        if status == "WAITING" and os.environ.get("CONFORMANCE_ENSURE_IDENTITY"):
            if not identity_started:
                restart = os.environ.get("CONFORMANCE_RESTART_IDENTITY", "1") not in (
                    "0",
                    "false",
                    "FALSE",
                )
                ensure_identity_running(restart=restart)
                identity_started = True
                perform_module_action(action, module_name)
                waiting_actions += 1
            elif (
                action == "login"
                and module_name == "oidcc-client-test-signing-key-rotation"
                and waiting_actions < 2
            ):
                # Full rotation module expects a second authorization after JWKS refresh.
                time.sleep(8)
                perform_module_action(action, module_name)
                waiting_actions += 1

        time.sleep(2)
    else:
        info = client.get_module_info(module_id)
        result = info.get("result", "FAILED")
        if result not in PASSING_RESULTS:
            raise ConformanceError(
                f"Module {module_id} timed out (last status: {info.get('status')})"
            )

    info = client.get_module_info(module_id)
    result = wait_for_module_settle(client, module_id)
    return {"name": module_name, "result": result, "module_id": module_id}


def run_plan(
    client: ConformanceClient,
    plan_name: str,
    config: dict,
    variant: dict | None,
    *,
    module_timeout: int,
    only_modules: list[str] | None = None,
) -> tuple[str, list[dict], bool]:
    try:
        plan_id = client.create_test_plan(plan_name, config, variant)
    except ConformanceError as e:
        if variant and "Variant" in str(e):
            logging.warning("Plan variant rejected by suite; retrying without variant: %s", e)
            plan_id = client.create_test_plan(plan_name, config, None)
        else:
            raise
    modules = client.get_plan_modules(plan_id)
    if only_modules:
        modules = [
            m
            for m in modules
            if (m.get("testModule") or m.get("name")) in only_modules
        ]

    results: list[dict] = []
    for module in modules:
        module_name = module.get("testModule") or module.get("name", "unknown")
        try:
            row = run_module(
                client,
                plan_id,
                module_name,
                module_timeout=module_timeout,
            )
        except ConformanceError as e:
            logging.error("%s: %s", module_name, e)
            row = {"name": module_name, "result": "FAILED", "module_id": ""}

        results.append(row)
        logging.info("[%s] %s", row["result"], module_name)

        if row["result"] not in PASSING_RESULTS and row.get("module_id"):
            log = format_module_log(client.get_module_log(row["module_id"]), verbose=True)
            if log:
                print(f"\n--- {module_name} ---\n{log}\n")

        time.sleep(float(os.environ.get("CONFORMANCE_MODULE_GAP", "8")))

    ok = all(r["result"] in PASSING_RESULTS for r in results)
    return plan_id, results, ok


def print_summary(plan_name: str, plan_id: str, results: list[dict], server: str) -> None:
    print("\n" + "=" * 72)
    print(f"Plan: {plan_name}")
    print(f"UI:   {server}/plan-detail.html?plan={plan_id}")
    for row in results:
        mark = "+" if row["result"] in PASSING_RESULTS else "!"
        result = row.get("result") or "UNKNOWN"
        print(f"  [{mark}] {result:<10} {row['name']}")
    print("=" * 72)


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    os.environ.setdefault("CONFORMANCE_ENSURE_IDENTITY", "1")
    os.environ.setdefault("CONFORMANCE_IDENTITY_START_CMD", DEFAULT_IDENTITY_START_CMD)
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--conformance-server",
        default=os.environ.get(
            "CONFORMANCE_SERVER", "https://localhost.emobix.co.uk:8443"
        ).rstrip("/"),
    )
    parser.add_argument("--client-id", default=os.environ.get("CLIENT_ID", "sigma-identity-conformance"))
    parser.add_argument(
        "--client-secret",
        default=os.environ.get("CLIENT_SECRET", "conformance-client-secret-not-for-production"),
    )
    parser.add_argument("--version", default=os.environ.get("CONFORMANCE_VERSION", "dev"))
    parser.add_argument(
        "--module-timeout",
        type=int,
        default=int(os.environ.get("CONFORMANCE_MODULE_TIMEOUT", "300")),
    )
    parser.add_argument("--plan", help="Run a single plan by name")
    parser.add_argument("--module", action="append", help="Run only these modules")
    parser.add_argument("--all", action="store_true", help="Run every plan in plans.json")
    args = parser.parse_args()

    client = ConformanceClient(server=args.conformance_server)
    client.wait_for_server()

    if args.all:
        plans = json.loads(PLANS_FILE.read_text())["plans"]
    elif args.plan:
        plans = [p for p in json.loads(PLANS_FILE.read_text())["plans"] if p["name"] == args.plan]
        if not plans:
            raise SystemExit(f"Unknown plan: {args.plan}")
    else:
        parser.error("Specify --all or --plan")

    all_ok = True
    aggregate: dict = {"plans": [], "conformance_server": args.conformance_server}

    for entry in plans:
        config_path = ROOT / entry["config"]
        config, file_variant = load_config(
            config_path,
            client_id=args.client_id,
            client_secret=args.client_secret,
            version=args.version,
        )
        variant = entry.get("variant") or file_variant
        plan_id, results, ok = run_plan(
            client,
            entry["name"],
            config,
            variant,
            module_timeout=args.module_timeout,
            only_modules=args.module,
        )
        print_summary(entry["name"], plan_id, results, args.conformance_server)
        aggregate["plans"].append(
            {"name": entry["name"], "plan_id": plan_id, "results": results, "ok": ok}
        )
        all_ok = all_ok and ok

    STATE_FILE.write_text(json.dumps(aggregate, indent=2) + "\n")
    sys.exit(0 if all_ok else 1)


if __name__ == "__main__":
    main()
