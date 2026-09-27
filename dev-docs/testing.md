# Testing

## Choose checks by scope

| Change | Required evidence |
| --- | --- |
| App, Swift, workers | `./macos-app/scripts/check.sh`; `./macos-app/scripts/build.sh` |
| User docs, site, site generator | Build guides; freshness check; publication check; browser preview |
| Developer docs only | Publication/link check; review commands and referenced behavior |
| Synthetic samples | `python3 -B scripts/prepare-demo.py`; compiler checks; publication check |
| Packaging | [Packaging verification](reference/packaging.md); distinguish inspection from installation |
| Release | All affected checks; real compiler coverage; [release checklist](releasing.md) |

## App checks

- `check.sh`: Swift tests, Python tests, publication checks.
- Real compiler tests are enabled by default (`SLIDEMATH_COMPILER_TESTS=1`).
- Missing tools can skip tests; a release pass must install requirements and explain remaining skips.
- Build verifies the local ad hoc signature; it does not verify notarization.
- Test storage/source changes with disposable copies and external-edit conflicts.
- Test worker changes with cancellation and late results after switching projects.
- Test exports by opening the result in the target app; do not infer drag support from copy support.

## Documentation/site checks

```sh
.venv-docs/bin/python scripts/build-guides.py
.venv-docs/bin/python scripts/build-guides.py --check
python3 -B scripts/check-publication.py
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
