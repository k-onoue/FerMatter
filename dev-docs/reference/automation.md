# Local automation: CLI and MCP

- macOS: the app bundle includes the CLI and a local **stdio MCP** server.
- Use from Codex, Claude Code, shell scripts, or other MCP clients; no model API key or HTTP service.
- Shared Rust compiler, settings, SVG recipes and Library storage; the resident app owns writes.
- The CLI starts a hidden resident when needed. An older running build returns `restart_required`; quit it and open the updated app.
- Open the app once to complete setup, configure [compiler paths](settings-files.md), and register projects.

## Connect an agent

- [Public MCP and CLI guide](../../user-docs/automation.md): client configuration, operations, examples and troubleshooting.
- Build checkout: `desktop-app/src-tauri/target/release/bundle/macos/FerMatter.app/Contents/MacOS/FerMatter`.
- Allow up to 300 seconds per tool call in the client's MCP configuration; compiler progress is forwarded when a progress token is supplied.
- Existing MCP configurations: update the executable path from `Mathlidelux.app/Contents/MacOS/Mathlidelux` to `FerMatter.app/Contents/MacOS/FerMatter`; keep the server key and tool names.
- Connecting is an explicit user setup step; installing FerMatter does not modify agent configuration.

## Tools

| Tool | Input / result |
| --- | --- |
| `list_projects` | Registered project IDs, paths, language, equation counts; `offset`, `limit` |
| `list_templates` | Instant template IDs, languages and sources; includes TikZ/CeTZ; `offset`, `limit` |
| `search_equations` | `projectId`, optional `query`/`kind`; saved Library matches and revision |
| `get_equation` | `projectId`, `equationId`; complete saved equation and revision |
| `render_equation` | `source`, existing absolute `outputDirectory`; optional `projectId` or `templateId`, `language`, `kind`, `contextFile`, `fontSize`, `color`, `format` |
| `export_equations` | `projectId`, 1–64 unique `equationIds`, `outputDirectory`; optional `fontSize`, `format` |
| `create_variant` | `projectId`, base `equationId`, `name`, `source`, `expectedRevision`, `requestId`; optional `fontSize`, `color` |
| `open_in_app` | Absolute `path` to a FerMatter SVG; queues the normal editable import flow |

- Full JSON input schemas: `FerMatter cli tools` or MCP `tools/list`.
- Unknown fields, unknown formats and invalid values return structured errors.
- Pagination: default 50, maximum 200. Query matches equation name, source, file, or outline title.
- Standalone language defaults to LaTeX; project/template language is inferred. A supplied language must match.
- Math input is the body without `$` / `\[...\]`; figures contain their full TikZ/CeTZ code.
- Format defaults to SVG; other formats: PNG, PDF, JPEG.
- Explicit size: 1–200 pt, continuous. Omitted size inherits card → project → app settings (new default: 30 pt); a saved “source size” setting remains supported. Standalone template/color defaults reuse Instant settings.
- Typst `contextFile` can choose definitions from a nested project file.

## CLI examples

```sh
APP=/Applications/FerMatter.app/Contents/MacOS/FerMatter
"$APP" cli list_projects
"$APP" cli list_templates
"$APP" cli render_equation --input - <<'JSON'
{"language":"latex","source":"\\frac{a}{b}","fontSize":30,"format":"svg","outputDirectory":"/tmp"}
JSON
"$APP" cli render_equation --input request.json
```

- stdout: one JSON `{ "result": ... }` or `{ "error": { "code": ..., "message": ... } }`.
- stderr: progress / diagnostics. CLI exit status is nonzero on command failure.
- Batch exports can return a mix of `ready` and `failed` items; inspect every item's status.
- Each successful output has `path`, `previewPath`, `format`, `fontSize`, `widthPt`, `heightPt`.
- Outputs live in a new `mathlidelux-*` child directory with `manifest.json`; no overwrite, independent of cache cleanup. Delete these exported folders when no longer needed.
- Output parent must already exist and be outside registered source folders, including symlink aliases.
- SVG carries editable source/project metadata. `open_in_app` restores it into the matching workspace/Instant using the existing draft-preserving import flow; `queued` is not proof of human review.

## Consistency and limits

- Search/get use the **saved Library**. Rendering uses supplied/saved equation text and **saved project files**; unsaved Source edits are excluded.
- Rendering and export leave manuscript contents and Instant drafts/history unchanged.
- `create_variant` appends a Library variant; it does not edit the original card or project files.
- Take `expectedRevision` from a fresh search/get. `project_conflict` means reread and reconsider the change.
- Generate a stable `requestId` for each intended addition. On uncertain completion, retry the same arguments and ID; changing arguments with that ID returns `request_id_conflict`. A deleted variant may be recreated by a later deliberate retry.
- GUI merges new API cards without replacing drafts or source buffers. Older queued saves preserve unseen API additions; previously seen cards remain deletable.
- The resident uses a private per-user socket; no listening network port. Only same-user local processes can connect. This is not a sandbox for untrusted TeX or project build configuration.
- MCP cancellation, client disconnect and the 300-second deadline cancel compiler jobs. An already committed operation may have completed before disconnect; use idempotency for additions.
- Protocol limits: 1 MiB request, 16 MiB response, 16 concurrent local connections, 64 equations per export.
- No source-editing, project deletion, settings-write or remote-service tools in this version.
- Transport is currently Unix-only; Windows needs a named-pipe adapter. This does not establish Ubuntu GUI/compiler support.

## Implementation and validation

- [`automation/`](../../desktop-app/src-tauri/src/automation/): typed schemas/service, CLI, MCP adapter, resident transport.
- CLI and MCP connect to the same running app; they never construct an independent `Storage` writer.
- Shared compiler cancellation and cache leases; API outputs copied outside the cache before returning.
- GUI/project snapshots carry `automationGeneration`; API cards also store their birth generation and request hash. Old project metadata needs no migration; missing generation is zero.
- `MATHLIDELUX_DATA_DIR` also isolates the resident identity for disposable verification.
- `check.sh` runs service, schema, conflict/merge and real compiler tests with synthetic data.
- After building: `node desktop-app/scripts/verify-automation.mjs /absolute/path/to/FerMatter` checks actual CLI/MCP/resident communication in isolated storage.

## References

- [Codex MCP](https://learn.chatgpt.com/docs/extend/mcp?surface=cli).
- [Claude Code MCP](https://code.claude.com/docs/en/mcp).
- [Official Rust MCP SDK](https://github.com/modelcontextprotocol/rust-sdk).
