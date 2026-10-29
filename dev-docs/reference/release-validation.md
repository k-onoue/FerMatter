# Release validation evidence

## Automated gate

```sh
cargo install --locked cargo-audit --version 0.22.2 --root desktop-app/build/audit-tools
./desktop-app/scripts/release-check.sh
```

- Run on the exact committed candidate with all compiler tools installed.
- `MATHLIDELUX_COMPILER_TESTS=0` is rejected. Missing tools, failed tests/builds/audits and publication failures fail the gate.
- `--allow-dirty` permits a local rehearsal; its report cannot authorize public packaging.
- The publication step rebuilds `site/dist` from the checkout before checking it (installing `scripts/` and `site/` dependencies with `npm ci` when missing), so a stale site build cannot pass.
- Evidence: `desktop-app/build/release-validation/run.*/report.json`, step logs, audit JSON and real-compiler report.
- Record commit, final commit, clean/dirty state, tool versions, architecture, OS, app file hashes, version and build number.
- Compiler verification always copies current `examples/` into a unique `build/qa-data/verify-*` directory with empty caches; old QA artifacts cannot satisfy a new run.
- Keep old verification directories only while needed for inspection; they are ignored test output, not user data.
- [Release validation workflow](../../.github/workflows/release-validation.yml): manually dispatched Apple Silicon job installs the real tools (BasicTeX with the Windows workflow TeX packages, Ghostscript, pdf2svg, Poppler and the pinned Typst 0.15.1) and runs the same gate; no signing secrets, release creation or publication.
- `audit.sh` records npm production and Cargo lockfile advisories. Vulnerabilities fail; informational maintenance/platform warnings remain visible and need review. Never add blanket ignores to make the gate pass.

## Final artifact checklist

- Copy this table into the release issue; replace `Pending` with the date, environment and evidence.
- Use synthetic `examples/` only. Do not reset real user storage or permissions.
- Run against the **final DMG/PKG**, with Gatekeeper enabled, on a clean supported Mac or macOS VM. Record the ad hoc app signature and unsigned/unnotarized installer state.
- Minimum deployment target is not evidence that the oldest supported OS passed.

| Operation | Required evidence | Result |
| --- | --- | --- |
| Anonymous download | Unauthenticated asset download; checksum matches manifest | Pending |
| DMG installation | Copy into Applications; record per-app opening approval and launch with Gatekeeper enabled | Pending |
| PKG installation | Only Applications app payload installed; launch succeeds | Pending |
| Missing compilers | First setup and Settings name missing tools and accept configured paths | Pending |
| First setup | Choose parent once; all four demos land below it; no repeated setup on relaunch | Pending |
| Rendering | LaTeX, Typst, TikZ, CeTZ preview/PDF/export after tools configured | Pending |
| Finder import | Drop a synthetic folder and ZIP; copied below configured parent; originals unchanged | Pending |
| PowerPoint output | Single/multiple SVG copy and drag; selected images pasted separately | Pending |
| macOS permission | Record any renewed folder consent after ID/ad hoc signature changes; chosen project parent remains usable | Pending |
| Update | Replace older build; project roots, variants, labels, SourceDrafts, Instant history and legacy Slide Math data survive | Pending |
| Close/save failure | Last typed characters retained; failed save offers recovery; reopen restores draft | Pending |
| External edits | Conflict shown; existing source never silently overwritten | Pending |
| Removal | Removing application leaves project folders and saved data intact | Pending |

- Automated functional checks do not mark these rows as passed.
- In-app updates require separate signed-artifact, failure/recovery and native restart checks; follow [the updater procedure](updates.md). Manual replacement remains necessary for previews without the updater. Local test feeds do not activate or validate public update delivery.
- Keep stable download links disabled until this checklist and [release conditions](../releasing.md#public-distribution-checklist) pass. Unsigned/unnotarized status is permitted but must be recorded. A preview does not mark these rows as passed.

## Windows and Ubuntu

- Windows automated gate: `./desktop-app/scripts/release-check-windows.ps1`; no skipped compilers or dirty candidates. Its report binds the NSIS bytes to the commit before packaging.
- The Windows workflow also installs the hash-pinned published preview, upgrades to the candidate, checks old/new resident compatibility, runs installed compilers and uninstalls with synthetic data preserved. This is Windows Server CI, not Windows 11 native UI validation.
- On a clean Windows 11 x64 account, repeat install/update, real Library/Source/Instant edits and recovery, PDF, IME/AltGr, clipboard/drag into a presentation app, and removal. Record OS build, candidate hash and SmartScreen/Smart App Control results.
- Ubuntu stays experimental: CI covers real compilers, Clippy, audits, signed-APT installation and headless startup. Record native desktop/session and receiving-app behavior separately; do not infer Wayland coverage.
- Anonymous download and public package-manager update checks belong to step 6. Before publication, validate local artifacts and preserve hashes; mark public delivery checks pending.
