# Architecture

- Native SwiftUI/AppKit app; Swift package has no third-party dependencies.
- Python workers extract/compile LaTeX, Typst, TikZ, and CeTZ in temporary project copies.
- Sources: [Swift](../macos-app/Sources/SlideMath/), [workers](../macos-app/Resources/), [tests](../macos-app/Tests/).

## Responsibilities

| Area | Main files |
| --- | --- |
| Scenes, selection, render/export coordination | `SlideMathApp.swift`, `LibraryStore.swift` |
| Identity, saved metadata, model compatibility | `AppIdentity.swift`, `ProjectStorage.swift`, `Models.swift` |
| Process transport and request ownership | `Worker.swift`, `WorkerSession.swift` |
| Source buffers, saves, conflicts, file operations | `ProjectFiles.swift`, `FileOperations.swift` |
| File tree, split panes, tabs | `ProjectFileTree.swift`, `ProjectFileViews.swift`, `SourcePaneLayout.swift`, `SourceTabInteractions.swift` |
| Retained native text editor | `SourceEditor.swift`, `AssistedTextView.swift` |
| Completion and argument fields | `SourceCompletion.swift`, `ProjectCompletions.swift`, `SourceCompletionViews.swift` |
| Line actions and outline | `SourceEditingCommands.swift`, `SourceLineRuler.swift`, `SourceOutline.swift` |
| PDF mapping and image representations | `PDFNavigation.swift`, `SVGExporter.swift`, `ImageExporter.swift` |
| Settings persistence and themes | `SettingsFiles.swift`, `KeyboardShortcuts.swift`, `ThemePreferences.swift` |
| Release checks and accepted downloads | `AppUpdates.swift`, `AppUpdateViews.swift`, `UpdateDownloader.swift` |
| Caret previews and draft snapshots | `SourceMathPreview.swift`, `source_preview.py` |
| Window preferences and wallpaper | `WindowPreferences.swift`, `DesktopWallpaper.swift`, `RememberedSplitView.swift` |

## Request lifecycle

- `Worker`: Python process; request JSON in a temporary file; ordered JSON-line events on the main queue.
- `WorkerSession`: owns one request; replacement/cancellation/completion invalidates old callbacks.
- Release the old worker before invoking completion; completion may start another request.
- Keep independent sessions for library render, edit preview, measurement, PDF build, and navigation.
- Keep manuscript files unchanged during extraction/rendering; see [storage](reference/storage.md).

## Editor lifecycle

- `ProjectFiles` owns shared file buffers and editor controllers.
- Retain native scroll/text views across SwiftUI reconstruction to preserve caret, scroll, and Undo.
- Use typed pane/path keys; callbacks hold the project weakly.
- Save only after checking original bytes; write atomically; surface conflicts.
- Auto save applies to edits made this session; restored drafts wait for an edit or explicit save.
- Remap open editors and pending saves when files move or rename.
- Library variants and Source edits have different persistence targets; preserve that boundary.
- One native editor supports library, source, templates, and new-project input.

## Caret previews

- Each editor uses an independent debounced worker; caret or draft changes invalidate stale results.
- UTF-16 ranges match AppKit caret positions, including CRLF and non-BMP characters.
- Overlay open drafts in a temporary project copy without saving the manuscript.
- Cancellation terminates the preview compiler process group and removes the snapshot.
- [PR snapshot](history/pr-snapshot-2026-09-28.md) records the earlier integration plan.

## Limits and validation

- Source extraction is literal/static, not full TeX/Typst execution: [user limits](../user-docs/limits.md).
- `latexmkrc`/TeX are executable input; shell escape disabled does not constitute an OS sandbox.
- [Testing](testing.md), [reviewing](reviewing.md), [historical validation](history/validation-2026-09.md).
