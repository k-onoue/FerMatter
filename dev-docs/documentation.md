# Documentation and website

## Where to edit

| Content | Source | Output |
| --- | --- | --- |
| Repository summary | `README.md` | GitHub |
| Development instructions | `dev-docs/`, `AGENTS.md` | Repository only |
| User guides | `user-docs/*.md` | GitHub and Starlight `/guide/` |
| Public guide allowlist | `user-docs/pages.json` | Content collection |
| Homepage, features, Download layout | `site/src/pages/` | Astro static pages |
| Download instructions | `site/src/content/download/*.md` | `/download/` |
| Announcements | `site/src/content/news/*.md` | `/news/` and individual articles |
| Navigation, shared UI | `site/src/components/`, `site/src/styles/site.css` | Site and guide |
| Demo descriptions and links | `site/src/data/demos.json` | Features page |
| Reviewed screenshots/video | `site/public/assets/` | Public media |
| Release availability and URLs | `site/src/data/site.json` | Download page |
| Example ZIPs | `docs/downloads/` from `examples/` | App setup and website |
| Download counters | GitHub release asset counters | Generated `site/public/stats/downloads.json` |

- Stack: [Astro](https://docs.astro.build/) + [Starlight](https://starlight.astro.build/); static HTML on GitHub Pages.
- Search: [Pagefind](https://pagefind.app/), generated at build time; no search backend.
- One canonical explanation per topic; short bullets, tables, commands, and primary-source links.
- Guide ownership: overview/project folders in `README.md`; tools/compiler errors in `requirements.md`; editing modes/PDF refresh in `paper-to-slides.md`; image format/size/color/SVG restoration in `export.md`; keybindings/zoom in `shortcuts.md`; app appearance/theme files in `appearance.md`; agent setup/permissions in `automation.md`; Library exclusions in `extraction.md`. Cross-link to the owner instead of repeating instructions.
- Download owns app installation and updates; installer notes cover macOS first-launch approval. Guide setup begins with workspace selection after the app opens. Keep existing guide routes and section anchors when reorganizing content.
- Release assets/counters use `releaseRepository`, separately from the application source `repository`. The download page owns the copyable full/BasicTeX/Typst install commands; requirements link there. Recheck installer and package-metadata sizes when changing the TeX release.
- Publish only the allowlisted guides and reviewed assets; developer docs remain on GitHub.
- No generated HTML in Git. `site/dist/`, `.astro/`, generated ZIP copies, and statistics are ignored.
- Architecture decision: [0002 — Static product site and searchable guide](decisions/0002-starlight-site.md).

## Build and preview

- Node.js 24.22.1+ LTS recommended; CI uses `.node-version`. Supported alternatives: 22.23.4+ and 26.12.1+ within those major versions. These minimums include the [2026-10-21 security fixes](https://nodejs.org/en/blog/vulnerability/october-2026-security-releases). App runtime requirements are separate.

```sh
npm --prefix scripts ci
npm --prefix site ci
npm --prefix scripts test
node scripts/build-guides.mjs --check
node scripts/check-publication.mjs
node scripts/preview-site.mjs
```

- Open [localhost:4174](http://localhost:4174/).
- `build-guides.mjs --check` runs Astro diagnostics and builds the complete site; retained as the repository's check entry point.
- Fast editing: `npm --prefix site run dev`. Production search is verified after `npm --prefix site run build`.
- Preview serves only `site/dist/`, at `/`, matching the organization site.
- Each guide has `title` and `description` frontmatter. `pages.json` defines the published pages and their sidebar/previous/next order; route helpers reject other guides. Retired Markdown may remain in the repository without a public route.
- Keep `.md` links readable on GitHub; the Markdown plugin validates and rewrites public guide/asset links.
- Local links outside the guide allowlist, public assets, and sample ZIPs fail the build, including symlink escapes.
- Old `/guide/topic.html` URLs redirect to `/guide/topic/`; `/guide/index.html` still works.
- UI checks: desktop/mobile, light/dark, keyboard navigation, search, old links, video controls/captions, sample downloads, no JavaScript for core reading/navigation.

## Markdown content

- **Guides:** edit `user-docs/*.md`; add new pages to `user-docs/pages.json`. The same Markdown renders on GitHub and the website; do not maintain a second HTML copy.
- **Download:** edit `site/src/content/download/*.md` (macOS, Windows, Windows installer notes and Ubuntu). Markdown owns headings, instructions, tables and shell commands. Fenced `sh` blocks get the same copy controls as guides. Keep Ubuntu last and preserve the explicit legacy anchors.
- `site/src/pages/download.astro` owns the dynamic release button/name and availability gates from `site.json`. Update those values through the release process; Markdown edits do not publish app binaries.
- **News:** add a dated `.md` file to `site/src/content/news/`. The homepage automatically lists the five newest dates and titles. Each article appears in `/news/`, linked from every footer, and gets its own `/news/<filename>/` route.

```yaml
---
title: "A concise announcement"
description: "What changes for the user."
date: 2026-10-26
status: upcoming
---
```

- `status` is `upcoming` or `available`. Use `upcoming` for merged changes absent from the downloadable release, and state the current release explicitly. Change to `available` only after release verification; link its release notes.
- Use Markdown headings and short paragraphs/bullets. Link to the owning guide instead of duplicating setup instructions.

## Product design

- Product design reference: [Ollama](https://ollama.com/), reviewed 2026-10-21. Use plain monochrome surfaces, left-aligned content, restrained type sizes, generous whitespace and a compact navigation bar.
- Keep product copy functional: feature names, supported formats, requirements and release status. Avoid repeated slogans and decorative section labels.
- Product pages use a centered reading column; guides reserve space for visible navigation. Intermediate widths must not reserve a guide sidebar on product pages. Shared styles live in `site/src/styles/site.css`. Guide pagination uses small text links between published guides.
- All content pages use the shared product footer, including downloads, guides, and 404. Guide pagination stays above the footer.
- Keep Features, Guide, and Download in the header. The footer links to News, Declaration and Issues; GitHub appears there only when hidden in the header.
- Website appearance initially follows the OS. The sun/moon button toggles light/dark and remembers the choice in `starlight-theme`; clearing that preference restores automatic detection.
- Use the transparent Fermat mark for headers; reserve its blue tile for the application icon. See [artwork sources and regeneration](../desktop-app/resources/AppIcon.md).
- Link-preview image metadata lives in `site/astro.config.mjs`: use an absolute public URL for `site/public/assets/icon.png`, with its PNG type, dimensions and alt text. X cards use the square `summary` layout.

## Publish

1. Review the PR; check the built site and publication scan.
2. Merge into `k-onoue/FerMatter:main`; **Publish website** builds and validates `site/dist/`.
3. The workflow pushes only static output to the public `FerMatter/fermatter.github.io:main`. That repository's **Publish website** workflow deploys to [fermatter.github.io](https://fermatter.github.io/).
4. After the new deployment is live, deploy `site/legacy-dist/` to the old project site. Old page URLs redirect to the same path on the new host, preserving query strings and fragments; existing assets and sample ZIPs remain available.
5. Verify the new site and an old guide link without authentication.

- App sources, issues, settings and Git history stay in `k-onoue/FerMatter`. Site and guide sources also stay here; the website repository contains generated output, its deployment workflow and public binary Releases.
- `WEBSITE_DEPLOY_KEY` is a repository secret here. Its public key is a write-enabled deploy key on the website repository only; it grants no access to app sources. GitHub's SSH host key is pinned in `scripts/website-known-hosts`.
- `scripts/publish-website.mjs` preserves the public repository's `.github/`, `.gitattributes`, `README.md` and `LICENSE`, removes retired generated assets, rejects symlinks, and uses a normal fast-forward push. Change public repository controls through a reviewed PR.
- Bootstrap the public repository with `scripts/website-pages.yml` at `.github/workflows/pages.yml`, `scripts/website-gitattributes` at `.gitattributes`, `README.md`, `LICENSE` and the reviewed static build. Enable **Settings → Pages → GitHub Actions**. Its deployment environment admits `main`.
- Root-hosted links use an empty `base` in `site/src/data/site.json`; the Astro configuration normalizes this to `/`. Local preview is at `http://127.0.0.1:4174/`.

- `Documentation` validates PRs without publishing; `Publish website` runs on main, published releases, daily schedule, and manual dispatch.
- GitHub Pages environment approvals/rules still apply.
- Keep `repositoryPublic: false` while source access is restricted; switch it only after verifying anonymous repository access.
- Download availability is manual: keep `releaseAvailable: false` until anonymous download, checksum and the selected release-channel checks pass. Preview downloads must explicitly state their unnotarized status; stable releases retain the full signing/install checklist. See [releasing](releasing.md).
- Windows uses independent `windowsReleaseAvailable`, `windowsReleaseTag`, `windowsReleaseName` and `windowsAssetName` fields. Enable its unsigned preview only after the exact EXE's anonymous download, checksum and installation/startup checks; retain the existing macOS asset fields.
- Review the Pages artifact and public-repository diff: app sources, Git history and secrets must not enter either. `deployment.json` exposes only the source commit ID for deployment readiness checks.

## Download counts

- `node scripts/download-stats.mjs` reads GitHub release asset counters; no browser API calls, analytics SDK, tracking cookies, or unique-user claims.
- CI uses its GitHub token for the API rate limit, but first checks repository visibility and rejects private repositories before reading release metadata. A private repository currently yields no new snapshot.
- Published installer names starting with FerMatter or legacy Mathlidelux count, including previews: DMG, PKG, EXE, MSI, AppImage, DEB, RPM.
- Samples, source archives, checksums, signatures and drafts are excluded.
- Each clean workflow build collects currently published assets; no statistics branch is created or updated.
- Asset ID identifies a counter. When a previous local snapshot exists, deleted assets retain their last observed count and counts never decrease; replacement uploads get a new counter. Clean workflow builds do not retain deleted assets across runs.
- API failure preserves an existing local snapshot and visible timestamp. No prior snapshot means “not available”, not zero.
- Site preparation imports the validated snapshot as generated build data. Publication checks compare the rendered total, release rows and timestamp with the published JSON.
- Full release and asset pagination must finish before writing. Deleted assets unseen by any snapshot cannot be recovered.
- The daily website workflow refreshes totals, not real-time installs. Historical snapshots are not stored in Git.
- References: [GitHub release assets API](https://docs.github.com/en/rest/releases/assets), [GitHub Pages workflows](https://docs.github.com/en/pages/getting-started-with-github-pages/using-custom-workflows-with-github-pages).

## Demo media

- Use synthetic `examples/` only; exclude private projects, user paths, desktop notifications, and account information.
- Add reviewed screen captures under `site/public/assets/demos/`; add one entry to `demos.json`. One component renders captions and guide/sample links.
- Current screenshots and earlier Swift video have separate provenance in [asset notes](../site/public/assets/README.md).
- Existing 18-second clip is a step-through of real Swift prototype screens, explicitly labeled historical; it is not current UI evidence.
- Videos: H.264 MP4, JPEG poster, WebVTT captions, native controls, `preload="none"`, no autoplay/loop. Keep equivalent text steps.
- Re-capture affected screens after UI changes; verify every frame before committing.
- Run `node scripts/prepare-demo.mjs` after sample edits. The four ZIPs have explicit allowlists; no symlinks or compiler output.
- Site preparation copies those same ZIPs; the desktop app embeds the originals in `docs/downloads/`.
