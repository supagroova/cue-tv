#!/usr/bin/env python3
"""Serve only the generated episode fixture over local TLS, with byte-range support."""
import argparse
import http.server
import re
import ssl
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument("--cert", required=True)
parser.add_argument("--key", required=True)
parser.add_argument("--port", type=int, required=True)
args = parser.parse_args()
media = Path(__file__).resolve().parents[2] / "apple-tv/Resources/PlaybackLab/episode01.mp4"

class Fixture(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/episode01.mp4":
            self.send_error(404)
            return
        size = media.stat().st_size
        start, end = 0, size - 1
        requested = self.headers.get("Range")
        if requested:
            match = re.fullmatch(r"bytes=(\d+)-(\d*)", requested)
            if not match:
                self.send_error(416)
                return
            start = int(match[1])
            end = min(int(match[2]), size - 1) if match[2] else size - 1
            if start > end:
                self.send_error(416)
                return
        self.send_response(206 if requested else 200)
        self.send_header("Content-Type", "video/mp4")
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Length", str(end - start + 1))
        if requested:
            self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.end_headers()
        try:
            with media.open("rb") as source:
                source.seek(start)
                remaining = end - start + 1
                while remaining:
                    chunk = source.read(min(65536, remaining))
                    if not chunk:
                        break
                    self.wfile.write(chunk)
                    remaining -= len(chunk)
        except (BrokenPipeError, ConnectionResetError, ssl.SSLError):
            pass
    def log_message(self, *_):
        pass

server = http.server.ThreadingHTTPServer(("127.0.0.1", args.port), Fixture)
context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
context.load_cert_chain(args.cert, args.key)
server.socket = context.wrap_socket(server.socket, server_side=True)
print(f"Owned HTTPS fixture listening on loopback port {args.port}", flush=True)
server.serve_forever()
