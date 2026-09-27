# Edit the project source

- **Source mode writes real project files.** For a slide-only change, use a [Library variant](edit-equation.md).

1. Choose **Source** (`⌥⌘E`).
2. Open a file from the tree; edit the source.
3. Wait for auto save, or press `⌘S`.
4. Return to **Library** to use the refreshed equations.

## Save and recover

- Default auto-save delay: 800 ms; change delay or choose Manual in **Settings → Editor**.
- Unsaved buffers survive tab/project changes and relaunches.
- Recovered drafts wait for an edit or explicit save before being written.
- External changes block saving; use **Reload…**, **Save Copy…**, or **Revert File…** as appropriate.
- UTF-8 text only, up to 2 MB; original BOM/CRLF conventions are preserved.
- For a ZIP project, edits affect its extracted copy.

## Files and panes

- `⌘N`: new file; Return/F2: rename; `⌘⌫`: Move to Trash.
- Tree context menu: copy, cut, paste, duplicate, reveal, and open with another app.
- Drag within the tree to move; Option-drag to copy; drag from Finder to import copies.
- Name collisions receive a numbered suffix; source import references need manual updates after moving files.
- `⌘Z` / `⇧⌘Z`: text Undo in the editor; file-operation Undo in the tree.
- `⌃Tab` / `⌃⇧Tab`: next/previous source tab.
- `⌘\` / `⌥⌘\`: split right/down; tab edge-drops also split; center-drop moves a tab.
- Closing the last unsaved tab offers Save / Don't Save / Cancel.
- Current File outline follows unsaved headings; Document outline uses the last scan.

## Preview at the caret

- Place the caret inside an equation, TikZ picture, or CeTZ canvas; pause briefly for its preview.
- Drag the preview to your slide or Finder, or use **Copy**. Its menu selects SVG, PNG, JPEG, or PDF.
- Preview includes open unsaved drafts and project imports; it compiles a temporary copy without saving source files.
- Move outside the item or press Escape to dismiss; moving away cancels obsolete compilation.
- The same [source-extraction limits](limits.md) apply; generated math may need the full document preview.

## Editing shortcuts

| Action | Keys |
| --- | --- |
| Find/replace | `⌘F` |
| Copy/move line up or down | `⌥⇧↑` / `⌥⇧↓`; `⌥↑` / `⌥↓` |
| Delete selected lines | `⇧⌘K` |
| Insert line below/above | `⌘Return` / `⇧⌘Return` |
| Toggle line comment | `⌘/` |
| Indent/outdent | `⌘]` / `⌘[`; Tab / Shift-Tab |

- [Command completion](edit-equation.md#type-less) works here too.
