# Prototype packaging

- Current implementation: [package.sh](../../desktop-app/scripts/package.sh); [recipient notes](../../desktop-app/packaging/PROTOTYPE-README.txt).
- Ad hoc signed app; unsigned, unnotarized containers. Packaging does not create a public release.

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
