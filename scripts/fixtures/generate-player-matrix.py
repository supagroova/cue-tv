#!/usr/bin/env python3
"""Owned HEVC SDR media with distinct audio and embedded subtitle tracks."""
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
media = root / 'apple-tv/Resources/PlaybackLab'
source = media / 'episode01.mp4'
output = media / 'hevc-tracks.mkv'
with tempfile.TemporaryDirectory() as scratch:
    srt = Path(scratch) / 'fixture.srt'
    vtt = Path(scratch) / 'fixture.vtt'
    srt.write_text('\n'.join(f'{i+1}\n00:00:{i:02d},000 --> 00:00:{i+1:02d},000\nCue TV SRT fixture\n' for i in range(12)))
    vtt.write_text('WEBVTT\n\n' + '\n'.join(f'00:{i:02d}.000 --> 00:{i+1:02d}.000\nCue TV WebVTT fixture\n' for i in range(12)))
    subprocess.run(['ffmpeg', '-v', 'error', '-y', '-i', str(source),
        '-f', 'lavfi', '-i', 'sine=frequency=880:sample_rate=48000:duration=12',
        '-i', str(srt), '-i', str(vtt), '-map', '0:v', '-map', '0:a', '-map', '1:a',
        '-map', '2:s', '-map', '3:s', '-c:v', 'hevc_videotoolbox', '-pix_fmt', 'yuv420p',
        '-color_primaries', 'bt709', '-color_trc', 'bt709', '-colorspace', 'bt709',
        '-c:a', 'aac', '-c:s:0', 'srt', '-c:s:1', 'webvtt',
        '-metadata:s:a:0', 'title=Original tone', '-metadata:s:a:1', 'title=880 Hz alternate',
        '-metadata:s:s:0', 'title=SRT fixture', '-metadata:s:s:1', 'title=WebVTT fixture',
        '-disposition:s:0', 'default', '-t', '12', '-fflags', '+bitexact', str(output)], check=True)
probe = json.loads(subprocess.check_output(['ffprobe', '-v', 'error', '-show_streams',
    '-show_format', '-of', 'json', str(output)]))
manifest = {
    'permission': 'Project-owned synthetic colors, tones and subtitle text.',
    'generator': 'scripts/fixtures/generate-player-matrix.py',
    'encoder': 'hevc_videotoolbox (macOS)',
    'ffmpeg': subprocess.check_output(['ffmpeg', '-version'], text=True).splitlines()[0],
    'source_sha256': hashlib.sha256(source.read_bytes()).hexdigest(),
    'filename': output.name, 'bytes': output.stat().st_size,
    'sha256': hashlib.sha256(output.read_bytes()).hexdigest(),
    'duration_seconds': float(probe['format']['duration']),
    'streams': [{k: stream[k] for k in ('index', 'codec_type', 'codec_name', 'profile',
        'width', 'height', 'pix_fmt', 'color_space', 'color_transfer', 'color_primaries',
        'sample_rate', 'channels', 'tags') if k in stream} for stream in probe['streams']],
}
(media / 'hevc-tracks-manifest.json').write_text(json.dumps(manifest, indent=2) + '\n')
