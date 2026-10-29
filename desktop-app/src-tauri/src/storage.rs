use crate::{files::*, project_data, scan};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
};
const OUTSIDE_STORAGE: &str = "Artifact is outside application storage. Render it again.";
const INSIDE_STORAGE: &str = "Choose a project folder outside application storage.";
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
        if !project_id(id) {
            return Err("Invalid project ID".into());
        }
        Ok(self.home.join("Projects").join(id))
    }
    pub fn register(&self, root: &Path) -> Result<String> {
        let root = root.canonicalize().map_err(err)?;
        if !root.is_dir() {
            return Err("Choose a project folder.".into());
        }
        if !self.project_root_allowed(&root) {
            return Err(INSIDE_STORAGE.into());
        }
        let ident = id(&path_string(&root));
        self.projects.lock().unwrap().insert(ident.clone(), root);
        Ok(ident)
    }
    /// Refuse a parent folder whose new projects `register` would reject,
    /// before anything is published there.
    pub fn check_project_parent(&self, parent: &Path) -> Result<()> {
        let parent = crate::project_location::resolve_destination(parent)?;
        if !self.project_root_allowed(&parent.join("project")) {
            return Err(INSIDE_STORAGE.into());
        }
        Ok(())
    }
    /// Application storage holds no project roots except the per-project
    /// folders Swift created and imported under legacy `Sources` and `Imports`.
    fn project_root_allowed(&self, root: &Path) -> bool {
        let Some((base, legacy)) = self.storage_base(root) else {
            return true;
        };
        let scope = root
            .strip_prefix(&base)
            .ok()
            .and_then(artifact_scope)
            .unwrap_or_default();
        legacy && matches!(scope.as_slice(), ["Sources" | "Imports", _, ..])
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
                if let Ok(mut p) = project_data::read(&entry.path().join("project.json")) {
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
        let mut p = if project_data::present(&dir.join("project.json"))
            || project_data::present(&project_data::backup(&dir.join("project.json")))
        {
            project_data::read(&dir.join("project.json"))?
        } else {
            // Swift may have hashed an alias such as /var rather than /private/var.
            // Match its saved root too, so canonicalizing does not lose variants.
            self.legacy_project(&ident, &root)?.unwrap_or_else(|| {
                json!({"root":root,"cards":[],"customOrder":[],"sections":[],"sectionAssignments":{},"exportFormat":"svg"})
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
        // Portable identity is additive; read-only projects remain usable. A
        // malformed identity is kept and reported if source-bearing SVG is exported.
        let _ = crate::roundtrip::project_uuid(&root, true);
        p["id"] = json!(ident);
        p["root"] = json!(root);
        p.as_object_mut()
            .ok_or("Invalid project metadata.")?
            .remove("demoRelocatedTo");
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
        project_data::normalize(p)
    }
    fn legacy_project(&self, ident: &str, root: &Path) -> Result<Option<Value>> {
        let projects = self.legacy.join("Projects");
        let exact = projects.join(ident).join("project.json");
        if project_data::present(&exact) {
            return project_data::read(&exact).map(Some);
        }
        let Ok(entries) = fs::read_dir(projects) else {
            return Ok(None);
        };
        Ok(entries.flatten().find_map(|entry| {
            let p = project_data::read(&entry.path().join("project.json")).ok()?;
            (Path::new(string(&p, "root")).canonicalize().ok()?.as_path() == root).then_some(p)
        }))
    }
    pub fn load(&self, ident: &str) -> Result<Value> {
        self.root(ident)?;
        project_data::read(&self.project_dir(ident)?.join("project.json"))
    }
    pub fn save(&self, ident: &str, p: &Value) -> Result<()> {
        if !p.is_object() {
            return Err("Project metadata must be an object.".into());
        }
        let mut p = p.clone();
        p["root"] = json!(self.root(ident)?);
        p["id"] = json!(ident);
        project_data::write(&self.project_dir(ident)?.join("project.json"), &p)
    }
    pub fn project_problems(&self) -> Result<Vec<Value>> {
        let mut problems = Vec::new();
        for base in [&self.home, &self.legacy] {
            let folder = base.join("Projects");
            if !folder.is_dir() {
                continue;
            }
            for entry in fs::read_dir(folder).map_err(err)?.flatten() {
                let path = entry.path().join("project.json");
                if !project_data::present(&path)
                    && !project_data::present(&project_data::backup(&path))
                {
                    continue;
                }
                if let Err(message) = project_data::read(&path) {
                    let ident = entry.file_name().to_string_lossy().into_owned();
                    let original = project_data::bytes(&path).ok();
                    let missing = path
                        .symlink_metadata()
                        .is_err_and(|e| e.kind() == std::io::ErrorKind::NotFound);
                    let revision = if missing {
                        Some("missing".to_string())
                    } else {
                        original.as_ref().map(hash)
                    };
                    let can_restore = base == &self.home
                        && self.project_dir(&ident).is_ok()
                        && (original.is_some() || missing)
                        && project_data::read(&project_data::backup(&path)).is_ok();
                    problems.push(json!({"id":ident,"path":path,"message":message,"canRestore":can_restore,"revision":revision}));
                }
            }
        }
        Ok(problems)
    }
    pub fn recover_project(&self, ident: &str, expected: &str) -> Result<()> {
        project_data::recover(&self.project_dir(ident)?.join("project.json"), expected)
    }
    /// Canonical rendered file under a managed Cache, Exports or PDF output folder.
    pub fn artifact_file(&self, path: &Path) -> Result<PathBuf> {
        let path = self.artifact(path, |scope, legacy| match scope {
            // Swift kept rendered and exported cards here; legacy has nothing else.
            ["Projects", project, "Cache" | "Exports", _, ..] => project_id(project),
            _ if legacy => false,
            ["Projects", project, "Instant", "Cache" | "Exports", _, ..]
            | ["Projects", project, "Documents", _, "Output", _, ..] => project_id(project),
            ["Instant", "Cache" | "Exports", _, ..] => true,
            _ => false,
        })?;
        path.is_file().then_some(path).ok_or(OUTSIDE_STORAGE.into())
    }
    /// Canonical source snapshot of a compiled PDF document.
    pub fn artifact_dir(&self, path: &Path) -> Result<PathBuf> {
        let path = self.artifact(path, |scope, legacy| {
            !legacy
                && matches!(scope, ["Projects", project, "Documents", _, "Source" | "Original"] if project_id(project))
        })?;
        path.is_dir().then_some(path).ok_or(OUTSIDE_STORAGE.into())
    }
    fn artifact(&self, path: &Path, allowed: fn(&[&str], bool) -> bool) -> Result<PathBuf> {
        let path = path.canonicalize().map_err(err)?;
        let (base, legacy) = self.storage_base(&path).ok_or(OUTSIDE_STORAGE)?;
        let scope = path
            .strip_prefix(&base)
            .ok()
            .and_then(artifact_scope)
            .ok_or(OUTSIDE_STORAGE)?;
        if !allowed(&scope, legacy) {
            return Err(OUTSIDE_STORAGE.into());
        }
        Ok(path)
    }
    /// The canonical home or legacy folder holding `path`, and whether it is
    /// legacy. Either may be nested in the other, so the longest one wins;
    /// home wins a tie.
    fn storage_base(&self, path: &Path) -> Option<(PathBuf, bool)> {
        [(&self.legacy, true), (&self.home, false)]
            .into_iter()
            .filter_map(|(base, legacy)| Some((base.canonicalize().ok()?, legacy)))
            .filter(|(base, _)| path.starts_with(base))
            .max_by_key(|(base, _)| base.components().count())
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
fn project_id(name: &str) -> bool {
    name.len() == 24 && name.bytes().all(|c| c.is_ascii_hexdigit())
}
/// Names below a storage base; `None` for anything but valid UTF-8 names.
fn artifact_scope(relative: &Path) -> Option<Vec<&str>> {
    relative
        .components()
        .map(|c| match c {
            Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect()
}
fn path_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
pub fn merge_scan(mut p: Value, scan: Value) -> Value {
    let mut old = p["cards"].as_array().cloned().unwrap_or_default();
    old.extend(
        p["extractionHiddenCards"]
            .as_array()
            .cloned()
            .unwrap_or_default(),
    );
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
    let hidden = scan["excludedCards"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|card| {
            old.iter()
                .find(|previous| previous["id"] == card["id"])
                .cloned()
                .unwrap_or(card)
        })
        .collect::<Vec<_>>();
    cards.extend(old.into_iter().filter(|v| string(v, "origin") == "custom"));
    p["cards"] = json!(cards);
    p["extractionHiddenCards"] = json!(hidden);
    p["outline"] = scan["outline"].clone();
    p["warnings"] = scan["warnings"].clone();
    p["files"] = scan["files"].clone();
    p
}
const ZIP_ENTRY_LIMIT: u64 = 10_000;
const ZIP_DIRECTORY_LIMIT: u64 = 64 * 1024 * 1024;
const ZIP_INPUT_LIMIT: u64 = 1024 * 1024 * 1024;
const ZIP_OUTPUT_LIMIT: u64 = 512 * 1024 * 1024;

#[derive(Clone, Copy)]
struct ZipLimits {
    entries: u64,
    directory: u64,
    input: u64,
    output: u64,
}
const ZIP_LIMITS: ZipLimits = ZipLimits {
    entries: ZIP_ENTRY_LIMIT,
    directory: ZIP_DIRECTORY_LIMIT,
    input: ZIP_INPUT_LIMIT,
    output: ZIP_OUTPUT_LIMIT,
};

fn zip_u16(bytes: &[u8], offset: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(
        bytes
            .get(offset..offset + 2)
            .ok_or("Invalid ZIP directory metadata.")?
            .try_into()
            .map_err(err)?,
    ))
}
fn zip_u32(bytes: &[u8], offset: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        bytes
            .get(offset..offset + 4)
            .ok_or("Invalid ZIP directory metadata.")?
            .try_into()
            .map_err(err)?,
    ))
}
fn zip_u64(bytes: &[u8], offset: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        bytes
            .get(offset..offset + 8)
            .ok_or("Invalid ZIP directory metadata.")?
            .try_into()
            .map_err(err)?,
    ))
}
fn validate_zip_directory(
    file: &mut fs::File,
    entries: u64,
    directory_size: u64,
    directory_end: u64,
) -> Result<()> {
    use std::io::{Read, Seek, SeekFrom};
    let directory_start = directory_end
        .checked_sub(directory_size)
        .ok_or("ZIP directory metadata is out of bounds.")?;
    file.seek(SeekFrom::Start(directory_start)).map_err(err)?;
    let mut cursor = directory_start;
    for _ in 0..entries {
        let header_end = cursor
            .checked_add(46)
            .ok_or("ZIP directory offset overflow.")?;
        if header_end > directory_end {
            return Err("ZIP directory record exceeds its declared size.".into());
        }
        let mut header = [0; 46];
        file.read_exact(&mut header).map_err(err)?;
        if &header[..4] != b"PK\x01\x02" {
            return Err("ZIP directory record is invalid or ambiguous.".into());
        }
        let name_size = u64::from(zip_u16(&header, 28)?);
        let extra_size = u64::from(zip_u16(&header, 30)?);
        let comment_size = u64::from(zip_u16(&header, 32)?);
        let variable_size = name_size
            .checked_add(extra_size)
            .and_then(|size| size.checked_add(comment_size))
            .ok_or("ZIP directory size overflow.")?;
        cursor = header_end
            .checked_add(variable_size)
            .ok_or("ZIP directory offset overflow.")?;
        if cursor > directory_end {
            return Err("ZIP directory record exceeds its declared size.".into());
        }
        file.seek(SeekFrom::Start(cursor)).map_err(err)?;
    }
    if cursor != directory_end {
        return Err("ZIP directory size does not match its records.".into());
    }
    Ok(())
}
fn preflight_zip(file: &mut fs::File, limits: ZipLimits) -> Result<u64> {
    use std::io::{Read, Seek, SeekFrom};
    let length = file.metadata().map_err(err)?.len();
    if length > limits.input {
        return Err("ZIP archive exceeds 1 GB.".into());
    }
    // zip 9 searches from file_len - (18-byte EOCD body + 4-byte signature
    // + 65,536-byte comment range). Cover that entire selection window.
    let tail_length = length.min(65_558) as usize;
    let mut tail = vec![0; tail_length];
    file.seek(SeekFrom::End(-(tail_length as i64)))
        .map_err(err)?;
    file.read_exact(&mut tail).map_err(err)?;
    let tail_start = length - tail_length as u64;
    // zip may reject a structurally bounded newest directory and backtrack to
    // any earlier EOCD signature in this window. Require a single signature,
    // not merely a single record whose comment happens to end at EOF.
    let mut signatures = tail
        .windows(4)
        .enumerate()
        .filter_map(|(index, signature)| (signature == b"PK\x05\x06").then_some(index));
    let eocd_index = signatures
        .next()
        .filter(|_| signatures.next().is_none())
        .ok_or("ZIP archive has no unambiguous end record.")?;
    let eocd_end = eocd_index
        .checked_add(22)
        .filter(|end| *end <= tail.len())
        .ok_or("ZIP archive has no unambiguous end record.")?;
    let eocd = &tail[eocd_index..eocd_end];
    let comment = usize::from(zip_u16(eocd, 20)?);
    if eocd_end.checked_add(comment) != Some(tail.len()) {
        return Err("ZIP archive has no unambiguous end record.".into());
    }
    let eocd_offset = tail_start + eocd_index as u64;
    let disk = zip_u16(eocd, 4)?;
    let directory_disk = zip_u16(eocd, 6)?;
    let disk_entries = zip_u16(eocd, 8)?;
    let entries = zip_u16(eocd, 10)?;
    let directory_size = zip_u32(eocd, 12)?;
    let directory_offset = zip_u32(eocd, 16)?;
    let zip64 = disk_entries == u16::MAX
        || entries == u16::MAX
        || directory_size == u32::MAX
        || directory_offset == u32::MAX;
    let (entries, directory_size, directory_offset, directory_end, archive_offset) = if zip64 {
        if disk != 0 || directory_disk != 0 || eocd_offset < 20 {
            return Err("ZIP64 archive has no valid locator.".into());
        }
        let locator_offset = eocd_offset - 20;
        file.seek(SeekFrom::Start(locator_offset)).map_err(err)?;
        let mut locator = [0; 20];
        file.read_exact(&mut locator).map_err(err)?;
        if &locator[..4] != b"PK\x06\x07"
            || zip_u32(&locator, 4)? != 0
            || zip_u32(&locator, 16)? != 1
        {
            return Err("ZIP64 multi-disk or ambiguous metadata is not supported.".into());
        }
        let declared_record_offset = zip_u64(&locator, 8)?;
        // ZIP64 locator offsets, like central-directory offsets, are relative to
        // the start of an archive. Find the record that ends at the locator so
        // prefixed archives retain one exact, bounded archive offset.
        let search_start = locator_offset.saturating_sub(limits.directory.saturating_add(56));
        let search_length = usize::try_from(locator_offset - search_start).map_err(err)?;
        let mut search = vec![0; search_length];
        file.seek(SeekFrom::Start(search_start)).map_err(err)?;
        file.read_exact(&mut search).map_err(err)?;
        let mut records = search
            .windows(4)
            .enumerate()
            .rev()
            .filter_map(|(index, signature)| {
                if signature != b"PK\x06\x06" || index + 12 > search.len() {
                    return None;
                }
                let size = u64::from_le_bytes(search[index + 4..index + 12].try_into().ok()?);
                let offset = search_start + index as u64;
                (size >= 44 && offset.checked_add(12)?.checked_add(size)? == locator_offset)
                    .then_some((index, offset, size))
            });
        let (record_index, record_offset, record_size) = records
            .next()
            .filter(|_| records.next().is_none())
            .ok_or("ZIP64 end record is truncated or ambiguous.")?;
        let record_end = record_index
            .checked_add(usize::try_from(record_size + 12).map_err(err)?)
            .ok_or("ZIP64 directory offset overflow.")?;
        let record = &search[record_index..record_end];
        let record_size = zip_u64(record, 4)?;
        let archive_offset = record_offset
            .checked_sub(declared_record_offset)
            .ok_or("ZIP64 directory metadata is out of bounds.")?;
        if &record[..4] != b"PK\x06\x06"
            || record_size < 44
            || zip_u32(record, 16)? != 0
            || zip_u32(record, 20)? != 0
            || zip_u64(record, 24)? != zip_u64(record, 32)?
        {
            return Err("ZIP64 multi-disk or ambiguous metadata is not supported.".into());
        }
        (
            zip_u64(record, 32)?,
            zip_u64(record, 40)?,
            zip_u64(record, 48)?,
            record_offset,
            archive_offset,
        )
    } else {
        let has_zip64_locator =
            eocd_index >= 20 && &tail[eocd_index - 20..eocd_index - 16] == b"PK\x06\x07";
        if disk != 0 || directory_disk != 0 || disk_entries != entries || has_zip64_locator {
            return Err("Multi-disk or ambiguous ZIP metadata is not supported.".into());
        }
        (
            u64::from(entries),
            u64::from(directory_size),
            u64::from(directory_offset),
            eocd_offset,
            eocd_offset
                .checked_sub(u64::from(directory_size))
                .and_then(|start| start.checked_sub(u64::from(directory_offset)))
                .ok_or("ZIP directory metadata is out of bounds.")?,
        )
    };
    if entries > limits.entries {
        return Err("Archive exceeds 10,000 entries.".into());
    }
    if directory_size > limits.directory {
        return Err("ZIP directory exceeds 64 MB.".into());
    }
    if archive_offset
        .checked_add(directory_offset)
        .and_then(|start| start.checked_add(directory_size))
        != Some(directory_end)
        || directory_offset.checked_add(directory_size).is_none()
    {
        return Err("ZIP directory metadata is out of bounds.".into());
    }
    validate_zip_directory(file, entries, directory_size, directory_end)?;
    file.rewind().map_err(err)?;
    Ok(archive_offset)
}

fn copy_zip_output(
    input: &mut impl std::io::Read,
    output: &mut impl std::io::Write,
    total: &mut u64,
    limit: u64,
) -> Result<()> {
    let mut buffer = [0; 64 * 1024];
    loop {
        let read = input.read(&mut buffer).map_err(err)?;
        if read == 0 {
            return Ok(());
        }
        *total = total
            .checked_add(read as u64)
            .ok_or("Archive size overflow")?;
        if *total > limit {
            return Err("Archive exceeds decompressed size limit.".into());
        }
        output.write_all(&buffer[..read]).map_err(err)?;
    }
}

fn snapshot_zip_input(
    input: &mut impl std::io::Read,
    declared_length: u64,
    limit: u64,
) -> Result<fs::File> {
    use std::io::{Seek, Write};
    if declared_length > limit {
        return Err("ZIP archive exceeds 1 GB.".into());
    }
    let mut snapshot = tempfile::tempfile().map_err(err)?;
    let mut limited = std::io::Read::take(input, limit.saturating_add(1));
    let copied = std::io::copy(&mut limited, &mut snapshot).map_err(err)?;
    if copied > limit {
        return Err("ZIP archive exceeds 1 GB.".into());
    }
    snapshot.flush().map_err(err)?;
    snapshot.rewind().map_err(err)?;
    Ok(snapshot)
}

fn snapshot_zip(archive: &Path, limit: u64) -> Result<fs::File> {
    if !fs::metadata(archive).map_err(err)?.file_type().is_file() {
        return Err("Choose a regular ZIP archive file.".into());
    }
    // A path may be replaced with a FIFO between metadata and open. On Unix,
    // nonblocking open lets the descriptor check below reject it without hanging.
    #[cfg(unix)]
    let mut source = {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(archive)
            .map_err(err)?
    };
    #[cfg(not(unix))]
    let mut source = fs::File::open(archive).map_err(err)?;
    let metadata = source.metadata().map_err(err)?;
    if !metadata.file_type().is_file() {
        return Err("Choose a regular ZIP archive file.".into());
    }
    let declared_length = metadata.len();
    snapshot_zip_input(&mut source, declared_length, limit)
}

fn import_zip_with_limits(archive: &Path, parent: &Path, limits: ZipLimits) -> Result<PathBuf> {
    // Preflight and zip must parse the same private bytes. The selected source
    // file may otherwise be rewritten between validation and indexing.
    let mut archive_file = snapshot_zip(archive, limits.input)?;
    let archive_offset = preflight_zip(&mut archive_file, limits)?;
    let config = zip::read::Config {
        archive_offset: zip::read::ArchiveOffset::Known(archive_offset),
    };
    let mut zip = zip::ZipArchive::with_config(config, archive_file).map_err(err)?;
    let mut declared_total = 0u64;
    for i in 0..zip.len() {
        let file = zip.by_index(i).map_err(err)?;
        relative(file.name().map_err(err)?.trim_end_matches('/'))?;
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            return Err("ZIP symbolic links are not supported.".into());
        }
        declared_total = declared_total
            .checked_add(file.size())
            .ok_or("Archive size overflow")?;
        if declared_total > limits.output {
            return Err("Archive exceeds 512 MB.".into());
        }
    }
    fs::create_dir_all(parent).map_err(err)?;
    let temp = tempfile::tempdir_in(parent).map_err(err)?;
    let mut actual_total = 0u64;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).map_err(err)?;
        let out = temp
            .path()
            .join(relative(file.name().map_err(err)?.trim_end_matches('/'))?);
        if file.is_dir() {
            fs::create_dir_all(&out).map_err(err)?
        } else {
            fs::create_dir_all(out.parent().unwrap()).map_err(err)?;
            let mut dest = fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&out)
                .map_err(err)?;
            copy_zip_output(&mut file, &mut dest, &mut actual_total, limits.output)?;
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
pub fn import_zip(archive: &Path, parent: &Path) -> Result<PathBuf> {
    import_zip_with_limits(archive, parent, ZIP_LIMITS)
}
#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    fn test_zip(path: &Path, names: &[&str]) {
        let mut writer = zip::ZipWriter::new(fs::File::create(path).unwrap());
        for name in names {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(b"$x$").unwrap();
        }
        writer.finish().unwrap();
    }

    fn duplicate_central_names(path: &Path) {
        let mut bytes = fs::read(path).unwrap();
        let mut offset = 0;
        while let Some(relative) = bytes[offset..]
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
        {
            let header = offset + relative;
            let name_length = u16::from_le_bytes([bytes[header + 28], bytes[header + 29]]) as usize;
            assert_eq!(name_length, 5);
            bytes[header + 46..header + 51].copy_from_slice(b"x.typ");
            offset = header + 51;
        }
        fs::write(path, bytes).unwrap();
    }

    fn bounded_zip(path: &Path) -> zip::ZipArchive<fs::File> {
        let mut file = fs::File::open(path).unwrap();
        let archive_offset = preflight_zip(&mut file, ZIP_LIMITS).unwrap();
        zip::ZipArchive::with_config(
            zip::read::Config {
                archive_offset: zip::read::ArchiveOffset::Known(archive_offset),
            },
            file,
        )
        .unwrap()
    }

    #[test]
    fn zip_preflight_bounds_raw_entries_before_parser_construction() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("duplicate.zip");
        test_zip(&archive, &["a.typ", "b.typ", "c.typ"]);
        duplicate_central_names(&archive);
        let length = archive.metadata().unwrap().len();
        let mut limits = ZIP_LIMITS;
        limits.entries = 2;
        assert!(
            preflight_zip(&mut fs::File::open(&archive).unwrap(), limits)
                .unwrap_err()
                .contains("10,000 entries")
        );
        limits.entries = 3;
        preflight_zip(&mut fs::File::open(&archive).unwrap(), limits).unwrap();
        assert_eq!(
            zip::ZipArchive::new(fs::File::open(&archive).unwrap())
                .unwrap()
                .len(),
            1
        );
        limits.input = length - 1;
        assert!(
            preflight_zip(&mut fs::File::open(&archive).unwrap(), limits)
                .unwrap_err()
                .contains("1 GB")
        );
    }

    #[test]
    fn zip_preflight_accepts_bounded_zip64_directory_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("zip64.zip");
        let mut bytes = b"PK\x01\x02".to_vec();
        bytes.extend_from_slice(&[0; 42]);
        bytes.extend_from_slice(b"PK\x06\x06");
        bytes.extend_from_slice(&44u64.to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u64.to_le_bytes());
        bytes.extend_from_slice(&1u64.to_le_bytes());
        bytes.extend_from_slice(&46u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(b"PK\x06\x07");
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&46u64.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(b"PK\x05\x06");
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&u16::MAX.to_le_bytes());
        bytes.extend_from_slice(&u16::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        fs::write(&archive, &bytes).unwrap();
        preflight_zip(&mut fs::File::open(&archive).unwrap(), ZIP_LIMITS).unwrap();
        bytes[86..94].copy_from_slice(&45u64.to_le_bytes());
        fs::write(&archive, bytes).unwrap();
        assert!(preflight_zip(&mut fs::File::open(archive).unwrap(), ZIP_LIMITS).is_err());
    }

    #[test]
    fn zip_preflight_and_parser_accept_the_same_prefixed_zip32_region() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("prefixed.zip");
        test_zip(&archive, &["main.typ"]);
        let original = fs::read(&archive).unwrap();
        let mut prefix = b"PK\x01\x02".to_vec();
        prefix.extend_from_slice(&[0; 42]);
        let expected_directory = prefix.len() as u64
            + original
                .windows(4)
                .position(|window| window == b"PK\x01\x02")
                .unwrap() as u64;
        prefix.extend_from_slice(&original);
        fs::write(&archive, prefix).unwrap();

        let mut file = fs::File::open(&archive).unwrap();
        assert_eq!(preflight_zip(&mut file, ZIP_LIMITS).unwrap(), 46);
        let mut zip = bounded_zip(&archive);
        assert_eq!(zip.central_directory_start(), expected_directory);
        assert_eq!(zip.len(), 1);
        assert_eq!(zip.by_index(0).unwrap().name().unwrap(), "main.typ");
    }

    #[test]
    fn zip_preflight_accepts_prefixed_zip64_locator_offsets() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("prefixed-zip64.zip");
        let prefix = b"self-extracting-prefix";
        let mut bytes = prefix.to_vec();
        bytes.extend_from_slice(b"PK\x01\x02");
        bytes.extend_from_slice(&[0; 42]);
        bytes.extend_from_slice(b"PK\x06\x06");
        bytes.extend_from_slice(&44u64.to_le_bytes());
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&1u64.to_le_bytes());
        bytes.extend_from_slice(&1u64.to_le_bytes());
        bytes.extend_from_slice(&46u64.to_le_bytes());
        bytes.extend_from_slice(&0u64.to_le_bytes());
        bytes.extend_from_slice(b"PK\x06\x07");
        bytes.extend_from_slice(&0u32.to_le_bytes());
        bytes.extend_from_slice(&46u64.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(b"PK\x05\x06");
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&u16::MAX.to_le_bytes());
        bytes.extend_from_slice(&u16::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&u32::MAX.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        fs::write(&archive, bytes).unwrap();

        assert_eq!(
            preflight_zip(&mut fs::File::open(archive).unwrap(), ZIP_LIMITS).unwrap(),
            prefix.len() as u64
        );
    }

    #[test]
    fn zip_preflight_rejects_understated_standard_directory_size() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("understated.zip");
        test_zip(&archive, &["main.typ"]);
        let mut bytes = fs::read(&archive).unwrap();
        let eocd = bytes.len() - 22;
        let size = u32::from_le_bytes(bytes[eocd + 12..eocd + 16].try_into().unwrap());
        bytes[eocd + 12..eocd + 16].copy_from_slice(&(size - 1).to_le_bytes());
        fs::write(&archive, bytes).unwrap();
        assert!(
            preflight_zip(&mut fs::File::open(archive).unwrap(), ZIP_LIMITS)
                .unwrap_err()
                .contains("directory record")
        );
    }

    #[test]
    fn zip_preflight_rejects_unconsumed_declared_directory_bytes() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("overstated.zip");
        test_zip(&archive, &["main.typ"]);
        let mut bytes = fs::read(&archive).unwrap();
        let eocd = bytes.len() - 22;
        let size = u32::from_le_bytes(bytes[eocd + 12..eocd + 16].try_into().unwrap());
        let offset = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap());
        bytes[eocd + 12..eocd + 16].copy_from_slice(&(size + 1).to_le_bytes());
        bytes[eocd + 16..eocd + 20].copy_from_slice(&(offset - 1).to_le_bytes());
        fs::write(&archive, bytes).unwrap();
        assert!(
            preflight_zip(&mut fs::File::open(archive).unwrap(), ZIP_LIMITS)
                .unwrap_err()
                .contains("invalid or ambiguous")
        );
    }

    #[test]
    fn zip_preflight_rejects_ambiguous_end_records_before_parser_selection() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("ambiguous.zip");
        test_zip(&archive, &["main.typ"]);
        let mut bytes = fs::read(&archive).unwrap();
        let eocd = bytes.len() - 22;
        bytes[eocd + 20..eocd + 22].copy_from_slice(&22u16.to_le_bytes());
        bytes.extend_from_slice(b"PK\x05\x06");
        bytes.extend_from_slice(&[0; 18]);
        fs::write(&archive, bytes).unwrap();
        assert!(
            preflight_zip(&mut fs::File::open(archive).unwrap(), ZIP_LIMITS)
                .unwrap_err()
                .contains("unambiguous")
        );
    }

    #[test]
    fn zip_preflight_conservatively_rejects_eocd_signature_in_comment() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("signature-comment.zip");
        test_zip(&archive, &["main.typ"]);
        let mut bytes = fs::read(&archive).unwrap();
        let eocd = bytes.len() - 22;
        bytes[eocd + 20..eocd + 22].copy_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(b"PK\x05\x06");
        fs::write(&archive, bytes).unwrap();

        assert!(
            preflight_zip(&mut fs::File::open(archive).unwrap(), ZIP_LIMITS)
                .unwrap_err()
                .contains("unambiguous")
        );
    }

    #[test]
    fn zip_preflight_rejects_backtracking_to_earlier_over_limit_directory() {
        let temp = tempfile::tempdir().unwrap();
        let archive = temp.path().join("backtracking.zip");
        test_zip(&archive, &["first.typ", "second.typ"]);
        let mut bytes = fs::read(&archive).unwrap();

        // The newest EOCD declares one raw, bounded CDFH. Its one-byte extra
        // field is invalid to zip's richer parser, which would otherwise retry
        // the earlier two-entry EOCD and bypass the pre-index entry cap.
        let directory_offset = u32::try_from(bytes.len()).unwrap();
        let mut header = b"PK\x01\x02".to_vec();
        header.extend_from_slice(&[0; 42]);
        header[30..32].copy_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&header);
        bytes.push(0);
        bytes.extend_from_slice(b"PK\x05\x06");
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&47u32.to_le_bytes());
        bytes.extend_from_slice(&directory_offset.to_le_bytes());
        bytes.extend_from_slice(&0u16.to_le_bytes());
        fs::write(&archive, bytes).unwrap();

        let mut limits = ZIP_LIMITS;
        limits.entries = 1;
        assert!(preflight_zip(&mut fs::File::open(archive).unwrap(), limits)
            .unwrap_err()
            .contains("unambiguous"));
    }

    #[test]
    fn streamed_zip_output_enforces_actual_aggregate_bytes() {
        let mut output = Vec::new();
        let mut total = 2;
        let error =
            copy_zip_output(&mut Cursor::new(b"1234"), &mut output, &mut total, 5).unwrap_err();
        assert!(error.contains("decompressed size"));
        assert!(output.is_empty());
    }

    #[test]
    fn zip_snapshot_bounds_declared_and_actual_input_bytes() {
        let mut grown = Cursor::new(b"123456");
        assert!(snapshot_zip_input(&mut grown, 5, 5)
            .unwrap_err()
            .contains("1 GB"));

        let mut oversized_metadata = Cursor::new(b"12345");
        assert!(snapshot_zip_input(&mut oversized_metadata, 6, 5)
            .unwrap_err()
            .contains("1 GB"));
        assert_eq!(oversized_metadata.position(), 0);
    }

    #[test]
    fn zip_snapshot_rejects_nonregular_input_before_copying() {
        let temp = tempfile::tempdir().unwrap();
        assert!(snapshot_zip(temp.path(), ZIP_INPUT_LIMIT)
            .unwrap_err()
            .contains("regular ZIP archive"));
    }

    #[test]
    fn artifact_access_compares_canonical_storage_roots() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("support"), temp.path().join("legacy")).unwrap();
        let image = store.home.join("Instant/Exports/sample.svg");
        atomic(&image, b"<svg/>").unwrap();
        assert_eq!(
            store.artifact_file(&image).unwrap(),
            image.canonicalize().unwrap()
        );
        let outside = temp.path().join("outside.svg");
        atomic(&outside, b"<svg/>").unwrap();
        assert!(store.artifact_file(&outside).is_err());
    }
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    const PROJECT: &str = "0123456789abcdef01234567";
    fn managed_store(home: &str, legacy: &str) -> (tempfile::TempDir, Storage) {
        let temp = tempfile::tempdir().unwrap();
        // macOS temp paths go through the /var symlink; bases stay as given.
        let base = temp.path().canonicalize().unwrap();
        let store = Storage::new(base.join(home), base.join(legacy)).unwrap();
        (temp, store)
    }
    fn file(path: &Path) -> PathBuf {
        atomic(path, b"x").unwrap();
        path.to_owned()
    }
    #[test]
    fn artifact_file_rejects_settings_metadata_drafts_and_templates() {
        let (temp, store) = managed_store("data", "legacy");
        let project = store.project_dir(PROJECT).unwrap();
        for path in [
            store.home.join("Settings/preferences.json"),
            project.join("project.json"),
            project.join("SourceMutation.json"),
            project.join("Documents/document-1/Source/main.tex"),
            project.join("Instant/session.json"),
            store.home.join("SourceDrafts/x.json"),
            store.home.join("templates.json"),
            store.home.join("Cache/x.svg"),
            store.home.join("Instant/session.json"),
            store.home.join("Projects/not-a-project-id/Cache/x.svg"),
            store.legacy.join("Settings/preferences.json"),
            store.legacy.join("Instant/Cache/a.svg"),
            store
                .legacy
                .join("Projects")
                .join(PROJECT)
                .join("Instant/Cache/a.svg"),
            store
                .legacy
                .join("Projects")
                .join(PROJECT)
                .join("Documents/document-1/Output/main.pdf"),
            temp.path().join("outside.svg"),
        ] {
            let path = file(&path);
            assert!(store.artifact_file(&path).is_err(), "{}", path.display());
        }
        fs::create_dir_all(project.join("Cache")).unwrap();
        for dir in [store.home.clone(), project.join("Cache")] {
            assert!(store.artifact_file(&dir).is_err(), "{}", dir.display());
        }
        assert!(store
            .artifact_file(&project.join("Cache/missing.svg"))
            .is_err());
    }
    #[test]
    fn artifact_file_accepts_project_instant_and_legacy_outputs() {
        let (_temp, store) = managed_store("data", "legacy");
        let project = store.project_dir(PROJECT).unwrap();
        for path in [
            project.join("Cache/a.svg"),
            project.join("Cache/PDFArtwork/a.pdf"),
            project.join("Exports/a-png.png"),
            project.join("Documents/document-1/Output/main.pdf"),
            project.join("Instant/Cache/a.svg"),
            project.join("Instant/Exports/a.svg"),
            store.home.join("Instant/Cache/a.png"),
            store.home.join("Instant/Exports/a.jpg"),
            store
                .legacy
                .join("Projects")
                .join(PROJECT)
                .join("Cache/a.svg"),
            store
                .legacy
                .join("Projects")
                .join(PROJECT)
                .join("Exports/a.pdf"),
        ] {
            let path = file(&path);
            assert_eq!(store.artifact_file(&path).unwrap(), path);
        }
    }
    #[test]
    fn artifact_resolvers_handle_home_inside_legacy_and_legacy_inside_home() {
        // Production nests home in legacy; MATHLIDELUX_DATA_DIR nests legacy in home.
        for (home, legacy) in [
            ("Slide Math/Rust Preview", "Slide Math"),
            ("data", "data/Legacy"),
        ] {
            let (_temp, store) = managed_store(home, legacy);
            let current = file(&store.project_dir(PROJECT).unwrap().join("Cache/a.svg"));
            let old = file(
                &store
                    .legacy
                    .join("Projects")
                    .join(PROJECT)
                    .join("Cache/a.svg"),
            );
            assert_eq!(store.artifact_file(&current).unwrap(), current);
            assert_eq!(store.artifact_file(&old).unwrap(), old);
            let settings = file(&store.home.join("Settings/preferences.json"));
            assert!(store.artifact_file(&settings).is_err());
            let source = store
                .project_dir(PROJECT)
                .unwrap()
                .join("Documents/document-1/Source");
            fs::create_dir_all(&source).unwrap();
            assert_eq!(store.artifact_dir(&source).unwrap(), source);
        }
    }
    #[cfg(unix)]
    #[test]
    fn artifact_file_rejects_symlink_escaping_a_managed_folder() {
        let (temp, store) = managed_store("data", "legacy");
        let cache = store.project_dir(PROJECT).unwrap().join("Cache");
        fs::create_dir_all(&cache).unwrap();
        let secret = file(&temp.path().join("secret.svg"));
        let settings = file(&store.home.join("Settings/preferences.json"));
        std::os::unix::fs::symlink(&secret, cache.join("outside.svg")).unwrap();
        std::os::unix::fs::symlink(&settings, cache.join("settings.svg")).unwrap();
        std::os::unix::fs::symlink(temp.path(), cache.join("folder")).unwrap();
        for name in ["outside.svg", "settings.svg", "folder/secret.svg"] {
            assert!(store.artifact_file(&cache.join(name)).is_err(), "{name}");
        }
        // A link into managed storage resolves to the managed file itself.
        let real = file(&cache.join("a.svg"));
        let alias = temp.path().join("alias.svg");
        std::os::unix::fs::symlink(&real, &alias).unwrap();
        assert_eq!(store.artifact_file(&alias).unwrap(), real);
    }
    #[cfg(unix)]
    #[test]
    fn artifact_file_rejects_non_utf8_components() {
        use std::{ffi::OsStr, os::unix::ffi::OsStrExt};
        let name = OsStr::from_bytes(b"\xff.svg");
        let relative = Path::new("Projects").join(PROJECT).join("Cache").join(name);
        assert!(artifact_scope(&relative).is_none());
        assert!(artifact_scope(Path::new("Projects/../Cache")).is_none());
        let (_temp, store) = managed_store("data", "legacy");
        let cache = store.project_dir(PROJECT).unwrap().join("Cache");
        fs::create_dir_all(&cache).unwrap();
        // APFS refuses such names; filesystems that store them must still be rejected.
        if fs::write(cache.join(name), b"x").is_ok() {
            assert!(store.artifact_file(&cache.join(name)).is_err());
        }
    }
    #[test]
    fn artifact_dir_accepts_document_source_and_original_only() {
        let (_temp, store) = managed_store("data", "legacy");
        let document = store
            .project_dir(PROJECT)
            .unwrap()
            .join("Documents/document-1");
        for name in ["Source", "Original", "Output"] {
            fs::create_dir_all(document.join(name)).unwrap();
        }
        let pdf = file(&document.join("Output/main.pdf"));
        assert_eq!(
            store.artifact_dir(&document.join("Source")).unwrap(),
            document.join("Source")
        );
        assert_eq!(
            store.artifact_dir(&document.join("Original")).unwrap(),
            document.join("Original")
        );
        let legacy = store
            .legacy
            .join("Projects")
            .join(PROJECT)
            .join("Documents/document-1/Source");
        fs::create_dir_all(&legacy).unwrap();
        for path in [
            document.join("Output"),
            document.clone(),
            document.join("Source/../Output"),
            legacy,
            pdf,
            store.home.join("Settings"),
            store.home.clone(),
        ] {
            assert!(store.artifact_dir(&path).is_err(), "{}", path.display());
        }
    }
    #[test]
    fn register_rejects_project_roots_inside_application_storage() {
        for (home, legacy) in [
            ("Slide Math/Rust Preview", "Slide Math"),
            ("data", "data/Legacy"),
        ] {
            let (temp, store) = managed_store(home, legacy);
            for root in [
                store.home.clone(),
                store.home.join("Settings"),
                store.home.join("Projects"),
                store.legacy.clone(),
                store.legacy.join("Projects"),
                store.legacy.join("Settings"),
                // The containers would expose every legacy project to deletion.
                store.legacy.join("Sources"),
                store.legacy.join("Imports"),
            ] {
                fs::create_dir_all(&root).unwrap();
                assert!(store.register(&root).is_err(), "{}", root.display());
            }
            // Swift created and imported projects in these legacy folders.
            for name in ["Sources/paper", "Imports/archive"] {
                let root = store.legacy.join(name);
                fs::create_dir_all(&root).unwrap();
                assert!(store.register(&root).is_ok(), "{name}");
            }
            let outside = temp.path().join("project");
            fs::create_dir_all(&outside).unwrap();
            assert!(store.register(&outside).is_ok());
            #[cfg(unix)]
            {
                let link = temp.path().join("settings-link");
                std::os::unix::fs::symlink(store.home.join("Settings"), &link).unwrap();
                assert!(store.register(&link).is_err());
            }
        }
        // Identical bases are current storage, without the legacy exception.
        let (_temp, store) = managed_store("data", "data");
        let root = store.legacy.join("Sources/paper");
        fs::create_dir_all(&root).unwrap();
        assert!(store.register(&root).is_err());
    }
    #[test]
    fn project_parents_inside_application_storage_are_refused() {
        let (temp, store) = managed_store("Slide Math/Rust Preview", "Slide Math");
        for parent in [
            store.home.clone(),
            store.home.join("Settings"),
            store.home.join("not-yet-created/projects"),
            store.legacy.clone(),
            store.legacy.join("Projects"),
        ] {
            assert!(
                store.check_project_parent(&parent).is_err(),
                "{}",
                parent.display()
            );
        }
        for parent in [
            store.legacy.join("Sources"),
            store.legacy.join("Imports"),
            temp.path().join("Projects"),
            temp.path().join("not-yet-created/projects"),
        ] {
            assert!(
                store.check_project_parent(&parent).is_ok(),
                "{}",
                parent.display()
            );
        }
    }
    #[test]
    fn extraction_exclusions_preserve_source_names_variants_and_recovery_metadata() {
        let source = json!({"id":"source", "origin":"source", "tex":"x", "name":"My score", "exportFontSize":36, "extensions":{"keep":true}});
        let custom = json!({"id":"custom", "origin":"custom", "tex":"styled", "parentID":"source"});
        let p = json!({"cards":[source,custom], "sectionAssignments":{"source":"results"}, "extensions":{"future":1}});
        let hidden = merge_scan(
            p.clone(),
            json!({"cards":[], "excludedCards":[{"id":"source","origin":"source","tex":"x"}],"outline":[],"files":[],"warnings":[]}),
        );
        assert_eq!(hidden["cards"], json!([custom]));
        assert_eq!(hidden["extractionHiddenCards"], json!([source]));
        let restored = merge_scan(
            hidden,
            json!({"cards":[{"id":"source","origin":"source","tex":"x"}],"outline":[],"files":[],"warnings":[]}),
        );
        assert_eq!(restored["cards"][0]["name"], source["name"]);
        assert_eq!(
            restored["cards"][0]["exportFontSize"],
            source["exportFontSize"]
        );
        assert_eq!(restored["cards"][1], custom);
        assert_eq!(restored["sectionAssignments"], p["sectionAssignments"]);
        assert_eq!(restored["extensions"], p["extensions"]);
    }
    #[test]
    fn corrupt_metadata_is_reported_without_blocking_other_projects() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let roots: Vec<_> = ["good", "bad"]
            .iter()
            .map(|name| {
                let root = temp.path().join(name);
                fs::create_dir(&root).unwrap();
                fs::write(root.join("main.typ"), "$ x $").unwrap();
                root
            })
            .collect();
        let good = store.open(&roots[0], None).unwrap();
        let bad = store.open(&roots[1], None).unwrap();
        let path = store
            .project_dir(string(&bad, "id"))
            .unwrap()
            .join("project.json");
        fs::write(&path, "[]").unwrap();
        assert!(store.open(&roots[1], None).is_err());
        assert!(store.save(string(&bad, "id"), &json!([])).is_err());
        assert_eq!(store.recent().unwrap().len(), 1);
        assert_eq!(store.recent().unwrap()[0]["id"], good["id"]);
        let problems = store.project_problems().unwrap();
        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0]["canRestore"], true);
        store
            .recover_project(string(&bad, "id"), string(&problems[0], "revision"))
            .unwrap();
        assert_eq!(store.recent().unwrap().len(), 2);
        assert!(store.project_problems().unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(roots[1].join("main.typ")).unwrap(),
            "$ x $"
        );
    }
    #[test]
    fn corrupt_legacy_project_is_kept_without_creating_an_empty_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("source");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("main.typ"), "$ x $").unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let ident = store.register(&root).unwrap();
        let path = store
            .legacy
            .join("Projects")
            .join(&ident)
            .join("project.json");
        atomic(&path, b"{broken").unwrap();
        assert!(store.open(&root, None).is_err());
        assert!(!store
            .project_dir(&ident)
            .unwrap()
            .join("project.json")
            .exists());
        assert_eq!(fs::read_to_string(path).unwrap(), "{broken");
    }
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
    #[cfg(unix)]
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
