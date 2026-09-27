# Settings files — integration reference

- Status: proposed in [PR #8](https://github.com/k-onoue/mathlidelux/pull/8), open when inspected on 2026-09-28.
- Full schema/examples: [PR settings specification](https://github.com/k-onoue/mathlidelux/blob/feat/file-settings/docs/settings-files.md).
- After merge: move that specification into this reference area; publish user operations separately.

## Contract to preserve

- Directory: `~/Library/Application Support/Slide Math/Settings/`.
- `keybindings.json`: `schemaVersion: 1`, `kind: mathlidelux.keybindings`; stable command IDs.
- `theme.json`: `schemaVersion: 1`, `kind: mathlidelux.theme`; namespaced ID; light/dark palettes.
- Files: at most 1 MiB; validate before activation; atomic replacement.
- Invalid edits retain current/last-valid settings; external edits require Reload before recorder writes.
- Failure to refresh the recovery copy must not invalidate a successful primary read/write.
- First launch migrates legacy shortcut preferences; retains the old value as backup.
- Theme colors: opaque `#RRGGBB` or `system`; background opacity remains a separate preference.
- Unknown command/color tokens and schema versions are rejected.
- `extensions` preserves namespaced JSON metadata; no extension execution or plugin downloads.
- Keep legacy `copySVG`/`exportSVG` IDs even though actions use the chosen export format.

## Review focus

- Invalid/missing files, write failures, recovery, external-edit conflicts, schema compatibility.
- Import/Restore Defaults intentionally replace active data; Export does not switch settings.
- Reference: [compatibility policy](../releasing.md#versioning).
