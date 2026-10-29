# Testing

## Choose checks by scope

| Change                          | Required evidence                                                                                                                                                        |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| App, frontend, compiler backend | macOS: `./desktop-app/scripts/check.sh`; `./desktop-app/scripts/build.sh`. Windows: `./desktop-app/scripts/check-windows.ps1`; `./desktop-app/scripts/build-windows.ps1` |
| User docs, site, site generator | Build guides; freshness check; publication check; browser preview                                                                                                        |
| Developer docs only             | Publication/link check; review commands and referenced behavior                                                                                                          |
| Synthetic samples               | `node scripts/prepare-demo.mjs`; compiler checks; publication check                                                                                                      |
| Packaging                       | [Packaging verification](reference/packaging.md); distinguish inspection from installation                                                                               |
| Release                         | All affected checks; real compiler coverage; [release checklist](releasing.md)                                                                                           |

## App checks

- `check.sh`: TypeScript/Vite production build, frontend tests, Rust tests, four synthetic real-compiler checks.
- TypeScript rejects unused imports, locals, and parameters. Keep the main/Instant appearance hook and command modules shared instead of copying handlers.
- Real compiler checks run by default; `MATHLIDELUX_COMPILER_TESTS=0` explicitly skips them.
- Missing compilers fail the default run. A skipped run is not a release pass.
- Run `cargo clippy --manifest-path desktop-app/src-tauri/Cargo.toml --lib -- -D warnings` with the same Rust environment.
- PR CI runs frontend/Rust checks on macOS with compiler checks explicitly skipped. The separate [release gate](reference/release-validation.md) requires all four real compiler paths, an app build and dependency audits; it records skips/failures distinctly.
- Build verifies the local ad hoc signature; it does not verify notarization.
- Test storage/source changes with disposable copies and external-edit conflicts.
- Test worker changes with cancellation and late results after switching projects.
- Test exports by opening the result in the target app; do not infer drag support from copy support.

### Windows checks

```powershell
./desktop-app/scripts/check-windows.ps1
./desktop-app/scripts/build-windows.ps1
./desktop-app/scripts/verify-windows-install.ps1 -Installer ./desktop-app/build/windows/FerMatter_0.11.0_windows_x64_preview.4_unsigned_setup.exe -CheckCompilers
```

- `check-windows.ps1` builds/tests the frontend, runs Rust tests and clippy, and verifies all four synthetic compiler projects. `-SkipCompilers` is an explicit development-only skip; it is not release evidence.
- `build.rs` embeds the Common Controls v6 manifest into MSVC app and test binaries. Tauri mock-runtime tests otherwise fail before execution with `STATUS_ENTRYPOINT_NOT_FOUND`; see [the upstream issue](https://github.com/tauri-apps/tauri/issues/13419).
- `./desktop-app/scripts/audit-windows.ps1` records production npm and Cargo dependency audits with a hash-pinned official cargo-audit binary. Release candidates require no reported vulnerabilities; retain informational warnings in the validation record.
- Install the packaged EXE into a disposable directory; verify bundled resources/notices, app startup, project-folder setup, Library/Source/Instant, rendered SVG/PNG/JPEG/PDF, document navigation and cancellation.
- `verify-windows-install.ps1 -CheckCompilers` requires an account/CI machine without an existing FerMatter installation. It tests the installed EXE's LaTeX align/equation and Typst paths with a minimal Windows system PATH, verifies SVG/PNG/PDF artifacts and unchanged synthetic sources, and retains compiler evidence beside the install report. Omission of the switch is recorded as a compiler-check skip.
- Install tools from a normal PowerShell/Explorer context, or into a physical user tools directory outside AppData. An [MSIX-packaged host can redirect AppData writes](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-behind-the-scenes#appdata-operations-on-windows-10-version-1903-and-later) into its private cache; a passing child-process check inside that host does not prove that an app launched from Explorer can see those tools. Include an Explorer-launched compiler check on the final machine.
- Check Ctrl/Alt shortcuts, IME, clipboard and drag into a receiving app separately. A CLI compiler pass does not establish native UI behavior.
- Run upgrade/uninstall checks with synthetic saved data and projects; record skipped clean-machine or receiving-app checks.

### Typst version compatibility

- Keep downloaded compiler binaries outside Git; leave installed tools/settings unchanged.
- Run against each supported executable (including 0.14.2 and 0.15.1):

```sh
source desktop-app/scripts/environment.sh
cargo run --manifest-path desktop-app/src-tauri/Cargo.toml --example verify -- . desktop-app/build/typst-compat /absolute/path/to/typst
node desktop-app/scripts/verify-automation.mjs /absolute/path/to/FerMatter /absolute/path/to/typst
```

- The optional verifier argument checks all Typst/CeTZ sample cards, measured font sizes, document PDF navigation and source preservation in fresh snapshots.
- The CLI/MCP check uses an isolated `tools.json` override and verifies exports, progress and cancellation.

## Documentation/site checks

```sh
npm ci
npm run format:check
npm --prefix scripts ci
npm --prefix site ci
npm --prefix scripts test
node scripts/build-guides.mjs --check
node scripts/check-publication.mjs
git diff --check
```

- Environment setup and preview: [documentation](documentation.md#build-and-preview).
- Check desktop/mobile widths, keyboard navigation, light/dark appearance, and JavaScript disabled.
- Check video play/pause, caption track, poster, and the same steps as text.
- Serve only `site/dist/`; also test at the root and under the legacy `/FerMatter/` project subpath.
- CI checks Astro diagnostics, static output links, safe public fixtures, download counters, and whitespace.

## Record results

- Record OS/build, CPU architecture, exact candidate, native/CI scope and unverified operations in the [platform validation record](reference/platform-validation.md).
- PR: commands, outcomes, relevant environment, skips, and limitations.
- Historical experiments: [history](history/validation-2026-09.md); do not copy old counts into a new validation claim.
- User-facing changes: [CHANGELOG.md](../CHANGELOG.md), not the test log.
- Reference: [Reviewing tests](https://google.github.io/eng-practices/review/reviewer/looking-for.html#tests).

## Cache lifetime regression checks

- Synthetic small-budget Rust fixtures cover repeated publication, competing producers, active PDF/work-directory leases, clipboard retention, grace expiry, recovery/Undo references, and original/draft preservation.
- Frontend drag tests distinguish invoke completion from native drag completion; early unmount releases only after lease acquisition.
- The compiler verifier enables cache admission in its isolated data directory and regenerates deliberately removed Typst SVG/PNG/PDF artifacts.
- No long-running user dataset, real Finder drag, or PowerPoint session is changed by these checks.
