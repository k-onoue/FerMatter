//! Platform entry points share a bounded, acknowledged Instant import queue.
use crate::{files::*, instant, native_import, project_setup, roundtrip, Runtime};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

#[derive(Default)]
pub struct Resident {
    pending: Mutex<HashMap<String, VecDeque<Value>>>,
    keys: Mutex<Vec<(Shortcut, String)>>,
    workspace: Mutex<std::collections::HashSet<String>>,
}
pub fn show_projects(app: &tauri::AppHandle) {
    let _ = crate::lifecycle::on_main(app, |app| {
        if crate::lifecycle::quitting(&app) {
            return Ok(());
        }
        crate::lifecycle::cancel_close(&app, "main");
        if let Some(window) = app.get_webview_window("main") {
            window.show().map_err(err)?;
            window.unminimize().map_err(err)?;
            window.set_focus().map_err(err)?;
        }
        Ok(())
    });
}
pub fn report(app: &tauri::AppHandle, error: String) {
    use tauri_plugin_dialog::DialogExt;
    app.dialog()
        .message(error)
        .title("Mathlidelux")
        .kind(tauri_plugin_dialog::MessageDialogKind::Error)
        .show(|_| {});
}
pub fn open_instant(app: &tauri::AppHandle) -> Result<()> {
    if crate::lifecycle::quitting(app) {
        return Err("Finish saving before opening another editor.".into());
    }
    let runtime = app.state::<Arc<Runtime>>();
    if project_setup::required(&runtime.store)? {
        show_projects(app);
        return Err("Choose a project folder in the main window first.".into());
    }
    instant::open(app, None).map_err(err)
}
pub fn import_svg(app: &tauri::AppHandle, sources: &[String]) -> Result<()> {
    import_svg_to(app, sources, "")
}
pub fn import_svg_to(app: &tauri::AppHandle, sources: &[String], target: &str) -> Result<()> {
    if crate::lifecycle::quitting(app) {
        return Err("Finish saving before opening another SVG.".into());
    }
    if sources.is_empty() || sources.len() > 16 {
        return Err("Open between 1 and 16 SVG files.".into());
    }
    if sources.iter().map(String::len).sum::<usize>() > 64 * 1024 * 1024 {
        return Err("SVG selection exceeds 64 MB.".into());
    }
    let runtime = app.state::<Arc<Runtime>>();
    if project_setup::required(&runtime.store)? {
        show_projects(app);
        return Err(
            "Choose a project folder in the main window first, then open the SVG again.".into(),
        );
    }
    // Validate every input before touching an existing draft or opening windows.
    let restored = sources
        .iter()
        .map(|s| roundtrip::decode(s).and_then(|v| roundtrip::resolve(&runtime.store, &v)))
        .collect::<Result<Vec<_>>>()?;
    let target = target.to_owned();
    crate::lifecycle::on_main(app, move |app| admit_imports(&app, restored, &target))
}
fn destination(project: &str, target: &str, workspace: bool, instant: bool) -> String {
    if project.is_empty() {
        return "instant".into();
    }
    let label = format!("instant-{project}");
    if target == label || (!workspace && instant) {
        return label;
    }
    if target == "workspace" || workspace {
        return "main".into();
    }
    label
}
pub fn workspace(app: &tauri::AppHandle, label: &str, ids: &Value) -> Result<()> {
    if label != "main" {
        return Err("Main workspace required.".into());
    }
    let ids = ids.as_array().ok_or("Expected workspace project IDs.")?;
    if ids.len() > 1000 {
        return Err("Too many workspace tabs.".into());
    }
    let runtime = app.state::<Arc<Runtime>>();
    let mut projects = std::collections::HashSet::new();
    for id in ids {
        let id = id.as_str().ok_or("Invalid workspace project ID.")?;
        runtime.store.root(id)?;
        projects.insert(id.to_owned());
    }
    *app.state::<Resident>().workspace.lock().unwrap() = projects;
    Ok(())
}
fn admit_imports(
    app: &tauri::AppHandle,
    restored: Vec<(Option<Value>, Value)>,
    target: &str,
) -> Result<()> {
    // Queue admission and window creation must be ordered with native Quit's
    // window snapshot; an editor created afterward could escape its save barrier.
    if crate::lifecycle::quitting(app) {
        return Err("Finish saving before opening another SVG.".into());
    }
    let state = app.state::<Resident>();
    let mut queued = Vec::new();
    let mut pending = state.pending.lock().unwrap();
    if pending.values().map(VecDeque::len).sum::<usize>() + restored.len() > 32 {
        return Err("Finish the pending SVG imports first.".into());
    }
    for (project, draft) in restored {
        let ident = project.as_ref().map(|p| string(p, "id")).unwrap_or("");
        let label = destination(
            ident,
            target,
            state.workspace.lock().unwrap().contains(ident),
            app.get_webview_window(&format!("instant-{ident}"))
                .is_some(),
        );
        let token = uuid::Uuid::new_v4().to_string();
        pending
            .entry(label.clone())
            .or_default()
            .push_back(json!({"token":token,"project":project,"draft":draft}));
        queued.push((project, label, token));
    }
    drop(pending);
    let mut failure = None;
    for (project, label, token) in queued {
        let opened = if label == "main" {
            show_projects(app);
            Ok(())
        } else {
            instant::open(app, project.as_ref())
        };
        if let Err(error) = opened {
            let mut pending = state.pending.lock().unwrap();
            if let Some(queue) = pending.get_mut(&label) {
                queue.retain(|v| v["token"] != token);
            }
            failure = Some(err(error));
            continue;
        }
        let event = if label == "main" {
            "workspace-import-pending"
        } else {
            "instant-import-pending"
        };
        let _ = app.emit_to(&label, event, ());
    }
    failure.map_or(Ok(()), Err)
}
pub fn incoming(app: &tauri::AppHandle, label: &str, token: &str) -> Result<Value> {
    if label != "main" && label != "instant" && !label.starts_with("instant-") {
        return Err("Editor window required.".into());
    }
    let state = app.state::<Resident>();
    let mut pending = state.pending.lock().unwrap();
    let queue = pending.entry(label.into()).or_default();
    if !token.is_empty() {
        if queue.front().is_none_or(|v| v["token"] != token) {
            return Err("SVG import changed; try again.".into());
        }
        queue.pop_front();
    }
    Ok(queue.front().cloned().unwrap_or(Value::Null))
}
pub fn open_files(app: &tauri::AppHandle, paths: Vec<PathBuf>) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if paths.is_empty() || paths.len() > 16 {
            report(&app, "Open between 1 and 16 SVG files.".into());
            return;
        }
        let result = (|| {
            let mut sources = Vec::new();
            let mut bytes = 0;
            for path in paths {
                let source = roundtrip::read_svg(&path)?;
                bytes += source.len();
                if bytes > 64 * 1024 * 1024 {
                    return Err("SVG selection exceeds 64 MB.".into());
                }
                sources.push(source);
            }
            import_svg(&app, &sources)
        })();
        if let Err(error) = result {
            report(&app, error);
        }
    });
}
pub fn from_clipboard(app: &tauri::AppHandle) -> Result<()> {
    from_clipboard_to(app, "")
}
pub fn from_clipboard_to(app: &tauri::AppHandle, target: &str) -> Result<()> {
    // AppKit clipboard access is dispatched to the main thread by all callers.
    import_svg_to(app, &native_import::clipboard()?, target)
}
fn shortcuts(value: &Value) -> Result<Vec<(Shortcut, String)>> {
    let mut keys = vec![];
    for (id, key) in [("globalInstant", "n"), ("globalInstantClipboard", "v")] {
        let fallback = json!({"key":key,"command":true,"control":true,"option":true,"shift":false});
        let binding = value["bindings"].get(id).unwrap_or(&fallback);
        if string(binding, "key").is_empty() {
            continue;
        }
        if value["bindings"].get(id).is_none()
            && value["bindings"]
                .as_object()
                .is_some_and(|all| all.values().any(|other| other == &fallback))
        {
            continue;
        }
        let mut parts = vec![];
        for (field, name) in [
            ("command", "Super"),
            ("control", "Control"),
            ("option", "Alt"),
            ("shift", "Shift"),
        ] {
            if binding[field] == true {
                parts.push(name.to_owned());
            }
        }
        parts.push(match string(binding, "key") {
            "\t" => "Tab".into(),
            "\u{f700}" => "ArrowUp".into(),
            "\u{f701}" => "ArrowDown".into(),
            "\u{f702}" => "ArrowLeft".into(),
            "\u{f703}" => "ArrowRight".into(),
            other => other.to_uppercase(),
        });
        let shortcut = parts.join("+").parse::<Shortcut>().map_err(err)?;
        keys.push((shortcut, id.into()));
    }
    Ok(keys)
}
pub fn apply_keys(app: &tauri::AppHandle, value: &Value) -> Result<()> {
    set_keys(app, shortcuts(value)?)
}
fn set_keys(app: &tauri::AppHandle, next: Vec<(Shortcut, String)>) -> Result<()> {
    let state = app.state::<Resident>();
    // Native registration can wait for the main loop. Do not hold the lock that
    // the main-loop hotkey callback needs while waiting for it.
    let current = state.keys.lock().unwrap().clone();
    let mut added = vec![];
    for (key, _) in &next {
        if current.iter().any(|(old, _)| old == key) {
            continue;
        }
        if let Err(error) = app.global_shortcut().register(*key) {
            for key in added {
                let _ = app.global_shortcut().unregister(key);
            }
            return Err(format!(
                "Global shortcut is unavailable: {error}. Choose another shortcut in Settings."
            ));
        }
        added.push(*key);
    }
    let mut removed = vec![];
    for (key, _) in current.iter() {
        if !next.iter().any(|(new, _)| new == key) {
            if let Err(error) = app.global_shortcut().unregister(*key) {
                for key in removed {
                    let _ = app.global_shortcut().register(key);
                }
                for key in added {
                    let _ = app.global_shortcut().unregister(key);
                }
                return Err(err(error));
            }
            removed.push(*key);
        }
    }
    *state.keys.lock().unwrap() = next;
    Ok(())
}
pub fn save_keys(
    app: &tauri::AppHandle,
    settings: &crate::settings::SettingsFiles,
    value: &Value,
    revision: &str,
) -> Result<String> {
    crate::settings::validate("keybindings.json", value)?;
    let snapshot = settings.snapshot("keybindings.json", true)?;
    if string(&snapshot, "revision") != revision {
        return Err("Settings changed on disk. Reload the file before saving.".into());
    }
    let before = app.state::<Resident>().keys.lock().unwrap().clone();
    apply_keys(app, value)?;
    match settings.save("keybindings.json", value, revision) {
        Ok(revision) => Ok(revision),
        Err(error) => {
            if let Err(rollback) = set_keys(app, before) {
                return Err(format!("{error}\nCould not restore shortcuts: {rollback}"));
            }
            Err(error)
        }
    }
}
pub fn hotkey(
    app: &tauri::AppHandle,
    shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state() != ShortcutState::Pressed {
        return;
    }
    let command = app
        .state::<Resident>()
        .keys
        .lock()
        .unwrap()
        .iter()
        .find(|(key, _)| key == shortcut)
        .map(|(_, name)| name.clone());
    if command.is_none() {
        return;
    }
    let app = app.clone();
    let cloned = app.clone();
    let _ = app.run_on_main_thread(move || {
        let result = if command.as_deref() == Some("globalInstantClipboard") {
            from_clipboard(&cloned)
        } else {
            open_instant(&cloned)
        };
        if let Err(error) = result {
            report(&cloned, error);
        }
    });
}

