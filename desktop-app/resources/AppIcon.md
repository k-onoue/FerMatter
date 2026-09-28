# Mathlidelux app icon

- Approved artwork: `AppIcon.icon/Assets/AppIcon.png`, created with `image_gen` on 2026-09-27.
- An x² tile is dragged onto a slide; the white cursor has a soft glow and no outline.
- The original Icon Composer package is retained here for future icon builds.
- Tauri bundles the existing `src-tauri/icons/icon.icns` and `icon.png`; it does not require Icon Composer or an image generation service.
- In-app mark: `public/app-icon.png`; website mark: `docs/assets/icon.png` at the repository root.

## Final image edit prompt

> Use case: precise-object-edit. This is a transparent production app icon with a cutout defect. Repair ONLY its external alpha edge. There are ragged blue flecks, wisps and thin artifacts OUTSIDE the main blue rounded square, especially above the top edge and below the bottom edge. REMOVE ALL detached flecks, residual backdrop, colored fringing and wisps from the transparent outer margin. The outer contour must be a pristine geometrically smooth continuous rounded square, with clean antialiasing directly at the contour and completely transparent pixels beyond that smooth boundary. Do not add any exterior shadow or glow. Keep the entire interior EXACTLY unchanged: blue gradient background, pale slide with blue title line, blue x² tile with white typography, solid white cursor with its subtle internal white halo, all placement and dimensions. Preserve square canvas and margin. This is alpha mask cleanup only, not a redesign. Use genuine alpha transparency outside the single clean rounded-square boundary.
