# Open PR snapshot — 2026-09-28

- Baseline: `main` at `8daaca4`; eight open PRs when inspected.
- All eight required review and had no reported check runs at inspection.
- Read PR descriptions, changed-file lists, and relevant implementation/docs; this is a scope inventory, not a full code review.
- Refresh live status with `gh pr list`; this page is dated history.
- Local `dev/prototype-integration` includes feature work but is not the default/released branch.

| PR | Scope | Documentation consequence |
| --- | --- | --- |
| [#3](https://github.com/k-onoue/FerMatter/pull/3) | Contributor guide, reviewed release workflow | This reorganization incorporates that intent; reconcile overlapping README/CONTRIBUTING/PUBLISHING edits before merging both |
| [#4](https://github.com/k-onoue/FerMatter/pull/4) | Remove old browser UI prototype | Do not reintroduce prototype instructions; removal remains in that PR |
| [#5](https://github.com/k-onoue/FerMatter/pull/5) | New projects in Documents; exact Finder destination | Update default path after merge; retain existing roots, no implicit migration |
| [#6](https://github.com/k-onoue/FerMatter/pull/6) | Approved app/site icon | Use the same approved PNG; preserve its provenance |
| [#8](https://github.com/k-onoue/FerMatter/pull/8) | Versioned shortcut/theme JSON, recovery | [Settings contract](../reference/settings-files.md); schema details belong outside public homepage |
| [#9](https://github.com/k-onoue/FerMatter/pull/9) | Stable update checks and accepted downloads | [Update contract](../reference/updates.md); user installs manually; requires public release access |
| [#10](https://github.com/k-onoue/FerMatter/pull/10) | Prototype DMG/PKG and checksums | [Packaging](../reference/packaging.md); unnotarized artifacts remain separate from public distribution |
| [#11](https://github.com/k-onoue/FerMatter/pull/11) | Caret equation previews from unsaved drafts | Add source-preview tutorial after integration; document snapshot isolation, UTF-16 mapping, and stale-result cancellation |

## Integration follow-up

- This docs PR contains no app implementations from the open feature PRs.
- Preserve the short legacy entry points when reconciling feature PR documentation changes.
- Move merged specifications into `dev-docs/reference/`; replace old paths with links.
- Update architecture, storage paths, user guides, and CHANGELOG in the integration PR.
- Replace proposal wording when the feature lands; keep this historical snapshot unchanged.
- Rebuild generated guides and rerun publication checks after conflict resolution.
- Site remains a prototype guide; no public installer or automatic-update availability is claimed.
- Release audit remains [issue #7](https://github.com/k-onoue/FerMatter/issues/7).
