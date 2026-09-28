# Architecture

- Maintained app: [desktop-app](../desktop-app/README.md), Tauri 2 + Rust + React/TypeScript.
- Monaco supplies the shared editor; PDF.js displays document PDFs.
- Rust invokes external LaTeX/Typst compilers in temporary project snapshots; no Python worker runtime.
- The retired Swift/Python implementation is available in Git history before its removal PR.

## Responsibilities

| Area | Location under `desktop-app/` |
| --- | --- |
| Project selection, imports, duplication and undo | `src/ProjectsPage.tsx`, `src/projectSelection.ts`, `src-tauri/src/project_commands.rs`, `src-tauri/src/project_location.rs`, `src-tauri/src/project_history.rs` |
| First-launch folder choice and demo migration | `src/ProjectSetup.tsx`, `src-tauri/src/project_setup.rs` |
| Workflows, selection, Library variants | `src/App.tsx`, `src/variants.ts` |
| Library outline/labels and variant editor | `src/LibraryNavigation.tsx`, `src/VariantEditor.tsx` |
| Shared Monaco models, coloring, completion | `src/editor.tsx` |
| Source buffers, file tree, tabs and split panes | `src/SourceWorkspace.tsx`, `src/panes.ts` |
| Source-file mutation commands | `src-tauri/src/source_commands.rs` |
| PDF viewer, source ranges and hit matching | `src/PdfPane.tsx`, `src/navigation.ts` |
| Settings and project templates | `src/settings.tsx`, `src/templates.ts`, `src/NewProjectDialog.tsx` |
| Shared theme palette and window/editor appearance | `src/appearance.ts`, `src/useAppearance.ts` |
| Project-free Instant window, drafts and history | `src/InstantWindow.tsx`, `src/instant.ts`, `src-tauri/src/instant.rs` |
| Shortcut matching and defaults | `src/shortcuts.ts`; matching Rust validation in `src-tauri/src/settings.rs` |
| IPC and request ownership | `src-tauri/src/lib.rs` |
| LaTeX/Typst extraction, includes and outline | `src-tauri/src/scan.rs` |
| Compiler snapshots, cancellation and SyncTeX | `src-tauri/src/compiler.rs` |
| Safe paths, atomic writes and conflict checks | `src-tauri/src/files.rs` |
| Project metadata and legacy draft import | `src-tauri/src/storage.rs` |
| Settings validation, recovery and migration | `src-tauri/src/settings.rs` |
| Image export and macOS clipboard | `src-tauri/src/export.rs`, `src-tauri/src/native.rs` |
| Full-screen wallpaper fallback | `src-tauri/src/wallpaper.rs`, `src/useDesktopBackground.ts` |
| Themes, artwork and attribution | `resources/`, `src-tauri/icons/` |

## Request lifecycle

- Reserve compiler request ownership before queuing blocking work.
- Replacing/cancelling a request terminates its process group; ignore late callbacks using request/project identity.
- Keep separate render, caret-preview, document and forward/reverse-navigation jobs.
- Namespace request ownership by the calling window; closing Instant must not cancel the project window's jobs.
- `instant` is global; `instant-<project-id>` owns local scratch state. Resolve its scope from the native window label, not the request's project field.
- Project Instant compiles saved project snapshots with existing preamble/import context; it does not alter manuscripts or consume unsaved Source buffers.
- Compile snapshots; do not write manuscripts to render a card or PDF.
- PDF navigation targets refer to the compiled snapshot; rebuild after source changes.

## Editor and persistence

- Share Monaco models between panes; retain independent tab/selection state.
- Source saves compare original bytes and write atomically; retain encoding/newlines.
- Recover drafts before opening tabs. Recovered drafts require an edit or explicit save.
- Library variants only change metadata; Source mode changes real files.
- Remap open buffers/tabs after file rename or move.
- Use UTF-16 offsets consistently for scans, caret previews and PDF navigation.
- See [storage](reference/storage.md) for legacy import and conflict handling.

## Limits and validation

- Extraction is literal/static, not full TeX/Typst execution: [user limits](../user-docs/limits.md).
- TeX, `latexmkrc`, and project code are executable input; temporary copies are not an OS sandbox.
- [Testing](testing.md), [reviewing](reviewing.md), [historical validation](history/validation-2026-09.md).

- Finder drops keep WebKit HTML drag handling enabled for Source tabs and artwork. `project_drop` captures file URLs from the actual `NSDraggingInfo` pasteboard before WebKit handles the drop. The DOM drop consumes this one-use, short-lived capture after matching filenames. Do not globally enable Tauri native drop interception on macOS.
