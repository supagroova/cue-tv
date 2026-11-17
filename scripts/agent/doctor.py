#!/usr/bin/env python3
import shutil
import subprocess
from pathlib import Path
root = Path(__file__).resolve().parents[2]
for tool in ('python3', 'make', 'git', 'xcodebuild', 'xcodegen', 'rustc', 'cargo', 'gh'):
    print(f'{tool}: {shutil.which(tool) or "not installed"}')
result = subprocess.run(['git', '-C', str(root), 'rev-parse', '--show-toplevel'], capture_output=True, text=True)
checkout = result.returncode == 0 and Path(result.stdout.strip()).resolve() == root
print('Source: Git checkout' if checkout else 'Source: extracted snapshot; Git-only commands unavailable')
print('Device pairing, signing, hook activation and physical playback: not checked')
