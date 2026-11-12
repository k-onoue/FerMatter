# Reviewing

## Review checklist

- **Design:** Aligns with existing module boundaries and invariants without gratuitous abstractions.
- **Safety:** Preserves manuscripts, presentation variants, drafts, and user preferences. Real files are only modified in Source mode.
- **Async & Cancellation:** Superseded compile/render tasks kill running process groups. Stale callbacks are ignored.
- **File Integrity:** Atomic writes, conflict detection on external changes, safe directory isolation.
- **Tests:** Includes reproducible tests exercising new behavior or edge cases.
- **Documentation:** Matching updates to `user-docs/` or `dev-docs/` in the same PR.

## Providing actionable feedback

- State impact and evidence clearly.
- Categorize comments: **[Required]**, **[Optional]**, or **[Nit]**.
- Verify external findings or LLM suggestions with tests before modifying core behavior.

## References

- [Google Code Review Standard](https://google.github.io/eng-practices/review/reviewer/standard.html)
