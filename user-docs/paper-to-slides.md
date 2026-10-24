---
title: "From your paper to a slide"
description: "Open a project, choose an equation, and paste it into PowerPoint."
sidebar:
  order: 2
---

1. Choose **Open Project** (`⌘O`); select your LaTeX/Typst folder. Use **Open file or ZIP** for an archive or main file.
2. Select an equation; use the outline, search, or **Block** filter to find it.
3. Choose **SVG** and **Copy image** (`⇧⌘C`).
4. In PowerPoint, paste (`⌘V`), then position and size the equation on your slide.

## Before you start

- Install the [tools for your project format](requirements.md).
- Opening/rendering a project leaves its manuscript source unchanged.
- **Open Project** opens folders in place; ZIPs are extracted into the default project folder.
- Drop folders or ZIPs from Finder onto **Projects** to import them into **Settings → Projects → Default project folder**. Originals stay unchanged; existing destination folders are never overwritten.
- Dropping a folder already directly inside the default folder opens that copy without duplicating it.
- **Load File…** belongs to [New Project](new-project.md); it reads one file's text only.

## Manage projects

- Click the lightning icon on a project card or list row to open its Instant Mode.
- Click to select; double-click, press `Return`, or click the arrow to open.
- `⌘`-click toggles selection; `Shift`-click selects a range; drag empty space to select an area. `⌘A` selects the visible results; `Esc` clears the selection.
- Arrow keys move selection; hold `Shift` to extend it. Search, grid/list view, and zoom remain available.
- **Duplicate** (`⌘D`) copies selected projects into the default folder, including Library variants, labels, saved Source drafts, and Instant history. Copies get unique names.
- **Delete…** (`⌘⌫`) asks for confirmation with folder paths, then moves the actual source folders to Finder Trash. This also applies to projects opened outside the default folder.
- `⌘Z` / `⇧⌘Z`, toolbar buttons, or the context menu undo/redo creation, imports, duplication and deletion. Any action moving folders to Trash asks for confirmation.
- Undo history survives app restarts. Restoring refuses to overwrite existing folders; emptying Trash or moving files in Finder may make an operation unavailable. Close a project’s Instant window before deleting/restoring it.
- Import limits: folders up to 30,000 entries / 1 GB; ZIPs up to 10,000 entries / 512 MB. Linked/special files are rejected; use **Open Project** for folders containing links.

## If it does not render

- Open **Project Settings**; check the main file and compiler.
- Keep companion files, images, imports, and packages with the source.
- Open the warning/log details for a failed equation.
- After external edits, choose **Rescan**.

## Next

- [Edit a presentation version](edit-equation.md).
- [Choose format and font size](export.md).
- [Find equations from the PDF](navigate-paper.md).
