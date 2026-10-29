---
title: "A LaTeX project with latexmkrc produces no PDF"
description: "PDF output is the default when the project has not selected another output mode."
group: "earlier"
availability: "Included in the first public macOS preview"
order: 14
---

## What changed

This addresses projects with a `latexmkrc` that does not explicitly choose an output mode.

## What to do

The fix is already in [macOS Preview 1](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-preview.1) and later. Check [LaTeX requirements](https://fermatter.github.io/guide/requirements/) if compilation still fails.
