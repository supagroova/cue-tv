#!/usr/bin/env python3
"""Manual simulator sampling; no automatic CI invocation or physical-device claim."""
import argparse
import json
from pathlib import Path
import statistics
import subprocess
import time


def app_pids():
    result = subprocess.run(['pgrep', '-f', '/SittingRoomTV.app/SittingRoomTV'], capture_output=True, text=True)
    return set(result.stdout.split())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--device', required=True, help='tvOS simulator UDID')
    parser.add_argument('--output', default='.agent-work/vlc-profile', help='output file prefix')
    args = parser.parse_args()
    if not Path('.agent-work/long-playback/long.torrent').is_file():
        parser.error('generate the optional long fixture and start its controlled seeder first')
    prefix = Path(args.output)
    prefix.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run(['xcodegen', 'generate', '--spec', 'apple-tv/project-long.yml'], check=True)
    command = ['xcodebuild', '-project', 'apple-tv/SittingRoomTV.xcodeproj', '-scheme', 'SittingRoomTV',
        '-configuration', 'Debug', '-derivedDataPath', '.agent-work/DerivedData', 'CODE_SIGNING_ALLOWED=NO',
        '-destination', f'platform=tvOS Simulator,id={args.device}',
        '-only-testing:SittingRoomTVUITests/LongPlaybackTests/testOneMinuteVLCAndDistantSeek', 'test']
    old = app_pids()
    samples = []
    start = time.monotonic()
    with prefix.with_suffix('.log').open('w') as log:
        process = subprocess.Popen(command, stdout=log, stderr=subprocess.STDOUT)
        try:
            while process.poll() is None:
                for pid in app_pids() - old:
                    row = subprocess.run(['ps', '-p', pid, '-o', 'rss=,%cpu='], capture_output=True, text=True).stdout.split()
                    if len(row) == 2:
                        samples.append({'elapsed': round(time.monotonic() - start, 2), 'pid': int(pid),
                                        'rss_kib': int(row[0]), 'cpu_percent': float(row[1])})
                time.sleep(1)
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
            result = {'exit_code': process.returncode, 'elapsed_seconds': time.monotonic() - start,
                'scope': 'macOS simulator process; sampled ps RSS and CPU, not physical tvOS or peak allocation',
                'sample_count': len(samples),
                'max_rss_kib': max((s['rss_kib'] for s in samples), default=None),
                'median_cpu_percent': statistics.median(s['cpu_percent'] for s in samples) if samples else None,
                'max_cpu_percent': max((s['cpu_percent'] for s in samples), default=None), 'samples': samples}
            prefix.with_suffix('.json').write_text(json.dumps(result, indent=2) + '\n')
            print(json.dumps({k: v for k, v in result.items() if k != 'samples'}))
    return process.returncode


if __name__ == '__main__':
    raise SystemExit(main())
