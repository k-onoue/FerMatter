---
title: "Start from pasted equations"
description: "Create a project from equations and a FerMatter template."
sidebar:
  order: 5
---

1. **New Project** (`⇧⌘N`): enter a name and choose LaTeX, Typst, LaTeX + TikZ or Typst + CeTZ.
2. Paste the input and choose a mode.
3. **Create Project** creates the source folder at the selected location.

| Input mode | FerMatter behavior |
| --- | --- |
| Auto | Detects common source markup; otherwise uses one equation per nonempty line |
| One per line | Treats each line as an equation without outer math delimiters |
| Source | Accepts complete markup, including multiline environments and drawings |

## Project location

- **Location → Choose…** overrides **Settings → Projects → Default project folder** for this project.
- **Load File…** reads one file's text. To keep imports and images together, use [Open Project](paper-to-slides.md).

## Switch between projects

Project tabs retain their Library/Source view, source tabs and PDF view. Opening the same project selects its existing tab.

## Use your own setup

- **Edit Template** customizes packages, definitions and compiler settings. Saving a built-in template creates a named copy.
- Keep exactly one `{{content}}` placeholder. Changes affect future projects only.
- Complete LaTeX documents retain their preamble in Auto/Source; the template still selects the compiler.
- Templates do not translate LaTeX and Typst. For drawing support, see [Figures](figures.md).
