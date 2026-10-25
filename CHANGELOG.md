# Changelog

- Record notable changes for users; omit commit-by-commit development logs.
- Keep changes under **Unreleased** until an actual release is published.
- Format: [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/).
- Version policy: [releasing](dev-docs/releasing.md#versioning).
- Earlier prototype validation: [development history](dev-docs/history/validation-2026-09.md); those version labels are not published releases.

## [Unreleased]

- Remove repeated Library/Source/PDF headings, group window controls at the upper right, and shorten Edit Variant to Edit.

- Add experimental Ubuntu 24.04 amd64 packages with signed APT installation; support safe Linux project moves and a default project folder when XDG Documents is unset.
- Group download-page compiler setup under Math rendering engine, remove its First launch section and rename Extraction limitations in the guide.
- Require security-patched Node.js for development and update vulnerable build/test dependencies.

- Organize guides by setup, editing, output and customization; clarify output controls, Instant input and MCP connection checks, and add installation and first-launch instructions to the download page.
- Link the homepage demo to Features and reuse its PDF-drag screenshot in the README.

## [0.11.0-preview.3] — 2026-10-25

- Show a white drag-to-Applications arrow with a black outline and fixed app/folder positions when opening the DMG.
- Note the Windows version in development below the macOS availability text.
- Include the Fermat app icon in shared-link previews for product pages and guides.
- Simplify the README around installation and usage; move repository layout and maintainer guidance to development docs.
- Offer only DMG and Homebrew on the download page, with Homebrew in its own section.
- Fix the README download badge to count DMG downloads across all published releases.
- Direct feature requests and bug reports to Issues; document that external pull requests are not accepted.
- Show direct dragging from the paper PDF on the homepage and guide, with Source view beside the PDF.
- Remove the homepage preview label and keep the macOS / Apple Silicon platform line.
- Emphasize dragging equations from Library cards, Source previews, Instant and the paper PDF.
- Stack guide/footer links in narrow reading columns and link feature headings directly to their guides; remove repeated Guide links and page-edit links.
- Simplify the homepage and download page; keep copyable FerMatter, LaTeX and Typst installation commands and TeX size comparisons. Move the download-count badge to the repository README.
- Publish seven concise visual guides, with all sixteen light/dark theme comparisons, and retire the extra public guide routes.
- Align the reading column across product pages and guides; replace large navigation cards with compact links between guides.
- Remove separate sample-project downloads from product pages and guides, using the demos bundled with the app.

## [0.11.0-preview.2] — 2026-10-22

- Unify all app and website artwork on the detailed Fermat portrait; retire Dock face switching while preserving legacy settings/history, and provide a GitHub Organization JPEG.

- Hide the Instant window's visible title while preserving its draggable top area and macOS window controls.
- Add public preview DMG and Homebrew downloads, copyable Typst/full TeX/BasicTeX setup commands, and TeX download/installed-size comparisons to the website.
- Fix website download counts appearing unavailable despite a published snapshot; remove the checksum link from the download page.

## [0.11.0-preview.1] — 2026-10-21

- First public, unnotarized preview for Apple Silicon, macOS 14+. Download the DMG from the website or install through `fermatter/tap`; first launch may require approval in macOS Privacy & Security.
- Set the project copyright notice to `2026 k-onoue`; preserve bundled third-party notices.
- Use DOMPurify 3.4.17 for Monaco's rendered documentation, fixing the advisories affecting its pinned 3.4.15 dependency.
- Update ZIP handling to zip 9 for the extraction security fixes; retain bounded, entry-by-entry imports.

- Publish the website at `fermatter.github.io`, keeping the application repository in place; redirect previous project-site page URLs.
- Use the owner-supplied simplified Fermat head across the app, menu bar, website and guide; transparent SVG marks follow light/dark themes, while the app icon keeps its blue background.
- Change the running macOS Dock icon among four Fermat faces according to local openings in the previous seven days (14+, 7–13, 3–6, or fewer than 3).
- Choose a fixed Dock face or return to Automatic in Settings → Appearance; selections apply immediately and persist across launches.
- Center the Instant window title and keep long project names clear of the macOS window controls.
- Enlarge the menu-bar Fermat face by reducing transparent padding in its template image.
- Present the menu-bar icon at 20 points and center it independently of text to align with neighboring status icons.
- Simplify the product website with monochrome colors, left-aligned content, compact navigation and concise feature descriptions.
- Detect the website's initial appearance from the OS; switch light/dark with one click on the sun/moon icon and remember the choice.
- Add a public MCP and CLI guide with client setup, available tools, export examples and troubleshooting.

- Bound published image/PDF caches with active-view, drag and clipboard leases; reclaim retired snapshots without deleting saved work.

### Added

- Instant library autosaves equations as you type, restores them across launches, and supports duplication for alternate styling. Standalone Instant is available before project-folder setup.
- macOS screen eyedropper in the shared Instant and Library color picker.
- Product website with visual feature demos, searchable user guides, download requirements and scheduled GitHub release counters.
- Local CLI and MCP tools for project/equation search, template-based or project-aware rendering, batch image export, additive Library variants and editable SVG review in the app.
- Diagnostic log in `~/Library/Logs/local.slidemath.prototype/`, recording compiler runs and app errors.
- Library variants can override output color using the same picker as Instant, with unchanged project sources.
- On-demand LaTeX/Typst formatting in Source, Library variants and Instant, with bundled formatters, undo, and cancellation when input changes.
- Project workspace tabs that retain Library, Source and PDF state independently.
- Restore SVG artwork dragged from PowerPoint into an existing project tab or Instant, opening the appropriate editor when needed.
- Fresh-input release verification, recorded dependency audits and an explicit signed/notarized packaging path with validation manifests.
- Editable source recipes in SVG exports, project-aware Instant restoration, macOS Open With/Dock import, a menu-bar entry and configurable global Instant shortcuts.

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

- Rename the app and distribution bundles to FerMatter; preserve existing project locations, settings, drafts, editable SVG metadata and bundle identity.
- Keep Typst query JSON separate from stderr warnings so Typst 0.15 renders equations and PDF navigation correctly, while retaining diagnostics on failure.
- Use direct PDF output when a project's `latexmkrc` does not choose an output mode, while respecting explicit project compiler settings.
- Apply the default output size to Library copy/drag/export and Source/PDF previews; replace fixed size presets with Default and decimal pt input while preserving saved overrides.
- Keep font-size spinner arrows visible in dark mode, and start Instant preview drags without waiting for an artifact-retention request.
- Restore SVG recipes with an automatic preview, and choose Instant output colors with default swatches, a draggable color picker, recent color history, and HEX input.
- Reuse the running macOS app when launched again, including from another app copy, to prevent duplicate menu-bar icons.
- Protect settings against stale-window overwrites and preserve rapid preference changes with ordered saves and explicit reload after conflicts.
- Stream large project snapshots and hashes with cancellation, bound image allocations and avoid loading every selected image when copying multiple items.
- Reclaim eligible old render/export/PDF caches at startup while preserving saved work and clipboard references.
- Preserve Source and Library drafts before closing or switching projects; failed saves remain retryable. Quit waits for all editor windows.
- Keep recent Library edits when rescans or renders finish late. Source moves retain drafts, main-file settings and shared editor Undo; unsaved files cannot be moved to Trash.
- Release closed project and transient variant editor models without discarding shared split-pane Undo history.

- Validate saved project metadata and offer explicit backup recovery while preserving unreadable originals, source files, drafts, and compatible unknown fields.

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

[Unreleased]: https://github.com/k-onoue/FerMatter/commits/main/

### Security

- Update the PDF parser to reject excessive object nesting instead of aborting the process (RUSTSEC-2026-0187).
