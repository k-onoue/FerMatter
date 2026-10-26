---
title: "For Windows (preview)"
description: "Windows preview installation and rendering tools."
platform: "Windows 11 · x64. ARM64 and 32-bit installers are not included."
availability: "The Windows installer is being validated. Downloads will appear here when ready."
---

### Typst on Windows

Run in PowerShell, then reopen FerMatter:

```powershell
winget install --exact --id Typst.Typst
```

[Typst installation instructions](https://github.com/typst/typst#installation).

### LaTeX on Windows

Install [TeX Live](https://tug.org/texlive/windows.html) or [TinyTeX](https://yihui.org/tinytex/), then reopen PowerShell and install the packages used by the included demos:

```powershell
tlmgr update --self
tlmgr install latexmk pdfcrop preview pgf mathtools tools cm-super standalone xcolor amsmath amsfonts grfext synctex
```

TinyTeX and TeX Live include the Windows Perl and Ghostscript helpers used by LaTeX rendering; separate installations are unnecessary for this setup.

Install the [Windows Poppler package](https://github.com/oschwartz10612/poppler-windows) in PowerShell:

```powershell
winget install --exact --id oschwartz10612.Poppler
```

Reopen FerMatter and check [Settings → Tools](https://fermatter.github.io/guide/requirements/#tool-discovery). If a tool is missing, select its executable path there.

Recoverable project deletion, the screen color eyedropper and MCP/CLI are unavailable in this Windows preview.

<h3 id="windows-update">Update</h3>

Quit FerMatter from its tray menu and run the newer installer. Your project files and saved settings are preserved.
