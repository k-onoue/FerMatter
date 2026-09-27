import importlib.util
import tempfile
from pathlib import Path
import unittest
import zipfile
import sys

sys.path.insert(0, str(Path(__file__).parents[1] / 'Resources'))

spec = importlib.util.spec_from_file_location('worker', Path(__file__).parents[1] / 'Resources/tex_worker.py')
w = importlib.util.module_from_spec(spec)
spec.loader.exec_module(w)

class ExtractionTests(unittest.TestCase):
    def extract(self, body, dependencies=None):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root/'main.tex').write_text('\\documentclass{article}\n\\newcommand{\\hidden}{$bad$}\n\\begin{document}\n' + body + '\n\\end{document}')
            for path, text in (dependencies or {}).items():
                (root/path).parent.mkdir(exist_ok=True, parents=True)
                (root/path).write_text(text)
            return w.extract(root, 'main.tex')

    def test_all_delimiters_comments_duplicates_and_verbatim(self):
        cards, warnings, _ = self.extract(r'''% $hidden$
Price \$5, $x$, $x$, \(y\), $$z$$, \[w\].
\verb|$not math$|
\begin{verbatim}$skip$\end{verbatim}
\begin{tikzpicture}$skip$\end{tikzpicture}
\newcommand{\xx}[1]{$#1$}
\begin{align}a&=b\\c&=\begin{cases}1&x>0\\0&x<0\end{cases}\end{align}
''')
        self.assertEqual([c['kind'] for c in cards], ['inline']*3 + ['block']*2 + ['figure', 'block'])
        self.assertEqual([c['tex'] for c in cards[:5]], ['x','x','y','z','w'])
        self.assertEqual(len(set(c['id'] for c in cards)), 7)
        self.assertEqual(warnings, [])
        self.assertIn('cases', cards[-1]['tex'])

    def test_include_order_and_includegraphics_boundary(self):
        cards, warnings, files = self.extract(r'$a$\input{parts/child}\includegraphics{image.pdf}$c$', {'parts/child.tex': '$b$'})
        self.assertEqual([c['tex'] for c in cards], ['a','b','c'])
        self.assertEqual(files, ['main.tex', 'parts/child.tex'])
        self.assertEqual(warnings, [])

    def test_ids_survive_line_insertion(self):
        before, _, _ = self.extract('$a$\n$b$')
        after, _, _ = self.extract('An inserted sentence.\n\n$a$\n$b$')
        self.assertEqual([c['id'] for c in before], [c['id'] for c in after])
        self.assertNotEqual(before[0]['line'], after[0]['line'])

    def test_cycles_and_escaping_root(self):
        cards, warnings, _ = self.extract(r'\input{child}', {'child.tex': r'$a$\input{child}\input{../outside}'})
        self.assertEqual(len(cards),1)
        self.assertEqual(len(warnings),2)

    def test_numbering_removed_without_blank_paragraph(self):
        result = w.math_body(dict(tex='\\begin{equation}\nx=1\n\\label{eq:x}\n\\end{equation}', kind='block'))
        self.assertEqual(result, '\\begin{equation*}\nx=1\n\\end{equation*}')

    def test_zip_traversal_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            archive = Path(directory)/'bad.zip'
            with zipfile.ZipFile(archive,'w') as z: z.writestr('../outside.tex','bad')
            with self.assertRaises(ValueError):
                w.import_zip(dict(path=str(archive),destination=str(Path(directory)/'imports')))
            self.assertFalse((Path(directory)/'outside.tex').exists())

if __name__ == '__main__': unittest.main()
