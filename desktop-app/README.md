# Mathlidelux desktop

- Desktop application: Tauri 2, Rust, React/TypeScript, Monaco, PDF.js.
- App name: **Mathlidelux**. Bundle ID: `local.slidemath.prototype` (preserved from the Swift app).
- This is the maintained implementation. The retired Swift/Python source remains available in Git history.
- Runtime: no Python, Node.js, Rust, Swift SDK, or Xcode installation required by app users.
- Compilers remain external: Typst; or TeX/`latexmk`, `pdfcrop`, `pdf2svg`, `pdftocairo`, `pdfinfo`, and `synctex`. See [requirements](../user-docs/requirements.md).

## Build and run

- Apple Silicon development build; macOS 14+ deployment target, verified on macOS 26. Rust stable, Node.js 22+, Xcode command-line tools.
- Verified development toolchain: Rust 1.98.1, Node.js 26.5.0, macOS 26.5.2.
- A project-local toolchain at `build/rustup/` is optional; scripts otherwise use the installed Rust toolchain.

```sh
cd desktop-app
npm ci
./scripts/dev.sh
./scripts/build.sh
open 'src-tauri/target/release/bundle/macos/Mathlidelux.app'
```

- `./scripts/build.sh --debug`: faster local bundle under `target/debug/bundle/macos/`.
- Local bundles default to ad hoc signing. To retain macOS folder consent across rebuilds, configure a stable local certificate using the [signing guide](../dev-docs/reference/signing.md). No public release or notarization is implied.
- `./scripts/package.sh`: verified local DMG and PKG; artifacts remain unnotarized prototypes.
- Dependency notices are generated into `public/vendor/` and included in the bundle. Theme source notices remain with the bundled theme files.

## Implemented paths

| Area | Rust desktop implementation |
| --- | --- |
| Projects | Existing folder/main file, validated ZIP import, new project from pasted content, editable templates |
| Extraction | LaTeX/Typst literal equations, TikZ/CeTZ, includes, document outline, stable source IDs |
| Library | Search/type/outline/label filters, range/toggle/rectangle selection, double-click variants, per-item output size |
| Images | SVG default; PNG/JPEG/vector PDF; native outbound file drag and image clipboard |
| Source | Real file editing, tree operations, shared editor models, nested horizontal/vertical panes, draggable/middle-click tabs |
| Editing | Monaco Undo/Redo, line numbers/actions, syntax coloring, command completion, project definitions, recoverable drafts |
| Preview | Unsaved source snapshot at caret; export/drag; isolated compiler processes with cancellation |
| PDF | Whole-document build, single-click PDF ↔ Library/Source navigation, selected-equation highlight, caret tracking, retained pane width |
| Settings | Light/dark/system appearance, fifteen theme pairs, shortcut JSON, recovery/conflict checks, background opacity, pinning |

## Compatibility boundaries

- Metadata: `~/Library/Application Support/Slide Math/Rust Preview/`.
- Existing project metadata and theme/shortcut files are read/copied on first use; original Swift storage is preserved.
- Old card IDs, variant metadata, labels, and unknown project fields are retained. Source scans preserve presentation variants.
- Library edits save to the existing internal `Rust Preview` directory. Keeping this name preserves earlier preview edits and cached paths; Swift metadata remains a read-only import source.
- Source edits modify the selected real project files. Avoid editing the same files in both apps simultaneously; saves check the original bytes for conflicts.
- New source folders: `~/Documents/Mathlidelux/`. ZIP imports also extract there.
- Swift drafts are imported once, preserving original-byte conflict checks, BOM and line endings. Current drafts win collisions; the complete Swift draft file is backed up under `MigratedSwiftDrafts/` and collisions are reported.
- Recovered drafts wait for an edit or explicit save; originals and source files are not changed during import.
- Settings use the existing version-1 theme/shortcut schemas; extension metadata survives edits.
- Extraction is literal/static. Symlinked resources are excluded from compilation snapshots; dynamic TeX/Typst programs are not full semantic extraction.
- PDF navigation uses the compiled snapshot. Rebuild the PDF after changing source text; Library variants navigate to their original equation.
- When SyncTeX reports several inline equations on one line without a column, Library selects those candidates; Source navigates to the line.
- Background opacity changes only background layers. In native full-screen Spaces, the active display's image/aerial wallpaper is used as a fallback; unrelated defaults and ambiguous shuffled/Space-specific choices are never guessed.

