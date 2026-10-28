# Development docs

- Shared by people and agents; Markdown in this repository is the source of truth.
- Start with the task below; reading every page is not required.
- User instructions live in [user-docs](../user-docs/README.md); public output lives in `site/dist/`.
- Agent entry point: [AGENTS.md](../AGENTS.md).

| Task | Document |
| --- | --- |
| Build and maintain | [Development](development.md) |
| Work on the desktop app | [Desktop implementation](../desktop-app/README.md) |
| Find the responsible module | [Architecture](architecture.md) |
| Choose validation | [Testing](testing.md) |
| Review a PR or LLM finding | [Reviewing](reviewing.md) |
| Version, tag, package, release | [Releasing](releasing.md) |
| Edit guides, website, media | [Documentation](documentation.md) |
| Preview the website | [Build and preview](documentation.md#build-and-preview) |

## Repository layout

| Path (from the repository root) | Purpose |
| --- | --- |
| `desktop-app/` | [Desktop app](../desktop-app/README.md): Rust/Tauri, React/TypeScript, tests, resources and packaging |
| `dev-docs/` | Shared development instructions and references |
| `user-docs/` | Markdown sources for the public guide |
| `site/` | Astro/Starlight website and approved demo media |
| `docs/downloads/` | Sample ZIPs shared by the app and website |
| `examples/` | Synthetic sample projects |
| `scripts/` | Site checks, download statistics, sample packaging |

## Reference

- [Local automation: CLI and MCP](reference/automation.md).
- [Storage and compatibility](reference/storage.md).
- [Platform validation record](reference/platform-validation.md), [unsigned stable preparation](reference/stable-readiness.md).
- [SVG restoration and resident Instant](reference/svg-recipes.md).
- [Settings-file schemas](reference/settings-files.md).
- [Update contract](reference/updates.md).
- [Prototype packaging](reference/packaging.md).
- [Local signing and macOS folder access](reference/signing.md).
- [Documentation structure decision](decisions/0001-documentation.md).
- [Open PR snapshot, 2026-09-28](history/pr-snapshot-2026-09-28.md): scope, overlap, and integration follow-up.
- [Earlier validation](history/validation-2026-09.md): historical evidence, not a current release pass.

## Original references

- [Semantic Versioning](https://semver.org/lang/ja/).
- [Keep a Changelog](https://keepachangelog.com/ja/1.0.0/).
- [Git tags](https://git-scm.com/book/ja/v2/Git-%E3%81%AE%E5%9F%BA%E6%9C%AC-%E3%82%BF%E3%82%B0).
- [GitHub flow](https://docs.github.com/ja/get-started/using-github/github-flow).
- [GitHub Releases](https://docs.github.com/ja/repositories/releasing-projects-on-github/about-releases).
- [Google Engineering Practices](https://google.github.io/eng-practices/).
