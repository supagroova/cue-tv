#!/usr/bin/env python3
"""Extend owned media for manual remote interactions without changing CI fixtures."""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
from generate import bencode

root = Path(__file__).resolve().parents[2]
output = Path(sys.argv[1])
media = output / "CueTV-fixtures"
media.mkdir()
files = []
for name in ("episode01.mp4", "episode02.mp4", "tail-index.mp4"):
    target = media / name
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-stream_loop", "2",
                    "-i", str(root / "apple-tv/Resources/PlaybackLab" / name),
                    "-c", "copy", "-movflags", "+faststart", str(target)], check=True)
    duration = float(subprocess.check_output(["ffprobe", "-v", "error", "-show_entries",
        "format=duration", "-of", "default=noprint_wrappers=1:nokey=1", str(target)], text=True))
    if duration < 35:
        raise RuntimeError("Journey fixture is too short")
    files.append({"index": len(files), "path": name, "bytes": target.stat().st_size, "durationSeconds": duration,
                  "sha256": hashlib.sha256(target.read_bytes()).hexdigest()})
content = b"".join((media / item["path"]).read_bytes() for item in files)
info = {"name": "CueTV-fixtures", "piece length": 16384,
        "pieces": b"".join(hashlib.sha1(content[i:i+16384]).digest() for i in range(0, len(content), 16384)),
        "files": [{"length": item["bytes"], "path": [item["path"]]} for item in files]}
(output / "journey.torrent").write_bytes(bencode({"info": info}))
(output / "manifest.json").write_text(json.dumps({"infoHash": hashlib.sha1(bencode(info)).hexdigest(),
    "files": files, "permission": "Owned synthetic project colors and sine audio, stream-copied three times.",
    "ffmpeg": subprocess.check_output(["ffmpeg", "-version"], text=True).splitlines()[0]}, indent=2))

print((output / "manifest.json").read_text())
