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
}
pub fn show_projects(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
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
    let runtime = app.state::<Arc<Runtime>>();
    if project_setup::required(&runtime.store)? {
        show_projects(app);
        return Err("Choose a project folder in the main window first.".into());
    }
    instant::open(app, None).map_err(err)
}
pub fn import_svg(app: &tauri::AppHandle, sources: &[String]) -> Result<()> {
    if sources.is_empty() || sources.len() > 16 {
        return Err("Open between 1 and 16 SVG files.".into());
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
    let state = app.state::<Resident>();
    for (project, draft) in restored {
        let ident = project.as_ref().map(|p| string(p, "id")).unwrap_or("");
        let label = if ident.is_empty() {
            "instant".to_owned()
        } else {
            format!("instant-{ident}")
        };
        let token = uuid::Uuid::new_v4().to_string();
        {
            let mut pending = state.pending.lock().unwrap();
            if pending.values().map(VecDeque::len).sum::<usize>() >= 32 {
                return Err("Finish the pending SVG imports first.".into());
            }
            pending
                .entry(label.clone())
                .or_default()
                .push_back(json!({"token":token,"draft":draft}));
        }
        if let Err(error) = instant::open(app, project.as_ref()) {
            let mut pending = state.pending.lock().unwrap();
            if let Some(queue) = pending.get_mut(&label) {
                queue.retain(|v| v["token"] != token);
            }
            return Err(err(error));
        }
        let _ = app.emit_to(&label, "instant-import-pending", ());
    }
    Ok(())
}
pub fn incoming(app: &tauri::AppHandle, label: &str, token: &str) -> Result<Value> {
    if !label.starts_with("instant") {
        return Err("Instant window required.".into());
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
        let result = paths
            .iter()
            .map(|p| roundtrip::read_svg(p))
            .collect::<Result<Vec<_>>>()
            .and_then(|sources| import_svg(&app, &sources));
        if let Err(error) = result {
            report(&app, error);
        }
    });
}
pub fn from_clipboard(app: &tauri::AppHandle) -> Result<()> {
    // AppKit clipboard access is dispatched to the main thread by all callers.
    import_svg(app, &native_import::clipboard()?)
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
    let next = shortcuts(value)?;
    let state = app.state::<Resident>();
    let mut current = state.keys.lock().unwrap();
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
    for (key, _) in current.iter() {
        if !next.iter().any(|(new, _)| new == key) {
            app.global_shortcut().unregister(*key).map_err(err)?;
        }
    }
    *current = next;
    Ok(())
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
