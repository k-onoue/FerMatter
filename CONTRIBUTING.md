# Contributing to Mathlidelux

This guide covers local development, branches, and pull requests.
For packaging and publishing a Mac release, see [PUBLISHING.md](PUBLISHING.md).
The product page in `docs/` is for users: features, requirements, samples, and downloads.

## Set up locally

Follow the [requirements](macos-app/README.md#requirements), then run from the
repository root:

```sh
./macos-app/scripts/build.sh
open macos-app/build/Mathlidelux.app
```

This produces an ad hoc signed development app. It is not the public installer.
See the [architecture guide](macos-app/ARCHITECTURE.md) before changing editor,
project-storage, or worker lifecycles. The [validation notes](macos-app/VALIDATION.md)
record tested behavior and remaining limitations.

## Branches

Keep `main` as the shared integration branch. Make changes on a separate branch
and merge through a reviewed pull request. Use short-lived branches; a permanent
`develop` branch is not needed for the current workflow.

| Purpose | Example |
| --- | --- |
| Prepare one release | `release/v0.10.0` |
| Add a feature | `feat/pdf-navigation` |
| Fix a bug | `fix/editor-undo` |
| Update documentation | `docs/development-workflow` |

These are naming examples, not existing or scheduled releases. Use lowercase,
hyphenated descriptions and agree on the version before starting a release branch.
A release branch can collect a small set of related release changes. Larger or
independent changes should use their own feature/fix branches and PRs into `main`.
Bring those changes into any open release branch before final validation.

Start from an up-to-date `main`, with a clean working tree. Commit or stash any
unfinished work first. For example:

```sh
git switch main
git pull --ff-only
git switch -c release/v0.10.0
```

Replace the branch name with the release or task you are actually working on.
Keep commits focused and describe what changed, for example
`Fix undo in source editors`. Avoid mixing unrelated cleanup into a fix.

## Check and submit a pull request

For application or worker changes, run:

```sh
./macos-app/scripts/check.sh
./macos-app/scripts/build.sh
```

The checks run Swift tests, real compiler integration, Python tests, and publication
checks. They require the tools listed in the [requirements](macos-app/README.md#requirements).
Also verify changed UI behavior in the app with a synthetic sample project.

For documentation or product-page changes, check the links and relevant content:

```sh
python3 -B scripts/check-publication.py
git diff --check
```

Preview site changes using the [local preview instructions](README.md#preview-the-product-page).
If you change demo sources, follow [Updating the demonstration](PUBLISHING.md#updating-the-demonstration)
before running publication checks.

Review `git diff`, stage the intended files, and review `git diff --cached`
before committing. Push your working branch, for example:

```sh
git push -u origin release/v0.10.0
```

Open a PR with **`main` as the base** and your working branch as the compare branch.
Describe the problem, resulting behavior, checks run, and any remaining limitations.
Include screenshots for visible changes and link a related issue when applicable.

Get at least one approval from another collaborator before merging. Resolve
review feedback and any conflicts, and rerun affected checks after changes.
This is the collaboration convention; enforcing it on GitHub requires a repository
owner to configure **Settings → Rules → Rulesets** for `main`, requiring a PR and
one approval, blocking force pushes, and restricting deletions. This document
does not indicate whether that ruleset has been enabled.

After merging, delete the merged branch on GitHub, update local `main` with
`git pull --ff-only`, and start a fresh branch for the next task. Release branches
are temporary too. This follows the lightweight
[GitHub flow](https://docs.github.com/en/get-started/using-github/github-flow).

## Release branches, tags, and notes

`release/v0.10.0` is a working branch. The release tag `v0.10.0` identifies the
exact tested commit being distributed, after its PR has merged into `main`.
Creating a branch does not create a tag or publish an app. Do not move a published
release tag; use a new version for a subsequent release.

Keep user-facing release notes and the downloadable DMG in
[GitHub Releases](https://github.com/k-onoue/mathlidelux/releases). Describe new
features, fixes, requirements, and known limitations there. Keep build, signing,
and deployment procedures in [PUBLISHING.md](PUBLISHING.md), and link to the release
from the product page. Branch conventions belong in this guide.

## Public files

Use synthetic material from `examples/` for fixtures, screenshots, and bug reports.
Keep private manuscripts (`paper/`), local builds, credentials, and ignored captures
out of Git. Do not force-add them. Publication checks help catch accidental
inclusions, but review the staged files before every commit.
