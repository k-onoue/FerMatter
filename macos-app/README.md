# Mathlidelux — native macOS prototype

Browse equations and TikZ/CeTZ figures from a LaTeX or Typst project, make presentation variants, and export
SVG (default), PNG, JPEG, or vector PDF. Equation variants leave the manuscript
unchanged; Source mode edits and saves project files directly.

## Requirements

- macOS 26 or later. The current build is for Apple Silicon.
- Xcode with the macOS 26 SDK to build. Validated with Xcode 26.6 / Swift 6.3.3.
- Python 3 at `/opt/homebrew/bin/python3`, `/usr/local/bin/python3`, or `/usr/bin/python3`.

**For Typst projects:** Typst CLI 0.14 or later (validated with 0.14.2).
No TeX installation or SVG conversion tools are needed for Typst projects.

**For LaTeX projects:**

- TeX Live (for example, MacTeX), including `latexmk`, the project's TeX engine,
  `preview.sty`, and `pdfcrop`.
- `synctex` for navigation between the document PDF and source equations.
- `pdf2svg` and Poppler (`pdfinfo`, `pdftocairo`).
- For DVI-based projects: `dvips` and Ghostscript (`ps2pdf`).
- Any additional packages and fonts required by the imported TeX project.

The worker searches `/Library/TeX/texbin`, `/opt/homebrew/bin`, `/usr/local/bin`,
and the system command paths. Runtimes and conversion tools are **not bundled**.
A packaged application would not require Xcode, but still needs a dependency
strategy before public distribution. See [the publication handoff](../PUBLISHING.md).

## Build and run

From the repository root:

```sh
cd macos-app
./scripts/build.sh
open "build/Mathlidelux.app"
```

The script uses `/Applications/Xcode.app` when present, or an explicit
`DEVELOPER_DIR`. It does not change the system-wide `xcode-select` setting.
The output is ad hoc signed for local development. It is not Developer ID signed
or notarized, and uses a prototype bundle identifier.

The app icon is defined in `Resources/AppIcon.icon`, including its PNG artwork.
The build script uses Xcode's `actool` to compile the native macOS 26 icon stack
into `Assets.car` and generate an `AppIcon.icns` fallback in the signed bundle.
See [the icon notes](Resources/AppIcon.md) for the artwork and generation prompt.

## App updates

The app checks for a stable GitHub release at most once a day while in use.
A new version appears in an in-app banner with **Download Update**. After the
download finishes, choose **Open Disk Image**, quit Mathlidelux, and copy the
new app to Applications. Downloads can be cancelled; the running app and your
projects stay open throughout the download.

Use **Mathlidelux → Check for Updates…** to check immediately, or
**Settings → Updates** to turn off automatic checks. If a release has no compatible
disk image, **View Release** opens its GitHub page. No public release may be
available yet. See [update behavior and release requirements](UPDATES.md).

## Development checks

Run `./macos-app/scripts/check.sh` from the repository root for Swift tests,
real compiler integration, Python tests, and publication checks. The script
does not modify the sample ZIPs; after intentionally changing sample sources,
run `python3 -B scripts/prepare-demo.py` first.

See [the architecture guide](ARCHITECTURE.md) for module responsibilities and
editor/worker lifecycles, and [validation results](VALIDATION.md) for tested scope.

## Start a project

| Action | Use it for | Result |
| --- | --- | --- |
| **Open Project** (Command-O) | A project folder, ZIP, or main `.tex` / `.typ` file | Opens the existing project and builds its equation library. |
| **New Project** (Shift-Command-N) | Equations or source text you want to paste | Creates a new project using a built-in or custom template. |
| **Load File…** inside New Project | The text of a UTF-8 `.tex`, `.typ`, or `.txt` file | Fills the text area; **Create Project** then saves a separate project. |

### Open a folder or ZIP

