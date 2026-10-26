# Packaging

- Current implementation: [package.sh](../../desktop-app/scripts/package.sh); [recipient notes](../../desktop-app/packaging/PROTOTYPE-README.txt).
- Ad hoc signed app; unsigned, unnotarized containers. Packaging does not create a public release.

## Windows preview

- Scope: Windows 11 x64, current-user NSIS installer. No ARM64 or 32-bit package. Windows installer metadata: [windows.json](../../desktop-app/packaging/windows.json).
- [build-windows.ps1](../../desktop-app/scripts/build-windows.ps1) uses Tauri's Windows configuration, includes the offline WebView2 installer, then calls [package-windows.ps1](../../desktop-app/scripts/package-windows.ps1).
- Output: ignored `desktop-app/build/windows/`; an unsigned setup EXE, `SHA256SUMS.txt` and `build-info.json`. LICENSE, NOTICE, theme attribution and generated dependency notices stay in the installed payload.
- TeX/Typst and PDF conversion tools remain external. App users do not need Rust, Node.js or Python.
- Preserve bundle identity, legacy `Slide Math` data names and user-selected project folders. Installation, upgrades and removal must not delete project files, drafts or settings.

```powershell
./desktop-app/scripts/check-windows.ps1
./desktop-app/scripts/audit-windows.ps1
./desktop-app/scripts/build-windows.ps1
Get-FileHash ./desktop-app/build/windows/*setup.exe -Algorithm SHA256
./desktop-app/scripts/verify-windows-install.ps1 -Installer ./desktop-app/build/windows/FerMatter_0.11.0_windows_x64_preview.3_unsigned_setup.exe -CheckCompilers
```

