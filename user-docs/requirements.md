---
title: "Requirements"
description: "Supported Macs and external tools for LaTeX and Typst."
sidebar:
  order: 11
---

- Development prototype; no signed, notarized public download yet.
- Apple Silicon Mac. The app targets macOS 14+ and has been verified on macOS 26.
- No Python, Node.js, Rust toolchain, Swift SDK, or Xcode installation is needed to run the packaged app.
- Install the external compiler tools for your project format.
- Building from source requires Rust, Node.js 22+ and Xcode command-line tools.
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

- First-launch setup and **Settings → Tools** let you choose compiler/converter executables; blank paths use automatic detection.
- Compiler/tool search includes `/Library/TeX/texbin`, `/opt/homebrew/bin`, `/usr/local/bin`, and system paths.
- Custom executable folders are searched first for companion tools. Exact overrides take precedence for commands invoked by name, including engines launched by `latexmk`; a project's explicit absolute commands in `latexmkrc` remain authoritative.
- Paths are saved in `tools.json`; no shell configuration changes are needed. [Tool settings](preferences.md#compiler-tools).
- In **Project Settings**, choose the main file and compiler when automatic selection is insufficient.
- [Extraction limits and troubleshooting](limits.md).
