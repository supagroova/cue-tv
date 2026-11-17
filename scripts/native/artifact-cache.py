#!/usr/bin/env python3
"""Exact-source simulator artifacts; dependency caches remain Cargo-owned."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = {
    'libcue_core.a': 'aarch64-apple-tvos-sim/debug/libcue_core.a',
    'libcue_torrent.a': 'aarch64-apple-tvos-sim/debug/libcue_torrent.a',
    'seed': 'debug/examples/seed',
}

def fingerprint():
    paths = subprocess.check_output(['git', 'ls-files', '--recurse-submodules', '-z', '--',
        'native', 'scripts/native', 'apple-tv/project.yml', '.github/workflows/apple-tv.yml',
        '.github/actions/setup-apple/action.yml'], cwd=ROOT).split(b'\0')
    digest = hashlib.sha256(b'cue-sim-artifacts-v1-rust1.100.0-xcode26.6-arm64-dev-debug0-incremental0')
    index = subprocess.check_output(['git', 'ls-files', '--stage', '-z', '--', 'native'], cwd=ROOT)
    for entry in index.split(b'\0'):
        if entry.startswith(b'160000 '):
            digest.update(entry + b'\0')
    for raw in sorted(filter(None, paths)):
        path = ROOT / os.fsdecode(raw)
        digest.update(raw + b'\0')
        digest.update(path.read_bytes())
        digest.update(b'\0')
    return digest.hexdigest()

def transfer(mode, key):
    if key != fingerprint():
        raise ValueError('Native artifact key does not match current sources')
    cache = ROOT / '.agent-work/native-artifacts'
    target = ROOT / '.agent-work/native-target'
    if mode == 'save':
        cache.mkdir(parents=True, exist_ok=True)
        manifest = {'key': key, 'files': {}}
        for name, relative in ARTIFACTS.items():
            source = target / relative
            if not source.is_file() or source.stat().st_size == 0:
                raise ValueError(f'Missing native artifact: {source}')
            shutil.copy2(source, cache / name)
            manifest['files'][name] = hashlib.sha256(source.read_bytes()).hexdigest()
        (cache / 'manifest.json').write_text(json.dumps(manifest))
    else:
        manifest = json.loads((cache / 'manifest.json').read_text())
        if manifest['key'] != key:
            raise ValueError('Cached native artifact source mismatch')
        # Validate the complete set before copying any artifact into build outputs.
        for name in ARTIFACTS:
            source = cache / name
            if not source.is_file() or source.stat().st_size == 0 or hashlib.sha256(source.read_bytes()).hexdigest() != manifest['files'][name]:
                raise ValueError(f'Invalid native artifact: {name}')
        for name, relative in ARTIFACTS.items():
            destination = target / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(cache / name, destination)
        (target / ARTIFACTS['seed']).chmod(0o755)

if __name__ == '__main__':
    if sys.argv[1] == 'key':
        print(fingerprint())
    else:
        transfer(sys.argv[1], sys.argv[2])
