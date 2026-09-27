from contextlib import redirect_stdout
import io
import json
import re
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
import xml.etree.ElementTree as ET

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'Resources'))
import tex_worker
import typst_worker
import document_worker
from test_document_worker import available, snapshot, layout, build, sync

EXAMPLES = Path(__file__).resolve().parents[2] / 'examples'
ENV = tex_worker.tool_env()


class FigureTests(unittest.TestCase):
    def test_tikz_picture_is_one_item_including_nested_math_and_scope(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = r'''\documentclass{article}
\usepackage{tikz}
\begin{document}
\section{Shapes}
\begin{figure}
\begin{tikzpicture}[blue]
\begin{scope}\node {$x^2$};\end{scope}
\draw (0,0) -- (1,1);
\end{tikzpicture}
\caption{A figure}
\end{figure}
$z$
\begin{verbatim}\begin{tikzpicture}bad\end{tikzpicture}\end{verbatim}
\end{document}'''
            (root / 'main.tex').write_text(source)
            outline = []; cards, warnings, _ = tex_worker.extract(root, 'main.tex', outline=outline)
            self.assertEqual([c['kind'] for c in cards], ['figure', 'inline'])
            self.assertIn('$x^2$', cards[0]['tex'])
            self.assertNotIn('caption', cards[0]['tex'])
            self.assertEqual(cards[0]['outlinePath'], [outline[0]['id']])
            self.assertEqual(tex_worker.math_body(cards[0]), cards[0]['tex'])
            self.assertEqual(warnings, [])

    def test_cetz_qualified_aliased_wildcard_and_wrapped_canvases(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            source = '''#import "@preview/cetz:0.5.2" as drawing
#import "@preview/cetz:0.5.2": canvas as sketch
#import "@preview/cetz:0.5.2": *
= Diagrams
#drawing.canvas({ content((0,0), $x$) })
#figure(sketch({ content((0,0), [A]) }), caption: [A sketch])
#canvas({ /* canvas({bad}) */ content((0,0), "canvas({bad})") })
$y$
#let hidden() = drawing.canvas({ content((0,0), $hidden$) })
#for n in range(3) { drawing.canvas({ content((0,0), $loop$) }) }
```typ
#drawing.canvas({ bad() })
```
'''
            (root / 'main.typ').write_text(source)
            cards, warnings, _ = typst_worker.extract(root, 'main.typ')
            self.assertEqual([c['kind'] for c in cards], ['figure'] * 3 + ['inline'])
            self.assertTrue(cards[0]['tex'].startswith('#drawing.canvas'))
            self.assertTrue(cards[1]['tex'].startswith('#sketch'))
            self.assertNotIn('caption', cards[1]['tex'])
            self.assertIn('$x$', cards[0]['tex'])
            self.assertEqual(typst_worker.math_body(cards[0]), cards[0]['tex'])
            self.assertTrue(any('for' in warning for warning in warnings))

    @unittest.skipUnless(available('latexmk', 'pdf2svg', 'pdftocairo'), 'TeX tools unavailable')
    def test_tikz_real_svg_colors_cache_and_no_source_changes(self):
        self.render_sample('demo-tikz', 'main.tex', tex_worker)

    @unittest.skipUnless(available('typst'), 'Typst unavailable')
    def test_cetz_real_svg_colors_cache_and_no_source_changes(self):
        self.render_sample('demo-cetz', 'main.typ', typst_worker)

    def render_sample(self, name, main, module):
        with tempfile.TemporaryDirectory() as temp:
            root, cache = Path(temp) / 'Source', Path(temp) / 'Cache'
            shutil.copytree(EXAMPLES / name, root)
            original = snapshot(root)
            cards, _, _ = module.extract(root, main)
            self.assertEqual([c['kind'] for c in cards], ['figure', 'inline', 'figure'])
            request = dict(root=str(root), main=main, cache=str(cache), engine='project', cards=cards)
            def render():
                if module == tex_worker:
                    out = io.StringIO()
                    with redirect_stdout(out): module.render(request)
                    return [json.loads(line) for line in out.getvalue().splitlines()]
                events = []
                module.render(request, lambda **e: events.append(e), ENV)
                return events
            first = render()
            self.assertEqual([e['status'] for e in first], ['ready'] * 3, first)
            for event in (first[0], first[2]):
                svg = Path(event['svgPath']).read_text()
                tree = ET.fromstring(svg)
                self.assertGreater(len(tree.findall('.//{http://www.w3.org/2000/svg}path')), 0)
                self.assertNotIn('<image', svg)
                self.assertTrue('#3478d4' in svg.lower() or any(abs(float(r) - 20.39) < .02 and abs(float(b) - 83.14) < .02 for r, g, b in re.findall(r'rgb\(([0-9.]+)%, ([0-9.]+)%, ([0-9.]+)%\)', svg)))
                self.assertTrue(Path(event['pngPath']).is_file())
            # The triangle is 2.6 cm tall plus labels. A normal preview page used
            # to clip the apex before pdfcrop; its SVG was only 73 pt high.
            triangle = ET.parse(first[0]['svgPath']).getroot()
            self.assertGreater(float(triangle.get('viewBox').split()[3]), 95)
            times = [Path(e['svgPath']).stat().st_mtime_ns for e in first]
            self.assertEqual([Path(e['svgPath']).stat().st_mtime_ns for e in render()], times)
            # A custom edit changes the figure's vector output and keeps the source intact.
            request['cards'] = [dict(cards[0], id='custom-figure', tex=cards[0]['tex'].replace('(4,0)', '(5,0)').replace('(4, 0)', '(5, 0)'))]
            changed = render()[0]
            self.assertEqual(changed['status'], 'ready', changed)
            self.assertNotEqual(changed['svgPath'], first[0]['svgPath'])
            self.assertEqual(snapshot(root), original)

    @unittest.skipUnless(available('typst', 'pdftotext'), 'Typst tools unavailable')
    def test_cetz_pdf_instrumentation_preserves_layout_and_maps_figures(self):
        with tempfile.TemporaryDirectory() as temp:
            root = EXAMPLES / 'demo-cetz'
            pdf = Path(temp) / 'original.pdf'
            subprocess.run(['typst', 'compile', '--root', str(root), str(root / 'main.typ'), str(pdf)], check=True, env=ENV)
            document = build(root, 'main.typ', Path(temp) / 'Cache')
            self.assertEqual(layout(pdf), layout(document['pdfPath']))
            cards, _, _ = typst_worker.extract(root, 'main.typ')
            for card in cards:
                anchor = sync(document, 'pdf_forward', targetID=card['id'])['anchor']
                self.assertEqual(sync(document, 'pdf_reverse', **{k: anchor[k] for k in ('page','x','y')})['id'], card['id'])
                if card['kind'] == 'figure':
                    self.assertGreater(anchor['width'], 30)
                    self.assertEqual(sync(document, 'pdf_reverse', page=anchor['page'], x=anchor['x'] + anchor['width']/2, y=anchor['y'] + anchor['height']/2)['id'], card['id'])
            self.assertEqual(document['warnings'], [])

if __name__ == '__main__':
    unittest.main()
