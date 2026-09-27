import contextlib
import io
import json
from pathlib import Path
import shutil
import subprocess
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch
import xml.etree.ElementTree as ET

sys.path.insert(0, str(Path(__file__).parents[1] / 'Resources'))
import tex_worker as dispatcher
import typst_worker as worker


class TypstExtractionTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()

    def write(self, name, text):
        path = self.root / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)

    def scan(self, **kwargs):
        with contextlib.redirect_stdout(io.StringIO()) as output:
            dispatcher.main(dict(action='scan', root=str(self.root), **kwargs))
        return next(e for e in map(json.loads, output.getvalue().splitlines()) if e['event'] == 'scanned')

    def test_comments_raw_strings_and_block_whitespace(self):
        self.write('main.typ', r'''// $hidden$
/* $hidden$ /* nested */ $hidden$ */
`$raw$` ```typ
$raw$
```
Price \$5; $x$, $ x^2 $, $ x$, $x $, $"cost $5" = 5$.
#let hidden = $not-a-card$
#let fn(x) = { let value = "$not-math$"; $x$ }
$ y = 2 $ <eq:second-result>
''')
        cards, warnings, _ = worker.extract(self.root, 'main.typ')
        self.assertEqual([c['kind'] for c in cards], ['inline', 'block', 'inline', 'inline', 'inline', 'block'])
        self.assertEqual(cards[-1]['name'], 'Second result')
        self.assertEqual(cards[-2]['tex'], '"cost $5" = 5')
        self.assertEqual(warnings, [])

    def test_include_order_scopes_duplicates_and_stable_ids(self):
        source = '#set text(size: 11pt)\n$a$\n#include "parts/child.typ"\n$a$'
        self.write('main.typ', source)
        self.write('parts/child.typ', '$ b $\n#include "/last.typ"')
        self.write('last.typ', '$c$')
        cards, warnings, files = worker.extract(self.root, 'main.typ')
        self.assertEqual([c['tex'] for c in cards], ['a', 'b', 'c', 'a'])
        self.assertEqual(len(set(c['id'] for c in cards)), 4)
        self.assertEqual([f['file'] for f in cards[2]['typstContext']], ['main.typ', 'parts/child.typ', 'last.typ'])
        self.assertEqual(warnings, [])
        self.assertEqual(len(files), 3)
        self.write('main.typ', 'An inserted sentence.\n' + source)
        updated, _, _ = worker.extract(self.root, 'main.typ')
        self.assertEqual([c['id'] for c in cards], [c['id'] for c in updated])
        self.assertNotEqual(cards[2]['typstContext'], updated[2]['typstContext'])

    def test_cycles_missing_includes_and_root_escape(self):
        self.write('main.typ', '#include "child.typ"\n#include "../outside.typ"')
        self.write('child.typ', '$x$\n#include "main.typ"\n#include "missing.typ"')
        cards, warnings, _ = worker.extract(self.root, 'main.typ')
        self.assertEqual(len(cards), 1)
        self.assertEqual(len(warnings), 3)

    def test_generated_content_is_skipped_with_warning(self):
        self.write('main.typ', '#for i in range(3) [$i$]\n#show: template\n#box[$hidden$]\n$x$\n$ unfinished')
        cards, warnings, _ = worker.extract(self.root, 'main.typ')
        self.assertEqual([c['tex'] for c in cards], ['x'])
        self.assertEqual(len(warnings), 4)

    def test_setup_and_computed_content_do_not_swallow_following_math(self):
        self.write('main.typ', '#set text(size: 12pt) $a$\n#box[$hidden$] $b$\n#{ let x = "$hidden$"; [Hello] } $c$\n#context [$hidden$]\n$d$')
        cards, warnings, _ = worker.extract(self.root, 'main.typ')
        self.assertEqual([c['tex'] for c in cards], ['a', 'b', 'c', 'd'])
        self.assertEqual(worker.setup_before((self.root / 'main.typ').read_text(), cards[0]['typstContext'][0]['offset']), '#set text(size: 12pt)')
        self.assertEqual(len(warnings), 3)

    def test_main_detection_format_filter_and_mixed_folder(self):
        self.write('main.typ', '$x$')
        self.write('helper.typ', '#let f = 1')
        self.write('build/ignored.typ', '$z$')
        result = self.scan()
        self.assertEqual(result['main'], 'main.typ')
        self.assertEqual(result['format'], 'typst')
        self.assertEqual(result['candidates'], ['main.typ', 'helper.typ'])
        self.assertFalse(result['hasRC'])
        self.write('main.tex', '\\documentclass{article}\n\\begin{document}$y$\\end{document}')
        with self.assertRaisesRegex(ValueError, 'both LaTeX and Typst'):
            self.scan()
        self.assertEqual(self.scan(main='main.typ')['format'], 'typst')
        self.assertEqual(self.scan(main='main.tex')['candidates'], ['main.tex'])
        with self.assertRaises(ValueError):
            self.scan(main='../outside.typ')

    def test_equation_wrapping_and_invalid_multiple_equations(self):
        self.assertEqual(worker.math_body(dict(tex='$ x^2 $', kind='inline')), '$x^2$')
        self.assertEqual(worker.math_body(dict(tex='x^2', kind='block')), '$ x^2 $')
        for body in ('', '$x$ $y$', '$unclosed'):
            with self.assertRaises(ValueError):
                worker.math_body(dict(tex=body, kind='block'))

    def test_missing_typst_error_is_actionable(self):
        with patch.object(worker.shutil, 'which', return_value=None):
            with self.assertRaisesRegex(ValueError, 'Install the Typst CLI'):
                worker.render({}, lambda **_: None, {'PATH': ''})


