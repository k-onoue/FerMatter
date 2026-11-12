# FerMatter desktop

Desktop application built with Tauri 2, Rust, React 19, TypeScript, Monaco Editor, and PDF.js.

- **Permanent Bundle ID:** `io.github.fermatter.fermatter`; [migration](../dev-docs/reference/storage.md#app-identity-migration) preserves legacy data and resident coordination.
- **External Compilers:** Typst CLI, or TeX (`latexmk`, `pdfcrop`, Poppler, SyncTeX); macOS/Linux also use `pdf2svg`. See [requirements](../user-docs/requirements.md).
- **No Extra Runtimes:** Users do not need Python, Node.js, or Rust installed.

## Build and run

### macOS (Apple Silicon, macOS 14+)

Requires Rust stable (1.92+), Node.js (24.22.1+ LTS, see `../.node-version`), and Xcode command-line tools.

```sh
cd desktop-app
npm ci
./scripts/dev.sh      # Start Vite dev server and Tauri
./scripts/build.sh    # Production bundle (or --debug for fast local bundle)
open 'src-tauri/target/release/bundle/macos/FerMatter.app'
```

- Ad hoc signing is used by default. To persist macOS folder permissions across builds, see [local signing](../dev-docs/reference/signing.md).
- To package DMG and PKG installers: `./scripts/package.sh`.

### Windows 11 x64

Requires Rust MSVC, Node.js, Visual Studio C++ Build Tools, and Windows SDK. See [Windows setup](../dev-docs/development.md#windows).

```powershell
./desktop-app/scripts/check-windows.ps1
./desktop-app/scripts/build-windows.ps1
./desktop-app/src-tauri/target/release/FerMatter.exe
```

## Implemented features

| Area | Capabilities |
| --- | --- |
| **Projects** | Open existing directory/file, ZIP import, new project from clipboard/template. |
| **Extraction** | Equations and TikZ/CeTZ diagrams, `input`/`include` tracking, document outline. |
| **Library** | Search, filter by type/outline, custom presentation variants (size, color). |
| **Images** | SVG with embedded source metadata, PNG, JPEG, vector PDF; clipboard and native drag. |
| **Source** | File tree, split panes, draggable tabs, Monaco editor with undo/redo and formatting. |
| **Preview** | Caret-level equation preview with cancellation and background compiling. |
| **PDF** | Document view, bi-directional SyncTeX navigation with Library/Source. |
| **Automation** | Local stdio MCP server and CLI on macOS/Linux. |
| **Appearance** | Light/dark/system themes, 16 color palettes, background opacity, always-on-top. |

## Release status

- macOS builds are ad hoc signed and unnotarized; Windows previews are unsigned. See [platform validation](../dev-docs/reference/platform-validation.md) for tested environments and remaining checks.
- In-app update checks and installation are implemented; the stable feed remains disabled pending packaged upgrade tests. Preview updates are manual or through Homebrew/APT. See [updates](../dev-docs/reference/updates.md).
- Ubuntu packages are experimental. See [packaging](../dev-docs/reference/packaging.md#experimental-ubuntu) for platform limitations.

## Code map

| Location | Responsibility |
| --- | --- |
| `src-tauri/src/scan.rs` | Source scanners, equation/diagram extraction |
| `src-tauri/src/compiler.rs` | Compiler process lifecycle, cancellation, PDFs, SyncTeX |
| `src-tauri/src/files.rs`, `storage.rs` | Paths, atomic writes, snapshots, metadata, migration |
| `src-tauri/src/export.rs`, `native.rs` | Image rasterization, scaling, clipboard, native drag |
| `src-tauri/src/settings.rs` | Settings persistence, recovery, shortcut validation |
| `src-tauri/src/automation/` | Stdio CLI and MCP service |
| `src-tauri/src/lib.rs` | Tauri IPC handlers and application lifecycle |
| `src/editor.tsx`, `SourceWorkspace.tsx` | Monaco editor setup, source buffers, layout panes |
| `src/PdfPane.tsx`, `LibraryNavigation.tsx` | PDF viewer, outline, card list, hit testing |
| `src/App.tsx`, `settings.tsx` | Main application shell, state management, preferences |

## Validation

```sh
./desktop-app/scripts/check.sh        # Typecheck, frontend tests, Rust tests, compiler tests
./desktop-app/scripts/build.sh        # Application bundle build
git diff --check                      # Check formatting and whitespace
```

For legacy migration and storage details, see [storage reference](../dev-docs/reference/storage.md). For release requirements, see [releasing](../dev-docs/releasing.md).
