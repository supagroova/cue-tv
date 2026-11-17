#!/usr/bin/env python3
"""Opt-in real-peer encrypted relaunch qualification; never part of routine CI."""
import argparse
import json
from pathlib import Path
import re
import subprocess
import uuid

ROOT = Path(__file__).resolve().parents[2]
MEASUREMENT = re.compile(r'CUE_RESUME_MEASUREMENT last=(\d+) restored=(\d+) resumed=(\d+)')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('destination', help='Explicit tvOS simulator destination')
    args = parser.parse_args()
    output = ROOT / '.agent-work/resume-cycles' / uuid.uuid4().hex
    output.mkdir(parents=True)
    report = {'scope': 'simulator, graceful Close followed by process termination; independent encrypted fixture namespaces',
              'required_cycles': 10, 'cycles': [], 'complete': False}
    print(f'Evidence: {output}', flush=True)
    for cycle in range(1, 11):
        log = output / f'cycle-{cycle:02d}.log'
        print(f'Running relaunch cycle {cycle}/10', flush=True)
        with log.open('w') as stream:
            result = subprocess.run([
                str(ROOT / 'scripts/fixtures/check-journey.sh'), args.destination,
                'StandaloneJourneyTests/testProgressSurvivesProcessRelaunch'
            ], cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT)
        matches = MEASUREMENT.findall(log.read_text())
        entry = {'cycle': cycle, 'exit_code': result.returncode, 'log': log.name}
        if len(matches) == 1:
            last, restored, resumed = map(int, matches[0])
            entry.update(last_seconds=last, restored_seconds=restored, resumed_seconds=resumed,
                         restore_error_seconds=abs(restored - last))
        passed = (result.returncode == 0 and len(matches) == 1
                  and entry['restore_error_seconds'] <= 15
                  and abs(entry['resumed_seconds'] - entry['restored_seconds']) <= 15)
        entry['passed'] = passed
        report['cycles'].append(entry)
        report['complete'] = cycle == 10 and passed
        (output / 'report.json').write_text(json.dumps(report, indent=2) + '\n')
        if not passed:
            raise SystemExit(f'Cycle {cycle} failed; inspect {log}. No automatic retry.')
        print(f'Cycle {cycle} passed; restore error {entry["restore_error_seconds"]}s', flush=True)
    print(f'All ten simulator relaunch cycles passed: {output / "report.json"}', flush=True)


if __name__ == '__main__':
    main()
