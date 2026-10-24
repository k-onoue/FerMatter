---
title: "Use TikZ or CeTZ drawings"
description: "Bring source drawings into your presentation."
sidebar:
  order: 9
---

1. Open a TikZ or CeTZ project from **Projects**.
2. Choose **Figures** to filter drawings.
3. Select a drawing; copy/export it, or double-click to edit a presentation variant.
4. Use an item **Export Font Size** override to scale the whole drawing.

- Previews preserve colors and transparency; the editor uses a white canvas for legibility.
- Math inside a drawing stays with that drawing; captions are omitted.
- **New Figure** (`⌥⌘N`) starts an editable drawing template.
- **Rescan** older libraries to discover supported figures.
- TikZ: literal `tikzpicture` environments; original preamble/styles/libraries retained.
- CeTZ: literal canvases, supported imports, and common figure/alignment wrappers.
- Dynamic drawings, wrapper-local variables, overlays, and externalized graphics may need manual adjustment.
- Embedded bitmap images remain bitmap images.

## Requirements and references

- TikZ requires the project's TeX packages: [TikZ manual](https://tikz.dev/tikz-scopes).
- The sample/template uses [CeTZ 0.5.2](https://typst.app/universe/package/cetz/0.5.2/), requiring Typst 0.14+.
- Uncached Typst packages require internet access on first compilation.
- [Compiler requirements](requirements.md), [export formats](export.md).
