import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'Resources'))
import document_worker as worker
import tex_worker
import typst_worker

EXAMPLES = Path(__file__).resolve().parents[2] / 'examples'
ENV = tex_worker.tool_env()


def available(*names):
    return all(shutil.which(name, path=ENV['PATH']) for name in names)


def snapshot(root):
    return {p.relative_to(root).as_posix(): p.read_bytes() for p in root.rglob('*') if p.is_file()}


def build(root, main, cache, force=False):
    events = []
    worker.build(dict(root=str(root), main=main, engine='pdflatex', cache=str(cache), force=force), lambda **e: events.append(e), ENV)
    return events[-1]['document']


def sync(document, action, **args):
    events = []
    worker.sync(dict(document=document, action=action, **args), lambda **e: events.append(e), ENV)
    return events[-1]


def layout(pdf):
    text = subprocess.run(['pdftotext', '-bbox', str(pdf), '-'], capture_output=True, text=True, env=ENV, check=True).stdout
    return text[text.index('<doc>'):]


class DocumentTests(unittest.TestCase):
    @unittest.skipUnless(available('latexmk', 'pdflatex', 'synctex'), 'TeX tools unavailable')
    def test_latex_pdf_round_trip_cache_and_failed_rebuild(self):
        with tempfile.TemporaryDirectory() as temp:
            root, cache = Path(temp) / 'Source with spaces', Path(temp) / 'Cache'
            shutil.copytree(EXAMPLES / 'demo-paper', root)
            original = snapshot(root)
            document = build(root, 'main.tex', cache)
            self.assertTrue(document['synchronized'])
            self.assertEqual(snapshot(root), original)
            self.assertEqual(build(root, 'main.tex', cache)['pdfPath'], document['pdfPath'])
            headings = []
            cards, _, _ = tex_worker.extract(root, 'main.tex', outline=headings)
            for card in [c for c in cards if c['kind'] == 'block']:
                anchor = sync(document, 'pdf_forward', targetID=card['id'], file=card['file'], line=(card['line'] + card['endLine']) // 2)['anchor']
                self.assertGreater(anchor['page'], 0)
                source = sync(document, 'pdf_reverse', **{k: anchor[k] for k in ('page', 'x', 'y')})
                self.assertEqual(source['file'], card['file'])
                self.assertGreaterEqual(source['line'], card['line'])
                self.assertLessEqual(source['line'], card['endLine'])
            (root / 'main.tex').write_text('\\documentclass{article}\\begin{document}\\undefinedCommand\\end{document}')
            with self.assertRaises(ValueError):
                build(root, 'main.tex', cache, force=True)
            self.assertTrue(Path(document['pdfPath']).is_file())
            # Even after a failed rebuild, the old SyncTeX source copy remains usable.
            self.assertTrue(Path(document['sourceRoot'], cards[0]['file']).is_file())

    @unittest.skipUnless(available('typst', 'pdftotext'), 'Typst tools unavailable')
    def test_typst_preserves_layout_and_source_and_maps_all_targets(self):
        with tempfile.TemporaryDirectory() as temp:
            root, cache = Path(temp) / 'Source', Path(temp) / 'Cache'
            shutil.copytree(EXAMPLES / 'demo-typst', root)
            original = snapshot(root)
            clean = Path(temp) / 'original.pdf'
            subprocess.run(['typst', 'compile', '--root', str(root), str(root / 'main.typ'), str(clean)], env=ENV, check=True)
            document = build(root, 'main.typ', cache)
            self.assertEqual(snapshot(root), original)
            self.assertEqual(layout(clean), layout(document['pdfPath']))
            headings = []
            cards, _, _ = typst_worker.extract(root, 'main.typ', outline=headings)
            self.assertEqual({a['targetID'] for a in document['anchors']}, {x['id'] for x in cards + headings})
            for card in cards:
                anchor = sync(document, 'pdf_forward', targetID=card['id'])['anchor']
                target = sync(document, 'pdf_reverse', **{k: anchor[k] for k in ('page', 'x', 'y')})
                self.assertEqual(target['id'], card['id'])
            self.assertEqual(build(root, 'main.typ', cache)['pdfPath'], document['pdfPath'])
            (root / 'symbols.typ').write_text((root / 'symbols.typ').read_text() + '\n// changed\n')
            self.assertNotEqual(build(root, 'main.typ', cache)['pdfPath'], document['pdfPath'])

    @unittest.skipUnless(available('typst', 'pdftotext'), 'Typst tools unavailable')
    def test_typst_page_breaks_and_repeated_includes(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp) / 'Source'; root.mkdir()
            (root / 'main.typ').write_text('#set page(height: 180pt, width: 240pt, margin: 20pt)\n#include "part.typ"\n#pagebreak()\n#include "part.typ"\n')
            (root / 'part.typ').write_text('= Title\nText $x + y$.\n\n$ z^2 = 1 $\n')
            clean = Path(temp) / 'original.pdf'
            subprocess.run(['typst', 'compile', '--root', str(root), str(root / 'main.typ'), str(clean)], env=ENV, check=True)
            document = build(root, 'main.typ', Path(temp) / 'Cache')
            self.assertEqual(layout(clean), layout(document['pdfPath']))
            self.assertEqual([a['page'] for a in document['anchors']], [1, 1, 1, 2, 2, 2])
            self.assertEqual(len({a['targetID'] for a in document['anchors']}), 6)
            self.assertEqual(sync(document, 'pdf_reverse', page=99, x=50, y=50)['event'], 'pdf_unmapped')

    def test_synctex_multiple_records_and_fingerprint_assets(self):
        records = worker.synctex_records('SyncTeX result begin\nOutput:test.pdf\nPage:1\nx:20\ny:30\nOutput:test.pdf\nPage:2\nx:21\ny:40\nSyncTeX result end')
        self.assertEqual([r['Page'] for r in records], ['1', '2'])
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp); (root / 'main.tex').write_text('test')
            before = worker.fingerprint(root, 'main.tex')
            (root / 'main.aux').write_text('generated'); (root / 'main.pdf').write_bytes(b'pdf')
            self.assertEqual(before, worker.fingerprint(root, 'main.tex'))
            (root / 'reference.bbl').write_text('source bibliography')
            self.assertNotEqual(before, worker.fingerprint(root, 'main.tex'))

if __name__ == '__main__':
    unittest.main()
