#!/usr/bin/env python3
"""Remux owned HEVC into MP4 and package existing decoder fixtures as torrents."""
import hashlib
import json
from pathlib import Path
import subprocess
from generate import bencode

root = Path(__file__).resolve().parents[2]
media = root / 'apple-tv/Resources/PlaybackLab'
output = media / 'hevc-tracks.mp4'
subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(media / 'hevc-tracks.mkv'),
    '-map', '0:v', '-map', '0:a', '-c', 'copy', '-tag:v', 'hvc1',
    '-movflags', '+faststart', '-fflags', '+bitexact', str(output)], check=True)
entries = []
for name, torrent_name in [('episode01.mkv', 'mkv'), ('hevc-tracks.mkv', 'hevc-mkv'), ('hevc-tracks.mp4', 'hevc-mp4')]:
    path = media / name
    content = path.read_bytes()
    info = {'name': name, 'length': len(content), 'piece length': 16384,
        'pieces': b''.join(hashlib.sha1(content[i:i+16384]).digest() for i in range(0, len(content), 16384))}
    torrent = bencode({'info': info})
    (root / 'fixtures/torrents' / (torrent_name + '.torrent')).write_bytes(torrent)
    probe = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_streams',
        '-show_format', '-of', 'json', str(path)]))
    entries.append({'filename': name, 'bytes': len(content), 'sha256': hashlib.sha256(content).hexdigest(),
        'torrent': torrent_name + '.torrent', 'info_hash': hashlib.sha1(bencode(info)).hexdigest(),
        'duration_seconds': float(probe['format']['duration']),
        'streams': [{k: s[k] for k in ('codec_type', 'codec_name', 'profile', 'width', 'height',
            'pix_fmt', 'sample_rate', 'channels') if k in s} for s in probe['streams']]})
(media / 'streaming-matrix-manifest.json').write_text(json.dumps({
    'permission': 'Project-owned synthetic colors, tones and subtitle text.',
    'generator': 'scripts/fixtures/generate-streaming-matrix.py',
    'ffmpeg': subprocess.check_output(['ffmpeg', '-version'], text=True).splitlines()[0],
    'fixtures': entries}, indent=2) + '\n')
