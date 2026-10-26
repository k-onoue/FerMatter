---
title: "Library controls and platform updates"
description: "Exclude tables from Library cards, with updates for Windows and Linux."
date: 2026-10-26
status: upcoming
---

The Library extraction controls below are planned for the next app release and are not included in **0.11.0 Preview 4**. See [Download](https://fermatter.github.io/download/) for current platform-specific installers.

## Choose what appears in your Library

A new **Settings → Extraction** option ignores math inside tables. It is off by default. You can also add your own LaTeX environment names and Typst function names to `extraction.json`.

Applying the settings refreshes open libraries. Source files, Source previews and the full paper PDF stay unchanged; saved presentation variants remain available.

See the [Card extraction guide](https://fermatter.github.io/guide/extraction/) for configuration examples. This addresses [issue #111](https://github.com/k-onoue/FerMatter/issues/111).

## Windows preview preparation

Windows 11 x64 support and EXE packaging have been merged in [PR #109](https://github.com/k-onoue/FerMatter/pull/109). ARM64 and 32-bit installers are not included.

Library and Source also share consistent toolbar icon alignment across desktop platforms.

See [Windows installation and rendering tools](https://fermatter.github.io/download/#windows) for installer availability, setup and current limitations.

## Reveal files on Linux

**Reveal in File Manager** opens the containing folder on Linux. The macOS action remains **Reveal in Finder** and selects the file.

This fixes a macOS-specific command used on other platforms. Ubuntu support remains experimental. See [PR #110](https://github.com/k-onoue/FerMatter/pull/110).
