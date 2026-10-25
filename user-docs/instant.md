---
title: "Make an equation instantly"
description: "Use a standalone template or a project's definitions in Instant."
sidebar:
  order: 6
---

1. Open **Instant** beside Projects, or use a project's lightning button (`⇧⌘K` follows the current workspace).
2. Enter source, choose size/color/format, then drag the preview or **Copy** (`⇧⌘C`). Export uses `⇧⌘S`.

- **Display** and **Inline** take math without outer delimiters. **Code** accepts complete environments, TikZ, CeTZ or Typst markup.
- **Open Instant** on the setup screen works without choosing a project folder. [Compiler requirements](requirements.md) still apply.
- The color picker saves the last 16 colors shared with Library. Export colors are independent of the app theme.

## Editing and templates

- **Format** (`⇧⌥F`) formats the input; `⌘Enter` renders again.
- Copy and drag wait for a preview matching the current input and settings. Errors keep the previous preview visible but prevent exporting it as the new result.

## From Projects

- Select a LaTeX, Typst, TikZ or CeTZ template. **Instant settings** edits its source and compiler; keep one `{{content}}` placeholder.
- **Save defaults** stores the template, input mode, size, color and format. **New equation** starts from these defaults.
- The folder button reveals `instant.json`; **Reload defaults** applies external edits. A language change keeps the old draft in the library before starting empty.

## Within a project

- Each project has its own Instant window, draft and library.
- LaTeX uses the saved main preamble, included definitions, compiler and `latexmkrc`. Typst uses the selected context file's top-level definitions/imports.
- Save Source edits first. Returning to Instant or refreshing reloads the project context without changing source files.
- Templates needing companion files should use project Instant.

## Come back later

- Drafts and library entries save automatically. Select an entry to restore its source, template and output settings.
- Editing updates the selected entry. **New equation** (`⌘N`) adds another; **Duplicate equation** keeps the original.
- Standalone and project libraries stay separate. Switching languages preserves the previous nonempty draft rather than translating it.

## Edit an exported SVG

- SVG exports from Library, Source/PDF previews and Instant include editable source metadata.
- Use **Open SVG**, **Edit from Clipboard**, or drop the SVG into FerMatter to restore it.
- Matching projects reopen in **Edit** or Instant; missing or ambiguous projects use standalone Instant. Existing drafts are preserved.
- SVG embeds equation/template source, not the whole manuscript. Project imports, packages and fonts may still be needed.
- Other apps may strip metadata; keep the original SVG. PDF, PNG and JPEG do not support source restoration.

## Open Instant while another app is active

- With FerMatter running: **Instant from Anywhere** (`⌃⌥⌘N`) or **Edit SVG from Clipboard** (`⌃⌥⌘V`). Both are configurable in [Shortcuts](shortcuts.md).
- Closing the main window keeps the menu-bar app running. `⌘Q` quits it.
