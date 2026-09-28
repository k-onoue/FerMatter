# Changelog

- Record notable changes for users; omit commit-by-commit development logs.
- Keep changes under **Unreleased** until an actual release is published.
- Format: [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/).
- Version policy: [releasing](dev-docs/releasing.md#versioning).
- Earlier prototype validation: [development history](dev-docs/history/validation-2026-09.md); those version labels are not published releases.

## [Unreleased]

### Added

- Parallel macOS port in Rust and TypeScript, with a Python-free backend, shared Monaco editors, equation/figure exports and PDF navigation; kept separate from the Swift app while replacement verification continues.

- Task-based user guides, maintained in Markdown and available on the product site.
- Shared contributor and agent documentation with review and release checklists.
- File-managed shortcuts and declarative color themes, with import/export and recovery.
- Fifteen bundled light/dark theme pairs with a picker and previews, including GitHub, Catppuccin, Solarized, Atom One, and Rosé Pine.
- Stable-release notifications and accepted update downloads.
- Draggable equation and figure previews at the Source editor caret.
- Prototype DMG and PKG packaging with verification and checksums.

### Changed

- Adopt the desktop app as Mathlidelux, remove the Rust badge, preserve the original bundle identity and preview data, and import Swift source drafts without overwriting manuscripts. Prototype DMG/PKG tooling now targets the desktop app.

- Rust preview: background-only translucency and active-display full-screen wallpaper; single-click PDF, Library, and Source equation navigation with visible selection.
- Make shortcut settings easier to scan with larger, spaced key symbols and alternating rows.
- Use a single theme background across each project card and its equation preview.
- Simplify the product page around moving an equation from a paper to a slide.
- Use the approved app artwork throughout the native app and website.
- Save newly created projects in Documents/Mathlidelux and reveal their source folders in Finder.

### Removed

- Obsolete web UI prototype; retain the native app and product site.

[Unreleased]: https://github.com/k-onoue/mathlidelux/commits/main/
