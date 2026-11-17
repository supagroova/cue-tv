#!/usr/bin/env python3
"""Record reproducible task verification, including failures, without marking tasks done."""
import argparse
import hashlib
import json
import re
import subprocess
import sys
import uuid
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXCLUDED = {'.git', '.agent-work', '__pycache__', 'DerivedData', 'build', 'target', 'node_modules'}


def identity(root):
    top = subprocess.run(['git', '-C', str(root), 'rev-parse', '--show-toplevel'], capture_output=True, text=True)
    checkout = top.returncode == 0 and Path(top.stdout.strip()).resolve() == root.resolve()
    head = subprocess.run(['git', '-C', str(root), 'rev-parse', 'HEAD'], capture_output=True, text=True) if checkout else None
    if checkout:
        listed = subprocess.run(['git', '-C', str(root), 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], capture_output=True, check=True)
        paths = [root / p for p in set(listed.stdout.decode().split('\0')) if p]
    else:
        paths = list(root.rglob('*'))
    digest = hashlib.sha256()
    for p in sorted(paths):
        parts = p.relative_to(root).parts
        if any(x in EXCLUDED or x.endswith('.xcodeproj') for x in parts):
            continue
        if p.name.startswith('.env') or p.name in {'Signing.mine.xcconfig', 'TVConfig.plist'} or p.suffix.lower() in {'.p12', '.p8', '.mobileprovision', '.provisionprofile'}:
            continue
        if p.is_symlink():
            data = str(p.readlink()).encode()
        elif p.is_file():
            data = p.read_bytes()
        else:
            continue
        digest.update(str(p.relative_to(root)).encode() + b'\0' + data + b'\0')
    return {'head': head.stdout.strip() if head and head.returncode == 0 else None, 'content_sha256': digest.hexdigest()}


def task_dir(root, name):
    if not re.fullmatch(r'[a-zA-Z0-9][a-zA-Z0-9_-]{0,79}', name):
        raise ValueError('task ID must be 1–80 letters, digits, hyphens or underscores')
    return root / '.agent-work/tasks' / name


def start(root, name):
    directory = task_dir(root, name)
    directory.mkdir(parents=True, exist_ok=True)
    with (directory / 'start.json').open('x') as f:
        json.dump({'task': name, 'started': datetime.now(timezone.utc).isoformat(), **identity(root)}, f, indent=2)
    return directory


def verify(root, name, command):
    directory = task_dir(root, name)
    if not (directory / 'start.json').exists():
        raise ValueError('start the task before recording verification')
    if not command:
        raise ValueError('verification needs a command after --')
    run_id = uuid.uuid4().hex
    log = directory / (run_id + '.log')
    before = identity(root)
    with log.open('x') as f:
        try:
            outcome = subprocess.run(command, cwd=root, stdout=f, stderr=subprocess.STDOUT)
            code = outcome.returncode
        except OSError as exc:
            f.write(str(exc) + '\n')
            code = 127
    result = {'task': name, 'at': datetime.now(timezone.utc).isoformat(), 'command': command,
              'exit_code': code, 'result': 'pass' if code == 0 else 'fail',
              'before': before, 'after': identity(root), 'log': str(log.relative_to(root)),
              'acceptance': 'requires review; command success is not task completion'}
    (directory / (run_id + '.json')).write_text(json.dumps(result, indent=2) + '\n')
    return result


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('action', choices=['start', 'verify'])
    parser.add_argument('task')
    parser.add_argument('command', nargs=argparse.REMAINDER)
    args = parser.parse_args()
    try:
        if args.action == 'start':
            print(start(ROOT, args.task))
            return 0
        command = args.command[1:] if args.command[:1] == ['--'] else args.command
        result = verify(ROOT, args.task, command)
        print(json.dumps(result, indent=2))
        return result['exit_code'] if result['exit_code'] >= 0 else 1
    except (ValueError, FileExistsError) as exc:
        print(str(exc), file=sys.stderr)
        return 2


if __name__ == '__main__':
    sys.exit(main())