Use **Open Project** even when the folder or ZIP is new to Mathlidelux. Select the
folder, ZIP, or main source file. Folders are used in place; ZIPs are extracted
into the app's local storage. Opening a main file uses its containing folder, so
relative imports and companion files remain available. Opening a project does not
rewrite its source.

Project Settings contains the main file, compiler, Rescan, and Render All controls.
For a LaTeX project without a `latexmkrc`, select a compiler there. Typst's compiler
is selected automatically. Keep LaTeX and Typst libraries in separate folders;
a folder containing both formats requires selecting a main file. Imported helper
modules are not main documents: choose the intended document in Project Settings.

### Create a project from text

1. Choose **New Project** and enter a name.
2. Select a template matching your source format.
3. Paste into the text area, click **Paste**, or choose **Load File…**.
4. Leave **Auto** selected, or choose an explicit input mode below.
5. Click **Create Project**. The app saves the source, extracts its equations and
   supported figures, and starts rendering SVGs.

**Load File…** does the equivalent of opening a file, selecting its contents,
copying, and pasting into this text area. It replaces the current text; it does
not open the original as a project or change that file. A `.tex` or `.typ`
extension switches the template's format when necessary; for `.txt`, choose the
appropriate template yourself. Companion files are not copied by Load File.

| Input mode | How to write the content |
| --- | --- |
| **Auto** | Recognizes common math delimiters, headings, and drawing source. Otherwise, treats each nonempty, non-comment line as a display equation. |
| **One per line** | Write one complete equation per line, without `$…$` or other outer math delimiters. Blank lines and line comments are retained. |
| **Source** | Keep your source markup, including math delimiters, headings, multiline environments, and drawings. |

For example, paste this into the **LaTeX** template with **Auto** or **One per line**:

```tex
x^2 + y^2 = r^2
\int_0^1 x^2\,dx = \frac{1}{3}
```

Or use the **Typst** template with the same input mode:

```typst
x^2 + y^2 = r^2
integral_0^1 x^2 dif x = 1/3
```

Each example creates two equation cards. For existing `align` environments,
multiline formulas, TikZ pictures, or CeTZ canvases, use **Source** or check that
**Auto** reports “Source”. Auto detection is a convenience heuristic, and the
explicit modes let you override it.

In **Auto** or **Source**, a complete LaTeX document keeps its own preamble
instead of using the template wrapper; the template still selects the compiler.
Typst setup in pasted source follows the template setup. Choosing a template does
not convert LaTeX syntax into Typst, or vice versa.

### Choose or customize a template

| Built-in template | Included setup |
| --- | --- |
| **LaTeX** | A 12pt article with `amsmath`, `amssymb`, `mathtools`, and `bm`; pdfLaTeX by default. |
| **Typst** | 12pt text and unnumbered equations, using Typst syntax. |
| **LaTeX + TikZ** | The LaTeX setup plus TikZ and the `arrows.meta`, `calc`, and `positioning` libraries for drawings. |
| **Typst + CeTZ** | The Typst setup plus a CeTZ 0.5.2 import for drawings. |

For ordinary equations, start with **LaTeX** or **Typst**. Choose the corresponding
drawing template when your pasted source also uses TikZ or CeTZ.

Open **Settings → Templates**, or the **Manage templates** button beside the
template picker. **Customize…** makes an editable copy of a built-in template;
**Edit…** changes a saved custom template. Give it a name, adjust the source,
and click **Save Template**. The options menu lets you duplicate a template,
set it as the default for New Project, or delete a custom template.

A template contains the document setup and exactly one `{{content}}` placeholder
where the pasted text will go. Add packages, macros, definitions, or fonts to
that setup. LaTeX templates also let you choose pdfLaTeX, XeLaTeX, or LuaLaTeX.

For example, a custom Typst template can define `rr` as blackboard-bold ℝ:

```typst
#set text(size: 14pt)
#set math.equation(numbering: none)
#let rr = $bb(R)$

{{content}}
```

