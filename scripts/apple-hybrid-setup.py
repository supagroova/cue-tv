#!/usr/bin/env python3
"""One-off Apple TV pairing and provisioning for the Android-hosted hybrid remote.

No always-on Mac service is installed. No action runs without an explicit operation flag.
Private config: host, port, name, input_id, input_port, home_bundle, tv_url, token_file.
Credentials are saved once in a new 0600 file, never printed or returned by the TV API.
"""
import argparse
import asyncio
import getpass
import ipaddress
import json
import os
from pathlib import Path
import sys
import urllib.request
from urllib.parse import urlencode, urlsplit

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'bridge'))
import appletv

def api(config, method, path, body=None):
    origin = urlsplit(config['tv_url'])
    address = ipaddress.ip_address(origin.hostname)
    if origin.scheme != 'http' or not address.is_private or address.is_loopback or origin.username or origin.password:
        raise ValueError('The setup API must be on the TV at a private home-network address')
    token = json.loads(Path(config['token_file']).read_text())['token']
    request = urllib.request.Request(config['tv_url'].rstrip('/') + path,
        data=json.dumps(body).encode() if body is not None else None, method=method,
        headers={'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token})
    with urllib.request.urlopen(request, timeout=35) as response:
        return json.load(response)

async def run(args):
    config = json.loads(args.config.read_text())
    if args.pair:
        if args.credentials.exists(): raise ValueError('Pairing file already exists. Choose a new filename to preserve it')
        link = appletv.Link(config['host'], int(config['port']))
        try:
            await link.open()
            state = await link.pair_setup_start()
            print('Apple TV is showing a four-digit pairing code.', flush=True)
            pin = await asyncio.to_thread(getpass.getpass, 'Code: ')
            if len(pin) != 4 or not pin.isdigit(): raise ValueError('Use the four digits currently displayed by Apple TV')
            credentials = await link.pair_setup_finish(state, pin)
            args.credentials.parent.mkdir(parents=True, exist_ok=True)
            descriptor = os.open(args.credentials, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
            with os.fdopen(descriptor, 'w') as handle: json.dump(credentials, handle)
            print('Apple TV pairing saved privately. No input or app was changed.', flush=True)
        finally: await link.close()
    elif args.provision:
        credentials = json.loads(args.credentials.read_text())
        payload = {key: config[key] for key in ('host', 'port', 'name', 'input_id', 'input_port', 'home_bundle')}
        payload['stremio_bundle'] = config.get('stremio_bundle', 'com.stremio.pal')
        payload['credentials'] = credentials
        result = api(config, 'POST', '/api/platform/apple/setup', payload)
        if result.get('apple_tv', {}).get('paired') is not True: raise RuntimeError('TV did not confirm the pairing')
        print('Android TV verified the Apple TV identity and saved the connection privately.', flush=True)
    elif args.connect_home:
        code = api(config, 'POST', '/api/platform/home-code', {})['code']
        credentials = json.loads(args.credentials.read_text())
        link = appletv.Link(config['host'], int(config['port']))
        try:
            await link.open(); await link.verify(credentials); await link.start_session()
            await link.ask('_hidC', {'_hBtS': 1, '_hidC': 13})
            await link.ask('_hidC', {'_hBtS': 2, '_hidC': 13})
            await link.ask('_launchApp', {'_bundleID': config['home_bundle']})
            uri = 'couchtv://pair?' + urlencode({'address': config['tv_url'], 'code': code})
            await link.ask('_launchApp', {'_urlS': uri})
            print('Opened TV on Apple TV with its own single-use library connection code.', flush=True)
        finally: await link.close()

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--config', type=Path, required=True)
    parser.add_argument('--credentials', type=Path, required=True)
    operation = parser.add_mutually_exclusive_group(required=True)
    operation.add_argument('--pair', action='store_true')
    operation.add_argument('--provision', action='store_true')
    operation.add_argument('--connect-home', action='store_true')
    args = parser.parse_args()
    try: asyncio.run(run(args))
    except (Exception, KeyboardInterrupt) as exc:
        # Network/auth errors can contain response bodies. Report only the exception type here.
        print('Setup did not complete (' + type(exc).__name__ + '). Existing pairings were preserved.', file=sys.stderr)
        sys.exit(1)
