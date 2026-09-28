# Mathlidelux app icon

- Artwork supplied and selected by the repository owner on 2026-09-28.
- An x² tile, cursor, and curved arrow point toward an image on a slide.
- Source asset: `AppIcon.icon/Assets/AppIcon.png`; the opaque interior is unchanged from the supplied image.
- The owner approved conventional image processing to remove the dark exterior. The final alpha mask follows the existing rounded-square edge, with edge colors corrected for the removed backdrop.
- Built-in `image_gen` background-extraction attempts left exterior artifacts and were not used in the shipped asset.
- Tauri bundles `src-tauri/icons/icon.icns` and `icon.png`; no image generation service or Icon Composer is needed at build time.
- In-app mark: `src-tauri/icons/icon.png`; website mark: `docs/assets/icon.png` at the repository root.
- ICNS contains 16, 32, 128, 256 and 512-point representations at 1×/2×, resized with Lanczos and packaged with `iconutil`.

## Image-generation edit prompt (unused output)

> Remove only the dark backdrop outside the blue-purple rounded square. Preserve the x² tile, cursor, curved arrow, slide, geometry, colors and framing. Use clean alpha transparency beyond the existing rounded-square boundary, without an exterior shadow or flecks.
