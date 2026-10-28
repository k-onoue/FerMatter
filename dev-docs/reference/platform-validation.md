# Platform validation record

Snapshot: 2026-10-29. Dates below use Asia/Tokyo.

- Record observed results for an exact candidate; this file does not declare stable support.
- Keep deployment targets, reported native use, automated checks and final-artifact checks distinct.
- A successful CI job covers only its recorded operations and environment. Preserve skipped checks.
- Add evidence for each release candidate; earlier results do not validate a newer binary.
- Use synthetic projects. Keep private projects, account details, logs and captures out of Git.

## Platform coverage

| Platform / architecture | Current target | Evidence available | Still to establish for stable distribution |
| --- | --- | --- | --- |
| macOS / arm64 (Apple Silicon) | macOS 14+ deployment target | Local development and compiler/build checks on macOS 26.5.2; historical native UI checks linked below | Final ad hoc artifact, clean installation and upgrade with actual opening prompts; test the oldest claimed OS or narrow the support claim |
| Windows / x64 | Windows 11 preview | Native use/build coverage reported in the desktop README; automated checks on Windows Server 2022 | Exact Windows 11 version/build and candidate for native UI, IME/AltGr, receiving-app clipboard/drag and unsigned clean installation/upgrade with actual security prompts |
| Ubuntu / amd64 | Ubuntu 24.04 experimental | CI compiler/build checks, container installation and headless startup | Physical desktop or VM GUI session, recorded Wayland/X11 environment, cross-application output; native image clipboard and recoverable project deletion are currently unavailable |
| macOS / x86_64, Windows / ARM64 or x86, other Linux environments | No current distribution target | No validation recorded here | Separate build and validation before advertising support |

