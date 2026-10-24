---
title: "Requirements"
description: "Rendering tools and compiler settings."
sidebar:
  order: 11
---

**Apple Silicon · macOS 14+ · LaTeX or Typst**

[Copy installation commands →](https://fermatter.github.io/download/#install)

| Project | Tools |
| --- | --- |
| Typst / CeTZ | Typst CLI 0.14+ |
| LaTeX / TikZ | TeX engine, `latexmk`, `preview.sty`, `pdfcrop`, `pdf2svg`, Poppler |
| DVI-based LaTeX | Also `dvips` and Ghostscript |

Project packages/fonts remain required. LaTeX PDF navigation uses SyncTeX.

## Tool discovery

![Settings → Tools: compiler executable paths, automatic detection, Check paths and Apply](../site/public/assets/guide/tools.webp)

**Settings → Tools** detects installed tools. Set an executable path and **Apply** to override; clear it to restore detection. **Project Settings** chooses the main file/compiler.

## Extraction limits

FerMatter extracts literal equations and drawings. Generated macros/loops and document-wide Typst state may need manual adjustment. A PDF alone cannot be imported as an equation project.
