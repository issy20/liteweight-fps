"""Package one release binary with its operating instructions and font notice."""

import argparse
import re
import tarfile
import zipfile
from pathlib import Path


TARGET_BINARIES = {
    'x86_64-unknown-linux-gnu': 'liteweight-fps',
    'x86_64-pc-windows-msvc': 'liteweight-fps.exe',
    'aarch64-apple-darwin': 'liteweight-fps',
    'x86_64-apple-darwin': 'liteweight-fps',
}


def package_version(project_root):
    cargo_toml = (project_root / 'Cargo.toml').read_text(encoding='utf-8')
    match = re.search(r'^version\s*=\s*"([^"]+)"\s*$', cargo_toml, re.MULTILINE)
    if not match:
        raise ValueError('Cargo.toml package version was not found')
    return match.group(1)


def create_package(target, project_root, binary_path, output_dir):
    if target not in TARGET_BINARIES:
        raise ValueError(f'Unsupported release target: {target}')

    project_root = Path(project_root)
    binary_path = Path(binary_path)
    output_dir = Path(output_dir)
    if binary_path.name != TARGET_BINARIES[target] or not binary_path.is_file():
        raise FileNotFoundError(f'Release binary not found: {binary_path}')

    version = package_version(project_root)
    package_root = f'liteweight-fps-{version}-{target}'
    files = [
        (binary_path, binary_path.name),
        (project_root / 'README.md', 'README.md'),
        (project_root / 'DISTRIBUTION.md', 'DISTRIBUTION.md'),
        (project_root / 'assets/fonts/OFL.txt', 'Cantarell-OFL.txt'),
    ]
    missing = [str(source) for source, _ in files if not source.is_file()]
    if missing:
        raise FileNotFoundError('Required package files are missing: ' + ', '.join(missing))

    output_dir.mkdir(parents=True, exist_ok=True)
    is_windows = target == 'x86_64-pc-windows-msvc'
    archive_path = output_dir / (
        f'{package_root}.zip' if is_windows else f'{package_root}.tar.gz'
    )
    if archive_path.exists():
        raise FileExistsError(f'Refusing to overwrite package: {archive_path}')

    if is_windows:
        with zipfile.ZipFile(archive_path, 'x', compression=zipfile.ZIP_DEFLATED) as archive:
            for source, name in files:
                archive.write(source, f'{package_root}/{name}')
    else:
        with tarfile.open(archive_path, 'x:gz') as archive:
            for source, name in files:
                archive.add(source, arcname=f'{package_root}/{name}')
    return archive_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', required=True, choices=sorted(TARGET_BINARIES))
    parser.add_argument('--release-dir', type=Path, default=Path('target/release'))
    parser.add_argument('--output-dir', type=Path, default=Path('dist'))
    args = parser.parse_args()

    project_root = Path(__file__).resolve().parents[1]
    binary_path = args.release_dir / TARGET_BINARIES[args.target]
    try:
        archive_path = create_package(
            args.target, project_root, binary_path, args.output_dir
        )
    except (FileNotFoundError, FileExistsError, ValueError) as error:
        parser.error(str(error))
    print(archive_path)


if __name__ == '__main__':
    main()
