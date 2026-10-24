---
title: "Use TikZ or CeTZ drawings"
description: "Extract drawings and edit presentation variants."
sidebar:
  order: 9
---

- Choose **Figures** in Library to filter drawings. Double-click one to edit a presentation variant.
- **New Figure** (`⌥⌘N`) starts from a drawing template.
- **Output font size** scales the whole drawing, including labels. Captions are omitted.
- Use **Rescan** to add supported figures to an older Library.

## Supported source

- **TikZ**: literal `tikzpicture` environments with the project's preamble, styles and libraries.
- **CeTZ**: literal canvases, supported imports and common figure/alignment wrappers. The built-in template uses CeTZ 0.5.2 with Typst 0.14+.
- Dynamic drawings, wrapper-local variables, overlays and externalized graphics may need manual adjustment.

[Compiler requirements](requirements.md) · [Choose size and format](export.md).
