# Documentation and website

## Where to edit

| Content | Source | Output |
| --- | --- | --- |
| Repository summary | `README.md` | GitHub |
| Development instructions | `dev-docs/`, `AGENTS.md` | Repository only |
| User guides | `user-docs/*.md` | GitHub and Starlight `/guide/` |
| Public guide allowlist | `user-docs/pages.json` | Content collection |
| Homepage, features, downloads | `site/src/pages/` | Astro static pages |
| Navigation, shared UI | `site/src/components/`, `site/src/styles/site.css` | Site and guide |
| Demo descriptions and links | `site/src/data/demos.json` | Features page |
| Reviewed screenshots/video | `site/public/assets/` | Public media |
| Release availability and URLs | `site/src/data/site.json` | Download page |
| Example ZIPs | `docs/downloads/` from `examples/` | App setup and website |
| Download counters | `download-stats` Git branch | Generated `site/public/stats/downloads.json` |

- Stack: [Astro](https://docs.astro.build/) + [Starlight](https://starlight.astro.build/); static HTML on GitHub Pages.
- Search: [Pagefind](https://pagefind.app/), generated at build time; no search backend.
- One canonical explanation per topic; short bullets, tables, commands, and primary-source links.
- Publish only the allowlisted guides and reviewed assets; developer docs remain on GitHub.
- No generated HTML in Git. `site/dist/`, `.astro/`, generated ZIP copies, and statistics are ignored.
- Architecture decision: [0002 — Static product site and searchable guide](decisions/0002-starlight-site.md).

## Build and preview

- Node.js 22.22+; CI uses Node.js 24. App runtime requirements are separate.

```sh
npm --prefix scripts ci
npm --prefix site ci
npm --prefix scripts test
node scripts/build-guides.mjs --check
node scripts/check-publication.mjs
node scripts/preview-site.mjs
```

- Open [localhost:4174/FerMatter/](http://localhost:4174/FerMatter/).
- `build-guides.mjs --check` runs Astro diagnostics and builds the complete site; retained as the repository's check entry point.
- Fast editing: `npm --prefix site run dev`. Production search is verified after `npm --prefix site run build`.
- Preview serves only `site/dist/`, under the same `/FerMatter/` prefix as Pages.
- Each guide has `title`, `description`, and `sidebar.order` frontmatter; add its filename to `pages.json` and navigation to `astro.config.mjs`.
- Keep `.md` links readable on GitHub; the Markdown plugin validates and rewrites public guide/asset links.
- Local links outside the guide allowlist, public assets, and sample ZIPs fail the build, including symlink escapes.
- Old `/guide/topic.html` URLs redirect to `/guide/topic/`; `/guide/index.html` still works.
- UI checks: desktop/mobile, light/dark, keyboard navigation, search, old links, video controls/captions, sample downloads, no JavaScript for core reading/navigation.

## Product design

- Product design reference: [Ollama](https://ollama.com/), reviewed 2026-10-21. Use plain monochrome surfaces, left-aligned content, restrained type sizes, generous whitespace and a compact navigation bar.
- Keep product copy functional: feature names, supported formats, requirements and release status. Avoid repeated slogans and decorative section labels.
- Website appearance initially follows the OS. The sun/moon button toggles light/dark and remembers the choice in `starlight-theme`; clearing that preference restores automatic detection.
- Use the transparent Fermat mark for headers; reserve its blue tile for the application icon. See [artwork sources and regeneration](../desktop-app/resources/AppIcon.md).

## Publish

1. Review the PR; check the built site and publication scan.
2. For the migration, select **Settings → Pages → Build and deployment → Source → GitHub Actions** before merging. The previous source was `main:/docs`.
3. Merge into `main`; the **Publish website** workflow builds and deploys `site/dist/`.
4. Verify [the public site](https://k-onoue.github.io/FerMatter/) without authentication.

- `Documentation` validates PRs without publishing; `Publish website` runs on main, published releases, daily schedule, and manual dispatch.
- GitHub Pages environment approvals/rules still apply.
- Keep `repositoryPublic: false` while source access is restricted; switch it only after verifying anonymous repository access.
- Download availability is manual: keep `releaseAvailable: false` until anonymous release download and notarization/install checks pass. See [releasing](releasing.md).
- Review the Pages artifact itself: repository files and secrets must not enter it.

## Download counts

- `node scripts/download-stats.mjs` reads GitHub release asset counters; no browser API calls, analytics SDK, tracking cookies, or unique-user claims.
- CI uses its GitHub token for the API rate limit, but first checks repository visibility and rejects private repositories before reading release metadata. A private repository currently yields no new snapshot.
- Only stable release installer names starting with FerMatter or legacy Mathlidelux count: DMG, PKG, EXE, MSI, AppImage, DEB, RPM.
- Samples, source archives, checksums, signatures, drafts, and prereleases are excluded.
- Asset ID identifies a counter. Deleted assets retain their last observed count; replacement uploads get a new counter. Counts never decrease.
- Successfully observed snapshots are committed only to `download-stats`, using a normal fast-forward push. Main is never auto-committed.
- API failure preserves the prior snapshot and visible timestamp. No prior snapshot means “not available”, not zero.
- Full release and asset pagination must finish before writing. Deleted assets unseen by any snapshot cannot be recovered.
- Daily schedule records totals, not real-time installs. Git history retains the dated snapshots.
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
