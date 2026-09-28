# Prototype packaging

- Current implementation: [package.sh](../../desktop-app/scripts/package.sh); [recipient notes](../../desktop-app/packaging/PROTOTYPE-README.txt).
- Ad hoc signed app; unsigned, unnotarized containers. Packaging does not create a public release.

## Build and inspect

```sh
./desktop-app/scripts/package.sh
./desktop-app/scripts/package.sh --app "/path/to/Mathlidelux.app"
```

- Default app: `desktop-app/src-tauri/target/release/bundle/macos/Mathlidelux.app`.
- Default output: ignored `desktop-app/build/distribution/`.
- DMG/PKG/SHA-256 names include version, build, architecture and `prototype-unnotarized`.
- Input: ad hoc signed arm64 app with identity `local.slidemath.prototype`, LICENSE and NOTICE; production signatures are rejected.
- Version/build/minimum macOS come from the bundle; packaging does not edit the signed input.
- Existing outputs are preserved unless `--overwrite` is supplied.
- DMG: app, Applications shortcut, README, LICENSE, NOTICE.
- PKG: only `/Applications/Mathlidelux.app`; no install scripts, services, dependencies or automatic launch.
- TeX/Typst/converters remain external. Python and development SDKs are not required to run the app.
- User projects/support data are outside the payload and survive replacement/removal.
- Verification checks signatures, a read-only mounted DMG, expanded PKG contents, disabled relocation, absence of installer scripts and hashes.
- Inspection does not install the package or establish clean-Mac compatibility. Archives are not promised byte-for-byte reproducible.

## Public release boundary

- Keep prototype artifacts separate from the public `Mathlidelux.dmg` asset; downloads remain disabled on the site.
- Developer ID signing, notarization and clean-Mac installation/upgrade validation remain release work.
- Keep Gatekeeper enabled; packaging must not remove quarantine or change security settings.
- [Release checklist](../releasing.md), [Apple Developer ID](https://developer.apple.com/developer-id/), [Apple app-opening guidance](https://support.apple.com/102445).
