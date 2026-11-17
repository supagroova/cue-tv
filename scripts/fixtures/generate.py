#!/usr/bin/env python3
"""Generate owned synthetic H.264/AAC fixtures and public-protocol test torrent metadata."""
import hashlib
import json
from pathlib import Path
import subprocess


def bencode(value):
    if isinstance(value, int):
        return b'i' + str(value).encode() + b'e'
    if isinstance(value, str):
        return bencode(value.encode())
    if isinstance(value, bytes):
        return str(len(value)).encode() + b':' + value
    if isinstance(value, list):
        return b'l' + b''.join(map(bencode, value)) + b'e'
    return b'd' + b''.join(bencode(k) + bencode(value[k]) for k in sorted(value)) + b'e'


def main():
    root = Path(__file__).resolve().parents[2]
    output = root / 'apple-tv/Resources/PlaybackLab'
    output.mkdir(parents=True, exist_ok=True)
    fixtures = []
    for name, color, fast in [('episode01', 'blue', True), ('episode02', 'green', True), ('tail-index', 'red', False)]:
        file = output / (name + '.mp4')
        command = ['ffmpeg', '-hide_banner', '-loglevel', 'error', '-y', '-f', 'lavfi', '-i', f'color=c={color}:s=1920x1080:r=24:d=12', '-f', 'lavfi', '-i', 'sine=frequency=440:sample_rate=48000:duration=12', '-c:v', 'libx264', '-pix_fmt', 'yuv420p', '-preset', 'fast', '-c:a', 'aac', '-b:a', '96k', '-shortest']
        if fast:
            command += ['-movflags', '+faststart']
        subprocess.run(command + [str(file)], check=True)
        fixtures.append(dict(id=name, title=name.replace('-', ' ').title(), filename=file.name, sha256=hashlib.sha256(file.read_bytes()).hexdigest(), durationSeconds=12))
    (output / 'playback-fixtures.json').write_text(json.dumps(fixtures, indent=2) + '\n')
    torrent_dir = root / 'fixtures/torrents'
    torrent_dir.mkdir(parents=True, exist_ok=True)
    manifest = {'permission': 'Synthetic colors and generated sine audio created for this project; no third-party media.', 'generator': 'scripts/fixtures/generate.py', 'ffmpeg': subprocess.check_output(['ffmpeg', '-version'], text=True).splitlines()[0], 'torrents': []}
    for name, selected in [('single', fixtures[:1]), ('multi-episode', fixtures), ('tail-index', fixtures[2:])]:
        files = [output / item['filename'] for item in selected]
        content = b''.join(p.read_bytes() for p in files)
        piece_length = 16384
        info = {'name': files[0].name if len(files) == 1 else 'CueTV-fixtures', 'piece length': piece_length, 'pieces': b''.join(hashlib.sha1(content[i:i+piece_length]).digest() for i in range(0, len(content), piece_length))}
        if len(files) == 1:
            info['length'] = len(content)
        else:
            info['files'] = [{'length':p.stat().st_size, 'path':[p.name]} for p in files]
        encoded = bencode(info)
        (torrent_dir / (name + '.torrent')).write_bytes(bencode({'info':info}))
        manifest['torrents'].append({'file':name+'.torrent', 'infoHash':hashlib.sha1(encoded).hexdigest(), 'files':[{'index':i, 'path':p.name, 'bytes':p.stat().st_size, 'sha256':selected[i]['sha256']} for i,p in enumerate(files)], 'discovery':'No tracker or web seed embedded. Controlled seeder/trackers must be configured during engine tests.'})
    (torrent_dir / 'manifest.json').write_text(json.dumps(manifest, indent=2)+'\n')


if __name__ == '__main__':
    main()
