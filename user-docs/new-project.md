# Start from pasted equations

1. Choose **New Project** (`⇧⌘N`) and enter a name.
2. Choose **LaTeX**, **Typst**, **LaTeX + TikZ**, or **Typst + CeTZ**.
3. Paste your text; choose Auto, One per line, or Source.
4. Check the destination using **Change…**, then choose **Create Project**.

| Input mode | Content |
| --- | --- |
| Auto | Detects common source markup; otherwise treats each nonempty line as an equation |
| One per line | One complete equation per line, without outer math delimiters |
| Source | Full source markup, headings, multiline environments, or drawings |

## Project location

- New projects default to `~/Documents/Mathlidelux`; **Change…** selects another parent folder.
- Existing projects retain their saved locations; opening a folder uses it in place.
- **Show in Finder** on a project opens its source folder.

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
- **Settings → Templates → Customize…** creates an editable copy of a built-in template.
- Add packages/macros/fonts; keep exactly one `{{content}}` placeholder; choose **Save Template**.
- LaTeX templates can choose pdfLaTeX, XeLaTeX, or LuaLaTeX.
- Template changes affect future projects; existing sources stay unchanged.
- Templates do not translate between LaTeX and Typst.
- A complete LaTeX document keeps its preamble in Auto/Source; the template still selects the compiler.
- Built-in drawing templates load TikZ libraries or CeTZ 0.5.2; [drawing requirements](figures.md).
