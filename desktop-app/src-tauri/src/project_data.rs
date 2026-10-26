//! Validate stored metadata before it reaches commands or the webview.
use crate::files::*;
use serde_json::{json, Value};
#[cfg(test)]
use std::fs;
use std::{collections::HashSet, io::Read, path::Path};

const LIMIT: u64 = 16 * 1024 * 1024;
pub fn bytes(path: &Path) -> Result<Vec<u8>> {
    let file = regular_file(path).map_err(|_| "Cannot read metadata file.".to_string())?;
    let metadata = file
        .metadata()
        .map_err(|_| "Cannot inspect metadata file.".to_string())?;
    if !metadata.is_file() || metadata.len() > LIMIT {
        return Err("Project metadata must be a regular file smaller than 16 MB.".into());
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Cannot read metadata file.".to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("Project metadata exceeds 16 MB.".into());
    }
    Ok(bytes)
}
pub fn decode(bytes: &[u8]) -> Result<Value> {
    let value = serde_json::from_slice(bytes)
        .map_err(|_| "Invalid or incomplete project JSON.".to_string())?;
    normalize(value)
}
pub fn read(path: &Path) -> Result<Value> {
    bytes(path)
        .and_then(|bytes| decode(&bytes))
        .map_err(|reason| {
            format!(
                "{reason} File: {}. Existing metadata has not been replaced.",
                path.display()
            )
        })
}
fn strings(value: &Value) -> bool {
    value
        .as_array()
        .is_some_and(|items| items.iter().all(Value::is_string))
}
fn optional_strings(value: &Value, fields: &[&str]) -> Result<()> {
    for key in fields {
        if value.get(*key).is_some_and(|v| !v.is_string()) {
            return Err(format!("Invalid project field type: {key}."));
        }
    }
    Ok(())
}
fn optional_numbers(value: &Value, fields: &[&str]) -> Result<()> {
    for key in fields {
        if value
            .get(*key)
            .is_some_and(|v| !v.as_f64().is_some_and(|n| n.is_finite() && n >= 0.))
        {
            return Err(format!("Invalid project field type: {key}."));
        }
    }
    Ok(())
}
fn normalize_card(card: &mut Value, index: usize) -> Result<()> {
    if !card.is_object() {
        return Err("Each project card must be an object.".into());
    }
    optional_strings(
        card,
        &[
            "id", "name", "tex", "kind", "origin", "status", "file", "svgPath", "pngPath",
            "pdfPath", "error", "parentID",
        ],
    )?;
    optional_numbers(
        card,
        &[
            "line",
            "endLine",
            "sourceOffset",
            "sourceEnd",
            "startUTF16",
            "endUTF16",
            "sourceOrder",
            "sourceFontSize",
            "exportFontSize",
        ],
    )?;
    if !card["tex"].is_string() {
        return Err("Each project card needs source text.".into());
    }
    if card.get("id").is_none() {
        // Old unversioned cards occasionally omitted IDs. Derive a stable ID;
        // keep every original field and source byte, including private variants.
        card["id"] = json!(format!("legacy-{}", id(&format!("{index}:{}", card))));
    }
    if string(card, "id").is_empty() {
        return Err("Project card IDs must be nonempty.".into());
    }
    if card.get("outlinePath").is_some_and(|v| !strings(v)) {
        return Err("Card outline paths must be string arrays.".into());
    }
    if let Some(context) = card.get("typstContext") {
        if !context.as_array().is_some_and(|items| {
            items
                .iter()
                .all(|v| v.is_object() && v["file"].is_string() && v["offset"].as_u64().is_some())
        }) {
            return Err("Invalid Typst source context.".into());
        }
    }
    Ok(())
}
pub fn normalize(mut value: Value) -> Result<Value> {
    if !value.is_object() {
        return Err("Project metadata must be a JSON object.".into());
    }
    if value.get("schemaVersion").is_some_and(|v| v != 1) {
        return Err("Unsupported project metadata version.".into());
    }
    if value["root"].as_str().is_none_or(str::is_empty) {
        return Err("Project metadata needs a source folder.".into());
    }
    if !Path::new(string(&value, "root")).is_absolute() {
        return Err("Project source folder must be an absolute path.".into());
    }
    optional_strings(
        &value,
        &[
            "id",
            "root",
            "main",
            "engine",
            "format",
            "exportFormat",
            "demoRelocatedTo",
        ],
    )?;
    optional_numbers(&value, &["exportFontSize"])?;
    if value
        .get("format")
        .is_some_and(|v| !matches!(v.as_str(), Some("latex" | "typst")))
    {
        return Err("Unknown project source format.".into());
    }
    let cards = value["cards"]
        .as_array_mut()
        .ok_or("Project cards must be an array.")?;
    let mut ids = HashSet::new();
    for (index, card) in cards.iter_mut().enumerate() {
        normalize_card(card, index)?;
        if !ids.insert(string(card, "id").to_owned()) {
            return Err("Project card IDs must be unique.".into());
        }
    }
    if let Some(hidden) = value.get_mut("extractionHiddenCards") {
        let hidden = hidden
            .as_array_mut()
            .ok_or("Hidden extraction cards must be an array.")?;
        for (index, card) in hidden.iter_mut().enumerate() {
            normalize_card(card, index)?;
            if string(card, "origin") != "source" || !ids.insert(string(card, "id").to_owned()) {
                return Err("Hidden extraction cards must be unique source cards.".into());
            }
        }
    }
    if let Some(draft) = value
        .get_mut("libraryDraft")
        .filter(|draft| !draft.is_null())
    {
        // The editor draft can share an ID with the stored variant it is editing.
        normalize_card(draft, 0)?;
    }

    for key in [
        "outline",
        "sections",
        "customOrder",
        "candidates",
        "warnings",
        "files",
    ] {
        if value.get(key).is_none() {
            value[key] = json!([]);
        }
        if !value[key].is_array() {
            return Err(format!("Project {key} must be an array."));
        }
    }
    for key in ["customOrder", "candidates", "warnings", "files"] {
        if !strings(&value[key]) {
            return Err(format!("Project {key} must contain strings."));
        }
    }
    for heading in value["outline"].as_array().ok_or("Invalid outline.")? {
        if !heading.is_object()
            || ["id", "title", "file"]
                .iter()
                .any(|key| !heading[key].is_string())
            || !strings(&heading["path"])
            || heading["level"].as_u64().is_none()
            || heading["line"].as_u64().is_none()
        {
            return Err("Invalid project outline entry.".into());
        }
    }
    for section in value["sections"].as_array().ok_or("Invalid labels.")? {
        if !section.is_object() || !section["id"].is_string() || !section["name"].is_string() {
            return Err("Invalid project label.".into());
        }
    }
    if value.get("sectionAssignments").is_none() {
        value["sectionAssignments"] = json!({});
    }
    if !value["sectionAssignments"]
        .as_object()
        .is_some_and(|assignments| assignments.values().all(Value::is_string))
    {
        return Err("Project label assignments must map IDs to strings.".into());
    }
    value["schemaVersion"] = json!(1);
    Ok(value)
}
pub fn present(path: &Path) -> bool {
    // A dangling link or unreadable entry is not a missing project. Keep it for
    // recovery instead of treating it as permission to create empty metadata.
    !path
        .symlink_metadata()
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
}
pub fn backup(path: &Path) -> std::path::PathBuf {
    path.with_file_name(".last-valid-project.json")
}
pub fn write(path: &Path, value: &Value) -> Result<()> {
    let value = normalize(value.clone())?;
    let next = serde_json::to_vec_pretty(&value).map_err(err)?;
    if next.len() as u64 > LIMIT {
        return Err("Project metadata exceeds 16 MB.".into());
    }
    // Never let a normal save turn corrupted or future-version metadata into an
    // empty/new project. Explicit recovery first retains the rejected original.
    if present(path) {
        read(path)?;
    } else if present(&backup(path)) {
        return Err(format!(
            "Project metadata is missing. Restore its backup before saving. File: {}.",
            path.display()
        ));
    }
    atomic(path, &next)?;
    atomic(&backup(path), &next)
}
pub fn recover(path: &Path, expected: &str) -> Result<()> {
    let original = if path
        .symlink_metadata()
        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound)
    {
        if expected != "missing" {
            return Err("Project metadata changed. Refresh recovery before restoring.".into());
        }
        None
    } else {
        let original = bytes(path)?;
        if expected.is_empty() || hash(&original) != expected {
            return Err("Project metadata changed. Refresh recovery before restoring.".into());
        }
        if decode(&original).is_ok() {
            return Err("Project metadata is valid; no recovery is required.".into());
        }
        Some(original)
    };
    let recovered = read(&backup(path))?;
    if let Some(original) = original {
        let retained =
            path.with_file_name(format!(".rejected-project-{}.json", &hash(&original)[..16]));
        if retained.exists() && bytes(&retained)? != original {
            return Err("The recovery copy already exists with different content.".into());
        }
        atomic(&retained, &original)?;
    }
    atomic(path, &serde_json::to_vec_pretty(&recovered).map_err(err)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn project() -> Value {
        json!({"root":std::env::temp_dir().join("synthetic/project"), "cards":[{"id":"custom-1", "tex":"x+y", "origin":"custom", "extensions":{"example.keep":true}}], "extensions":{"future":{"value":7}}, "sectionAssignments":{"custom-1":"talk"}, "sections":[{"id":"talk", "name":"Talk"}]})
    }
    #[test]
    fn missing_active_file_does_not_replace_a_valid_backup_with_empty_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        write(&path, &project()).unwrap();
        fs::remove_file(&path).unwrap();
        let mut empty = project();
        empty["cards"] = json!([]);
        assert!(write(&path, &empty).is_err());
        recover(&path, "missing").unwrap();
        assert_eq!(read(&path).unwrap()["cards"], project()["cards"]);
    }
    #[test]
    fn invalid_documents_never_overwrite_original_or_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        let good = project();
        write(&path, &good).unwrap();
        let backup_before = fs::read(backup(&path)).unwrap();
        for broken in [
            "{unfinished".to_string(), "[]".into(),
            json!({"root":"/synthetic", "cards":{}}).to_string(),
            json!({"root":"/synthetic", "cards":[], "schemaVersion":999}).to_string(),
            json!({"root":"/synthetic", "cards":[{"id":"test", "tex": ["PRIVATE_SOURCE_SENTINEL"]}]}).to_string(),
            json!({"root":"/synthetic", "cards":[], "outline":{}}).to_string(),
        ] {
            fs::write(&path, broken.as_bytes()).unwrap();
            let error = read(&path).unwrap_err();
            assert!(error.contains(path.to_str().unwrap()));
            assert!(!error.contains("PRIVATE_SOURCE_SENTINEL"));
            assert!(write(&path, &good).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), broken);
            assert_eq!(fs::read(backup(&path)).unwrap(), backup_before);
        }
    }
    #[test]
    fn recovery_is_explicit_checks_revision_and_retains_rejected_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        write(&path, &project()).unwrap();
        let corrupt = b"{interrupted-save";
        fs::write(&path, corrupt).unwrap();
        assert!(recover(&path, "stale").is_err());
        assert_eq!(fs::read(&path).unwrap(), corrupt);
        recover(&path, &hash(corrupt)).unwrap();
        assert_eq!(read(&path).unwrap(), normalize(project()).unwrap());
        let retained =
            path.with_file_name(format!(".rejected-project-{}.json", &hash(corrupt)[..16]));
        assert_eq!(fs::read(retained).unwrap(), corrupt);
        assert!(recover(&path, &hash(fs::read(&path).unwrap())).is_err());
    }
    #[test]
    fn legacy_migration_retains_variants_and_unknown_fields() {
        let mut old = project();
        old["cards"][0].as_object_mut().unwrap().remove("id");
        let migrated = normalize(old.clone()).unwrap();
        assert_eq!(migrated["schemaVersion"], 1);
        assert_eq!(migrated["outline"], json!([]));
        assert_eq!(migrated["extensions"], old["extensions"]);
        assert_eq!(migrated["cards"][0]["tex"], old["cards"][0]["tex"]);
        assert_eq!(
            migrated["cards"][0]["extensions"],
            old["cards"][0]["extensions"]
        );
        assert_eq!(normalize(old).unwrap(), migrated);
        assert_eq!(normalize(migrated.clone()).unwrap(), migrated);
    }
    #[test]
    fn invalid_nested_types_are_rejected_without_echoing_values() {
        let mut p = project();
        p["cards"][0]["name"] = json!(["PRIVATE_SOURCE_SENTINEL"]);
        assert!(!normalize(p)
            .unwrap_err()
            .contains("PRIVATE_SOURCE_SENTINEL"));
        let mut p = project();
        p["cards"][0]["outlinePath"] = json!({});
        assert!(normalize(p).is_err());
        let mut p = project();
        p["sectionAssignments"] = json!([]);
        assert!(normalize(p).is_err());
    }
    #[test]
    fn library_editor_draft_is_validated_without_losing_unknown_fields() {
        let mut p = project();
        p["libraryDraft"] = p["cards"][0].clone();
        p["libraryDraft"]["tex"] = json!("unfinished +");
        let validated = normalize(p.clone()).unwrap();
        assert_eq!(validated["libraryDraft"], p["libraryDraft"]);
        p["libraryDraft"] = Value::Null;
        assert!(normalize(p.clone()).unwrap()["libraryDraft"].is_null());
        for draft in [
            json!("PRIVATE_SOURCE_SENTINEL"),
            json!({"id":"draft", "tex":["PRIVATE_SOURCE_SENTINEL"]}),
            json!({"id":"draft", "tex":"x", "name":[]}),
        ] {
            p["libraryDraft"] = draft;
            assert!(!normalize(p.clone())
                .unwrap_err()
                .contains("PRIVATE_SOURCE_SENTINEL"));
        }
    }
    #[cfg(unix)]
    #[test]
    fn dangling_metadata_links_are_not_treated_as_new_projects() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        let missing = dir.path().join("missing.json");
        std::os::unix::fs::symlink(&missing, &path).unwrap();
        assert!(present(&path));
        assert!(write(&path, &project()).is_err());
        assert_eq!(fs::read_link(&path).unwrap(), missing);
        assert!(!backup(&path).exists());
    }
    #[test]
    fn bounds_metadata_before_reading_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("project.json");
        let file = fs::File::create(&path).unwrap();
        file.set_len(LIMIT + 1).unwrap();
        assert!(read(&path).is_err());
        assert_eq!(fs::metadata(path).unwrap().len(), LIMIT + 1);
    }
}
