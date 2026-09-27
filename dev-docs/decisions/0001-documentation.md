# 0001 — Shared Markdown and a small public guide

- Date: 2026-09-28.
- Status: adopted by this documentation change; no deployment settings changed.

## Decision

- Share the same development docs between people and agents.
- Keep `AGENTS.md` as a concise index and invariant list.
- Keep developer docs in repository Markdown; no separate hosted developer site yet.
- Write user instructions once in Markdown; generate public guide HTML with a pinned Markdown library.
- Publish only the explicit user-guide manifest and existing site assets.
- Check in generated pages so the existing `main:/docs` Pages deployment continues working.
- Add a freshness check to PR CI to prevent source/output drift.
- Use task-based pages, short bullets, and primary-source links.
- Keep detailed historic verification separate from current operating instructions.

## Why

- Repository docs remain readable without a site build.
- Public guides must work even while repository access is private.
- A small guide does not need a second theme, framework, or deployment pipeline.
- Checked-in HTML creates some diff duplication; deterministic generation and CI make that explicit.
- A later MkDocs migration can reuse Markdown if navigation/search needs grow; reconsider with a concrete need.

## References

- [GitHub Pages source configuration](https://docs.github.com/en/pages/getting-started-with-github-pages/configuring-a-publishing-source-for-your-github-pages-site).
- [Python-Markdown](https://python-markdown.github.io/).
- [MkDocs Markdown documentation](https://www.mkdocs.org/user-guide/writing-your-docs/).
