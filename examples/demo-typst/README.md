# Typst demonstration

A fictional teaching note using standard textbook mathematics. All prose and
equations are public sample material; no private manuscript is included.

Open this folder or `main.typ` in Mathlidelux. The app follows the two literal
includes, keeps each file's definitions and imports, and renders SVGs with Typst.
The outline follows the document title and three nested section headings.
The fitting section uses a different text size to exercise per-equation export
scaling. `symbols.typ` is a helper module, not the main document.

To compile the complete note with Typst 0.14 or later:

```sh
typst compile main.typ demo.pdf
```

Generated PDFs and caches are not included in the downloadable sample.
