# Settings files

**Settings → Shortcuts** and **Settings → Appearance** provide Open File, Show in
Finder, Reload, Import, Export, and Restore Defaults. Files live in
`~/Library/Application Support/Slide Math/Settings/`:

- `keybindings.json` contains shortcut overrides. Recording or clearing a shortcut
  saves the file immediately.
- `theme.json` contains the active theme, including both light and dark palettes.

Open File uses the system's default JSON editor. Save external edits, then choose
Reload. Import validates a local JSON file and copies it into the settings folder;
future changes to the imported source do not affect the active settings. Export
copies the active settings to a chosen file. Import and Restore Defaults explicitly
replace the active file. Keep exported themes to switch between them using Import.

- **Appearance → Color Theme** also offers Default and fifteen bundled light/dark pairs; see the [theme list](../../user-docs/preferences.md#color-themes).
- Selecting a preset writes its complete document to `theme.json`; no theme download is needed at runtime.
- A modified/imported document appears as **Custom**, even if it retains a preset's ID. Export it before switching to keep a reusable copy.
- Preset selection refuses to overwrite external edits until Reload accepts them.
- Bundled JSON, pinned source revisions, and MIT licenses: [Themes](../../macos-app/Sources/SlideMath/Themes/README.md).

An invalid file leaves the current settings in use. The app retains hidden
`.last-valid-*.json` recovery files, so invalid edits also retain the last valid
settings across launches. If the active file was removed, the next launch restores
the recovery copy. The shortcut recorder refuses to overwrite external edits until
Reload accepts them. A failed file write leaves the active settings unchanged.
Files are limited to 1 MB and writes use atomic replacement.

The first launch with file settings migrates the previous `keyboardShortcuts`
UserDefaults value. That value remains as a backup; the JSON file takes precedence
from then on. Appearance mode, background opacity, and window pinning retain their
existing preferences and behavior.

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
| Projects | `open`, `projects`, `newProject`, `newEquation`, `newFigure` |
| Editing | `find`, `edit`, `duplicate`, `saveEquation`, `copySVG`, `exportSVG`, `rescan` |
| PDF | `togglePDF`, `rebuildPDF`, `previousPage`, `nextPage`, `zoomIn`, `zoomOut`, `fitPDF` |
| Window | `inspector`, `pin`, `fullScreen`, `projectSource` |
| Source panes | `nextSourceTab`, `previousSourceTab`, `splitSource`, `splitSourceDown` |

The existing `copySVG` and `exportSVG` IDs retain their names for compatibility;
they operate on the currently selected export format.

## Themes, version 1

The included [Ocean example](../../docs/settings/ocean.theme.json) can be imported directly.
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

Liquid Glass and native material surfaces keep their native rendering. Themes do
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
Additional metadata belongs inside `extensions`; unrelated top-level fields are
ignored and are not retained. Restore Defaults intentionally clears extensions.

`SettingsDocument`, `SettingsFile`, and the separate shortcut/theme models provide
a validation and persistence boundary for future declarative contributions.
Future command providers can register stable IDs before their bindings are used;
new document meanings require explicit compatibility handling. A future plugin
package manager, permission model, or execution host is separate work. Version 1
loads local declarative data only: it executes no extension code and downloads no
plugins or themes.