## Remaining release gates

- The maintainer verified the current UI and approved the replacement. Broader native checks remain before public release: Undo/Redo, IME, file operations, clipboard/drag into presentation software.
- External-monitor changes and clamshell transitions still need physical verification. Full-screen wallpaper and background-opacity changes have been checked on the current screen.
- Existing manual Library order metadata is retained; the current UI displays source order and does not expose manual ordering.
- In-app release checks/downloads are not yet implemented in this app; install newer builds manually. Signed updates need a separate release feed/artifact setup.
- Intel macOS, Ubuntu, and Windows have not been built or tested. Native clipboard, reveal, and no-overwrite rename adapters currently target macOS.
- Public signing/notarization and installer/update verification remain release work.

## Validation

```sh
./desktop-app/scripts/check.sh
./desktop-app/scripts/build.sh
node scripts/build-guides.mjs --check
node scripts/check-publication.mjs
git diff --check
```

- Rust unit tests: extraction compatibility, Unicode positions, conflict saves, encoding/newlines, snapshots, settings recovery, process cancellation, SVG/raster export, active-display wallpaper selection.
- Frontend tests: nested editor-group state, independent tabs, pasted equation composition, template validation, source-range and PDF hit matching.
- Real-compiler check: four synthetic `examples/` projects; scans, all image formats, scaled vector PDF, document PDFs, equation/align SyncTeX round trips, unchanged source fingerprints.
- Native UI checks: LaTeX/Typst card ↔ PDF, PDF → exact Source selection, Source caret → PDF; LaTeX cross-page navigation; full-screen wallpaper and 45%/72% background opacity with opaque text.
- Comparison fixture: [`tests/fixtures/extraction.json`](tests/fixtures/extraction.json), captured from the existing Python workers using only synthetic examples. Tests do not require Python.
- Compiler checks run by default. `MATHLIDELUX_COMPILER_TESTS=0` explicitly prints a skip.
- Isolated UI QA: set `MATHLIDELUX_DATA_DIR` to `desktop-app/build/qa-data` when launching the executable. This excludes the real legacy library and uses only the generated examples.
- UI QA blocked by a locked Mac must be reported as unverified, never as a pass.

## Code map

| Location | Responsibility |
| --- | --- |
| `src-tauri/src/scan.rs` | Source scanners and compatible IDs/positions |
| `src-tauri/src/compiler.rs` | Isolated compilers, cancellation, PDFs and SyncTeX |
| `src-tauri/src/files.rs`, `storage.rs` | Path boundaries, conflicts, snapshots, archives and metadata |
| `src-tauri/src/export.rs`, `native.rs` | Image scaling/encoding and macOS clipboard |
| `src-tauri/src/settings.rs` | Validated file settings, recovery and external-edit conflicts |
| `src-tauri/src/wallpaper.rs` | Read-only active-display wallpaper selection and cached native image decoding |
| `src-tauri/src/lib.rs` | Tauri IPC, session ownership, OS operations |
| `src/editor.tsx`, `SourceWorkspace.tsx`, `panes.ts` | Shared Monaco models, source buffers and groups |
| `src/PdfPane.tsx`, `settings.tsx`, `App.tsx` | Viewer, preferences and application workflows |

## Original references

- [Tauri architecture](https://v2.tauri.app/concept/architecture/).
- [Tauri distribution](https://v2.tauri.app/distribute/).
- [Native drag plugin](https://docs.crabnebula.dev/plugins/drag-rs/).
- [Monaco Editor](https://github.com/microsoft/monaco-editor).
- [PDF.js](https://github.com/mozilla/pdf.js).
