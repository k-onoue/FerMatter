# FerMatter

It is well known that Fermat never wrote down his proof because his paper had been accepted at a conference—and he dreaded making the slides. With FerMatter, he’d gladly fill in the missing proof.

[![DMG downloads](https://img.shields.io/github/downloads/FerMatter/fermatter.github.io/FerMatter.dmg?label=downloads&displayAssetName=false)](https://github.com/FerMatter/fermatter.github.io/releases)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="site/public/assets/fermat-dark.svg">
  <img src="site/public/assets/fermat-light.svg" width="96" height="96" alt="FerMatter — Pierre de Fermat">
</picture>

- Bring equations from a LaTeX or Typst paper into your presentation.
- Browse equations and TikZ/CeTZ drawings; edit slide-specific variants; copy/export SVG, PDF, PNG, or JPEG.
- Unnotarized public preview for **Apple Silicon Mac** (macOS 14+ build target; verified on macOS 26): [DMG and Homebrew installation](https://fermatter.github.io/download/).
- No Python or development SDK is needed to run the app. Compiler tools remain external: [requirements](user-docs/requirements.md).

![FerMatter Library with a synthetic sample](site/public/assets/demos/library.png)

## Start here

| Task | Read |
| --- | --- |
| Use the app | [Short user guides](user-docs/README.md) · [public website](https://fermatter.github.io/) |
| Development and feedback | [Development docs](dev-docs/README.md) · [Feedback policy](CONTRIBUTING.md) |
| Work with an agent | [MCP and CLI guide](user-docs/automation.md) · [Development API reference](dev-docs/reference/automation.md) · [AGENTS.md](AGENTS.md) |
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
| `site/` | Astro/Starlight website and approved demo media |
| `docs/downloads/` | Sample ZIPs shared by the app and website |
| `examples/` | Synthetic sample projects |
| `scripts/` | Site checks, download statistics, sample packaging |

## Preview the product page

```sh
npm --prefix scripts ci
npm --prefix site ci
npm --prefix site run build
node scripts/preview-site.mjs
```

- Open [localhost:4174](http://localhost:4174/); Node.js 22.22+.
- Edit and publish: [documentation workflow](dev-docs/documentation.md).
- Only the static `site/dist/` output is served.

## License

- [Apache License 2.0](LICENSE); copyright and attribution: [NOTICE](NOTICE).
- The license does not grant trademark permission; do not distribute modified builds under the FerMatter name or icon.
- External tools retain their own licenses and are not bundled.
