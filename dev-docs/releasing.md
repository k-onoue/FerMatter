# Releasing

## Current boundary

- Development prototype: Apple Silicon; macOS 14+ deployment target, verified on macOS 26. Only compiler tools remain external; no Python runtime.
- Public previews may be distributed with ad hoc signing and an explicit unnotarized label, as requested by the owner. Stable downloads still require Developer ID signing and notarization.
- [Packaging](reference/packaging.md) keeps ad hoc prototypes separate from an explicit Developer ID/notarization path. Production credentials and clean-Mac final-artifact validation are still required; no public release is implied. In-app update transport is not yet implemented.
- Repository is private as of 2026-09-28; Pages is public. Test release access without authentication before advertising downloads.
- Keep stable downloads disabled until all public-release checks pass. A preview download must identify its prerelease tag and unnotarized status; verify anonymous access and its checksum before enabling it.

## Unnotarized public preview

- First preview: app version `0.11.0`, build `12`, prerelease tag `v0.11.0-preview.1`.
- Current preview: app version `0.11.0`, build `15`, prerelease tag `v0.11.0-preview.4`; compact and consistent workspace controls, updated guides, and experimental Ubuntu APT distribution.
- Source stays in `k-onoue/FerMatter`; publish reviewed binaries and notices in `FerMatter/fermatter.github.io` Releases. Repository visibility is managed separately from distribution.
- Run app checks, clippy, build, dependency audits and publication checks on the committed candidate. Record skipped native/clean-Mac checks honestly; a preview is not a stable-release validation pass.
- Preserve the development-signed local bundle. Copy it to an ignored staging directory, sign that copy ad hoc, then run `package.sh --preview --app PATH`.
- Create a draft **prerelease** with the DMG and SHA-256 manifest. Inspect the payload and bundled notices before publication. Keep published assets fixed.
- The tap is `FerMatter/homebrew-tap`; the cask pins the version/build, URL and SHA-256. It installs only `FerMatter.app`, keeps quarantine enabled, and does not delete user data.
- Verify anonymous DMG download, checksum, and a Homebrew install into an isolated `--appdir`; then enable the website download and Homebrew command in a separate reviewed site PR.
- First launch may require **System Settings → Privacy & Security → Open Anyway**; link to [Apple's instructions](https://support.apple.com/102445). Never remove quarantine or disable Gatekeeper in the installer/cask.
- Developer ID/public-mode packaging and the clean-Mac stable-release checklist remain separate requirements.

## Versioning

- Format: `MAJOR.MINOR.PATCH`; tag: `vMAJOR.MINOR.PATCH`.
- **Project policy during 0.x:** patch for compatible fixes; minor for features or incompatible changes.
- Describe incompatible changes and migration steps explicitly; `0.x` is not permission to silently discard data.
- At 1.0+: major for breaking public contracts, minor for compatible features/deprecations, patch for compatible fixes.
- Compatibility surface: saved projects/drafts/templates, documented settings schemas/command IDs, and documented import/export behavior.
- Internal Rust/TypeScript interfaces and cosmetic layout are not a promised external API.
- Preserve bundle ID/support paths unless a tested migration accompanies the change.
- Prerelease example: `0.11.0-rc.1`; do not offer it through stable update/download links.
- Set the version in `desktop-app/src-tauri/tauri.conf.json`; keep Cargo/npm versions consistent. Increment `bundle.macOS.bundleVersion` for each distributable build.
- Keep site version and release notes consistent; a future updater must compare semantic versions, not only build metadata.
- Reference: [SemVer](https://semver.org/lang/ja/); its 0.x rules allow instability, while the stricter rules above are this project's policy.

## Changelog

- Maintain [CHANGELOG.md](../CHANGELOG.md) in the change's PR.
- Keep unreleased changes at the top; group under Added, Changed, Deprecated, Removed, Fixed, Security as needed.
- Write user impact, requirements, and migration notes; omit empty groups and raw commit lists.
- On release: replace the section with version + `YYYY-MM-DD`; add a comparison/tag link; start a new Unreleased section.
- Use that section as the source for GitHub release notes; add installation notes and known limitations.
- Do not invent historical releases from prototype version labels.
- Reference: [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/).

## Prepare through a PR

1. Choose version/scope; use a task branch, optionally `release/vX.Y.Z`.
2. Finish [issue #7](https://github.com/k-onoue/FerMatter/issues/7)'s verified release-blocking findings; document deferrals.
3. Update versions, changelog, requirements, compatibility notes, and affected guides.
4. Run the [automated release gate and final artifact checklist](reference/release-validation.md); verify real compilers and synthetic workflows.
5. Review/merge the preparation PR; retest the exact resulting commit before tagging.

## Public distribution checklist

- Decide and document dependency bundling/setup for a clean supported Mac.
- Sign app/bundled executables with Developer ID; use appropriate hardened runtime; notarize and staple.
- Include LICENSE, NOTICE, and notices for bundled third-party components.
- Verify installation, launch, compilers, exports, and upgrading from an older build on a clean Mac.
- Verify manual app replacement preserves data. Before offering in-app updates, implement and test the [update contract](reference/updates.md).
- Prepare `FerMatter.dmg`; do not reuse the [unnotarized prototype artifacts](reference/packaging.md).
- Verify anonymous access to the release asset; never embed a repository credential in the app/site.
- Reference: [Apple Developer ID distribution](https://developer.apple.com/developer-id/).

## Tag and publish

- Create an **annotated tag** on the exact tested commit in `main`; verify the commit before pushing.
- Example placeholders below must be replaced with the chosen version and tested commit:

```sh
git tag -a vX.Y.Z TESTED_COMMIT_SHA -m "FerMatter X.Y.Z"
git show vX.Y.Z
git push origin vX.Y.Z
```

- Keep published tags and binaries fixed; release a new version for a code/binary correction.
- Create a draft GitHub Release from the tag; attach tested artifacts and curated notes; inspect before publication.
- Publish as stable only when the release is stable; keep previews marked prerelease.
- After anonymous download/install checks pass, use a separate reviewed site change to enable downloads and set the version.
- Website asset name: `FerMatter.dmg`; the proposed [update contract](reference/updates.md) defines a versioned fallback.
- References: [Git tags](https://git-scm.com/book/ja/v2/Git-%E3%81%AE%E5%9F%BA%E6%9C%AC-%E3%82%BF%E3%82%B0), [GitHub Releases](https://docs.github.com/ja/repositories/releasing-projects-on-github/about-releases), [release asset links](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases).

## Publish the website

- Source: GitHub Actions; `.github/workflows/pages.yml` builds and deploys `site/dist/`.
- Guide Markdown and Astro sources are reviewed; generated HTML is not committed.
- Developer notes and repository contents are not copied into the site.
- One-time migration setting, counters, and preview: [documentation](documentation.md).
- Reference: [Pages publishing sources](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).