pub fn install(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::TrayIconBuilder,
    };
    let instant = MenuItem::with_id(app, "resident-instant", "Instant", true, None::<&str>)?;
    let clipboard = MenuItem::with_id(
        app,
        "resident-clipboard",
        "Edit from Clipboard",
        true,
        None::<&str>,
    )?;
    let open = MenuItem::with_id(app, "resident-open", "Open SVG…", true, None::<&str>)?;
    let projects = MenuItem::with_id(app, "resident-projects", "Projects", true, None::<&str>)?;
    let quit = PredefinedMenuItem::quit(app, Some("Quit Mathlidelux"))?;
    let separator = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(
        app,
        &[&instant, &clipboard, &open, &projects, &separator, &quit],
    )?;
    TrayIconBuilder::with_id("instant")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Mathlidelux")
        .menu(&menu)
        .on_menu_event(|app, event| {
            let result = match event.id.as_ref() {
                "resident-instant" => open_instant(app),
                "resident-clipboard" => from_clipboard(app),
                "resident-open" => {
                    use tauri_plugin_dialog::DialogExt;
                    let handle = app.clone();
                    app.dialog()
                        .file()
                        .add_filter("SVG", &["svg"])
                        .pick_files(move |files| {
                            if let Some(files) = files {
                                open_files(
                                    &handle,
                                    files
                                        .into_iter()
                                        .filter_map(|f| f.into_path().ok())
                                        .collect(),
                                );
                            }
                        });
                    Ok(())
                }
                "resident-projects" => {
                    show_projects(app);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(error) = result {
                report(app, error);
            }
        })
        .build(app)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imported_svg_reuses_matching_windows_without_crossing_project_scopes() {
        assert_eq!(destination("", "workspace", false, false), "instant");
        assert_eq!(destination("p", "workspace", false, false), "main");
        assert_eq!(destination("p", "workspace", false, true), "instant-p");
        assert_eq!(destination("p", "", true, true), "main");
        assert_eq!(destination("p", "instant-p", true, true), "instant-p");
        assert_eq!(destination("q", "instant-p", false, false), "instant-q");
    }
    #[test]
    fn shortcut_overrides_disable_and_legacy_conflicts_match_editor_settings() {
        let mut value = json!({"schemaVersion":1,"kind":"mathlidelux.keybindings","bindings":{}});
        assert_eq!(shortcuts(&value).unwrap().len(), 2);
        value["bindings"]["globalInstant"] =
            json!({"key":"","command":true,"control":true,"option":true,"shift":false});
        assert_eq!(shortcuts(&value).unwrap().len(), 1);
        value["bindings"]
            .as_object_mut()
            .unwrap()
            .remove("globalInstant");
        value["bindings"]["open"] =
            json!({"key":"n","command":true,"control":true,"option":true,"shift":false});
        let keys = shortcuts(&value).unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].1, "globalInstantClipboard");
    }
}
