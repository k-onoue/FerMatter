//! Local launch history controls the running application's Dock artwork.
use crate::files::{self, Result};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    sync::{
        atomic::{AtomicU64, AtomicU8, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::Manager;

const WEEK: u64 = 7 * 24 * 60 * 60;
const LIMIT: usize = 128;
static ICON_REVISION: AtomicU64 = AtomicU64::new(0);
static APPLIED_RANK: AtomicU8 = AtomicU8::new(0);

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct History {
    schema_version: u8,
    openings: Vec<u64>,
}

fn rank(count: usize) -> u8 {
    match count {
        14.. => 1,
        7..=13 => 2,
        3..=6 => 3,
        _ => 4,
    }
}

fn record(home: &Path, now: u64, opening: bool) -> Result<u8> {
    let path = home.join("IconUsage.json");
    let mut history = match path.symlink_metadata() {
        Ok(_) => serde_json::from_slice::<History>(&files::read_bounded(&path, 16 * 1024)?)
            .map_err(|_| "Cannot read icon usage history; the original was preserved.")?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => History {
            schema_version: 1,
            openings: vec![],
        },
        Err(e) => return Err(files::err(e)),
    };
    if history.schema_version != 1
        || history.openings.len() > LIMIT
        || history.openings.windows(2).any(|pair| pair[0] > pair[1])
    {
        return Err("Unsupported icon usage history; the original was preserved.".into());
    }
    // Preserve history if the system clock moved backwards. Future entries
    // must neither inflate the count nor cause an additional opening.
    if !opening || history.openings.last().is_some_and(|last| *last > now) {
        return Ok(rank(
            history
                .openings
                .iter()
                .filter(|t| **t <= now && now - **t < WEEK)
                .count(),
        ));
    }
    history.openings.retain(|t| now - *t < WEEK);
    // Ready/Reopen and a forwarded single-instance launch can describe the
    // same opening. Coalesce only duplicate native events within two seconds.
    if history.openings.last().is_none_or(|last| now - *last >= 2) {
        history.openings.push(now);
    }
    if history.openings.len() > LIMIT {
        history.openings.drain(..history.openings.len() - LIMIT);
    }
    files::atomic(&path, &serde_json::to_vec(&history).map_err(files::err)?)?;
    Ok(rank(history.openings.len()))
}

pub fn opened(app: &tauri::AppHandle) {
    update(app, true);
}

pub fn refresh(app: &tauri::AppHandle) {
    update(app, false);
}

fn selected_rank(selection: &str, automatic: u8) -> u8 {
    match selection {
        "detailed" => 1,
        "standard" => 2,
        "smile" => 3,
        "frown" => 4,
        _ => automatic,
    }
}

fn update(app: &tauri::AppHandle, opening: bool) {
    let Some(runtime) = app.try_state::<Arc<crate::Runtime>>() else {
        return;
    };
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return;
    };
    let (rank, revision) = {
        let _lock = runtime.store.writes.lock().unwrap();
        let automatic = match record(&runtime.store.home, now.as_secs(), opening) {
            Ok(rank) => rank,
            Err(error) => {
                log::warn!("Dock icon usage unavailable: {error}");
                2
            }
        };
        let preferences = runtime
            .settings
            .load("preferences.json")
            .unwrap_or_default();
        (
            selected_rank(files::string(&preferences, "dockIcon"), automatic),
            ICON_REVISION.fetch_add(1, Ordering::SeqCst).wrapping_add(1),
        )
    };
    if APPLIED_RANK.load(Ordering::SeqCst) == rank {
        return;
    }
    let bytes: &'static [u8] = match rank {
        1 => include_bytes!("../icons/usage-1.png"),
        2 => include_bytes!("../icons/icon.png"),
        3 => include_bytes!("../icons/usage-3.png"),
        _ => include_bytes!("../icons/usage-4.png"),
    };
    let _ = app.run_on_main_thread(move || {
        if ICON_REVISION.load(Ordering::SeqCst) != revision {
            return;
        }
        match crate::native::set_application_icon(bytes) {
            Ok(()) => APPLIED_RANK.store(rank, Ordering::SeqCst),
            Err(error) => log::warn!("Dock icon unchanged: {error}"),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn manual_preference_persists_and_refresh_does_not_count_an_opening() {
        let home = tempfile::tempdir().unwrap();
        let settings = crate::settings::SettingsFiles::new(home.path());
        let snapshot = settings.snapshot("preferences.json", false).unwrap();
        let mut preferences = snapshot["value"].clone();
        preferences["dockIcon"] = serde_json::json!("smile");
        let revision = settings
            .save(
                "preferences.json",
                &preferences,
                files::string(&snapshot, "revision"),
            )
            .unwrap();
        let loaded = settings.load("preferences.json").unwrap();
        assert_eq!(selected_rank(files::string(&loaded, "dockIcon"), 1), 3);
        assert_eq!(selected_rank(files::string(&loaded, "dockIcon"), 4), 3);
        record(home.path(), WEEK * 2, true).unwrap();
        let path = home.path().join("IconUsage.json");
        let before = std::fs::read(&path).unwrap();
        let automatic = record(home.path(), WEEK * 2 + 100, false).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        preferences["dockIcon"] = serde_json::json!("automatic");
        settings
            .save("preferences.json", &preferences, &revision)
            .unwrap();
        assert_eq!(
            selected_rank(
                files::string(&settings.load("preferences.json").unwrap(), "dockIcon"),
                automatic
            ),
            4
        );
    }
    #[test]
    fn launches_cross_all_thresholds_and_expire_after_seven_days() {
        let home = tempfile::tempdir().unwrap();
        let start = WEEK * 2;
        for (n, expected) in [4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 1]
            .into_iter()
            .enumerate()
        {
            let level = record(home.path(), start + n as u64 * 10, true).unwrap();
            assert_eq!(level, expected);
        }
        assert_eq!(record(home.path(), start + WEEK + 130, true).unwrap(), 4);
        let state: History =
            serde_json::from_slice(&std::fs::read(home.path().join("IconUsage.json")).unwrap())
                .unwrap();
        assert_eq!(state.openings, vec![start + WEEK + 130]);
    }
    #[test]
    fn duplicate_events_and_clock_rollback_do_not_inflate_launches() {
        let home = tempfile::tempdir().unwrap();
        let start = WEEK * 2;
        record(home.path(), start, true).unwrap();
        record(home.path(), start + 1, true).unwrap();
        let path = home.path().join("IconUsage.json");
        let before = std::fs::read(&path).unwrap();
        record(home.path(), start - 60, true).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), before);
        record(home.path(), start + 2, true).unwrap();
        let state: History = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(state.openings.len(), 2);
    }
    #[test]
    fn invalid_and_future_schema_history_is_never_replaced() {
        for bytes in [
            b"broken".as_slice(),
            br#"{"schemaVersion":2,"openings":[]}"#,
        ] {
            let home = tempfile::tempdir().unwrap();
            let path = home.path().join("IconUsage.json");
            std::fs::write(&path, bytes).unwrap();
            assert!(record(home.path(), WEEK, true).is_err());
            assert_eq!(std::fs::read(path).unwrap(), bytes);
        }
    }
}
