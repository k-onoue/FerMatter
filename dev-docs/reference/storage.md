# Storage and compatibility

## Stable identity

- Display/app/executable: `FerMatter` (formerly `Mathlidelux`); bundle ID: `local.slidemath.prototype`.
- Renaming does not migrate data. Keep the existing Documents/Mathlidelux default, saved project roots, `mathlidelux.*` schemas/keys, SVG namespace, project UUID files and `MATHLIDELUX_*` environment variables.
- Bundled demo contents remain unchanged so first-launch migration can still recognize existing samples.
- Keep the same resident lock identity so launching an older app copy cannot create a second resident.
- Preserve `~/Library/Application Support/Slide Math` and existing absolute source/cache paths.
- Current metadata directory: `~/Library/Application Support/Slide Math/Rust Preview/`.
- The internal preview directory name stays unchanged to preserve already-saved work after promotion.
- Sources: [storage.rs](../../desktop-app/src-tauri/src/storage.rs), [files.rs](../../desktop-app/src-tauri/src/files.rs), [settings.rs](../../desktop-app/src-tauri/src/settings.rs).

## Locations

| Location under the current metadata directory | Contents |
| --- | --- |
| `Projects/<project-id>/project.json` | Cards, labels, outline, export settings and compatible unknown fields |
| `Projects/<project-id>/SourceMutation.json` | Temporary recovery journal for interrupted Source rename/move |
| `Projects/<project-id>/.last-valid-project.json` | Last valid metadata; explicit recovery preserves rejected originals as `.rejected-project-<hash>.json` |
| `Projects/<project-id>/Cache/` | Rendered images, previews and logs |
| `Projects/<project-id>/Exports/` | Images at selected export sizes |
| `Projects/<project-id>/Documents/` | Compiled PDFs and navigation snapshots |
| `Settings/` | Preferences, shortcuts and theme JSON; last-valid recovery copies |
| `SourceDrafts/` | Recoverable source buffers |
| `MigratedSwiftDrafts/` | Original Swift draft backups and import markers |
| `templates.json` | User templates |
| `Setup.json` | Required first-launch folder selection, demo destinations, and resumable setup journal |
| `Instant/session.json` | Project-free draft and deduplicated source/template history |
| `Instant/Cache/`, `Instant/Exports/` | Instant render artifacts and prepared drag/copy files |
| `Projects/<project-id>/Instant/` | Per-project Instant draft/history and artifacts; independent of global Instant |
| `Settings/instant.json` | Reusable global Instant templates and output defaults |

- Project ID: first 24 hex characters of the canonical root-path SHA-256; draft filename uses the full hash.
- Opened folders stay in place; opening a main file uses its containing folder.
- New projects, Finder drops, ZIP imports and duplication use `preferences.json.projectDirectory`; missing/empty values use `~/Documents/Mathlidelux/`. New Project can override the parent folder once.
- Folder drops copy originals, including hidden/Git files. Existing direct children are opened in place. Reject links/special files, destination-inside-source, oversize inputs and invalid projects before publication; stage copies and publish without replacing names.
- `ProjectHistory.json` preserves undo/redo groups across launches. Deletion uses Finder Trash and records resulting paths; source folder identity checks and no-replace moves protect external replacements. `ProjectHistoryData/` retains metadata, variants, Source drafts and Instant sessions for restoration, including when a new project reuses a deleted project’s path.
- Duplicates retain independent project JSON and saved drafts/Instant history; rendered artifacts can be reused from the retained original metadata cache.
- Existing legacy `Sources/` and `Imports/` folders remain valid and are not moved/deleted.

## First-launch project folder

- The app requires a native folder-picker choice before Projects or Instant can open; this runs on first launch after DMG/PKG installation, including older prototype upgrades.
- Save the chosen folder in `preferences.json.projectDirectory`; stage the four publication-checked demo ZIPs under it and publish a unique `Demos` directory without overwriting existing files.
- `Setup.json` journals publication before metadata/settings writes. Resume interrupted setup using the ownership token in `Demos/.mathlidelux-demos.json`; commit completion last. Corrupt setup records are kept and reported.
- Existing projects matching a built-in demo name and source contents receive independent copied metadata, variants, drafts, and Instant sessions. Changed sources and projects with symlinks are not migrated.
- Mark matching originals with `demoRelocatedTo` to show the new copies in Projects; preserve original source files, metadata, and cached artifacts. Explicitly opening the original restores it to the list.
- Completed setup never recreates deleted demos or repeats the folder picker. Changing the default folder later does not relocate existing projects.

## Import from Swift and previews

