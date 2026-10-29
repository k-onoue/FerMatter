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
| Windows / x64 | Windows 11 preview | Windows 11 Pro 23H2 native setup/editing/IME, draft recovery and Impress clipboard checks below; Windows Server 2022 CI | Clean-account installation/update/removal and security prompts, physical AltGr, receiving-app drag and remaining final-artifact checks |
| Ubuntu / amd64 | Ubuntu 24.04 experimental | Physical Ubuntu 24.04.5 / GNOME 46 X11 session through RDP; native setup, saved-data update, Library/Source/Instant, rendering and Impress SVG insertion below; CI packaging | Wayland, IME, remaining recovery/updater/removal cases and receiving-app drag; native image clipboard and recoverable project deletion are currently unavailable |
| macOS / x86_64, Windows / ARM64 or x86, other Linux environments | No current distribution target | No validation recorded here | Separate build and validation before advertising support |

- Native baseline and platform limitations: [desktop README](../../desktop-app/README.md#remaining-release-gates).
- Historical native observations: [September validation](../history/validation-2026-09.md); do not apply Swift-era observations to a current Rust build.
- Distribution scope and procedures: [packaging](packaging.md).

## Dated evidence

### 2026-10-29 — Ubuntu native installation and desktop icons

- Environment: physical x86_64 machine, Ubuntu 24.04.5 LTS, kernel `7.0.0-38-generic`, GNOME 46, xrdp X11 session. Existing account without FerMatter installed before this check. Remote desktop input and screenshots; this is not Wayland or local-display validation. WebKitGTK 4.1 runtime `2.52.6-0ubuntu0.24.04.1`; TeX Live 2023/Debian, latexmk 4.83, Typst 0.15.1 and pdf2svg. RDP emitted graphics-device/DRI3 warnings but rendered the tested UI.
- Installed the public Preview 4 DEB, then replaced it using `apt-get install --reinstall` with the [main CI candidate](https://github.com/k-onoue/FerMatter/actions/runs/40853996414), commit `759eccd14696943f8877e39c5744aa93863b747e`, 0.11.0 build 15. Its app sources match main `e6c2d3e3d00b80e3a784331ea288c7ff3c7999df`. Package version remains `0.11.0~preview.4+ubuntu.1` for local verification; no public package was replaced.
- Public DEB SHA-256: `3e69e7a977f2d5fdcc3beac26ad3fab89a5bfc9b8e16168256dff5298d79760f`. Baseline candidate DEB SHA-256: `ca1c8dbf61eaaee34f94e867f56f00da26a0c2af622a68490525456473730f32`; installed payload SHA-256: `84d307a1829121370b1c60149a723e85d1e2f2424fd27e231602e6494ad0bd51`.

| Native operation | Observed result |
| --- | --- |
| First setup | Passed: project-parent selection, missing-Typst message, explicit compiler path detection, all four demos created |
| Library variants and labels | Passed on Preview: created a named Typst variant and label; all original `.tex`/`.typ` source hashes unchanged |
| Source save | Passed on Preview: Ctrl+S saved a synthetic Typst comment to the real source file |
| Instant and PDF | Passed on Preview: rendered `x^2 + 42`, retained draft/history records, displayed the one-page Typst PDF |
| Replacement and saved data | Passed: backed up data; all 14 saved settings/project/Instant records remained byte-identical during installation; candidate reopened without repeating setup |
| Candidate figure rendering | Passed: native TikZ and CeTZ libraries each displayed three cards; CeTZ document PDF displayed |
| Native menu appearance | Public Preview showed transparency/overlap; current candidate displayed an opaque menu. This fixes the observed appearance, not every menu lifecycle case |

- Baseline candidate automation on this Ubuntu machine passed CLI/MCP/resident export, idempotency/conflict, progress and cancellation checks. Project-free LaTeX align/equation and Typst produced SVG/PNG/PDF with synthetic sources unchanged; TikZ/CeTZ each produced nonempty SVG/PNG/PDF. These are automated exports, not receiving-application or drag verification.
- Found: the DEB supplied only a 1024 px icon, outside this system's hicolor directories. GTK lookup failed at every tested size. [#149](https://github.com/k-onoue/FerMatter/pull/149) supplies standard 32/128/256/512 px icons and adds an installed-desktop-entry/icon-load check to Ubuntu CI.
- Final icon candidate: [Ubuntu CI](https://github.com/k-onoue/FerMatter/actions/runs/40866786002), merge-test commit `d0de81a1f7bed95f6f5e66673e71ea70bb7e0707` (identical tree to PR head `3760c513917be35a303bba7b5964eed0ae1471c3`). DEB SHA-256 `fe2c5805d05c94d80b332df678774fefae0b913314889f6c1bd4ee75fbc68536`; installed payload SHA-256 `fc75a3ab211fda419903c88227b0b7a26149d078b9c01e6a002a17c72a5cc2a2`. Installed through authenticated APT replacement; GTK resolved/loaded the icon at 32/64/128/256/512 px, and GNOME search and the dock displayed the FerMatter artwork.
- Final candidate native checks: launched the installed desktop entry through GIO with the isolated profile, retained all four projects without setup, displayed 27 LaTeX cards and a three-page PDF, and exported a card through the native SVG save dialog. LibreOffice Impress 24.2.7.2 inserted that file and saved it in a synthetic ODP; its embedded SVG equals the exported file byte-for-byte. A direct card-drag attempt did not insert an object; the cause is not isolated, so drag remains unverified. Manual public-feed check succeeded and reported no newer stable version; automatic checking was enabled.
- Final candidate CI passed frontend/Rust tests, script/APT checks, audits, Clippy, real LaTeX/Typst/TikZ/CeTZ checks, DEB build, clean container APT installation and headless startup/icon lookup. Ordinary Rust tests reported 191 passed and one compiler-dependent test ignored; the separate real-compiler check passed. macOS/Windows PR checks and local macOS `check.sh`/`build.sh` also passed. Retain informational audit warnings; the `published` job was skipped and this does not validate the public APT repository.
- Scope: synthetic projects and isolated app data. Closing the windows left a resident running; after confirming saved records and taking backups, those processes were terminated for replacement/restart. Do not count this as a passing normal-Quit or old/new concurrent-launch check. Not checked: clean OS/account, Wayland, physical IME/AltGr, unsaved Source recovery/conflicts, native image clipboard, package removal or signed in-app updater/polkit flows. Direct drag was unsuccessful as described above. Linux remains experimental; [#134](https://github.com/k-onoue/FerMatter/issues/134) remains open.
- Ignored evidence: `desktop-app/build/ubuntu-native-20261029/` (package metadata, checksums, compiler/export reports and native captures), with the corresponding isolated profile and backups retained on the test machine. No tag, release, Homebrew/APT publication or public update feed was changed.

### 2026-10-29 — Windows final main refresh

- Environment: the Windows 11 Pro 23H2 desktop recorded below; existing account, Explorer launch, isolated synthetic profiles.
- Installed candidate: main `759eccd14696943f8877e39c5744aa93863b747e`, 0.11.0 build 15, including automatic checks from [#144](https://github.com/k-onoue/FerMatter/pull/144) and the Windows/Linux Format hint correction in [#143](https://github.com/k-onoue/FerMatter/pull/143). Subsequent main `b55e416` changes only site/documentation files; its app sources are identical.
- Clean `release-check-windows.ps1` passed: 261 frontend tests (including explicit Win32 shortcut tests), 183 ordinary Rust tests, Clippy, real LaTeX/Typst/TikZ/CeTZ checks, audits, NSIS packaging and publication checks. All three normally ignored Rust tests also passed separately from Explorer with PATH restricted to System32; none were skipped in that separate run. Retain informational audit warnings with the reports.
- Installer SHA-256: `77652fea01b65b6630c308fcbfb0f4839cd3c05918363d8328254dab32378108`; installed payload SHA-256: `0b7fd8ca2fec55b6139804fd13590948b4325d5a242e3ff669f67f1ff173bd8a`. Existing-account replacement of the `3baf830` candidate passed; all 28 saved user-data files remained unchanged, with app/data backups retained.
- Native update checks passed: missing preferences enabled an automatic public-feed check; it reported no newer stable version. Turning checks off in the UI still allowed a manual check. After quit/reopen the checkbox stayed off and the preferences file was byte-identical. The normal user's saved opt-out was also preserved.
- The installed EXE rendered project-free LaTeX align/equation and Typst to SVG/PNG/PDF from Explorer with PATH restricted to System32; synthetic sources were unchanged. Three pairs of launches, 100 ms apart, forwarded SVG to pending Instant drafts without losing the resident. The 28 real user-data files remained unchanged during these isolated tests; the normal user profile was restored afterward.
- Evidence: primary checkout `desktop-app/build/windows-updater-68ee2e3/latest-main-*`; clean gate in the validation worktree at `desktop-app/build/release-validation/windows-2a0b8a25851441c0befa810d7d96f9f4/`. [#143 Windows CI](https://github.com/k-onoue/FerMatter/actions/runs/40850518395), macOS and Ubuntu checks passed before its owner-authorized merge.
- Scope: signed local updater installation/failure cases below used the earlier `68ee2e3` source; this refresh changed updater defaults and history navigation, not download verification or installation. Public update delivery remains inactive. The untested native/release cases below still apply; no public asset, tag or feed was replaced.

### 2026-10-29 — Windows updater and import follow-up

- Environment: Windows 11 Pro 23H2, build 22631.4602, x64; normal Explorer launch, Japanese JIS keyboard and Microsoft IME. Existing account; synthetic projects and isolated test profiles.
- Latest-main baseline: `68ee2e37a2faca43ce5c929ba8a7357cde7effd9`, including #136, #137 and #140. Clean release gate passed: 260 frontend tests, 182 Rust tests, Clippy, real LaTeX/Typst/TikZ/CeTZ verification, audits, NSIS packaging and publication checks. The three normally ignored Rust tests also passed separately from Explorer with PATH restricted to System32 (project-free Instant, ZIP compilers and Windows clipboard formats).
- Final candidate: `3baf8306a42062ecb9b3bd5684031eb850e5866e`, 0.11.0 build 15. The only app change from that baseline corrects Format's Windows/Linux shortcut hint to `Shift+Alt+F`, preserving the macOS label. Its full clean release gate also passed; the ordinary Rust run still reports three ignored tests, with their separate baseline execution recorded above. Informational audit warnings remain in the reports.
- Final installer SHA-256: `1ae0e7f90f54460362e7b50cdad5f43f1c4bada5a791a6865ff2f9cc3e883713`; installed payload SHA-256: `c508f93731bc986718941da857c8f2ff9181d5ea170ad60c89b146a4dfc54c10`. Local unsigned candidate; no public artifact, tag or update feed was replaced.
- Existing-account installation passed from the earlier `ae26789` build to the baseline, then to the final candidate. App/data backups were retained; the final installer left all 28 saved user-data files unchanged. The installed final candidate displayed `Format Typst (Shift+Alt+F)`, rendered Instant, and retained the main native menus after Instant closed.
- Baseline native checks passed: project-free LaTeX/Typst Instant; local project-parent selection and all four demos; a valid Typst ZIP produced 20 equation cards and a one-page PDF; a 10,001-entry ZIP was refused before creating a project. Source editing and Ctrl+S persisted a synthetic comment; Ctrl+Tab and Ctrl+Shift+Tab switched source files.
- Imported SVG source remained pending across Instant close/reopen until explicit Render. A recipe requiring an unregistered project's `projectValue` correctly reported its missing definition; editing to a self-contained expression recovered rendering. Single SVG copy/paste into LibreOffice Impress 26.8.1.1 remained an embedded SVG in the saved synthetic ODP.
- The installed baseline's public update check succeeded and reported no newer stable version; the public feed remains inactive. Local updater fixtures used the baseline source with version, app/data identity, test key and loopback transport/provenance overlays; they reused the installed WebView runtime and disabled file-association registration. They exercised the real WebView action API, save protocol, signature/version verification and NSIS handoff, not production HTTPS delivery or publisher credentials.

| Native updater case | Observed result |
| --- | --- |
| Default installation, 0.11.1 → 0.11.2 | Passed: language selection confirmed, installer replaced the payload, then restarted it from the same path |
| Custom installation path containing spaces | Passed: 0.11.1 → 0.11.2 retained that path; matching EXE hash and uninstall registration; no default-path duplicate |
| HTTP 503, stalled feed and retry | Passed: bounded failure, app remained usable and a later valid check succeeded |
| Missing Windows target, downgrade, package over 1 GB | Passed: no inappropriate update offered; invalid size refused before download |
| Changed EXE, malformed signature, signed-version replay | Passed: refused before installation; the replay used a valid 0.11.2 signature with a 0.11.3 feed version |
| Truncated response, download cancellation and retry | Passed: partial bytes rejected; cancellation returned to Download; a fresh valid download reached Ready |
| Save failure with two projects, project Instant and global Instant open | Passed: exclusive lock on synthetic Instant storage cancelled installation and kept editors open |
| Save recovery, installation and restart | Passed: unlock/retry saved the last `x^2 + 42` edit and rendered it after restart; other saved JSON records were unchanged |
| Light/dark theme after update | Passed: native menu colors followed appearance and labels remained present |
| Removal of the custom test app | Passed: application removed and 196 saved test files unchanged |

- Fixture payloads: 0.11.1 SHA-256 `20db908a5bda8f2534f5b54a5e341fd54d7971966d321562a6bf9ea6f9ad2214`; 0.11.2 SHA-256 `f912ca2cd935ee2fc4e76b02abe6732b63883d765177cc40e555cec8ffc91741`. Fixture versions are local test identities, not published FerMatter versions.
- Final installed payload: Explorer launch with PATH restricted to System32 rendered project-free LaTeX align/equation and Typst to nonempty SVG/PNG/PDF, with sources unchanged. Old/new resident forwarding passed in both directions after startup; a new resident also accepted an old launcher after only 100 ms. Three 100-ms current/current launches successfully forwarded SVG into persisted pending Instant drafts. All 28 real user-data files remained unchanged during isolated rapid-launch tests; the normal user profile was restored.
- Observed historical failure: launching the final app 100 ms after an old Preview 4 resident crashed that old resident with `state() called before manage() for Lifecycle`; the final secondary reported that the resident did not accept the launch. This is not a passing compatibility case. The current resident queues startup callbacks and passed the rapid-launch checks above; do not replace published old binaries.
- Ignored local evidence: `desktop-app/build/windows-updater-68ee2e3/` in the primary checkout (artifact metadata, native observations, Explorer helper results, fixture request log and saved ODP); final release gate in the validation worktree at `desktop-app/build/release-validation/windows-8d0029573d52477696d6e63934401b9c/`. Fixture binaries and signing material remain outside Git.
- Not checked: production signed HTTPS update delivery, clean-account installation/security prompts, installer permission denial, an unresponsive-window timeout, physical AltGr, PowerPoint, receiving-app drag, macOS or Linux native operations. Earlier drag automation rejected an endpoint outside the source window; this is not an application failure. These results do not complete [issue #134](https://github.com/k-onoue/FerMatter/issues/134) or authorize release/feed activation.

### 2026-10-29 — Windows 11 native observations

- Environment: Windows 11 Pro 23H2, build 22631.4602, x64; local Lenovo 20L6 desktop session, normal Explorer launch. Japanese JIS keyboard and Microsoft IME; physical AltGr was unavailable.
- Baseline: 0.11.0 build 15, commit `3fc1ef71e1a48d365f3379d73bac7d98cc0a4e04`, [Windows CI run](https://github.com/k-onoue/FerMatter/actions/runs/40747703780); installer SHA-256 `1364bec22293b057cc5e0773c0a550b42044b611f7f6effcd2ea9442580aeedd`. Extracted NSIS payload, isolated synthetic data and projects; this was not a clean-account installation.
- Passed on that baseline: local project-parent setup, four demos created once, invalid Typst path error and automatic rediscovery, LaTeX rendering (27 original cards), three-page PDF, Library variant and label definition without changing original sources. Source checks covered IME composition/commit of `あ`, save, undo/redo, split editor and manual-save draft recovery; project Instant retained two history entries.
- Data checks across Preview 4 rollback and the #138 candidate retained projects, variant, label, preferences, unsaved Source draft and Instant history; newer code read edits written by Preview 4. Old/new resident forwarding passed in both directions on `a40ec69ac12c835f94682364953c71f7aa56fe04`. Source recovery/conflict checks on `e500e4f1ce7a6137eac0eba1a44cea94e3d0b7fa` preserved an external synthetic edit on disk.
- Found and fixed: single SVG copy pasted into Impress as PNG ([#138](https://github.com/k-onoue/FerMatter/pull/138)); closing Instant destroyed the shared Windows menu, causing a white bar or missing menu labels ([#139](https://github.com/k-onoue/FerMatter/pull/139)).
- Final local candidate: `ae2678972b5983cfe549b6e0f98ab67f6e1fd71c`, 0.11.0 build 15; full clean-commit `release-check-windows.ps1` passed (255 frontend tests, 154 Rust tests, Clippy, audits, LaTeX/Typst/TikZ/CeTZ verifier, NSIS and publication checks). Three ordinary Rust tests remain ignored; separate compiler and native output checks are recorded here. Retain audit reports and informational advisories.
- Final installer SHA-256: `adb403fbe827d4c224c857a4db30efc8737800cd509c25e6201e824d459c8154`; installed payload SHA-256: `27381350350dc2fd263a8de40299367f9e1cd5262cdc595b1632b2019ac28163`. Unsigned local candidate; no public asset or tag was replaced.
- Passed on the installed payload: project Instant render/copy, close without losing the main File/Edit/Window/Help menus, subsequent light/dark switching, and single SVG paste into LibreOffice Impress 26.8.1.1 retained as SVG in saved ODP. The earlier #138 payload (`a40ec69ac12c835f94682364953c71f7aa56fe04`) also verified explicit PNG paste; baseline two-card paste retained separate SVGs. Project-free Instant rendered LaTeX, and File → New Instant remained usable after closing another Instant on the candidate's development build.
- Passed: existing-account installer updates from Preview 4 via `0acb6b887f0d201e222de0d51da07aa2f0e30df7` to the final candidate; all 27 saved user-data files were unchanged by installation, with prior app/data backed up locally. The final installed EXE, launched from the Explorer desktop with PATH restricted to System32, rendered project-free LaTeX align/equation and Typst to nonempty SVG/PNG/PDF and preserved synthetic sources.
- Passed on the final installed payload: Preview 4 → final and final → Preview 4 resident forwarding, each repeated twice with isolated fresh profiles from Explorer; second processes exited successfully and resident processes survived. All 27 saved user-data files remained unchanged; the normal user profile was restored afterward. Evidence: `final-resident-result.json` below.
- Local ignored evidence: `desktop-app/build/windows-native-3fc1ef7-f23c9eb2a18b4a24be824ab109e9ce8c/`, including `detach-candidate/`, installed verifier results, native checks and saved ODP inspection; release gate `desktop-app/build/release-validation/windows-6e5753c7f5f74f28be4510489eb32c11/`.
- Not checked: clean-account installation/update/removal and actual SmartScreen/Smart App Control prompts, physical AltGr, PowerPoint, and other platforms. Cross-app drag was not performed because automation rejected an endpoint outside the source window; this is not a receiving-app failure. These observations do not complete [issue #134](https://github.com/k-onoue/FerMatter/issues/134) or authorize stable publication.

### 2026-10-29 — unsigned macOS preparation candidate

- Environment: macOS 26.5.2 (25F84), arm64, local development Mac.
- Candidate: 0.11.0 build 15, commit `3b83fc82bcc8fff9520a6e30632f406320c3fc42`, [preparation PR #133](https://github.com/k-onoue/FerMatter/pull/133). This reuses the internal preview version for local validation only; it is not a new published asset.
- Passed: clean-commit `release-check.sh` (255 frontend tests, 174 Rust tests, real LaTeX/Typst/TikZ/CeTZ verification, Clippy, ad hoc app build, dependency audits and publication checks). One compiler-dependent unit test remains covered by the standalone verifier.
- Passed: native synthetic WKWebView LocalStorage read after switching bundle IDs and after rollback; the production alias/conflict behavior has separate Rust tests.
- Passed: `package.sh --stable`; read-only DMG layout/signature inspection, expanded PKG payload equality, no installer scripts, notices and checksums. This does not test installation into `/Applications` or new-account security prompts.
- Passed with the unmodified app copied from that DMG: isolated first setup, four demo projects, `demo-paper` extraction/rendering (27 cards), quit/reopen without repeated setup, and installed LaTeX/Typst SVG/PNG/PDF verification.
- Passed with synthetic preview data: Library variants and settings retained, unsaved Source draft recovered, Source save/reopen and PDF rendering, Instant save and old-version readback, and old/new CLI plus resident launch forwarding in both directions. The old `bf58f94` test wrapper used a unique plist bundle ID to isolate its WebView from real user data; this is not a public-DMG installation/upgrade test.
- Passed: single and two-card SVG copy/paste into PowerPoint 16.114.1 (16.114.26102031). The saved synthetic PPTX retains three separate picture objects referencing embedded SVGs. Native drag remains unverified.
- DMG SHA-256: `56a9b510ad961cbf51152ebcacc8e4f61e69f64e2c3f41a717de1994b32bfe35`.
- PKG SHA-256: `ed4279189b72498fa7238839a311d3becab6f79fb91adc0289e6169f2d62a496`.
- Local ignored evidence: `desktop-app/build/release-validation/run.1EabwD/`, `desktop-app/build/stable-candidate-3b83fc8/` and `desktop-app/build/mac-install-check/manual-validation.json`. Preserve these reports with the candidate, not in Git.
- Audits: zero entries classified as vulnerabilities by npm/Cargo. Informational warnings remain for unmaintained `proc-macro-error`, `rustybuzz` and `ttf-parser`, and Linux GTK's `glib 0.18.5` [VariantStrIter unsoundness](https://rustsec.org/advisories/RUSTSEC-2024-0429.html). Do not describe this as “no advisories”; retain the reports. Linux stays experimental; a stable Linux claim requires resolving or establishing the affected dependency usage.
- Signature: macOS app ad hoc; DMG/PKG unsigned, not notarized. Native Windows 11, older macOS, clean-account prompts and receiving-app drag remain required before stable publication. Track remaining native checks in [issue #134](https://github.com/k-onoue/FerMatter/issues/134).

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