Save it under a name such as **Typst · Blackboard**, then paste `x in rr` with
**One per line** to use the definition. “Typst · Blackboard” is a custom example,
not a fifth built-in template; it only appears if saved on that Mac.
Editing or deleting a template leaves already-created project sources unchanged.

### Where new projects are saved

New projects default to `~/Documents/Mathlidelux/`. The New Project window shows
the destination; use **Change…** to choose another location. Each project is a
normal folder with a `main.tex` or `main.typ` (and a compiler `latexmkrc` for LaTeX).
Reusing a name adds a numeric suffix without overwriting the previous project.
Existing projects, including those created under
`~/Library/Application Support/Slide Math/Sources/`, stay in their original locations.
**Show in Finder** opens that project's folder. Templates are saved locally in
`~/Library/Application Support/Slide Math/templates.json`.
These are source-text templates: relative imports, images, and helper files need
to be copied into the created project folder separately. Use **Open Project**
for an existing project with such files. **Change…** in New Project chooses the
parent folder in which the new project folder will be created.

## Edit project files

Choose **Source** above the sidebar, or press **Option-Command-E**, to edit the
project itself. The file tree includes main files, imported modules, styles,
packages, bibliography files, and configuration such as `latexmkrc`. Expand
folders, open files in tabs, and use **+** or a folder's context menu to create a
file or folder. **Command-N** creates a file in Source mode.

Select files with Command-click, Shift-click, or Command-A while the tree has
focus. Return or F2 renames inline, initially selecting the name without its
extension. The context menu includes Cut, Copy, Paste, Duplicate, Move to Trash,
Copy Relative Path, Copy Path, Reveal in Finder, and Open With. Command-C/V work
with Finder file selections; Command-X then Command-V moves project entries.
Drag within the tree to move, hold Option to copy, or drag files and folders from
Finder to import copies. Name collisions get a numbered suffix. Command-Delete
moves selected entries to the macOS Trash. Command-Z / Shift-Command-Z undo and
redo file operations while the tree has focus; inside the editor they undo text.
Dirty drafts remain available even if their source file is moved to Trash.
Renames and moves update open tabs and the selected main-file path; literal
`\input`, `#include`, and `#import` references are edited manually.

Use **Control-Tab** and **Control-Shift-Tab** to cycle open files in the active
pane. Middle-click a tab or use its × button to close it; the last open copy of
an unsaved file offers Save, Don’t Save, and Cancel. Drag a tab to a pane’s top,
bottom, left, or right edge to split there; the highlighted half shows the
destination. Dropping in the center moves the tab to that pane. Edge splits keep
the original tab open. Each pane's split menu offers **Split Right**, **Split Down**, and **Close
Pane**. Splits can be nested in either direction, with draggable dividers and
independent file selection. Command-\ splits right; Option-Command-\ splits
down. These bindings can be changed in Settings → Shortcuts. Closing a pane
keeps its open files and drafts available in the remaining panes.

**Outline** below the file tree follows the active file, including unsaved
headings. Its menu switches between Current File and the scanned Document
outline. Click a heading to jump to its source line; collapse nested headings
with their disclosure arrows. Current File detects literal LaTeX sectioning
commands and Typst markup headings; generated headings require a document scan.

The editor uses the same line numbers, syntax highlighting, completion,
argument-field navigation, line editing commands, and Undo as equation editing. Completion includes
definitions from project files and open drafts. **Command-F** opens Find/Replace.
Source files save automatically after 800 ms without typing. **Settings → Editor**
changes the delay or switches to **Manual**; **Command-S** saves immediately in
either mode. Saving rescans equations, refreshes completion, and rebuilds the PDF
if it is visible. Adding a package or macro therefore applies to the project's
rendered items. Library equation/figure dialogs still save a presentation variant
with Command-S or their Save action, leaving the manuscript unchanged.

