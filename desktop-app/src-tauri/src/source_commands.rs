use crate::{files::*, storage::Storage};
use serde_json::{json, Value};
use std::fs;

fn remap(name: &str, from: &str, to: &str) -> String {
    if name == from || name.starts_with(&format!("{from}/")) {
        format!("{to}{}", &name[from.len()..])
    } else {
        name.into()
    }
}

// A journal lets startup finish a rename interrupted between the filesystem and metadata writes.
pub fn recover_pending(store: &Storage, ident: &str) -> Result<()> {
    let _guard = store.writes.lock().unwrap();
    recover(store, ident)
}
fn recover(store: &Storage, ident: &str) -> Result<()> {
    let path = store.project_dir(ident)?.join("SourceMutation.json");
    if !path.exists() {
        return Ok(());
    }
    let journal = json_read(&path)?;
    let root = store.root(ident)?;
    let from = root.join(relative(string(&journal, "from"))?);
    let to = root.join(relative(string(&journal, "to"))?);
    if !from.exists() && to.exists() {
        json_write(&store.draft_path(ident)?, &journal["drafts"])?;
        store.save(ident, &journal["project"])?;
    } else if from.exists() && !to.exists() {
        json_write(&store.draft_path(ident)?, &journal["originalDrafts"])?;
        store.save(ident, &journal["originalProject"])?;
    } else {
        return Err(
            "An interrupted file move needs recovery; its journal and drafts have been kept."
                .into(),
        );
    }
    fs::remove_file(path).map_err(err)
}

