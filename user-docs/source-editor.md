# Edit the project source

- **Source mode writes real project files.** For a slide-only change, use a [Library variant](edit-equation.md).

1. Choose **Source** (`⌥⌘E`).
2. Open a file from the tree; edit the source.
3. Wait for auto save, or press `⌘S`.
4. Return to **Library** to use the refreshed equations.

## Save and recover

- Auto save runs after about 700 ms of inactivity; choose After changes or Manually in **Settings → Editor**.
- Unsaved buffers survive tab/project changes and relaunches.
- Recovered drafts wait for an edit or explicit save before being written.
- External changes block saving. Keep a copy of your draft before using **Reload…** to accept the disk version.
- UTF-8 text only, up to 10 MB; original BOM/CRLF conventions are preserved.
- For a ZIP project, edits affect its extracted copy.

## Files and panes

- Use the tree context menu for New File/Folder, Rename, Duplicate, Copy/Cut/Paste, Reveal in Finder, Copy Path and Move to Trash.
- Existing destination names are protected; choose another name when a collision is reported.
- Source imports need manual updates after renaming or moving files.
- `⌘Z` / `⇧⌘Z`: text Undo/Redo in the editor; file-operation Undo is not available.
- `⌃Tab` / `⌃⇧Tab`: next/previous source tab.
- `⌘\` / `⌥⌘\`: split right/down; tab edge-drops also split; center-drop moves a tab.
- Closing a tab retains its buffer and recovery draft; reopen the file to continue.
- **Files** and **Outline** share the sidebar vertically and scroll independently. Short windows switch to Files/Outline tabs.
- Source **Outline** follows the current file's unsaved headings; the Library outline uses the document scan.

## Preview at the caret

- Place the caret inside an equation, TikZ picture, or CeTZ canvas; pause briefly for its preview.
- Drag the preview to your slide or Finder, or use **Copy**. The Library export setting selects SVG, PNG, JPEG, or PDF.
- Preview includes open unsaved drafts and project imports; it compiles a temporary copy without saving source files.
- Move outside the item or press Escape to dismiss; moving away cancels obsolete compilation.
- The same [source-extraction limits](limits.md) apply; generated math may need the full document preview.

## Editing shortcuts

- **Format** (`⇧⌥F`) formats the open LaTeX or Typst file. `⌘Z` undoes the change; the usual save setting applies.
- Formatters are bundled: [Prettier/unified-latex](https://github.com/siefkenj/prettier-plugin-latex) for LaTeX and [Typstyle](https://github.com/typstyle-rs/typstyle) for Typst. No separate formatter installation is needed.
- Formatting is manual, limited to 1 MiB per file. Errors leave the buffer unchanged; typing or switching files cancels pending formatting.

| Action | Keys |
| --- | --- |
| Find/replace | `⌘F` |
| Copy/move line up or down | `⌥⇧↑` / `⌥⇧↓`; `⌥↑` / `⌥↓` |
| Delete selected lines | `⇧⌘K` |
| Insert line below/above | `⌘Return` / `⇧⌘Return` |
| Toggle line comment | `⌘/` |
| Format LaTeX/Typst | `⇧⌥F` |
| Indent/outdent | `⌘]` / `⌘[`; Tab / Shift-Tab |

- [Command completion](edit-equation.md#type-less) works here too.
