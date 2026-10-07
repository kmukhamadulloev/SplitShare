#!/usr/bin/env python3
import importlib.util
from pathlib import Path
import unittest

spec = importlib.util.spec_from_file_location('notes', Path(__file__).with_name('release-notes.py'))
notes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(notes)


class ReleaseNotesTests(unittest.TestCase):
    def test_only_matching_version_preserves_categories(self):
        source = '# History\n\n## 0.2.0 — date\n\n- New\n\n## 0.1.0 — date\n\n### Added\n\n- First\n\n### Fixed\n\n- Bug\n\n## 0.0.1\n\n- Old\n'
        self.assertEqual(notes.extract_notes(source, '0.1.0'), '### Added\n\n- First\n\n### Fixed\n\n- Bug\n')

    def test_missing_empty_or_duplicate_fails(self):
        for source in ['## 0.2.0\n- Other', '## 0.1.0\n',
                       '## 0.1.0\n### Added', '## 0.1.0\n- A\n## 0.1.0\n- B']:
            with self.subTest(source=source), self.assertRaises(ValueError):
                notes.extract_notes(source, '0.1.0')

    def test_repository_versions_and_tag(self):
        version = notes.release_version(notes.REPO)
        self.assertEqual(notes.release_version(notes.REPO, 'v' + version), version)
        for tag in ['v99.0.0', version, 'v0.1.0;echo bad']:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                notes.release_version(notes.REPO, tag)


if __name__ == '__main__':
    unittest.main()
