#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0 OR MIT
"""Bootstrap the conformance suite fake OP for alias sigma-identity."""

from __future__ import annotations

import argparse
import json
import logging
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path(__file__).resolve().parent))

from conformance import ConformanceClient, ConformanceError  # noqa: E402
from run import load_config  # noqa: E402

STATE_FILE = ROOT / ".bootstrap-plan.json"
DEFAULT_CONFIG = ROOT / "config" / "oidcc-client-test.json"
DEFAULT_PLAN = "oidcc-client-test-plan"
DEFAULT_VARIANT = {
    "client_auth_type": "client_secret_post",
    "response_type": "code",
    "client_registration": "static_client",
    "request_type": "plain_http_request",
    "response_mode": "default",
}


def main() -> None:
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    parser = argparse.ArgumentParser(description="Bootstrap conformance fake OP issuer")
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
    args = parser.parse_args()

    client = ConformanceClient(server=args.conformance_server)
    client.wait_for_server()

    config, variant = load_config(
        DEFAULT_CONFIG,
        client_id=args.client_id,
        client_secret=args.client_secret,
        version=os.environ.get("CONFORMANCE_VERSION", "dev"),
    )
    variant = variant or DEFAULT_VARIANT

    plan_id = client.create_test_plan(DEFAULT_PLAN, config, variant)
    issuer = f"{args.conformance_server.rstrip('/')}/test/a/sigma-identity/"
    state = {
        "plan_id": plan_id,
        "plan_name": DEFAULT_PLAN,
        "issuer": issuer,
        "alias": "sigma-identity",
        "conformance_server": args.conformance_server,
    }
    STATE_FILE.write_text(json.dumps(state, indent=2) + "\n")
    print(json.dumps(state, indent=2))
    print(f"\nFake OP issuer ready: {issuer}", file=sys.stderr)


if __name__ == "__main__":
    try:
        main()
    except ConformanceError as e:
        print(f"ERROR: {e}", file=sys.stderr)
        sys.exit(1)
