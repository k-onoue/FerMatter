use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use walkdir::WalkDir;

pub type Result<T> = std::result::Result<T, String>;
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn hash(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}
pub fn id(s: &str) -> String {
    hash(s)[..24].into()
}
pub fn string<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or("")
}
pub fn ignored(name: &str) -> bool {
    matches!(
        name,
        ".git" | ".build" | "build" | "target" | "node_modules" | "__MACOSX" | "__pycache__"
    )
}
pub fn relative(name: &str) -> Result<&Path> {
    let p = Path::new(name);
    if name.is_empty() || p.components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("Choose a path inside the project.".into());
    }
    Ok(p)
}
pub fn safe(root: &Path, name: &str) -> Result<PathBuf> {
    let root = root.canonicalize().map_err(err)?;
    let p = root.join(relative(name)?).canonicalize().map_err(err)?;
    if !p.starts_with(&root) || p == root {
        return Err("Path leaves the project.".into());
    }
    Ok(p)
}
pub fn destination(root: &Path, name: &str) -> Result<PathBuf> {
    let path = root.join(relative(name)?);
    let parent = path.parent().unwrap().canonicalize().map_err(err)?;
    if !parent.starts_with(root) {
        return Err("Path leaves the project.".into());
    }
    if path.exists() || path.symlink_metadata().is_ok() {
        return Err("A file with that name already exists.".into());
    }
    Ok(parent.join(path.file_name().unwrap()))
}
pub fn rename_no_replace(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let source = CString::new(source.as_os_str().as_bytes()).map_err(err)?;
        let destination = CString::new(destination.as_os_str().as_bytes()).map_err(err)?;
        if unsafe { libc::renamex_np(source.as_ptr(), destination.as_ptr(), libc::RENAME_EXCL) }
            != 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let source = CString::new(source.as_os_str().as_bytes()).map_err(err)?;
        let destination = CString::new(destination.as_os_str().as_bytes()).map_err(err)?;
        if unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                source.as_ptr(),
                libc::AT_FDCWD,
                destination.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        } != 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
        Ok(())
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        let _ = (source, destination);
        Err("Safe file moves are not implemented on this platform.".into())
    }
}
pub const EDITOR_LIMIT: u64 = 10_000_000;
pub const ARTIFACT_LIMIT: u64 = 100 * 1024 * 1024;
pub const PROJECT_LIMIT: u64 = 1024 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;

