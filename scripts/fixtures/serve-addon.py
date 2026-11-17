#!/usr/bin/env python3
"""Owned Stremio add-on fixture; loopback only, no external media or accounts."""
import json
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import unquote, urlsplit

MANIFEST = {
    "id": "org.cue.journey", "version": "1.0.0", "name": "Cue journey fixture",
    "description": "Owned simulator navigation fixture", "types": ["series"],
    "resources": ["catalog", {"name": "meta", "types": ["series"], "idPrefixes": ["cue-owned-"]},
                  {"name": "stream", "types": ["series"], "idPrefixes": ["cue-owned-"]}],
    "catalogs": [{"id": "owned", "type": "series", "name": "Owned series", "extra": [{"name": "search"}]}],
}

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        path = unquote(urlsplit(self.path).path)
        if path == "/captions.vtt":
            payload = b"WEBVTT\n\n00:00:00.000 --> 00:00:12.000\nOWNED CAPTION\n"
            self.send_response(200)
            self.send_header("Content-Type", "text/vtt")
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        if path == "/announce":
            # Compact peer list points only at our owned local seeder. No HTTP media.
            payload = b"d8:intervali30e5:peers6:" + bytes([127, 0, 0, 1]) + (42428).to_bytes(2, "big") + b"e"
            self.send_response(200)
            self.send_header("Content-Length", str(len(payload)))
            self.end_headers()
            self.wfile.write(payload)
            return
        if path == "/manifest.json":
            body = MANIFEST
        elif path.startswith("/catalog/series/owned") and path.endswith(".json"):
            body = {"metas": [{"id": f"cue-owned-{n}", "name": f"Owned series {n}", "type": "series"} for n in (1, 2, 3)]}
        elif path.startswith("/meta/series/cue-owned-") and path.endswith(".json"):
            identity = path.rsplit("/", 1)[1][:-5]
            body = {"meta": {"id": identity, "type": "series", "name": f"Owned series {identity[-1]}",
                "description": "Synthetic project media", "videos": [
                    {"id": f"{identity}:{season}:{episode}", "title": f"Episode {episode}", "season": season, "episode": episode,
                     "released": "2024-01-01T00:00:00.000Z"} for season in (1, 2) for episode in (1, 2)]}}
        elif path.startswith("/stream/series/cue-owned-") and path.endswith(".json"):
            body = {"streams": [{"name": "Owned peer fixture " + path.rsplit("/", 1)[1][:-5], "infoHash": "66848a1f4b3f287a33b10509f20da4a086a7910d",
                                  "subtitles": [{"id": "owned", "lang": "eng", "label": "Owned English", "url": "http://127.0.0.1:42427/captions.vtt"}],
                                  "fileIdx": 1 if path.endswith(":2.json") else 0, "sources": ["tracker:http://127.0.0.1:42427/announce"]}]}
        else:
            self.send_error(404)
            return
        payload = json.dumps(body).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *_):
        pass

if __name__ == "__main__":
    ThreadingHTTPServer(("127.0.0.1", 42427), Handler).serve_forever()
