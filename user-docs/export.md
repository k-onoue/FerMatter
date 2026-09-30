# Choose size and format

1. Select an equation or figure.
2. Choose **SVG**, **PNG**, **JPEG**, or **PDF**.
3. Keep **Default** or enter an **Output font size** in points, including decimals; press Enter or leave the field to apply it.
4. Copy (`⌘C` with cards selected, or `⇧⌘C`), export (`⇧⌘S`), or drag from the card, preview, or [mapped PDF artwork](navigate-paper.md).

| Format | Use |
| --- | --- |
| SVG | Scalable artwork; default; math glyphs stored as paths |
| PDF | Vector artwork for apps that accept PDF |
| PNG | Transparent raster image |
| JPEG | Raster image with a white background |

- Raster exports use 216 dpi at the selected physical size.
- **Settings → Projects → Default output font size** starts at **30 pt**. Custom sizes accept **1–200 pt**.
- **Default** follows the next setting: item → project → app default. The number field shows the effective size; click **Default** to clear an override.
- Library copy/drag/export, variant previews, Source previews and PDF drags follow this rule. Instant keeps its own [saved defaults](instant.md).
- Existing explicit sizes remain unchanged. Legacy source-size settings keep their scale; mixed source sizes show **—** until one number is chosen.
- An item override applies to all formats, including an unsaved editor preview.
- Font-size changes scale the whole drawing for figures, including their labels.
- Vector artwork stays vector; embedded raster images remain raster.
- Copy/paste into PowerPoint is demonstrated here; drag behavior depends on the receiving app.
- Hover over a PDF equation or figure and wait for its grab handle to drag the compiled source artwork. PDF drags export one item, using the project format and its source card's output size override if set.

## Several equations

- Command-click individual cards; Shift-click a range; drag a selection rectangle from empty space.
- `⌘A` selects the current filtered view; Escape clears selection.
- Copy (`⌘C` or the copy button), then paste into PowerPoint (`⌘V`); each selected item becomes a separate image.
- SVG remains vector artwork. Multiple-image drag support depends on the receiving app.
- Export saves one selected image; use copy or drag for several items.
- Use the context menu to assign a label to the selection.

## Troubleshooting

- If the receiving app rejects SVG, export a file or try PNG/PDF.
- Select PDF to prepare a missing vector PDF for an older cached item.
- Verify the final exported slide/PDF in your presentation app.
