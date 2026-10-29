---
title: "App identity needs to stay stable across upgrades"
description: "A permanent app ID preserves existing data and recovery drafts through migration."
group: "pending"
availability: "All packaged platforms: awaiting a release"
order: 11
---

## What changed

The permanent identity is `io.github.fermatter.fermatter`. Source builds retain the existing project/settings storage and explicitly migrate or reuse the previous WebView storage.

## What to do

The identity migration is in source builds. It is not yet included in every packaged platform; use the release notes and platform label when choosing an update.
