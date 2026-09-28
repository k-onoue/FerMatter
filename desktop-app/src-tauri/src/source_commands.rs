use crate::{files::*, storage::Storage};
use serde_json::{json, Value};
use std::fs;

pub fn operate(store: &Storage, ident: &str, v: &Value) -> Result<Value> {
    let _guard = store.writes.lock().unwrap();
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
            rename_no_replace(&source, &dest)?;
            let mut p = store.load(ident)?;
            let main = string(&p, "main").to_owned();
            if main == name || main.starts_with(&format!("{name}/")) {
                p["main"] = json!(format!(
                    "{}{}",
                    string(v, "destination"),
                    &main[name.len()..]
                ));
                store.save(ident, &p)?;
            }
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
            let source = safe(&root, name)?;
            trash::delete(source).map_err(err)?
        }
        _ => return Err("Unknown file operation".into()),
    };
    Ok(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;
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
