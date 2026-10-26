---
title: "Linux installation"
description: "Ubuntu packages and rendering tools."
platform: "Ubuntu 24.04 · amd64"
---

<span id="for-ubuntu-experimental"></span>

<h3 id="ubuntu-install">Install</h3>

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

```sh
sudo apt update
sudo apt install fermatter
```

<h3 id="ubuntu-rendering-tools">Rendering tools</h3>

Install LaTeX or [Typst](https://github.com/typst/typst#installation).

<details>
<summary>LaTeX tools for Ubuntu</summary>

```sh
sudo apt install latexmk texlive-latex-extra texlive-extra-utils texlive-pictures ghostscript pdf2svg poppler-utils lmodern cm-super-minimal
```

</details>

[Check installed tools →](https://fermatter.github.io/guide/requirements/#tool-discovery)

<h3 id="ubuntu-update">Update</h3>

Use the same [APT commands](#ubuntu-install) to update FerMatter.
