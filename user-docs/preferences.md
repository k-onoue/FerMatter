# Shortcuts and appearance

- Open **Settings** using the gear button or `⌘,`.
- **Shortcuts**: record a shortcut, reset one assignment, or restore defaults; changes persist.
- Duplicate assignments and reserved macOS/text-editing shortcuts are rejected.
- **Editor → Save files**: choose After changes or Manually (`⌘S`).
- **New Project → Edit Template**: customize the setup for [new projects](new-project.md).

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

- Palettes are adapted to Mathlidelux; exported artwork retains its original rendering.

## Manage shortcuts and themes as files

- **Shortcuts** and **Appearance** provide **Edit JSON**, **Reload**, **Import…**, and **Export…**.
- **Tools → Settings folder → Reveal in Finder** opens the storage location.
- Edit `keybindings.json` or `theme.json`, save, then choose **Reload** to apply external changes.
- Import replaces the active settings after validation; export saves a reusable copy.
- Invalid files keep the last valid settings; reset shortcuts or select the Default theme to restore built-ins.
- Theme files customize light/dark interface and editor colors; exported artwork keeps its original colors.

## Updates

- This version uses manual updates; it has no in-app release checker or downloader.
- Save your work, quit Mathlidelux, and replace the app in Applications with the newer build.
- Projects and saved settings remain outside the application bundle.

## Keep equations beside your slides

- **Always on Top** (`⇧⌘P`) pins the window.
- **Settings → Appearance** chooses System/Light/Dark and background opacity.
- Opacity affects the background; text and equations stay opaque.
- `⌃⌘F` enters native full screen.
- If entering full screen from a pinned window fails, turn pinning off, enter full screen, then re-enable it.
- Some wallpaper providers/Spaces cannot be reproduced behind a transparent full-screen window; native transparency remains available.
