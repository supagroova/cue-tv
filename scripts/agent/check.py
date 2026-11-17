#!/usr/bin/env python3
"""Portable structure checks; no claim of app or device verification."""
import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def sensitive(path):
    p = Path(path)
    return (p.name.startswith('.env') and not p.name.endswith('.example')
            or p.name == 'Signing.mine.xcconfig'
            or p.name == 'TVConfig.plist'
            or p.suffix.lower() in {'.p12', '.p8', '.mobileprovision', '.provisionprofile'}
            or any(part in {'.agent-work', 'DerivedData', 'xcuserdata'} for part in p.parts)
            or any(part.endswith('.xcodeproj') for part in p.parts))


def staged(root):
    top = subprocess.run(['git', '-C', str(root), 'rev-parse', '--show-toplevel'], capture_output=True, text=True)
    if top.returncode or Path(top.stdout.strip()).resolve() != root.resolve():
        return ['staged-check requires a real checkout at this root']
    result = subprocess.run(['git', '-C', str(root), 'diff', '--cached', '--name-only', '--diff-filter=ACMR', '-z'], capture_output=True, check=True)
    return [f'keep local: {p}' for p in result.stdout.decode().split('\0') if p and sensitive(p)]


def check(root):
    errors = []
    files = [root / 'AGENTS.md', root / 'CLAUDE.md']
    files += list((root / 'docs/agent-harness').glob('*.md'))
    files += list((root / 'docs/agent-harness/templates').glob('*.md'))
    skills = list((root / '.agents/skills').glob('*/SKILL.md'))
    if not skills:
        errors.append('no canonical skills found')
    for p in skills:
        text = p.read_text()
        name = p.parent.name
        if not text.startswith('---\n') or text.count('---\n') < 2 or not re.search(r'^description: .+', text, re.M):
            errors.append(f'{p}: missing skill frontmatter')
        if f'name: {name}\n' not in text:
            errors.append(f'{p}: name does not match folder')
        alias = root / '.claude/skills' / name
        if not alias.is_symlink() or alias.resolve() != p.parent.resolve():
            errors.append(f'{alias}: expected symlink to canonical skill')
        files.append(p)
    for p in files:
        if not p.exists():
            errors.append(f'missing {p}')
            continue
        if p.name == 'UPSTREAM-SETUP.md':
            continue  # Historical upstream paths belong to the full multi-platform repository.
        text = p.read_text()
        if sum(line.startswith('```') for line in text.splitlines()) % 2:
            errors.append(f'{p}: unclosed code fence')
        prose = re.sub(r'```.*?```', '', text, flags=re.S)
        for link in re.findall(r'!?\[[^\]]*\]\(([^)]+)\)', prose):
            link = link.split('#')[0]
            if not link or '://' in link or link.startswith('mailto:'):
                continue
            target = (p.parent / link).resolve()
            if not target.exists():
                errors.append(f'{p}: missing link {link}')
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--staged', action='store_true')
    args = parser.parse_args()
    errors = staged(ROOT) if args.staged else check(ROOT)
    for error in errors:
        print(error, file=sys.stderr)
    if not errors:
        print('Staged paths passed' if args.staged else 'Harness structure passed (not app/device verification)')
    return bool(errors)


if __name__ == '__main__':
    sys.exit(main())
