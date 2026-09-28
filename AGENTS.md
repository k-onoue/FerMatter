# Working on Mathlidelux

- Humans and agents use the same [development docs](dev-docs/README.md).
- Read only the pages relevant to the task; keep personal preferences outside this file.
- Use short bullets, tables, commands, and links in documentation; link to original references for detail.
- Keep changes focused; use a branch and a reviewed PR into `main`.
- Preserve existing work; do not merge unrelated feature branches into a documentation change.

## Invariants

- Library edits create presentation variants; Source mode edits real project files.
- Preserve saved data, drafts, bundle identity, and legacy `Slide Math` paths. Migrations need explicit handling.
- Cancel obsolete worker requests and ignore their late callbacks.
- Use synthetic `examples/` material for screenshots, tests, and public demos.
- Keep private manuscripts, credentials, builds, and local captures out of Git.
- A local ad hoc signed build is not a notarized public release.

## Read when needed

| Task | Read |
| --- | --- |
| Setup, branches, PRs | [Development](dev-docs/development.md) |
| Editor, workers, storage | [Architecture](dev-docs/architecture.md), [storage](dev-docs/reference/storage.md) |
| Validation and review | [Testing](dev-docs/testing.md), [reviewing](dev-docs/reviewing.md) |
| Versions, tags, releases | [Releasing](dev-docs/releasing.md) |
| Site, guides, tutorial media | [Documentation](dev-docs/documentation.md) |
| Settings, updates, packaging | [Reference index](dev-docs/README.md#reference) |

## Checks from the repository root

- App changes: `./desktop-app/scripts/check.sh` and `./desktop-app/scripts/build.sh`.
- Docs/site changes: `node scripts/build-guides.mjs --check` and `node scripts/check-publication.mjs`.
- Every change: `git diff --check`; inspect the staged diff before committing.
- See [setup](dev-docs/documentation.md#build-and-preview) for the docs environment.
- Report missing compiler tests or skipped UI checks; do not present them as passes.
