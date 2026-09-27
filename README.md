# Mathlidelux

- Bring equations from a LaTeX or Typst paper into your presentation.
- Browse equations and TikZ/CeTZ drawings; edit slide-specific variants; copy/export SVG, PDF, PNG, or JPEG.
- Development prototype for **macOS 26+ / Apple Silicon**; no signed, notarized public download yet.
- Python and compiler tools are external: [requirements](user-docs/requirements.md).

![Mathlidelux with a synthetic sample](docs/assets/app-light.jpg)

## Start here

| Task | Read |
| --- | --- |
| Use the app | [Short user guides](user-docs/README.md) · [public website](https://k-onoue.github.io/mathlidelux/) |
| Build or contribute | [Development docs](dev-docs/README.md) · [CONTRIBUTING.md](CONTRIBUTING.md) |
| Work with an agent | [AGENTS.md](AGENTS.md) |
| Prepare a release | [Release checklist](dev-docs/releasing.md) · [CHANGELOG.md](CHANGELOG.md) |
| Edit the website or docs | [Documentation workflow](dev-docs/documentation.md) |

## Try a sample

- Choose **Open Project** and select a folder/ZIP; existing projects need no template.
- Samples: [LaTeX](examples/demo-paper/), [Typst](examples/demo-typst/), [TikZ](examples/demo-tikz/), [CeTZ](examples/demo-cetz/).
- Pasted equations: **New Project** → choose a template → paste → **Create Project**.
- Library variants leave manuscript source unchanged; **Source** mode edits real project files.

## Repository layout

| Path | Purpose |
| --- | --- |
| `macos-app/` | Native app, workers, tests, build tooling |
| `dev-docs/` | Shared development instructions and references |
| `user-docs/` | Markdown sources for the public guide |
| `docs/` | Public Pages site, generated guides, approved media, sample ZIPs |
| `examples/` | Synthetic sample projects |
| `scripts/` | Guide generation, sample packaging, publication checks |

## Preview the product page

```sh
python3 -m http.server 4174 --bind 127.0.0.1 --directory docs
```

- Open [localhost:4174](http://localhost:4174); no install needed to preview checked-in pages.
- After editing guide Markdown: [rebuild guides](dev-docs/documentation.md#build-and-preview).
- Only `docs/` is served; keep private manuscripts and local builds outside it.

## License

- [Apache License 2.0](LICENSE); copyright and attribution: [NOTICE](NOTICE).
- The license does not grant trademark permission; do not distribute modified builds under the Mathlidelux name or icon.
- External tools retain their own licenses and are not bundled.
