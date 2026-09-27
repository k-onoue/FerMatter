"""Public-boundary and reproducibility checks for the guide publishing pipeline."""
from contextlib import redirect_stdout, redirect_stderr
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location('guides', Path(__file__).resolve().parents[1] / 'build-guides.py')
guides = importlib.util.module_from_spec(spec)
spec.loader.exec_module(guides)


class GuidePublishingTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.source = self.root / 'user-docs'
        self.site = self.root / 'docs'
        self.output = self.site / 'guide'
        self.source.mkdir()
        self.site.mkdir()
        for key, value in [('ROOT', self.root), ('SOURCE', self.source), ('SITE', self.site), ('OUTPUT', self.output)]:
            patcher = patch.object(guides, key, value)
            patcher.start()
            self.addCleanup(patcher.stop)
        self.page = {'source': 'README.md', 'slug': 'index', 'title': 'Guide', 'summary': 'A guide.'}
        (self.source / 'pages.json').write_text(json.dumps([self.page]))
        (self.source / 'README.md').write_text('# Guide\n\n- First instruction.\n')

    def run_build(self, *args):
        with patch('sys.argv', ['build-guides.py', *args]), redirect_stdout(io.StringIO()), redirect_stderr(io.StringIO()):
            return guides.main()

    def test_explicit_manifest_never_copies_unlisted_notes(self):
        (self.source / 'private-draft.md').write_text('# Not published\nConfidential draft')
        (self.root / 'dev-docs').mkdir()
        (self.root / 'dev-docs/internal.md').write_text('# Internal')
        self.assertEqual(self.run_build(), 0)
        self.assertEqual(sorted(p.name for p in self.output.iterdir()), ['index.html'])
        self.assertNotIn('Confidential', (self.output / 'index.html').read_text())

    def test_private_link_fails_before_output_is_written(self):
        (self.root / 'dev-docs').mkdir()
        (self.root / 'dev-docs/internal.md').write_text('# Internal')
        (self.source / 'README.md').write_text('# Guide\n\n[Internal](../dev-docs/internal.md)')
        with self.assertRaisesRegex(ValueError, 'not a public guide or asset'):
            self.run_build()
        self.assertFalse(self.output.exists())

    def test_links_resolve_from_a_project_subpath(self):
        (self.site / 'sample.zip').write_bytes(b'example')
        page2 = dict(self.page, source='next.md', slug='next', title='Next')
        (self.source / 'pages.json').write_text(json.dumps([self.page, page2]))
        (self.source / 'next.md').write_text('# Next\n\n## Details\n\n- Details.')
        (self.source / 'README.md').write_text('# Guide\n\n[Next](next.md#details) [Sample](../docs/sample.zip)')
        self.run_build()
        html = (self.output / 'index.html').read_text()
        self.assertIn('href="next.html#details"', html)
        self.assertIn('href="../sample.zip"', html)
        self.assertNotIn('href="/', html)

    def test_stale_check_fails_without_rewriting(self):
        self.run_build()
        output = self.output / 'index.html'
        before = output.read_bytes()
        (self.source / 'README.md').write_text('# Guide\n\n- Changed instruction.\n')
        self.assertEqual(self.run_build('--check'), 1)
        self.assertEqual(output.read_bytes(), before)
        self.assertEqual(self.run_build(), 0)
        self.assertEqual(self.run_build('--check'), 0)


if __name__ == '__main__':
    unittest.main()
