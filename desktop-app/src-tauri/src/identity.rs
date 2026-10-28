//! Public bundle identity changes once; storage and resident coordination do not.
#[cfg(target_os = "macos")]
use crate::files::err;
use crate::files::Result;
use std::path::PathBuf;
use tauri::utils::config::{AppDirectoriesOverride, AppDirectoryOverrides};

pub const LEGACY_ID: &str = "local.slidemath.prototype";

pub fn configure(context: &mut tauri::Context<tauri::Wry>) -> Result<()> {
    let isolated = std::env::var_os("MATHLIDELUX_DATA_DIR").map(PathBuf::from);
    // Tauri uses this for every Windows/Linux WebView, including dynamic Instant windows.
    let local_data = match &isolated {
        Some(root) => root.join("WebView"),
        None => dirs::data_local_dir()
            .ok_or("Local application data directory unavailable")?
            .join(LEGACY_ID),
    };
    context.config_mut().app.app_directories_override =
        Some(AppDirectoriesOverride::Directories(AppDirectoryOverrides {
            local_data: Some(local_data),
            ..Default::default()
        }));
    #[cfg(target_os = "macos")]
    if let Some(root) = isolated {
        // WKWebView ignores dataDirectory. A separate named store keeps QA from
        // reading or modifying a user's pending Instant draft and color history.
        for window in &mut context.config_mut().app.windows {
            window.data_store_identifier = Some(store_id(&root));
        }
    } else {
        let parent = dirs::home_dir()
            .ok_or("Home directory unavailable")?
            .join("Library/WebKit");
        preserve_webkit(&parent, &context.config().identifier)?;
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn store_id(root: &std::path::Path) -> [u8; 16] {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(root.as_os_str().as_encoded_bytes());
    digest[..16].try_into().unwrap()
}

#[cfg(target_os = "macos")]
pub fn isolated_store_id() -> Option<[u8; 16]> {
    std::env::var_os("MATHLIDELUX_DATA_DIR").map(|root| store_id(&PathBuf::from(root)))
}

#[cfg(target_os = "macos")]
fn preserve_webkit(parent: &std::path::Path, identifier: &str) -> Result<()> {
    use std::{fs, os::unix::fs::symlink};
    let legacy = parent.join(LEGACY_ID);
    let current = parent.join(identifier);
    fs::create_dir_all(parent).map_err(err)?;
    match fs::symlink_metadata(&legacy) {
        Ok(metadata) if metadata.is_dir() && !metadata.is_symlink() => {}
        Ok(_) => {
            return Err(format!(
                "Expected a real WebKit directory: {}",
                legacy.display()
            ))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(&legacy).map_err(err)?;
        }
        Err(error) => return Err(err(error)),
    }
    let legacy = legacy.canonicalize().map_err(err)?;
    // Keep the database and its journal together, without copying a possibly
    // checkpointing SQLite store. The resident lock is acquired before this.
    // Older previews retain the same storage, including after a rollback.
    match fs::symlink_metadata(&current) {
        Ok(metadata) if metadata.is_symlink() && fs::read_link(&current).map_err(err)? == legacy => Ok(()),
        Ok(_) => Err(format!(
            "WebKit migration conflict at {}. Both stores were preserved; resolve this before opening FerMatter.",
            current.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => symlink(&legacy, &current).map_err(err),
        Err(error) => Err(err(error)),
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::fs;
    const CURRENT: &str = "io.github.fermatter.fermatter";

    #[test]
    fn webkit_preserves_pending_drafts_and_rollback_without_copying() {
        let root = tempfile::tempdir().unwrap();
        let old = root.path().join(LEGACY_ID);
        fs::create_dir(&old).unwrap();
        fs::write(old.join("draft"), "pending synthetic draft").unwrap();
        preserve_webkit(root.path(), CURRENT).unwrap();
        preserve_webkit(root.path(), CURRENT).unwrap();
        let new = root.path().join(CURRENT);
        assert_eq!(
            fs::read_to_string(new.join("draft")).unwrap(),
            "pending synthetic draft"
        );
        fs::write(new.join("draft"), "updated draft").unwrap();
        assert_eq!(
            fs::read_to_string(old.join("draft")).unwrap(),
            "updated draft"
        );
    }

    #[test]
    fn webkit_refuses_conflicting_stores_and_links_without_overwriting() {
        let root = tempfile::tempdir().unwrap();
        let new = root.path().join(CURRENT);
        fs::create_dir(&new).unwrap();
        fs::write(new.join("draft"), "new draft").unwrap();
        assert!(preserve_webkit(root.path(), CURRENT).is_err());
        assert_eq!(fs::read_to_string(new.join("draft")).unwrap(), "new draft");
        fs::remove_file(new.join("draft")).unwrap();
        fs::remove_dir(&new).unwrap();
        std::os::unix::fs::symlink(root.path(), &new).unwrap();
        assert!(preserve_webkit(root.path(), CURRENT).is_err());
        assert_eq!(fs::read_link(&new).unwrap(), root.path());
    }

    #[test]
    fn isolated_webviews_share_a_stable_store_only_with_the_same_qa_root() {
        assert_eq!(
            store_id(std::path::Path::new("/qa/a")),
            store_id(std::path::Path::new("/qa/a"))
        );
        assert_ne!(
            store_id(std::path::Path::new("/qa/a")),
            store_id(std::path::Path::new("/qa/b"))
        );
    }
}
