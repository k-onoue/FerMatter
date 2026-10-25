use crate::{files::*, scan, storage};
use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn default_directory() -> Result<PathBuf> {
    let documents = dirs::document_dir();
    // Minimal Linux installations may not define XDG_DOCUMENTS_DIR.
    #[cfg(target_os = "linux")]
    let documents = documents.or_else(|| dirs::home_dir().map(|home| home.join("Documents")));
    Ok(documents
        .ok_or("Documents folder is unavailable")?
        .join("Mathlidelux"))
}

pub fn validate_path(path: &str) -> Result<()> {
    if !path.is_empty() && (!Path::new(path).is_absolute() || path.contains('\0')) {
        return Err("Choose an absolute project folder path.".into());
    }
    Ok(())
}

pub fn directory(preferences: &Value, override_path: &str) -> Result<PathBuf> {
    let configured = if override_path.is_empty() {
        string(preferences, "projectDirectory")
    } else {
        override_path
    };
    validate_path(configured)?;
    if configured.is_empty() {
        default_directory()
    } else {
        Ok(PathBuf::from(configured))
    }
}

pub fn create(preferences: &Value, input: &Value) -> Result<PathBuf> {
    let name = string(input, "name").trim();
    if name.is_empty() || name.contains(['/', ':', '\0']) || name == "." || name == ".." {
        return Err("Choose a valid project name.".into());
    }
    let parent = directory(preferences, string(input, "directory"))?;
    fs::create_dir_all(&parent).map_err(err)?;
    let target = parent.join(name);
    if target.symlink_metadata().is_ok() {
        return Err("A project with that name already exists.".into());
    }
    let temp = tempfile::tempdir_in(&parent).map_err(err)?;
    let typst = string(input, "format") == "typst";
    fs::write(
        temp.path()
            .join(if typst { "main.typ" } else { "main.tex" }),
        string(input, "text"),
    )
    .map_err(err)?;
    if !typst {
        fs::write(temp.path().join("latexmkrc"), "$pdf_mode = 1;\n").map_err(err)?;
    }
    rename_no_replace(temp.path(), &target)?;
    Ok(target)
}

// Stage the whole import before publishing it; never merge into an existing folder.
pub fn publish(source: &Path, parent: &Path, name: &str) -> Result<PathBuf> {
    for suffix in 0..10_000 {
        let name = if suffix == 0 {
            name.to_owned()
        } else {
            format!("{name} ({suffix})")
        };
        let target = parent.join(name);
        if target.symlink_metadata().is_ok() {
            continue;
        }
        match rename_no_replace(source, &target) {
            Ok(()) => return Ok(target),
            Err(_) if target.symlink_metadata().is_ok() => continue,
            Err(error) => return Err(error),
        }
    }
    Err("Too many projects with the same name.".into())
}

pub fn import(path: &Path, parent: &Path) -> Result<PathBuf> {
    if path.is_file()
        && path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("zip"))
    {
        return storage::import_zip(path, parent);
    }
    if !path.is_dir() {
        return Err("Drop a project folder or ZIP archive.".into());
    }
    let source = path.canonicalize().map_err(err)?;
    if scan::candidates(&source)?.is_empty() {
        return Err("No LaTeX or Typst entry file found.".into());
    }
    let prospective = resolve_destination(parent)?;
    if source.parent() == Some(prospective.as_path()) {
        return Ok(source);
    }
    copy_folder(
        &source,
        parent,
        &source
            .file_name()
            .ok_or("Choose a named project folder")?
            .to_string_lossy(),
    )
}

fn resolve_destination(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path.canonicalize().map_err(err);
    }
    Ok(
        resolve_destination(path.parent().ok_or("Invalid destination")?)?
            .join(path.file_name().ok_or("Invalid destination")?),
    )
}

