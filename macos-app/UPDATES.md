# App updates

The native app checks the public repository's
[latest stable release](https://github.com/k-onoue/mathlidelux/releases/latest)
using the [GitHub Releases API](https://docs.github.com/en/rest/releases/releases#get-the-latest-release).
It makes an unauthenticated request containing no project information. Checks
run when the packaged app opens or becomes active, at most once per 24 hours
(including failed attempts). Settings → Updates can disable these checks.
The app menu's **Check for Updates…** always checks immediately, unless a check
or download is already running. No macOS notification permission is requested.

The in-app banner appears only for a newer stable version during automatic
checks. Offline failures and repositories without a published release stay
quiet; manual checks and Settings show their result. Dismissing a version's
banner hides it for the current app session. A manual check can show it again.

**Download Update** accepts a download. A progress indicator and **Cancel** stay
visible while it runs. When ready, **Open Disk Image** hands the file to macOS;
**Show in Finder** locates it. The user then quits the app and copies the new app
to Applications. The updater does not replace the app or restart it. Files are
saved in a unique `Mathlidelux-<version>-<id>` folder inside Downloads, avoiding
collisions with earlier downloads. The user can delete the disk image after
installation. Download failures leave the release available for retry.
Rechecking the same release keeps the downloaded file ready in the current app
session. A failed check shows its error while keeping Open Disk Image and Show in
Finder available. A newer release, changed asset metadata, or a removed file
offers a new download; existing files in Downloads are never removed by a check.

## Release contract

- The release belongs to `k-onoue/mathlidelux` on GitHub, is not a draft or
  prerelease, and has a [SemVer](https://semver.org/) tag such as `v0.11.0`.
  A leading `v` is optional. Prerelease tags are excluded even if GitHub's flag
  is false. Versions compare numerically; build metadata does not affect order.
- `CFBundleShortVersionString` in the installed app is the version used for
  comparison. Increment it in `scripts/build.sh` for each release. A bare Swift
  executable lacks that bundle version, so it does not check automatically.
- Attach an uploaded, nonempty `Mathlidelux.dmg` to the release. This preserves
  the product site's existing asset name. As a fallback the updater accepts
  `Mathlidelux-<version>-macOS-arm64.dmg`. The current advertised platform is
  Apple Silicon / macOS 26+. Prototype files whose names contain
  `prototype-unnotarized`, PKGs, source archives, and other assets are not offered
  by the download button. Without a matching asset, **View Release** remains
  available.
- Asset URLs must use HTTPS and exactly match that repository, tag and filename.
  Downloads permit redirects only to GitHub's `release-assets.githubusercontent.com`
  CDN or the original asset URL. Disk images are limited to 2 GB and checked for
  the release's exact size and UDIF trailer. If GitHub supplies a `sha256:` digest,
  the file must match it before it is offered for opening.
- The file receives `com.apple.quarantine` before opening, preserving normal
  macOS downloaded-file checks. SHA-256 checks transfer integrity; they do not
  replace Developer ID signing or notarization. Follow the signing and clean-Mac
  validation steps in [PUBLISHING.md](../PUBLISHING.md) before publishing a public
  distribution. No release or signing credentials are bundled in the app.

GitHub's latest endpoint selects one stable release; the app never downgrades
when that release's version is older than the installed app. Release notes open
on GitHub in the default browser.

## Validation

`AppUpdatesTests` covers SemVer precedence and malformed versions, stable-release
selection, asset naming and repository boundaries, redirect restrictions,
disk-image size/trailer/hash verification, opt-out and interval persistence,
manual override, quiet automatic failures, development builds, acceptance,
cancellation, retryable download errors, and completion. A stub URL protocol
checks GitHub headers and success, 404, rate-limit, server-error, and malformed
responses without accessing GitHub.

The public repository does not yet have a usable distribution DMG, so a live
end-to-end download/install is not part of this change's validation. Once a
signed and notarized release exists, test the banner from an older packaged app,
download/cancel/retry, Gatekeeper handling, and manual installation on a clean
Mac before activating the site's public download link.
