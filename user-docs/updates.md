---
title: "Updates"
description: "Check your FerMatter version and install available updates."
sidebar:
  order: 9
---

To find a fix for a particular problem, see [Resolved issues](https://fermatter.github.io/resolved-issues/).

## Find your version

Previews through Preview 5 all display **0.11.0** inside the app. Use the preview number from your downloaded installer or release page as well. Windows and macOS have separate preview releases; matching version numbers do not mean they contain the same fixes.

## Update FerMatter

Preview 5 includes the Updates panel, but the stable feed currently offers no update. Install preview updates manually or through Homebrew/APT.

- **Preview 4 and earlier:** follow the [Download page](https://fermatter.github.io/download/). The first move to a version with the updater is manual.
- **Versions with the updater:** open **Settings → Updates** or the download icon beside the window controls. Choose **Check for updates**, **Download update**, then **Install and restart**. All open editors must finish saving first; a failed save cancels installation.
- Automatic checks are on by default and run at most once a day. An indicator appears when an update is available. You can turn checks off in Updates; an existing off setting is preserved. Downloading and installing still require your action.
- Only published stable updates for your OS and CPU are offered. Preview releases are installed manually. A merged fix does not trigger an update notification.
- Downloads are verified before installation. Failed downloads can be retried or cancelled. On Windows, FerMatter closes after handing off to the installer; if you cancel that installer, reopen the app. On Ubuntu, a graphical authentication agent is required to install the DEB; APT remains available.
- If you installed through Homebrew, you can continue using `brew upgrade --cask fermatter/tap/fermatter` to keep Homebrew's package record synchronized. The in-app updater replaces the application without updating Homebrew's own receipt.

Update checks contact GitHub Pages; update downloads use GitHub Releases. No documents, equations, settings or usage data are uploaded. Update-file signatures verify the publisher's file; they do not mean the macOS app is notarized or the Windows installer has Authenticode signing.
