# Documentation and website

## Where to edit

| Content | Source | Audience/output |
| --- | --- | --- |
| Project summary | `README.md` | Repository visitors |
| Agent entry point | `AGENTS.md` | Short rules and task links |
| Development instructions | `dev-docs/` | Humans and agents in the repository |
| User guides | `user-docs/*.md`, `user-docs/pages.json` | Repository readers and generated `docs/guide/*.html` |
| Product homepage/design | `docs/index.html`, `docs/styles.css`, `docs/site.js` | Public GitHub Pages site |
| Tutorial video/poster/captions | `docs/assets/tutorials/` | Public website |
| User-facing changes | `CHANGELOG.md` | Release history |
| Past verification | `dev-docs/history/` | Dated evidence |

- One canonical explanation per topic; old entry points link to it.
- Short bullets, tables, and commands; one action or fact per bullet.
- Put references beside the relevant rule; link to original sources for detail.
- Update behavior and its documentation in the same PR.
- Do not present an open PR's feature as shipped; record proposals in the dated [PR snapshot](history/pr-snapshot-2026-09-28.md).

## Build and preview

- Python 3.11+ for guide generation; app runtime requirements are separate.

```sh
python3 -m venv .venv-docs
.venv-docs/bin/python -m pip install -r scripts/requirements-docs.txt
.venv-docs/bin/python scripts/build-guides.py
.venv-docs/bin/python scripts/build-guides.py --check
python3 -B scripts/check-publication.py
python3 -m http.server 4174 --bind 127.0.0.1 --directory docs
```

- Open [localhost:4174](http://localhost:4174); serve `docs/` only.
- Add a Markdown page plus a `pages.json` entry; first heading must match the entry title.
- Link to other `.md` guides normally; generation rewrites links to their HTML pages.
- Local assets must resolve inside `docs/`; references to developer/private files fail the build.
- Generated HTML is committed; do not edit it by hand. `--check` writes nothing and fails for stale output.
- When changing shared CSS/JS for publication, bump its `?v=` token in the homepage and guide template together to refresh cached assets.
- Remove obsolete generated pages explicitly when removing a manifest entry.
- GitHub Pages remains `main:/docs`; no deployment-mode change or runtime framework required.
- CI checks generated freshness and local links; site preview itself needs no installed dependencies.
- Reference: [Python-Markdown](https://python-markdown.github.io/), [Pages publishing sources](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).

## Product tone

- Lead with the task: bring an equation from a paper into a slide.
- Use the app's actual control labels; explain one operation at a time.
- Support the speaker's choices about content and arrangement.
- Keep requirements/release status visible; put advanced detail on its own guide page.
- Prefer whitespace, readable text, and actual app screens; avoid decorative feature grids.
- Motivation: [Make the talk your own](../user-docs/why.md).

## Tutorial media

- Use synthetic `examples/` content; exclude project-history lists, personal paths, accounts, and desktop notifications.
- Primary clip: 18-second step-through of real screens, not a continuous recording or an interactive mockup.
- It shows Library → selected Trajectory/Copy SVG → a real pasted/resized PowerPoint graphic.
- Current captures: prototype 0.10.0, integration commit `7b0205a`, macOS 26; core workflow also exists on the `main` baseline.
- Capture recipe: [asset notes](../docs/assets/README.md).
- Use H.264 MP4, a JPEG poster, and WebVTT captions; retain matching text steps.
- Native playback controls; no autoplay or automatic loops; `preload="none"`; no motion until requested.
- Re-capture affected screens when controls or behavior change; verify the final paste in the receiving app.
- For new clips: one task, roughly 10–25 seconds; update the recipe/version and verify every frame for public content.
- References: [Video performance](https://web.dev/learn/performance/video-performance), [HTML video](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/video).

## Samples

- Edit only synthetic files in `examples/`.
- Run `python3 -B scripts/prepare-demo.py`; review the four ZIPs and page copy.
- Each ZIP has an explicit source allowlist; no private sources, generated compiler files, or symlinks.
- Run publication checks and inspect the staged diff before pushing.