- Existing current-directory metadata wins; missing projects are read from the legacy support root.
- Match saved root aliases as well as IDs so variants survive canonical path changes.
- Copy missing templates/theme/shortcut files and recovery copies; keep originals untouched.
- When no current preferences exist, import Swift appearance, opacity, pinning and auto-save mode from its preferences plist.
- Import Swift drafts once per project: decode original bytes, preserve conflict hashes, BOM and CRLF.
- Current drafts win path collisions; back up all Swift drafts and report collisions with the backup location.
- Imported drafts never overwrite source files automatically. Saving checks the old source revision.
- Corrupt recovery data produces an error and is kept; it must not be replaced by an empty editor session.
- Manual Library order metadata is retained; the current UI displays source order.
- Instant stores schema-versioned drafts/history atomically. Invalid saved state is preserved and blocks replacement; a pending webview-local draft covers an interrupted save.
- Instant pending-recovery keys include the project ID; retain the original global key/path for compatibility. No source files move when opening either Instant mode.
- Recent output colors use the WebView-local `mathlidelux.color-history.v1` key, shared across Instant and Library editors and capped at 16 opaque HEX values. This convenience history is independent of saved drafts and `Settings/instant.json`; invalid or unavailable history never blocks rendering.
- The first global settings file is seeded from built-ins, existing shared templates, and a valid old Instant draft; keep the old session unchanged. Subsequent global defaults are independent of project templates.
- Do not edit one project simultaneously in old and new apps; their Library metadata is not synchronized.

## Boundaries

- Library edit → presentation variant in metadata; source card/manuscript unchanged.
- Source edit → actual file; original-byte conflict check before atomic write.
- ZIP import → validate paths, symlinks and size before extraction.
- Compiler → temporary copy preserving imports and relative resources; do not follow symlinks outside the snapshot.
- Test migrations on disposable data, including conflicting drafts and external edits.
- [Settings schemas](settings-files.md); [versioning](../releasing.md#versioning).

## Cache reclamation

- Reclaim only recognized render/export files and UUID document snapshots inside current application storage; never follow cache symlinks.
- Published cache budget: **2 GiB** across projects and Instant. Stage images privately and admit completed images/documents before exposing them. If protected data fills the budget, reject new output with a recoverable error; keep saved work and existing output files.
- Existing installations may already exceed the budget with protected files. Preserve them and refuse growth until enough eligible data can be reclaimed.
- Protect active requests, frontend-owned preview state, visible PDFs, native drags through their completion callback, and the last successful image clipboard. Closing a window releases its leases; the clipboard survives app restarts. Backend results have only a five-minute handoff lease and cannot replace a newer request’s lease. Closing PDF discards its cached document state; reopening rebuilds it.
- Protect artifact references in `project.json`, `.last-valid-project.json`, and project Undo metadata backups. Invalid metadata cancels cleanup rather than guessing its references.
- Clipboard mutation and reference publication are serialized; persist conservative old/new references before changing the system clipboard. Failed copies preserve the previous clipboard references.
- Start a **five-minute grace** when an artifact is used or released. Remove other artifacts older than 30 days; under budget pressure, remove oldest eligible artifacts first. Idle maintenance runs once a minute and during new output admission.
- Startup uses a **24-hour grace** and also reclaims abandoned UUID document directories and `.artifact-*` publication staging files.
- `CacheMaintenance.json` records reclaimed/retained bytes. SourceDrafts, variants/project JSON, Instant sessions/history, project Undo data, original project folders and legacy storage are never eviction targets.
- Unreferenced image previews regenerate from source/history after reclamation. A live document retains its matching source/navigation snapshot until released.
- Compiler work in progress is separate from published cache admission: at most two jobs, source snapshots up to 1 GiB each (Typst document work retains a second navigation copy), generated images/PDFs up to 100 MiB. External compilers are not an OS resource sandbox; interrupted working directories are reclaimed on a later startup.
- See [tests](../testing.md) and [issue #39](https://github.com/k-onoue/mathlidelux/issues/39).

## Closing and Source moves

- Close/switch flushes pending Source saves, recovery drafts and Library metadata. Manual save mode keeps source files untouched.
- Uncommitted Library editing is retained as `libraryDraft` in project metadata and reopened with the project. Cancel explicitly discards that editor draft.
- Main-window close hides the app after saving; Quit waits for all open editor windows. Save failures or a missing flush response cancel closing.
- Rename/move waits for active saves, journals old/new metadata and recovery paths, then updates the file, drafts and main-file selection. The next project request completes or rolls back interrupted work.
- Source Trash refuses files/folders containing unsaved drafts; save or explicitly reload those files first.

## Project metadata validation and recovery

- `project.json` uses `schemaVersion: 1`; unversioned Swift/preview files migrate on load without dropping unknown fields, cards, labels, or variants. Missing legacy collection fields receive empty defaults; missing card IDs receive deterministic IDs.
- Validate root/card/outline/label types before commands or UI consume them; reject unsupported versions. Metadata reads/writes are limited to 16 MB and reads reject links/special files.
- Successful saves retain a last-valid copy. Invalid or missing active metadata blocks ordinary saves; never silently start an empty replacement over a backup or invalid legacy project.
- Projects shows **Review recovery** while healthy projects remain usable. **Restore last valid backup** checks the original revision, keeps rejected bytes separately, and restores metadata only; source files and Source/Instant drafts remain untouched.
- Without a valid backup, retain the file for manual recovery. Legacy originals are never rewritten by recovery.
- Parsing errors identify the metadata file and structural field, not its source contents. A root render boundary offers **Reload view** for unexpected display failures.
