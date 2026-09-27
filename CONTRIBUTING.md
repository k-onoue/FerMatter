# Contributing to Mathlidelux

- **Contributors:** use this guide for local development, branches, and PRs.
- **Release maintainers:** use [PUBLISHING.md](PUBLISHING.md) for packaging and publishing.
- **App users:** use the product page in `docs/` and the [usage guide](macos-app/README.md#start-a-project).

## Set up locally

- Install the tools listed in [Requirements](macos-app/README.md#requirements).
- Run from the repository root:

```sh
./macos-app/scripts/build.sh
open macos-app/build/Mathlidelux.app
```

- The build is ad hoc signed for local development.
- Read [Architecture](macos-app/ARCHITECTURE.md) before changing editor, storage, or worker lifecycles.
- Check [Validation](macos-app/VALIDATION.md) for tested behavior and limitations.

## Branches

- **Base:** start from the latest `main`; commit or stash unfinished work first.
- **Working branch:** create your own branch for each task or release.
- **Scope:** keep unrelated changes in separate branches and PRs.
- **Names:** use lowercase and hyphens. Examples:
  - `feat/pdf-navigation` — feature.
  - `fix/editor-undo` — bug fix.
  - `docs/development-workflow` — documentation.
  - `release/v0.10.0` — release preparation; agree on the version first.
- **Release branches:** include relevant changes from `main` before final validation.

Example — replace the branch name for your task:

```sh
git switch main
git pull --ff-only
git switch -c release/v0.10.0
```

## Checks

- **App or worker changes:** run the tests and build; verify changed UI behavior with a synthetic sample.

```sh
./macos-app/scripts/check.sh
./macos-app/scripts/build.sh
```

- `check.sh` runs Swift tests, real compiler integration, Python tests, and publication checks.
- **Docs or product-page changes:** check content and links, then run:

```sh
python3 -B scripts/check-publication.py
git diff --check
```

- **Site changes:** [preview locally](README.md#preview-the-product-page).
- **Demo changes:** [refresh fixtures and ZIPs](PUBLISHING.md#updating-the-demonstration) before checking.

## Pull requests

- **Review:** inspect `git diff`, stage only intended files, then inspect `git diff --cached`.
- **Commit:** keep changes focused; use a descriptive message, e.g. `Fix undo in source editors`.
- **Push:** publish your working branch. Example:

```sh
git push -u origin release/v0.10.0
```

- **Target:** open a PR from your working branch into `main`; do not push directly to `main`.
- **Description:** summarize the problem, changes, checks, and limitations; link related issues.
- **UI changes:** attach screenshots when useful.
- **Approval:** get at least one approval from another collaborator.
- **Before merging:** resolve feedback and conflicts; rerun affected checks.
- **After merging:** delete the merged branch, including release branches.
- **Next task:** switch to `main`, run `git pull --ff-only`, and create a fresh branch.
- **Reference:** [GitHub flow](https://docs.github.com/en/get-started/using-github/github-flow).

## Release branches, tags, and notes

- **Branch:** `release/v0.10.0` is temporary working space.
- **Tag:** `v0.10.0` marks the exact tested commit in `main` after the PR merges.
- **Published tags:** keep them fixed; use a new version for subsequent releases.
- **Notes and DMG:** publish in [GitHub Releases](https://github.com/k-onoue/mathlidelux/releases).
- **Release notes:** list features, fixes, requirements, and known limitations.
- **Packaging and deployment:** follow [PUBLISHING.md](PUBLISHING.md).

## Public files

- Use synthetic material from `examples/` for fixtures, screenshots, and bug reports.
- Keep `paper/`, local builds, credentials, and ignored captures out of Git; never force-add them.
- Review staged files before every commit, even when publication checks pass.
