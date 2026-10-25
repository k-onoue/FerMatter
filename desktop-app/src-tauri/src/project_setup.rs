use crate::{
    files::*,
    project_history, project_location,
    settings::SettingsFiles,
    storage::{self, Storage},
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

// Only the publication-checked, allowlisted sample archives are embedded.
const DEMOS: &[(&str, &[u8])] = &[
    (
        "demo-paper",
        include_bytes!("../../../docs/downloads/demo-paper.zip"),
    ),
    (
        "demo-typst",
        include_bytes!("../../../docs/downloads/demo-typst.zip"),
    ),
    (
        "demo-tikz",
        include_bytes!("../../../docs/downloads/demo-tikz.zip"),
    ),
    (
        "demo-cetz",
        include_bytes!("../../../docs/downloads/demo-cetz.zip"),
    ),
];

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Setup {
    schema_version: u32,
    directory: PathBuf,
    demos: PathBuf,
    token: String,
    complete: bool,
    projects: Vec<DemoProject>,
}
#[derive(Serialize, Deserialize)]
struct DemoProject {
    name: String,
    source: Option<PathBuf>,
}
fn load(store: &Storage) -> Result<Option<Setup>> {
    let path = store.home.join("Setup.json");
    if !path.exists() {
        return Ok(None);
    }
    let setup: Setup = serde_json::from_value(json_read(&path)?).map_err(err)?;
    if setup.schema_version != 1
        || !setup.directory.is_absolute()
        || setup.demos.parent() != Some(setup.directory.as_path())
        || setup.projects.len() < DEMOS.len()
        || setup
            .projects
            .iter()
            .map(|p| &p.name)
            .collect::<HashSet<_>>()
            .len()
            != setup.projects.len()
        || DEMOS
            .iter()
            .any(|(name, _)| !setup.projects.iter().any(|p| p.name == *name))
        || uuid::Uuid::parse_str(&setup.token).is_err()
        || setup
            .projects
            .iter()
            .any(|p| relative(&p.name).is_err() || Path::new(&p.name).components().count() != 1)
    {
        return Err("The setup record is invalid and has been kept.".into());
    }
    Ok(Some(setup))
}
fn save(store: &Storage, setup: &Setup) -> Result<()> {
    json_write(
        &store.home.join("Setup.json"),
        &serde_json::to_value(setup).map_err(err)?,
    )
}
pub fn required(store: &Storage) -> Result<bool> {
    Ok(load(store)?.is_none_or(|setup| !setup.complete))
}
fn same_sources(a: &Path, b: &Path) -> bool {
    let compare = || -> Result<bool> {
        let list = |root: &Path| -> Result<Vec<String>> {
            for entry in walkdir::WalkDir::new(root)
                .into_iter()
                .filter_entry(|e| e.depth() == 0 || !ignored(&e.file_name().to_string_lossy()))
            {
                if entry.map_err(err)?.file_type().is_symlink() {
                    return Err("Demo contains symbolic links.".into());
                }
            }
            let mut paths = entries(root)?
                .into_iter()
                .filter(|(name, _, dir)| !dir && name != ".mathlidelux-project.json")
                .map(|(name, _, _)| name)
                .collect::<Vec<_>>();
            paths.sort();
            Ok(paths)
        };
        let names = list(a)?;
        if names != list(b)? {
            return Ok(false);
        }
        for name in names {
            if fs::read(safe(a, &name)?).map_err(err)? != fs::read(safe(b, &name)?).map_err(err)? {
                return Ok(false);
            }
        }
        Ok(true)
    };
    compare().unwrap_or(false)
}
fn prepare(store: &Storage, directory: &Path) -> Result<Setup> {
    let stage = tempfile::tempdir_in(directory).map_err(err)?;
    let archives = tempfile::tempdir().map_err(err)?;
    let recent = store.recent()?;
    let mut projects = Vec::new();
    for (name, bytes) in DEMOS {
        let zip = archives.path().join(format!("{name}.zip"));
        fs::write(&zip, bytes).map_err(err)?;
        let root = storage::import_zip(&zip, stage.path())?;
        let origins: Vec<PathBuf> = recent
            .iter()
            .filter_map(|p| {
                let source = PathBuf::from(string(p, "root"));
                (source.file_name().is_some_and(|n| n == *name) && same_sources(&source, &root))
                    .then_some(source)
            })
            .collect();
        projects.push(DemoProject {
            name: (*name).into(),
            source: origins.first().cloned(),
        });
        // Independently edited Library copies stay independent during migration.
        for origin in origins.into_iter().skip(1) {
            let copy = project_location::copy_folder(&root, stage.path(), name)?;
            projects.push(DemoProject {
                name: copy.file_name().unwrap().to_string_lossy().into_owned(),
                source: Some(origin),
            });
        }
    }
    let token = uuid::Uuid::new_v4().to_string();
    json_write(
        &stage.path().join(".mathlidelux-demos.json"),
        &json!({"token": token}),
    )?;
    let demos = (0..10_000)
        .map(|n| {
            directory.join(if n == 0 {
                "Demos".into()
            } else {
                format!("Demos ({n})")
            })
        })
        .find(|path| path.symlink_metadata().is_err())
        .ok_or("Too many demo folders.")?;
    let setup = Setup {
        schema_version: 1,
        directory: directory.to_owned(),
        demos,
        token,
        complete: false,
        projects,
    };
    // Journal before publication, so interrupted metadata/settings writes resume
    // with the same demo copies instead of overwriting edits or making duplicates.
    save(store, &setup)?;
    rename_no_replace(stage.path(), &setup.demos)?;
    Ok(setup)
}
pub fn finish(store: &Storage, settings: &SettingsFiles, chosen: &str) -> Result<Value> {
    project_location::validate_path(chosen)?;
    if chosen.is_empty() {
        return Err("Choose a project folder to continue.".into());
    }
    let directory = Path::new(chosen).canonicalize().map_err(err)?;
    if !directory.is_dir() {
        return Err("Choose a project folder to continue.".into());
    }
    let previous = load(store)?;
    if previous.as_ref().is_some_and(|p| p.complete) {
        return Err("Project setup is already complete. Change the folder in Settings.".into());
    }
    let mut setup = match previous {
        Some(setup)
            if setup.demos.is_dir()
                && json_read(&setup.demos.join(".mathlidelux-demos.json"))?["token"]
                    == setup.token =>
        {
            if setup.directory != directory {
                return Err(format!(
                    "Continue setup with {}. The prepared demos have been kept there.",
                    setup.directory.display()
                ));
            }
            setup
        }
        _ => prepare(store, &directory)?,
    };
    for demo in &setup.projects {
        let target = setup.demos.join(&demo.name);
        let ident = store.register(&target)?;
        let saved = store.project_dir(&ident)?.join("project.json");
        if !saved.exists() || json_read(&saved)?["setupToken"] != setup.token {
            if let Some(source) = demo
                .source
                .as_ref()
                .filter(|source| same_sources(source, &target))
            {
                project_history::copy_state(store, source, &target)?;
            } else {
                project_history::open_created(store, &target, None)?;
            }
            let mut project = store.load(&ident)?;
            project["setupToken"] = json!(setup.token);
            store.save(&ident, &project)?;
        }
    }
    let snapshot = settings.snapshot("preferences.json", false)?;
    let mut preferences = snapshot["value"].clone();
    preferences["projectDirectory"] = json!(directory);
    let revision = settings.save(
        "preferences.json",
        &preferences,
        string(&snapshot, "revision"),
    )?;
    for demo in &setup.projects {
        let target = setup.demos.join(&demo.name);
        if let Some(source) = demo
            .source
            .as_ref()
            .filter(|source| same_sources(source, &target))
        {
            let mut original = store.open(source, None)?;
            original["demoRelocatedTo"] = json!(target);
            store.save(string(&original, "id"), &original)?;
        }
    }
    let projects = store.recent()?;
    setup.complete = true;
    save(store, &setup)?;
    Ok(json!({"settings": {"value": preferences, "revision": revision}, "projects": projects}))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn store(base: &Path) -> (Storage, SettingsFiles) {
        let store = Storage::new(base.join("Data"), base.join("Legacy")).unwrap();
        let settings = SettingsFiles::new(&store.home);
        (store, settings)
    }
    fn sample(base: &Path, name: &str) -> PathBuf {
        let bytes = DEMOS.iter().find(|(n, _)| *n == name).unwrap().1;
        let zip = base.join(format!("{name}.zip"));
        fs::create_dir_all(base).unwrap();
        fs::write(&zip, bytes).unwrap();
        storage::import_zip(&zip, base).unwrap()
    }
    #[test]
    fn first_launch_requires_a_folder_and_installs_all_demos_once() {
        let temp = tempfile::tempdir().unwrap();
        let (store, settings) = store(temp.path());
        assert!(required(&store).unwrap());
        for invalid in ["", "relative", "/does-not-exist"] {
            assert!(finish(&store, &settings, invalid).is_err());
        }
        assert!(!store.home.join("Setup.json").exists());
        let base = temp.path().join("Chosen");
        fs::create_dir(&base).unwrap();
        let result = finish(&store, &settings, base.to_str().unwrap()).unwrap();
        assert!(!required(&store).unwrap());
        assert_eq!(
            result["settings"]["value"]["projectDirectory"],
            json!(base.canonicalize().unwrap())
        );
        let projects = result["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 4);
        for (name, _) in DEMOS {
            assert!(projects
                .iter()
                .any(|p| Path::new(string(p, "root")).ends_with(Path::new("Demos").join(name))));
            assert!(base.join("Demos").join(name).join("README.md").is_file());
        }
        assert!(finish(&store, &settings, base.to_str().unwrap()).is_err());
        let restarted = Storage::new(store.home.clone(), store.legacy.clone()).unwrap();
        assert!(!required(&restarted).unwrap());
        assert_eq!(restarted.recent().unwrap().len(), 4);
    }
    #[test]
    fn existing_demos_folder_is_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let (store, settings) = store(temp.path());
        let existing = temp.path().join("Demos");
        fs::create_dir(&existing).unwrap();
        fs::write(existing.join("keep.txt"), "keep").unwrap();
        finish(&store, &settings, temp.path().to_str().unwrap()).unwrap();
        assert_eq!(
            fs::read_to_string(existing.join("keep.txt")).unwrap(),
            "keep"
        );
        assert!(temp.path().join("Demos (1)/demo-typst/main.typ").is_file());
    }
    #[test]
    fn migration_preserves_variants_drafts_instant_and_legacy_sources() {
        let temp = tempfile::tempdir().unwrap();
        let (store, settings) = store(temp.path());
        let source = sample(&temp.path().join("Old"), "demo-typst");
        let mut original = store.open(&source, None).unwrap();
        original["cards"] = json!([{"id":"custom-example", "origin":"custom", "tex":"x+y"}]);
        original["sections"] = json!([{"id":"label", "name":"Talk"}]);
        let ident = string(&original, "id");
        store.save(ident, &original).unwrap();
        let legacy = store
            .legacy
            .join("Projects")
            .join(ident)
            .join("project.json");
        json_write(&legacy, &original).unwrap();
        let drafts =
            json!({"main.typ":{"path":"main.typ", "text":"$ unsaved $", "revision":"old"}});
        json_write(&store.draft_path(ident).unwrap(), &drafts).unwrap();
        let session = json!({"draft":{"source":"local"}});
        json_write(
            &store
                .project_dir(ident)
                .unwrap()
                .join("Instant/session.json"),
            &session,
        )
        .unwrap();
        let before = fs::read(source.join("main.typ")).unwrap();
        let result = finish(&store, &settings, temp.path().to_str().unwrap()).unwrap();
        let projects = result["projects"].as_array().unwrap();
        assert_eq!(projects.len(), 4); // Legacy metadata cannot resurrect the old demo.
        let target = projects
            .iter()
            .find(|p| {
                Path::new(string(p, "root"))
                    .file_name()
                    .is_some_and(|name| name == "demo-typst")
            })
            .unwrap();
        assert_eq!(target["cards"], original["cards"]);
        assert_eq!(target["sections"], original["sections"]);
        let new_id = string(target, "id");
        assert_eq!(store.read_drafts(new_id).unwrap()["buffers"], drafts);
        assert_eq!(
            json_read(
                &store
                    .project_dir(new_id)
                    .unwrap()
                    .join("Instant/session.json")
            )
            .unwrap(),
            session
        );
        assert_eq!(fs::read(source.join("main.typ")).unwrap(), before);
        assert_eq!(json_read(&legacy).unwrap(), original);
        store.open(&source, None).unwrap(); // Explicitly reopening the original remains possible.
        assert_eq!(store.recent().unwrap().len(), 5);
    }
    #[cfg(unix)]
    #[test]
    fn modified_or_linked_sources_are_not_reclassified_as_built_in_demos() {
        let temp = tempfile::tempdir().unwrap();
        let (store, settings) = store(temp.path());
        let changed = sample(&temp.path().join("Old"), "demo-typst");
        fs::write(changed.join("main.typ"), "$ my work $").unwrap();
        store.open(&changed, None).unwrap();
        let linked = sample(&temp.path().join("Old"), "demo-paper");
        std::os::unix::fs::symlink(changed.join("main.typ"), linked.join("extra.typ")).unwrap();
        store.open(&linked, None).unwrap();
        finish(&store, &settings, temp.path().to_str().unwrap()).unwrap();
        assert_eq!(store.recent().unwrap().len(), 6);
        assert_eq!(
            fs::read_to_string(changed.join("main.typ")).unwrap(),
            "$ my work $"
        );
    }
    #[test]
    fn interrupted_setup_resumes_without_overwriting_completed_demo_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let (store, settings) = store(temp.path());
        let base = temp.path().canonicalize().unwrap();
        let setup = prepare(&store, &base).unwrap();
        let mut demo =
            project_history::open_created(&store, &setup.demos.join("demo-typst"), None).unwrap();
        demo["setupToken"] = json!(setup.token);
        demo["cards"] = json!([{"id":"custom-retained", "tex":"keep"}]);
        store.save(string(&demo, "id"), &demo).unwrap();
        assert!(required(&store).unwrap());
        let other = temp.path().join("Other");
        fs::create_dir(&other).unwrap();
        assert!(finish(&store, &settings, other.to_str().unwrap()).is_err());
        finish(&store, &settings, base.to_str().unwrap()).unwrap();
        assert_eq!(
            store.load(string(&demo, "id")).unwrap()["cards"],
            demo["cards"]
        );
        assert!(!base.join("Demos (1)").exists());
    }
    #[test]
    fn corrupt_setup_is_kept_and_cannot_be_skipped() {
        let temp = tempfile::tempdir().unwrap();
        let (store, settings) = store(temp.path());
        let file = store.home.join("Setup.json");
        fs::write(&file, "corrupt").unwrap();
        assert!(required(&store).is_err());
        assert!(finish(&store, &settings, temp.path().to_str().unwrap()).is_err());
        assert_eq!(fs::read_to_string(file).unwrap(), "corrupt");
    }
}
