# Bundled themes

- Fifteen local JSON documents; each includes both light and dark palettes.
- The original Default theme remains native and requires no JSON resource.
- Source repositories, pinned revisions, source files, and license paths live in each document's `extensions.org.mathlidelux.theme-source` metadata.
- Original MIT notices are preserved verbatim in `Licenses/`; shipped in the app's `Contents/Resources/Mathlidelux_SlideMath.bundle/Themes/`.
- These are palette adaptations for FerMatter's eight tokens, not complete VS Code extensions. No extension code is included or executed.

| Pair | Palette source | Mapping |
| --- | --- | --- |
| GitHub | [GitHub VS Code Theme](https://github.com/primer/github-vscode-theme), classic light/dark | Editor/sidebar backgrounds; blue accent/numbers; purple entity commands; text-colored delimiters; comment gray |
| Catppuccin | [Official palette](https://github.com/catppuccin/palette), Latte/Mocha | Base/mantle backgrounds; mauve accent/commands; teal delimiters; peach numbers; overlay1 comments |
| Solarized | [VS Code](https://github.com/microsoft/vscode), Solarized Light/Dark | Editor/sidebar backgrounds; function blue accent/commands; keyword green delimiters; numeric magenta; comments |
| Atom One | [One Light](https://github.com/akamud/vscode-theme-onelight), [One Dark](https://github.com/akamud/vscode-theme-onedark) | Editor/sidebar backgrounds; focus accent; keyword commands; text-colored delimiters; numbers/comments |
| Rosé Pine | [Rosé Pine for VS Code](https://github.com/rose-pine/vscode), Dawn/main | Editor/widget backgrounds; iris accent/commands; punctuation delimiters; numeric rose; comments |
| VS Code Modern | [VS Code](https://github.com/microsoft/vscode), Light/Dark Modern including inherited Plus/VS palettes | Editor/sidebar backgrounds; link accent; keyword commands; operator delimiters; numbers/comments |
| Ayu | [Ayu colors](https://github.com/ayu-theme/ayu-colors), light/dark | Editor/base surfaces and accent; base orange keywords, peach/pink operators, purple numbers, gray comments; no generated luminance variants |
| Tokyo Night | [Tokyo Night](https://github.com/tokyo-night/tokyo-night-vscode-theme), light/night | Editor/sidebar backgrounds; link accent; keyword commands; punctuation delimiters; numbers/comments |
| Night Owl | [Night Owl](https://github.com/sdras/night-owl-vscode-theme), Light Owl/Night Owl | Editor/sidebar backgrounds; keyword accent/commands; operator delimiters; numbers/comments |
| Horizon | [Horizon](https://github.com/jolaleye/horizon-theme-vscode), Bright/dark | Editor/sidebar backgrounds; link accent; keyword commands; operator delimiters; constants/comments |
| Vitesse | [Vitesse](https://github.com/antfu/vscode-theme-vitesse), light/dark | Editor/sidebar backgrounds; link accent; keyword commands; punctuation delimiters; numbers/comments |
| Everforest | [Everforest](https://github.com/sainnhe/everforest-vscode), light/dark defaults | Editor/sidebar backgrounds; link accent; keyword commands; punctuation delimiters; numbers/comments |
| Gruvbox Material | [Gruvbox Material](https://github.com/sainnhe/gruvbox-material-vscode), light/dark defaults | Editor/sidebar backgrounds; link accent; keyword commands; punctuation delimiters; numbers/comments |
| Noctis | [Noctis](https://github.com/liviuschera/noctis), Lux/Noctis | Editor/sidebar backgrounds; link accent; keyword commands; text delimiters; numbers/comments |
| Min | [Min](https://github.com/miguelsolorio/min-theme), light/dark | Editor/sidebar backgrounds; function accent; keyword commands; text delimiters; numbers/comments; dark uses input foreground for neutral text |

- Every color is opaque sRGB. Upstream alpha is removed from RGB colors. Window opacity remains a separate background-only setting.
- The SwiftPM target copies this directory; `build.sh` includes the resource bundle in the signed app.
- To update: fetch pinned upstream files, review palette/token changes and license notices, update source metadata, and run the app checks/build.
