---
title: "Edit the project source"
description: "Edit project files and recover unsaved Source drafts."
sidebar:
  order: 8
---

**Source mode writes real project files.** Use a [Library variant](edit-equation.md) for slide-only changes.

Choose **Source** (`⌥⌘E`) and open a file. **Settings → Editor → Save files** selects automatic saving or manual `⌘S`.

## Save and recover

- Unsaved buffers survive tab/project changes and relaunches. Recovered drafts wait for an edit or explicit save before being written.
- External changes block saving; **Reload…** accepts the disk version. Keep a copy of any draft you need first.
- UTF-8 files up to 10 MB are supported. ZIP projects edit the extracted copy.

## Files and panes

- Use the file-tree context menu to create, rename, duplicate or move files. Update source imports yourself after a rename or move.
- `⌘Z` undoes text edits; file operations have no Undo.
- `⌘\` / `⌥⌘\` split right/down; dropping a tab at an edge also splits.
- Closing a source tab retains its buffer and recovery draft.
- Source **Outline** follows unsaved headings in the current file; Library uses the last scan.

## Preview at the caret

- Pause inside an equation, TikZ picture or CeTZ canvas to preview it, then copy or drag the result.
- Preview compiles a temporary copy with open unsaved drafts and project imports; it does not save source files.
- Moving outside the item or pressing Escape dismisses the preview.
- [Extraction limits](limits.md) also apply here.

## Editing shortcuts

- **Format** (`⇧⌥F`) uses bundled LaTeX/Typst formatters; no separate installation is needed.
- Formatting is manual and limited to 1 MiB. Errors leave the buffer unchanged; typing or switching files cancels pending formatting.
- Completion includes [project-defined commands](edit-equation.md#type-less).