- Native baseline and platform limitations: [desktop README](../../desktop-app/README.md#remaining-release-gates).
- Historical native observations: [September validation](../history/validation-2026-09.md); do not apply Swift-era observations to a current Rust build.
- Distribution scope and procedures: [packaging](packaging.md).

## Dated evidence

### 2026-10-29 — unsigned macOS preparation candidate

- Environment: macOS 26.5.2 (25F84), arm64, local development Mac.
- Candidate: 0.11.0 build 15, commit `3b83fc82bcc8fff9520a6e30632f406320c3fc42`, [preparation PR #133](https://github.com/k-onoue/FerMatter/pull/133). This reuses the internal preview version for local validation only; it is not a new published asset.
- Passed: clean-commit `release-check.sh` (255 frontend tests, 174 Rust tests, real LaTeX/Typst/TikZ/CeTZ verification, Clippy, ad hoc app build, dependency audits and publication checks). One compiler-dependent unit test remains covered by the standalone verifier.
- Passed: native synthetic WKWebView LocalStorage read after switching bundle IDs and after rollback; the production alias/conflict behavior has separate Rust tests.
- Passed: `package.sh --stable`; read-only DMG layout/signature inspection, expanded PKG payload equality, no installer scripts, notices and checksums. This does not test installation into `/Applications` or new-account security prompts.
- Passed with the unmodified app copied from that DMG: isolated first setup, four demo projects, `demo-paper` extraction/rendering (27 cards), quit/reopen without repeated setup, and installed LaTeX/Typst SVG/PNG/PDF verification.
- Passed with synthetic preview data: Library variants and settings retained, unsaved Source draft recovered, Source save/reopen and PDF rendering, Instant save and old-version readback, and old/new CLI plus resident launch forwarding in both directions. The old `bf58f94` test wrapper used a unique plist bundle ID to isolate its WebView from real user data; this is not a public-DMG installation/upgrade test.
- DMG SHA-256: `56a9b510ad961cbf51152ebcacc8e4f61e69f64e2c3f41a717de1994b32bfe35`.
- PKG SHA-256: `ed4279189b72498fa7238839a311d3becab6f79fb91adc0289e6169f2d62a496`.
- Local ignored evidence: `desktop-app/build/release-validation/run.1EabwD/`, `desktop-app/build/stable-candidate-3b83fc8/` and `desktop-app/build/mac-install-check/manual-validation.json`. Preserve these reports with the candidate, not in Git.
- Audits: zero entries classified as vulnerabilities by npm/Cargo. Informational warnings remain for unmaintained `proc-macro-error`, `rustybuzz` and `ttf-parser`, and Linux GTK's `glib 0.18.5` [VariantStrIter unsoundness](https://rustsec.org/advisories/RUSTSEC-2024-0429.html). Do not describe this as “no advisories”; retain the reports. Linux stays experimental; a stable Linux claim requires resolving or establishing the affected dependency usage.
- Signature: macOS app ad hoc; DMG/PKG unsigned, not notarized. Native Windows 11, older macOS, clean-account prompts and receiving-app clipboard/drag are still required before stable publication.

### 2026-10-28 — macOS local integration checks

- Environment: macOS 26.5.2, build 25F84; arm64, local development Mac.
- Candidate: app 0.11.0, build 15; combined #131/#132 integration tree, compared equal to merged main [`bf58f94`](https://github.com/k-onoue/FerMatter/commit/bf58f940737764e9b559db531a2afadc390c3326).
- Passed: `check.sh` (255 frontend tests, 171 Rust tests), real LaTeX/Typst/TikZ/CeTZ verifier, Clippy and `build.sh`.
- One ordinary Rust test was ignored because it requires real compilers; its ZIP compiler-default verification was executed by the standalone verifier.
- Evidence: [PR #131 review](https://github.com/k-onoue/FerMatter/pull/131), [PR #132 review](https://github.com/k-onoue/FerMatter/pull/132). Local ignored logs: `desktop-app/build/review-oct28-{check,clippy,build}.log`.
- Signature: Apple Development local build, not Developer ID/notarized. No final installer hash recorded for this check.
- Not performed in this run: native UI, clean installation, final-artifact update, external receiving-app checks or older macOS validation. This was not a complete `release-check.sh` run.

### 2026-10-28 — Windows CI

- Environment: Windows Server 2022 Datacenter 10.0.20348, x64; GitHub runner `windows-2022`, image `20261025.346.1`.
- Candidate: app 0.11.0, PR #132 head `0cec7999cc13df9be130ccbfa64dccea6d13627d`.
- Evidence: [successful package job](https://github.com/k-onoue/FerMatter/actions/runs/40702495495/job/122865732343).
- Passed: rendering-tool installation checks, frontend/Rust/Clippy and real compiler checks, dependency audits, unsigned NSIS build, install/replace/startup/uninstall and installed compiler checks.
- Not established: Windows 11 client compatibility, physical AltGr/IME input, native drag/clipboard into presentation software or stable distribution. This is an unsigned CI artifact, not a new published release.

### 2026-10-28 — Ubuntu CI

- Environment: Ubuntu 24.04.5 LTS, amd64; GitHub runner `ubuntu-24.04`, image `20261012.334.1`. Installation checks use the workflow's separate Ubuntu container.
- Candidate: app 0.11.0, PR #131 head `4177e8f8dcc402e70b5651bf0c5567815687c23b`.
- Evidence: [successful package job](https://github.com/k-onoue/FerMatter/actions/runs/40680688756/job/122790329974).
- Passed: frontend/Rust and real compiler checks, APT repository tests, Debian packaging, disposable signed-APT installation and headless startup.
- Clippy and dependency audits are not steps in this Ubuntu job; do not infer those results from its success.
- The separate `published` job was skipped on this PR; this run does not validate an update from the public APT repository.
- Not established: native desktop session, Wayland/X11 behavior, receiving-app drag/clipboard or stable Linux support.

## Record the next candidate

Copy this table for each environment; use `Passed`, `Failed`, `Skipped` or `Not checked` for each operation. Include the reason for any skip.

| Field | Value to record |
| --- | --- |
| Date / tester | Date with timezone; maintainer or CI run |
| OS | Product, version, build; Linux distribution and desktop/session |
| CPU / environment | Architecture; hardware model if relevant; physical machine, VM, container or CI image |
| App | Version/build, exact source commit; release tag when published |
| Artifact | Installer filename, SHA-256, signature/notarization state |
| Tools / context | Compiler and converter versions, WebView runtime where relevant; Finder/Explorer/terminal launch |
| Automated checks | Commands, results and durable CI/report links; retain reports before CI artifacts expire |
| Native operations | Installation, first setup, Library/Source/Instant, PDF, save/reopen, shortcuts/IME, export/clipboard/drag; receiving app/version |
| Update / removal | Previous version, data preservation, permissions, uninstall result |
| Limitations | Failed, skipped, unavailable and untested operations, with follow-up |

- Capture OS details with `sw_vers` / `uname -m` on macOS, `Get-ComputerInfo` on Windows, and `/etc/os-release` / `uname -m` on Linux. Copy only relevant fields, not host/account identifiers.
- Existing check commands: [testing](../testing.md). Stable release decisions additionally require the [final-artifact checklist](release-validation.md#final-artifact-checklist).
