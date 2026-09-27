# Native app architecture

The Swift package has no third-party dependencies. UI, workspace state, and
compiler transport stay separate; the Python workers own document extraction
and compilation. Resource paths and saved JSON formats remain compatible with
the existing prototype.

## Responsibilities

| Area | Files | Responsibility |
| --- | --- | --- |
| Application and library | `SlideMathApp.swift`, `LibraryStore.swift` | Scene wiring, library selection, labels, scan/render coordination and export actions. |
| Library storage | `ProjectStorage.swift`, `Models.swift` | Atomic metadata persistence, existing hashed project paths, recent projects, backward-compatible Codable models. |
| Compiler requests | `Worker.swift`, `WorkerSession.swift` | Python process transport and cancellation-safe request ownership. |
| Project source | `ProjectFiles.swift`, `FileOperations.swift` | Open buffers, conflict checks, draft recovery, auto save, file operations and Undo/Redo. |
| Source workspace UI | `ProjectFileTree.swift`, `ProjectFileViews.swift`, `SourcePaneLayout.swift`, `SourceTabInteractions.swift` | Native file tree, recursive splits, independent pane tabs, tab drag/drop and shortcuts. |
| Shared text editor | `SourceEditor.swift`, `AssistedTextView.swift` | SwiftUI/AppKit bridge, retained editor instances, syntax coloring, text Undo and input handling. |
| Syntax completion | `SourceCompletion.swift`, `ProjectCompletions.swift`, `SourceCompletionViews.swift` | Built-in and project-defined candidates, argument fields and completion popup. |
| Editor actions | `SourceEditingCommands.swift`, `SourceLineRuler.swift`, `SourceOutline.swift` | Line editing, logical line numbers and source heading navigation. |
| PDF and export | `PDFNavigation.swift`, `SVGExporter.swift`, `ImageExporter.swift` | PDF/source mapping and vector/raster representations. |
| App updates | `AppUpdates.swift`, `AppUpdateViews.swift`, `UpdateDownloader.swift` | SemVer and release validation, throttled in-app notifications, explicit download acceptance and disk-image handoff. |
| Window state | `WindowPreferences.swift`, `DesktopWallpaper.swift`, `RememberedSplitView.swift` | Appearance, opacity, wallpaper resolution and remembered pane widths. |

## Lifecycle rules

- `AppIdentity` supplies the Mathlidelux display name and the unchanged legacy
  support directory. The bundle identifier, persisted keys and source paths
  remain stable across the rename; the Swift target remains `SlideMath`.
- `Worker` emits ordered JSON events on the main queue. Each `WorkerSession`
  owns one request; replacing, cancelling or finishing it invalidates callbacks
  from that request. Library rendering, preview, font measurement, PDF building
  and PDF navigation use independent sessions. Completion may start the next
  request, so the session releases its old worker before calling completion.
- `ProjectFiles` owns shared file buffers and editor controllers. A controller
  retains its native scroll/text view to preserve caret, scroll and Undo across
  SwiftUI pane reconstruction. Text/focus callbacks hold the project weakly.
  Controllers are keyed by a typed pane/path pair, not a parsed string.
- Library variants live in `ProjectStorage`; Source mode writes real project
  files. Saving checks the original bytes before an atomic write. Auto save only
  queues edits made in the current session; restored drafts wait for editing or
  an explicit save. File renames also remap pending saves and open editors.
- The completion catalog supplies syntax assistance. It does not expose a
  separate Snippets menu. Library, source, template and new-project editors all
  use the same native text editor.

## Checks

From the repository root:

```sh
./macos-app/scripts/check.sh
./macos-app/scripts/build.sh
```

The first command runs Swift tests (compiler integration enabled), Python tests,
and publication checks. Compiler-dependent tests report skips when their required
tools are missing; install the dependencies in the README for a release pass.
The second builds and verifies the local ad hoc signature. Distribution signing
and notarization are separate steps in `PUBLISHING.md`.

Tests cover cancelled/late worker callbacks, legacy storage, project isolation,
file operations, auto-save conflicts, pane reconstruction, editor lifetimes,
completion, exports and the real LaTeX/Typst/TikZ/CeTZ workers. They use disposable
projects; personal manuscripts and build output are excluded from publication.
