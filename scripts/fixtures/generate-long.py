#!/usr/bin/env python3
"""Generate optional owned long-form media outside Git for sustained simulator proof."""
import hashlib
import json
from pathlib import Path
import subprocess
from generate import bencode

root = Path(__file__).resolve().parents[2]
out = root / '.agent-work/long-playback'
out.mkdir(parents=True, exist_ok=True)
video = out / 'long.mp4'
subprocess.run(['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i',
                'color=c=blue:s=1920x1080:r=24:d=720', '-f', 'lavfi', '-i',
                'sine=frequency=440:sample_rate=48000:duration=720', '-c:v', 'libx264',
                '-pix_fmt', 'yuv420p', '-preset', 'ultrafast', '-c:a', 'aac', '-b:a', '96k',
                '-shortest', '-movflags', '+faststart', str(video)], check=True)
content = video.read_bytes()
info = {'name': video.name, 'length': len(content), 'piece length': 16384,
        'pieces': b''.join(hashlib.sha1(content[i:i+16384]).digest() for i in range(0, len(content), 16384))}
(out / 'long.torrent').write_bytes(bencode({'info': info}))
(out / 'manifest.json').write_text(json.dumps({'durationSeconds': 720, 'bytes': len(content),
    'sha256': hashlib.sha256(content).hexdigest(), 'permission': 'Project-generated colors and sine audio.'}, indent=2))
print(out)
