---
title: "Resolved issues"
description: "Match an older FerMatter problem to the release that fixes it."
sidebar:
  order: 9
---

Use this page to check whether updating can resolve a problem. If the symptom remains on the listed version or a newer one, [report it](https://github.com/k-onoue/FerMatter/issues) with your OS, CPU, version and a small synthetic example.

## Find your version

The older previews all display **0.11.0** inside the app. Use the preview number from your downloaded installer or release page as well. Windows and macOS have separate preview releases; matching version numbers do not mean they contain the same fixes.

## Fixes available in downloads

| Symptom | Fixed in | What changed |
| --- | --- | --- |
| Initial project setup fails or chooses a OneDrive-managed folder | [Windows Preview 4](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-windows-preview.4) | New workspaces default to the local user folder's `FerMatter` directory, created during setup. Existing project locations are preserved. |
| A shortcut is accepted in Settings but rejected when saved or reloaded | [Windows Preview 3](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-windows-preview.3) | Settings and saved files use the same validation rules; custom shortcuts and Ctrl+Tab navigation are preserved. |
| Rendering or export reports a sharing error involving a settings backup | [Windows Preview 2](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-windows-preview.2) | Reading unchanged settings no longer rewrites their recovery copies. |
| A card's context menu exports another selected card | [Windows Preview 2](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-windows-preview.2) | Export uses the card whose menu was opened. |
| Demos or project setup cannot be reached from standalone Instant | [Windows Preview 2](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-windows-preview.2) | **Demo projects** opens setup or Projects while preserving Instant drafts. |
| An empty titlebar strip appears, or native menus ignore the chosen appearance | [Windows Preview 2](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-windows-preview.2) | Native titlebar spacing and window/menu appearance follow Windows conventions. |
| Library and Source toolbars have repeated headings or excessive vertical spacing | [macOS Preview 4](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-preview.4) | Compact toolbars replace repeated headings; window controls are grouped separately. Further icon-position alignment is listed below. |

### Already included in the first public macOS preview

These fixes predate later public previews. Updating from a local prototype may help; if you already use a public preview, check the linked troubleshooting instructions too.

| Symptom in an older prototype | Fix included in [Preview 1](https://github.com/FerMatter/fermatter.github.io/releases/tag/v0.11.0-preview.1) | Related guide |
| --- | --- | --- |
| Typst 0.15 produces a JSON parsing error | Compiler warnings are separated from query JSON. | [Compiler requirements](requirements.md) |
| A LaTeX project with `latexmkrc` produces no PDF when no output mode is specified | PDF becomes the default when the project has not explicitly chosen another output mode. | [LaTeX requirements](requirements.md) |
| Closing an editor after a failed save discards pending edits | Source and Library drafts are protected; failed saves cancel closing. | [Editing modes](paper-to-slides.md) |
| A late project scan overwrites a recent Library edit | Obsolete scan results are ignored. | [Library variants](paper-to-slides.md) |
| Moving or renaming Source files loses draft/main-file associations | File operations preserve drafts, main-file references and Undo consistency. | [Source editing](paper-to-slides.md) |
| Corrupted metadata is silently replaced | Recovery preserves the original file; invalid metadata must be addressed explicitly. | [Project setup](README.md) |

## Fixed in source, awaiting a download

**These changes are merged but are not in the currently linked downloads for every platform.** Installing the current macOS DMG or Ubuntu package will not necessarily include them.

| Change | Available now | Still awaiting a release |
| --- | --- | --- |
| [Configurable table and environment exclusions](extraction.md) | Windows Preview 2+ | macOS and Ubuntu |
| Context-menu export targets the clicked card; unchanged settings backups are not rewritten | Windows Preview 2+ | macOS and Ubuntu |
| Demos/setup from standalone Instant; additional Library/Source icon alignment | Windows Preview 2+ | macOS and Ubuntu |
| Single SVG copies stay vector artwork when pasted into LibreOffice Impress on Windows | Source builds | Windows |
| Linux titlebar spacing and appearance synchronization | Source builds | Ubuntu |
| Permanent app identity with migration of existing data and recovery drafts | Source builds | All packaged platforms |
| In-app update notification, download and installation | Source builds; installation requires a packaged build | All packaged platforms |

Ubuntu remains experimental. The Windows releases above do not establish Ubuntu compatibility. See the [source changelog](https://github.com/k-onoue/FerMatter/blob/main/CHANGELOG.md) for complete changes and unreleased work.

## Update FerMatter

- **Existing previews:** follow the [Download page](https://fermatter.github.io/download/). The first move to a version with the updater is manual.
- **Versions with the updater:** open **Settings → Updates** or the download icon beside the window controls. Choose **Check for updates**, **Download update**, then **Install and restart**. All open editors must finish saving first; a failed save cancels installation.
- Automatic checks are optional and off initially. Enable them in Updates to check at most once a day and show an indicator when an update is available. Downloading and installing still require your action.
- Only published stable updates for your OS and CPU are offered. Preview releases are installed manually. A merged fix does not trigger an update notification.
- Downloads are verified before installation. Failed downloads can be retried or cancelled. On Windows, FerMatter closes after handing off to the installer; if you cancel that installer, reopen the app. On Ubuntu, a graphical authentication agent is required to install the DEB; APT remains available.
- If you installed through Homebrew, you can continue using `brew upgrade --cask fermatter/tap/fermatter` to keep Homebrew's package record synchronized. The in-app updater replaces the application without updating Homebrew's own receipt.

Update checks contact GitHub Pages; update downloads use GitHub Releases. No documents, equations, settings or usage data are uploaded. Update-file signatures verify the publisher's file; they do not mean the macOS app is notarized or the Windows installer has Authenticode signing.
