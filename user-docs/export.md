---
title: "Choose size and format"
description: "Choose image format, size and color; copy, drag, export or reopen SVG."
sidebar:
  order: 4
---

These settings control the image you send to another app. For finding or editing its source, see [From your paper to a slide](paper-to-slides.md).

## Choose a format

| Format | Use |
| --- | --- |
| **SVG** | Vector artwork for PowerPoint; includes editable FerMatter source metadata |
| **PDF** | Vector artwork for apps and workflows that accept PDF images |
| **PNG** | Raster artwork with a transparent background |
| **JPEG** | Raster artwork on a white background |

Choose the format in the Library output bar or the Instant controls.

![Library output bar: SVG format, Default size, output font size, Copy image and Export image](../site/public/assets/guide/output-controls.webp)

For PowerPoint, use **SVG**. PDF artwork can be rasterized when PowerPoint exports a `.pptx` to PDF. [Reported issue](https://learn.microsoft.com/en-ie/answers/questions/5701486/recurring-issue-save-as-pdf-converts-vector-graphi).

## Output size

**Output font size** accepts 1–200 pt. For project artwork, the first available setting wins:

| Priority | Setting | Where to change it |
| --- | --- | --- |
| 1 | Item override | Library variant editor or the selected card's inspector (`⌥⌘I`) |
| 2 | Project | Library output bar |
| 3 | App default | **Settings → Projects → Default output font size** |

Click **Default** on an item or project to inherit the next setting. Instant has its own controls; standalone **Save defaults** remembers its output settings.

Size applies to copy, drag and export. Figures scale as a whole, including labels.

## Output color

Use the color picker in a Library variant or Instant to override output color. **Reset color** in a Library variant restores the source color. App themes and display zoom do not change exported artwork.

## Copy, drag or export

| Action | Result |
| --- | --- |
| **Copy image** (`⇧⌘C`) | Puts the selected artwork on the clipboard for pasting |
| Drag a card, preview or mapped PDF equation | Sends artwork directly to the receiving app |
| **Export image** (`⇧⌘S`) | Saves one image to a chosen file |

### Several equations

Use Command-click to toggle cards or Shift-click for a range, then copy to paste separate images. Select a single card for file export. Multiple-image dragging depends on the receiving app.

## Re-edit SVG

1. Keep the original FerMatter **SVG** export; other apps may strip its source metadata.
2. Choose **Open SVG** in Instant, or drop the SVG into FerMatter.
3. Edit the restored source in the matching project's variant editor or Instant. Missing or ambiguous projects open in standalone Instant.

SVG includes equation/template source, not the whole manuscript. Project imports, packages and fonts may still be needed. PDF, PNG and JPEG do not restore editable source.
