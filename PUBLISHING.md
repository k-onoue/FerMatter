# Publication handoff

Repository: [k-onoue/mathlidelux](https://github.com/k-onoue/mathlidelux).
The product page lives in `docs/`. Pushing source code does not enable GitHub
Pages or create a downloadable Mac release; those are separate steps below.

## 1. Review and commit

The branch is `main`, and `origin` is `git@github.com:k-onoue/mathlidelux.git`.

```sh
python3 scripts/prepare-demo.py
./macos-app/scripts/check.sh
./macos-app/scripts/build.sh
git add .
git diff --cached --stat
git diff --cached
# After reviewing the staged files:
git commit -m "Describe the changes"
```

Keep the private `paper/` directory and ignored development captures out of Git.
The site assets were captured with `examples/demo-paper/`, not a private project.
Select an application license before describing the project as open source.

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

## 3. Push and enable GitHub Pages

Push committed changes with `git push origin main`. Then, in the repository's
**Settings → Pages → Build and deployment**:

- Source: **Deploy from a branch**
- Branch: **main**
- Folder: **/docs**

Save those settings. This is a static HTML/CSS/JS site with `.nojekyll`, relative
asset paths, and no build dependencies. No custom workflow is needed.
GitHub's [publishing-source instructions](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site)
describe this configuration. The usual project URL is
`https://k-onoue.github.io/mathlidelux/`.

Only `docs/` should be selected as the site source. The repository's code is
separately visible if the repository itself is public.

## 4. Prepare the Mac distribution (still to do)

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

[Apple's Developer ID distribution guide](https://developer.apple.com/developer-id/)
covers signing and notarization. This repository contains no signing credentials.

## 5. Attach the DMG and activate downloads

Create a published, non-prerelease GitHub Release for the tested version, then
attach the notarized disk image as **Mathlidelux.dmg**. Use that asset name for each
release. Update `version`, set `releaseAvailable: true`, and commit/push the page
configuration.

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