@unittest.skipUnless(shutil.which('typst', path=dispatcher.tool_env()['PATH']), 'Typst CLI is not installed')
class TypstRenderingTests(unittest.TestCase):
    def test_real_compiler_scopes_font_metrics_cache_and_failure_isolation(self):
        with tempfile.TemporaryDirectory() as folder:
            base = Path(folder).resolve()
            root, cache = base / 'project', base / 'cache'
            (root / 'parts').mkdir(parents=True)
            (root / 'symbols.typ').write_text('#let symbol = $z$\n')
            (root / 'main.typ').write_text('#set text(size: 16pt)\n#let symbol = $a$\n$symbol^2$\n#include "parts/child.typ"\n$symbol$')
            (root / 'parts/child.typ').write_text('#import "../symbols.typ": symbol\n#set text(size: 20pt)\n$ symbol = 2 $')
            original = {p.relative_to(root): p.read_bytes() for p in root.rglob('*') if p.is_file()}
            cards, warnings, _ = worker.extract(root, 'main.typ')
            self.assertEqual(warnings, [])
            broken = dict(cards[0], id='broken', tex='unknown-name(x)')
            custom = dict(id='custom', tex='symbol^3', kind='inline')
            request = dict(action='render', format='typst', root=str(root), main='main.typ', cache=str(cache), cards=[cards[0], broken, *cards[1:], custom])
            with contextlib.redirect_stdout(io.StringIO()) as output:
                dispatcher.main(request)
            events = [e for e in map(json.loads, output.getvalue().splitlines()) if e['event'] == 'card']
            self.assertEqual([e['status'] for e in events], ['ready', 'failed', 'ready', 'ready', 'ready'])
            ready = [e for e in events if e['status'] == 'ready']
            self.assertEqual([e['fontSize'] for e in ready], [16, 20, 16, 16])
            timestamps = {}
            for event in ready:
                svg = Path(event['svgPath'])
                parsed = ET.parse(svg).getroot()
                self.assertEqual(parsed.tag, '{http://www.w3.org/2000/svg}svg')
                self.assertTrue(parsed.get('width').endswith('pt'))
                self.assertTrue(parsed.findall('.//{http://www.w3.org/2000/svg}path'))
                self.assertFalse(parsed.findall('.//{http://www.w3.org/2000/svg}image'))
                self.assertEqual(Path(event['pngPath']).read_bytes()[:8], b'\x89PNG\r\n\x1a\n')
                width, height = struct.unpack('>II', Path(event['pngPath']).read_bytes()[16:24])
                self.assertGreaterEqual(max(width, height), 850)
                self.assertLessEqual(max(width, height), 901)
                timestamps[event['id']] = svg.stat().st_mtime_ns
            self.assertTrue(Path(events[1]['logPath']).is_file())
            request['cards'] = [*cards, custom]
            cached = []
            worker.render(request, lambda **e: cached.append(e), dispatcher.tool_env())
            self.assertEqual(len(cached), 4)
            self.assertTrue(all(e['status'] == 'ready' for e in cached))
            self.assertEqual(timestamps, {e['id']: Path(e['svgPath']).stat().st_mtime_ns for e in cached})
            self.assertEqual(original, {p.relative_to(root): p.read_bytes() for p in root.rglob('*') if p.is_file()})
            # An imported definition changes the cache identity, not the source card identity.
            (root / 'symbols.typ').write_text('#let symbol = $w$\n')
            changed = []
            request['cards'] = [cards[1]]
            worker.render(request, lambda **e: changed.append(e), dispatcher.tool_env())
            self.assertEqual(changed[0]['status'], 'ready')
            self.assertNotEqual(changed[0]['svgPath'], ready[1]['svgPath'])

    def test_public_sample_full_document_and_every_equation(self):
        root = Path(__file__).parents[2] / 'examples/demo-typst'
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            result = subprocess.run(['typst', 'compile', str(root / 'main.typ'), str(output / 'demo.pdf')], env=dispatcher.tool_env(), capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            cards, warnings, _ = worker.extract(root, 'main.typ')
            # The shipped sample intentionally omits the old angular-speed example.
            self.assertEqual(len(cards), 20)
            self.assertEqual(warnings, [])
            events = []
            worker.render(dict(root=str(root), main='main.typ', cache=str(output / 'cache'), cards=cards), lambda **e: events.append(e), dispatcher.tool_env())
            self.assertEqual(len(events), len(cards))
            self.assertTrue(all(e['status'] == 'ready' for e in events), [e for e in events if e['status'] != 'ready'])
            self.assertEqual({e['fontSize'] for e in events}, {11, 12})


if __name__ == '__main__':
    unittest.main()
