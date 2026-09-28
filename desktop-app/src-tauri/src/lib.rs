mod compiler;
mod export;
mod files;
mod native;
mod scan;
mod settings;
mod storage;
pub mod verification;
#[cfg(target_os = "macos")]
mod wallpaper;
use files::*;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use tauri::{Emitter, Manager};
struct Runtime {
    store: storage::Storage,
    settings: settings::SettingsFiles,
    jobs: Mutex<HashMap<String, compiler::Cancellation>>,
}
fn job(runtime: &Runtime, key: &str) -> compiler::Cancellation {
    let c = compiler::Cancellation::default();
    if let Some(old) = runtime.jobs.lock().unwrap().insert(key.into(), c.clone()) {
        old.cancel()
    }
    c
}
#[tauri::command]
async fn request(
    app: tauri::AppHandle,
    state: tauri::State<'_, Arc<Runtime>>,
    operation: String,
    payload: Value,
) -> Result<Value> {
    let runtime = state.inner().clone();
    // Reserve ownership before queuing blocking work. A delayed old request must
    // never replace a newer compiler session after its cancellation was received.
    if operation == "cancel" {
        if let Some(old) = runtime.jobs.lock().unwrap().remove(string(&payload, "key")) {
            old.cancel();
        }
        return Ok(Value::Null);
    }
    let cancellation = if matches!(
        operation.as_str(),
        "scan" | "render" | "source_preview" | "document" | "pdf_navigate"
    ) {
        Some(job(&runtime, string(&payload, "key")))
    } else {
        None
    };
    tauri::async_runtime::spawn_blocking(move || {
        dispatch(&app, &runtime, &operation, payload, cancellation)
    })
    .await
    .map_err(err)?
}
fn dispatch(
    app: &tauri::AppHandle,
    runtime: &Runtime,
    op: &str,
    v: Value,
    cancellation: Option<compiler::Cancellation>,
) -> Result<Value> {
    let store = &runtime.store;
    let ident = string(&v, "project");
    match op {
        "desktop_background" => {
            #[cfg(target_os = "macos")]
            return wallpaper::background(app, string(&v, "key"));
            #[cfg(not(target_os = "macos"))]
            Ok(Value::Null)
        }
        "startup" => {
            let templates = store.home.join("templates.json");
            if !templates.exists() && store.legacy.join("templates.json").is_file() {
                fs::copy(store.legacy.join("templates.json"), &templates).map_err(err)?;
            }
            let settings = store.home.join("Settings");
            for file in ["theme.json", "keybindings.json"] {
                let dest = settings.join(file);
                let legacy = store.legacy.join("Settings").join(file);
                if !dest.exists() && legacy.exists() {
                    fs::copy(legacy, dest).map_err(err)?;
                }
            }
            let themes_path = app.path().resource_dir().map_err(err)?.join("themes");
            let themes_path = if themes_path.is_dir() {
                themes_path
            } else {
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("../../macos-app/Sources/SlideMath/Themes")
            };
            let themes: Vec<_> = fs::read_dir(themes_path)
                .map_err(err)?
                .flatten()
                .filter(|p| p.file_name().to_string_lossy().ends_with(".theme.json"))
                .filter_map(|p| json_read(&p.path()).ok())
                .collect();
            Ok(
                json!({"projects":store.recent()?,"settings":runtime.settings.load("preferences.json")?,"theme":runtime.settings.load("theme.json")?,"keybindings":runtime.settings.load("keybindings.json")?,"themes":themes,"templates":json_read(&store.home.join("templates.json")).unwrap_or(json!({})),"storage":store.home}),
            )
        }
        "open_project" => {
            let mut path = PathBuf::from(string(&v, "path"));
            if path.extension().is_some_and(|e| e == "zip") {
                path = storage::import_zip(
                    &path,
                    &dirs::document_dir()
                        .ok_or("Documents folder is unavailable")?
                        .join("Mathlidelux"),
                )?
            }
            let (root, main) = if path.is_file() {
                (
                    path.parent().unwrap().to_owned(),
                    Some(path.file_name().unwrap().to_string_lossy().into_owned()),
                )
            } else {
                (path, None)
            };
            let main = v["main"].as_str().map(str::to_owned).or(main);
            store.open(&root, main.as_deref())
        }
        "new_project" => {
            let name = string(&v, "name").trim();
            if name.is_empty() || name.contains(['/', ':', '\0']) || name == "." || name == ".." {
                return Err("Choose a valid project name.".into());
            }
            let parent = dirs::document_dir()
                .ok_or("Documents folder is unavailable")?
                .join("Mathlidelux");
            fs::create_dir_all(&parent).map_err(err)?;
            let target = parent.join(name);
            if target.exists() {
                return Err("A project with that name already exists.".into());
            }
            let temp = tempfile::tempdir_in(&parent).map_err(err)?;
            let typst = string(&v, "format") == "typst";
            let main = if typst { "main.typ" } else { "main.tex" };
            fs::write(temp.path().join(main), string(&v, "text")).map_err(err)?;
            if !typst {
                fs::write(temp.path().join("latexmkrc"), "$pdf_mode = 1;\n").map_err(err)?
            }
            rename_no_replace(temp.path(), &target)?;
            let mut project = store.open(&target, Some(main))?;
            if !typst {
                let engine = string(&v, "engine");
                if ["pdflatex", "xelatex", "lualatex"].contains(&engine) {
                    project["engine"] = json!(engine);
                }
                store.save(string(&project, "id"), &project)?;
            }
            Ok(project)
        }
        "save_project" => {
            let _guard = store.writes.lock().unwrap();
            store.save(ident, &v["value"])?;
            Ok(Value::Null)
        }
        "tree" => Ok(json!(entries(&store.root(ident)?)?
            .into_iter()
            .map(|(name, _, dir)| json!({"path":name,"directory":dir}))
            .collect::<Vec<_>>())),
        "read_file" => read_buffer(&store.root(ident)?, string(&v, "path")),
        "save_file" => {
            let _guard = store.writes.lock().unwrap();
            save_buffer(&store.root(ident)?, &v["buffer"])
        }
        "read_drafts" => Ok(json_read(&store.draft_path(ident)?).unwrap_or(json!({}))),
        "save_drafts" => {
            let _guard = store.writes.lock().unwrap();
            json_write(&store.draft_path(ident)?, &v["value"])?;
            Ok(Value::Null)
        }
        "file_operation" => {
            let _guard = store.writes.lock().unwrap();
            let root = store.root(ident)?;
            let action = string(&v, "action");
            let name = string(&v, "path");
            match action {
                "create_file" => {
                    let dest = destination(&root, name)?;
                    use std::io::Write;
                    fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .open(dest)
                        .map_err(err)?
                        .write_all(string(&v, "text").as_bytes())
                        .map_err(err)?;
                }
                "create_folder" => {
                    let dest = destination(&root, name)?;
                    fs::create_dir(dest).map_err(err)?
                }
                "rename" | "move" => {
                    let source = safe(&root, name)?;
                    let dest = destination(&root, string(&v, "destination"))?;
                    if dest.starts_with(&source) {
                        return Err("A folder cannot be moved inside itself.".into());
                    }
                    rename_no_replace(&source, &dest)?;
                    let mut p = store.load(ident)?;
                    let main = string(&p, "main").to_owned();
                    if main == name || main.starts_with(&format!("{name}/")) {
                        p["main"] = json!(format!(
                            "{}{}",
                            string(&v, "destination"),
                            &main[name.len()..]
                        ));
                        store.save(ident, &p)?;
                    }
                }
                "copy" => {
                    let source = safe(&root, name)?;
                    let dest = destination(&root, string(&v, "destination"))?;
                    let staging = tempfile::tempdir_in(dest.parent().ok_or("Missing parent")?)
                        .map_err(err)?;
                    let copy = staging.path().join("copy");
                    if source.is_dir() {
                        if dest.starts_with(&source) {
                            return Err("A folder cannot be copied inside itself.".into());
                        }
                        snapshot(&source, &copy, &Value::Null)?;
                    } else {
                        fs::copy(source, &copy).map_err(err)?;
                    }
                    rename_no_replace(&copy, &dest)?;
                }
                "trash" => {
                    let source = safe(&root, name)?;
                    trash::delete(source).map_err(err)?
                }
                _ => return Err("Unknown file operation".into()),
            };
            Ok(Value::Null)
        }
        "reveal" => {
            let path = if ident.is_empty() {
                store.home.join("Settings")
            } else {
                let root = store.root(ident)?;
                if string(&v, "path").is_empty() {
                    root
                } else {
                    safe(&root, string(&v, "path"))?
                }
            };
            std::process::Command::new("/usr/bin/open")
                .arg("-R")
                .arg(path)
                .spawn()
                .map_err(err)?;
            Ok(Value::Null)
        }
        "settings" => {
            let _guard = store.writes.lock().unwrap();
            runtime.settings.save(string(&v, "file"), &v["value"])?;
            Ok(Value::Null)
        }
        "reload_settings" => runtime.settings.reload(string(&v, "file")),
        "export_settings" => {
            settings::validate(string(&v, "file"), &v["value"])?;
            json_write(Path::new(string(&v, "path")), &v["value"])?;
            Ok(Value::Null)
        }
        "templates" => {
            settings::validate_templates(&v["value"])?;
            json_write(&store.home.join("templates.json"), &v["value"])?;
            Ok(Value::Null)
        }
        "read_json" => {
            let path = Path::new(string(&v, "path"));
            if fs::metadata(path).map_err(err)?.len() > 1_000_000 {
                return Err("Settings exceed 1 MB.".into());
            }
            json_read(path)
        }
        "artifact" => {
            use base64::Engine;
            let p = store.allowed_artifact(Path::new(string(&v, "path")))?;
            let bytes = fs::read(&p).map_err(err)?;
            if bytes.len() > 100 * 1024 * 1024 {
                return Err("Artifact exceeds 100 MB.".into());
            }
            let mime = match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
                "svg" => "image/svg+xml",
                "pdf" => "application/pdf",
                "png" => "image/png",
                _ => "application/octet-stream",
            };
            Ok(json!(format!(
                "data:{mime};base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )))
        }
        "export" => {
            let p = store.load(ident)?;
            let ids = v["ids"].as_array().ok_or("Choose an equation")?;
            let mut paths = vec![];
            for id in ids {
                let card = p["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"] == *id)
                    .ok_or("Equation not found")?;
                store.allowed_artifact(Path::new(string(card, "svgPath")))?;
                let size = card["exportFontSize"]
                    .as_f64()
                    .or(p["exportFontSize"].as_f64())
                    .unwrap_or(0.);
                paths.push(export::image_file(
                    card,
                    string(&v, "format"),
                    size,
                    &store.project_dir(ident)?.join("Exports"),
                )?);
            }
            if let Some(dest) = v["destination"].as_str() {
                if paths.len() != 1 {
                    return Err("Choose one image to export.".into());
                }
                fs::copy(&paths[0], dest).map_err(err)?;
            }
            Ok(json!(paths))
        }
        "copy_images" => {
            let paths = v["paths"]
                .as_array()
                .ok_or("No images selected")?
                .iter()
                .map(|p| store.allowed_artifact(Path::new(p.as_str().unwrap_or(""))))
                .collect::<Result<Vec<_>>>()?;
            let (tx, rx) = std::sync::mpsc::sync_channel(1);
            app.run_on_main_thread(move || {
                let _ = tx.send(native::clipboard(&paths));
            })
            .map_err(err)?;
            rx.recv().map_err(err)??;
            Ok(Value::Null)
        }
        "scan" | "render" | "source_preview" | "document" => {
            let root = store.root(ident)?;
            let p = store.load(ident)?;
            let main = string(&p, "main");
            let engine = string(&p, "engine");
            let key = string(&v, "key");
            let c = cancellation.ok_or("Missing request ownership")?;
            c.check()?;
            let ticket = v["ticket"].clone();
            let cache = store.project_dir(ident)?.join("Cache");
            let result = (|| match op {
                "scan" => {
                    let scan = scan::extract(&root, main, None)?;
                    c.check()?;
                    Ok(storage::merge_scan(p.clone(), scan))
                }
                "render" => {
                    let cards = v["cards"].as_array().ok_or("Missing equations")?;
                    let mut results = vec![];
                    compiler::render(&root, main, engine, cards, &cache, &c, |event| {
                        if c.check().is_ok() {
                            let _ = app.emit(
                                "render-progress",
                                json!({"ticket":ticket,"project":ident,"card":event}),
                            );
                            results.push(event)
                        }
                    })?;
                    Ok(json!(results))
                }
                "source_preview" => {
                    let temp = tempfile::tempdir().map_err(err)?;
                    let stage = temp.path().join("source");
                    snapshot(&root, &stage, &v["sources"])?;
                    let file = string(&v, "file");
                    safe(&stage, file)?;
                    let mut scan = scan::extract(&stage, main, None)?;
                    if !scan["files"].as_array().unwrap().iter().any(|s| s == file) {
                        scan = scan::extract(&stage, main, Some(file))?
                    }
                    let caret = v["offset"].as_u64().unwrap_or(0);
                    let card = scan["cards"].as_array().unwrap().iter().find(|card| {
                        string(card, "file") == file
                            && card["startUTF16"].as_u64().unwrap_or(u64::MAX) <= caret
                            && card["endUTF16"].as_u64().unwrap_or(0) >= caret
                    });
                    if let Some(card) = card {
                        let mut card = card.clone();
                        let preview_cache = cache.join("Previews");
                        compiler::render(
                            &stage,
                            main,
                            engine,
                            &[card.clone()],
                            &preview_cache,
                            &c,
                            |event| {
                                for (k, v) in event.as_object().unwrap() {
                                    card[k] = v.clone()
                                }
                            },
                        )?;
                        Ok(card)
                    } else {
                        Ok(Value::Null)
                    }
                }
                "document" => compiler::document(
                    &root,
                    main,
                    engine,
                    &store.project_dir(ident)?.join("Documents"),
                    &v["sources"],
                    &c,
                ),
                _ => unreachable!(),
            })();
            let mut jobs = runtime.jobs.lock().unwrap();
            if jobs
                .get(key)
                .is_some_and(|current| Arc::ptr_eq(&current.0, &c.0))
            {
                jobs.remove(key);
            }
            result
        }
        "pdf_navigate" => {
            let c = cancellation.unwrap_or_default();
            store.allowed_artifact(Path::new(string(&v["document"], "pdfPath")))?;
            store.allowed_artifact(Path::new(string(&v["document"], "sourceRoot")))?;
            let result = compiler::navigate(&v["document"], &v, &c);
            let mut jobs = runtime.jobs.lock().unwrap();
            let key = string(&v, "key");
            if jobs
                .get(key)
                .is_some_and(|current| Arc::ptr_eq(&current.0, &c.0))
            {
                jobs.remove(key);
            }
            result
        }
        "load_source" => {
            let p = Path::new(string(&v, "path"));
            Ok(json!(read(p)?))
        }
        "preview_export" => {
            let card = &v["card"];
            store.allowed_artifact(Path::new(string(card, "svgPath")))?;
            let p = store.load(ident)?;
            Ok(json!(export::image_file(
                card,
                string(&v, "format"),
                v["fontSize"]
                    .as_f64()
                    .or(p["exportFontSize"].as_f64())
                    .unwrap_or(0.),
                &store.project_dir(ident)?.join("Exports")
            )?))
        }
        "tools" => {
            let root = store.home.clone();
            let c = compiler::Cancellation::default();
            let mut result = serde_json::Map::new();
            for (name, flag) in [
                ("typst", "--version"),
                ("latexmk", "-v"),
                ("pdf2svg", "--version"),
                ("pdfcrop", "--version"),
                ("pdftocairo", "-v"),
            ] {
                let value = compiler::run(name, &compiler::args(&[flag]), &root, &c);
                result.insert(name.into(), json!(value.unwrap_or_else(|e| e)));
            }
            Ok(Value::Object(result))
        }
        _ => Err(format!("Unknown operation: {op}")),
    }
}
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_drag::init())
        .setup(|app| {
            let legacy = dirs::data_dir()
                .ok_or("Application support folder unavailable")?
                .join("Slide Math");
            let override_home = std::env::var_os("MATHLIDELUX_DATA_DIR").map(PathBuf::from);
            let home = override_home
                .clone()
                .unwrap_or_else(|| legacy.join("Rust Preview"));
            let legacy = if override_home.is_some() {
                home.join("Legacy")
            } else {
                legacy
            };
            let store = storage::Storage::new(home, legacy)?;
            app.manage(Arc::new(Runtime {
                settings: settings::SettingsFiles::new(&store.home),
                store,
                jobs: Mutex::new(HashMap::new()),
            }));
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                // The CSS background controls translucency. A native material here
                // leaves an opaque tint behind it and defeats the opacity slider.
                window.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 0)))?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                let runtime = window.state::<Arc<Runtime>>();
                for job in runtime.jobs.lock().unwrap().values() {
                    job.cancel();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![request])
        .run(tauri::generate_context!())
        .expect("Unable to start Mathlidelux");
}
