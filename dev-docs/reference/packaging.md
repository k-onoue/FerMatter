# Prototype packaging — integration reference

- Available in the current source tree; public distribution remains a separate release step.
- Details: [distribution specification](../../macos-app/DISTRIBUTION.md), [recipient notes](../../macos-app/packaging/PROTOTYPE-README.txt).

## Build and inspect

```sh
./macos-app/scripts/package.sh
./macos-app/scripts/package.sh --app "/path/to/Mathlidelux.app"
```

- Default output: ignored `macos-app/build/distribution/`.
- DMG, PKG, SHA-256 filenames include version, build, architecture, and `prototype-unnotarized`.
- Input: ad hoc signed arm64 prototype app with LICENSE/NOTICE; production-signed inputs are rejected.
- Existing outputs preserved unless `--overwrite` is supplied.
- DMG: app, Applications shortcut, README, LICENSE, NOTICE.
- PKG: only `/Applications/Mathlidelux.app`; no install scripts, services, runtime installers, or automatic launch.
- Python/TeX/Typst/converters are external; source folders and user data are not installer payload.
- Packaging verifies signatures, DMG contents/checksums, expanded PKG contents, and hashes without installing.
- Inspection is not a clean-Mac launch/install test; archives are not promised byte-for-byte reproducible.

## Public release boundary

- Do not rename a prototype artifact to `Mathlidelux.dmg` or activate website downloads for it.
- Developer ID signing, notarization, dependency strategy, and clean-Mac validation remain separate work.
- Preserve normal Gatekeeper checks; do not distribute instructions to disable them.
- References: [release checklist](../releasing.md), [Apple Developer ID](https://developer.apple.com/developer-id/), [Apple app-opening guidance](https://support.apple.com/102445).
