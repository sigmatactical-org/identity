#!/usr/bin/env python3
"""Minimal HTTP echo server for identity E2E proxy tests."""

from http.server import BaseHTTPRequestHandler, HTTPServer


class EchoHandler(BaseHTTPRequestHandler):
    def log_message(self, _format, *_args):
        return

    def do_GET(self):
        self._respond()

    def do_POST(self):
        self._respond()

    def do_PUT(self):
        self._respond()

    def do_PATCH(self):
        self._respond()

    def do_DELETE(self):
        self._respond()

    def do_OPTIONS(self):
        self.send_response(204)
        self.end_headers()

    def _respond(self):
        length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(length) if length else b""
        lines = [f"{self.command} request at {self.path}"]
        for key, value in self.headers.items():
            lines.append(f"{key.lower()}: {value}")
        if body:
            lines.append(body.decode("utf-8", errors="replace"))
        payload = "\n".join(lines).encode()
        self.send_response(200)
        self.send_header("Content-Type", "text/plain; charset=utf-8")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)


if __name__ == "__main__":
    HTTPServer(("0.0.0.0", 3000), EchoHandler).serve_forever()