/// Open without following a replacement symlink or blocking on a FIFO.
pub fn regular_file(path: &Path) -> Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW);
    }
    let file = options.open(path).map_err(err)?;
    if !file.metadata().map_err(err)?.is_file() {
        return Err("Only regular files can be read.".into());
    }
    Ok(file)
}
fn chunks(
    path: &Path,
    limit: u64,
    check: &dyn Fn() -> Result<()>,
    mut visit: impl FnMut(&[u8]) -> Result<()>,
) -> Result<u64> {
    check()?;
    let mut file = regular_file(path)?;
    if file.metadata().map_err(err)?.len() > limit {
        return Err(format!(
            "File exceeds the remaining {} MB limit.",
            limit / 1024 / 1024
        ));
    }
    let mut buffer = [0; CHUNK];
    let mut total = 0;
    loop {
        check()?;
        let n = file.read(&mut buffer).map_err(err)?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > limit {
            return Err("File grew beyond its size limit while being read.".into());
        }
        visit(&buffer[..n])?;
    }
    check()?;
    Ok(total)
}
pub fn read_bounded(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    chunks(path, limit, &|| Ok(()), |chunk| {
        bytes.extend_from_slice(chunk);
        Ok(())
    })?;
    Ok(bytes)
}
pub fn read(path: &Path) -> Result<String> {
    Ok(String::from_utf8(read_bounded(path, EDITOR_LIMIT)?)
        .map_err(err)?
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n"))
}
pub fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("Missing parent directory")?;
    fs::create_dir_all(parent).map_err(err)?;
    let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(err)?;
    if let Ok(meta) = path.metadata() {
        temp.as_file()
            .set_permissions(meta.permissions())
            .map_err(err)?;
    }
    temp.write_all(bytes).map_err(err)?;
    temp.as_file().sync_all().map_err(err)?;
    temp.persist(path).map_err(err)?;
    Ok(())
}
pub fn json_read(path: &Path) -> Result<Value> {
    serde_json::from_slice(&fs::read(path).map_err(err)?).map_err(err)
}
pub fn json_write(path: &Path, value: &Value) -> Result<()> {
    atomic(path, &serde_json::to_vec_pretty(value).map_err(err)?)
}
pub fn entries(root: &Path) -> Result<Vec<(String, PathBuf, bool)>> {
    entries_checked(root, &|| Ok(()))
}
pub fn entries_checked(
    root: &Path,
    check: &dyn Fn() -> Result<()>,
) -> Result<Vec<(String, PathBuf, bool)>> {
    let mut result = vec![];
    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| e.depth() == 0 || !ignored(&e.file_name().to_string_lossy()))
    {
        check()?;
        let e = entry.map_err(err)?;
        if e.depth() == 0 {
            continue;
        }
        // A compiler snapshot must never follow a link to a private file outside the selected project.
        if e.file_type().is_symlink() {
            continue;
        }
        if !e.file_type().is_dir() && !e.file_type().is_file() {
            return Err(format!(
                "Project contains a special file: {}",
                e.path().display()
            ));
        }
        result.push((
            e.path()
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            e.path().to_path_buf(),
            e.file_type().is_dir(),
        ));
        if result.len() > 30_000 {
            return Err("Project exceeds 30,000 entries.".into());
        }
    }
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}
pub fn snapshot(root: &Path, target: &Path, drafts: &Value) -> Result<()> {
    snapshot_checked(root, target, drafts, &|| Ok(()))
}
pub fn snapshot_checked(
    root: &Path,
    target: &Path,
    drafts: &Value,
    check: &dyn Fn() -> Result<()>,
) -> Result<()> {
    check()?;
    fs::create_dir_all(target).map_err(err)?;
    let mut size = 0;
    for (name, path, dir) in entries_checked(root, check)? {
        check()?;
        let out = target.join(&name);
        if dir {
            fs::create_dir_all(out).map_err(err)?;
        } else {
            let mut dest = tempfile::NamedTempFile::new_in(out.parent().unwrap()).map_err(err)?;
            size += chunks(&path, PROJECT_LIMIT - size, check, |chunk| {
                dest.write_all(chunk).map_err(err)
            })?;
            dest.persist(&out).map_err(err)?;
        }
    }
    if let Some(map) = drafts.as_object() {
        for (name, content) in map {
            check()?;
            relative(name)?;
            if root.join(name).exists() {
                safe(root, name)?;
            }
            let content = content.as_str().ok_or("Invalid draft")?;
            size += content.len() as u64;
            if size > PROJECT_LIMIT || content.len() as u64 > EDITOR_LIMIT {
                return Err("Drafts exceed the project or editor size limit.".into());
            }
            atomic(&target.join(name), content.as_bytes())?;
        }
    }
    check()
}
pub fn fingerprint(root: &Path) -> Result<String> {
    fingerprint_checked(root, &|| Ok(()))
}
pub fn fingerprint_checked(root: &Path, check: &dyn Fn() -> Result<()>) -> Result<String> {
    let mut digest = Sha256::new();
    let mut size = 0;
    for (name, path, dir) in entries_checked(root, check)? {
        if dir
            || name == ".mathlidelux-project.json"
            || matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("aux" | "log" | "out" | "toc" | "fls" | "fdb_latexmk" | "synctex")
            )
        {
            continue;
        }
        digest.update(name);
        size += chunks(&path, PROJECT_LIMIT - size, check, |chunk| {
            digest.update(chunk);
            Ok(())
        })?;
    }
    check()?;
    Ok(format!("{:x}", digest.finalize()))
}
pub fn read_buffer(root: &Path, name: &str) -> Result<Value> {
    let path = safe(root, name)?;
    let bytes = read_bounded(&path, EDITOR_LIMIT)?;
    let text = String::from_utf8(bytes.clone()).map_err(|_| "This file is not UTF-8 text.")?;
    if bytes.len() > 10_000_000 || text.contains('\0') {
        return Err("This file cannot be opened in the text editor.".into());
    }
    Ok(
        json!({"path":name,"text":text.trim_start_matches('\u{feff}').replace("\r\n","\n"),"revision":hash(&bytes),"bom":bytes.starts_with(&[239,187,191]),"newline":if text.contains("\r\n") {"\r\n"} else {"\n"}}),
    )
}
pub fn save_buffer(root: &Path, buffer: &Value) -> Result<Value> {
    let name = string(buffer, "path");
    let path = safe(root, name)?;
    if hash(read_bounded(&path, EDITOR_LIMIT)?) != string(buffer, "revision") {
        return Err(
            "This file changed on disk. Reload it or save your draft to a new file.".into(),
        );
    }
    let mut text = string(buffer, "text").replace("\r\n", "\n");
    if string(buffer, "newline") == "\r\n" {
        text = text.replace('\n', "\r\n");
    }
    if buffer["bom"].as_bool() == Some(true) {
        text.insert(0, '\u{feff}');
    }
    atomic(&path, text.as_bytes())?;
    read_buffer(root, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn large_streaming_fixture_records_cancel_latency() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("large.dat");
        fs::File::create(file)
            .unwrap()
            .set_len(256 * 1024 * 1024)
            .unwrap();
        let start = std::time::Instant::now();
        fingerprint(temp.path()).unwrap();
        let hashed = start.elapsed();
        let checks = std::cell::Cell::new(0);
        let start = std::time::Instant::now();
        let result = fingerprint_checked(temp.path(), &|| {
            checks.set(checks.get() + 1);
            if checks.get() > 100 {
                Err("Cancelled".into())
            } else {
                Ok(())
            }
        });
        assert_eq!(result.unwrap_err(), "Cancelled");
        println!(
            "256 MiB streaming hash: {hashed:?}; cancellation after 100 checks: {:?}",
            start.elapsed()
        );
    }
    #[test]
    fn reads_are_bounded_before_allocation_and_cancel_between_chunks() {
        use std::cell::Cell;
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("large.dat");
        fs::write(&path, vec![0; CHUNK * 4]).unwrap();
        assert_eq!(
            read_bounded(&path, (CHUNK * 4) as u64).unwrap().len(),
            CHUNK * 4
        );
        assert!(read_bounded(&path, (CHUNK * 4 - 1) as u64).is_err());
        fs::File::create(&path)
            .unwrap()
            .set_len(PROJECT_LIMIT + 1)
            .unwrap();
        assert!(fingerprint(temp.path()).is_err());
        fs::write(&path, vec![0; CHUNK * 4]).unwrap();
        let checks = Cell::new(0);
        let cancel = || {
            let n = checks.get() + 1;
            checks.set(n);
            if n >= 4 {
                Err("Cancelled".into())
            } else {
                Ok(())
            }
        };
        assert_eq!(
            fingerprint_checked(temp.path(), &cancel).unwrap_err(),
            "Cancelled"
        );
        let out = tempfile::tempdir().unwrap();
        checks.set(0);
        assert_eq!(
            snapshot_checked(temp.path(), out.path(), &Value::Null, &cancel).unwrap_err(),
            "Cancelled"
        );
        assert!(!out.path().join("large.dat").exists());
        assert!(out.path().read_dir().unwrap().next().is_none());
    }
    #[cfg(unix)]
    #[test]
    fn special_files_cannot_block_reads_or_snapshots() {
        use std::{ffi::CString, os::unix::ffi::OsStrExt};
        let temp = tempfile::tempdir().unwrap();
        let pipe = temp.path().join("source.typ");
        let name = CString::new(pipe.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(read(&pipe).is_err());
        assert!(entries(temp.path()).is_err());
        assert!(fingerprint(temp.path()).is_err());
    }
    #[test]
    fn conflict_and_encoding() {
        let t = tempfile::tempdir().unwrap();
        let root = t.path().canonicalize().unwrap();
        let file = root.join("a.typ");
        fs::write(&file, "\u{feff}a\r\nb").unwrap();
        let mut b = read_buffer(&root, "a.typ").unwrap();
        b["text"] = json!("c\nd");
        save_buffer(&root, &b).unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "\u{feff}c\r\nd");
        assert!(save_buffer(&root, &b).is_err());
    }
    #[test]
    fn boundaries() {
        let t = tempfile::tempdir().unwrap();
        assert!(safe(t.path(), "../a").is_err());
        assert!(relative("/tmp/a").is_err());
        assert!(relative("a/../b").is_err());
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn rename_never_overwrites_existing_file() {
        let t = tempfile::tempdir().unwrap();
        let source = t.path().join("a.typ");
        let destination = t.path().join("b.typ");
        fs::write(&source, "source").unwrap();
        fs::write(&destination, "existing").unwrap();
        assert!(rename_no_replace(&source, &destination).is_err());
        assert_eq!(read(&source).unwrap(), "source");
        assert_eq!(read(&destination).unwrap(), "existing");
        let moved = t.path().join("c.typ");
        rename_no_replace(&source, &moved).unwrap();
        assert_eq!(read(&moved).unwrap(), "source");
        assert!(!source.exists());
    }
    #[test]
    fn snapshot_does_not_touch_original() {
        let t = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        fs::write(t.path().join("a.typ"), "old").unwrap();
        snapshot(t.path(), out.path(), &json!({"a.typ":"new"})).unwrap();
        assert_eq!(read(&t.path().join("a.typ")).unwrap(), "old");
        assert_eq!(read(&out.path().join("a.typ")).unwrap(), "new");
    }
}
