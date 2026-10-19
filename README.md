# FerMatter

- Bring equations from a LaTeX or Typst paper into your presentation.
- Browse equations and TikZ/CeTZ drawings; edit slide-specific variants; copy/export SVG, PDF, PNG, or JPEG.
- Development prototype for **Apple Silicon Mac** (macOS 14+ build target; verified on macOS 26); no signed, notarized public download yet.
- No Python or development SDK is needed to run the app. Compiler tools remain external: [requirements](user-docs/requirements.md).

![Earlier Mathlidelux workflow with a synthetic sample](docs/assets/app-light.jpg)

*Workflow image from the earlier Swift prototype; the current interface differs.*

## Start here

| Task | Read |
| --- | --- |
| Use the app | [Short user guides](user-docs/README.md) · [public website](https://k-onoue.github.io/mathlidelux/) |
| Build or contribute | [Development docs](dev-docs/README.md) · [CONTRIBUTING.md](CONTRIBUTING.md) |
| Work with an agent | [App automation (CLI / MCP)](dev-docs/reference/automation.md) · [AGENTS.md](AGENTS.md) |
| Prepare a release | [Release checklist](dev-docs/releasing.md) · [CHANGELOG.md](CHANGELOG.md) |
| Edit the website or docs | [Documentation workflow](dev-docs/documentation.md) |

## Try a sample

- Choose **Open Project** for a folder or **Open file or ZIP** for an archive; existing projects need no template.
- Samples: [LaTeX](examples/demo-paper/), [Typst](examples/demo-typst/), [TikZ](examples/demo-tikz/), [CeTZ](examples/demo-cetz/).
- Pasted equations: **New Project** → choose a template → paste → **Create Project**.
- Single equations: [Instant](user-docs/instant.md); open it inside a project to reuse its packages and commands.
- Library variants leave manuscript source unchanged; **Source** mode edits real project files.

## Repository layout

| Path | Purpose |
| --- | --- |
| `desktop-app/` | [Desktop app](desktop-app/README.md): Rust/Tauri, React/TypeScript, tests, resources and packaging |
| `dev-docs/` | Shared development instructions and references |
| `user-docs/` | Markdown sources for the public guide |
| `docs/` | Public Pages site, generated guides, approved media, sample ZIPs |
| `examples/` | Synthetic sample projects |
| `scripts/` | Guide generation, sample packaging, publication checks |

## Preview the product page

```sh
node scripts/preview-site.mjs
```

- Open [localhost:4174](http://localhost:4174); Node.js 22+; no npm install needed to preview checked-in pages.
- After editing guide Markdown: [rebuild guides](dev-docs/documentation.md#build-and-preview).
- Only `docs/` is served; keep private manuscripts and local builds outside it.

## License

- [Apache License 2.0](LICENSE); copyright and attribution: [NOTICE](NOTICE).
- The license does not grant trademark permission; do not distribute modified builds under the FerMatter name or icon.
- External tools retain their own licenses and are not bundled.
