# 0002 — Static product site and searchable guide

- Date: 2026-10-20.
- Supersedes the website generation/deployment parts of [0001](0001-documentation.md).

## Decision

- Astro for the homepage, visual feature demos, and downloads.
- Starlight for guide navigation, accessible search, page contents, and appearance switching.
- Keep `user-docs/` Markdown as the source; an explicit manifest selects published pages.
- Keep developer documentation in the repository.
- Shared header/footer/theme; screenshots and media metadata managed separately from prose.
- Build static output in GitHub Actions; deploy only `site/dist/` to Pages.
- Read GitHub installer counters at build time; retain observed totals on a separate data branch.

## Tradeoffs

- Adds a Node build and framework dependencies; lockfiles, CI, and dependency audits cover upgrades.
- Removes the custom HTML renderer and checked-in generated pages.
- No hosted backend or per-visitor API dependency; basic reading and links work without JavaScript.
- Search and theme controls need small client scripts.
- Download counts describe GitHub asset requests, not users, installs, or sample ZIP downloads.
- Existing guide URLs remain supported; switching Pages from branch deployment to Actions is a one-time repository setting.

## References

- [Astro documentation](https://docs.astro.build/).
- [Starlight documentation](https://starlight.astro.build/).
- [Yazi feature page](https://yazi-rs.github.io/features/): reference for large demonstrations and concise feature text; no assets copied.
- [Maintainer instructions](../documentation.md).
