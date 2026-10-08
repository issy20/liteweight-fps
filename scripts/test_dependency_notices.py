import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent))
from dependency_notices import render_notices


class DependencyNoticeTests(unittest.TestCase):
    def test_notice_lists_dependencies_and_embeds_available_license_text(self):
        packages = [
            {
                'name': 'sample-dep',
                'version': '1.2.3',
                'license': 'MIT OR Apache-2.0',
                'authors': ['Example Author'],
                'repository': 'https://example.invalid/sample-dep',
                'manifest_path': '/unused/sample-dep/Cargo.toml',
                'license_files': [('LICENSE-MIT', 'Sample MIT terms')],
            },
            {
                'name': 'license-less-package',
                'version': '0.4.0',
                'license': 'BSD-3-Clause',
                'authors': [],
                'repository': None,
                'manifest_path': '/unused/license-less-package/Cargo.toml',
                'license_files': [],
            },
        ]

        notices = render_notices(packages)

        self.assertIn('sample-dep 1.2.3 — MIT OR Apache-2.0', notices)
        self.assertIn('Example Author', notices)
        self.assertIn('Sample MIT terms', notices)
        self.assertIn('license-less-package 0.4.0 — BSD-3-Clause', notices)
        self.assertIn('No license text file was included', notices)


if __name__ == '__main__':
    unittest.main()
