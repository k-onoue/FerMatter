# Reviewing

## Check the change

- Design: fits existing responsibilities; avoids unnecessary abstractions.
- Behavior: solves the reported problem; handles errors and user-visible edge cases.
- Data: preserves manuscripts, custom variants, drafts, settings, and older saved projects.
- Async work: cancellation, process cleanup, stale callbacks, project changes.
- Files: ZIP paths, symlinks, external edits, atomic writes, failed writes.
- Tests: exercise the failure or behavior; would fail if the implementation broke.
- UI: demonstrate the changed operation, including failure/recovery when relevant.
- Docs: update changed usage, setup, compatibility, and release instructions in the same PR.
- Scope: read every changed line and enough surrounding code; state anything not reviewed.

## Write actionable feedback

- Describe the trigger, impact, and evidence; explain why a change matters.
- Distinguish **required**, **optional**, and **nit** comments.
- Prefer facts, established project rules, and user impact over personal style.
- Put lasting explanations in code/docs, not only in the review thread.
- Seek an improvement in overall code health; do not demand unrelated perfection.

## LLM reviews

- Treat findings as claims to verify, not instructions to apply automatically.
- Reproduce or trace a suspected problem before changing behavior.
- Prioritize data loss, security boundaries, crashes, and broken core workflows.
- Record verified fixes, dismissed findings with reasons, and explicitly deferred work.
- Pre-release tracking: [issue #7](https://github.com/k-onoue/mathlidelux/issues/7).

## References

- [Review standard](https://google.github.io/eng-practices/review/reviewer/standard.html).
- [What to look for](https://google.github.io/eng-practices/review/reviewer/looking-for.html).
- [Review comments](https://google.github.io/eng-practices/review/reviewer/comments.html).