Unsaved tabs keep their contents when switching files, modes, or projects;
drafts are also stored locally for the next launch. Saving refuses to overwrite
external changes. The pane menu's **Revert File…** discards a draft with confirmation;
conflicting files also show **Reload…** and **Save Copy…**. Recovered drafts are
not automatically written until edited again. Files must be UTF-8 text and at most 2 MB; binary assets remain
visible in the tree but cannot be edited. Original BOM and CRLF conventions are
preserved. Source mode edits the opened folder; for ZIP imports this is the
app's extracted copy.

The PDF and Inspector each remember their width across switching and relaunches.
Drag the divider to resize. A temporarily narrow window does not erase the
preferred width. PDF navigation in Source mode opens the matching file and line.

Both editors keep text opaque over the same glass background as the toolbar.

### Preview at the caret

- Place the caret inside an equation, TikZ picture, or CeTZ canvas in Source mode.
  A small preview appears after a short pause. Move outside the item or press
  Escape to dismiss it.
- Drag the preview into a slide or Finder, or use its Copy button. Its format
  menu selects SVG (default), PNG, JPEG, or PDF, shared with library exports.
- Preview uses the project's preamble/imports and current open drafts, including
  unsaved companion files. Compilation runs in a temporary project copy and
  does not save your drafts or change the source files.
- It uses the same literal source extraction as the library. Generated math or
  unsupported dynamic imports may require rendering the full document instead.
- Errors stay in the preview; moving the caret cancels obsolete compilation.