pub fn operate(store: &Storage, ident: &str, v: &Value) -> Result<Value> {
    let _guard = store.writes.lock().unwrap();
    recover(store, ident)?;
    let root = store.root(ident)?;
    let action = string(v, "action");
    let name = string(v, "path");
    match action {
        "create_file" => {
            let dest = destination(&root, name)?;
            use std::io::Write;
            fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(dest)
                .map_err(err)?
                .write_all(string(v, "text").as_bytes())
                .map_err(err)?;
        }
        "create_folder" => {
            let dest = destination(&root, name)?;
            fs::create_dir(dest).map_err(err)?
        }
        "rename" | "move" => {
            let source = safe(&root, name)?;
            let dest = destination(&root, string(v, "destination"))?;
            if dest.starts_with(&source) {
                return Err("A folder cannot be moved inside itself.".into());
            }
            let original = store.load(ident)?;
            let mut p = original.clone();
            let target = string(v, "destination");
            p["main"] = json!(remap(string(&p, "main"), name, target));
            if let Some(candidates) = p["candidates"].as_array_mut() {
                for candidate in candidates {
                    if let Some(value) = candidate.as_str() {
                        *candidate = json!(remap(value, name, target));
                    }
                }
            }
            let draft_path = store.draft_path(ident)?;
            let original_drafts = store.read_drafts(ident)?["buffers"].clone();
            let mut drafts = serde_json::Map::new();
            for (path, value) in original_drafts.as_object().ok_or("Invalid source drafts")? {
                let next = remap(path, name, target);
                let mut value = value.clone();
                value["path"] = json!(next);
                drafts.insert(next, value);
            }
            let journal = store.project_dir(ident)?.join("SourceMutation.json");
            json_write(
                &journal,
                &json!({"from":name,"to":target,"project":p,"drafts":drafts,"originalProject":original,"originalDrafts":original_drafts}),
            )?;
            if let Err(error) = rename_no_replace(&source, &dest) {
                let _ = fs::remove_file(journal);
                return Err(error);
            }
            if let Err(error) =
                json_write(&draft_path, &json!(drafts)).and_then(|_| store.save(ident, &p))
            {
                // Roll back if possible. Otherwise leave the forward journal for deterministic recovery.
                if rename_no_replace(&dest, &source).is_ok()
                    && json_write(&draft_path, &original_drafts)
                        .and_then(|_| store.save(ident, &original))
                        .is_ok()
                {
                    let _ = fs::remove_file(&journal);
                }
                return Err(format!(
                    "The file move could not finish: {error}. Recovery data has been kept."
                ));
            }
            fs::remove_file(journal).map_err(err)?;
        }
        "copy" => {
            let source = safe(&root, name)?;
            let dest = destination(&root, string(v, "destination"))?;
            let staging =
                tempfile::tempdir_in(dest.parent().ok_or("Missing parent")?).map_err(err)?;
            let copy = staging.path().join("copy");
            if source.is_dir() {
                if dest.starts_with(&source) {
                    return Err("A folder cannot be copied inside itself.".into());
                }
                snapshot(&source, &copy, &Value::Null)?;
            } else {
                fs::copy(source, &copy).map_err(err)?;
            }
            rename_no_replace(&copy, &dest)?;
        }
        "trash" => {
            let drafts = store.read_drafts(ident)?;
            if drafts["buffers"].as_object().is_some_and(|buffers| {
                buffers
                    .keys()
                    .any(|path| path == name || path.starts_with(&format!("{name}/")))
            }) {
                return Err("Save or reload unsaved files before moving them to Trash. Drafts have been kept.".into());
            }
            let source = safe(&root, name)?;
            trash::delete(source).map_err(err)?
        }
        _ => return Err("Unknown file operation".into()),
    };
    let project = store.load(ident)?;
    Ok(json!({"main": project["main"], "candidates": project["candidates"]}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn move_keeps_unsaved_drafts_and_returns_new_main_without_writing_manuscript() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Project");
        fs::create_dir_all(root.join("source")).unwrap();
        fs::write(root.join("source/main.typ"), "$ original $").unwrap();
        let project = store.open(&root, Some("source/main.typ")).unwrap();
        let ident = string(&project, "id");
        let mut draft = read_buffer(&root, "source/main.typ").unwrap();
        draft["text"] = json!("$ unsaved $");
        json_write(
            &store.draft_path(ident).unwrap(),
            &json!({"source/main.typ":draft}),
        )
        .unwrap();
        let result = operate(
            &store,
            ident,
            &json!({"action":"move","path":"source","destination":"renamed"}),
        )
        .unwrap();
        assert_eq!(result["main"], "renamed/main.typ");
        let drafts = store.read_drafts(ident).unwrap();
        assert_eq!(drafts["buffers"]["renamed/main.typ"]["text"], "$ unsaved $");
        assert!(drafts["buffers"].get("source/main.typ").is_none());
        assert_eq!(
            fs::read_to_string(root.join("renamed/main.typ")).unwrap(),
            "$ original $"
        );
        assert!(operate(&store, ident, &json!({"action":"trash","path":"renamed"})).is_err());
        assert!(root.join("renamed/main.typ").exists());
    }
    #[test]
    fn interrupted_move_finishes_drafts_and_main_before_the_next_operation() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Project");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.typ"), "$ original $").unwrap();
        let project = store.open(&root, None).unwrap();
        let ident = string(&project, "id");
        let mut next = project.clone();
        next["main"] = json!("moved.typ");
        let mut draft = read_buffer(&root, "main.typ").unwrap();
        draft["path"] = json!("moved.typ");
        draft["text"] = json!("$ latest $");
        let journal = store
            .project_dir(ident)
            .unwrap()
            .join("SourceMutation.json");
        json_write(&journal, &json!({"from":"main.typ","to":"moved.typ","project":next,"drafts":{"moved.typ":draft},"originalProject":project,"originalDrafts":{}})).unwrap();
        fs::rename(root.join("main.typ"), root.join("moved.typ")).unwrap();
        recover_pending(&store, ident).unwrap();
        assert_eq!(store.load(ident).unwrap()["main"], "moved.typ");
        assert_eq!(
            store.read_drafts(ident).unwrap()["buffers"]["moved.typ"]["text"],
            "$ latest $"
        );
        assert!(!journal.exists());
    }
    #[test]
    fn interrupted_rollback_restores_original_metadata_and_keeps_collision_journal() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Project");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.typ"), "$ x $").unwrap();
        let project = store.open(&root, None).unwrap();
        let ident = string(&project, "id");
        let mut next = project.clone();
        next["main"] = json!("moved.typ");
        let journal = store
            .project_dir(ident)
            .unwrap()
            .join("SourceMutation.json");
        let record = json!({"from":"main.typ","to":"moved.typ","project":next,"drafts":{},"originalProject":project,"originalDrafts":{}});
        json_write(&journal, &record).unwrap();
        store.save(ident, &next).unwrap();
        recover_pending(&store, ident).unwrap();
        assert_eq!(store.load(ident).unwrap()["main"], "main.typ");
        json_write(&journal, &record).unwrap();
        fs::write(root.join("moved.typ"), "external").unwrap();
        assert!(recover_pending(&store, ident).is_err());
        assert!(journal.exists());
        assert_eq!(
            fs::read_to_string(root.join("moved.typ")).unwrap(),
            "external"
        );
    }
    #[test]
    fn moving_a_main_file_parent_updates_project_without_touching_variants() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Project");
        fs::create_dir_all(root.join("source")).unwrap();
        fs::write(root.join("source/main.typ"), "$ x $").unwrap();
        let mut project = store.open(&root, Some("source/main.typ")).unwrap();
        project["cards"] = json!([{"id":"custom", "tex":"variant"}]);
        let ident = string(&project, "id");
        store.save(ident, &project).unwrap();
        operate(
            &store,
            ident,
            &json!({"action":"move", "path":"source", "destination":"renamed"}),
        )
        .unwrap();
        let saved = store.load(ident).unwrap();
        assert_eq!(saved["main"], "renamed/main.typ");
        assert_eq!(saved["cards"], project["cards"]);
        assert_eq!(
            fs::read_to_string(root.join("renamed/main.typ")).unwrap(),
            "$ x $"
        );
        assert!(!root.join("source").exists());
    }
    #[test]
    fn source_copy_and_create_reject_overwrites_escapes_and_recursive_copies() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Project");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("main.typ"), "$ original $").unwrap();
        let project = store.open(&root, None).unwrap();
        let ident = string(&project, "id");
        operate(
            &store,
            ident,
            &json!({"action":"copy", "path":"main.typ", "destination":"copy.typ"}),
        )
        .unwrap();
        assert_eq!(
            fs::read(root.join("copy.typ")).unwrap(),
            fs::read(root.join("main.typ")).unwrap()
        );
        for request in [
            json!({"action":"copy", "path":"main.typ", "destination":"copy.typ"}),
            json!({"action":"create_file", "path":"main.typ", "text":"overwrite"}),
            json!({"action":"create_file", "path":"../escape.typ", "text":"escape"}),
        ] {
            assert!(operate(&store, ident, &request).is_err());
        }
        operate(
            &store,
            ident,
            &json!({"action":"create_folder", "path":"nested"}),
        )
        .unwrap();
        assert!(operate(
            &store,
            ident,
            &json!({"action":"copy", "path":"nested", "destination":"nested/child"})
        )
        .is_err());
        assert_eq!(
            fs::read_to_string(root.join("main.typ")).unwrap(),
            "$ original $"
        );
        assert!(!temp.path().join("escape.typ").exists());
    }
}
