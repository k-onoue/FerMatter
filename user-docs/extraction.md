---
title: "Card extraction"
description: "Exclude tables or custom source environments from your Library."
---

**Coming in the next release.** These settings are not included in 0.11.0 Preview 4.

## Ignore tables

1. Open **Settings → Extraction**.
2. Enable **Ignore math in tables**.
3. Choose **Apply and rescan**. All open project libraries refresh; closed projects refresh when opened.

The default is off. This option excludes cards inside these containers:

| Format | Containers |
| --- | --- |
| LaTeX | `table`, `table*`, `tabular`, `tabular*`, `tabularx`, `longtable` environments |
| Typst | Literal `table(...)` calls, including their content blocks |

Equations outside these containers keep their identities. Source files, Source previews and the full paper PDF stay unchanged. Saved presentation variants remain in the Library; disabling an exclusion restores source cards and their saved names and font sizes.

## Customize exclusions

Choose **Edit JSON** to change `extraction.json`, or **Reveal settings folder** to edit the file externally. Settings apply to all projects.

```json
{
  "schemaVersion": 1,
  "kind": "mathlidelux.extraction",
  "ignoreMathInTables": true,
  "excludedLatexEnvironments": ["results", "tabu"],
  "excludedTypstFunctions": ["grid", "paper.results"]
}
```

| Field | Effect |
| --- | --- |
| `ignoreMathInTables` | Enables the table containers listed above |
| `excludedLatexEnvironments` | Additional `\begin{name}…\end{name}` environments to exclude, including nested content and included files |
| `excludedTypstFunctions` | Additional literal function calls to exclude, including nested content |

- Names are exact and case-sensitive; omit `\begin`, `#`, parentheses and arguments. No regular expressions or wildcards. A trailing `*` in a LaTeX name is literal.
- Custom lists apply even when **Ignore math in tables** is off. Empty lists add no custom exclusions.
- Qualified Typst names use their written spelling, such as `paper.results`. Aliases and dynamically constructed calls are not resolved.
- Each list accepts up to 128 names, each at most 128 ASCII characters. LaTeX names accept letters, digits, `_`, `*`, `@`, `:`, `-`; Typst names use dot-separated identifiers containing letters, digits, `_`, `-`, starting with a letter or `_`.

In the built-in editor, choose **Use configuration**, then **Apply and rescan**. After external edits, choose **Reload**. Import loads JSON into the editor for review; Apply saves it to the active settings file. Export saves a reusable copy.

Invalid JSON or names leave the last valid settings in use and preserve the original file. Repair the external file and Reload before saving. If another window or editor changed the file, Reload before applying your changes.

## Extraction boundaries

Exclusions filter literal cards that FerMatter can already extract; they do not evaluate macros, Typst functions or loops. Typst math inside unsupported calls may already be absent when table filtering is off. See [Extraction limitations](requirements.md#extraction-limitations).
