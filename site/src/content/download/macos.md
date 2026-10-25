---
title: "Download"
description: "Download FerMatter for macOS, Windows or Linux, and install LaTeX or Typst."
requirements: "macOS 14+ · Apple Silicon"
---

<h3 id="mac-install">Install</h3>

Open the DMG and move FerMatter to Applications, or use Homebrew:

#### Homebrew

```sh
brew install --cask fermatter/tap/fermatter
```

<span id="rendering-engines"></span>
<span id="math-rendering-engine"></span>

<h3 id="mac-rendering-tools">Rendering tools</h3>

Install LaTeX or Typst to render equations and drawings.

<span id="install"></span>

#### LaTeX

##### Full TeX Live

```sh
brew install --cask mactex-no-gui
brew install pdf2svg poppler ghostscript
```

##### BasicTeX

```sh
brew install --cask basictex
brew install pdf2svg poppler ghostscript
sudo /Library/TeX/texbin/tlmgr update --self
sudo /Library/TeX/texbin/tlmgr install latexmk preview pdfcrop pgf mathtools
```

| TeX option | Installer download | Installed TeX files |
| --- | --- | --- |
| Full MacTeX | 6.4 GB | 10 GB |
| BasicTeX + packages above | 134 MB + extra packages | 0.5–1.3 GB |

#### Typst

```sh
brew install typst
```

<span id="requirements"></span>

[Requirements →](https://fermatter.github.io/guide/requirements/)

<span id="updates"></span>

<h3 id="update">Update</h3>

Replace FerMatter in Applications with the newer download, or update Homebrew:

```sh
brew upgrade --cask fermatter/tap/fermatter
```
