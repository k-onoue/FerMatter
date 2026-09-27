# A Small Study in Motion

A fictional teaching note created specifically for Mathlidelux. It contains only
synthetic prose and standard textbook equations, with no research manuscript,
personal data, external figures, or bibliography.

In Mathlidelux, choose **Open Project** and select this folder (or the downloadable
`demo-paper.zip`). `main.tex` and the included `latexmkrc` are detected automatically.

The example uses pdfLaTeX, standard TeX Live packages, and preamble-defined macros.
It includes inline math, labeled equations, matrices, cases, and multiline align
environments across four input files. Four sections and four subsections
demonstrate the document outline.

To compile the whole note independently:

```sh
latexmk main.tex
```

Mathlidelux also needs its rendering dependencies listed in `macos-app/README.md`
in the repository. Its extracted equations use the same preamble as this document.
