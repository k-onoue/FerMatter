# Settings files

**Settings → Shortcuts** and **Settings → Appearance** provide Edit JSON, Reload,
Import and Export. Shortcuts can be reset; choose Default to reset the theme.
**Settings → Tools** reveals the storage folder. Files live in
`~/Library/Application Support/Slide Math/Rust Preview/Settings/`:

- `keybindings.json` contains shortcut overrides. Recording or clearing a shortcut
  saves the file immediately.
- `theme.json` contains the active theme, including both light and dark palettes.
- `preferences.json` stores appearance, auto-save and window preferences. `projectView` is `grid` (default) or `list`; older files without it keep the grid view. Optional `projectDirectory` is an absolute path; missing/empty uses Documents/Mathlidelux. **Settings → Projects** changes it without moving existing projects.
- `instant.json` stores reusable global Instant defaults; edit through the settings of Instant opened from Projects, or the revealed JSON file. Drafts/history are separate.
- `tools.json` stores compiler/converter executable overrides; first-launch setup and **Settings → Tools** edit it. Apply saves changes; Reload accepts external edits.

Edit JSON opens the built-in editor. For external edits, save the file and choose
Reload. Import validates a local JSON file and copies it into the settings folder;
future changes to the imported source do not affect the active settings. Export
copies the active settings to a chosen file. Import and Restore Defaults explicitly
replace the active file. Keep exported themes to switch between them using Import.

