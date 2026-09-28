# Find an equation in the paper

1. Open your project and choose **PDF** (`⌥⌘P`).
2. Use the outline to narrow the Library, then select an equation card to reveal and highlight its PDF location.
3. Click an equation/figure in the PDF to select its Library card or Source range.
4. Hover over a mapped equation/figure in the PDF. Once the grab handle appears, drag it into your slide; alternatively copy its card or [edit a presentation version](edit-equation.md).

- The outline follows headings across included files; parent headings include descendants.
- Labels are manual groups, separate from the document outline.
- Source mode opens the matching file and equation range. Placing the caret in an equation also highlights its PDF target.
- Custom variants navigate to their original source; newly created equations have no paper location.
- Resize the PDF with its divider; its width is remembered.
- PDF drags use the displayed document's compiled source snapshot and the project's export format/size. They work in both Library and Source mode.
- A short spinner means the image is being prepared. Text and headings are not exported as equation images.

## Refresh and troubleshoot

- After external source edits, choose **Rescan** and then **Rebuild PDF**. Navigation uses the last compiled snapshot.
- **Rebuild PDF** (`⇧⌘R`) rebuilds only the document.
- A failed rebuild retains the last PDF and its snapshot; it does not represent the latest edits.
- LaTeX requires SyncTeX. If several equations share a source line, Library selects the candidates; Source navigates to the line.
- Ambiguous LaTeX locations cannot be dragged directly; use the matching Library card or Source preview.
- Generated content may have no navigation target; see [limits](limits.md).
- Previous/next page: `⌘[` / `⌘]`; PDF zoom: `⌃⌘+` / `⌃⌘−` or Control-scroll over the PDF; fit: `⌃⌘0`.
