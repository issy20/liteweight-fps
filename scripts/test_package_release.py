import sys
import tempfile
import unittest
import zipfile
import tarfile
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from package_release import create_package, validate_release_tag


class PackageReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        (self.root / 'Cargo.toml').write_text(
            '[package]\nversion = "0.1.0"\nlicense = "MIT OR Apache-2.0"\n'
        )
        (self.root / 'README.md').write_text('Run the game')
        (self.root / 'RELEASE_NOTES.md').write_text('Preview notes')
        (self.root / 'DISTRIBUTION.md').write_text('Runtime requirements')
        (self.root / 'LICENSE-MIT').write_text('MIT License')
        (self.root / 'LICENSE-APACHE').write_text('Apache License 2.0')
        (self.root / 'target').mkdir()
        (self.root / 'target/THIRD_PARTY_NOTICES.txt').write_text('Third-party licenses')
        (self.root / 'target/linux-runtime-dependencies.txt').write_text('libc.so.6')
        (self.root / 'assets/fonts').mkdir(parents=True)
        (self.root / 'assets/fonts/OFL.txt').write_text('Font license')
        self.output = self.root / 'dist'
        self.release = self.root / 'release'
        self.release.mkdir()

    def tearDown(self):
        self.temp.cleanup()

    def test_packages_each_target_with_expected_binary_and_notices(self):
        for target, binary in [
            ('x86_64-unknown-linux-gnu', 'liteweight-fps'),
            ('x86_64-pc-windows-msvc', 'liteweight-fps.exe'),
            ('aarch64-apple-darwin', 'liteweight-fps'),
            ('x86_64-apple-darwin', 'liteweight-fps'),
        ]:
            binary_path = self.release / binary
            binary_path.write_bytes(b'binary:' + target.encode())
            archive_path = create_package(target, self.root, binary_path, self.output)
            package_root = f'liteweight-fps-0.1.0-{target}'
            expected = {
                f'{package_root}/{binary}',
                f'{package_root}/README.md',
                f'{package_root}/RELEASE_NOTES.md',
                f'{package_root}/DISTRIBUTION.md',
                f'{package_root}/LICENSE-MIT',
                f'{package_root}/LICENSE-APACHE',
                f'{package_root}/Cantarell-OFL.txt',
                f'{package_root}/THIRD_PARTY_NOTICES.txt',
            }

            if target == 'x86_64-pc-windows-msvc':
                with zipfile.ZipFile(archive_path) as archive:
                    self.assertEqual(set(archive.namelist()), expected)
                    self.assertEqual(archive.read(f'{package_root}/{binary}'), b'binary:' + target.encode())
            else:
                with tarfile.open(archive_path, 'r:gz') as archive:
                    if target == 'x86_64-unknown-linux-gnu':
                        expected.add(f'{package_root}/LINUX_RUNTIME_DEPENDENCIES.txt')
                    self.assertEqual(set(archive.getnames()), expected)
                    self.assertEqual(
                        archive.extractfile(f'{package_root}/{binary}').read(),
                        b'binary:' + target.encode(),
                    )

    def test_existing_archive_is_never_overwritten(self):
        binary_path = self.release / 'liteweight-fps'
        binary_path.write_bytes(b'game')
        archive_path = create_package(
            'aarch64-apple-darwin', self.root, binary_path, self.output
        )
        archive_path.write_bytes(b'keep existing')

        with self.assertRaises(FileExistsError):
            create_package('aarch64-apple-darwin', self.root, binary_path, self.output)
        self.assertEqual(archive_path.read_bytes(), b'keep existing')

    def test_release_tag_must_match_package_version(self):
        self.assertEqual(validate_release_tag('v0.1.0', self.root), '0.1.0')
        with self.assertRaisesRegex(ValueError, 'does not match'):
            validate_release_tag('v1.0.0', self.root)
        with self.assertRaisesRegex(ValueError, 'must start with v'):
            validate_release_tag('0.1.0', self.root)


if __name__ == '__main__':
    unittest.main()
