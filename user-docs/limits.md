# Limits and troubleshooting

## Missing equations

- Extraction follows literal source syntax; it does not execute the full document to discover every equation.
- LaTeX: common math delimiters and equation/align/gather/multline/displaymath/math environments; literal input/include paths.
- LaTeX generated paths, custom catcodes/environments, conditional definitions, and body-local macros can need manual work.
- Typst: literal `$…$` and `#include`; generated equations in loops/functions/containers/contextual code are not extracted.
- Typst document-wide templates, counters, state, labels, and queries may not reproduce in isolated exports.
- Repeated equations remain separate; inserting an identical earlier equation can change card matching.
- Use **Rescan** after external changes; inspect warnings in **Project Settings**.

## Failed rendering or navigation

- Check the [required tools](requirements.md), main file, compiler, packages, fonts, and relative imports.
- Open the equation/compiler log for the specific error.
- A failed PDF build retains the old PDF but disables navigation until rebuilding succeeds.
- Generated content can lack a PDF/source target; LaTeX equations on the same source line can share a target.
- Use an explicit input mode if [Auto](new-project.md) misidentifies pasted text.

## Not available

- PDF/OCR input, cloud sync, and automatic rescanning of externally edited sources.
- Fully bundled runtime dependencies and a signed/notarized public installer.
- Guaranteed drag compatibility with every presentation app; try copy/paste or an exported file.

## Local data

- Opened folders stay where they are; ZIPs are extracted into local app storage.
- Metadata, caches, exports, and drafts live under `~/Library/Application Support/Slide Math`.
- The legacy folder name preserves existing projects after the Mathlidelux rename.
- Removing the app does not remove your project sources or support data.
