# Storage and compatibility

## Stable identity

- Display/app/executable: `Mathlidelux`; bundle ID: `local.slidemath.prototype`.
- Preserve `~/Library/Application Support/Slide Math` and existing absolute source/cache paths.
- Current metadata directory: `~/Library/Application Support/Slide Math/Rust Preview/`.
- The internal preview directory name stays unchanged to preserve already-saved work after promotion.
- Sources: [storage.rs](../../desktop-app/src-tauri/src/storage.rs), [files.rs](../../desktop-app/src-tauri/src/files.rs), [settings.rs](../../desktop-app/src-tauri/src/settings.rs).

## Locations

| Location under the current metadata directory | Contents |
| --- | --- |
| `Projects/<project-id>/project.json` | Cards, labels, outline, export settings and compatible unknown fields |
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

- At startup, reclaim only recognized render/export files and UUID document-snapshot directories inside current application storage; never traverse cache symlinks.
- Keep saved project artifact references, the last copied image paths recorded in `ClipboardArtifacts.json`, and artifacts younger than 24 hours.
- Remove other artifacts older than 30 days; above a 2 GiB target, remove the oldest eligible artifacts first. Protected/recent data can exceed the target.
- `CacheMaintenance.json` records reclaimed/retained bytes and whether protected data exceeds the target. Invalid saved metadata cancels cleanup rather than guessing its references.
- Drafts, variants/project JSON, SourceDrafts, Instant sessions/history, project undo data, original project folders and legacy storage are never eviction targets.
- No live-session eviction: visible PDFs and ongoing drags remain untouched. Full live leases and a hard per-session quota remain tracked in [issue #39](https://github.com/k-onoue/mathlidelux/issues/39).
- Regenerate unreferenced previews from source/history when requested after cleanup.
