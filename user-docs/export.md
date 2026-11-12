---
title: "Size and format"
description: "Image formats, output sizing, colors, and SVG round-trip editing."
sidebar:
  order: 4
---

Configure output image settings in the Library bottom bar or Instant header.

<span id="choose-a-format"></span>

## Image formats

| Format | Best for |
| --- | --- |
| **SVG** | Vector artwork for PowerPoint, Keynote, and web. Preserves editable FerMatter source metadata. |
| **PDF** | Vector artwork for vector design applications. |
| **PNG** | High-resolution raster image with transparent background. |
| **JPEG** | Raster image with opaque white background. |

![Library output bar: SVG format, Default size, output font size, Copy image and Export image](../site/public/assets/guide/output-controls.webp)

For PowerPoint, prefer **SVG**. PowerPoint may rasterize imported PDF vector graphics during PDF presentation exports. [Reported issue](https://learn.microsoft.com/en-ie/answers/questions/5701486/recurring-issue-save-as-pdf-converts-vector-graphi).

On Windows, SVG copies contain vector artwork and files. Choose **PNG** when the receiving app needs a bitmap.

## Output size

Font size can be set continuously from 1 to 200 pt. Priority order:

1. **Card override:** In the Library variant editor or inspector (`⌥⌘I`).
2. **Project default:** In the Library output bar.
3. **Global default:** In **Settings → Projects → Default output font size**.

Diagrams (TikZ / CeTZ) scale proportionally as a whole, including labels.

Click **Default** to inherit the next setting. Instant has separate controls; standalone **Save defaults** remembers its output settings.

## Output color

Set custom colors via the color picker in the variant editor or Instant. **Reset color** restores original document colors. Application themes and display zoom do not affect exported assets.

<span id="copy-drag-or-export"></span>

## Copy, drag, and export

| Action | Usage |
| --- | --- |
| **Copy image** (`⇧⌘C`) | Copies selected artwork to the system clipboard |
| **Drag and drop** | Drag a card, preview, or mapped PDF equation straight into another app |
| **Export image** (`⇧⌘S`) | Saves the rendered asset as an image file |

<span id="several-equations"></span>

Batch selection is supported via `⌘`-click and `⇧`-click for copying multiple items. Select one card for file export; multiple-image dragging depends on the receiving app.

<span id="re-edit-svg"></span>

## Re-editing SVGs

FerMatter embeds source code inside exported SVGs:

1. Drop any FerMatter-exported SVG back into the application, or select **Open SVG** in Instant.
2. The original LaTeX/Typst code restores into the corresponding project or Instant scratchpad.
3. Review the restored source and press **Render**. It remains pending across reopening and history restoration until you do so.

Keep the original SVG; other apps may strip its metadata. Project imports, packages and fonts may still be needed. PDF, PNG, and JPEG files do not contain embedded source metadata.
