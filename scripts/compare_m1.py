"""M0保存バイナリとM1レンダラーを同一シーンで交互に3回測定する。"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--baseline', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    baseline = args.baseline.resolve()
    candidate = ROOT / 'target/release/liteweight-fps'
    manifest = {'started': time.strftime('%Y-%m-%dT%H:%M:%S%z'), 'runs': [],
                'binary_sha256': {str(p): hashlib.sha256(p.read_bytes()).hexdigest() for p in [baseline, candidate]}}
    for repeat in range(1, 4):
        for label in (['m0', 'm1'] if repeat % 2 else ['m1', 'm0']):
            prefix = output / f'{label}-{repeat}'
            command = [str(baseline)] if label == 'm0' else [str(candidate), '--benchmark']
            command += ['--width', '1280', '--height', '720', '--dummies', '5', '--warmup', '5', '--seconds', '30', '--vsync', 'on', '--output', str(prefix)]
            print(f'{label}-{repeat}', flush=True)
            item = {'label': label, 'repeat': repeat, 'command': command}
            manifest['runs'].append(item)
            (output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
            with prefix.with_suffix('.log').open('w') as log:
                result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, timeout=90)
            item['exit_code'] = result.returncode
            (output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
            if result.returncode or not json.loads(prefix.with_suffix('.json').read_text())['comparison_valid']:
                raise SystemExit('Failed/invalid trial; stopped: ' + str(prefix))
    manifest['completed'] = time.strftime('%Y-%m-%dT%H:%M:%S%z')
    (output / 'manifest.json').write_text(json.dumps(manifest, indent=2))
    print('Completed', flush=True)


if __name__ == '__main__':
    main()
