#!/usr/bin/env python3
"""Build and inventory an unsigned candidate; never asserts device qualification."""
import argparse
import hashlib
import json
import os
import plistlib
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def run(*args):
    return subprocess.check_output(args, cwd=ROOT, text=True).strip()


def checksum(path):
    digest = hashlib.sha256()
    with path.open('rb') as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b''):
            digest.update(chunk)
    return digest.hexdigest()


def inventory(directory):
    result = {}
    for path in sorted(directory.rglob('*')):
        if path.is_symlink():
            raise ValueError(f'Symlink requires explicit inventory handling: {path.name}')
        if path.is_file():
            result[str(path.relative_to(directory))] = {'bytes': path.stat().st_size, 'sha256': checksum(path)}
    if not result:
        raise ValueError('Empty artifact inventory')
    return result


def source_identity():
    if run('git', 'status', '--porcelain', '--untracked-files=normal'):
        raise ValueError('Commit or isolate source changes before recording a candidate')
    submodules = run('git', 'submodule', 'status', '--recursive').splitlines()
    # Leading space means checked out at the parent commit; +, - and U do not.
    raw = subprocess.check_output(['git', 'submodule', 'status', '--recursive'], cwd=ROOT, text=True)
    if any(line and not line.startswith(' ') for line in raw.splitlines()):
        raise ValueError('Submodule checkout differs from pinned source')
    return {'commit': run('git', 'rev-parse', 'HEAD'), 'submodules': submodules, 'clean': True}


def require_unchanged(before, after):
    if before != after:
        raise ValueError('Source changed during build; no candidate manifest issued')


def build_environment(environment):
    # Retained locally with the candidate. Do not dump credentials or unrelated env.
    keys = ('CARGO_PROFILE_DEV_DEBUG', 'CARGO_PROFILE_DEV_OPT_LEVEL',
            'CARGO_PROFILE_TEST_DEBUG', 'CARGO_INCREMENTAL', 'CARGO_BUILD_JOBS',
            'CARGO_TARGET_DIR', 'RUSTFLAGS', 'CARGO_ENCODED_RUSTFLAGS',
            'RUSTC_WRAPPER', 'RUSTC_WORKSPACE_WRAPPER', 'CC', 'CXX', 'CFLAGS',
            'CXXFLAGS', 'LDFLAGS', 'AR', 'SDKROOT', 'DEVELOPER_DIR',
            'MACOSX_DEPLOYMENT_TARGET', 'TVOS_DEPLOYMENT_TARGET')
    return {key: environment.get(key) for key in keys}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', required=True, type=Path, help='New local evidence directory; must not exist')
    args = parser.parse_args()
    before = source_identity()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    private_names = ('apple-tv/Config/Signing.mine.xcconfig', 'apple-tv/TVConfig.plist')
    private_before = {name: checksum(ROOT / name) for name in private_names}
    environment = build_environment(os.environ)
    command = ['make', 'apple-release-build']
    tools = {'xcode': run('xcodebuild', '-version'), 'sdk': run('xcrun', '--sdk', 'appletvos', '--show-sdk-version'),
             'rust': run('rustc', '+1.100.0', '--version'), 'xcodegen': run('xcodegen', '--version')}
    with (output / 'build.log').open('w') as log:
        result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT)
    receipt = {'command': command, 'exit_code': result.returncode, 'source_before': before,
               'at': datetime.now(timezone.utc).isoformat(), 'toolchain': tools, 'build_environment': environment}
    (output / 'build-receipt.json').write_text(json.dumps(receipt, indent=2) + '\n')
    if result.returncode:
        raise SystemExit(result.returncode)
    require_unchanged(before, source_identity())
    bundle = ROOT / '.agent-work/DerivedData/Build/Products/Release-appletvos/SittingRoomTV.app'
    info = plistlib.loads((bundle / 'Info.plist').read_bytes())
    executable = bundle / info['CFBundleExecutable']
    if not executable.is_file():
        raise ValueError('Declared executable missing')
    signature = subprocess.run(['codesign', '-d', str(bundle)], capture_output=True, text=True)
    if signature.returncode == 0:
        raise ValueError('Expected unsigned artifact; signed candidate needs separate recording')
    if 'code object is not signed at all' not in signature.stderr:
        raise ValueError('Could not establish unsigned artifact state')
    inputs = ['native/cue-core/Cargo.lock', 'native/cue-torrent/Cargo.lock',
              'apple-tv/SittingRoomTV.xcodeproj/project.xcworkspace/xcshareddata/swiftpm/Package.resolved']
    dependencies = {}
    for name in inputs:
        path = ROOT / name
        dependencies[name] = {'sha256': checksum(path)}
        destination = output / 'dependency-inputs' / name
        destination.parent.mkdir(parents=True, exist_ok=True)
        destination.write_bytes(path.read_bytes())
    archives = {}
    for name in ('libcue_core.a', 'libcue_torrent.a'):
        path = ROOT / '.agent-work/native-target/aarch64-apple-tvos/debug' / name
        archives[name] = {'sha256': checksum(path), 'bytes': path.stat().st_size}
    private_inputs = {name: checksum(ROOT / name) for name in private_names}
    require_unchanged(private_before, private_inputs)
    subprocess.run([sys.executable, str(ROOT / 'scripts/agent/collect-native-notices.py'),
                    '--output', str(output / 'native-notices')], cwd=ROOT, check=True)
    notice_manifest = output / 'native-notices/manifest.json'
    manifest = {'schema': 1, 'source': before, 'command': command, 'toolchain': tools,
                'configuration': 'Xcode Release; Cargo development profile', 'signed': False,
                'build_environment': environment,
                'bundle': {key: info.get(key) for key in ('CFBundleIdentifier', 'CFBundleVersion', 'CFBundleShortVersionString', 'CFBundleExecutable')},
                'dependencies': dependencies, 'native_archives': archives,
                'native_notices': {'manifest': 'native-notices/manifest.json', 'sha256': checksum(notice_manifest)},
                'private_input_sha256': private_inputs, 'files': inventory(bundle),
                'linked_libraries': run('otool', '-L', str(executable)).splitlines()[1:],
                'qualification': 'Inventory only; not a signed or device-qualified release',
                'limitations': ['Incremental build; clean rebuild repeatability still requires independent evidence',
                               'Private build inputs are hashed, not copied; retain them locally to reproduce',
                               'Build environment records common overrides, not arbitrary custom tool wrappers/configuration',
                               'Dependency inventory does not establish complete license/source obligations']}
    require_unchanged(before, source_identity())
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
    print(f'Unsigned candidate inventory: {output / "manifest.json"}')


if __name__ == '__main__':
    main()
