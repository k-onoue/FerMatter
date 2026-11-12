# Development docs

Developer guidelines and architectural references for humans and agents. Markdown files in this repository serve as the source of truth.

- User-facing guides: [`user-docs/`](../user-docs/README.md)
- Agent instructions & invariants: [`AGENTS.md`](../AGENTS.md)

## Guides

| Topic | Document |
| --- | --- |
| **Setup & Workflow** | [Development](development.md) |
| **Desktop App** | [Desktop App Overview](../desktop-app/README.md) |
| **Architecture** | [Architecture & Responsibilities](architecture.md) |
| **Testing** | [Testing & Verification](testing.md) |
| **Code Review** | [Reviewing Standards](reviewing.md) |
| **Releasing** | [Versioning & Release Checklist](releasing.md) |
| **Site & Documentation** | [Documentation & Website Guide](documentation.md) |

## Repository layout

| Path | Contents |
| --- | --- |
| `desktop-app/` | Rust (Tauri 2) and React/TypeScript desktop application |
| `dev-docs/` | Development, architecture, and release documentation |
| `user-docs/` | Markdown sources for public guides |
| `site/` | Astro / Starlight website and public media assets |
| `examples/` | Synthetic demo projects (LaTeX, Typst, TikZ, CeTZ) |
| `scripts/` | Build validation, site checks, and packaging scripts |

## Reference

- [Automation: CLI and MCP](reference/automation.md)
- [Storage, metadata & compatibility](reference/storage.md)
- [Platform validation record](reference/platform-validation.md) · [Unsigned stable preparation](reference/stable-readiness.md)
- [SVG restoration recipes](reference/svg-recipes.md)
- [Settings file schemas](reference/settings-files.md)
- [Update mechanisms](reference/updates.md)
- [Packaging and installer verification](reference/packaging.md)
- [Local signing & permissions](reference/signing.md)
