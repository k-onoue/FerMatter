# Public assets

- `app-light.jpg` / `app-dark.jpg`: native app, synthetic `examples/demo-paper/`, opaque background.
- `trajectory.svg`: demo equation rendered by the TeX worker; glyphs are paths.
- `icon.png`: unchanged approved artwork from `macos-app/Resources/AppIcon.icon/Assets/AppIcon.png`; same source as PR #6.
- Never capture private manuscripts, project-history lists, accounts, or desktop notifications.

## Paper-to-slides tutorial

- Files: `tutorials/paper-to-slides.mp4`, `.jpg` poster, `.vtt` captions.
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