pub fn copy_folder(source: &Path, parent: &Path, name: &str) -> Result<PathBuf> {
    let source = source.canonicalize().map_err(err)?;
    if resolve_destination(parent)?.starts_with(&source) {
        return Err("The destination cannot be inside the imported folder.".into());
    }
    fs::create_dir_all(parent).map_err(err)?;
    let parent = parent.canonicalize().map_err(err)?;
    let temp = tempfile::tempdir_in(&parent).map_err(err)?;
    let mut size = 0u64;
    // Unlike compiler snapshots, an imported folder preserves hidden files and Git data.
    for (index, entry) in walkdir::WalkDir::new(&source)
        .min_depth(1)
        .into_iter()
        .enumerate()
    {
        let entry = entry.map_err(err)?;
        if index >= 30_000 {
            return Err("Project exceeds 30,000 entries.".into());
        }
        let kind = entry.file_type();
        if kind.is_symlink() || (!kind.is_dir() && !kind.is_file()) {
            return Err("Folder imports cannot contain symbolic links or special files. Use Open Project to open this folder in place.".into());
        }
        let target = temp
            .path()
            .join(entry.path().strip_prefix(&source).map_err(err)?);
        if kind.is_dir() {
            fs::create_dir(&target).map_err(err)?;
        } else {
            size = size
                .checked_add(entry.metadata().map_err(err)?.len())
                .ok_or("Project size overflow")?;
            if size > 1024 * 1024 * 1024 {
                return Err("Project exceeds 1 GB.".into());
            }
            fs::copy(entry.path(), target).map_err(err)?;
        }
    }
    publish(temp.path(), &parent, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn creation_uses_saved_location_and_per_project_override_without_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let configured = temp.path().join("saved location");
        let prefs = json!({"projectDirectory": configured});
        assert_eq!(
            directory(&Value::Null, "").unwrap(),
            default_directory().unwrap()
        );
        let mut input = json!({"name":"Test", "format":"typst", "text":"$ x $"});
        let root = create(&prefs, &input).unwrap();
        assert_eq!(root, configured.join("Test"));
        assert!(create(&prefs, &input).is_err());
        assert_eq!(fs::read_to_string(root.join("main.typ")).unwrap(), "$ x $");
        input["directory"] = json!(temp.path().join("override"));
        assert_eq!(
            create(&prefs, &input).unwrap(),
            temp.path().join("override/Test")
        );
        assert!(directory(&json!({"projectDirectory":"relative"}), "").is_err());
    }

    #[test]
    fn folder_import_preserves_original_hidden_files_and_existing_names() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Demo");
        fs::create_dir_all(source.join("sections")).unwrap();
        fs::create_dir_all(source.join(".git")).unwrap();
        let original =
            "\\documentclass{article}\n\\begin{document}\\input{sections/model}\\end{document}";
        fs::write(source.join("main.tex"), original).unwrap();
        fs::write(source.join("sections/model.tex"), "$x$").unwrap();
        fs::write(source.join(".git/HEAD"), "test").unwrap();
        let parent = temp.path().join("Destination");
        let copied = import(&source, &parent).unwrap();
        let parent = parent.canonicalize().unwrap();
        assert_eq!(copied, parent.join("Demo"));
        assert_eq!(fs::read(copied.join(".git/HEAD")).unwrap(), b"test");
        fs::write(
            copied.join("main.tex"),
            "\\documentclass{article}\n\\begin{document}changed copy\\end{document}",
        )
        .unwrap();
        assert_eq!(
            fs::read(source.join("main.tex")).unwrap(),
            original.as_bytes()
        );
        assert_eq!(import(&source, &parent).unwrap(), parent.join("Demo (1)"));
        assert_eq!(import(&copied, &parent).unwrap(), copied);
        assert!(import(&source, &source.join("inside")).is_err());
    }

    #[test]
    fn failed_folder_import_leaves_no_partial_project() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("Demo");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("main.typ"), "$ x $").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("main.typ", source.join("linked.typ")).unwrap();
        let parent = temp.path().join("Destination");
        #[cfg(unix)]
        assert!(import(&source, &parent)
            .unwrap_err()
            .contains("symbolic links"));
        assert_eq!(fs::read_dir(parent).unwrap().count(), 0);
    }

    #[test]
    fn zip_import_handles_nested_root_names_and_rejects_unsafe_or_empty_archives() {
        use std::io::Write;
        let temp = tempfile::tempdir().unwrap();
        let parent = temp.path().join("Projects");
        let archive = temp.path().join("Demo.ZIP");
        let write_zip = |name: &str| {
            let mut zip = zip::ZipWriter::new(fs::File::create(&archive).unwrap());
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"$x$").unwrap();
            zip.finish().unwrap();
        };
        write_zip("Demo/main.typ");
        let root = import(&archive, &parent).unwrap();
        assert_eq!(root, parent.join("Demo"));
        assert!(root.join("main.typ").is_file());
        assert_eq!(import(&archive, &parent).unwrap(), parent.join("Demo (1)"));
        write_zip("../outside.typ");
        assert!(import(&archive, &parent).is_err());
        write_zip("readme.txt");
        assert!(import(&archive, &parent).is_err());
        assert_eq!(fs::read_dir(&parent).unwrap().count(), 2);
        assert!(archive.is_file());
    }
}
