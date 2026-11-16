#!/usr/bin/env python3
"""Provision the TV's Google device-OAuth registration without printing credentials.

Use a dedicated Google Cloud project, YouTube Data API v3 enabled, and a client whose
type is TVs and Limited Input devices. Each already-paired phone then authorises its
own read-only connection in Google's browser page. No Google user token is copied here.
"""
import argparse
import ipaddress
import json
from pathlib import Path
import sys
import urllib.error
import urllib.request
from urllib.parse import urlsplit

ROOT = Path(__file__).resolve().parents[1]

def run(args):
    origin = urlsplit(args.tv_url)
    address = ipaddress.ip_address(origin.hostname)
    if origin.scheme != 'http' or address.version != 4 or not address.is_private or address.is_loopback or origin.username or origin.password or origin.path not in ('', '/') or origin.query or origin.fragment:
        raise ValueError('Use the TV’s private home-network address')
    source = json.loads(args.client.read_text())
    client = source.get('installed')
    if not isinstance(client, dict) or not client.get('client_id') or not client.get('client_secret'):
        raise ValueError('Download the JSON for the TVs and Limited Input devices OAuth client')
    token = json.loads(args.tv_token_file.read_text())['token']
    payload = {key: client[key] for key in ('client_id', 'client_secret')}
    request = urllib.request.Request(args.tv_url.rstrip('/') + '/api/youtube/setup',
        data=json.dumps(payload).encode(), method='POST',
        headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token})
    with urllib.request.urlopen(request, timeout=20) as response:
        result = json.load(response)
    if result.get('configured') is not True: raise RuntimeError('The TV did not confirm setup')
    print('TV confirmed the YouTube app registration. Each phone can connect its own account.')

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--client', required=True, type=Path)
    parser.add_argument('--tv-url', required=True)
    parser.add_argument('--tv-token-file', type=Path, default=ROOT / 'dist/private/tv-setup-token.json')
    arguments = parser.parse_args()
    try: run(arguments)
    except urllib.error.HTTPError as exc:
        print('YouTube setup was not confirmed by the TV (HTTP ' + str(exc.code) + '). Existing account connections were preserved.', file=sys.stderr)
        sys.exit(1)
    except (Exception, KeyboardInterrupt) as exc:
        print('YouTube setup did not complete (' + type(exc).__name__ + '). No credentials were printed.', file=sys.stderr)
        sys.exit(1)
