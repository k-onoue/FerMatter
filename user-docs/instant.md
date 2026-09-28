# Make an equation instantly

1. Choose **Instant** beside Projects for standalone work, or the lightning button inside a project to use that project's definitions. `⇧⌘K` follows the current workspace.
2. Type an equation. The preview updates automatically.
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
- `⌘Enter` or the refresh button renders again.
- Copy and drag become available when the preview matches the current input and settings. Errors retain the last preview for reference, but prevent exporting it as the new result.

## From Projects

- Use for independent equations with reusable templates. **File → New Instant** also opens this window from anywhere.
- Choose a LaTeX, Typst, TikZ, or CeTZ template. **Instant settings** edits the template and compiler; keep one `{{content}}` placeholder.
- Template definitions are available to syntax completion and compilation.
- **Save defaults** writes the current template, input mode, size, color, and output format to `Settings/instant.json`.
- **New equation** starts from these saved defaults; the current draft is kept in history.
- The folder button reveals `instant.json` in Finder. Edit it with a text editor and choose **Reload defaults** to validate and apply it. JSON stores template source as a string; the in-app template editor avoids manual escaping.
- Reload keeps the current equation when the language matches. A language change preserves the old draft in history before starting empty.
- Invalid settings leave the current defaults active. Valid backups and external-edit conflict checks use the same mechanism as other [settings files](preferences.md).
- Initial defaults copy existing custom project templates and a valid earlier Instant draft once. Later edits are independent of shared project templates.
- For templates requiring companion files, open Instant from within the project.

## Within a project

- One separate window, draft, and history per project. Standalone and project windows can coexist; the title identifies the project.
- LaTeX uses the project's main preamble, included definition/package files, compiler, and `latexmkrc`.
- Typst uses the selected context file's top-level definitions/imports, including definitions after its last equation. Choose another `.typ` file for definitions in that module; imported modules keep their normal scope.
- Syntax completion reads the project's `.tex`, `.sty`, `.cls`, and `.typ` files.
- Source files remain unchanged. Instant reads saved project files; save Source edits first when using manual save.
- Returning to the window or choosing refresh reloads the project context. If the project entry file's language changed, choose **New equation** for a fresh compatible draft; the previous draft stays in history.
- Add packages or shared definitions in the project's Source editor. The local settings panel shows the entry file/compiler and reveals the project folder.

## Come back later

- Drafts save automatically, including unfinished input and template edits.
- Closing and reopening Instant restores the draft.
- **History** stores copied, exported, or dragged entries and drafts replaced with **New equation** (`⌘N`), another history entry, or a different template.
- Search history and click an entry to restore its source, template, color, size, and format.
- Changing between LaTeX and Typst starts an empty input and keeps the previous nonempty draft in history; it does not translate source.
- Ordinary `⌘C` copies selected editor text; `⇧⌘C` copies the rendered image.
- The New Instant, copy, export, new equation, and pin shortcuts follow [custom keybindings](preferences.md).

| Under `~/Library/Application Support/Slide Math/Rust Preview/` | Contents |
| --- | --- |
| `Settings/instant.json` | Reusable global defaults; copy this file to back them up |
| `Instant/session.json` | Global working draft and history |
| `Projects/<id>/Instant/session.json` | That project's working draft and history |

- Session files are app-managed. Keep them for backup/recovery; use the UI to edit drafts and history.
- Each scope has its own `Instant/Cache/` and `Instant/Exports/`; these contain generated artwork.

## Edit an exported SVG

- Export or copy **SVG** from Library, Source/PDF previews, or Instant to include editable source and output settings.
- Choose **Open SVG** or **Edit from Clipboard** in Instant or the Mathlidelux menu-bar menu. Dropping an SVG into the app also restores it.
- In Finder, use **Open With → Mathlidelux**, or drop the SVG onto the Dock icon.
- A uniquely matching registered project opens its own Instant. Missing or ambiguous projects open the standalone Instant.
- The current draft is kept in History. Imported source waits for **Render** (`⌘Enter`); opening a file does not execute its code.
- SVG source includes the equation and template/preamble. Review it before sharing; the complete manuscript and app storage paths are not embedded.
- Project-specific Typst definitions, imported files, packages and fonts may still require the original project. Standalone fallback cannot recreate missing dependencies.
- Other applications may strip SVG metadata when pasting or copying it. Keep the original SVG or use Instant History if the copied artwork has no editable source. SVGs exported by older versions have no restoration metadata.

## Open Instant while another app is active

- Keep Mathlidelux running and use its menu-bar menu.
- **Instant from Anywhere**: `⌃⌥⌘N`. **Edit SVG from Clipboard**: `⌃⌥⌘V`.
- Change or disable either shortcut in **Settings → Shortcuts** or `keybindings.json`. If another app owns a shortcut, choose a different binding.
- Closing the main window keeps the app available. **Quit Mathlidelux** (`⌘Q`) ends the app after saving recoverable work.
- Login autostart is not enabled automatically.

## Current scope

- One global window and one local window per project, separate from the project workspace.
- Editable restoration metadata is included in SVG only. PDF, PNG and JPEG remain ordinary image exports.
- LaTeXiT library import, LinkBack, and macOS Services are not included. Native background entry points are currently tested on macOS only.
