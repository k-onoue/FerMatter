import json
import io
import contextlib
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).parents[1] / 'Resources'))
import tex_worker
import typst_worker


class OutlineTests(unittest.TestCase):
    def test_latex_hierarchy_includes_starred_and_optional_titles(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            main = r'''\documentclass{article}
\begin{document}
$before$
\section[Short {title}]{Introduction and \textbf{motivation}}
$a$
\input{child}
$c$
\section*{Results}
\subsubsection{Skipped level}
$d$
\section{Results}
$e$
\end{document}'''
            (root / 'main.tex').write_text(main)
            (root / 'child.tex').write_text(r'\subsection{Model $x$}\begin{equation}b=1\end{equation}')
            outline = []
            cards, warnings, files = tex_worker.extract(root, 'main.tex', outline=outline)
            self.assertEqual(warnings, [])
            self.assertEqual([h['title'] for h in outline], ['Introduction and motivation', 'Model x', 'Results', 'Skipped level', 'Results'])
            self.assertEqual([len(h['path']) for h in outline], [1, 2, 1, 2, 1])
            self.assertEqual(len(cards), 6)  # Heading math is part of the title, not a card.
            self.assertEqual(cards[0]['outlinePath'], [])
            self.assertEqual(cards[2]['outlinePath'], outline[1]['path'])
            self.assertEqual(cards[3]['outlinePath'], outline[1]['path'])
            self.assertEqual(cards[4]['outlinePath'], outline[3]['path'])
            self.assertNotEqual(outline[2]['id'], outline[4]['id'])
            self.assertEqual(outline[1]['file'], 'child.tex')
            (root / 'main.tex').write_text(main.replace('$a$', 'Some extra prose.\n$a$'))
            updated = []
            tex_worker.extract(root, 'main.tex', outline=updated)
            self.assertEqual([h['id'] for h in outline], [h['id'] for h in updated])

    def test_latex_ignores_headings_in_comments_verbatim_and_definitions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'main.tex').write_text(r'''\documentclass{book}
\begin{document}
% \section{Comment}
\newcommand{\hidden}{\section{Macro}}
\def\hidden#1{\section{Definition}}
\begin{verbatim}\section{Raw}\end{verbatim}
\verb|\section{Literal}|
\part{Part}\chapter{Chapter}\section{Section}\subsection{Subsection}
\subsubsection{Subsubsection}\paragraph{Paragraph}\subparagraph{Subparagraph}$x$
\end{document}''')
            outline = []
            cards, _, _ = tex_worker.extract(root, 'main.tex', outline=outline)
            self.assertEqual([h['level'] for h in outline], list(range(7)))
            self.assertEqual(cards[0]['outlinePath'], [h['id'] for h in outline])

    def test_typst_headings_include_chain_labels_and_raw_exclusions(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / 'main.typ').write_text('''$before$
= Main *title* <main>
$a$
#include "child.typ"
$c$
== Results
=== Detail
$d$
```
= Hidden
```
// == Hidden
#let hidden = [
== Hidden
]
''')
            (root / 'child.typ').write_text('== Model $x$\n$b$')
            outline = []
            cards, warnings, _ = typst_worker.extract(root, 'main.typ', outline=outline)
            self.assertEqual(warnings, [])
            self.assertEqual([h['title'] for h in outline], ['Main title', 'Model x', 'Results', 'Detail'])
            self.assertEqual([len(h['path']) for h in outline], [1, 2, 2, 3])
            self.assertEqual([c['tex'] for c in cards], ['before', 'a', 'b', 'c', 'd'])
            self.assertEqual(cards[3]['outlinePath'], outline[1]['path'])
            with contextlib.redirect_stdout(io.StringIO()) as output:
                tex_worker.main(dict(action='scan', root=str(root)))
            scanned = json.loads(output.getvalue().splitlines()[0])
            self.assertEqual(scanned['outline'], outline)


if __name__ == '__main__':
    unittest.main()
