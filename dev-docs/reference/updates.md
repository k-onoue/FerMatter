# Updates — future integration contract

- The current desktop app has no in-app release checker/downloader; install newer builds manually.
- The Swift implementation was retired. Its [historical specification](https://github.com/k-onoue/mathlidelux/blob/629ec5445e09880e2aa1796603157f5bd73d8ab5/macos-app/UPDATES.md) remains a reference, not a current feature.
- The requirements below describe a future implementation; do not enable it against unnotarized prototype artifacts.
- Repository was private at inspection; unauthenticated public release access must be verified before release activation.

## Proposed user behavior

- Automatic check at most once per 24 hours, including failures; Settings can disable it.
- Manual check bypasses the interval; automatic failures stay quiet.
- Request contains no project information; no GitHub token shipped in the app.
- Download starts after acceptance; cancellation/retry available.
- Open Disk Image/Show in Finder hand off to macOS; installation is manual.
- Rechecking the same release retains downloaded-file actions; checks do not delete earlier downloads.

## Release contract

- Repository: `k-onoue/mathlidelux`; published stable release; valid SemVer tag with optional leading `v`.
- Exclude prerelease tags even when GitHub's prerelease flag is false.
- Compare `CFBundleShortVersionString`; build metadata does not trigger an update.
- Preferred asset: `Mathlidelux.dmg`; fallback: `Mathlidelux-<version>-macOS-arm64.dmg`.
- Reject PKG, source archives, and `prototype-unnotarized` artifacts as update downloads.
- Exact HTTPS repository/tag/filename; redirects restricted to the original URL or GitHub's release-assets CDN.
- Verify declared size, 2,000,000,000-byte cap, UDIF trailer, and SHA-256 digest when supplied.
- Preserve quarantine; checksum validation does not replace signing/notarization.
- Before public activation: test from an older packaged app, clean-Mac installation, failures, cancel, and retry.

## References

- [GitHub latest-release API](https://docs.github.com/en/rest/releases/releases#get-the-latest-release).
- [SemVer](https://semver.org/lang/ja/).
- [Release checklist](../releasing.md).
