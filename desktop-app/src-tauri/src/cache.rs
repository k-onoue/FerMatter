//! Bounded published caches with leases for consumers; durable user data is never evicted.
use crate::files::*;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, Weak},
    time::{Duration, Instant, SystemTime},
};

const BUDGET: u64 = 2 * 1024 * 1024 * 1024;
const MAX_AGE: Duration = Duration::from_secs(30 * 24 * 3600);
const GRACE: Duration = Duration::from_secs(24 * 3600);
const LIVE_GRACE: Duration = Duration::from_secs(5 * 60);
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
fn collect(
    folder: &Path,
    documents: bool,
    items: &mut Vec<Item>,
    excluded: &HashSet<PathBuf>,
) -> Result<()> {
    if !plain_directory(folder) {
        return Ok(());
    }
    for entry in fs::read_dir(folder).map_err(err)? {
        let entry = entry.map_err(err)?;
        let path = entry.path();
        if excluded.contains(&path) {
            continue;
        }
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
            if let Some(item) = document_item(&path)? {
                items.push(item);
            }
        } else if !documents && meta.is_dir() && ["Previews", "PDFArtwork"].contains(&name.as_str())
        {
            collect(&path, false, items, excluded)?;
        } else if !documents
            && meta.is_file()
            && ((name
                .get(..24)
                .is_some_and(|p| p.bytes().all(|c| c.is_ascii_hexdigit()))
                && ["svg", "png", "jpg", "pdf", "log"]
                    .contains(&path.extension().and_then(|s| s.to_str()).unwrap_or("")))
                || name.strip_prefix(".artifact-").is_some_and(|suffix| {
                    suffix.len() == 6 && suffix.bytes().all(|b| b.is_ascii_alphanumeric())
                }))
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
fn document_item(path: &Path) -> Result<Option<Item>> {
    let mut bytes = 0;
    let mut modified = path
        .symlink_metadata()
        .map_err(err)?
        .modified()
        .map_err(err)?;
    for child in walkdir::WalkDir::new(path) {
        let child = child.map_err(err)?;
        if child.file_type().is_symlink()
            || !(child.file_type().is_dir() || child.file_type().is_file())
        {
            return Ok(None);
        }
        let meta = child.metadata().map_err(err)?;
        if meta.is_file() {
            bytes += meta.len();
        }
        modified = modified.max(meta.modified().map_err(err)?);
    }
    Ok(Some(Item {
        path: path.to_owned(),
        bytes,
        modified,
        directory: true,
    }))
}
fn metadata(path: &Path) -> Result<Value> {
    serde_json::from_slice(&read_bounded(path, ARTIFACT_LIMIT)?).map_err(err)
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
static CLIPBOARD: Mutex<()> = Mutex::new(());
pub fn copy_images(
    home: &Path,
    paths: &[PathBuf],
    copy: impl FnOnce() -> Result<()>,
) -> Result<()> {
    let _ordering = CLIPBOARD.lock().unwrap();
    let previous: Vec<PathBuf> = if home.join("ClipboardArtifacts.json").exists() {
        serde_json::from_value(metadata(&home.join("ClipboardArtifacts.json"))?).map_err(err)?
    } else {
        vec![]
    };
    let pending: Vec<_> = previous
        .iter()
        .chain(paths)
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    // Write a conservative record before changing the system clipboard. If the
    // final narrowing fails, both old and new references are still durable.
    remember_clipboard(home, &pending)?;
    let _lease = Lease::new(home, &json!(paths));
    if let Err(error) = copy() {
        let _ = remember_clipboard(home, &previous);
        return Err(error);
    }
    if let Some(cache) = manager(home) {
        cache.lease("clipboard", &json!(paths));
    }
    if let Err(error) = remember_clipboard(home, paths) {
        eprintln!("Keeping conservative clipboard references: {error}");
    }
    Ok(())
}
pub fn maintain(home: &Path) -> Result<Value> {
    sweep(
        home,
        SystemTime::now(),
        BUDGET,
        GRACE,
        &HashSet::new(),
        &HashSet::new(),
    )
}
#[cfg(test)]
fn maintain_with(home: &Path, now: SystemTime, budget: u64) -> Result<Value> {
    sweep(home, now, budget, GRACE, &HashSet::new(), &HashSet::new())
}
fn sweep(
    home: &Path,
    now: SystemTime,
    budget: u64,
    grace: Duration,
    live: &HashSet<PathBuf>,
    working: &HashSet<PathBuf>,
) -> Result<Value> {
    let mut items = vec![];
    let mut pinned = live.clone();
    let clipboard = home.join("ClipboardArtifacts.json");
    if clipboard.exists() {
        let value = metadata(&clipboard)?;
        if !value.as_array().is_some_and(|items| {
            items
                .iter()
                .all(|v| v.as_str().is_some_and(|s| Path::new(s).is_absolute()))
        }) {
            return Err(
                "Invalid saved clipboard artifact references; cache cleanup skipped.".into(),
            );
        }
        references(&value, &mut pinned);
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
            // Both recovery copies and live metadata may own presentation variants.
            for name in ["project.json", ".last-valid-project.json"] {
                let project = entry.path().join(name);
                if project.exists() {
                    references(&crate::project_data::read(&project)?, &mut pinned);
                }
            }
            scopes.push(entry.path());
        }
    }
    // Undo backups can still reference an old project cache after its path is reused.
    let history = home.join("ProjectHistoryData");
    if plain_directory(&history) {
        for entry in fs::read_dir(history).map_err(err)? {
            let dir = entry.map_err(err)?.path();
            if !plain_directory(&dir)
                || !dir
                    .file_name()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| uuid::Uuid::parse_str(s).is_ok())
            {
                continue;
            }
            for name in ["0.json", "3.json"] {
                let path = dir.join(name);
                if path.exists() {
                    references(&crate::project_data::read(&path)?, &mut pinned);
                }
            }
        }
    }
    for scope in scopes {
        for folder in ["Cache", "Exports"] {
            collect(&scope.join(folder), false, &mut items, working)?;
            if plain_directory(&scope.join("Instant")) {
                collect(
                    &scope.join("Instant").join(folder),
                    false,
                    &mut items,
                    working,
                )?;
            }
        }
        collect(&scope.join("Documents"), true, &mut items, working)?;
    }
    let pinned: HashSet<_> = pinned
        .iter()
        .filter_map(|path| normalize(home, path))
        .collect();
    items.retain(|item| !working.contains(&item.path));
    items.sort_by_key(|item| item.modified);
    let mut retained: u64 = items.iter().map(|item| item.bytes).sum();
    let before = retained;
    let mut removed = 0;
    for item in items {
        let age = now.duration_since(item.modified).unwrap_or_default();
        let protected = pinned
            .iter()
            .any(|p| p == &item.path || (item.directory && p.starts_with(&item.path)));
        if protected || age < grace || (age < MAX_AGE && retained <= budget) {
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
// Only app-owned published artifacts are accounted here. Compiler working directories
// remain independently bounded by snapshot limits and the two-worker semaphore.
static MANAGERS: LazyLock<Mutex<Vec<Weak<Cache>>>> = LazyLock::new(|| Mutex::new(vec![]));
#[derive(Default)]
struct State {
    leases: HashMap<String, HashSet<PathBuf>>,
    generations: HashMap<String, uuid::Uuid>,
    handoffs: HashMap<String, Instant>,
    working: HashSet<PathBuf>,
    bytes: u64,
    refreshed: Option<Instant>,
}
pub struct Cache {
    home: PathBuf,
    budget: u64,
    state: Mutex<State>,
}
impl Cache {
    pub fn new(home: &Path) -> Arc<Self> {
        Self::with_budget(home, BUDGET)
    }
    fn with_budget(home: &Path, budget: u64) -> Arc<Self> {
        let cache = Arc::new(Self {
            home: home.to_owned(),
            budget,
            state: Mutex::default(),
        });
        MANAGERS.lock().unwrap().push(Arc::downgrade(&cache));
        cache
    }
    pub fn begin_result(&self, key: &str) -> uuid::Uuid {
        let generation = uuid::Uuid::new_v4();
        self.state
            .lock()
            .unwrap()
            .generations
            .insert(key.into(), generation);
        generation
    }
    pub fn result(&self, key: &str, generation: uuid::Uuid, value: &Value) {
        let mut state = self.state.lock().unwrap();
        if state.generations.get(key) == Some(&generation) {
            self.replace_lease(&mut state, key, value);
            state.handoffs.insert(key.into(), Instant::now());
        }
    }
    pub fn lease(&self, key: &str, value: &Value) {
        self.replace_lease(&mut self.state.lock().unwrap(), key, value);
    }
    fn replace_lease(&self, state: &mut State, key: &str, value: &Value) {
        let mut paths = HashSet::new();
        references(value, &mut paths);
        let paths: HashSet<_> = paths
            .iter()
            .filter_map(|p| normalize(&self.home, p))
            .collect();
        if paths.is_empty() {
            if let Some(old) = state.leases.remove(key) {
                touch(&self.home, &old);
            }
        } else {
            touch(&self.home, &paths);
            if let Some(old) = state.leases.insert(key.into(), paths) {
                touch(&self.home, &old);
            }
        }
    }
    pub fn release_window(&self, prefix: &str) {
        let mut state = self.state.lock().unwrap();
        state.generations.retain(|key, _| !key.starts_with(prefix));
        state.handoffs.retain(|key, _| !key.starts_with(prefix));
        state.leases.retain(|key, paths| {
            if key.starts_with(prefix) {
                touch(&self.home, paths);
                false
            } else {
                true
            }
        });
    }
    fn refresh(&self, state: &mut State, budget: u64) -> Result<()> {
        let expired: Vec<_> = state
            .handoffs
            .iter()
            .filter(|(_, started)| started.elapsed() >= LIVE_GRACE)
            .map(|(key, _)| key.clone())
            .collect();
        for key in expired {
            // This only bridges delivery to a webview. Visible/retained state owns
            // explicit leases; an old result must not outlive its closed preview.
            state.handoffs.remove(&key);
            state.leases.remove(&key);
        }
        let pinned = state
            .leases
            .values()
            .flat_map(|v| v.iter().cloned())
            .collect();
        let report = sweep(
            &self.home,
            SystemTime::now(),
            budget,
            LIVE_GRACE,
            &pinned,
            &state.working,
        )?;
        state.bytes = report["retainedBytes"].as_u64().unwrap();
        state.refreshed = Some(Instant::now());
        json_write(&self.home.join("CacheMaintenance.json"), &report)?;
        Ok(())
    }
    pub fn idle(&self) -> Result<()> {
        let mut state = self.state.lock().unwrap();
        if state
            .refreshed
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(30))
        {
            self.refresh(&mut state, self.budget)?;
        }
        Ok(())
    }
    fn admit(&self, state: &mut State, extra: u64) -> Result<()> {
        if state
            .refreshed
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(30))
            || state.bytes.saturating_add(extra) > self.budget
        {
            self.refresh(state, self.budget.saturating_sub(extra))?;
        }
        if state.bytes.saturating_add(extra) > self.budget {
            return Err("The image cache is full (2 GiB). Close unused previews and try again after five minutes. Saved work has been kept.".into());
        }
        Ok(())
    }
}
fn manager(path: &Path) -> Option<Arc<Cache>> {
    let mut managers = MANAGERS.lock().unwrap();
    managers.retain(|v| v.strong_count() > 0);
    managers
        .iter()
        .filter_map(Weak::upgrade)
        .find(|cache| path.starts_with(&cache.home))
}
fn managed_path(home: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(home) else {
        return false;
    };
    let components: Vec<_> = relative
        .components()
        .filter_map(|c| c.as_os_str().to_str())
        .collect();
    let scope = if components.first() == Some(&"Projects")
        && components.len() > 2
        && components[1].len() == 24
        && components[1].bytes().all(|b| b.is_ascii_hexdigit())
    {
        &components[2..]
    } else {
        &components[..]
    };
    let scope = if scope.first() == Some(&"Instant") {
        &scope[1..]
    } else {
        scope
    };
    scope
        .first()
        .is_some_and(|name| ["Cache", "Exports", "Documents"].contains(name))
        && !scope.contains(&"..")
}
fn normalize(home: &Path, path: &Path) -> Option<PathBuf> {
    let real_home = home.canonicalize().unwrap_or_else(|_| home.to_owned());
    if let Ok(real) = path.canonicalize() {
        let relative = real.strip_prefix(&real_home).ok()?;
        let normalized = home.join(relative);
        return managed_path(home, &normalized).then_some(normalized);
    }
    managed_path(home, path).then(|| path.to_owned())
}
fn touch(home: &Path, paths: &HashSet<PathBuf>) {
    for path in paths {
        let Some(path) = normalize(home, path) else {
            continue;
        };
        if !path
            .symlink_metadata()
            .is_ok_and(|m| m.is_file() || m.is_dir())
        {
            continue;
        }
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.access_mode(0x100).custom_flags(0x0220_0000);
        }
        if let Ok(file) = options.open(&path) {
            let _ = file.set_modified(SystemTime::now());
        }
    }
}

