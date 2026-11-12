# Development

## Setup

<span id="macos"></span>

### macOS (Apple Silicon, macOS 14+)

Prerequisites: Rust stable (1.92+), Node.js (24.22.1+ LTS, see `.node-version`), Xcode command-line tools.

```sh
./desktop-app/scripts/build.sh
open desktop-app/src-tauri/target/release/bundle/macos/FerMatter.app
```

- Ad hoc signed by default. For persistent macOS folder permissions across builds, see [local signing](reference/signing.md).
- To run development server with hot reload: `./desktop-app/scripts/dev.sh`.

<span id="windows"></span>

### Windows 11 x64

Prerequisites: Rust MSVC (1.92+), Node.js (24.22.1+ LTS), Visual Studio C++ Build Tools with Desktop C++ workload, and Windows SDK.

```powershell
./desktop-app/scripts/check-windows.ps1
./desktop-app/scripts/build-windows.ps1
./desktop-app/src-tauri/target/release/FerMatter.exe
```

- `-DebugBuild` creates an unoptimized build for fast iteration.
- Output installer metadata: `desktop-app/build/windows/`.

<span id="formatting"></span>

## Formatting and style

```sh
npm ci
npm run format         # Prettier for web/config files, then rustfmt
npm run format:check   # Formatting check in CI
cargo fmt --manifest-path desktop-app/src-tauri/Cargo.toml
```

- Shared configurations: `.editorconfig`, `.prettierrc.json`, `rustfmt.toml`.
- CI checks web/config files and Rust. Markdown is reviewed separately.

<span id="branch-and-pr"></span>

## Branch and PR workflow

1. Branch from `origin/main` (`git switch -c feat/my-change origin/main`).
2. Keep each PR focused on a single change with associated tests and docs.
3. Validate locally:
   - App changes: `./desktop-app/scripts/check.sh`
   - Documentation: `node scripts/build-guides.mjs --check && node scripts/check-publication.mjs`
   - Format: `git diff --check`
4. Open a PR against `main`. Require peer approval or explicit owner override before merge.
