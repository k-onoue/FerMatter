# FerMatter artwork

- The owner selected the earlier detailed Fermat portrait for all branding on 2026-10-22. It has detailed eyes and hair, with no neck, collar or shoulders; originally created with ImageGen and vectorized with Potrace 1.16.
- Vector master: [`fermat.svg`](fermat.svg), the detailed portrait previously named `fermat-active.svg`. Background, facial lines and hair gaps are transparent; no embedded bitmap, texture or blue outlines.
- `currentColor` allows any foreground color. App and site use a CSS alpha mask with `background-color: currentColor`, so an explicit theme change updates the mark even when it differs from the OS theme.
- One portrait is used throughout the app, Dock, menu bar, website, guide and README. Usage-based and manual face switching have been retired.

| Asset | Use |
| --- | --- |
| `fermat.svg` | Recolorable transparent master; default black when opened alone |
| `AppIcon.svg` | Off-white (`#f2f2e8`) head on a `#4a7699` rounded square, transparent outside the square |
| `AppIcon.icon/Assets/AppIcon.png` | Transparent off-white artwork for Icon Composer's blue fill |
| `../src-tauri/icons/icon.png`, `icon.icns` | Tauri bundle, Dock, Finder and app switcher |
| `../src-tauri/icons/tray.png` | Black alpha silhouette; macOS template mode supplies menu-bar contrast |
| `fermatter-organization.jpg` | 1024×1024 sRGB JPEG on full-bleed blue for the GitHub Organization avatar |
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
- The 44-pixel menu-bar template trims the SVG margins and fits the face within 42 pixels. `native::configure_menu_bar_icon` presents it at 20 points, centered as an image-only AppKit button instead of using the tray backend's 18-point, image-beside-title default.
- Review the mark at small sizes on light, dark and colored backgrounds; check the blue app tile separately.
- Historical screenshots/tutorial footage retain the interface actually captured; their provenance remains in [public asset notes](../../site/public/assets/README.md).
- The Dock always uses the bundled portrait. Legacy icon preferences and usage-history files remain untouched; see [preferences](../../user-docs/preferences.md#dock-icon).
