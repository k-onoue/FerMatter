# Publishing

- Canonical checklist: [Releasing](dev-docs/releasing.md).
- Versions, changelog, tags, reviewed releases: [versioning](dev-docs/releasing.md#versioning).
- Prototype DMG/PKG proposal: [packaging](dev-docs/reference/packaging.md).
- Public update contract proposal: [updates](dev-docs/reference/updates.md).
- Site build/preview and sample refresh: [documentation](dev-docs/documentation.md).
- Keep `releaseAvailable: false` until a signed, notarized, tested, anonymously accessible release exists.
- Reference: [GitHub Releases](https://docs.github.com/ja/repositories/releasing-projects-on-github/about-releases).

## Updating the demonstration

- Use synthetic `examples/` only; run `node scripts/prepare-demo.mjs` after changing samples.
- Refresh affected guides/media; run `node scripts/check-publication.mjs`.
- Full recipe: [documentation and media](dev-docs/documentation.md#tutorial-media).
