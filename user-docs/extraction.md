---
title: "Card extraction"
description: "Exclude tables and custom environments from your Library."
---

Configure which equations and diagrams are collected into your Library.

## Ignore tables

In **Settings → Extraction**, toggle **Ignore math in tables** and click **Apply and rescan**.

The default is off. These settings affect Library cards, not source files or the full paper PDF.

| Format | Excluded containers |
| --- | --- |
| **LaTeX** | `table`, `table*`, `tabular`, `tabular*`, `tabularx`, `longtable` environments |
| **Typst** | `table(...)` function calls and their contents |

Existing presentation variants are preserved. Disabling this filter restores original cards.

<span id="customize-exclusions"></span>

## Custom exclusions

Edit `extraction.json` directly via **Edit JSON** in settings:

```json
{
  "schemaVersion": 1,
  "kind": "mathlidelux.extraction",
  "ignoreMathInTables": true,
  "excludedLatexEnvironments": ["results", "tabu"],
  "excludedTypstFunctions": ["grid", "paper.results"]
}
```

| Field | Purpose |
| --- | --- |
| `ignoreMathInTables` | Toggles default table exclusions |
| `excludedLatexEnvironments` | Environment names to omit (matches `\begin{name}...\end{name}`) |
| `excludedTypstFunctions` | Function identifiers to omit (e.g. `grid`, `module.func`) |

- Environment and function names are exact and case-sensitive.
- Omit prefixes like `\begin` or `#`.
- Custom lists apply even when **Ignore math in tables** is off. Names are literal; aliases, wildcards and dynamically constructed calls are not resolved.
- Click **Use configuration** followed by **Apply and rescan** to apply changes.
- After external edits, choose **Reload**. Invalid JSON leaves the last valid settings active.

<span id="extraction-boundaries"></span>

Exclusions only filter literal cards that FerMatter can extract. See [Extraction scope](requirements.md#extraction-limitations).
