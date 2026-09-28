# Testing

## Choose checks by scope

| Change | Required evidence |
| --- | --- |
| App, frontend, compiler backend | `./desktop-app/scripts/check.sh`; `./desktop-app/scripts/build.sh` |
| User docs, site, site generator | Build guides; freshness check; publication check; browser preview |
| Developer docs only | Publication/link check; review commands and referenced behavior |
| Synthetic samples | `node scripts/prepare-demo.mjs`; compiler checks; publication check |
| Packaging | [Packaging verification](reference/packaging.md); distinguish inspection from installation |
| Release | All affected checks; real compiler coverage; [release checklist](releasing.md) |

## App checks

- `check.sh`: TypeScript/Vite production build, frontend tests, Rust tests, four synthetic real-compiler checks.
- TypeScript rejects unused imports, locals, and parameters. Keep the main/Instant appearance hook and command modules shared instead of copying handlers.
- Real compiler checks run by default; `MATHLIDELUX_COMPILER_TESTS=0` explicitly skips them.
- Missing compilers fail the default run. A skipped run is not a release pass.
- Run `cargo clippy --manifest-path desktop-app/src-tauri/Cargo.toml --lib -- -D warnings` with the same Rust environment.
- PR CI runs frontend/Rust checks on macOS with compiler checks explicitly skipped. The separate [release gate](reference/release-validation.md) requires all four real compiler paths, an app build and dependency audits; it records skips/failures distinctly.
- Build verifies the local ad hoc signature; it does not verify notarization.
- Test storage/source changes with disposable copies and external-edit conflicts.
- Test worker changes with cancellation and late results after switching projects.
- Test exports by opening the result in the target app; do not infer drag support from copy support.

## Documentation/site checks

```sh
npm --prefix scripts ci
npm --prefix scripts test
node scripts/build-guides.mjs
node scripts/build-guides.mjs --check
node scripts/check-publication.mjs
git diff --check
```

- Environment setup and preview: [documentation](documentation.md#build-and-preview).
- Check desktop/mobile widths, keyboard navigation, light/dark appearance, and JavaScript disabled.
- Check video play/pause, caption track, poster, and the same steps as text.
- Serve only `docs/`; also test under the `/mathlidelux/` project subpath.
- CI checks guide freshness, local links, public fixtures, and whitespace.

## Record results

- PR: commands, outcomes, relevant environment, skips, and limitations.
- Historical experiments: [history](history/validation-2026-09.md); do not copy old counts into a new validation claim.
- User-facing changes: [CHANGELOG.md](../CHANGELOG.md), not the test log.
- Reference: [Reviewing tests](https://google.github.io/eng-practices/review/reviewer/looking-for.html#tests).
