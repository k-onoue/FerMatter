# Public assets

- `app-light.jpg`: historical Swift app with synthetic `examples/demo-paper/`, retained for older image links; the README uses the current desktop Library screenshot.
- Fermat artwork selected by the repository owner on 2026-10-21; sources and regeneration: [artwork notes](../../../desktop-app/resources/AppIcon.md).
- `fermat.svg`: transparent, recolorable head used as the product/guide header mask. Facial details and hair gaps remain transparent on every background.
- `fermat-light.svg`, `fermat-dark.svg`: dark and off-white versions for standalone images and the README.
- `favicon.svg`: transparent mark with an OS light/dark color query. `icon.png`: blue app tile for existing image links.
- Never capture private manuscripts, project-history lists, accounts, or desktop notifications.

## Paper-to-slides tutorial

- Files: `tutorials/paper-to-slides.mp4`, `.jpg` poster, `.vtt` captions.
- Historical Swift workflow; current desktop controls differ.
- Date: 2026-09-28; Mathlidelux 0.10.0; integration commit `7b0205a`; macOS 26.
- Form: three real application stills held for six seconds each; a step-through, not continuous screen recording.
- Source material: the synthetic `examples/demo-paper/` project; a newly created blank PowerPoint presentation.
- Verify screenshots contain only the intended window before encoding.

## Re-capture

1. Open `examples/demo-paper/`; choose **Block**; capture the library with no item selected.
2. Select **Trajectory**; leave **SVG** selected; capture the inspector; choose **Copy SVG**.
3. Create a blank PowerPoint slide; paste; resize the graphic with its aspect ratio locked; capture the result.
4. Fit each capture within 1440×820 on a 1440×920 light canvas; reserve the bottom strip for the matching instruction.
5. Encode three six-second segments: H.264, 24 fps, yuv420p, CRF 21, no audio; concatenate with MP4 fast-start metadata.
6. Use the second segment as the JPEG poster; update WebVTT at 0/6/12/18 seconds and matching guide steps.
7. Review all frames, captions, keyboard-accessible playback, and the final SVG paste before committing.

- Raw captures remain local; only reviewed compressed media is published.
- Keep the actual controls visible; do not fabricate app states or claim unsupported drag behavior.
- MP4 is about 212 KiB; poster about 84 KiB in this revision.
- Page policy: controls, no autoplay/loop, `preload="none"`, text steps available without playback.
- References: [video performance](https://web.dev/learn/performance/video-performance), [HTML video](https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/video).

## Current desktop screenshots

- Captured 2026-10-20, FerMatter 0.10.0, Rust/Tauri interface, name-change branch plus titlebar cleanup (`d3ea55a`).
- `demos/library.png`: rendered `examples/demo-paper` equations.
- `demos/source.png`: `sections/model.tex` beside its compiled PDF.
- `demos/instant.png`: the same trajectory equation in project Instant.
- `demos/figures.png`: rendered `examples/demo-cetz` examples.
- Captured through the native window API in an isolated demo app/data directory; no personal projects, accounts, absolute paths, or desktop contents are visible.
- Original reviewed PNGs are retained; Astro emits responsive WebP derivatives for the site.
- Demo title, alt text, guide, and sample links: `site/src/data/demos.json`.
