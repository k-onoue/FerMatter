# Stable release preparation — unsigned policy

Decision date: 2026-10-29 (Asia/Tokyo). This preparation stops before public release.

| Step | Decision / implementation | Completion evidence |
| --- | --- | --- |
| 1. Define support | macOS arm64 and Windows 11 x64 are candidate targets; Ubuntu 24.04 amd64 remains experimental. macOS 14 is a deployment target, not a verified support claim. | [Platform record](platform-validation.md): exact OS/build/CPU, operations and limitations. Validate the oldest claimed OS or narrow the claim before publication. |
| 2. Permanent ID (#125) | `io.github.fermatter.fermatter`, fixed across versions/channels. Retain existing data, WebView recovery, resident coordination and installer compatibility. | [Migration contract](storage.md#app-identity-migration), migration tests and final-artifact upgrade checks. OS consent cannot be silently transferred. |
| 3. Distribution validation (#130) | Require real compilers, tests, Clippy, builds, dependency audits and publication checks. Reject dirty/different commits and skipped/failed gates. | [Gate reports](release-validation.md), exact candidate commit, bundle/installer hashes. Preview re-signing has an explicit commit-only macOS gate; stable candidates require exact bundle hashes. |
| 4. Unsigned distribution | macOS ad hoc app, no Developer ID/notarization; Windows unsigned. Keep APT repository signatures. Channel and signing status are separate. | [Candidate packaging](packaging.md#unsigned-stable-candidates), recorded signature states, concise per-app opening instructions. No OS security bypass. |
| 5. Candidate validation | Test fresh installation, current-preview upgrade, data/recovery, rendering, UI and external output on synthetic material. | [Final-artifact checklist](release-validation.md#final-artifact-checklist). Native Windows 11, older macOS and Linux desktop results must not be inferred from CI. Unavailable checks stay pending. |
| 6. Publish — deferred | Select final version/build, tag and release notes; publish immutable verified binaries; update site, Homebrew and signed APT. | Separate publication authorization after remaining candidate checks; no new release is implied by this preparation. |

- Current public version/build/download metadata stays pinned until step 6. Local candidate artifacts are not a new published version; never upload changed bytes under an existing version/tag.
- Keep full validation reports and hashes beside each local candidate. A later code change invalidates earlier candidate results.
- Public download and package-manager delivery checks happen during step 6; candidate packaging never creates a release, pushes a tag or modifies the tap/APT repository.
- Track concrete native failures separately from unavailable checks; do not label all of step 5 complete while either remains.
- Remaining native checks and the conditions for entering step 6: [issue #134](https://github.com/k-onoue/FerMatter/issues/134).
