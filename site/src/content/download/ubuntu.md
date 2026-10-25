---
title: "Linux installation"
description: "Experimental Ubuntu packages and rendering tools."
---

<h2 id="for-ubuntu-experimental">Linux (experimental)</h2>

Ubuntu 24.04 · amd64. Limited testing.

<details>
<summary>First-time APT setup</summary>

```sh
sudo apt update
sudo apt install curl
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://fermatter.github.io/apt/fermatter.gpg | sudo tee /etc/apt/keyrings/fermatter.gpg > /dev/null
curl -fsSL https://fermatter.github.io/apt/fermatter.sources | sudo tee /etc/apt/sources.list.d/fermatter.sources > /dev/null
```

</details>

<h3 id="ubuntu-update">Install or update</h3>

```sh
sudo apt update
sudo apt install fermatter
```

Install LaTeX or [Typst](https://github.com/typst/typst#installation) separately.

<details>
<summary>LaTeX tools for Ubuntu</summary>

```sh
sudo apt install latexmk texlive-latex-extra texlive-extra-utils texlive-pictures ghostscript pdf2svg poppler-utils lmodern cm-super-minimal
```

</details>

[Check installed tools →](https://fermatter.github.io/guide/requirements/#tool-discovery)

Use file export for images. Native image clipboard and recoverable project deletion are unavailable.
