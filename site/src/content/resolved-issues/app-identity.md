---
title: "App identity needs to stay stable across upgrades"
description: "A permanent app ID preserves existing data and recovery drafts through migration."
group: "released"
availability: "macOS, Windows and Ubuntu Preview 5"
order: 11
---

## What changed

The permanent identity is `io.github.fermatter.fermatter`. Preview 5 retains the existing project/settings storage and explicitly migrates or reuses the previous WebView storage.

## What to do

Install Preview 5 or later from [Download](https://fermatter.github.io/download/). Existing project folders and saved data are retained. macOS may request folder access again after the one-time identity migration.
