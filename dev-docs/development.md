# Development

## Setup

- Apple Silicon Mac; Rust stable (1.92+), Node.js 24.22.1+ LTS (see `.node-version`), Xcode command-line tools.
- macOS 14+ deployment target; verified on macOS 26.
- Install the tools for the selected format: [requirements](../user-docs/requirements.md).
- Run from the repository root:

```sh
./desktop-app/scripts/build.sh
open desktop-app/src-tauri/target/release/bundle/macos/FerMatter.app
```

- Output defaults to an ad hoc signed local app; no notarization. For remembered folder consent across rebuilds, configure [local signing](reference/signing.md).
- Scripts install locked npm dependencies when missing; an optional project-local Rust toolchain is supported. See [desktop setup](../desktop-app/README.md#build-and-run).
- App icon provenance: [AppIcon.md](../desktop-app/resources/AppIcon.md).

## Formatting

```sh
npm ci
npm run format
npm run format:check
```

- [Prettier](https://prettier.io/docs/configuration) formats JS/TS, HTML, CSS, Astro, JSON and workflow YAML; the app and repository use the same version.
- [rustfmt](https://github.com/rust-lang/rustfmt) formats the Rust crate, including examples and tests. Install the `rustfmt` component with the stable Rust toolchain.
- Shared settings: `.prettierrc.json`, `.prettierignore`, `rustfmt.toml` and `.editorconfig`. Generated files and private projects are excluded.
- The **Formatting** workflow checks both tools on PRs and `main`.

## Local generated files

- Canonical artwork lives in `desktop-app/resources/`; regenerate derivatives with `node scripts/build-icons.mjs`. Its intermediate images use an automatically removed temporary directory, including when rendering fails.
- Old captures, QA apps, local app backups and superseded installers under `desktop-app/build/` can be moved to Trash after checking they are unused. Preserve the current release, `rustup/`, `audit-tools/`, `dmg-tools/`, compiler tools and `signing-identity.txt`.
- Check nested Git checkouts for unpublished work before removing them. Keep real projects, drafts and settings outside cleanup scope.

## Branch and PR

1. Account for uncommitted work; start a task branch from current `main`.
2. Keep one purpose per PR, including its relevant tests and documentation.
3. Run the [affected checks](testing.md); inspect `git diff` and the staged diff.
4. Push the task branch; open a PR into `main`.
5. Obtain another collaborator's approval and satisfy repository rules. The repository owner may explicitly bypass the approval requirement through a PR; direct pushes remain restricted.
6. Resolve feedback/conflicts; rerun affected checks; merge through the PR.
7. Delete the merged task branch when no work still depends on it.

```sh
git fetch origin
git switch -c docs/short-guides origin/main
# Make and review the intended changes before committing.
git push -u origin docs/short-guides
```

- Names: lowercase, descriptive; examples `feat/pdf-navigation`, `fix/editor-undo`, `docs/short-guides`, `codex/short-guides`.
- Release branches are optional, temporary preparation branches; see [releasing](releasing.md).
- Open a draft PR early when feedback would help.
- Keep unrelated formatting/refactoring separate from behavior changes.
- PR description: problem, resulting behavior, validation, limitations, related issue.
- UI changes: demonstrate the affected operation with a synthetic sample.
- Update descriptions to match the final change before merging.

## Data and publication

- Use `examples/`; never commit `paper/`, ignored captures, credentials, or build output.
- Do not force-add ignored files to make a check pass.
- [Publication checks](testing.md) supplement staged-diff review; they do not replace it.
- Reference: [GitHub flow](https://docs.github.com/ja/get-started/using-github/github-flow).
- Reference: [Small changes](https://google.github.io/eng-practices/review/developer/small-cls.html), [PR descriptions](https://google.github.io/eng-practices/review/developer/cl-descriptions.html).
