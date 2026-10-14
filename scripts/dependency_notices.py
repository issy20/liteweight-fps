"""Generate a third-party license inventory and bundle available license texts."""

import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def metadata_packages(project_root):
    project_root = Path(project_root).resolve()
    metadata = subprocess.run(
        ['cargo', 'metadata', '--locked', '--format-version', '1'],
        cwd=project_root,
        check=True,
        capture_output=True,
        text=True,
        encoding='utf-8',
        errors='replace',
    )
    packages = json.loads(metadata.stdout)['packages']
    dependencies = []
    for package in packages:
        manifest = Path(package['manifest_path']).resolve()
        if manifest == project_root / 'Cargo.toml':
            continue
        root = manifest.parent
        license_files = []
        license_file = package.get('license_file')
        if license_file:
            declared = Path(license_file)
            candidate = declared if declared.is_absolute() else root / declared
            if candidate.is_file():
                license_files.append((candidate.name, candidate.read_text(encoding='utf-8', errors='replace')))
        for candidate in sorted(root.iterdir()):
            if candidate.is_file() and candidate.name.upper().startswith(
                ('LICENSE', 'COPYING', 'NOTICE')
            ):
                item = (candidate.name, candidate.read_text(encoding='utf-8', errors='replace'))
                if item not in license_files:
                    license_files.append(item)
        dependencies.append(
            {
                'name': package['name'],
                'version': package['version'],
                'license': package.get('license') or 'UNDECLARED',
                'authors': package.get('authors') or [],
                'repository': package.get('repository') or package.get('source'),
                'manifest_path': str(manifest),
                'license_files': license_files,
            }
        )
    return dependencies


def render_notices(packages):
    lines = [
        'Third-party dependency notices',
        'Generated from Cargo metadata and the resolved Cargo.lock dependency graph.',
        'The inventory includes target-specific dependencies for all release targets.',
        '',
        '## Dependency inventory',
    ]
    packages = sorted(packages, key=lambda package: (package['name'].lower(), package['version']))
    for package in packages:
        authors = ', '.join(package.get('authors') or []) or 'authors not declared'
        source = package.get('repository') or 'source URL not declared'
        lines.append(
            f"- {package['name']} {package['version']} — {package['license']}; "
            f'{authors}; {source}'
        )

    texts = {}
    missing = []
    for package in packages:
        license_files = package.get('license_files') or []
        if not license_files:
            missing.append(package)
        for filename, body in license_files:
            digest = hashlib.sha256(body.encode('utf-8')).hexdigest()
            entry = texts.setdefault(digest, {'sources': [], 'body': body})
            entry['sources'].append(f"{package['name']} {package['version']}: {filename}")

    lines.extend(['', '## Bundled license texts', ''])
    if not texts:
        lines.append('No license text files were found in dependency sources.')
    for index, entry in enumerate(sorted(texts.values(), key=lambda item: item['sources'][0]), 1):
        lines.append(f"### License text {index}")
        lines.extend(f"- {source}" for source in sorted(set(entry['sources'])))
        lines.extend(['', '```text', entry['body'].rstrip(), '```', ''])

    lines.extend(['## Dependencies without an included license text', ''])
    if missing:
        lines.extend(
            f"- {package['name']} {package['version']} — {package['license']}"
            for package in missing
        )
        lines.append('No license text file was included with these crate sources; verify their notices before public distribution.')
    else:
        lines.append('None.')
    return '\n'.join(lines).rstrip() + '\n'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=Path('target/THIRD_PARTY_NOTICES.txt'))
    args = parser.parse_args()
    project_root = Path(__file__).resolve().parents[1]
    output = args.output if args.output.is_absolute() else project_root / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(render_notices(metadata_packages(project_root)), encoding='utf-8')
    print(output)


if __name__ == '__main__':
    main()
