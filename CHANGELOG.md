# Changelog

- Record notable changes for users; omit commit-by-commit development logs.
- Keep changes under **Unreleased** until an actual release is published.
- Format: [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/).
- Version policy: [releasing](dev-docs/releasing.md#versioning).
- Earlier prototype validation: [development history](dev-docs/history/validation-2026-09.md); those version labels are not published releases.

## [Unreleased]

### Added

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

- Adopt the desktop app as Mathlidelux, remove the Rust badge, preserve the original bundle identity and preview data, and import Swift source drafts without overwriting manuscripts. Prototype DMG/PKG tooling now targets the desktop app.

- Background rendering: background-only translucency and active-display full-screen wallpaper; single-click PDF, Library, and Source equation navigation with visible selection.
- Make shortcut settings easier to scan with larger, spaced key symbols and alternating rows.
- Use a single theme background across each project card and its equation preview.
- Simplify the product page around moving an equation from a paper to a slide.
- Use the approved app artwork throughout the native app and website.
- Save newly created projects in Documents/Mathlidelux and reveal their source folders in Finder.

### Removed

- Retired Swift app and Python workers; the maintained implementation and packaging live in `desktop-app/`. Old source remains in Git history; user data is preserved.
- Swift in-app update downloader; use manual app replacement until a new signed-update integration is available.

- Obsolete web UI prototype; retain the native app and product site.

[Unreleased]: https://github.com/k-onoue/mathlidelux/commits/main/
