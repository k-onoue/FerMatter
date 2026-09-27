# Mathlidelux Rust — macOS port

- Parallel desktop implementation: Tauri 2, Rust, React/TypeScript, Monaco, PDF.js.
- App name: **Mathlidelux Rust**. Bundle ID: `local.slidemath.prototype.rust-preview`.
- Existing Swift app: [`macos-app/`](../macos-app/); unchanged by this port.
- Runtime: no Python, Node.js, Rust, Swift SDK, or Xcode installation required by app users.
- Compilers remain external: Typst; or TeX/`latexmk`, `pdfcrop`, `pdf2svg`, `pdftocairo`, `pdfinfo`, and `synctex`. See [requirements](../user-docs/requirements.md).

## Build and run

- macOS / Apple Silicon development build; Rust stable, Node.js 22+, Xcode command-line tools.
- Verified development toolchain: Rust 1.98.1, Node.js 26.5.0, macOS 26.5.2.
- A project-local toolchain at `build/rustup/` is optional; scripts otherwise use the installed Rust toolchain.

```sh
cd desktop-app
npm ci
./scripts/dev.sh
./scripts/build.sh
open 'src-tauri/target/release/bundle/macos/Mathlidelux Rust.app'
```

- `./scripts/build.sh --debug`: faster local bundle under `target/debug/bundle/macos/`.
- Local bundles use ad hoc signing. No public release, Developer ID signing, or notarization is implied.
- `npm run tauri build -- --bundles app,dmg`: optional local DMG; use the same Rust environment as the build script.
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
| PDF | Whole-document build, PDF.js viewer, Typst anchors and LaTeX SyncTeX navigation, retained pane width |
| Settings | Light/dark/system appearance, fifteen theme pairs, shortcut JSON, recovery/conflict checks, background opacity, pinning |

## Compatibility boundaries

- Preview metadata: `~/Library/Application Support/Slide Math/Rust Preview/`.
- Existing project metadata and theme/shortcut files are read/copied on first use; original Swift storage is preserved.
- Old card IDs, variant metadata, labels, and unknown project fields are retained. Source scans preserve presentation variants.
- Library edits save to Rust preview metadata. They do not modify the manuscript or synchronize back into the Swift library.
- Source edits modify the selected real project files. Avoid editing the same files in both apps simultaneously; saves check the original bytes for conflicts.
- New source folders: `~/Documents/Mathlidelux/`. ZIP imports also extract there.
- Recovered Rust drafts wait for an edit or explicit save. Swift's unsaved drafts remain in Swift storage; they are not automatically adopted.
- Preview settings use the existing version-1 theme/shortcut schemas; extension metadata survives edits.
- Extraction is literal/static. Symlinked resources are excluded from compilation snapshots; dynamic TeX/Typst programs are not full semantic extraction.

## Remaining release gates

- Complete native macOS interaction verification before treating this as a replacement for the Swift app: Undo/Redo, IME, file operations, PDF navigation, native clipboard/drag into presentation software.
- The Swift full-screen wallpaper resolver is not ported. Rust uses native macOS material; external-monitor/clamshell/full-screen appearance needs separate validation.
- Existing manual library ordering and Swift draft migration are not replacement-complete.
- Rust release updates need their own signed release artifacts/feed. The preview does not offer Swift builds as Rust updates.
- Intel macOS, Ubuntu, and Windows have not been built or tested. Native clipboard, reveal, and no-overwrite rename adapters currently target macOS.
- Public signing/notarization and installer/update verification remain release work.

## Validation

```sh
./desktop-app/scripts/check.sh
./macos-app/scripts/check.sh
./macos-app/scripts/build.sh
.venv-docs/bin/python scripts/build-guides.py --check
python3 -B scripts/check-publication.py
git diff --check
```

- Rust unit tests: extraction compatibility, Unicode positions, conflict saves, encoding/newlines, snapshots, settings recovery, process cancellation, SVG/raster export.
- Frontend tests: nested editor-group state, independent tabs, pasted equation composition, and template validation.
- Real-compiler check: four synthetic `examples/` projects; scans, all image formats, scaled vector PDF, document PDFs, SyncTeX forward/reverse navigation, unchanged source fingerprints.
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
| `src-tauri/src/lib.rs` | Tauri IPC, session ownership, OS operations |
| `src/editor.tsx`, `SourceWorkspace.tsx`, `panes.ts` | Shared Monaco models, source buffers and groups |
| `src/PdfPane.tsx`, `settings.tsx`, `App.tsx` | Viewer, preferences and application workflows |

## Original references

- [Tauri architecture](https://v2.tauri.app/concept/architecture/).
- [Tauri distribution](https://v2.tauri.app/distribute/).
- [Native drag plugin](https://docs.crabnebula.dev/plugins/drag-rs/).
- [Monaco Editor](https://github.com/microsoft/monaco-editor).
- [PDF.js](https://github.com/mozilla/pdf.js).