- Run installer verification on a disposable Windows account or CI machine without an existing FerMatter installation. `-CheckCompilers` tests the installed EXE with a minimal Windows system PATH: LaTeX align/equation and Typst must produce nonempty SVG/PNG/PDF files without changing the synthetic sources. The script also checks replacement, notices, startup, uninstall and data preservation; it does not establish native UI or clean-machine compatibility.
- Preview tag: `v0.11.0-windows-preview.3`; filename: `FerMatter_0.11.0_windows_x64_preview.3_unsigned_setup.exe`. Increment the preview revision for changed published bytes; never replace an existing asset.
- Publish reviewed binaries to a **prerelease** in `FerMatter/fermatter.github.io`, with the checksum manifest and installation/validation notes. Windows assets have independent site fields; existing macOS downloads stay pinned.
- Before setting `windowsReleaseAvailable: true`, verify anonymous download/checksum and install/launch the exact downloaded installer. Record the tested Windows version and native/clean-machine limitations.
- Record signing state and any OS first-launch prompts with release metadata and maintainer validation. Product-facing instructions follow the concise [website presentation rules](../documentation.md#publish). The installer must keep OS security features enabled.
- Authenticode signing and broader clean-machine/upgrade validation remain requirements for a stable Windows release; macOS notarization is a separate process.

## Experimental Ubuntu

- Scope: Ubuntu 24.04, amd64, `.deb` only. No Snap/Flatpak service, root installer scripts or bundled TeX/Typst toolchains.
- [Ubuntu workflow](../../.github/workflows/ubuntu.yml): builds with Tauri, runs frontend/Rust tests and all four real compiler paths, installs through signed APT in a clean Ubuntu container, and checks headless startup with an isolated data directory. The public check also upgrades from the previous retained APT version and preserves synthetic project/draft files. Physical desktops, Wayland, native clipboard and cross-application drag are not established by these checks.
- Package name/executable: `fermatter`; suite: `experimental`; metadata/version: [ubuntu.json](../../desktop-app/packaging/ubuntu.json). Increment the Debian version for every changed published binary; never replace a published version. Bundle identity and legacy data/project paths remain unchanged.
- Rendering dependencies remain optional external tools. Native image clipboard and recoverable project deletion are not available on Linux; use file export for images.
- [APT builder](../../scripts/apt-repository.py) preserves old package versions and immutable by-hash indexes. Release metadata expires after 365 days; refresh it before expiry even if the app has not changed.
- Public packages and `apt-repository.tar.gz` live in the website repository's `ubuntu-apt` prerelease. Pages uses the [staging helper](../../scripts/stage-apt-repository.py) to verify signatures, checksums, allowed package names and archive paths before deploying `/apt/`; binaries stay out of Git.

### Signing and publication

- Maintainer requirements: Python 3.11+, GnuPG and authenticated `gh` with access to source Actions artifacts and public website Releases/Actions/variables.
- Private key: `${XDG_DATA_HOME:-$HOME/.local/share}/fermatter/apt-signing`, mode 0700; keep an encrypted offline backup. Never commit or upload the private key, revocation certificate or signing home. The build workflow does not receive it.
- Public key: [fermatter-archive-keyring.asc](../../desktop-app/packaging/fermatter-archive-keyring.asc); fingerprint `8FB5E7A5ECB5E99601CD9C1BD049D613DE9DE3AF`, expires 2028-10-24. A rotation requires reviewed changes to the configuration, website workflow's pinned fingerprint and public key. Existing users must install the replacement scoped key before relying on it.

```sh
# Check out clean current main and run/wait for its Ubuntu workflow.
gh workflow run ubuntu.yml --ref main
gh run list --workflow ubuntu.yml --branch main
python3 scripts/publish-ubuntu.py SUCCESSFUL_RUN_ID desktop-app/build/apt-publication-NEW \
  --signing-home "${XDG_DATA_HOME:-$HOME/.local/share}/fermatter/apt-signing"
gh workflow run ubuntu.yml --ref main -f verify-published=true
```

- Publication checks the successful workflow/source commit, artifact checksum and Debian control metadata. It rejects changed bytes for an already published version, uploads the signed snapshot, sets `FERMATTER_APT_AVAILABLE=true` on the website repository and requests Pages deployment. Wait for Pages before running the public installation check.
- Keep `site/src/data/site.json` → `ubuntuAptAvailable` false until the public clean-container installation check passes. Then enable commands through a reviewed PR. Users register the scoped APT source once; after that `sudo apt install fermatter` installs/upgrades the package.
- APT signing and metadata verification do not prove full desktop compatibility. Keep this distribution explicitly experimental.

```sh
python3 -m unittest discover -s scripts/tests -p 'test_apt_repository.py' -v
# Linux with Docker; use a disposable Ubuntu host for the startup check.
bash desktop-app/scripts/verify-ubuntu-install.sh /absolute/path/to/fermatter_VERSION_amd64.deb
bash desktop-app/scripts/verify-ubuntu-install.sh --published
```

## Public preview

```sh
./desktop-app/scripts/package.sh --preview --app /path/to/ad-hoc/FerMatter.app
```

- Explicit owner-approved preview path; requires an existing ad hoc signed bundle. Normal local signing remains unchanged.
- Names contain `preview-unnotarized`; the DMG/PKG include [preview installation notes](../../desktop-app/packaging/PREVIEW-README.txt).
- Same archive/signature checks as prototype packaging; no Developer ID signing, notarization, Gatekeeper bypass, installation or publication.
- Publish as a GitHub **prerelease**, with anonymous download/checksum verification and the [preview checklist](../releasing.md#unnotarized-public-preview).

## Build and inspect

```sh
./desktop-app/scripts/package.sh
./desktop-app/scripts/package.sh --app "/path/to/FerMatter.app"
```

- Default app: `desktop-app/src-tauri/target/release/bundle/macos/FerMatter.app`.
- Default output: ignored `desktop-app/build/distribution/`.
- DMG/PKG/SHA-256 names include version, build, architecture and `prototype-unnotarized`.
- Input: ad hoc signed arm64 app with identity `local.slidemath.prototype`, LICENSE and NOTICE; production signatures are rejected.
- If [local certificate signing](signing.md) is configured, use `APPLE_SIGNING_IDENTITY=- ./desktop-app/scripts/package.sh` to build an ad hoc prototype. Switching the installed build's signing identity may request folder access again.
- Version/build/minimum macOS come from the bundle; packaging does not edit the signed input.
- Existing outputs are preserved unless `--overwrite` is supplied.
- DMG: fixed app → Applications layout with a Retina installation arrow. README, LICENSE and NOTICE stay in the image as hidden files; LICENSE and NOTICE are also bundled with the app.
- Packaging uses Python 3.9+ with hash-pinned `dmgbuild` tools in ignored `desktop-app/build/dmg-tools/`; the app does not need or bundle Python. No Finder automation permission is required to create the layout.
- PKG: only `/Applications/FerMatter.app`; no install scripts, services, dependencies or automatic launch.
- Both packages require a project-folder choice when the user first opens the app. Demos and future projects use that per-user location; the installer never writes personal project data as root.
- TeX/Typst/converters remain external. Python and development SDKs are not required to run the app.
- User projects/support data are outside the payload and survive replacement/removal.
- Verification checks signatures, Finder icon positions/background in the read-only mounted DMG, expanded PKG contents, disabled relocation, absence of installer scripts and hashes.
- Inspection does not install the package or establish clean-Mac compatibility. Archives are not promised byte-for-byte reproducible.

## Public release boundary

- Keep prototype artifacts separate from the public `FerMatter.dmg` asset; only an explicitly labeled preview may be offered without notarization.
- Developer ID signing, notarization and clean-Mac installation/upgrade validation remain release work.
- Keep Gatekeeper enabled; packaging must not remove quarantine or change security settings.
- [Release checklist](../releasing.md), [Apple Developer ID](https://developer.apple.com/developer-id/), [Apple app-opening guidance](https://support.apple.com/102445).

## Public preparation path

```sh
APPLE_SIGNING_IDENTITY='Developer ID Application: NAME (TEAM)' ./desktop-app/scripts/release-check.sh
./desktop-app/scripts/package.sh --public \
  --app desktop-app/src-tauri/target/release/bundle/macos/FerMatter.app \
  --validation desktop-app/build/release-validation/run.EXAMPLE/report.json \
  --signing-identity 'Developer ID Application: NAME (TEAM)' \
  --installer-identity 'Developer ID Installer: NAME (TEAM)' \
  --notary-profile EXISTING_KEYCHAIN_PROFILE
```

- Requires existing Developer ID Application/Installer certificates and a `notarytool` Keychain profile. Never store private keys, passwords, tokens or certificate exports in the repository.
- Validates the clean tested commit and hashes of every input bundle file. Dirty rehearsals, skipped gates or changed bundles are rejected.
- Checks Developer ID, hardened runtime and secure timestamp before submission; ad hoc and Apple Development builds are rejected.
- Submits a staged app ZIP to Apple, requires `Accepted`, staples the staged app and checks Gatekeeper.
- Signs DMG/PKG; submits both to Apple, requires `Accepted`, staples/validates tickets and checks Gatekeeper again.
- Preserves the input app. Output names use `notarized`, and include SHA-256, validation report and notarization receipts. No installation, tag, GitHub release, website change or publication occurs.
- Dependency license texts are included in `Contents/Resources/licenses/`, in addition to LICENSE, NOTICE and theme attribution.
- These commands **submit binaries to Apple**; they are the explicit public-preparation path, not part of normal checks/prototype packaging.
- Signing/notarization acceptance does not prove clean-Mac installation or upgrade behavior. Complete [final artifact validation](release-validation.md) before enabling downloads.
- This path has not yet been exercised with production credentials at the documentation baseline; [issue #42](https://github.com/k-onoue/FerMatter/issues/42) tracks the remaining checks.
- References: [Apple notarization](https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution), [Tauri signing](https://v2.tauri.app/distribute/sign/macos/).
