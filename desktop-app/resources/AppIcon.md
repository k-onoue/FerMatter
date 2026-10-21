# FerMatter artwork

- Artwork supplied and selected by the repository owner on 2026-10-21: a simplified, clean-shaven Fermat head with rounded hair, dot eyes and no neck, collar or shoulders.
- Vector master: [`fermat.svg`](fermat.svg), traced directly from the supplied design using Potrace 1.16. Background, facial lines and hair gaps are transparent; no embedded bitmap, texture or blue outlines.
- `currentColor` allows any foreground color. App and site use a CSS alpha mask with `background-color: currentColor`, so an explicit theme change updates the mark even when it differs from the OS theme.
- Four Dock states, most to least frequent: the earlier clean-shaven detailed portrait (`fermat-active.svg`, built-in ImageGen); the standard mark (`fermat.svg`); the supplied smile (`fermat-smile.svg`); and the supplied downturned mouth (`fermat-frown.svg`). Each was vectorized with Potrace.

| Asset | Use |
| --- | --- |
| `fermat.svg` | Recolorable transparent master; default black when opened alone |
| `AppIcon.svg` | Off-white (`#f2f2e8`) head on a `#4a7699` rounded square, transparent outside the square |
| `AppIcon.icon/Assets/AppIcon.png` | Transparent off-white artwork for Icon Composer's blue fill |
| `../src-tauri/icons/icon.png`, `icon.icns` | Tauri bundle, Dock, Finder and app switcher |
| `../src-tauri/icons/tray.png` | Black alpha silhouette; macOS template mode supplies menu-bar contrast |
| `../src-tauri/icons/usage-1.png`, `usage-3.png`, `usage-4.png` | Running Dock variants; rank 2 uses `icon.png` |
| `../../site/public/assets/fermat.svg` | Site header mask, shared by product pages and guide |
| `../../site/public/assets/fermat-light.svg`, `fermat-dark.svg` | Dark-on-light and light-on-dark standalone SVGs; README and Starlight logo |
| `../../site/public/assets/favicon.svg` | Transparent browser icon following `prefers-color-scheme` |
| `../../site/public/assets/icon.png` | Blue app-icon raster for existing image links |

## Regenerate derived assets

```sh
# macOS; ImageMagick, librsvg and iconutil must be available.
node scripts/build-icons.mjs
git diff --check
```

- Normal app/site builds consume committed assets; they need no image generation, tracing or icon-generation tools.
- Edit the SVG master, then regenerate all derivatives together. The generator uses no private source files or network services.
- ICNS includes 16, 32, 128, 256 and 512-point representations at 1×/2×, resized with Lanczos.
- Review the mark at small sizes on light, dark and colored backgrounds; check the blue app tile separately.
- Historical screenshots/tutorial footage retain the interface actually captured; their provenance remains in [public asset notes](../../site/public/assets/README.md).
- Dock thresholds and behavior: [preferences](../../user-docs/preferences.md#dock-icon). `usage_icon.rs` records local openings; `NSApplication.applicationIconImage` changes only the running Dock display, preserving the signed bundle.