pub struct Lease {
    cache: Option<Arc<Cache>>,
    key: String,
}
impl Lease {
    pub fn new(path: &Path, value: &Value) -> Self {
        let cache = manager(path);
        let key = format!("request-{}", uuid::Uuid::new_v4());
        if let Some(cache) = &cache {
            cache.lease(&key, value);
        }
        Self { cache, key }
    }
    pub fn add(&self, value: &Value) {
        if let Some(cache) = &self.cache {
            let mut paths = HashSet::new();
            references(value, &mut paths);
            let paths: HashSet<_> = paths
                .iter()
                .filter_map(|p| normalize(&cache.home, p))
                .collect();
            let mut state = cache.state.lock().unwrap();
            touch(&cache.home, &paths);
            state
                .leases
                .entry(self.key.clone())
                .or_default()
                .extend(paths);
        }
    }
}
impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(cache) = &self.cache {
            cache.lease(&self.key, &Value::Null);
        }
    }
}
/// Copy from private staging only after quota admission. Publication never mutates
/// a file another window or external app may already be reading.
pub fn publish(files: &[(PathBuf, PathBuf)]) -> Result<()> {
    publish_checked(files, &|| Ok(()))
}
pub fn publish_checked(files: &[(PathBuf, PathBuf)], check: &dyn Fn() -> Result<()>) -> Result<()> {
    check()?;
    let Some((_, first)) = files.first() else {
        return Ok(());
    };
    let cache = manager(first);
    let mut lock = cache.as_ref().map(|c| c.state.lock().unwrap());
    let mut staged = vec![];
    let mut extra = 0;
    for (source, dest) in files {
        if dest.is_file() {
            regular_file(dest)?;
            continue;
        }
        let parent = dest.parent().ok_or("Invalid artifact path")?;
        fs::create_dir_all(parent).map_err(err)?;
        let mut temp = tempfile::Builder::new()
            .prefix(".artifact-")
            .tempfile_in(parent)
            .map_err(err)?;
        let mut input = regular_file(source)?;
        if input.metadata().map_err(err)?.len() > ARTIFACT_LIMIT {
            return Err("Rendered image exceeds the 100 MiB limit.".into());
        }
        let mut buffer = [0; 64 * 1024];
        let mut bytes = 0;
        loop {
            check()?;
            let n = input.read(&mut buffer).map_err(err)?;
            if n == 0 {
                break;
            }
            bytes += n as u64;
            if bytes > ARTIFACT_LIMIT {
                return Err("Rendered image exceeds the 100 MiB limit.".into());
            }
            temp.write_all(&buffer[..n]).map_err(err)?;
        }
        extra += bytes;
        staged.push((temp, dest, bytes));
    }
    if staged.is_empty() {
        return Ok(());
    }
    if let (Some(cache), Some(state)) = (&cache, lock.as_mut()) {
        state
            .working
            .extend(staged.iter().map(|(temp, _, _)| temp.path().to_owned()));
        if let Err(error) = cache.admit(state, extra) {
            for (temp, _, _) in &staged {
                state.working.remove(temp.path());
            }
            return Err(error);
        }
    }
    if let Some(state) = lock.as_mut() {
        for (temp, _, _) in &staged {
            state.working.remove(temp.path());
        }
    }
    for (temp, dest, bytes) in staged {
        check()?;
        temp.persist_noclobber(dest).map_err(err)?;
        if let Some(state) = lock.as_mut() {
            state.bytes += bytes;
        }
    }
    Ok(())
}
pub struct DocumentWork {
    cache: Option<Arc<Cache>>,
    path: PathBuf,
}
impl DocumentWork {
    pub fn new(path: &Path) -> Self {
        let cache = manager(path);
        if let Some(cache) = &cache {
            cache.state.lock().unwrap().working.insert(path.to_owned());
        }
        Self {
            cache,
            path: path.to_owned(),
        }
    }
    pub fn commit(&self) -> Result<()> {
        if let Some(cache) = &self.cache {
            let mut state = cache.state.lock().unwrap();
            let bytes = document_item(&self.path)?
                .ok_or("Invalid document snapshot")?
                .bytes;
            cache.admit(&mut state, bytes)?;
            state.bytes += bytes;
        }
        Ok(())
    }
}
impl Drop for DocumentWork {
    fn drop(&mut self) {
        if let Some(cache) = &self.cache {
            cache.state.lock().unwrap().working.remove(&self.path);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn age(path: &Path) {
        for entry in walkdir::WalkDir::new(path).contents_first(true) {
            let entry = entry.unwrap();
            let mut options = fs::OpenOptions::new();
            options.read(true);
            #[cfg(windows)]
            {
                use std::os::windows::fs::OpenOptionsExt;
                options.access_mode(0x100).custom_flags(0x0200_0000);
            }
            options
                .open(entry.path())
                .unwrap()
                .set_modified(SystemTime::now() - MAX_AGE - GRACE)
                .unwrap();
        }
    }
    fn image(home: &Path, number: u32) -> PathBuf {
        home.join(format!("Instant/Exports/{number:024x}.svg"))
    }
    #[test]
    fn returned_result_handoffs_expire_but_frontend_owned_previews_stay_live() {
        let temp = tempfile::tempdir().unwrap();
        let cache = Cache::with_budget(temp.path(), 100);
        let file = image(temp.path(), 1);
        atomic(&file, b"preview").unwrap();
        let key = "main::result::source_preview";
        let generation = cache.begin_result(key);
        cache.result(key, generation, &json!(file));
        cache.lease("main::artifacts::visible", &json!(file));
        age(&file);
        {
            let mut state = cache.state.lock().unwrap();
            state.handoffs.insert(
                key.into(),
                Instant::now() - LIVE_GRACE - Duration::from_secs(1),
            );
            cache.refresh(&mut state, 0).unwrap();
            assert!(!state.leases.contains_key(key));
        }
        assert!(file.exists());
        cache.lease("main::artifacts::visible", &Value::Null);
        age(&file);
        cache.refresh(&mut cache.state.lock().unwrap(), 0).unwrap();
        assert!(!file.exists());
    }
    #[test]
    fn clipboard_publication_matches_native_order_and_failed_copy_keeps_previous_refs() {
        let temp = tempfile::tempdir().unwrap();
        let cache = Cache::with_budget(temp.path(), 100);
        let native = Arc::new(Mutex::new(PathBuf::new()));
        let threads: Vec<_> = (0..6)
            .map(|i| {
                let home = temp.path().to_owned();
                let native = native.clone();
                let path = image(&home, i);
                atomic(&path, b"image").unwrap();
                std::thread::spawn(move || {
                    copy_images(&home, std::slice::from_ref(&path), || {
                        *native.lock().unwrap() = path.clone();
                        std::thread::sleep(Duration::from_millis(2));
                        Ok(())
                    })
                    .unwrap()
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        let actual = native.lock().unwrap().clone();
        assert_eq!(
            metadata(&temp.path().join("ClipboardArtifacts.json")).unwrap(),
            json!([actual.clone()])
        );
        assert!(copy_images(temp.path(), &[image(temp.path(), 99)], || Err(
            "failed".into()
        ))
        .is_err());
        assert_eq!(
            metadata(&temp.path().join("ClipboardArtifacts.json")).unwrap(),
            json!([actual.clone()])
        );
        assert_eq!(
            cache.state.lock().unwrap().leases["clipboard"],
            HashSet::from([actual])
        );
    }
    #[test]
    fn late_results_cannot_replace_newer_visible_output_or_revive_a_closed_window() {
        let temp = tempfile::tempdir().unwrap();
        let cache = Cache::with_budget(temp.path(), 100);
        let first = image(temp.path(), 1);
        let second = image(temp.path(), 2);
        atomic(&first, b"first").unwrap();
        atomic(&second, b"second").unwrap();
        let key = "instant::result::instant_render";
        let a = cache.begin_result(key);
        let b = cache.begin_result(key);
        cache.result(key, b, &json!(second));
        cache.result(key, a, &json!(first));
        assert_eq!(
            cache.state.lock().unwrap().leases[key],
            HashSet::from([second])
        );
        cache.release_window("instant::");
        cache.result(key, b, &json!(first));
        assert!(!cache.state.lock().unwrap().leases.contains_key(key));
        let c = cache.begin_result(key);
        cache.result(key, c, &json!(first));
        assert!(cache.state.lock().unwrap().leases.contains_key(key));
    }
    #[test]
    fn structurally_invalid_project_metadata_aborts_cleanup_without_deleting_artifacts() {
        let temp = tempfile::tempdir().unwrap();
        let retained = image(temp.path(), 1);
        atomic(&retained, b"recoverable").unwrap();
        age(&retained);
        let metadata = temp
            .path()
            .join("Projects/aaaaaaaaaaaaaaaaaaaaaaaa/project.json");
        json_write(&metadata, &json!({"root":temp.path(),"cards":"invalid"})).unwrap();
        assert!(maintain_with(temp.path(), SystemTime::now(), 0).is_err());
        assert_eq!(fs::read(retained).unwrap(), b"recoverable");
    }
    #[test]
    fn existing_over_budget_artifacts_remain_reusable_and_are_never_replaced() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("data");
        let _cache = Cache::with_budget(&home, 4);
        let existing = image(&home, 1);
        atomic(&existing, b"existing").unwrap();
        let input = temp.path().join("input");
        fs::write(&input, b"different").unwrap();
        publish(&[(input, existing.clone())]).unwrap();
        assert_eq!(fs::read(existing).unwrap(), b"existing");
    }
    #[test]
    fn cancelling_publication_drops_staging_without_publishing_a_partial_file() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("data");
        let cache = Cache::with_budget(&home, 1_000_000);
        let input = temp.path().join("input");
        fs::write(&input, vec![0; 512 * 1024]).unwrap();
        let output = image(&home, 1);
        let ticks = std::cell::Cell::new(0);
        let result = publish_checked(&[(input, output.clone())], &|| {
            ticks.set(ticks.get() + 1);
            if ticks.get() >= 4 {
                Err("Cancelled".into())
            } else {
                Ok(())
            }
        });
        assert_eq!(result.unwrap_err(), "Cancelled");
        assert!(!output.exists());
        assert_eq!(fs::read_dir(output.parent().unwrap()).unwrap().count(), 0);
        assert_eq!(cache.state.lock().unwrap().bytes, 0);
    }
    #[cfg(unix)]
    #[test]
    fn leases_normalize_aliases_without_touching_source_or_symlink_escape() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("data");
        fs::create_dir_all(&home).unwrap();
        let alias = temp.path().join("alias");
        std::os::unix::fs::symlink(&home, &alias).unwrap();
        let cache = Cache::with_budget(&alias, 0);
        let file = image(&home, 1);
        atomic(&file, b"keep").unwrap();
        cache.lease("main::pdf", &json!(file.canonicalize().unwrap()));
        age(&file);
        cache.refresh(&mut cache.state.lock().unwrap(), 0).unwrap();
        assert!(file.exists());
        let original = temp.path().join("original.typ");
        atomic(&original, b"private").unwrap();
        age(&original);
        let before = original.metadata().unwrap().modified().unwrap();
        let link = alias.join("Cache");
        std::os::unix::fs::symlink(temp.path(), &link).unwrap();
        cache.lease("main::source", &json!(link.join("original.typ")));
        assert_eq!(original.metadata().unwrap().modified().unwrap(), before);
    }
    #[test]
    fn startup_reclaims_only_owned_abandoned_publication_files() {
        let temp = tempfile::tempdir().unwrap();
        let abandoned = temp.path().join("Cache/.artifact-ABC123");
        let unrelated = temp.path().join("Cache/.artifact-private.tex");
        atomic(&abandoned, b"unfinished").unwrap();
        atomic(&unrelated, b"keep").unwrap();
        age(&abandoned);
        age(&unrelated);
        maintain_with(temp.path(), SystemTime::now(), 0).unwrap();
        assert!(!abandoned.exists());
        assert!(unrelated.exists());
    }
    #[test]
    fn quota_keeps_live_and_clipboard_files_then_reclaims_retired_generations() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("data");
        let cache = Cache::with_budget(&home, 12);
        let input = temp.path().join("input");
        fs::write(&input, b"123456").unwrap();
        let first = image(&home, 1);
        let second = image(&home, 2);
        publish(&[(input.clone(), first.clone())]).unwrap();
        publish(&[(input.clone(), second.clone())]).unwrap();
        cache.lease("main::drag", &json!(first));
        remember_clipboard(&home, std::slice::from_ref(&second)).unwrap();
        age(&first);
        age(&second);
        assert!(publish(&[(input.clone(), image(&home, 3))]).is_err());
        assert_eq!(fs::read(&first).unwrap(), b"123456");
        assert!(second.exists());
        cache.lease("main::drag", &Value::Null);
        // End of native drag starts grace even when the artifact is old.
        assert!(publish(&[(input.clone(), image(&home, 3))]).is_err());
        age(&first);
        for number in 3..30 {
            let next = image(&home, number);
            publish(&[(input.clone(), next.clone())]).unwrap();
            assert!(second.exists());
            assert!(cache.state.lock().unwrap().bytes <= 12);
            age(&next);
        }
        assert!(!first.exists());
        assert!(!image(&home, 3).exists());
        assert!(image(&home, 29).exists());
    }
    #[test]
    fn parallel_publish_cannot_over_admit_or_replace_a_visible_file() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path().join("data");
        let cache = Cache::with_budget(&home, 10);
        let input = temp.path().join("input");
        fs::write(&input, b"123456").unwrap();
        let threads: Vec<_> = (0..8)
            .map(|number| {
                let source = input.clone();
                let dest = image(&home, number);
                std::thread::spawn(move || publish(&[(source, dest)]))
            })
            .collect();
        let completed: Vec<_> = threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .collect();
        assert_eq!(completed.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(cache.state.lock().unwrap().bytes, 6);
    }
    #[test]
    fn document_viewers_and_work_in_progress_survive_idle_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let cache = Cache::with_budget(home, 8);
        let current = home.join(format!("Documents/document-{}", uuid::Uuid::new_v4()));
        let work = DocumentWork::new(&current);
        atomic(&current.join("Source/main.typ"), b"keep").unwrap();
        work.commit().unwrap();
        drop(work);
        cache.lease("main::pdf", &json!(current.join("Source")));
        age(&current);
        let pending = home.join(format!("Documents/document-{}", uuid::Uuid::new_v4()));
        let work = DocumentWork::new(&pending);
        atomic(&pending.join("Source/main.typ"), b"incomplete").unwrap();
        age(&pending);
        let mut state = cache.state.lock().unwrap();
        cache.refresh(&mut state, 0).unwrap();
        assert_eq!(state.bytes, 4);
        drop(state);
        assert!(current.exists() && pending.exists());
        assert!(work.commit().is_err());
        assert_eq!(fs::read(current.join("Source/main.typ")).unwrap(), b"keep");
        drop(work);
        fs::remove_dir_all(&pending).unwrap();
        cache.release_window("main::");
        age(&current);
        cache.refresh(&mut cache.state.lock().unwrap(), 0).unwrap();
        assert!(!current.exists());
    }
    #[test]
    fn backup_and_project_undo_references_are_never_evicted() {
        let temp = tempfile::tempdir().unwrap();
        let home = temp.path();
        let backup = image(home, 1);
        let undo = image(home, 2);
        atomic(&backup, b"backup").unwrap();
        atomic(&undo, b"undo").unwrap();
        json_write(
            &home.join("Projects/aaaaaaaaaaaaaaaaaaaaaaaa/.last-valid-project.json"),
            &json!({"root":home.join("Original"),"cards":[{"tex":"x", "svgPath":backup}]}),
        )
        .unwrap();
        json_write(
            &home.join(format!(
                "ProjectHistoryData/{}/0.json",
                uuid::Uuid::new_v4()
            )),
            &json!({"root":home.join("Original"),"cards":[{"tex":"x", "svgPath":undo}]}),
        )
        .unwrap();
        age(&backup);
        age(&undo);
        let report = maintain_with(home, SystemTime::now(), 0).unwrap();
        assert_eq!(report["removedEntries"], 0);
        assert!(backup.exists() && undo.exists());
    }
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
            &json!({"root":home.join("Original"),"cards":[{"tex":"x", "svgPath":pinned}]}),
        )
        .unwrap();
        remember_clipboard(home, std::slice::from_ref(&copied)).unwrap();
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
