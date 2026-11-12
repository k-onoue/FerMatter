# Architecture

- **Stack:** Tauri 2 (Rust) + React 19 / TypeScript + Monaco Editor + PDF.js.
- **External Compilers:** Rust invokes system LaTeX and Typst tools inside isolated temporary snapshots.
- **Automation:** Stdio MCP server and CLI are built directly into the macOS/Linux binary.

## Module responsibilities

| Area | Implementation (`desktop-app/`) |
| --- | --- |
| Project selection, import, undo | `src/ProjectsPage.tsx`, `src/projectSelection.ts`, `src-tauri/src/project_commands.rs`, `project_location.rs`, `project_history.rs` |
| Workspace tabs, active state | `src/workspaces.ts`, `src/WorkspaceTabs.tsx`, `src/App.tsx` |
| Library navigation, variant editor | `src/LibraryNavigation.tsx`, `src/VariantEditor.tsx`, `src/variants.ts` |
| Monaco editor & formatters | `src/editor.tsx`, `src/formatting.ts`, `src/latexFormatting.ts`, `src-tauri/src/formatting.rs` |
| Source file tree, editor panes | `src/SourceWorkspace.tsx`, `src/panes.ts`, `src-tauri/src/source_commands.rs` |
| PDF viewer & SyncTeX matching | `src/PdfPane.tsx`, `src/navigation.ts`, `src-tauri/src/compiler.rs` |
| Settings & preferences | `src/settings.tsx`, `src/appearance.ts`, `src-tauri/src/settings.rs` |
| Color picker & screen sampler | `src/ColorPicker.tsx`, `src/outputColor.ts`, `src-tauri/src/color_sampler.rs` |
| Instant window & preview | `src/InstantWindow.tsx`, `src/instant.ts`, `src-tauri/src/instant.rs` |
| Scanner & extraction | `src-tauri/src/scan.rs`, `src-tauri/src/extraction.rs` |
| Compilers & process cancellation | `src-tauri/src/compiler.rs` |
| Files, storage & migration | `src-tauri/src/files.rs`, `src-tauri/src/storage.rs` |
| Image export & native clipboard | `src-tauri/src/export.rs`, `src-tauri/src/native.rs` |
| Local MCP server & CLI | `src-tauri/src/automation/` |
| Tauri IPC & window lifecycle | `src-tauri/src/lib.rs` |

## Core principles & invariants

1. **Library vs. Source:**
   - Library edits create and modify presentation variants (metadata only). The original manuscript files remain untouched.
   - Source mode directly edits real project files on disk with atomic writes.
2. **Compiler isolation & cancellation:**
   - Compilations run against temporary project snapshots, never modifying the source directory to render.
   - Superseded render or navigation requests terminate the entire compiler process group immediately. Late callbacks are ignored.
3. **Synchronization & offsets:**
   - UTF-16 code units are used consistently across scans, Monaco caret previews, and SyncTeX PDF navigation hits.
   - PDF navigation targets are tied to the compiled snapshot; source edits require rebuilding the PDF to re-map.
4. **Limits & defense in depth:**
   - Snapshot creation streams 64 KiB chunks with a 1 GiB project cap. Editor reads cap at 10 MB, scans at 100 MB.
   - Rust validates all artifact and export paths against canonical managed directories (`Cache`, `Exports`, `Output`).
   - Literal extraction does not execute arbitrary TeX macros or dynamic Typst scripts: see [extraction scope](../user-docs/requirements.md#extraction-limitations).
   - Rendering does execute project code and compiler configuration. Temporary snapshots and path checks are not an OS sandbox.
