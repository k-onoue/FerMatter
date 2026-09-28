# SVG restoration recipes

- Design tracking: [issue #43](https://github.com/k-onoue/mathlidelux/issues/43).
- Shared format and restore logic live in `roundtrip.rs`; native entry points in `resident.rs` and `native_import.rs`.
- macOS first. Windows/Linux entry points and clipboard adapters remain future work.

## Version 1

- Standard `image/svg+xml`; a SVG `metadata` element contains a `recipe` child in `https://mathlidelux.app/ns/recipe/1`.
- Child text is UTF-8 JSON, XML-escaped when serialized. No base64 dependency or new file extension.
- Fields: `schemaVersion: 1`, `kind: "mathlidelux.svg"`, `draft`, optional `project`.
- `draft`: source, template `{id,name,format,compiler,source}`, input kind, fontSize in pt, color, exportFormat, optional relative contextFile.
- `project`: UUID and display name. Never embed a machine-specific project root or internal cache paths.
- Library/Source/PDF exports include the equation and a fallback template. LaTeX includes the main preamble only; Typst uses the built-in template, since arbitrary definitions can contain manuscript text.
- Instant exports include its reusable template. This is source-bearing artwork: review before sharing.
- Unknown recipe versions, ambiguous/multiple recipes, invalid settings and non-relative context paths are rejected.
- SVG limit: 16 MiB. Recipe limit: 2 MiB. Parsing does not resolve external entities or permit DTDs.

## Project identity

- `.mathlidelux-project.json` in the real project root contains `schemaVersion: 1`, `kind: "mathlidelux.project"`, `uuid`.
- Created when opening the project, or lazily on export for an older registered project. Read-only projects can still export without a project identity; a corrupt identity is preserved and reported.
- UUID survives moving/syncing the folder. Explicit app duplication creates a new UUID.
- Existing path-hash IDs, saved variants/drafts/history, bundle identity and legacy paths are unchanged.
- Resolve against registered projects only; do not search the disk or follow paths embedded by an SVG.
- One match with the same language opens local Instant. Missing or duplicate UUIDs use standalone Instant.
- External packages, fonts and referenced resources are not embedded; standalone fallback may require setup.

## Restore and native lifetime

- All native/file/drop/clipboard inputs enter the same decoder and resolver.
- A bounded per-window queue waits for the webview to load. Acknowledge only after the replaced draft is in History and the imported draft is saved.
- Imported drafts persist `requiresRender: true`; neither initial import, restart nor editing automatically compiles them. Explicit Render clears the flag.
- OS clipboard reads happen only after an explicit user command; no polling of clipboard contents.
- Finder Open With and Dock file events use Tauri `RunEvent::Opened`. Existing windows are reused by scope.
- Global shortcut bindings use existing `keybindings.json` entries `globalInstant` and `globalInstantClipboard`; empty key disables either.
- Native global shortcut conflicts report an error. Login autostart is not changed.
- Third-party SVG metadata retention is a separate interoperability check; a normal image without the recipe is never reported as source-restorable.

## References

- [SVG metadata](https://www.w3.org/TR/SVG/struct.html#MetadataElement).
- [Tauri tray](https://v2.tauri.app/learn/system-tray/).
- [Tauri shortcuts](https://v2.tauri.app/plugin/global-shortcut/).
