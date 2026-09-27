# Shortcuts and appearance

- Open **Mathlidelux → Settings…** (`⌘,`).
- **Shortcuts**: record/clear a shortcut or restore defaults; changes persist and update menus.
- Duplicate assignments and reserved macOS/text-editing shortcuts are rejected.
- **Editor**: choose auto save or Manual; set the auto-save delay.
- **Templates**: customize the setup for [new projects](new-project.md).

## Color themes

- Open **Settings → Appearance → Color Theme**; choose Default or one of fifteen bundled pairs.
- **System** follows macOS; **Light** and **Dark** select the matching palette. Preview both palettes below the picker.
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

- Palettes are adapted to Mathlidelux; native glass and exported artwork retain their existing rendering.

## Manage shortcuts and themes as files

- **Shortcuts** and **Appearance** provide **Open File**, **Show in Finder**, **Reload**, **Import…**, and **Export…**.
- Edit `keybindings.json` or `theme.json`, save, then choose **Reload** to apply external changes.
- Import replaces the active settings after validation; export saves a reusable copy.
- Invalid files keep the last valid settings; **Restore Defaults** replaces custom settings.
- Theme files customize light/dark interface and editor colors; exported artwork keeps its original colors.

## Updates

- **Settings → Updates** controls automatic checks; **Check for Updates…** checks immediately.
- **Download Update** accepts a release download. **Open Disk Image** opens it for manual installation.
- Quit Mathlidelux and copy the new app into Applications; the updater does not replace the running app.
- A private repository or unavailable release cannot provide a public update download.

## Keep equations beside your slides

- **Always on Top** (`⇧⌘P`) pins the window.
- **Appearance & Window** chooses System/Light/Dark and background opacity.
- Opacity affects the background; text and equations stay opaque.
- `⌃⌘F` enters native full screen.
- If entering full screen from a pinned window fails, turn pinning off, enter full screen, then re-enable it.
- Some wallpaper providers/Spaces cannot be reproduced behind a transparent full-screen window; native transparency remains available.
