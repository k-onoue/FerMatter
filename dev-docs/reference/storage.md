# Storage and compatibility

## Stable identity

- Display/app/executable name: `Mathlidelux`; Swift target: `SlideMath`.
- Bundle ID: `local.slidemath.prototype`; support directory: `~/Library/Application Support/Slide Math`.
- Preserve existing keys and paths across branding changes; saved roots can contain absolute paths.
- Source: [AppIdentity.swift](../../macos-app/Sources/SlideMath/AppIdentity.swift), [ProjectStorage.swift](../../macos-app/Sources/SlideMath/ProjectStorage.swift).

## Locations

| Location under Application Support/Slide Math | Contents |
| --- | --- |
| `Projects/<project-id>/project.json` | Settings, cards, order, outline, labels, export options |
| `Projects/<project-id>/Cache/` | SVG/PDF/PNG and compiler logs |
| `Projects/<project-id>/Exports/` | Images at selected export sizes |
| `Projects/<project-id>/Documents/` | Compiled PDFs and navigation snapshots; older snapshots can accumulate |
| `Imports/` | Extracted ZIP projects |
| `Sources/` | Legacy app-created source projects; existing roots remain valid |
| `Settings/` | Shortcut and theme JSON with last-valid recovery files |
| `SourceDrafts/` | Recoverable unsaved source buffers |
| `templates.json` | User templates |

- Library project ID: first 24 hex characters of the root-path SHA-256; draft storage uses the full root hash.
- Opened folders stay in place; opening a main file uses its containing folder.
- New projects default to `~/Documents/Mathlidelux`; existing roots are not automatically migrated.
- [Settings files](settings-files.md) documents the schemas and legacy shortcut migration.

## Boundaries

- Library edit → custom presentation variant in metadata; original source card/manuscript stays intact.
- Source edit → actual project file; compare original bytes before atomic write.
- Recovered drafts → no automatic write until edited or explicitly saved.
- ZIP import → validate paths, symlinks, and size before extraction.
- Compiler render → temporary project copy; preserve imports, packages, and relative paths.
- Avoid resetting old projects because a new optional field is missing; preserve compatible decoding defaults.
- Test migrations on disposable copies with older metadata and settings.
- Reference: [SemVer public API requirement](https://semver.org/lang/ja/); project compatibility policy: [releasing](../releasing.md#versioning).
