use crate::{files::*, project_location, storage::Storage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
pub struct History {
    #[serde(default = "schema_version", rename = "schemaVersion")]
    schema_version: u32,
    undo: Vec<Action>,
    redo: Vec<Action>,
}
fn schema_version() -> u32 {
    1
}
impl Default for History {
    fn default() -> Self {
        Self {
            schema_version: 1,
            undo: vec![],
            redo: vec![],
        }
    }
}
#[derive(Serialize, Deserialize)]
struct Action {
    label: String,
    deletion: bool,
    items: Vec<Item>,
}
#[derive(Serialize, Deserialize)]
struct Item {
    root: PathBuf,
    trash: Option<PathBuf>,
    identity: String,
    #[serde(default)]
    backup: Option<String>,
}
fn state_paths(store: &Storage, root: &Path) -> Result<Vec<PathBuf>> {
    let root = root.canonicalize().map_err(err)?;
    let key = root.to_string_lossy();
    let metadata = store.project_dir(&id(&key))?;
    Ok(vec![
        metadata.join("project.json"),
        store
            .home
            .join("SourceDrafts")
            .join(format!("{}.json", hash(key.as_bytes()))),
        metadata.join("Instant/session.json"),
        // Append to keep the original three history slots compatible.
        metadata.join(".last-valid-project.json"),
    ])
}
fn backup_state(store: &Storage, root: &Path) -> Result<String> {
    let backup = uuid::Uuid::new_v4().to_string();
    let dir = store.home.join("ProjectHistoryData").join(&backup);
    fs::create_dir_all(&dir).map_err(err)?;
    for (index, path) in state_paths(store, root)?.into_iter().enumerate() {
        if path.exists() {
            atomic(
                &dir.join(format!("{index}.json")),
                &fs::read(path).map_err(err)?,
            )?;
        }
    }
    Ok(backup)
}
fn restore_state(store: &Storage, root: &Path, backup: &str) -> Result<()> {
    uuid::Uuid::parse_str(backup).map_err(err)?;
    let dir = store.home.join("ProjectHistoryData").join(backup);
    if !dir.is_dir() {
        return Err("Project recovery data is missing.".into());
    }
    for (index, path) in state_paths(store, root)?.into_iter().enumerate() {
        let saved = dir.join(format!("{index}.json"));
        if saved.exists() {
            atomic(&path, &fs::read(saved).map_err(err)?)?;
        } else if path.exists() {
            fs::remove_file(path).map_err(err)?;
        }
    }
    Ok(())
}
pub fn open_created(store: &Storage, root: &Path, main: Option<&str>) -> Result<Value> {
    let root = root.canonicalize().map_err(err)?;
    // A deleted project may have used this path. Keep its state for Undo while
    // ensuring the new folder doesn't inherit another project's cards/drafts.
    if state_paths(store, &root)?.iter().any(|path| path.exists()) {
        backup_state(store, &root)?;
        for path in state_paths(store, &root)? {
            if path.exists() {
                fs::remove_file(path).map_err(err)?;
            }
        }
    }
    // Seed a clean document so legacy lookup cannot resurrect an old project.
    let ident = store.register(&root)?;
    store.save(&ident,&json!({"root":root,"cards":[],"customOrder":[],"sections":[],"sectionAssignments":{},"exportFormat":"svg","exportFontSize":0}))?;
    store.open(&root, main)
}
fn identity(path: &Path) -> Result<String> {
    let metadata = path.symlink_metadata().map_err(err)?;
    if !metadata.is_dir() {
        return Err("The project folder was replaced or moved.".into());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!("{}:{}", metadata.dev(), metadata.ino()))
    }
    #[cfg(not(unix))]
    {
        Ok(format!("{:?}", metadata.created().map_err(err)?))
    }
}
impl History {
    pub fn load(store: &Storage) -> Result<Self> {
        let path = store.home.join("ProjectHistory.json");
        if !path.exists() {
            return Ok(Self::default());
        }
        let history: Self = serde_json::from_value(json_read(&path)?).map_err(err)?;
        if history.schema_version != 1 {
            return Err("Unsupported project history version; the file has been kept.".into());
        }
        Ok(history)
    }
    fn save(&self, store: &Storage) -> Result<()> {
        json_write(
            &store.home.join("ProjectHistory.json"),
            &serde_json::to_value(self).map_err(err)?,
        )
    }
    pub fn status(&self) -> Value {
        json!({"undo":self.undo.last().map(|a| &a.label),"redo":self.redo.last().map(|a| &a.label),
            "undoTrash":self.undo.last().filter(|a| !a.deletion).map(|a| a.items.iter().map(|i| &i.root).collect::<Vec<_>>()).unwrap_or_default(),
            "redoTrash":self.redo.last().filter(|a| a.deletion).map(|a| a.items.iter().map(|i| &i.root).collect::<Vec<_>>()).unwrap_or_default()})
    }
    pub fn affected(&self, redo: bool) -> Vec<PathBuf> {
        (if redo {
            self.redo.last()
        } else {
            self.undo.last()
        })
        .map(|a| a.items.iter().map(|i| i.root.clone()).collect())
        .unwrap_or_default()
    }
    pub fn record_created(
        &mut self,
        store: &Storage,
        roots: Vec<PathBuf>,
        label: &str,
    ) -> Result<()> {
        if roots.is_empty() {
            return Ok(());
        }
        self.undo.push(Action {
            label: label.into(),
            deletion: false,
            items: roots
                .into_iter()
                .map(|root| {
                    let root = root.canonicalize().map_err(err)?;
                    Ok(Item {
                        identity: identity(&root)?,
                        root,
                        trash: None,
                        backup: None,
                    })
                })
                .collect::<Result<_>>()?,
        });
        self.redo.clear();
        self.save(store)
    }
    pub fn delete(
        &mut self,
        store: &Storage,
        roots: Vec<PathBuf>,
        trash: impl Fn(&Path) -> Result<PathBuf>,
    ) -> Result<()> {
        if roots.is_empty() {
            return Err("Select a project.".into());
        }
        // Record the intended group first so partial failures remain undoable.
        self.undo.push(Action {
            label: "Delete Projects".into(),
            deletion: true,
            items: roots
                .into_iter()
                .map(|root| {
                    Ok(Item {
                        identity: identity(&root)?,
                        root,
                        trash: None,
                        backup: None,
                    })
                })
                .collect::<Result<_>>()?,
        });
        self.redo.clear();
        self.save(store)?;
        self.set_trashed(store, false, true, trash)
    }
    fn set_trashed(
        &mut self,
        store: &Storage,
        from_redo: bool,
        trashed: bool,
        trash: impl Fn(&Path) -> Result<PathBuf>,
    ) -> Result<()> {
        let count = (if from_redo { &self.redo } else { &self.undo })
            .last()
            .ok_or("Nothing to undo or redo.")?
            .items
            .len();
        for index in 0..count {
            let stack = if from_redo {
                &mut self.redo
            } else {
                &mut self.undo
            };
            let item = &mut stack.last_mut().unwrap().items[index];
            let current = item.trash.as_deref().unwrap_or(&item.root);
            if identity(current)? != item.identity {
                return Err(format!(
                    "{} was replaced. The replacement has been kept.",
                    current.display()
                ));
            }
            if trashed && item.trash.is_none() {
                item.backup = Some(backup_state(store, &item.root)?);
                item.trash = Some(trash(&item.root)?);
                item.identity = identity(item.trash.as_deref().unwrap())?;
            } else if !trashed {
                if let Some(path) = &item.trash {
                    if item.root.symlink_metadata().is_ok() {
                        return Err(format!(
                            "Cannot restore: {} already exists.",
                            item.root.display()
                        ));
                    }
                    if !path.is_dir() {
                        return Err("The project is no longer in Trash. It may have been restored or removed in Finder.".into());
                    }
                    fs::create_dir_all(item.root.parent().ok_or("Missing original folder")?)
                        .map_err(err)?;
                    rename_no_replace(path, &item.root)?;
                    if let Some(backup) = &item.backup {
                        if let Err(error) = restore_state(store, &item.root, backup) {
                            rename_no_replace(&item.root,path).map_err(|rollback|format!("{error}; folder restored but recovery could not finish: {rollback}"))?;
                            return Err(error);
                        }
                    }
                    item.trash = None;
                    item.identity = identity(&item.root)?;
                }
            }
            self.save(store)?;
        }
        Ok(())
    }
    pub fn travel(
        &mut self,
        store: &Storage,
        redo: bool,
        trash: impl Fn(&Path) -> Result<PathBuf>,
    ) -> Result<()> {
        let action = (if redo { &self.redo } else { &self.undo })
            .last()
            .ok_or("Nothing to undo or redo.")?;
        let trashed = if redo {
            action.deletion
        } else {
            !action.deletion
        };
        self.set_trashed(store, redo, trashed, trash)?;
        if redo {
            self.undo.push(self.redo.pop().unwrap());
        } else {
            self.redo.push(self.undo.pop().unwrap());
        }
        self.save(store)
    }
}

