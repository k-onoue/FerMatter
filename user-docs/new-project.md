---
title: "Start from pasted equations"
description: "Create a LaTeX or Typst project from text and a template."
sidebar:
  order: 5
---

1. Choose **New Project** (`⇧⌘N`) and enter a name.
2. Choose **LaTeX**, **Typst**, **LaTeX + TikZ**, or **Typst + CeTZ**.
3. Paste your text; choose Auto, One per line, or Source.
4. Choose **Create Project**; the source folder is created in the selected location.

| Input mode | Content |
| --- | --- |
| Auto | Detects common source markup; otherwise treats each nonempty line as an equation |
| One per line | One complete equation per line, without outer math delimiters |
| Source | Full source markup, headings, multiline environments, or drawings |

## Project location

- First launch requires choosing a project folder; `~/Documents/Mathlidelux` is the suggested location. All four demos are placed under its **Demos** subfolder.
- **Settings → Projects → Default project folder** changes the location for future new projects, dropped folders/ZIPs, and duplicates.
- **New Project → Location → Choose…** overrides the folder for this project only; **Default** returns to the saved default.
- Existing projects retain their saved locations; opening a folder uses it in place.
- **Reveal in Finder** on a project opens its source folder.

## Switch between projects

- Open projects appear as tabs above the workspace; **Projects** returns to the project list.
- Opening the same project selects its existing tab.
- Each tab retains its Library/Source view, open source files and PDF view while you switch.
- Close a project tab with its **×** button. Source file tabs remain inside that project's Source view.

## Try two equations

- LaTeX, **One per line**:

```tex
x^2 + y^2 = r^2
\int_0^1 x^2\,dx = \frac{1}{3}
```

- Typst, **One per line**:

```typst
x^2 + y^2 = r^2
integral_0^1 x^2 dif x = 1/3
```

## Use your own setup

- **Load File…** reads a UTF-8 `.tex`, `.typ`, or `.txt` file into the text area; it does not modify the original or copy companion files.
- To keep imports/images together, use [Open Project](paper-to-slides.md) instead.
- In **New Project**, choose **Edit Template** to customize a template. Saving a built-in creates a named copy.
- Add packages/macros/fonts; keep exactly one `{{content}}` placeholder; choose **Save Template**.
- LaTeX templates can choose pdfLaTeX, XeLaTeX, or LuaLaTeX.
- Template changes affect future projects; existing sources stay unchanged.
- Templates do not translate between LaTeX and Typst.
- A complete LaTeX document keeps its preamble in Auto/Source; the template still selects the compiler.
- Built-in drawing templates load TikZ libraries or CeTZ 0.5.2; [drawing requirements](figures.md).
