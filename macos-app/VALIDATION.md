# Validation

## Project folder locations, 2026-09-27

- New Project defaults to `~/Documents/Mathlidelux` and displays the destination path.
- Show in Finder opens the selected project folder from both Projects and Project Settings.
- Missing, invalid, and failed-to-open paths produce an alert without opening a parent folder.
- Regression checks: 85 Swift tests and 28 Python tests passed, including three Finder-action tests.
- Rebuilt the app and verified the new destination display and both Finder entry points in the UI.

## Mathlidelux naming, 2026-09-27

- The app's window, menu and project-home title, local `.app` bundle and
  executable use Mathlidelux. Product-page metadata, copy, release asset name,
  documentation, browser prototype and all four sample ZIPs use the new name.
  Light/dark product screenshots were recaptured from the native app.
- The original bundle identifier and application-support directory are retained.
  Native launch confirms the existing project list and saved library still open;
  no data migration or preferences reset is needed.
- **82 Swift tests and 28 Python tests pass**, including real compiler checks.
  The renamed release app builds and passes local code-signature verification;
  publication checks cover 113 candidates, local links and all four sample ZIPs.

## Release preparation refactor, 2026-09-27

- **82 Swift tests pass**, with compiler integration enabled; **28 Python tests
  pass**. The publication checker passes for 112 candidates, local page links,
  and all four demo ZIPs. `git diff --check` is clean.
- Five compiler request lifecycles now share `WorkerSession`. Tests exercise
  late events after cancellation/replacement, duplicate completions, owner
  release, and starting the next request from completion. The Python process
  transport remains unchanged in its own file.
- `ProjectStorage` owns metadata I/O and the existing 24-character hash paths.
  Legacy metadata, project isolation, failed encoding, and corrupt-file reads
  are covered. No saved-data migration is required.
- Completion models/catalogs, popup views, the SwiftUI editor bridge, and the
  native text view have separate files. The former `SourceSnippet` type is now
  `SourceCompletion`, matching its retained syntax-assistance role.
- Pane editor identifiers use typed pane/path pairs. Text/focus callbacks no
  longer retain a closed project through its retained text view; a native
  offscreen lifetime regression test checks this along with the existing
  nested-pane, selection, scroll and Undo checks.
- Undo/Redo now routes native menu actions and Command-Z / Shift-Command-Z to
  the focused editor's independent history. Regression tests cover input,
  completion and line edits, binding updates, menu validation and isolation
  between editors. Native keyboard checks pass in both the library dialog and
  Source mode, including saving and restoring the exact source file bytes.
- The main headings are consistently Library / Source. The native window uses
  a transparent title bar with the background extended to its top edge; close,
  minimize and full-screen controls remain available. Native checks confirm
  the opaque top strip is gone and the wallpaper fallback still fills full screen.
- The release app builds and passes local code-signature verification. Native
  checks also confirm restored library cards, syntax-colored source with line
  numbers, a rendered equation preview and the three-page LaTeX PDF.
- The Typst sample remains at its current 20 equations. Its test and generated
  download ZIP now agree with those sources, resolving the discrepancy recorded
  below without restoring the removed equation. Demo ZIPs also include the
  current LaTeX sample edits without overwriting the working sources.
- `scripts/check.sh` runs the validation suite, and `ARCHITECTURE.md` documents
  responsibility boundaries and lifecycle rules. Distribution signing,
  notarization and clean-Mac installation remain separate release work.

## Editor controls and background, 0.10.0

- **74 Swift tests pass**, including real compiler integration. After removing
  the redundant path/language rows, all **37 affected regression tests pass**.
  The release builds and passes local code-signature verification.
- New editor tests cover whole-line copy/cut/paste, line duplication and movement,
  selection boundaries, Unicode and CRLF, repeated deletion back across a line
  break, comments, indentation, insertion, Undo, and logical line numbering.
  Both library and file editors use this shared native text view. Completion and
  project definition tests continue to pass; the manual Snippets UI is removed.
- Auto-save tests cover debounce/coalescing, explicit manual mode, external
  conflicts, pending saves after rename, and recovery without silently writing an
  old draft. Source saves retain BOM and line endings. Project switches and app
  deactivation flush pending automatic saves; conflicts retain the draft.
- A user-supplied capture of the updated native Source editor confirms visible
  wallpaper beneath the editor, opaque syntax-colored text, and aligned line
  numbers. The redundant filename/language rows below tabs were subsequently
  removed. Editor and equation-card backgrounds use the same glass modifier as
  the Source / All equations toolbar; foregrounds retain their original opacity.

## File tree and source panes, 0.9.0

