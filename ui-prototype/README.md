# Mathlidelux — earlier browser prototype

A local HTML/CSS/JavaScript experiment for equation browsing, editing, and SVG
export. The [native Mac app](../macos-app/README.md) is the current implementation;
this earlier prototype does not implement local TeX project import or compilation.
The [product page](../docs/) is separate and has no npm dependencies.

## Run locally

From the repository root:

```sh
cd ui-prototype
npm ci
npm start
```

Open [localhost:4173](http://localhost:4173). Set `PORT` to use another port.
MathJax is installed locally and does not need a CDN at runtime.

## Demo data

The 30 cards contain 27 source equations from `examples/demo-paper/` and three
synthetic custom variants. They include source locations and nearby synthetic
text. Regenerate them from the repository root with:

```sh
python3 ui-prototype/scripts/build-samples.py
```

The script reuses the native worker's extractor rather than parsing a private
manuscript. Rendering in this browser experiment uses MathJax rather than TeX
Live. Demo macros are defined in `index.html`.

The prototype stores changes in localStorage under `slide-math-demo-v2`. Clear
that key to reset to the current fixture. Earlier prototype data is not loaded or
deleted by this version.

## Features

- Search and filter inline/display equations and source/custom cards.
- Double-click to edit, create or duplicate equations, and reorder custom cards.
- Inspect TeX source and context, export SVG, and record UI feedback.
- Keyboard shortcuts: Command/Ctrl-K to search, N for new, E to edit, R to review.

The older review controls and some labels remain in Japanese. Public product
copy and the current native app are in English.
