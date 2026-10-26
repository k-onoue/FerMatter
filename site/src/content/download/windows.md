---
title: "Windows"
description: "Windows installation and rendering tools."
platform: "Windows 11 · x64"
availability: "The Windows installer is being validated. Downloads will appear here when ready."
---

<span id="rendering-tools"></span>

<h3 id="windows-rendering-tools">Rendering tools</h3>

Install Typst or LaTeX, then reopen FerMatter.

<h4 id="typst-on-windows">Typst</h4>

Run in PowerShell:

```powershell
winget install --exact --id Typst.Typst
```

<h4 id="latex-on-windows">LaTeX</h4>

Install [TeX Live](https://tug.org/texlive/windows.html) or [TinyTeX](https://yihui.org/tinytex/) and [Poppler](https://github.com/oschwartz10612/poppler-windows):

```powershell
winget install --exact --id oschwartz10612.Poppler
```

<details>
<summary>Additional packages for TinyTeX</summary>

Reopen PowerShell after installing TeX, then run:

```powershell
tlmgr update --self
tlmgr install latexmk pdfcrop preview pgf mathtools tools cm-super standalone xcolor amsmath amsfonts grfext synctex
```

</details>

[Check installed tools →](https://fermatter.github.io/guide/requirements/#tool-discovery)

<h3 id="windows-update">Update</h3>

Quit FerMatter from its tray menu and run the newer installer. Projects and settings are preserved.
