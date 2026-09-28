//! Startup-only reclamation. Never delete live-session artifacts, saved drafts or history.
use crate::files::*;
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

const BUDGET: u64 = 2 * 1024 * 1024 * 1024;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);
const GRACE: Duration = Duration::from_secs(24 * 3600);
struct Item {
    path: PathBuf,
    bytes: u64,
    modified: SystemTime,
    directory: bool,
}
fn plain_directory(path: &Path) -> bool {
    path.symlink_metadata()
        .is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
}
fn collect(folder: &Path, documents: bool, items: &mut Vec<Item>) -> Result<()> {
    if !plain_directory(folder) {
        return Ok(());
    }
    for entry in fs::read_dir(folder).map_err(err)? {
        let entry = entry.map_err(err)?;
        let path = entry.path();
        let meta = path.symlink_metadata().map_err(err)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if documents
            && meta.is_dir()
            && name
                .strip_prefix("document-")
                .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok())
        {
            let mut bytes = 0;
            let mut modified = meta.modified().map_err(err)?;
            let mut safe = true;
            for child in walkdir::WalkDir::new(&path) {
                let child = child.map_err(err)?;
                if child.file_type().is_symlink()
                    || !(child.file_type().is_dir() || child.file_type().is_file())
                {
                    safe = false;
                    break;
                }
                let meta = child.metadata().map_err(err)?;
                if meta.is_file() {
                    bytes += meta.len();
                }
                modified = modified.max(meta.modified().map_err(err)?);
            }
            if safe {
                items.push(Item {
                    path,
                    bytes,
                    modified,
                    directory: true,
                });
            }
        } else if !documents && meta.is_dir() && ["Previews", "PDFArtwork"].contains(&name.as_str())
        {
            collect(&path, false, items)?;
        } else if !documents
            && meta.is_file()
            && name
                .get(..24)
                .is_some_and(|p| p.bytes().all(|c| c.is_ascii_hexdigit()))
            && ["svg", "png", "jpg", "pdf", "log"]
                .contains(&path.extension().and_then(|s| s.to_str()).unwrap_or(""))
        {
            items.push(Item {
                path,
                bytes: meta.len(),
                modified: meta.modified().map_err(err)?,
                directory: false,
            });
        }
    }
    Ok(())
}
fn references(value: &Value, pinned: &mut HashSet<PathBuf>) {
    match value {
        Value::String(s) if Path::new(s).is_absolute() => {
            pinned.insert(PathBuf::from(s));
        }
        Value::Array(values) => {
            for value in values {
                references(value, pinned);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                references(value, pinned);
            }
        }
        _ => {}
    }
}
pub fn remember_clipboard(home: &Path, paths: &[PathBuf]) -> Result<()> {
    json_write(&home.join("ClipboardArtifacts.json"), &json!(paths))
}
pub fn maintain(home: &Path) -> Result<Value> {
    maintain_with(home, SystemTime::now(), BUDGET)
}
fn maintain_with(home: &Path, now: SystemTime, budget: u64) -> Result<Value> {
    let mut items = vec![];
    let mut pinned = HashSet::new();
    let clipboard = home.join("ClipboardArtifacts.json");
    if clipboard.exists() {
        references(&json_read(&clipboard)?, &mut pinned);
    }
    let mut scopes = vec![home.to_path_buf()];
    let projects = home.join("Projects");
    if plain_directory(&projects) {
        for entry in fs::read_dir(&projects).map_err(err)? {
            let entry = entry.map_err(err)?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.len() != 24
                || !name.bytes().all(|b| b.is_ascii_hexdigit())
                || !plain_directory(&entry.path())
            {
                continue;
            }
            let project = entry.path().join("project.json");
            // Fail closed if saved metadata cannot be read. It may reference a cache file.
            if project.exists() {
                references(&json_read(&project)?, &mut pinned);
            }
            scopes.push(entry.path());
        }
    }
    for scope in scopes {
        for folder in ["Cache", "Exports"] {
            collect(&scope.join(folder), false, &mut items)?;
            if plain_directory(&scope.join("Instant")) {
                collect(&scope.join("Instant").join(folder), false, &mut items)?;
            }
        }
        collect(&scope.join("Documents"), true, &mut items)?;
    }
    items.sort_by_key(|item| item.modified);
    let mut retained: u64 = items.iter().map(|item| item.bytes).sum();
    let before = retained;
    let mut removed = 0;
    for item in items {
        let age = now.duration_since(item.modified).unwrap_or_default();
        let protected = pinned
            .iter()
            .any(|p| p == &item.path || (item.directory && p.starts_with(&item.path)));
        if protected || age < GRACE || (age < MAX_AGE && retained <= budget) {
            continue;
        }
        if item.directory {
            fs::remove_dir_all(&item.path).map_err(err)?;
        } else {
            fs::remove_file(&item.path).map_err(err)?;
        }
        retained -= item.bytes;
        removed += 1;
    }
    Ok(
        json!({"removedEntries":removed,"reclaimedBytes":before-retained,"retainedBytes":retained,"budgetBytes":budget,"overBudget":retained>budget}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_preserves_sources_drafts_history_references_and_clipboard() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let scope = home.join("Projects/aaaaaaaaaaaaaaaaaaaaaaaa");
        fs::create_dir_all(scope.join("Cache")).unwrap();
        fs::create_dir_all(scope.join("Exports")).unwrap();
        let pinned = scope.join("Cache/aaaaaaaaaaaaaaaaaaaaaaaa.svg");
        let copied = scope.join("Exports/bbbbbbbbbbbbbbbbbbbbbbbb-svg.svg");
        let expired = scope.join("Cache/cccccccccccccccccccccccc.svg");
        for path in [&pinned, &copied, &expired] {
            fs::write(path, "image").unwrap();
        }
        json_write(
            &scope.join("project.json"),
            &json!({"cards":[{"svgPath":pinned}]}),
        )
        .unwrap();
        remember_clipboard(home, &[copied.clone()]).unwrap();
        for path in [
            "SourceDrafts/a.json",
            "Instant/session.json",
            "ProjectHistoryData/a.json",
            "Projects/aaaaaaaaaaaaaaaaaaaaaaaa/Instant/session.json",
            "Sources/main.typ",
        ] {
            atomic(&home.join(path), b"keep").unwrap();
        }
        let doc = scope.join(format!("Documents/document-{}", uuid::Uuid::new_v4()));
        atomic(&doc.join("Source/main.typ"), b"snapshot").unwrap();
        let report = maintain_with(home, SystemTime::now() + MAX_AGE + GRACE, 1).unwrap();
        assert_eq!(report["removedEntries"], 2);
        assert!(pinned.exists() && copied.exists());
        assert!(!expired.exists() && !doc.exists());
        for path in [
            "SourceDrafts/a.json",
            "Instant/session.json",
            "ProjectHistoryData/a.json",
            "Projects/aaaaaaaaaaaaaaaaaaaaaaaa/Instant/session.json",
            "Sources/main.typ",
        ] {
            assert_eq!(fs::read(home.join(path)).unwrap(), b"keep");
        }
    }
    #[cfg(unix)]
    #[test]
    fn symlinked_cache_parent_cannot_reach_external_files() {
        let home = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let source = outside.path().join("Cache/aaaaaaaaaaaaaaaaaaaaaaaa.svg");
        atomic(&source, b"private original").unwrap();
        std::os::unix::fs::symlink(outside.path(), home.path().join("Instant")).unwrap();
        maintain_with(home.path(), SystemTime::now() + MAX_AGE + GRACE, 0).unwrap();
        assert_eq!(fs::read(source).unwrap(), b"private original");
    }
    #[test]
    fn young_incomplete_work_and_unknown_names_are_not_removed() {
        let temp = tempfile::tempdir().unwrap();
        atomic(
            &temp
                .path()
                .join("Instant/Cache/aaaaaaaaaaaaaaaaaaaaaaaa.svg"),
            b"new",
        )
        .unwrap();
        atomic(
            &temp.path().join("Instant/Cache/private.typ"),
            b"unrecognized",
        )
        .unwrap();
        assert_eq!(
            maintain_with(temp.path(), SystemTime::now(), 0).unwrap()["removedEntries"],
            0
        );
        assert_eq!(
            maintain_with(temp.path(), SystemTime::now() + MAX_AGE + GRACE, 0).unwrap()
                ["removedEntries"],
            1
        );
        assert!(temp.path().join("Instant/Cache/private.typ").exists());
    }
}
