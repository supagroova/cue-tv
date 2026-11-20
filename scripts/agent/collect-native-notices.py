#!/usr/bin/env python3
"""Retain notices for the locked native dependency graph, including build dependencies."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def reachable_packages(metadata):
    nodes = {node['id']: node for node in metadata['resolve']['nodes']}
    pending = [metadata['resolve']['root']]
    seen = set()
    while pending:
        key = pending.pop()
        if key in seen:
            continue
        seen.add(key)
        pending.extend(dep['pkg'] for dep in nodes[key]['deps']
                       if any(kind['kind'] != 'dev' for kind in dep['dep_kinds']))
    return [package for package in metadata['packages'] if package['id'] in seen]


def notice_inputs(package, overrides, root):
    source = Path(package['manifest_path']).parent
    files = set()
    for directory, children, names in os.walk(source, followlinks=False):
        children[:] = [name for name in children if name not in {'.git', 'target', 'node_modules', '.agent-work'}]
        parent = Path(directory)
        license_directory = any(part.lower() in {'licenses', 'licences'} for part in parent.relative_to(source).parts)
        for name in names:
            path = parent / name
            if license_directory or name.lower().startswith(('license', 'copying', 'notice', 'copyright')):
                if path.is_symlink():
                    raise ValueError(f'Notice symlink needs explicit review: {package["name"]}/{name}')
                files.add(path)
    if package.get('license_file'):
        files.add(source / package['license_file'])
    for record in overrides:
        if package['name'] not in record['packages']:
            continue
        if record.get('versions', {}).get(package['name']) != package['version']:
            raise ValueError(f"Notice override version mismatch: {package['name']}")
        identity = package.get('source') or str(source.relative_to(root))
        if record.get('resolved_sources', {}).get(package['name']) != identity:
            raise ValueError(f"Notice override source mismatch: {package['name']}")
        path = root / record['retained']
        if digest(path) != record['sha256']:
            raise ValueError(f"Notice override checksum mismatch: {package['name']}")
        files.add(path)
        if record.get('license_text'):
            files.add(root / record['license_text'])
    if not files:
        raise ValueError(f"No notice matched: {package['name']}@{package['version']}")
    return sorted(files)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path, help='New local output directory')
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    provenance = json.loads((ROOT / 'native/licenses/PROVENANCE.json').read_text())
    for record in provenance.get('standard_texts', []):
        if digest(ROOT / record['retained']) != record['sha256']:
            raise ValueError('Standard license checksum mismatch')
    packages = {}
    locks = {}
    for component in ('core', 'torrent'):
        manifest = f'native/cue-{component}/Cargo.toml'
        lock = ROOT / f'native/cue-{component}/Cargo.lock'
        before = digest(lock)
        metadata = json.loads(subprocess.check_output([
            'cargo', '+1.100.0', 'metadata', '--locked', '--offline', '--format-version', '1',
            '--filter-platform', 'aarch64-apple-tvos', '--manifest-path', manifest], cwd=ROOT))
        if digest(lock) != before:
            raise ValueError('Lockfile changed during dependency resolution')
        locks[str(lock.relative_to(ROOT))] = before
        for package in reachable_packages(metadata):
            packages[package['id']] = package
    records = []
    missing = []
    texts = output / 'texts'
    texts.mkdir()
    for package in sorted(packages.values(), key=lambda p: (p['name'], p['version'], p['id'])):
        try:
            files = notice_inputs(package, provenance['records'], ROOT)
        except ValueError as error:
            missing.append(str(error))
            continue
        notices = []
        for path in files:
            sha = digest(path)
            destination = texts / (sha + '.txt')
            if not destination.exists():
                destination.write_bytes(path.read_bytes())
            package_root = Path(package['manifest_path']).parent
            origin = str(path.relative_to(package_root)) if path.is_relative_to(package_root) else str(path.relative_to(ROOT))
            notices.append({'source_path': origin, 'sha256': sha, 'text': 'texts/' + destination.name})
        source = package.get('source') or str(Path(package['manifest_path']).parent.relative_to(ROOT))
        records.append({'name': package['name'], 'version': package['version'], 'source': source,
                        'declared_license': package.get('license'), 'notices': notices})
    result = {'schema': 1, 'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
              'source_dirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=ROOT)),
              'collector_sha256': digest(Path(__file__)),
              'lockfiles': locks, 'provenance_sha256': digest(ROOT / 'native/licenses/PROVENANCE.json'),
              'scope': 'tvOS non-dev dependency graph including build dependencies; not exact binary linkage',
              'limitations': ['Filename/directory-based source notice discovery plus reviewed overrides; embedded source headers still need audit',
                             'Does not inventory VLCKit components or establish license compliance'],
              'packages': records, 'missing': missing}
    (output / 'manifest.json').write_text(json.dumps(result, indent=2) + '\n')
    if missing:
        raise SystemExit('Notice collection incomplete: ' + '; '.join(missing))
    print(f'Retained notices for {len(records)} packages at {output}')


if __name__ == '__main__':
    main()
