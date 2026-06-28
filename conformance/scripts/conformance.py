# SPDX-License-Identifier: Apache-2.0 OR MIT
"""API client for the OpenID Foundation conformance suite."""

from __future__ import annotations

import json
import logging
import ssl
import time
import urllib.error
import urllib.parse
import urllib.request
from typing import Any

log = logging.getLogger(__name__)

DEFAULT_SERVER = "https://localhost.emobix.co.uk:8443"
POLL_INTERVAL = 2.0
DEFAULT_TIMEOUT = 300
PASSING_RESULTS = {"PASSED", "WARNING", "REVIEW", "SKIPPED"}


class ConformanceError(Exception):
    """Unexpected conformance API response."""


def _ssl_context() -> ssl.SSLContext:
    ctx = ssl.create_default_context()
    ctx.check_hostname = False
    ctx.verify_mode = ssl.CERT_NONE
    return ctx


class ConformanceClient:
    def __init__(
        self,
        server: str = DEFAULT_SERVER,
        token: str = "",
        verify_ssl: bool = False,
    ) -> None:
        self.server = server.rstrip("/")
        self._token = token
        self._ssl_ctx = None if verify_ssl else _ssl_context()
        self._host_header = self._host_header_for(server)

    @staticmethod
    def _host_header_for(server: str) -> str | None:
        parsed = urllib.parse.urlparse(server)
        if parsed.hostname == "127.0.0.1":
            return "localhost.emobix.co.uk"
        return None

    def _build_url(self, path: str, params: dict | None = None) -> str:
        parsed = urllib.parse.urlparse(self.server)
        if parsed.hostname == "127.0.0.1":
            port = parsed.port or 8443
            base = f"{parsed.scheme}://127.0.0.1:{port}"
        else:
            base = self.server
        url = f"{base}{path}"
        if params:
            url = f"{url}?{urllib.parse.urlencode(params)}"
        return url

    def _prepare_request(self, url: str, headers: dict[str, str] | None = None) -> urllib.request.Request:
        hdrs = dict(headers or {})
        if self._host_header:
            hdrs.setdefault("Host", self._host_header)
        if self._token:
            hdrs["Authorization"] = f"Bearer {self._token}"
        return urllib.request.Request(url, headers=hdrs)

    def _get(self, path: str, params: dict | None = None) -> Any:
        return json.loads(self._get_bytes(path, params=params))

    def _get_bytes(self, path: str, params: dict | None = None) -> bytes:
        url = self._build_url(path, params)
        req = self._prepare_request(url)
        try:
            with urllib.request.urlopen(req, context=self._ssl_ctx) as resp:
                return resp.read()
        except urllib.error.HTTPError as e:
            raise ConformanceError(
                f"GET {path} failed: HTTP {e.code}: {e.read().decode()}"
            ) from e

    def _post(
        self, path: str, params: dict | None = None, body: Any = None
    ) -> Any:
        url = self._build_url(path, params)
        headers: dict[str, str] = {"Content-Type": "application/json"}
        data = json.dumps(body).encode() if body is not None else None
        if self._host_header:
            headers["Host"] = self._host_header
        if self._token:
            headers["Authorization"] = f"Bearer {self._token}"
        req = urllib.request.Request(url, data=data, method="POST", headers=headers)
        try:
            with urllib.request.urlopen(req, context=self._ssl_ctx) as resp:
                return json.loads(resp.read())
        except urllib.error.HTTPError as e:
            raise ConformanceError(
                f"POST {path} failed: HTTP {e.code}: {e.read().decode()}"
            ) from e

    def wait_for_server(self, timeout: float = 180) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            try:
                url = self._build_url("/")
                req = self._prepare_request(url)
                with urllib.request.urlopen(req, context=self._ssl_ctx) as resp:
                    if resp.status in (200, 302):
                        log.info("Conformance suite is up at %s", self.server)
                        return
            except urllib.error.HTTPError as e:
                if e.code in (200, 302, 401, 403):
                    log.info("Conformance suite is up at %s (HTTP %s)", self.server, e.code)
                    return
            except Exception as e:
                log.debug("Waiting for conformance suite: %s", e)
                time.sleep(3)
        raise ConformanceError(f"Conformance suite not ready after {timeout}s")

    def create_test_plan(
        self,
        plan_name: str,
        config: dict[str, Any],
        variant: dict[str, str] | None = None,
    ) -> str:
        params: dict[str, str] = {"planName": plan_name}
        if variant:
            params["variant"] = json.dumps(variant)
        log.info("Creating test plan %s", plan_name)
        data = self._post("/api/plan", params=params, body=config)
        plan_id = data.get("id") or data.get("plan", {}).get("id")
        if not plan_id:
            raise ConformanceError(f"No plan ID in create response: {data}")
        log.info("Created plan %s (%s)", plan_name, plan_id)
        return plan_id

    def get_plan_modules(self, plan_id: str) -> list[dict[str, Any]]:
        plan = self._get(f"/api/plan/{plan_id}")
        modules = plan.get("modules", [])
        if not modules:
            raise ConformanceError(f"Plan {plan_id} has no modules")
        return modules

    def start_test_module(self, plan_id: str, module_name: str) -> str:
        data = self._post(
            "/api/runner", params={"plan": plan_id, "test": module_name}
        )
        module_id = data.get("id")
        if not module_id:
            raise ConformanceError(f"No module ID in start response: {data}")
        return module_id

    def get_module_info(self, module_id: str) -> dict[str, Any]:
        return self._get(f"/api/info/{module_id}")

    def get_module_log(self, module_id: str) -> list[dict[str, Any]]:
        return self._get(f"/api/log/{module_id}")

    def wait_for_state(
        self,
        module_id: str,
        terminal_states: set[str] | None = None,
        timeout: float = DEFAULT_TIMEOUT,
    ) -> dict[str, Any]:
        if terminal_states is None:
            terminal_states = {"FINISHED", "INTERRUPTED", "FAILED"}
        deadline = time.monotonic() + timeout
        while True:
            info = self.get_module_info(module_id)
            status = info.get("status", "")
            if status in terminal_states:
                return info
            if time.monotonic() >= deadline:
                raise ConformanceError(
                    f"Module {module_id} timed out (last status: {status})"
                )
            time.sleep(POLL_INTERVAL)

    def wait_for_status(
        self,
        module_id: str,
        status: str,
        *,
        timeout: float = DEFAULT_TIMEOUT,
    ) -> dict[str, Any]:
        return self.wait_for_state(module_id, terminal_states={status}, timeout=timeout)


def format_module_log(entries: list[dict], verbose: bool = False) -> str:
    lines: list[str] = []
    for entry in entries:
        result = entry.get("result", "")
        src = entry.get("src", "")
        msg = entry.get("msg", "")
        if verbose or result not in ("SUCCESS", "INFO", ""):
            lines.append(f"[{result}] {src}: {msg}")
        http = entry.get("http")
        if http:
            lines.append(f"  http: {http}")
    return "\n".join(lines)