pub fn roots(store: &Storage, ids: &Value) -> Result<Vec<PathBuf>> {
    let ids = ids.as_array().ok_or("Select projects.")?;
    if ids.is_empty() || ids.len() > 1000 {
        return Err("Select between 1 and 1,000 projects.".into());
    }
    let mut roots = Vec::new();
    for id in ids {
        let root = store
            .root(id.as_str().ok_or("Invalid project ID")?)?
            .canonicalize()
            .map_err(err)?;
        if roots
            .iter()
            .any(|other: &PathBuf| root.starts_with(other) || other.starts_with(&root))
        {
            return Err(
                "Select either a containing project or its nested project, not both.".into(),
            );
        }
        roots.push(root);
    }
    Ok(roots)
}

pub fn duplicate(store: &Storage, source: &Path, parent: &Path) -> Result<PathBuf> {
    let state = CopyState::read(store, source)?;
    let name = format!(
        "{} copy",
        source
            .file_name()
            .ok_or("Invalid project folder")?
            .to_string_lossy()
    );
    let target = project_location::copy_folder(source, parent, &name)?;
    state.write(store, &target)?;
    Ok(target)
}

pub fn copy_state(store: &Storage, source: &Path, target: &Path) -> Result<()> {
    CopyState::read(store, source)?.write(store, target)
}
struct CopyState {
    original: Value,
    drafts: Value,
    instant: Option<Vec<u8>>,
}
impl CopyState {
    fn read(store: &Storage, source: &Path) -> Result<Self> {
        let original = store.open(source, None)?;
        let drafts = store.read_drafts(string(&original, "id"))?["buffers"].clone();
        let instant = store
            .project_dir(string(&original, "id"))?
            .join("Instant/session.json");
        let instant = if instant.exists() {
            Some(fs::read(instant).map_err(err)?)
        } else {
            None
        };
        Ok(Self {
            original,
            drafts,
            instant,
        })
    }
    fn write(self, store: &Storage, target: &Path) -> Result<()> {
        let Self {
            original,
            drafts,
            instant,
        } = self;
        let copy = open_created(store, target, original["main"].as_str())?;
        let mut metadata = original;
        metadata["id"] = copy["id"].clone();
        metadata["root"] = copy["root"].clone();
        // Library variants and labels are independent JSON; rendered artifacts may be
        // reused until the copy is edited. Original metadata/caches remain untouched.
        store.save(string(&copy, "id"), &metadata)?;
        json_write(&store.draft_path(string(&copy, "id"))?, &drafts)?;
        if let Some(bytes) = instant {
            atomic(
                &store
                    .project_dir(string(&copy, "id"))?
                    .join("Instant/session.json"),
                &bytes,
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reused_path_keeps_each_projects_variants_and_drafts_during_undo_redo() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Project");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("main.typ"), "$old$").unwrap();
        let mut original = store.open(&root, None).unwrap();
        original["cards"] = json!([{"id":"old-variant","tex":"old"}]);
        let ident = string(&original, "id").to_owned();
        store.save(&ident, &original).unwrap();
        let old_drafts = json!({"main.typ":{"text":"$unsaved$","revision":"old"}});
        json_write(&store.draft_path(&ident).unwrap(), &old_drafts).unwrap();
        let session = store
            .project_dir(&ident)
            .unwrap()
            .join("Instant/session.json");
        json_write(&session, &json!({"draft":{"source":"old"}})).unwrap();
        let trash = |p: &Path| {
            let dest = temp.path().join(uuid::Uuid::new_v4().to_string());
            rename_no_replace(p, &dest)?;
            Ok(dest)
        };
        let mut history = History::default();
        history.delete(&store, vec![root.clone()], trash).unwrap();
        fs::create_dir(&root).unwrap();
        fs::write(root.join("main.typ"), "$new$").unwrap();
        let mut new = open_created(&store, &root, None).unwrap();
        assert_eq!(new["cards"], json!([]));
        assert!(!session.exists());
        assert!(!store.draft_path(&ident).unwrap().exists());
        new["cards"] = json!([{"id":"new-variant","tex":"new"}]);
        store.save(&ident, &new).unwrap();
        history
            .record_created(&store, vec![root.clone()], "New Project")
            .unwrap();
        history.travel(&store, false, trash).unwrap();
        let mut history = History::load(&store).unwrap();
        history.travel(&store, false, trash).unwrap();
        assert_eq!(store.open(&root, None).unwrap()["cards"], original["cards"]);
        assert_eq!(store.read_drafts(&ident).unwrap()["buffers"], old_drafts);
        assert_eq!(json_read(&session).unwrap()["draft"]["source"], "old");
        history.travel(&store, true, trash).unwrap();
        history.travel(&store, true, trash).unwrap();
        assert_eq!(store.open(&root, None).unwrap()["cards"], new["cards"]);
        assert!(!session.exists());
        assert!(!store.draft_path(&ident).unwrap().exists());
    }
    #[test]
    fn group_delete_undo_redo_survives_restart_and_refuses_restore_collision() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let roots: Vec<_> = ["A", "B"]
            .iter()
            .map(|name| {
                let p = temp.path().join(name);
                fs::create_dir(&p).unwrap();
                fs::write(p.join("main.typ"), "$x$").unwrap();
                p
            })
            .collect();
        let trash_dir = temp.path().join("Trash");
        fs::create_dir(&trash_dir).unwrap();
        let trash = |p: &Path| {
            let dest = trash_dir.join(p.file_name().unwrap());
            rename_no_replace(p, &dest)?;
            Ok(dest)
        };
        let mut history = History::load(&store).unwrap();
        history.delete(&store, roots.clone(), trash).unwrap();
        assert!(roots.iter().all(|p| !p.exists()));
        let mut history = History::load(&store).unwrap();
        fs::create_dir(&roots[0]).unwrap();
        assert!(history
            .travel(&store, false, trash)
            .unwrap_err()
            .contains("already exists"));
        assert!(trash_dir.join("A/main.typ").exists());
        fs::remove_dir(&roots[0]).unwrap();
        history.travel(&store, false, trash).unwrap();
        assert!(roots.iter().all(|p| p.join("main.typ").exists()));
        history.travel(&store, true, trash).unwrap();
        assert!(roots.iter().all(|p| !p.exists()));
    }
    #[test]
    fn duplicate_keeps_variants_and_undo_moves_only_copy() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Source");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("main.typ"), "$x$").unwrap();
        let mut p = store.open(&root, None).unwrap();
        p["cards"] = json!([{"id":"variant","tex":"y","origin":"custom"}]);
        store.save(string(&p, "id"), &p).unwrap();
        let drafts = json!({"main.typ":{"text":"$y$","revision":"saved-revision"}});
        json_write(&store.draft_path(string(&p, "id")).unwrap(), &drafts).unwrap();
        atomic(
            &store
                .project_dir(string(&p, "id"))
                .unwrap()
                .join("Instant/session.json"),
            b"{\"draft\":{\"source\":\"z\"}}",
        )
        .unwrap();
        let copied = duplicate(&store, &root, temp.path()).unwrap();
        let p2 = store.open(&copied, None).unwrap();
        assert_eq!(p2["cards"], p["cards"]);
        assert_eq!(
            store.read_drafts(string(&p2, "id")).unwrap()["buffers"],
            drafts
        );
        assert!(store
            .project_dir(string(&p2, "id"))
            .unwrap()
            .join("Instant/session.json")
            .is_file());
        let mut history = History::default();
        history
            .record_created(&store, vec![copied.clone()], "Duplicate Projects")
            .unwrap();
        let trash = |p: &Path| {
            let dest = temp.path().join("Trashed copy");
            rename_no_replace(p, &dest)?;
            Ok(dest)
        };
        history.travel(&store, false, trash).unwrap();
        assert!(root.exists());
        assert!(!copied.exists());
        history.travel(&store, true, trash).unwrap();
        assert!(copied.exists());
    }

    #[test]
    fn undo_does_not_trash_a_replacement_and_partial_delete_can_be_undone() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("Data"), temp.path().join("Legacy")).unwrap();
        let root = temp.path().join("Created");
        fs::create_dir(&root).unwrap();
        let mut history = History::default();
        history
            .record_created(&store, vec![root.clone()], "New Project")
            .unwrap();
        fs::rename(&root, temp.path().join("Moved")).unwrap();
        fs::create_dir(&root).unwrap();
        assert!(history
            .travel(&store, false, |_| panic!("must not touch replacement"))
            .unwrap_err()
            .contains("replaced"));
        let second = temp.path().join("Second");
        fs::create_dir(&second).unwrap();
        let trash = |p: &Path| {
            if p == second {
                return Err("Test permission error".into());
            }
            let dest = temp.path().join("Trash");
            rename_no_replace(p, &dest)?;
            Ok(dest)
        };
        history
            .delete(&store, vec![root.clone(), second.clone()], trash)
            .unwrap_err();
        let mut reloaded = History::load(&store).unwrap();
        reloaded.travel(&store, false, trash).unwrap();
        assert!(root.is_dir() && second.is_dir());
    }
}
