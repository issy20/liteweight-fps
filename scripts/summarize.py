"""有効な同一条件の試行から中央値を集計する。"""
import argparse
import json
from pathlib import Path
from statistics import median


def summarize(reports):
    if len(reports) != 3:
        raise ValueError('Exactly three trials are required')
    if any(not r['comparison_valid'] or not r['completed'] or not r['release'] for r in reports):
        raise ValueError('Invalid trial')
    keys = ['width', 'height', 'dummies', 'warmup', 'seconds', 'vsync', 'input', 'window_system']
    baseline = reports[0]
    for r in reports:
        if any(r['options'][k] != baseline['options'][k] for k in keys):
            raise ValueError('Mismatched options')
        if r['options']['input'] or r['scene'] != baseline['scene'] or r['backend'] != baseline['backend']:
            raise ValueError('Mismatched scene/backend or input mode')
        if r['display_pixels_initial'] != baseline['display_pixels_initial'] or r['display_pixels_final'] != baseline['display_pixels_initial']:
            raise ValueError('Mismatched display dimensions')
    return {
        'average_fps': median(r['frame_statistics']['average_fps'] for r in reports),
        'one_percent_low_fps': median(r['frame_statistics']['one_percent_low_fps'] for r in reports),
        'p99_ms': median(r['frame_statistics']['p99_ms'] for r in reports),
        'rss_mib': median(r['rss_sampled_max_bytes'] / 1048576 for r in reports),
        'startup_ms': median(r['startup_to_first_submit_return_ms'] for r in reports),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    args = parser.parse_args()
    manifest = json.loads((args.directory / 'manifest.json').read_text())
    if 'completed' not in manifest or len(manifest['runs']) != 24:
        raise SystemExit('Incomplete session')
    groups = {}
    displays = set()
    for run in manifest['runs']:
        name = '{backend}-{height}p-{dummies}-{repeat}'.format(**run)
        report = json.loads((args.directory / (name + '.json')).read_text())
        displays.add(tuple(report['display_pixels_initial']))
        key = (run['height'], run['dummies'], run['backend'])
        groups.setdefault(key, []).append(report)
    if len(displays) != 1:
        raise SystemExit('Display dimensions differ across backends')
    print('| 内部解像度 | ダミー | 実装 | 平均FPS | 1% Low | p99 ms | RSS MiB | 起動 ms |')
    print('|---|---:|---|---:|---:|---:|---:|---:|')
    for (height, dummies, backend), reports in sorted(groups.items()):
        row = summarize(reports)
        print(f"| {height}p | {dummies} | {backend} | {row['average_fps']:.2f} | {row['one_percent_low_fps']:.2f} | {row['p99_ms']:.2f} | {row['rss_mib']:.2f} | {row['startup_ms']:.2f} |")


if __name__ == '__main__':
    main()