- **67 Swift tests pass**, including compiler integration. The final tab changes
  also pass the 30 affected regression tests. Native/offscreen AppKit tests cover
  recursive horizontal/vertical layout, retained text views, selection and Undo,
  tab movement, close/save/cancel, drop zones, outline parsing, and shortcut
  persistence. File operations cover collision refusal, rollback, containment,
  clipboard moves/copies, path remapping, trash and undo/redo.
- Native UI checks on a disposable project confirm file copy/paste, inline
  rename with extension preservation, Undo Rename/Copy, Control-Tab and reverse
  cycling, Split Down, and tab drags creating additional panes.
- The current working-tree Typst sample already differs from its published ZIP:
  its last equation was removed before these changes. Python validation is
  **27 passed / 1 failed** because the fixture expects 21 equations and now finds
  20. The publication checker also reports that stale demo ZIP. The existing
  source edit and ZIP were not overwritten to make those checks pass.

## Project source editor and canvas selection, 0.8.0

- Swift: **54 tests pass** with compiler integration enabled; Python: **28 tests
  pass**. Coverage includes independent persisted PDF/Inspector widths, filtered
  selection and ranges, source draft recovery across projects, external file
  conflicts, safe file creation, symlink containment, binary rejection, and
  preservation of UTF-8 BOM / CRLF. Import-only completion entries no longer
  hide a project function's argument fields.
- Native UI on a disposable copy of the Typst demo: file tree and tabs open
  companion modules; unsaved main-file text survives tab changes. Command-S
  updates the actual file, grows the library from 21 to 22 rendered cards, and
  rebuilds the PDF with the new equation. Command-F opens native Find/Replace.
- PDF width changed from 500 to 520 points, then remained 520 after switching
  to Inspector and back and changing Source/Library mode. Divider dragging also
  resizes the PDF successfully.
- Command-A selects all 22 visible cards, Escape clears them, and Shift-arrow
  extends a range. Dragging from the canvas margin across two rows and columns
  selects exactly four cards. Bulk export writes two selected cards as separate
  SVG files in a disposable output folder. Typing `no` in the full source editor
  and accepting completion inserts `norm(x)` with its argument selected.
- The final focus and chooser changes pass the 14 source/selection regression
  tests. Inactive native editors are hidden inside a retained container, keeping
  their undo history without accepting invisible input. Bulk export attaches
  its folder chooser to the library window for use while Always on Top is on.
- Final native pass confirms hidden tabs are absent from the editable accessibility
  tree. With Always on Top enabled, Go to Folder accepts Return and all 22 selected
  images export successfully; the two earlier files remain alongside uniquely
  named copies. The app is left on the original demo in full screen with its
  wallpaper visible and the PDF width restored to 1,114 points.
- README, source-editing workflow, shortcuts, and local product-page text are
  updated. Publication checks cover 91 candidates and all four demo ZIPs.

## Wallpaper selection and inline equation bounds, 2026-09-27

- Swift: **46 tests pass** with compiler integration enabled; Python: **28 tests
  pass**. The release app builds successfully.
- Reproduced `NSWorkspace.desktopImageURL(for:)` returning `DefaultDesktop.heic`
  for all three connected displays while System Settings selected **Tahoe
  Evening**. An `NSVisualEffectView` alone remained dark in native full screen.
- The fallback now resolves the selected wallpaper from the user's read-only
  store, using display UUIDs rather than screen order. Tests cover external and
  internal displays, global selection precedence, photos, separate screen
  savers, ambiguous Spaces, unknown providers, missing assets, and rejection of
  default-image substitution. No wallpaper settings are modified.
- Native full-screen UI displays the selected Tahoe Evening aerial as a still
  frame under the background layers. The user also confirmed that the wallpaper
  remains visible with the MacBook lid closed and external monitors connected.
  Different wallpaper assignments across displays are covered by resolver
  fixtures, not by changing the user's desktop settings.
- Typst's default inline line box cut off the denominator in
  `v(t) = (dif x(t)) / (dif t)`. Glyph-bound text edges fix the generated image,
  preserving inline math style. A renderer cache revision and saved-preview
  check regenerate old output when the project opens.
- Real compiler tests check transparent margins around fractions, nested
  fractions, roots, scripts, sums, and vectors at 11 and 48 pt, matching SVG/PDF
  dimensions, and unchanged source bytes. Native UI shows the full denominator;
  all 22 cards in the existing Typst demo were regenerated successfully.
- The wallpaper fallback depends on the current macOS store format. It supports
  local photos and single downloaded Apple aerials, not arbitrary live providers
  or ambiguous per-Space shuffles. Aerials use a video frame, not a capture of the
  current desktop. Native full-screen entry from a floating window can still
  require temporarily turning Always on Top off.

