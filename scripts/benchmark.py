"""M0の逐次実測。Python標準ライブラリのみ。既存出力の上書きは禁止。"""
import argparse
import hashlib
import json
import platform
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parent.parent


def plan(height=None, dummies=None):
    requested_height, requested_dummies = height, dummies
    runs = []
    for width, height in [(1280, 720), (1920, 1080)]:
        for dummies in [5, 20]:
            for repeat in range(1, 4):
                order = ['wgpu', 'miniquad'] if repeat % 2 else ['miniquad', 'wgpu']
                for backend in order:
                    runs.append(dict(width=width, height=height, dummies=dummies,
                                     repeat=repeat, backend=backend))
    return [r for r in runs if (requested_height is None or r['height'] == requested_height)
            and (requested_dummies is None or r['dummies'] == requested_dummies)]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--dry-run', action='store_true')
    parser.add_argument('--height', type=int, choices=[720, 1080])
    parser.add_argument('--dummies', type=int, choices=[5, 20])
    args = parser.parse_args()
    runs = plan(args.height, args.dummies)
    if args.dry_run:
        print(json.dumps(runs, indent=2))
        return
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    hashes = {}
    for path in [ROOT / 'Cargo.lock', *sorted((ROOT / 'src').rglob('*.rs')),
                 *sorted((ROOT / 'assets/shaders').iterdir())]:
        hashes[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
    manifest = dict(started=time.strftime('%Y-%m-%dT%H:%M:%S%z'), platform=platform.platform(),
                    source_sha256=hashes, planned_runs=runs, runs=[])
    manifest_path = output / 'manifest.json'
    for index, run in enumerate(runs, 1):
        name = '{backend}-{height}p-{dummies}-{repeat}'.format(**run)
        binary = ROOT / 'target/release' / ('bench-' + run['backend'] + ('.exe' if platform.system() == 'Windows' else ''))
        command = [str(binary), '--width', str(run['width']), '--height', str(run['height']),
                   '--dummies', str(run['dummies']), '--warmup', '5', '--seconds', '30',
                   '--vsync', 'on', '--output', str(output / name)]
        print(f'[{index}/{len(runs)}] {name}', flush=True)
        item = dict(run, command=command, binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest())
        manifest['runs'].append(item)
        manifest_path.write_text(json.dumps(manifest, indent=2))
        with (output / (name + '.log')).open('w') as log:
            result = subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, timeout=90)
        item['exit_code'] = result.returncode
        manifest_path.write_text(json.dumps(manifest, indent=2))
        if result.returncode:
            raise SystemExit(f'Failed: {name}; inspect log')
        report = json.loads((output / (name + '.json')).read_text())
        if not report['comparison_valid']:
            raise SystemExit(f'Invalid comparison: {name}; stopped without changing the report')
    manifest['completed'] = time.strftime('%Y-%m-%dT%H:%M:%S%z')
    manifest_path.write_text(json.dumps(manifest, indent=2))
    print(f'Completed: {output}', flush=True)


if __name__ == '__main__':
    main()
