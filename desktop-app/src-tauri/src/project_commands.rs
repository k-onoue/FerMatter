use crate::{
    files::*,
    native, project_history, project_location,
    settings::SettingsFiles,
    storage::{self, Storage},
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use tauri::Manager;

pub fn open(store: &Storage, settings: &SettingsFiles, v: &Value) -> Result<Value> {
    let path = PathBuf::from(string(v, "path"));
    if path.is_file()
        && path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"))
    {
        let _guard = store.writes.lock().unwrap();
        let mut history = project_history::History::load(store)?;
        let imported = storage::import_zip(
            &path,
            &project_location::directory(&settings.load("preferences.json")?, "")?,
        )?;
        let project = project_history::open_created(store, &imported, None)?;
        history.record_created(store, vec![imported], "Import Projects")?;
        return Ok(project);
    }
    let (root, main) = if path.is_file() {
        (
            path.parent().unwrap().to_owned(),
            Some(path.file_name().unwrap().to_string_lossy().into_owned()),
        )
    } else {
        (path, None)
    };
    let main = v["main"].as_str().map(str::to_owned).or(main);
    let ident = store.register(&root)?;
    let recovering = store
        .project_dir(&ident)?
        .join("SourceMutation.json")
        .exists();
    crate::source_commands::recover_pending(store, &ident)?;
    // Recent-project entries may still contain the pre-crash main path.
    store.open(&root, if recovering { None } else { main.as_deref() })
}

pub fn create(store: &Storage, settings: &SettingsFiles, v: &Value) -> Result<Value> {
    let _guard = store.writes.lock().unwrap();
    let mut history = project_history::History::load(store)?;
    let target = project_location::create(&settings.load("preferences.json")?, v)?;
    let typst = string(v, "format") == "typst";
    let main = if typst { "main.typ" } else { "main.tex" };
    let mut project = project_history::open_created(store, &target, Some(main))?;
    if !typst {
        let engine = string(v, "engine");
        if ["pdflatex", "xelatex", "lualatex"].contains(&engine) {
            project["engine"] = json!(engine);
        }
        store.save(string(&project, "id"), &project)?;
    }
    history.record_created(store, vec![target], "New Project")?;
    Ok(project)
}

pub fn action(
    app: &tauri::AppHandle,
    store: &Storage,
    settings: &SettingsFiles,
    v: &Value,
) -> Result<Value> {
    let _guard = store.writes.lock().unwrap();
    let mut history = project_history::History::load(store)?;
    let action = string(v, "action");
    let mut errors = Vec::new();
    let mut created = Vec::new();
    let parent = project_location::directory(&settings.load("preferences.json")?, "")?;
    let ensure_closed = |roots: &[PathBuf]| -> Result<()> {
        for root in roots {
            if store.home.starts_with(root) || store.legacy.starts_with(root) {
                return Err(
                    "This folder contains application data and cannot be deleted as a project."
                        .into(),
                );
            }
            if app
                .webview_windows()
                .keys()
                .filter_map(|label| label.strip_prefix("instant-"))
                .any(|ident| {
                    store
                        .root(ident)
                        .is_ok_and(|source| source.starts_with(root))
                })
            {
                return Err(
                    "Close the project's Instant window before deleting or restoring it.".into(),
                );
            }
        }
        Ok(())
    };
    match action {
        "import" => {
            let paths = v["paths"]
                .as_array()
                .ok_or("Drop project folders or ZIPs.")?;
            if paths.len() > 1000 {
                return Err("Import up to 1,000 projects at a time.".into());
            }
            for path in paths {
                let path = Path::new(path.as_str().ok_or("Invalid path")?);
                match project_location::import(path, &parent).and_then(|root| {
                    if path.canonicalize().ok().as_ref() == Some(&root) {
                        store.open(&root, None)?;
                    } else {
                        project_history::open_created(store, &root, None)?;
                    }
                    Ok(root)
                }) {
                    Ok(root) => {
                        if path.canonicalize().ok().as_ref() != Some(&root) {
                            created.push(root);
                        }
                    }
                    Err(error) => errors.push(format!("{}: {error}", path.display())),
                }
            }
            history.record_created(store, created.clone(), "Import Projects")?;
        }
        "duplicate" => {
            for root in project_history::roots(store, &v["ids"])? {
                match project_history::duplicate(store, &root, &parent) {
                    Ok(copy) => created.push(copy),
                    Err(error) => errors.push(format!("{}: {error}", root.display())),
                }
            }
            history.record_created(store, created.clone(), "Duplicate Projects")?;
        }
        "delete" => {
            let roots = project_history::roots(store, &v["ids"])?;
            ensure_closed(&roots)?;
            if let Err(error) = history.delete(store, roots, native::trash_project) {
                errors.push(error);
            }
        }
        "undo" | "redo" => {
            ensure_closed(&history.affected(action == "redo"))?;
            if let Err(error) = history.travel(store, action == "redo", native::trash_project) {
                errors.push(error);
            }
        }
        _ => return Err("Unknown project action.".into()),
    }
    Ok(
        json!({"projects":store.recent()?,"history":history.status(),"created":created,"errors":errors}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn create_and_open_keep_location_engine_and_history() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let settings = SettingsFiles::new(&store.home);
        let chosen = temp.path().join("Chosen");
        let mut preferences = settings.load("preferences.json").unwrap();
        preferences["projectDirectory"] = json!(chosen);
        settings
            .save(
                "preferences.json",
                &preferences,
                string(
                    &settings.snapshot("preferences.json", false).unwrap(),
                    "revision",
                ),
            )
            .unwrap();
        let input = json!({"name":"Example", "format":"latex", "text":"\\documentclass{article}\n\\begin{document}$x$\\end{document}", "engine":"xelatex"});
        let project = create(&store, &settings, &input).unwrap();
        let root = chosen.join("Example").canonicalize().unwrap();
        assert_eq!(project["root"], json!(root));
        assert_eq!(project["engine"], "xelatex");
        assert_eq!(
            project_history::History::load(&store).unwrap().status()["undo"],
            "New Project"
        );
        let reopened = open(&store, &settings, &json!({"path":root.join("main.tex")})).unwrap();
        assert_eq!(reopened["id"], project["id"]);
        assert_eq!(reopened["engine"], "xelatex");
        assert!(create(&store, &settings, &input).is_err());
        assert_eq!(
            std::fs::read_to_string(root.join("main.tex")).unwrap(),
            string(&input, "text")
        );
    }
}
