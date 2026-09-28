# Make an equation instantly

1. Choose **Instant** on the Projects page, the lightning button in a project, or **File → New Instant** (`⇧⌘K`).
2. Choose a template and type an equation. The preview updates automatically.
3. Choose the output size in **pt**, a color, and **SVG**, **PDF**, **PNG**, or **JPEG**.
4. Drag the preview into your slide, choose **Copy** (`⇧⌘C`), or export (`⇧⌘S`).

- Instant opens beside the main window; no project folder is created.
- **Display** and **Inline** accept math without outer delimiters. Multiline input stays one equation.
- Use **Code** for complete environments, TikZ, CeTZ, or Typst markup.
- Existing [compiler requirements](requirements.md) still apply.
- Images have a transparent background; JPEG uses white. The checkerboard is only the preview background.
- Output color is independent of the app theme. The opacity slider changes the window background, not text or exported artwork.

## Editing and templates

- The editor includes syntax completion, undo/redo, line numbers, and the usual editing shortcuts.
- Choose **Instant settings** to edit the template and LaTeX compiler; keep one `{{content}}` placeholder.
- Template edits are saved with the Instant draft and each history entry. Shared project templates are unchanged.
- Definitions in the template are available to completion and compilation.
- Templates requiring companion files should be used in a regular project.
- `⌘Enter` or the refresh button renders again.
- Copy and drag become available when the preview matches the current input and settings. Errors retain the last preview for reference, but prevent exporting it as the new result.

## Come back later

- Drafts save automatically, including unfinished input and template edits.
- Closing and reopening Instant restores the draft.
- **History** stores copied, exported, or dragged entries and drafts replaced with **New equation** (`⌘N`), another history entry, or a different template.
- Search history and click an entry to restore its source, template, color, size, and format.
- Changing between LaTeX and Typst starts an empty input and keeps the previous nonempty draft in history; it does not translate source.
- Ordinary `⌘C` copies selected editor text; `⇧⌘C` copies the rendered image.
- The New Instant, copy, export, new equation, and pin shortcuts follow [custom keybindings](preferences.md).

## Current scope

- One Instant window, separate from the project workspace.
- Source and history are local to Mathlidelux; exported images do not yet include editable source metadata.
- LaTeXiT library import, LinkBack, global shortcuts, and macOS Services are not included.
