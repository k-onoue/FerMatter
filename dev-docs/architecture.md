# Architecture

- Maintained app: [desktop-app](../desktop-app/README.md), Tauri 2 + Rust + React/TypeScript.
- Monaco supplies the shared editor; PDF.js displays document PDFs.
- Rust invokes external LaTeX/Typst compilers in temporary project snapshots; no Python worker runtime.
- The retired Swift/Python implementation is available in Git history before its removal PR.

## Responsibilities

| Area | Location under `desktop-app/` |
| --- | --- |
| Workflows, selection, Library variants | `src/App.tsx`, `src/variants.ts` |
| Shared Monaco models, coloring, completion | `src/editor.tsx` |
| Source buffers, file tree, tabs and split panes | `src/SourceWorkspace.tsx`, `src/panes.ts` |
| PDF viewer, source ranges and hit matching | `src/PdfPane.tsx`, `src/navigation.ts` |
| Settings and project templates | `src/settings.tsx`, `src/templates.ts`, `src/NewProjectDialog.tsx` |
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