## Image exports, completion, and window background, 0.7.0

- Swift: **38 tests pass** with `SLIDEMATH_COMPILER_TESTS=1`; Python: **28 tests
  pass**. Real workers compile LaTeX, Typst, TikZ, and CeTZ, all built-in math
  snippets, and custom templates. Existing source bytes remain unchanged.
- SVG remains the default and preserves original bytes when no size override is
  set. PDF export is vector; PNG/JPEG use 216 dpi with transparent/white canvases.
  Tests verify dimensions, vector PDF content, raster alpha/color, cache reuse,
  legacy PNG fallback, and file-provider representations for all four formats.
- Per-item font sizes override project defaults, survive saving a previously
  rendered draft and reopening the library, and can reset to Original or Default.
  Figures retain original size by default and support explicit item scaling.
- Completion tests cover Unicode selections, editable argument fields, Tab and
  Shift-Tab navigation, Undo, non-destructive suggestions, project overrides,
  commented-out definitions, companion files, and optional arguments with braces.
- Native UI: typing `\f` shows `\frac`; Tab accepts and moves between numerator
  and denominator. `\ve` suggests the demo's `\vect` from `main.tex`; Return
  inserts its argument field. Typst `no` suggests `norm(x)` from `symbols.typ`.
  Escape dismisses suggestions without closing the editor. Menu insertion
  restores source focus even when the name field was previously focused.
- Both editor and inspector expose the item font-size picker. A 48 pt unsaved
  LaTeX preview drag generated its scaled SVG and a 502 × 225 PNG. External slide
  application drop behavior depends on that application's supported formats;
  this pass verifies the native drag handler and provider data, not a new
  PowerPoint round-trip.
- The initial full-screen image fallback used the public desktop-image URL.
  Subsequent validation found that modern aerials could return the system default;
  this is superseded by the wallpaper-selection fix above. Text remains opaque.
- Release 0.7.0 builds and passes local signature verification; publication
  checks cover **82 candidates**, local page links, and four source-only demo ZIPs.
  Build artifacts and the private manuscript remain excluded.

Completion uses a static local definition index; generated commands, package
introspection, advanced argument syntax, and complete scope/module resolution
are outside its supported scope. See the README for recognized definitions.

## Pasted projects and templates, 0.6.0

- Swift: 25 tests pass with `SLIDEMATH_COMPILER_TESTS=1`. Eight new tests cover
  input modes, comments and CRLF, complete LaTeX documents, source preservation,
  invalid input, safe folder names and collision handling, compiler selection,
  custom template persistence/defaults/deletion, and corrupt-settings preservation.
- Real workers compile all four built-in templates and custom LaTeX/Typst macros
  into SVGs. Existing source bytes are checked after rendering.
- Native UI: New Project and Shift-Command-N open the composer. Three pasted
  LaTeX lines produce three ready SVG cards. A custom Typst template was edited
  and saved in the template manager, selected in the composer, and used to create
  three ready cards with two source headings. Its PDF also compiles and displays
  the custom blackboard-bold macro correctly.
- The release app builds and passes local code-signature verification. New
  project and template editors were visually checked in the native dark UI.
- Auto detection is a convenience heuristic; Source and One per line are explicit
  overrides. Templates store source text and compiler settings, not asset folders.

## PDF navigation, shortcuts, and figures, 0.5.0

- **28 Python tests** and **17 Swift tests** pass. Real compiler tests include
  source immutability, cache reuse/invalidation, failed PDF rebuild preservation,
  source line/column selection, and preference persistence/conflict detection.
- LaTeX whole-document builds produce SyncTeX data; all 18 display equations in
  the public paper round-trip from source to PDF and back into their source range.
- Typst PDF instrumentation preserves the exact text bounding boxes and page
  layout of the math and CeTZ demos. Every extracted item and heading is mapped;
  repeated includes and explicit page breaks are covered. CeTZ drawing centers
  resolve to their source figure using measured bounds.
- Both drawing demos contain two colored figures and one inline equation. All six
  items render as SVG/PNG; figures retain vector paths and colors, edits invalidate
  their cache, and figure exports ignore the equation font-size override.
- TikZ environments keep nested math/scopes together. CeTZ module imports,
  wildcard imports, renamed canvases, and figure wrappers are covered. Source
  snippets in comments, raw blocks, and definitions are excluded.
- Native TikZ import and PDF display were checked. Tight preview pages preserve
  the triangle apex and all three vertex labels; a size regression test covers
  the earlier clipping bug.
- Native CeTZ import, Figures filtering, Edit Figure preview, Command-S save,
  PDF display, and a custom PDF-toggle shortcut were checked. Duplicate shortcut
  assignment reports the conflicting action. The test assignment was restored.
