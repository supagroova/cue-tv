#!/usr/bin/env python3
"""Remux the project's owned H.264/AAC fixture for the candidate decoder probe."""
import hashlib
import json
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[2]
media = root / 'apple-tv/Resources/PlaybackLab'
source = media / 'episode01.mp4'
output = media / 'episode01.mkv'
subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(source),
                '-map', '0', '-c', 'copy', '-fflags', '+bitexact', str(output)], check=True)
probe = json.loads(subprocess.check_output([
    'ffprobe', '-v', 'error', '-show_streams', '-show_format', '-of', 'json', str(output)
]))
manifest = {
    'permission': 'Remux of project-owned synthetic colors and sine audio; no third-party media.',
    'generator': 'scripts/fixtures/generate-mkv.py',
    'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
    'filename': output.name,
    'bytes': output.stat().st_size,
    'sha256': hashlib.sha256(output.read_bytes()).hexdigest(),
    'duration_seconds': float(probe['format']['duration']),
    'streams': [{k: stream[k] for k in ('codec_type', 'codec_name', 'profile', 'width', 'height', 'sample_rate', 'channels') if k in stream}
                for stream in probe['streams']],
}
(media / 'mkv-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
