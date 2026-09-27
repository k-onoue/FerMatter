# Development

## Setup

- Apple Silicon Mac; macOS 26+; Xcode with the macOS 26 SDK.
- Install Python and the tools for the selected format: [requirements](../user-docs/requirements.md).
- Run from the repository root:

```sh
./macos-app/scripts/build.sh
open macos-app/build/Mathlidelux.app
```

- Output: an ad hoc signed local app; no Developer ID signing or notarization.
- `DEVELOPER_DIR` overrides Xcode selection; the script otherwise uses `/Applications/Xcode.app` when present.
- App icon provenance: [AppIcon.md](../macos-app/Resources/AppIcon.md).

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