- The native LaTeX PDF pane loads the three-page math demo; Outline selection
  navigates to its last section. PDF controls expose page, zoom, and rebuild.
- Release build and local signature verification pass after bundled worker use.
- Four source-only demo ZIPs and local product-page links pass publication checks.

PDF navigation follows compiler source information, not OCR. A SyncTeX result
without a column can be ambiguous for multiple inline items on one source line.
Generated content and complex third-party document templates remain outside the
validated extraction scope. Native Command-click hit testing has not been driven
by the UI automation; the same coordinate mapping and backend reverse lookup are
covered by tests.

## Typst and source outline, 0.4.0

- Typst 0.14.2: the full sample compiles, and all **21 equations** render as SVG
  paths with transparent previews. Includes, relative imports, definitions, and
  per-equation text sizes (11/12 pt) are exercised.
- Typst errors are isolated per equation; cache reuse, invalidation after imports
  change, source immutability, and 900 px previews are verified with the real CLI.
- **19 Python tests** and **12 Swift tests** pass. The latter include legacy data,
  project separation, Typst scaling, source context, and outline/label persistence.
- LaTeX headings from part to subparagraph, starred/optional titles, nested
  includes, duplicate titles, and exclusions for raw text/definitions are tested.
- Typst headings and literal includes form the same outline model. Parent filters
  include descendant equations, and collapse state and manual labels persist.
- The TeX demo now has **4 sections and 4 subsections**, with 27 equations.
- Native Typst import, double-click editing, preview, custom save, label assignment,
  24 pt SVG copy, project switching, and settings persistence were checked.
- Native outline migration was checked on a saved LaTeX library: parent selection
  returns 8 equations, subsection selection returns 4, and collapse changes only
  the tree. Cached rendered cards remain ready.
- All 27 equations in the updated LaTeX sample render successfully.
- Bundled Typst scan/render passes with bytecode writes disabled, and the app’s
  local signature remains valid after worker execution.
- Both demo ZIPs use explicit source allowlists and pass publication checks.
  The product page displays separate LaTeX and Typst downloads.

## Earlier native prototype, 0.3.0

Validated on macOS 26.5.2 with Xcode 26.6 (17F113), Swift 6.3.3, macOS SDK 26.5,
and TeX Live 2026.

- Release build, application bundle creation, local signing, and native launch.
- Native SwiftUI Liquid Glass controls, project switching, equation editing,
  section assignment and persistence, and project-specific export size.
- System / Light / Dark, always-on-top, and background opacity. Window alpha stays
  at 1; only background layers are faded.
- Nine Swift tests covering saved-data compatibility, sections, project isolation,
  window configuration, and vector-preserving SVG dimension changes.
- Six Python tests covering extraction delimiters, comments, duplicates, input
  ordering, stable IDs after line insertion, numbering removal, and ZIP traversal.
- TeX font metrics measured on a 12 pt fixture and recovered from cached SVGs.
- A 24 pt SVG pasted into PowerPoint as vector graphics, with Convert to Shape
  available and the expected physical dimensions.

## Public demonstration, 2026-09-27

- `examples/demo-paper/main.tex` and four input files yield **27 equations**:
  **9 inline** and **18 block**, with no extraction warnings.
- All **27** render successfully through the project's pdfLaTeX configuration.
- The sample loads and renders in the native app.
- Page screenshots show this sample only, with fully opaque backgrounds.
- The earlier browser prototype's 30-card fixture contains these 27 source
  equations plus three synthetic custom variants.
- Private manuscripts and their development screenshots are excluded from Git.
  The downloadable sample uses an explicit seven-file source allowlist.

## Not yet verified for distribution

- A clean Mac without development tools or the external TeX dependencies.
- Developer ID signing, notarization, DMG installation, and automatic updates.
- Intel builds and macOS versions earlier than 26.
- Native card-to-PowerPoint drag and drop end to end.
- XeLaTeX / LuaLaTeX, Japanese document classes, and arbitrary third-party templates.
- Fully general TeX macro execution and document cross-references.

Private-project logs and captures are not part of this repository.

## Product page handoff

- Static site checked at desktop width and a 390 px mobile viewport, with no
  horizontal overflow. Light/dark switching and the requirements disclosure work.
- The sample ZIP is served successfully and matches the seven allowlisted sources.
- Configured release and setup links were checked locally without navigating to
  GitHub. The final configuration has no repository set and keeps app downloads
  disabled until a distribution build is available.
- At the original handoff, the synthetic paper compiled as a two-page PDF, and the six Python
  extraction tests pass. No native source code changed during publication setup.
