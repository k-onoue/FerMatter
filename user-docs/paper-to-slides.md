---
title: "From your paper to a slide"
description: "Find, edit and export equations in FerMatter."
sidebar:
  order: 2
---

1. **Open Project** (`⌘O`): select your LaTeX or Typst folder.
2. Select a Library card; double-click to edit a presentation variant.
3. Choose **SVG**, then copy (`⇧⌘C`) or drag into PowerPoint.

## Library

![Library: document outline, Block filter, equation cards and output controls](../site/public/assets/guide/library.webp)

**Save Variant** (`⌘S`) keeps the manuscript unchanged. [Output size and format →](export.md)

## PDF

![PDF pane displaying the compiled synthetic paper](../site/public/assets/guide/pdf.webp)

**PDF** (`⌥⌘P`): hover over a mapped equation and drag its grab handle. Library cards, Source previews and Instant previews can also be dragged.

## Instant

![Instant: equation source beside its preview, with size, color, format and Copy controls](../site/public/assets/guide/instant.webp)

Use the lightning button for the project's definitions, or **Instant** on Projects for a standalone template. Drafts save automatically.

## Source

![Source files and outline beside the editor and compiled paper PDF](../site/public/assets/guide/source.webp)

**Source** (`⌥⌘E`) edits actual project files. **Settings → Editor** selects automatic saving or manual `⌘S`.

## Figures

![CeTZ drawings shown as Library cards](../site/public/assets/guide/figures.webp)

**Figures** filters TikZ/CeTZ drawings. Double-click to make a presentation variant; output size scales the whole drawing.

## Project folders

| Action | Folder behavior |
| --- | --- |
| Open Project | Opens in place |
| Drop a folder / import a ZIP | Copies into the default project folder |
| Delete (`⌘⌫`) | Moves the actual source folder to Finder Trash |

## If it does not render

Check **Project Settings** (main file/compiler) and the failed item's log. After external edits, **Rescan** and **Rebuild PDF** (`⇧⌘R`). A failed build keeps the previous PDF.

[Required tools and extraction limits →](requirements.md)
