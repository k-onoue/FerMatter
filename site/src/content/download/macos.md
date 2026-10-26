---
title: "Download"
description: "Download FerMatter for macOS or Windows, and install LaTeX or Typst."
platform: "macOS · Windows"
requirements: "Apple Silicon Mac · macOS 14 or later · unnotarized preview."
---

## Homebrew

```sh
brew install --cask fermatter/tap/fermatter
```

<span id="updates"></span>

## Update

```sh
brew upgrade --cask fermatter/tap/fermatter
```

<span id="rendering-engines"></span>

## Math rendering engine

FerMatter requires LaTeX or Typst to render equations and drawings. The commands below are for macOS.

<span id="install"></span>

### LaTeX

#### Full TeX Live

```sh
brew install --cask mactex-no-gui
brew install pdf2svg poppler ghostscript
```

#### BasicTeX

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

### Typst

```sh
brew install typst
```

<span id="requirements"></span>

[Requirements →](https://fermatter.github.io/guide/requirements/)