| Editor action | Shortcut |
| --- | --- |
| Copy / cut selection, or the whole current line when nothing is selected | Command-C / Command-X |
| Copy line up / down | Option-Shift-Up / Down |
| Move line up / down | Option-Up / Down |
| Delete selected lines | Shift-Command-K |
| Delete to line start; repeat to join the previous line | Command-Delete |
| Insert line below / above | Command-Return / Shift-Command-Return |
| Toggle line comments | Command-/ |
| Indent / outdent lines | Command-] / Command-[; Tab / Shift-Tab |
| Find / replace | Command-F |
| Undo / redo | Command-Z / Shift-Command-Z |

When a completion argument is active, Tab / Shift-Tab move between its fields.
Command-Return inserts a line while the source editor has focus; use Command-S
to save a library edit without moving focus.

## Try the samples

Choose **Open Project** (Command-O) and select `examples/demo-paper` from this
repository. You can also open the provided `demo-paper.zip`. The example has 27
equations, a detected `main.tex`, and a pdfLaTeX `latexmkrc`.

For Typst, open `examples/demo-typst`, `main.typ`, or `demo-typst.zip`.
The example has 21 equations. Open `examples/demo-tikz` or `examples/demo-cetz`
for drawing samples. These folders and their downloadable ZIPs all use **Open
Project**; no template selection is needed.

## Library controls

| Action | Control |
| --- | --- |
| Return to the project list | Projects / Command-Shift-O |
| Create a project from pasted text | New Project / Shift-Command-N |
| Open a folder, ZIP, or main source file | Open Project / Command-O |
| Edit a selected equation | Double-click / Command-E |
| Create an equation | Plus menu / Command-N |
| Toggle project source editor | Source / Option-Command-E |
| Save the current project file | Command-S in Source mode |
| Create a project file | Command-N in Source mode |
| Next / previous source tab | Control-Tab / Control-Shift-Tab |
| Split editor right / down | Command-\ / Option-Command-\ |
| Create a figure | Plus menu / Option-Command-N |
| Save an edit | Command-S / Command-Return |
| Copy image in the selected format | Inspector / Command-Shift-C |
| Export image in the selected format | Inspector / Command-Shift-S |
| Browse source headings | Outline; chevrons expand/collapse children |
| Add a label | Plus beside Labels |
| Assign a label | Inspector or card context menu |
| Find equations or projects | Command-F |
| Duplicate an equation | Command-D |
| Show/hide the document PDF | PDF button / Option-Command-P |
| Rebuild the document PDF | Shift-Command-R |
| Previous / next PDF page | Command-[ / Command-] |
| Zoom PDF in / out; fit | Command-+ / Command-−; Command-0 |
| Toggle inspector | Option-Command-I |
| Customize templates or shortcuts | Mathlidelux → Settings… / Command-, |
| Pin above other windows | Pin button / Shift-Command-P |
| Change appearance / background opacity | Appearance & Window |

Editing an extracted equation creates a Custom variant. Source cards remain
intact. Deleting a label keeps its equations. All / Inline / Block, search,
and Filter & Sort narrow the library; the inspector can be hidden.

Select several cards with **Command-click**, extend a range with **Shift-click**,
or drag a rectangle from empty canvas space. **Command-A** selects all cards in
the current filtered view; **Escape** or an empty-space click clears selection.
Arrow keys move between cards; Shift extends the selection. The selected count
appears in the footer. Use **Export…** or **Shift-Command-S** to write all selected
images to a folder (existing files receive unique names), **Shift-Command-C** to
copy their file URLs, or the context menu to assign a label to the selection.

The **Outline** follows literal LaTeX headings from `\part` through
`\subparagraph`, including starred headings, or Typst’s `=` through `======`.
It follows the document order across input/include files. Selecting a parent
includes equations below its descendants; chevrons only expand or collapse the
tree. Custom copies keep their source heading. **Labels** are separate manual
groups; existing sections migrate to labels without losing assignments. Older
libraries acquire their outline when opened. Source mode rescans after saving;
use **Rescan** after edits made in an external editor.
Dynamically generated headings and custom sectioning macros are not expanded.

Choose **SVG**, **PNG**, **JPEG**, or **PDF** from the format picker in the library,
inspector, or editor preview. SVG is the default for new and older libraries;
the choice is saved per project. Copy, Save, and drag use this same selection.
PNG keeps transparency; JPEG has a white background. PDF remains vector artwork.
Raster exports use 216 dpi at the selected physical size. Previously cached items
can use their existing PNG preview until rendered again; choose PDF to prepare a
missing vector PDF for the selected item.

Drag from a library card, the inspector image, or the editor's **Preview**.
The editor exports the current successfully compiled draft, even before saving.
A stale preview disappears while changed source is awaiting compilation.

The Export Font Size picker supports Original or 10–96 pt. Copy, drag, and export
use the selected physical size without changing vector paths or the source
cache. LaTeX uses the measured normal font size, with TeX-point conversion.
Typst uses the text size at each equation’s source position (72 pt/in), so
sections with different sizes export consistently. Relative math styling is kept.
The editor and inspector also have an **Export Font Size** picker for the current
item. **Default** follows the project setting, **Original** preserves the source
size, and a point size overrides only that item. The override is saved with the
item and applies to every image format, including an unsaved preview drag.

System / Light / Dark and the always-on-top setting persist across launches.
Background opacity affects background layers only; text, equations, and icons
remain opaque. Normal windows use native transparency. Native full-screen Spaces
can supply a black backdrop, so the app reads the current user's wallpaper
selection for the window's display and draws it under the translucent surfaces.
Photos use their selected local file; Apple aerial wallpapers use a still frame
from the selected downloaded video. This does not reproduce live playback or
the exact paused frame of the desktop. Display changes, Space changes, wake, and
wallpaper-setting changes refresh the selection.

The modern wallpaper store is read-only and its format is not a public API.
Supported selections are image files and a single Apple aerial, including global
and per-display settings. Ambiguous per-Space aerials, shuffles, and procedural
or third-party providers cannot always be resolved. In those cases the app keeps
native transparency and may have a dark full-screen background; it never picks
an unrelated image or silently substitutes `DefaultDesktop.heic`.

Control-Command-F and **Window → Toggle Full Screen** request native full screen.
First entry from a pinned window can be ignored by AppKit. If this happens, turn
Always on Top off, enter full screen, then turn it back on. Pinning resumes when
leaving full screen.

Typst inline previews measure the actual glyph bounds, including fractions,
roots, and subscripts. Older previews are regenerated when their project opens;
SVG, PDF, and raster exports use the repaired bounds too.

## Input assistance

Type a command prefix (for example `\f` in LaTeX or `sq` in Typst) to see automatic
suggestions. Use Up/Down to choose, Return or Tab to insert, and Esc to dismiss.
After insertion, Tab and Shift-Tab move between argument fields.
Esc with no suggestions open requests completion manually. Commands, delimiters,
numbers, and comments are colored as you type.

The equation editor also indexes definitions in the open project's `.tex`, `.sty`,
`.cls`, or `.typ` files. It recognizes common `\newcommand`, `\renewcommand`,
`\providecommand`, `\DeclareRobustCommand`, `\DeclareMathOperator`, simple `\def`,
and simple `\NewDocumentCommand` definitions, plus Typst `#let` functions/variables
and named imports. Known arguments become editable fields. The index refreshes
when opening an editor, rescanning, or returning to the app after external edits.
Definitions in the current text also contribute suggestions.

This is a static local index, not a language server: it does not expand generated
definitions, inspect installed packages, or resolve every conditional, scope,
Typst module alias, or advanced TeX argument specification. Candidates from other
project files may still require an import in the active source context. Figure
fragments such as a line or circle belong inside the corresponding drawing canvas.


## TikZ and CeTZ figures

Use **Figures** to filter drawings. Double-click a figure to create an editable
variant; copy, drag, and export preserve colors and transparency. Figure previews
use a white canvas for legibility in either appearance. The project font-size
default affects equations; figures keep their original physical size unless you
set an item override. A figure override scales the entire drawing, including its
labels, relative to the measured base font size; individually styled labels keep
their relative sizes.

- **TikZ:** literal `tikzpicture` environments, including those inside `figure`
  floats or included files. The original preamble, styles, and libraries are used;
  picture contents (including math labels) stay together. Captions are omitted.
  A New Figure can use TikZ even if the original preamble does not load it.
- **CeTZ:** literal `cetz.canvas(...)` calls, direct/renamed/wildcard canvas imports,
  and canvases inside `figure(...)` or alignment wrappers. Imports and setup are
  replayed for editing; math inside a canvas remains part of the drawing.
- **New Figure** starts a small editable template. Existing libraries need a
  **Rescan** to discover drawings that were skipped by earlier versions.

Open `examples/demo-tikz` or `examples/demo-cetz` for two colored drawings and one
inline equation each. The CeTZ sample and new-figure template use
[CeTZ 0.5.2](https://typst.app/universe/package/cetz/), which needs Typst 0.14+;
uncached packages require internet access on first compilation. TikZ needs the
project's TeX libraries; see the [TikZ manual](https://tikz.dev/tikz-scopes).

Drawing extraction follows literal source syntax. Function-generated canvases,
canvases requiring wrapper-local variables, dynamically generated TikZ, body-local
styles, page-relative/overlay pictures, and externalized graphics can require
manual adjustments. Raster images embedded in drawings remain raster images.

## Document PDF and navigation

The PDF button opens the compiled document beside the equation library. Select a
card or Outline heading to reveal its position. Command-click an equation or figure
in the PDF to select its source card; filters are cleared as needed to reveal it.
Normal PDF selection, scrolling, page controls, and zoom remain available.

LaTeX uses SyncTeX; when a compiler reports only a source line, multiple inline
equations on that line may resolve to the first matching card. Typst uses invisible
position markers in a project copy. Generated content outside the supported
source extraction may have no target. Custom variants navigate to their source
parent; equations created from scratch have no manuscript location.

Use Rescan after editing manuscript sources. The visible PDF rebuilds after a
rescan; the PDF refresh button rebuilds only the document. A failed rebuild keeps
the previous PDF visible, with navigation disabled until a successful build.
Open the warning icon for compiler details. Rendering does not rewrite the
original project; automatic and manual saves in Source mode do.

**Settings → Shortcuts** lets you record or clear each app shortcut and
restore defaults. Assignments persist in `Settings/keybindings.json` and update the menus.
Duplicates and standard macOS/text-editing combinations are rejected. Escape and
the editor's Command-Return remain standard dialog controls.

**Settings → Appearance** manages the active `Settings/theme.json`, with light and
dark colors for the window, surfaces, accents, and source syntax. Both settings
panes can open, reveal, reload, import, and export their JSON file. Invalid edits
keep the last valid settings. See [Settings files](../docs/settings-files.md) for
the versioned formats, an importable theme, and the declarative extension boundary.

## Storage and rendering

Mathlidelux retains the prototype's `local.slidemath.prototype` bundle identifier
and `Slide Math` support directory so existing settings, templates, project
paths and unsaved drafts continue to work after the rename. The app and its
executable are named `Mathlidelux`.

Libraries are stored under
`~/Library/Application Support/Slide Math/Projects/<project-id>/`:

- `project.json`: project settings, cards, order, outline, labels, and export size.
- `Cache/`: rendered SVGs, single-item vector PDFs, preview PNGs, and compiler logs.
- `Exports/`: images in the chosen format and physical dimensions.
- `Documents/`: compiled PDFs and source snapshots for source navigation. Rebuilds keep older snapshots; they can grow with repeated source changes.

ZIP imports are extracted to the app's `Imports/` folder. Unsafe archive paths,
symlinks, and oversized archives are rejected.

Both renderers work in a temporary project copy. LaTeX uses the original preamble
and project compiler settings. `preview` separates equations into pages, `pdfcrop` adds a
small margin, and SVG glyphs become paths. Recipients of the exported SVG do not
need to install its math fonts.

Typst replays preceding `#let`, `#set`, `#import`, and equation show rules along
the literal include chain, keeping relative imports and lexical scope. Its CLI
emits SVG paths and transparent PNG previews directly. Equation numbering and
page backgrounds are removed for export. Cache keys include project sources,
common data/font/image files, compiler version, and equation context.

TeX and `latexmkrc` are executable input. Open projects you trust. Shell escape
is disabled, but this prototype does not provide an OS sandbox for TeX or Perl.

## Current limits

- LaTeX supports `$…$`, `$$…$$`, `\(…\)`, `\[…\]`, equation, align, gather, multline,
  displaymath, and math. Matrices and cases remain within their containing equation.
- Follows literal input/include references. This is not a full TeX execution parser:
  dynamic definitions, conditional branches, custom catcodes, generated input paths,
  and custom equation environments can need manual work.
- Typst extracts literal `$…$` math, distinguishing inline and block by the
  whitespace around the body, and follows literal `#include` paths. Raw code,
  comments, and equations inside definitions are excluded. Generated equations
  in loops, functions, containers, or contextual code are not extracted; skipped
  constructs appear in Project Settings warnings. General document templates
  (`#show: template`) are not applied to individual exports. Counters, labels,
  state, and document-wide queries may need manual adjustments.
- Repeated equations remain separate cards. Inserting another identical equation
  earlier in a file can change occurrence-to-card matching.
- Body-local macros and manuscript cross-references may not reproduce fully.
  Render failures expose their logs.
- PDF/OCR input, equation numbering, cloud sync, automatic
  rescanning, distribution signing/notarization, and dependency bundling are not implemented.
- Copying an SVG to PowerPoint has been checked. Direct drag compatibility varies
  by target application and has not been verified end to end.

## Validation

From `macos-app/`:

```sh
python3 -m unittest discover -s Tests -v
SLIDEMATH_COMPILER_TESTS=1 DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer xcrun swift test
./scripts/build.sh
```

See [VALIDATION.md](VALIDATION.md) for the tested scope. Public screenshots and
fixtures use only the synthetic demonstration project.