- **Appearance → Color Theme** also offers Default and fifteen bundled light/dark pairs; see the [theme list](../../user-docs/preferences.md#color-themes).
- Selecting a preset writes its complete document to `theme.json`; no theme download is needed at runtime.
- Export modified/imported themes before switching presets to keep a reusable copy; the picker identifies presets by ID.
- Preset selection refuses to overwrite external edits until Reload accepts them.
- Bundled JSON, pinned source revisions, and MIT licenses: [Themes](../../desktop-app/resources/themes/README.md).

An invalid file leaves the current settings in use. The app retains hidden
`.last-valid-*.json` recovery files, so invalid edits also retain the last valid
settings across launches. If the active file was removed, the next launch restores
the recovery copy. The shortcut recorder refuses to overwrite external edits until
Reload accepts them. A failed file write leaves the active settings unchanged.
Files are limited to 1 MB and writes use atomic replacement.

- Reads return the document and its byte revision together; saves must supply that revision.
- Opening another window or reading compiler preferences cannot authorize an older view to overwrite external changes.
- Preferences merge patches and save in order. Failed saves keep the pending view and block later writes until Reload succeeds; Appearance includes a preferences Reload button.
- Invalid active JSON may supply last-valid fallback values but never a writable revision. Fix the external file and Reload before saving.
- Theme/shortcut/Instant/tool settings use the same revision contract. Tool changes still require Apply.

The desktop app copies missing Swift JSON settings and recovery files on first
use. Current preview settings take precedence. Appearance, opacity, pinning and
auto-save mode migrate from Swift preferences when current preferences are absent.
The much older `keyboardShortcuts` UserDefaults blob is retained by macOS but is
not imported directly; export it through a Swift build first if needed.

## Keyboard shortcuts, version 1

```json
{
  "schemaVersion": 1,
  "kind": "mathlidelux.keybindings",
  "bindings": {
    "togglePDF": {
      "key": "k",
      "command": true,
      "shift": false,
      "option": true,
      "control": false
    }
  }
}
```

Omitted commands retain their built-in defaults. Set `key` to `""` to clear a
shortcut. All four modifier booleans are required. Keys use lowercase letters,
digits, unshifted punctuation, `"\t"` for Tab, or `"\uF700"` through `"\uF703"`
for Up, Down, Left, and Right. Assigned shortcuts require Command or Control.
The same reserved macOS/text-editing combinations and duplicate checks apply to
files and the recorder. Import and Reload validate the whole assignment at once,
so exchanging two commands' shortcuts in a file works.

Version 1 command IDs are stable:

| Group | IDs |
| --- | --- |
| Projects | `open`, `projects`, `newProject`, `instant`, `newEquation`, `newFigure` |
| Editing | `find`, `edit`, `duplicate`, `saveEquation`, `copySVG`, `exportSVG`, `rescan` |
| PDF | `togglePDF`, `rebuildPDF`, `previousPage`, `nextPage`, `zoomIn`, `zoomOut`, `fitPDF` |
| Window | `inspector`, `pin`, `fullScreen`, `projectSource` |
| Window zoom | `zoomWindowIn`, `zoomWindowOut`, `resetWindowZoom` |
| Source panes | `nextSourceTab`, `previousSourceTab`, `splitSource`, `splitSourceDown` |

The existing `copySVG` and `exportSVG` IDs retain their names for compatibility;
they operate on the currently selected export format.

- Window zoom defaults: `⌘=`, `⌘−`, `⌘0`; `⌘+` also enlarges.
- PDF zoom IDs keep their PDF behavior; defaults now add Control (`⌃⌘=`, `⌃⌘−`, `⌃⌘0`).
- Saved overrides take precedence over these six zoom defaults without rewriting their JSON. An implicit zoom default that conflicts with an override is shown as unassigned; explicit duplicate assignments are still rejected.
- Window zoom is per launch (50–200%); it uses the [Tauri webview zoom API](https://v2.tauri.app/reference/javascript/api/namespacewebview/#setzoom) and leaves export scale unchanged.
- `instant` defaults to `⇧⌘K`: global from Projects, local from a project. Its implicit default also yields to saved overrides.

## Compiler tools, version 1

```json
{
  "schemaVersion": 1,
  "kind": "mathlidelux.tools",
  "paths": {
    "typst": "/opt/homebrew/bin/typst",
    "pdflatex": "/Library/TeX/texbin/pdflatex"
  }
}
```

- Supported keys: `typst`, `latexmk`, `pdflatex`, `xelatex`, `lualatex`, `latex`, `pdftex`, `pdfcrop`, `pdf2svg`, `pdfinfo`, `pdftocairo`, `synctex`, `dvips`, `ps2pdf`, `gs`.
- Values: absolute executable paths; omitted/empty entries use automatic detection. Paths are literal, without arguments, `~`, or environment-variable expansion.
- Apply checks explicit overrides are executable files. Reload validates the schema and shows unavailable paths; an unavailable explicit override never falls back to an automatic choice.
- Search: configured executable parents, standard TeX/Homebrew/system directories, inherited absolute PATH entries.
- Each compiler request retains its configuration snapshot. Per-command PATH launchers also route child commands such as `latexmk` → `pdflatex`; the app never mutates process-global PATH.
- Shell launchers quote literal paths and forward arguments unchanged. TeX symlinks retain their original names because these select the engine format.
- Project `latexmkrc` commands with explicit absolute paths bypass name-based overrides. Package/font discovery remains the selected installation's responsibility.
- Configuration changes invalidate equation render-cache keys. Existing jobs finish with their original configuration; subsequent renders/builds use the applied settings.
- Startup uses the last valid settings/recovery file; external edits require Reload while the app is running. Compiler requests do not reset external-edit conflict tracking.
- First-launch folder selection is followed by tool configuration; no missing auto-detected tool blocks setup. Existing completed setups are not repeated.

## Global Instant, version 1

| Field | Value |
| --- | --- |
| `schemaVersion`, `kind` | `1`, `mathlidelux.instant` |
| `templates` | 1–100 named templates with unique `id`, `name`, `format`, `compiler`, and `source` containing one `{{content}}` |
| `defaultTemplateID` | An ID from `templates` |
| `inputMode` | `block`, `inline`, or `figure` |
| `fontSize` | 1–200 pt |
| `color` | Opaque `#RRGGBB` |
| `exportFormat` | `svg`, `pdf`, `png`, or `jpeg` |

- **Save defaults** writes the current global template and output defaults; unfinished template edits stay recoverable in the draft but cannot become defaults until valid.
- **Reload defaults** validates and applies the external file. Invalid input preserves the existing settings and file; writes detect changes since load/reload.
- `.last-valid-instant.json` supports recovery; the active file is not replaced just because parsing failed.
- Built-in source templates are shared with project creation: [template-defaults.json](../../desktop-app/src/template-defaults.json).
- Shared project-template edits and Project Instant do not update global defaults. See [Instant guide](../../user-docs/instant.md).

## Themes, version 1

The included [Ocean example](../../site/public/settings/ocean.theme.json) can be imported directly.
Each theme has a stable, namespaced `id` independent of its displayed `name`.
Use lowercase letters and digits separated by dots or hyphens, for example
`community.author.ocean`. IDs must contain at least one separator and have at most
120 characters; names contain 1–80 characters. A theme's ID is data, never a file
path. The built-in theme is `org.mathlidelux.default`.

```json
{
  "schemaVersion": 1,
  "kind": "mathlidelux.theme",
  "id": "community.example.ocean",
  "name": "Ocean",
  "light": { "accent": "#006F77", "command": "#6842A6" },
  "dark": { "accent": "#63D5D9", "command": "#C8A6FF" }
}
```

Both palettes are required. Omitted colors and the value `"system"` use the
existing native color for that token. Custom colors use opaque sRGB `#RRGGBB`.
Alpha values are rejected: opacity remains a background-only window preference.
System appearance switches palettes automatically; Light/Dark choose one explicitly.

| Token | Applied to |
| --- | --- |
| `accent` | Control tint, selection accents, and window gradient |
| `windowBackground` | Main window background |
| `surfaceBackground` | Project cards, inspector/editor previews, new-project editor surface |
| `text` | Source editor text and newly typed text |
| `command` | LaTeX commands and Typst calls/directives |
| `delimiter` | Source brackets, math delimiters, alignment characters |
| `number` | Source numeric literals |
| `comment` | Source comments |

Background opacity remains independent of theme colors. Themes do
not recolor rendered equations, figures, PDF pages, or exported artwork. Choose
light and dark colors with enough contrast for the backgrounds and opacity you use.

## Extension boundary

Both document formats accept an optional `extensions` object, keyed by a
community namespace. Its JSON data is retained on import, export, reload, and
shortcut edits. For example:

```json
"extensions": {
  "community.example": { "license": "MIT", "author": "Example" }
}
```

Only declared fields and supported command/color tokens affect the app. Unknown
commands, unknown color tokens, and unsupported schema versions are rejected.
Additional metadata belongs inside `extensions`; unrelated top-level fields have
no rendering behavior. Restore Defaults intentionally clears extensions.

`SettingsFiles` and the separate shortcut/theme documents provide
a validation and persistence boundary for future declarative contributions.
Future command providers can register stable IDs before their bindings are used;
new document meanings require explicit compatibility handling. A future plugin
package manager, permission model, or execution host is separate work. Version 1
loads local declarative data only: it executes no extension code and downloads no
plugins or themes.
