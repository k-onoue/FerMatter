# Mathlidelux app icon

`AppIcon.icon/Assets/AppIcon.png` is the approved raster master, with transparent surroundings.
It depicts an x² tile being dragged onto a slide. The white cursor has a soft
white glow and no outline. The icon was created and refined with the built-in
`image_gen` tool on 2026-09-27.

`AppIcon.icon/icon.json` places the approved artwork over a matching blue
background, with additional glass, specular, translucency, and group shadow
effects disabled. `scripts/build.sh` compiles the package with Xcode's `actool`
into `Assets.car` plus an `AppIcon.icns` fallback. Both `CFBundleIconName` and
`CFBundleIconFile` point to `AppIcon`. Generated files remain in the ignored
build folder; no image generation service is needed to build the app.

The native icon stack avoids the extra white plate observed when macOS 26
renders the legacy ICNS alone. The artwork stays in one layer to preserve the
approved design; the system supplies the native outer enclosure.
See [Apple's icon documentation](https://developer.apple.com/documentation/xcode/creating-your-app-icon-using-icon-composer).

## Final image edit prompt

> Use case: precise-object-edit. This is a transparent production app icon with a cutout defect. Repair ONLY its external alpha edge. There are ragged blue flecks, wisps and thin artifacts OUTSIDE the main blue rounded square, especially above the top edge and below the bottom edge. REMOVE ALL detached flecks, residual backdrop, colored fringing and wisps from the transparent outer margin. The outer contour must be a pristine geometrically smooth continuous rounded square, with clean antialiasing directly at the contour and completely transparent pixels beyond that smooth boundary. Do not add any exterior shadow or glow. Keep the entire interior EXACTLY unchanged: blue gradient background, pale slide with blue title line, blue x² tile with white typography, solid white cursor with its subtle internal white halo, all placement and dimensions. Preserve square canvas and margin. This is alpha mask cleanup only, not a redesign. Use genuine alpha transparency outside the single clean rounded-square boundary.
