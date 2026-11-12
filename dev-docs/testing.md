# Testing

## Validation by scope

| Change scope | Required checks |
| --- | --- |
| **Desktop App (macOS)** | `./desktop-app/scripts/check.sh`<br>`./desktop-app/scripts/build.sh` |
| **Desktop App (Windows)** | `./desktop-app/scripts/check-windows.ps1`<br>`./desktop-app/scripts/build-windows.ps1` |
| **Documentation & Website** | `node scripts/build-guides.mjs --check`<br>`node scripts/check-publication.mjs`<br>Preview affected pages in a browser |
| **Synthetic Samples** | `node scripts/prepare-demo.mjs`<br>Verify compiler checks and publication |
| **Formatting** | `npm run format:check`<br>`git diff --check` |

## Desktop app test suites

`check.sh` executes the standard verification pipeline:

1. **Frontend build & tests:** Typecheck/Vite build, Vitest suites, and script checks in `desktop-app/`.
2. **Rust tests:** `cargo test --manifest-path desktop-app/src-tauri/Cargo.toml --lib`. Covers scanners, encoding, atomic saves, snapshots, and cancellations; macOS also checks WebView migration.
3. **Compiler integration tests:** Runs all four synthetic projects under `examples/` with the required system compilers.
   - Set `MATHLIDELUX_COMPILER_TESTS=0` to explicitly bypass real-compiler tests during fast iteration.

### Windows testing

Run from repository root in PowerShell:

```powershell
./desktop-app/scripts/check-windows.ps1
./desktop-app/scripts/build-windows.ps1
```

Use `-SkipCompilers` for fast local iterations without TeX/Typst tools.

- `build.rs` embeds the Common Controls v6 manifest into MSVC app/test binaries; omission can prevent tests from starting ([upstream issue](https://github.com/tauri-apps/tauri/issues/13419)).
- Installer checks wait for the forwarded app's `Ready` log and retain failure evidence. See [packaging verification](reference/packaging.md#windows-preview) for clean-install, compiler, upgrade and uninstall checks.

## Documentation and publication checks

```sh
npm --prefix scripts ci
npm --prefix site ci
node scripts/build-guides.mjs --check   # Runs astro check and static production build
node scripts/check-publication.mjs      # Validates links, anchors, and release data
git diff --check                       # Ensures no trailing whitespace or merge markers
```

See [documentation guide](documentation.md#build-and-preview) for local preview commands.

## Record results

- Record the exact candidate, OS/build, CPU and native/CI scope in the [platform validation record](reference/platform-validation.md).
- Report skipped compiler tests and unverified native UI operations explicitly; automated passes do not establish clipboard/drag, IME or clean-machine behavior.
- Release candidates also require audits and the [release checklist](releasing.md).
