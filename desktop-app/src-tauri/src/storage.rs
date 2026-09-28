use crate::{files::*, scan};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
pub struct Storage {
    pub home: PathBuf,
    pub legacy: PathBuf,
    projects: Mutex<HashMap<String, PathBuf>>,
    pub writes: Mutex<()>,
}
impl Storage {
    pub fn new(home: PathBuf, legacy: PathBuf) -> Result<Self> {
        fs::create_dir_all(home.join("Projects")).map_err(err)?;
        fs::create_dir_all(home.join("Settings")).map_err(err)?;
        Ok(Self {
            home,
            legacy,
            projects: Mutex::new(HashMap::new()),
            writes: Mutex::new(()),
        })
    }
    pub fn project_dir(&self, id: &str) -> Result<PathBuf> {
        if id.len() != 24 || !id.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err("Invalid project ID".into());
        }
        Ok(self.home.join("Projects").join(id))
    }
    pub fn register(&self, root: &Path) -> Result<String> {
        let root = root.canonicalize().map_err(err)?;
        if !root.is_dir() {
            return Err("Choose a project folder.".into());
        }
        let ident = id(&path_string(&root));
        self.projects.lock().unwrap().insert(ident.clone(), root);
        Ok(ident)
    }
    pub fn root(&self, id: &str) -> Result<PathBuf> {
        self.projects
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or("Open this project first.".into())
    }
    pub fn recent(&self) -> Result<Vec<Value>> {
        let mut found: HashMap<String, Value> = HashMap::new();
        for base in [&self.legacy, &self.home] {
            let dir = base.join("Projects");
            if !dir.is_dir() {
                continue;
            }
            for entry in fs::read_dir(dir).map_err(err)? {
                let entry = entry.map_err(err)?;
                if let Ok(mut p) = json_read(&entry.path().join("project.json")) {
                    // Keep the old sources/metadata recoverable after demo setup,
                    // while showing their editable copies at the selected location.
                    if let Some(target) = p["demoRelocatedTo"].as_str() {
                        if Path::new(target).is_dir()
                            && self
                                .home
                                .join("Projects")
                                .join(id(target))
                                .join("project.json")
                                .is_file()
                        {
                            let root = Path::new(string(&p, "root"));
                            if let Ok(root) = root.canonicalize() {
                                found.remove(&id(&path_string(&root)));
                            }
                            continue;
                        }
                    }
                    let root = Path::new(string(&p, "root"));
                    if root.is_dir() {
                        if let Ok(ident) = self.register(root) {
                            p["id"] = json!(ident);
                            found.insert(ident, p);
                        }
                    }
                }
            }
        }
        let mut items: Vec<_> = found.into_values().collect();
        items.sort_by_key(|p| string(p, "root").to_lowercase());
        Ok(items)
    }
    pub fn open(&self, root: &Path, main: Option<&str>) -> Result<Value> {
        let ident = self.register(root)?;
        let root = self.root(&ident)?;
        let dir = self.project_dir(&ident)?;
        let mut p = if dir.join("project.json").exists() {
            json_read(&dir.join("project.json"))?
        } else {
            // Swift may have hashed an alias such as /var rather than /private/var.
            // Match its saved root too, so canonicalizing does not lose variants.
            self.legacy_project(&ident, &root).unwrap_or_else(|| {
                json!({"root":root,"cards":[],"customOrder":[],"sections":[],"sectionAssignments":{},"exportFormat":"svg","exportFontSize":0})
            })
        };
        let candidates = scan::candidates(&root)?;
        let main = main
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .or_else(|| p["main"].as_str().map(str::to_owned))
            .or_else(|| candidates.first().cloned())
            .ok_or("No LaTeX or Typst entry file found.")?;
        safe(&root, &main)?;
        p["id"] = json!(ident);
        p["root"] = json!(root);
        p.as_object_mut().unwrap().remove("demoRelocatedTo");
        p["main"] = json!(main);
        p["format"] = json!(if main.ends_with(".typ") {
            "typst"
        } else {
            "latex"
        });
        p["candidates"] = json!(candidates);
        if string(&p, "engine").is_empty() {
            p["engine"] = json!(if root.join("latexmkrc").exists()
                || root.join(".latexmkrc").exists()
            {
                "project"
            } else {
                "pdflatex"
            })
        }
        self.save(&ident, &p)?;
        Ok(p)
    }
    fn legacy_project(&self, ident: &str, root: &Path) -> Option<Value> {
        let projects = self.legacy.join("Projects");
        if let Ok(p) = json_read(&projects.join(ident).join("project.json")) {
            return Some(p);
        }
        fs::read_dir(projects).ok()?.flatten().find_map(|entry| {
            let p = json_read(&entry.path().join("project.json")).ok()?;
            (Path::new(string(&p, "root")).canonicalize().ok()?.as_path() == root).then_some(p)
        })
    }
    pub fn load(&self, ident: &str) -> Result<Value> {
        self.root(ident)?;
        json_read(&self.project_dir(ident)?.join("project.json"))
    }
    pub fn save(&self, ident: &str, p: &Value) -> Result<()> {
        let mut p = p.clone();
        p["root"] = json!(self.root(ident)?);
        p["id"] = json!(ident);
        json_write(&self.project_dir(ident)?.join("project.json"), &p)
    }
    pub fn allowed_artifact(&self, p: &Path) -> Result<PathBuf> {
        let path = p.canonicalize().map_err(err)?;
        if [&self.home, &self.legacy]
            .iter()
            .any(|base| path.starts_with(base))
        {
            Ok(path)
        } else {
            Err("Artifact is outside application storage.".into())
        }
    }
    pub fn draft_path(&self, ident: &str) -> Result<PathBuf> {
        Ok(self
            .home
            .join("SourceDrafts")
            .join(format!("{}.json", hash(path_string(&self.root(ident)?)))))
    }
    pub fn read_drafts(&self, ident: &str) -> Result<Value> {
        use base64::{engine::general_purpose::STANDARD, Engine};
        let path = self.draft_path(ident)?;
        // A corrupt recovery file must never be replaced by an empty editor session.
        let mut drafts = if path.exists() {
            json_read(&path)?
        } else {
            json!({})
        };
        let buffers = drafts
            .as_object_mut()
            .ok_or("Invalid source draft file; it has been kept.")?;
        let legacy = self
            .legacy
            .join("SourceDrafts")
            .join(path.file_name().unwrap());
        let backup = self
            .home
            .join("MigratedSwiftDrafts")
            .join(path.file_name().unwrap());
        let marker = backup.with_extension("imported");
        let mut warnings = Vec::new();
        if legacy.is_file() && !marker.exists() {
            let original = fs::read(&legacy).map_err(err)?;
            let old: Value = serde_json::from_slice(&original).map_err(err)?;
            let old = old
                .as_array()
                .ok_or("Invalid Swift source drafts; originals have been kept.")?;
            for buffer in old {
                let name = string(buffer, "path");
                relative(name)?;
                let text = buffer["text"].as_str().ok_or("Invalid Swift draft text")?;
                let bytes = STANDARD.decode(string(buffer, "original")).map_err(err)?;
                let bom = buffer["hasBOM"]
                    .as_bool()
                    .ok_or("Invalid Swift draft encoding")?;
                let crlf = buffer["crlf"]
                    .as_bool()
                    .ok_or("Invalid Swift draft line endings")?;
                let converted = json!({"path":name,"text":text.replace("\r\n", "\n"),"revision":hash(bytes),"bom":bom,"newline":if crlf {"\r\n"} else {"\n"}});
                if let Some(current) = buffers.get(name) {
                    if current["text"] != converted["text"] {
                        warnings.push(format!("A different Swift draft for {name} is preserved in {}. The current draft is open.", backup.display()));
                    }
                } else {
                    buffers.insert(name.into(), converted);
                }
            }
            // Backup first, then the merged recovery data, and commit the marker last.
            // Originals stay untouched; repeat startup never resurrects saved drafts.
            atomic(&backup, &original)?;
            json_write(&path, &drafts)?;
            atomic(&marker, b"1\n")?;
        }
        Ok(json!({"buffers":drafts,"warnings":warnings}))
    }
}
fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
pub fn merge_scan(mut p: Value, scan: Value) -> Value {
    let old = p["cards"].as_array().cloned().unwrap_or_default();
    let mut cards = scan["cards"].as_array().cloned().unwrap_or_default();
    for card in &mut cards {
        if let Some(previous) = old.iter().find(|o| o["id"] == card["id"]) {
            for key in ["name", "exportFontSize"] {
                if previous.get(key).is_some() {
                    card[key] = previous[key].clone()
                }
            }
        }
    }
    cards.extend(old.into_iter().filter(|v| string(v, "origin") == "custom"));
    p["cards"] = json!(cards);
    p["outline"] = scan["outline"].clone();
    p["warnings"] = scan["warnings"].clone();
    p["files"] = scan["files"].clone();
    p
}
pub fn import_zip(archive: &Path, parent: &Path) -> Result<PathBuf> {
    use std::io::{Read, Write};
    let mut zip = zip::ZipArchive::new(fs::File::open(archive).map_err(err)?).map_err(err)?;
    if zip.len() > 10000 {
        return Err("Archive exceeds 10,000 entries.".into());
    }
    let mut total = 0u64;
    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(err)?;
        relative(file.name().trim_end_matches('/'))?;
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("ZIP symbolic links are not supported.".into());
        }
        total = total
            .checked_add(file.size())
            .ok_or("Archive size overflow")?;
        if total > 512 * 1024 * 1024 {
            return Err("Archive exceeds 512 MB.".into());
        }
    }
    fs::create_dir_all(parent).map_err(err)?;
    let temp = tempfile::tempdir_in(parent).map_err(err)?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(err)?;
        let out = temp
            .path()
            .join(relative(file.name().trim_end_matches('/'))?);
        if file.is_dir() {
            fs::create_dir_all(&out).map_err(err)?
        } else {
            fs::create_dir_all(out.parent().unwrap()).map_err(err)?;
            let mut dest = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&out)
                .map_err(err)?;
            let mut bytes = vec![];
            file.by_ref()
                .take(512 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(err)?;
            if bytes.len() > 512 * 1024 * 1024 {
                return Err("ZIP entry exceeds size limit.".into());
            }
            dest.write_all(&bytes).map_err(err)?;
        }
    }
    let children: Vec<_> = fs::read_dir(temp.path())
        .map_err(err)?
        .flatten()
        .filter(|e| e.file_name() != ".DS_Store" && e.file_name() != "__MACOSX")
        .collect();
    let root = if children.len() == 1 && children[0].path().is_dir() {
        children[0].path()
    } else {
        temp.path().to_owned()
    };
    if scan::candidates(&root)?.is_empty() {
        return Err("No LaTeX or Typst entry file found.".into());
    }
    crate::project_location::publish(
        &root,
        parent,
        &archive.file_stem().unwrap_or_default().to_string_lossy(),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    #[test]
    fn swift_drafts_migrate_once_without_writing_source() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("source");
        fs::create_dir_all(&root).unwrap();
        let bytes = "\u{feff}$ old $\r\n".as_bytes();
        fs::write(root.join("main.typ"), bytes).unwrap();
        let store = Storage::new(dir.path().join("new"), dir.path().join("old")).unwrap();
        let ident = store.register(&root).unwrap();
        let path = store.draft_path(&ident).unwrap();
        let legacy = store
            .legacy
            .join("SourceDrafts")
            .join(path.file_name().unwrap());
        let old = json!([{"path":"main.typ","text":"$ edited $\r\n","original":STANDARD.encode(bytes),"hasBOM":true,"crlf":true}]);
        json_write(&legacy, &old).unwrap();
        let migrated = store.read_drafts(&ident).unwrap();
        let buffer = &migrated["buffers"]["main.typ"];
        assert_eq!(buffer["text"], "$ edited $\n");
        assert_eq!(buffer["revision"], hash(bytes));
        assert_eq!(buffer["bom"], true);
        assert_eq!(buffer["newline"], "\r\n");
        assert_eq!(fs::read(root.join("main.typ")).unwrap(), bytes);
        assert_eq!(json_read(&legacy).unwrap(), old);
        fs::write(root.join("main.typ"), "$ external change $").unwrap();
        assert!(save_buffer(&root.canonicalize().unwrap(), buffer).is_err());
        json_write(&path, &json!({})).unwrap();
        assert_eq!(store.read_drafts(&ident).unwrap()["buffers"], json!({}));
    }
    #[test]
    fn draft_collisions_keep_both_versions_and_invalid_data_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let store = Storage::new(dir.path().join("new"), dir.path().join("old")).unwrap();
        let ident = store.register(dir.path()).unwrap();
        let path = store.draft_path(&ident).unwrap();
        let legacy = store
            .legacy
            .join("SourceDrafts")
            .join(path.file_name().unwrap());
        let old = json!([{"path":"main.typ","text":"Swift edit","original":STANDARD.encode("old"),"hasBOM":false,"crlf":false}]);
        json_write(&legacy, &old).unwrap();
        let current = json!({"main.typ":{"path":"main.typ","text":"Current edit","revision":hash("old"),"bom":false,"newline":"\n"}});
        json_write(&path, &current).unwrap();
        let result = store.read_drafts(&ident).unwrap();
        assert_eq!(result["buffers"], current);
        assert_eq!(result["warnings"].as_array().unwrap().len(), 1);
        assert_eq!(
            json_read(
                &store
                    .home
                    .join("MigratedSwiftDrafts")
                    .join(path.file_name().unwrap())
            )
            .unwrap(),
            old
        );
        fs::write(&path, "invalid json").unwrap();
        assert!(store.read_drafts(&ident).is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "invalid json");
        assert_eq!(json_read(&legacy).unwrap(), old);
    }
    #[test]
    fn scan_preserves_variants_and_labels() {
        let p = json!({"cards":[{"id":"a","origin":"source","name":"Named","exportFontSize":24},{"id":"custom-1","origin":"custom","tex":"x"}],"sectionAssignments":{"a":"section"},"extensions":{"future":1}});
        let scan = json!({"cards":[{"id":"a","origin":"source","tex":"y"}],"outline":[],"warnings":[],"files":[]});
        let next = merge_scan(p, scan);
        assert_eq!(next["cards"].as_array().unwrap().len(), 2);
        assert_eq!(next["cards"][0]["name"], "Named");
        assert_eq!(next["sectionAssignments"]["a"], "section");
        assert_eq!(next["extensions"]["future"], 1);
    }
    #[test]
    fn legacy_is_only_read() {
        let d = tempfile::tempdir().unwrap();
        let legacy = d.path().join("old");
        let preview = d.path().join("new");
        let project = d.path().join("source");
        fs::create_dir_all(&project).unwrap();
        let project = project.canonicalize().unwrap();
        fs::write(project.join("main.typ"), "$ x $").unwrap();
        let ident = id(&path_string(&project));
        let old = legacy.join("Projects").join(&ident).join("project.json");
        json_write(&old,&json!({"root":project,"main":"main.typ","cards":[],"engine":"project","customOrder":[]})).unwrap();
        let before = fs::read(&old).unwrap();
        let store = Storage::new(preview, legacy).unwrap();
        let mut p = store.open(&project, None).unwrap();
        p["exportFontSize"] = json!(24);
        store.save(&ident, &p).unwrap();
        assert_eq!(fs::read(old).unwrap(), before);
    }
    #[test]
    fn legacy_alias_keeps_variants() {
        let d = tempfile::tempdir().unwrap();
        let source = d.path().join("source");
        fs::create_dir_all(&source).unwrap();
        fs::write(source.join("main.typ"), "$ x $").unwrap();
        let alias = d.path().join("alias");
        std::os::unix::fs::symlink(&source, &alias).unwrap();
        let legacy = d.path().join("legacy");
        let file = legacy
            .join("Projects")
            .join(id(&path_string(&alias)))
            .join("project.json");
        json_write(
            &file,
            &json!({"root":alias,"main":"main.typ","cards":[{"origin":"custom","tex":"y"}]}),
        )
        .unwrap();
        let before = fs::read(&file).unwrap();
        let store = Storage::new(d.path().join("preview"), legacy).unwrap();
        let p = store.open(&source, None).unwrap();
        assert_eq!(p["cards"][0]["tex"], "y");
        assert_eq!(fs::read(file).unwrap(), before);
    }
}
