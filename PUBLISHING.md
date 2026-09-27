# Publishing Mathlidelux

This guide is for release maintainers. For local development, branch names, and
PR review, see [CONTRIBUTING.md](CONTRIBUTING.md).

Repository: [k-onoue/mathlidelux](https://github.com/k-onoue/mathlidelux).
The user-facing product page lives in `docs/`. Version notes and installers belong
in [GitHub Releases](https://github.com/k-onoue/mathlidelux/releases).
Publishing source code does not enable GitHub Pages or create a downloadable Mac
release; those are separate steps below.

## 1. Prepare a release branch

Start a `release/vX.Y.Z` branch from an up-to-date `main` using the
[contribution workflow](CONTRIBUTING.md#branches). Use separate branches and PRs
for later page-only updates as well; do not push changes directly to `main`.

For a new app version, update `CFBundleShortVersionString` and increment
`CFBundleVersion` in [`macos-app/scripts/build.sh`](macos-app/scripts/build.sh).
Keep the site's advertised `version` aligned with the downloadable release when
activating downloads in step 5.

After preparing the changes, validate them:

```sh
./macos-app/scripts/check.sh
./macos-app/scripts/build.sh
```

Rebuild sample ZIPs only if their sources changed, using the instructions below.
Review and commit the intended files on the working branch, then open a PR to
`main`. Complete the distribution preparation below before merging an app release.

Keep the private `paper/` directory and ignored development captures out of Git.
The site assets were captured with `examples/demo-paper/`, not a private project.

## 2. Configure the page

Edit [`docs/site-config.js`](docs/site-config.js):

```js
window.MATHLIDELUX_SITE = Object.freeze({
  repository: "k-onoue/mathlidelux",
  releaseAvailable: false,
  version: "0.10.0",
  assetName: "Mathlidelux.dmg",
});
```

`repository` enables the GitHub and build-instructions links. Leave
`releaseAvailable: false` until a tested distribution DMG exists in Releases.
The page accurately displays “Mac download coming soon” in the meantime.
The four sample ZIPs are already included and can be downloaded independently.
Open them with **Open Project** in the app. **New Project** creates a project
from pasted text; its **Load File…** button reads a single source file's text.
The [usage guide](macos-app/README.md#start-a-project) explains both workflows
and the built-in and custom templates.

Local preview (serve **docs only**, not the repository or private manuscripts):

```sh
python3 -m http.server 4174 --bind 127.0.0.1 --directory docs
```

## 3. Prepare the Mac distribution (still to do)

The current build script creates an **ad hoc signed development app**, not a
public distribution build. Do not upload it as the finished installer.

Before releasing:

- Choose a permanent bundle identifier. The Mathlidelux icon is already included
  in `macos-app/Resources/AppIcon.icon` and compiled by the build script.
  The local Mathlidelux build keeps `local.slidemath.prototype` to preserve the
  existing development settings. A later identifier change needs a preferences
  migration; do not rename or move existing project storage just for branding.
- Decide which dependencies to bundle. Today Python 3 and the Typst CLI are external dependencies for Typst projects.
  LaTeX projects need Python 3, TeX, latexmk, preview.sty,
  pdfcrop, pdf2svg, and Poppler. DVI workflows also use dvips and
  Ghostscript. A clean Mac needs setup and missing-tool handling.
- Keep the advertised target at macOS 26+ / Apple Silicon until other targets
  are built and tested.
- Sign the app and any bundled executables with Developer ID, enable the hardened
  runtime as appropriate, notarize it, and attach the notarization ticket.
- Package and test `Mathlidelux.dmg` on a Mac without this development environment.
- Check notices and licenses for any tools or fonts included in the distribution.
- Place `LICENSE` and `NOTICE` at the DMG root. `build.sh` already copies them
  into `Contents/Resources/`. Add third-party notices for any bundled tools.

[Apple's Developer ID distribution guide](https://developer.apple.com/developer-id/)
covers signing and notarization. This repository contains no signing credentials.

## 4. Merge the PR and enable GitHub Pages

Merge the reviewed PR into `main` after the relevant checks pass. For the initial
site deployment, in the repository's **Settings → Pages → Build and deployment**:

- Source: **Deploy from a branch**
- Branch: **main**
- Folder: **/docs**

Save those settings. This is a static HTML/CSS/JS site with `.nojekyll`, relative
asset paths, and no build dependencies. No custom workflow is needed.
GitHub's [publishing-source instructions](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)
describe this configuration. The usual project URL is
`https://k-onoue.github.io/mathlidelux/`.

Only `docs/` should be selected as the site source. Subsequent page changes deploy
after their PRs merge into `main`; check the Pages deployment result in Actions.
The product page can be published before an installer is ready by keeping
`releaseAvailable: false`.

## 5. Attach the DMG and activate downloads

Create a version tag such as `v0.10.0` on the exact tested commit in `main` after
the release PR has merged. Build and validate the distribution from that commit;
do not tag an unrelated, newer commit merely because it is now the branch tip.

Prepare a GitHub Release for that tag, with user-facing notes covering features,
fixes, requirements, and known limitations. Attach the notarized disk image as
**Mathlidelux.dmg**, then publish the release as a non-prerelease. Use that asset
name for each release. Update the site's `version` and set `releaseAvailable: true`
through a separate reviewed PR after the download is available.

The button then resolves to:

```text
https://github.com/k-onoue/mathlidelux/releases/latest/download/Mathlidelux.dmg
```

The [latest-release asset link](https://docs.github.com/en/repositories/releasing-projects-on-github/linking-to-releases)
follows GitHub's latest published release. It does not target a draft or preview
release. Keep `releaseAvailable` false if there is no matching asset.

## Updating the demonstration

1. Edit only `examples/demo-paper/`, `examples/demo-typst/`, `examples/demo-tikz/`,
   or `examples/demo-cetz/` with synthetic content.
2. Run `python3 ui-prototype/scripts/build-samples.py` to refresh the browser fixture.
3. Run `python3 scripts/prepare-demo.py` to rebuild the four downloadable ZIPs.
4. If the content or counts changed, refresh page copy and native screenshots.
5. Run `python3 scripts/check-publication.py` and review staged changes.

Each demo ZIP uses its own explicit source allowlist. Local TeX outputs, extra
files, private manuscripts, caches, and symlinks cannot be silently added to it.
