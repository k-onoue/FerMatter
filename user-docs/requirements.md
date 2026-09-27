# Requirements

- Development prototype; no signed, notarized public download yet.
- macOS 26+; Apple Silicon.
- Python 3, plus the tools for your project format; runtimes are not bundled.
- Xcode with the macOS 26 SDK is required to build from source, not to run a packaged app.
- Open projects you trust: TeX and `latexmkrc` can execute code; this prototype is not an OS sandbox.

## Typst projects

- Typst CLI 0.14+.
- Project fonts/packages; uncached packages need internet access initially.
- No TeX installation or SVG conversion tools required.
- Reference: [Typst installation](https://github.com/typst/typst#installation).

## LaTeX projects

- TeX Live/MacTeX: selected TeX engine, `latexmk`, `preview.sty`, `pdfcrop`.
- SyncTeX for PDF/source navigation.
- `pdf2svg`, Poppler (`pdfinfo`, `pdftocairo`).
- DVI projects: `dvips`, Ghostscript (`ps2pdf`).
- Additional packages/fonts used by your manuscript.
- References: [MacTeX](https://www.tug.org/mactex/), [pdf2svg](https://github.com/dawbarton/pdf2svg), [Poppler](https://poppler.freedesktop.org/).

## Tool discovery

- Python locations: `/opt/homebrew/bin/python3`, `/usr/local/bin/python3`, `/usr/bin/python3`.
- Compiler/tool search includes `/Library/TeX/texbin`, `/opt/homebrew/bin`, `/usr/local/bin`, and system paths.
- In **Project Settings**, choose the main file and compiler when automatic selection is insufficient.
- [Extraction limits and troubleshooting](limits.md).
