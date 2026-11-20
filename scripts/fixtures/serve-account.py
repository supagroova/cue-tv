#!/usr/bin/env python3
"""Loopback-only synthetic account API for the opt-in tvOS recovery journey."""
import argparse
import json
import threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

MANIFEST = {"id": "org.cue.account-journey", "version": "1.0.0", "name": "Account journey",
            "description": "Owned account fixture", "resources": ["catalog", "meta"],
            "types": ["movie"], "catalogs": [{"id": "owned", "type": "movie", "name": "Owned"}]}
USER = {"_id": "fixture-user", "email": "fixture@example.test", "lastModified": "2026-01-01T00:00:00Z",
        "dateRegistered": "2026-01-01T00:00:00Z", "premium_expire": None,
        "gdpr_consent": {"tos": True, "privacy": True, "marketing": False, "from": None}}

class Handler(BaseHTTPRequestHandler):
    counts = {"accepted_logins": 0, "rejected_logins": 0, "accepted_uploads": 0, "rejected_uploads": 0}
    lock = threading.Lock()

    def log_message(self, *_):
        pass  # Requests contain synthetic credentials; never log their bodies.

    def reply(self, body, status=200):
        payload = json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.send_header("Connection", "close")
        self.end_headers()
        self.wfile.write(payload)

    def do_GET(self):
        if self.path == "/probe":
            with self.lock:
                self.reply(dict(self.counts))
        elif self.path == "/fixture/catalog/movie/owned.json":
            self.reply({"metas": [{"id": "account-title", "type": "movie", "name": "Owned account title"}]})
        elif self.path == "/fixture/meta/movie/account-title.json":
            self.reply({"meta": {"id": "account-title", "type": "movie", "name": "Owned account title"}})
        else:
            self.reply({}, 404)

    def do_POST(self):
        try:
            length = int(self.headers.get("Content-Length", "0"))
            if not 0 < length <= 65536:
                raise ValueError()
            value = json.loads(self.rfile.read(length))
            if not isinstance(value, dict):
                raise ValueError()
        except (ValueError, json.JSONDecodeError):
            self.reply({}, 400)
            return
        if self.path == "/api/login":
            password = value.get("password")
            if value.get("email") != USER["email"] or password not in ("fixture-password", "replacement-password"):
                with self.lock:
                    self.counts["rejected_logins"] += 1
                self.reply({"error": {"code": 1, "message": "Synthetic rejection"}})
                return
            with self.lock:
                self.counts["accepted_logins"] += 1
            token = "fixture-token" if password == "fixture-password" else "replacement-token"
            self.reply({"result": {"authKey": token, "user": USER}})
        elif self.path == "/api/addonCollectionGet":
            self.reply({"result": {"addons": [{"transportUrl": "https://api.strem.io/fixture/manifest.json", "manifest": MANIFEST}],
                                   "lastModified": "2026-01-01T00:00:00Z"}})
        elif self.path == "/api/datastorePut":
            changes = value.get("changes", [])
            if value.get("collection") != "libraryItem" or len(changes) != 1 or changes[0].get("_id") != "account-title":
                self.reply({}, 400)
                return
            accepted = value.get("authKey") == "replacement-token"
            with self.lock:
                self.counts["accepted_uploads" if accepted else "rejected_uploads"] += 1
            self.reply({"result": {"success": True}} if accepted else {}, 200 if accepted else 401)
        elif self.path.startswith("/api/datastore"):
            self.reply({"result": []})
        elif self.path == "/api/addonCollectionSet":
            self.reply({"result": {"success": True}})
        else:
            self.reply({}, 404)

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port", type=int, default=42429)
    args = parser.parse_args()
    server = ThreadingHTTPServer(("127.0.0.1", args.port), Handler)
    print("Synthetic account fixture ready", flush=True)
    server.serve_forever()
