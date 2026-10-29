# In-app updates

## User behavior

- **Settings → Updates** and the compact global update icon share one native state across Library, Source and Instant windows.
- Manual check; optional automatic check at most once per 24 hours, including failures. Automatic checking starts **off**. Failed background checks do not open a dialog.
- A colored indicator announces an available update. Download and **Install and restart** are separate explicit actions; checking/downloading can be cancelled and retried.
- Download verification completes before installation. All open windows are frozen and flushed through the existing persistence protocol; a failed save or 30-second timeout cancels installation. Admission of new windows/automation remains closed until installation succeeds or fails.
- macOS replaces the application bundle; Windows hands off to the NSIS installer; Ubuntu installs the DEB through `/usr/bin/pkexec /usr/bin/dpkg -i`. Ubuntu requires a graphical polkit agent. Cancelling authentication does not fall through to a password form or terminal sudo.
- Unix relaunch waits for the previous resident to exit before claiming its lock. Existing projects, settings, drafts, app identity and legacy storage locations remain unchanged.
- Homebrew's receipt is not modified by in-app replacement. Users who want Homebrew to track installation should use `brew upgrade --cask fermatter/tap/fermatter`. DEB installation updates dpkg's package database; APT remains usable.
- The currently published previews have no updater: their first update to an updater-enabled release is manual.

## Network and trust boundary

| Item | Contract |
| --- | --- |
| Feed | Fixed HTTPS `https://fermatter.github.io/updates/stable.json`; no account, token, document, settings or usage upload |
| Channels | Stable SemVer greater than the installed app version; no previews, downgrades or build-metadata updates |
| Targets | `darwin-aarch64-app`, `windows-x86_64-nsis`, `linux-x86_64-deb`; missing target means no offered update |
| Source | Exact `FerMatter/fermatter.github.io` release path and version; GitHub release-assets CDN redirects only |
| Bounds | Feed 256 KiB / 30 seconds; package 1 GB maximum, exact declared size and SHA-256, connection/read/total timeouts |
| Authentication | Embedded Minisign public key; artifact **and signed version** verified by Tauri before installation |
| Frontend | No permission to supply update URLs, keys, installer paths or arguments; only the native action API |
| Cancellation | Drop the network future; operation generations reject late progress/results; retry begins a fresh download |
| Settings | `UpdatePreferences.json` in the existing app-data directory; corrupt files remain untouched and disable automatic checks |

Update signatures are free publisher authenticity checks. They are distinct from Developer ID, notarization and Authenticode; the existing unsigned/ad hoc distribution policy still applies.

## Prepare update artifacts

1. Complete the [release gate](release-validation.md) and native final-artifact checks on the exact candidate. Choose a **new** stable version in Cargo, npm and Tauri, and increment the macOS build. All historical previews show `0.11.0`; do not republish changed bytes under that version.
2. Preserve the private key outside Git. The local maintainer key is stored at `$HOME/.config/fermatter/signing/updater.key` with owner-only permissions; retain a separate secure backup. Never regenerate or overwrite the key for ordinary releases. Only its public key is embedded in `tauri.conf.json`.
3. Supply `TAURI_SIGNING_PRIVATE_KEY` (path or contents) and, if used, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` through the release environment. Add `--config src-tauri/tauri.updater.conf.json` to the normal Tauri bundle command. Ordinary local/CI builds do not require the secret or produce update artifacts.
4. Retain the `.app.tar.gz`, NSIS `.exe` and DEB `.deb` plus their `.sig` files. If signing an existing, already validated artifact, use `npm --prefix desktop-app run tauri -- signer sign --app-version X.Y.Z FILE`. The signed version is mandatory. Check final package contents/version against the validation record.
5. Prepare a local manifest; each `--asset` is optional except that at least one validated target is required. This verifies file signatures, signed version, sizes and hashes and **does not publish**:

```sh
node scripts/prepare-update.mjs \
  --version X.Y.Z --date YYYY-MM-DDT00:00:00Z \
  --notes-file RELEASE_NOTES.txt \
  --asset darwin-aarch64-app=/path/to/FerMatter.app.tar.gz \
  --asset windows-x86_64-nsis=/path/to/FerMatter-setup.exe \
  --asset linux-x86_64-deb=/path/to/fermatter_amd64.deb \
  --output desktop-app/build/stable-update.json
```

6. During authorized release publication, upload immutable artifacts to the **stable** `vX.Y.Z` release. Download anonymously and rerun preparation against those bytes/signatures. Test the update from an older packaged app on each offered OS/CPU, including data preservation, authentication cancellation and restart.
7. Only then replace `site/public/updates/stable.json` through a reviewed PR. Keep its release/version consistent with Download, Homebrew and APT. Website CI validates the feed's shape, targets and provenance. Missing platforms remain unavailable rather than borrowing another platform's artifact.

The committed `0.0.0` / empty-platform manifest deliberately offers **no updates**. Implementing this feature is not authorization to publish a release or activate the feed. Track native activation evidence in [issue #134](https://github.com/k-onoue/FerMatter/issues/134).

## Verification

- Unit tests cover asset provenance/size, automatic-check throttling, corrupt preferences, multi-window save completion/failure/timeout, UI actions and stale state.
- Manifest tooling verifies an independent Tauri CLI signature fixture, corrupt bytes, signed-version replay and tampered comments.
- Before activation: actual old→new packaged update on macOS/Windows/Ubuntu, changed/unsigned payload refusal, offline/truncated/oversize downloads, cancellation/retry, insufficient permissions, failed saves, global Instant plus multiple projects, process restart and retained data. Record every unverified platform explicitly.
- Fix history is Markdown in [resolved issues](../../user-docs/resolved-issues.md). Move an item from “awaiting a download” only after that platform's artifact is published.

## References

- [Tauri updater](https://v2.tauri.app/plugin/updater/).
- [Tauri updater 2.14.1 source](https://docs.rs/tauri-plugin-updater/2.14.1/tauri_plugin_updater/).
- [Minisign signature format](https://jedisct1.github.io/minisign/).
- [Release checklist](../releasing.md).
