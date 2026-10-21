---
title: "MCP and CLI"
description: "Connect a local agent to search, render and export equations."
sidebar:
  order: 12
---

- FerMatter includes a local MCP server and CLI in its macOS app bundle.
- Open the app once, finish setup, configure the [compiler tools](requirements.md), and open any projects you want to use.
- The examples assume `/Applications/FerMatter.app`; adjust the executable path if installed elsewhere.

## Connect through MCP

Use an MCP client that supports local **stdio** servers on this Mac.

| Setting | Value |
| --- | --- |
| Transport | `stdio` |
| Command | `/Applications/FerMatter.app/Contents/MacOS/FerMatter` |
| Arguments | `mcp` |
| Tool timeout | 300 seconds |

- No separate server installation, HTTP address or model API key is needed by FerMatter. Your agent's own account requirements still apply.
- The server connects to the running app, starting it in the background when needed. Installing FerMatter does not change your agent configuration.

### Codex

```sh
codex mcp add fermatter -- /Applications/FerMatter.app/Contents/MacOS/FerMatter mcp
codex mcp list
```

In `~/.codex/config.toml`, add `tool_timeout_sec = 300` to the generated server entry. Alternatively, add the complete entry directly:

```toml
[mcp_servers.fermatter]
command = "/Applications/FerMatter.app/Contents/MacOS/FerMatter"
args = ["mcp"]
tool_timeout_sec = 300
```

- Configuration reference: [official OpenAI documentation](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).

### Claude Code

```sh
claude mcp add --transport stdio fermatter -- /Applications/FerMatter.app/Contents/MacOS/FerMatter mcp
claude mcp list
```

- Configuration and scope options: [Claude Code MCP documentation](https://code.claude.com/docs/en/mcp).
- Older configurations can keep the server name `mathlidelux`; update only the executable path from `Mathlidelux.app/Contents/MacOS/Mathlidelux` to `FerMatter.app/Contents/MacOS/FerMatter`.

## Available operations

| Tool | Use |
| --- | --- |
| `list_projects` | Find registered project IDs and paths. |
| `list_templates` | Find LaTeX, Typst, TikZ and CeTZ template IDs. |
| `search_equations` | Search a project's saved Library. |
| `get_equation` | Read an equation's saved source and revision. |
| `render_equation` | Render supplied source with a project or template's context. |
| `export_equations` | Export 1–64 saved equations to SVG, PNG, PDF or JPEG. |
| `create_variant` | Add a named Library variant; preserve the original source. |
| `open_in_app` | Open a FerMatter SVG for editing. |

- Example request: “Use the demo-paper project to find the least-squares equations. Export them as SVG at 30 pt into `/tmp`, inspect the previews, and report their dimensions.”
- Math input is the equation body without `$` or `\[...\]`; figures use full TikZ/CeTZ code.
- `outputDirectory` must already exist and be outside registered source folders. Exports create a new `mathlidelux-*` subfolder with `manifest.json`.
- Check every returned item's `status`. Successful items include `path`, `previewPath`, `widthPt` and `heightPt`; batch results can contain failures.

## Use the CLI

```sh
APP=/Applications/FerMatter.app/Contents/MacOS/FerMatter
"$APP" cli tools
"$APP" cli list_projects
"$APP" cli render_equation --input - <<'JSON'
{"language":"latex","source":"\\frac{a}{b}","fontSize":30,"format":"svg","outputDirectory":"/tmp"}
JSON
```

- `cli tools` lists complete input schemas; MCP clients can use `tools/list`.
- stdout contains one JSON `result` or `error`; stderr carries progress and diagnostics. Command errors return a nonzero exit status.
- To read input from a file, use `cli render_equation --input request.json`.

## Saved data and troubleshooting

- Search reads the saved Library; rendering uses saved project files. Save Source edits before asking an agent to use them.
- `create_variant` adds a presentation variant. Take `expectedRevision` from a fresh search/get result and reuse the same `requestId` and arguments when retrying an uncertain addition. On `project_conflict`, reread before trying again.
- `restart_required`: quit the older running app, open the updated FerMatter, and reconnect the client.
- Empty project results: open the project in FerMatter first. Compiler failures: review **Settings → Tools** and the returned diagnostic.
- This version has no source-editing, project-deletion or settings-write tools. Rendering/export keeps manuscript files and Instant drafts intact; editable SVG exports include their source metadata.
