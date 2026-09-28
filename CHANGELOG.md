# Changelog

- Record notable changes for users; omit commit-by-commit development logs.
- Keep changes under **Unreleased** until an actual release is published.
- Format: [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/).
- Version policy: [releasing](dev-docs/releasing.md#versioning).
- Earlier prototype validation: [development history](dev-docs/history/validation-2026-09.md); those version labels are not published releases.

## [Unreleased]

### Added

- Editable compiler and conversion-tool paths in Settings and first-launch setup, with automatic detection, executable checks, and recoverable file-managed configuration.

- Required first-launch project-folder selection, with four bundled editable demos and preservation of existing sample variants, drafts, and Instant history.

- Project cards and list rows include an Instant Mode shortcut.

- Configurable default project folder, one-off New Project location, and Finder folder/ZIP imports. Projects supports multiple selection, duplication, confirmed Trash deletion, and persistent undo/redo.

- Global Instant and per-project Instant windows with live previews, completion, size/color controls, drag/copy/export, separate recoverable drafts/history, and file-managed global defaults. Project Instant reuses saved LaTeX packages/commands and Typst definitions/imports.
- Drag equations and figures directly from mapped PDF locations, using the compiled snapshot and current export settings.
- Independent Control-scroll zoom for Projects, the Library canvas and the PDF pane, with percentage/reset controls for Projects and Library.
- Window-wide zoom with Command +/− and Command 0, customizable shortcuts, and separate Control-Command PDF zoom controls.
- Grid/list switching on Projects, with the selected view remembered and folder paths shown beneath project names.
- Desktop implementation in Rust and TypeScript, with a Python-free backend, shared Monaco editors, equation/figure exports and PDF navigation.

- Task-based user guides, maintained in Markdown and available on the product site.
- Shared contributor and agent documentation with review and release checklists.
- File-managed shortcuts and declarative color themes, with import/export and recovery.
- Fifteen bundled light/dark theme pairs with a picker and previews, including GitHub, Catppuccin, Solarized, Atom One, and Rosé Pine.
- Draggable equation and figure previews at the Source editor caret.
- Prototype DMG and PKG packaging with verification and checksums.

### Changed

- Protect settings against stale-window overwrites and preserve rapid preference changes with ordered saves and explicit reload after conflicts.
- Stream large project snapshots and hashes with cancellation, bound image allocations and avoid loading every selected image when copying multiple items.
- Reclaim eligible old render/export/PDF caches at startup while preserving saved work and clipboard references.
- Preserve Source and Library drafts before closing or switching projects; failed saves remain retryable. Quit waits for all editor windows.
- Keep recent Library edits when rescans or renders finish late. Source moves retain drafts, main-file settings and shared editor Undo; unsaved files cannot be moved to Trash.
- Release closed project and transient variant editor models without discarding shared split-pane Undo history.

- Align slider thumbs with track endpoints at minimum and maximum values.

- Display source font sizes in points instead of “Original” throughout the Library and Source previews.

- Show Source Files and Outline together in a stacked sidebar, with tabs for short windows.

- Adopt the desktop app as Mathlidelux, remove the Rust badge, preserve the original bundle identity and preview data, and import Swift source drafts without overwriting manuscripts. Prototype DMG/PKG tooling now targets the desktop app.

- Background rendering: background-only translucency and active-display full-screen wallpaper; single-click PDF, Library, and Source equation navigation with visible selection.
- Make shortcut settings easier to scan with larger, spaced key symbols and alternating rows.
- Use a single theme background across each project card and its equation preview.
- Simplify the product page around moving an equation from a paper to a slide.
- Use the owner-supplied x²-to-slide artwork throughout the native app and website, with a transparent exterior and updated macOS icon sizes.
- Save newly created projects in Documents/Mathlidelux and reveal their source folders in Finder.

### Removed

- Retired Swift app and Python workers; the maintained implementation and packaging live in `desktop-app/`. Old source remains in Git history; user data is preserved.
- Swift in-app update downloader; use manual app replacement until a new signed-update integration is available.

- Obsolete web UI prototype; retain the native app and product site.

[Unreleased]: https://github.com/k-onoue/mathlidelux/commits/main/

### Security

- Update the PDF parser to reject excessive object nesting instead of aborting the process (RUSTSEC-2026-0187).
