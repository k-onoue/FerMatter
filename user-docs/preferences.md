---
title: "Shortcuts and appearance"
description: "Change editing shortcuts and keep the app beside your slides."
sidebar:
  order: 10
---

- Open **Settings** using the gear button or `⌘,`.
- **Shortcuts**: record a shortcut, reset one assignment, or restore defaults; changes persist.
- Duplicate assignments and reserved macOS/text-editing shortcuts are rejected.
- **Projects → Default project folder**: choose where to create/import/duplicate projects; existing projects stay in place.
- **Editor → Save files**: choose After changes or Manually (`⌘S`).
- **New Project → Edit Template**: customize the setup for [new projects](new-project.md).

## Compiler tools

- Review tool paths during first-launch setup or later in **Settings → Tools**.
- Enter an absolute executable path or use its folder button; choose **Apply** to save.
- Empty fields use automatic detection. The reset arrow clears an override; choose **Apply** to restore detection.
- **Additional tools** includes PDF converters, SyncTeX, and Ghostscript. **Check paths** checks executable availability without running the tools.
- Paths apply to subsequent Library renders, Source previews, PDF builds/navigation, and both Instant modes. Use **Rescan** or rebuild the PDF to refresh existing results.
- Edit `tools.json` in the revealed Settings folder and choose **Reload** to accept external changes. Invalid JSON keeps the last valid configuration; invalid executable paths are reported without falling back to another binary.
- Tool installation and the per-project main file/compiler choice remain separate: see [requirements](requirements.md).

## Window zoom

- `⌘+` or `⌘=` enlarges the interface; `⌘−` reduces it; `⌘0` restores 100%.
- Zoom works in Projects, Library, Source and dialogs, including while editing. The range is 50–200%, in 10% steps; restarting the app restores 100%.
- **Shortcuts → Zoom In / Zoom Out / Actual Size** customizes these keys.
- PDF-only controls use `⌃⌘+`, `⌃⌘−` and `⌃⌘0` (Fit PDF); the PDF toolbar also remains available.
- Saved custom shortcuts take precedence over new zoom defaults. A conflicting default appears unassigned until the custom assignment is changed or reset.
- Window zoom changes the interface, not exported image dimensions or output font sizes.

## Zoom one pane

- Point at Projects, the Library canvas or the PDF pane; hold `Control` and scroll up to enlarge or down to reduce it.
- Each area keeps its own zoom. Regular scrolling and `⌘+` / `⌘−` window zoom remain available.
- Projects (grid and list) and the Library canvas support 50–200%; click their percentage to reset to 100%.
- PDF supports 20–400%. Scrolling from **Fit** starts at the displayed scale; **Fit width** returns to automatic sizing.
- Pane zoom changes the display only; exported image sizes and source files are unchanged.

## Color themes

- Open **Settings → Appearance → Color Theme**; choose Default or one of fifteen bundled pairs.
- **System** follows macOS; **Light** and **Dark** select the matching palette. The preview follows the selected appearance.
- Themes apply immediately and persist across launches; switching replaces the active `theme.json`. Export custom colors first to keep a reusable copy.

| Theme | Light | Dark |
| --- | --- | --- |
| [GitHub](https://github.com/primer/github-vscode-theme) | GitHub Light (classic) | GitHub Dark (classic) |
| [Catppuccin](https://github.com/catppuccin/vscode) | Latte | Mocha |
| [Solarized](https://github.com/microsoft/vscode/tree/main/extensions/theme-solarized-light) | Solarized Light | Solarized Dark |
| [Atom One](https://github.com/akamud/vscode-theme-onelight) | One Light | One Dark |
| [Rosé Pine](https://github.com/rose-pine/vscode) | Dawn | Rosé Pine |
| [VS Code Modern](https://github.com/microsoft/vscode/tree/main/extensions/theme-defaults) | Light Modern | Dark Modern |
| [Ayu](https://github.com/ayu-theme/vscode-ayu) | Ayu Light | Ayu Dark |
| [Tokyo Night](https://github.com/tokyo-night/tokyo-night-vscode-theme) | Tokyo Night Light | Tokyo Night |
| [Night Owl](https://github.com/sdras/night-owl-vscode-theme) | Light Owl | Night Owl |
| [Horizon](https://github.com/jolaleye/horizon-theme-vscode) | Horizon Bright | Horizon |
| [Vitesse](https://github.com/antfu/vscode-theme-vitesse) | Vitesse Light | Vitesse Dark |
| [Everforest](https://github.com/sainnhe/everforest-vscode) | Everforest Light | Everforest Dark |
| [Gruvbox Material](https://github.com/sainnhe/gruvbox-material-vscode) | Gruvbox Material Light | Gruvbox Material Dark |
| [Noctis](https://github.com/liviuschera/noctis) | Noctis Lux | Noctis |
| [Min](https://github.com/miguelsolorio/min-theme) | Min Light | Min Dark |

- Palettes are adapted to FerMatter; exported artwork retains its original rendering.

## Manage shortcuts and themes as files

- **Shortcuts** and **Appearance** provide **Edit JSON**, **Reload**, **Import…**, and **Export…**.
- **Tools → Settings folder → Reveal in Finder** opens the storage location.
- Edit `keybindings.json` or `theme.json`, save, then choose **Reload** to apply external changes.
- Import replaces the active settings after validation; export saves a reusable copy.
- Invalid files keep the last valid settings; reset shortcuts or select the Default theme to restore built-ins.
- Theme files customize light/dark interface and editor colors; exported artwork keeps its original colors.

## Dock icon

- **Settings → Appearance → Dock icon** chooses Automatic or a fixed face: Detailed Fermat, Standard, Smiling, or Downturned mouth. Changes apply immediately and persist across launches.
- Automatic follows the number of openings in the last seven days, including the current opening. Manual choices stay fixed until Automatic is selected again.

| Openings | Face |
| --- | --- |
| 14 or more | Detailed Fermat portrait |
| 7–13 | Standard |
| 3–6 | Smiling |
| 0–2 | Downturned mouth |

- Launching FerMatter or reopening it from the Dock updates the count. Duplicate native events within two seconds count once; background automation does not count.
- History stays on this Mac in `IconUsage.json`. Finder and the app at rest use the standard blue icon; the active Dock artwork is recalculated when opened.

## Updates

- This version uses manual updates; it has no in-app release checker or downloader.
- Save your work, quit FerMatter, and replace the app in Applications with the newer build.
- Upgrading from Mathlidelux: quit the old app, install `FerMatter.app`, then remove only the old `Mathlidelux.app` bundle. Projects, settings, drafts and SVG re-editing remain available; project folders stay in place.
- Projects and saved settings remain outside the application bundle.

## Keep equations beside your slides

- **Always on Top** (`⇧⌘P`) pins the window.
- **Settings → Appearance** chooses System/Light/Dark and background opacity.
- Opacity affects the background; text and equations stay opaque.
- `⌃⌘F` enters native full screen.
- If entering full screen from a pinned window fails, turn pinning off, enter full screen, then re-enable it.
- Some wallpaper providers/Spaces cannot be reproduced behind a transparent full-screen window; native transparency remains available.
