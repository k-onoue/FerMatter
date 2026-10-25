mod automation;
mod cache;
mod color_sampler;
mod compiler;
mod export;
mod files;
mod formatting;
mod instant;
mod instant_settings;
mod lifecycle;
mod native;
mod native_import;
mod project_commands;
mod project_data;
mod project_drop;
mod project_history;
mod project_location;
mod project_setup;
mod resident;
mod roundtrip;
mod scan;
mod settings;
#[cfg(unix)]
mod single_instance;
mod source_commands;
mod storage;
mod svg_payload;
mod toolchain;
pub mod verification;
#[cfg(target_os = "macos")]
mod wallpaper;
#[cfg(windows)]
mod windows_native;
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
    cache: Arc<cache::Cache>,
    settings: settings::SettingsFiles,
    jobs: Mutex<HashMap<String, compiler::Cancellation>>,
    tools: Mutex<Arc<toolchain::Toolchain>>,
    tools_revision: Mutex<String>,
}
fn job(runtime: &Runtime, key: &str) -> compiler::Cancellation {
    let c = compiler::Cancellation::with_tools(runtime.tools.lock().unwrap().clone());
    if let Some(old) = runtime.jobs.lock().unwrap().insert(key.into(), c.clone()) {
        old.cancel()
    }
    c
}
fn close_jobs(runtime: &Runtime, label: &str) {
    let prefix = format!("{label}::");
    runtime.cache.release_window(&prefix);
    runtime.jobs.lock().unwrap().retain(|key, job| {
        if key.starts_with(&prefix) {
            job.cancel();
            false
        } else {
            true
        }
    });
}
fn reveal_command(path: &Path) -> std::process::Command {
    #[cfg(target_os = "macos")]
    {
        let mut command = std::process::Command::new("/usr/bin/open");
        command.arg("-R").arg(path);
        command
    }
    #[cfg(windows)]
    {
        let mut command = std::process::Command::new("explorer.exe");
        // Explorer expects a conventional drive/UNC path in one /select argument.
        command.arg(format!("/select,{}", windows_path(path)));
        command
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        // xdg-open cannot select an item, so open the folder that holds it.
        let folder = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        let mut command = std::process::Command::new("xdg-open");
        command.arg(folder);
        command
    }
}
#[tauri::command]
async fn request(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    state: tauri::State<'_, Arc<Runtime>>,
    operation: String,
    mut payload: Value,
) -> Result<Value> {
    if operation == "sample_color" {
        return color_sampler::sample(&app).await.map(|color| json!(color));
    }
    if operation == "window_flush_complete" {
        lifecycle::complete(
            &app,
            window.label(),
            payload["token"].as_u64().unwrap_or(0),
            string(&payload, "error"),
        )?;
        return Ok(Value::Null);
    }
    let runtime = state.inner().clone();
    payload["window"] = json!(window.label());
    if operation != "desktop_background" {
        payload["key"] = json!(format!("{}::{}", window.label(), string(&payload, "key")));
    }
    if operation == "format_typst" {
        let key = string(&payload, "key").to_owned();
        let cancellation = job(&runtime, &key);
        return tauri::async_runtime::spawn_blocking(move || {
            let result = formatting::format(&payload, &cancellation);
            let mut jobs = runtime.jobs.lock().unwrap();
            if jobs
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(&current.0, &cancellation.0))
            {
                jobs.remove(&key);
            }
            result
        })
        .await
        .map_err(err)?;
    }
    if operation == "resident_workspace" {
        resident::workspace(&app, window.label(), &payload["projects"])?;
        return Ok(Value::Null);
    }
    if operation == "instant_import_pending" || operation == "incoming_import" {
        return resident::incoming(&app, window.label(), string(&payload, "token"));
    }
    if operation == "import_svg" {
        let sources = if let Some(sources) = payload["sources"].as_array() {
            sources
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "Invalid SVG batch.".to_string())
                })
                .collect::<Result<Vec<_>>>()?
        } else if let Some(svg) = payload["svg"].as_str() {
            vec![svg.to_owned()]
        } else {
            vec![roundtrip::read_svg(Path::new(string(&payload, "path")))?]
        };
        let target = if string(&payload, "target") == "workspace" {
            "workspace"
        } else if window.label().starts_with("instant") {
            window.label()
        } else {
            ""
        };
        resident::import_svg_to(&app, &sources, target)?;
        return Ok(Value::Null);
    }
    if operation == "instant_clipboard" {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let handle = app.clone();
        let target = if window.label() == "main" {
            "workspace".to_owned()
        } else {
            window.label().to_owned()
        };
        app.run_on_main_thread(move || {
            let _ = tx.send(resident::from_clipboard_to(&handle, &target));
        })
        .map_err(err)?;
        rx.recv().map_err(err)??;
        return Ok(Value::Null);
    }
    if operation == "instant_open" {
        if lifecycle::quitting(&app) {
            return Err("Finish saving before opening another editor.".into());
        }
        if !string(&payload, "project").is_empty() && project_setup::required(&runtime.store)? {
            return Err("Choose a project folder in the main window first.".into());
        }
        let project = if string(&payload, "project").is_empty() {
            None
        } else {
            source_commands::recover_pending(&runtime.store, string(&payload, "project"))?;
            Some(runtime.store.load(string(&payload, "project"))?)
        };
        instant::open(&app, project.as_ref()).map_err(err)?;
        return Ok(Value::Null);
    }
    if matches!(
        operation.as_str(),
        "instant_session" | "instant_context" | "instant_render" | "instant_export"
    ) {
        // A local window owns exactly one project's scratch data, independent of navigation in main.
        payload["project"] = json!(window.label().strip_prefix("instant-").unwrap_or(""));
    }
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
        "scan"
            | "instant_render"
            | "render"
            | "source_preview"
            | "document"
            | "pdf_navigate"
            | "pdf_drag_target"
            | "pdf_prepare_drag"
    ) {
        Some(job(&runtime, string(&payload, "key")))
    } else {
        None
    };
    let window_label = string(&payload, "window").to_owned();
    let output_key = format!("{window_label}::result::{operation}");
    let generation = matches!(
        operation.as_str(),
        "document"
            | "source_preview"
            | "instant_render"
            | "preview_export"
            | "pdf_prepare_drag"
            | "export"
    )
    .then(|| runtime.cache.begin_result(&output_key));
    tauri::async_runtime::spawn_blocking(move || {
        let _lease = cache::Lease::new(&runtime.store.home, &payload);
        let result = dispatch(&app, &runtime, &operation, payload, cancellation.clone());
        if app.get_webview_window(&window_label).is_some()
            && cancellation.as_ref().is_none_or(|c| c.check().is_ok())
        {
            if let (Some(generation), Ok(value)) = (generation, &result) {
                runtime.cache.result(&output_key, generation, value);
            }
        }
        if let Err(error) = runtime.cache.idle() {
            eprintln!("Cache maintenance skipped: {error}");
        }
        result
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
    if !ident.is_empty() {
        source_commands::recover_pending(store, ident)?;
    }
    match op {
        "instant_session" => instant::session(store, &v),
        "instant_context" => instant::context(store, ident),
        "instant_configuration" => {
            instant_settings::load(store, &runtime.settings)?;
            runtime.settings.snapshot("instant.json", false)
        }
        "reveal_instant_configuration" => {
            reveal_command(&store.home.join("Settings/instant.json"))
                .spawn()
                .map_err(err)?;
            Ok(Value::Null)
        }
        "instant_export" => {
            let path = store.allowed_artifact(Path::new(string(&v, "path")))?;
            if !path.starts_with(
                instant::home(store, ident)?
                    .join("Instant/Exports")
                    .canonicalize()
                    .map_err(err)?,
            ) {
                return Err("Not an Instant image.".into());
            }
            fs::copy(path, string(&v, "destination")).map_err(err)?;
            Ok(Value::Null)
        }
        "instant_render" => {
            let c = cancellation.ok_or("Missing request ownership")?;
            let result = if ident.is_empty() {
                instant::render(&store.home, &v["draft"], &c)
            } else {
                instant::render_project(store, ident, &v["draft"], &c)
            };
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
        "desktop_background" => {
            #[cfg(target_os = "macos")]
            return wallpaper::background(app, string(&v, "window"), string(&v, "key"));
            #[cfg(not(target_os = "macos"))]
            Ok(Value::Null)
        }
        "startup" => {
            let templates = store.home.join("templates.json");
            if !templates.exists() && store.legacy.join("templates.json").is_file() {
                fs::copy(store.legacy.join("templates.json"), &templates).map_err(err)?;
            }
            let settings = store.home.join("Settings");
            for file in [
                "theme.json",
                "keybindings.json",
                ".last-valid-theme.json",
                ".last-valid-keybindings.json",
            ] {
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
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../resources/themes")
            };
            let themes: Vec<_> = fs::read_dir(themes_path)
                .map_err(err)?
                .flatten()
                .filter(|p| p.file_name().to_string_lossy().ends_with(".theme.json"))
                .filter_map(|p| json_read(&p.path()).ok())
                .collect();
            let setup_required = project_setup::required(store)?;
            let projects = if setup_required {
                vec![]
            } else {
                store.recent()?
            };
            Ok(
                json!({"setupRequired":setup_required,"projects":projects,"projectProblems":store.project_problems()?,"settings":runtime.settings.snapshot("preferences.json", false)?,"defaultProjectDirectory":project_location::default_directory()?,"theme":runtime.settings.snapshot("theme.json", false)?,"keybindings":runtime.settings.snapshot("keybindings.json", false)?,"themes":themes,"templates":json_read(&store.home.join("templates.json")).unwrap_or(json!({})),"storage":store.home}),
            )
        }
        "automation_projects" => Ok(json!(store
            .recent()?
            .into_iter()
            .filter(|p| p["automationGeneration"].as_u64().unwrap_or(0) > 0)
            .collect::<Vec<_>>())),
        "project_problems" => Ok(json!(store.project_problems()?)),
        "recover_project" => {
            let _guard = store.writes.lock().unwrap();
            store.recover_project(ident, string(&v, "revision"))?;
            Ok(json!({"projects":store.recent()?,"problems":store.project_problems()?}))
        }
        "complete_project_setup" => {
            let _guard = store.writes.lock().unwrap();
            project_setup::finish(store, &runtime.settings, string(&v, "directory"))
        }
        "open_project" => project_commands::open(store, &runtime.settings, &v),
        "new_project" => project_commands::create(store, &runtime.settings, &v),
        "project_action" => project_commands::action(app, store, &runtime.settings, &v),
        "file_operation" => source_commands::operate(store, ident, &v),
        "dropped_project_paths" => project_drop::paths(&v["names"]),
        "project_history" => Ok(project_history::History::load(store)?.status()),
        "save_project" => {
            let _guard = store.writes.lock().unwrap();
            let current = store.load(ident)?;
            store.save(
                ident,
                &automation::merge_unseen_variants(&v["value"], &current),
            )?;
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
        "read_drafts" => {
            let _guard = store.writes.lock().unwrap();
            store.read_drafts(ident)
        }
        "save_drafts" => {
            let _guard = store.writes.lock().unwrap();
            json_write(&store.draft_path(ident)?, &v["value"])?;
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
            reveal_command(&path).spawn().map_err(err)?;
            Ok(Value::Null)
        }
        "settings" => {
            let _guard = store.writes.lock().unwrap();
            if string(&v, "file") == "keybindings.json" {
                return Ok(json!(resident::save_keys(
                    app,
                    &runtime.settings,
                    &v["value"],
                    string(&v, "revision")
                )?));
            }
            let revision =
                runtime
                    .settings
                    .save(string(&v, "file"), &v["value"], string(&v, "revision"))?;
            drop(_guard);
            Ok(json!(revision))
        }
        "reload_settings" => {
            let _guard = store.writes.lock().unwrap();
            let snapshot = runtime.settings.snapshot(string(&v, "file"), true)?;
            if string(&v, "file") == "keybindings.json" {
                resident::apply_keys(app, &snapshot["value"])?;
            }
            drop(_guard);
            Ok(snapshot)
        }
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
        "artifact_lease" => {
            let paths = v["paths"]
                .as_array()
                .ok_or("Missing artifact paths")?
                .iter()
                .map(|p| store.allowed_artifact(Path::new(p.as_str().unwrap_or(""))))
                .collect::<Result<Vec<_>>>()?;
            runtime.cache.lease(string(&v, "key"), &json!(paths));
            Ok(Value::Null)
        }
        "artifact" => {
            use base64::Engine;
            let p = store.allowed_artifact(Path::new(string(&v, "path")))?;
            let bytes = read_bounded(&p, ARTIFACT_LIMIT)?;
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
                    .ok_or(
                        "Invalid project cards. Reopen the project or restore its metadata backup.",
                    )?
                    .iter()
                    .find(|c| c["id"] == *id)
                    .ok_or("Equation not found")?;
                store.allowed_artifact(Path::new(string(card, "svgPath")))?;
                let size = export::font_size(&Value::Null, card, &p, &runtime.settings)?;
                let mut card = card.clone();
                if string(&v, "format") == "svg" {
                    card["restoreMetadata"] = roundtrip::card_recipe(store, &p, &card, size)?;
                }
                paths.push(export::image_file(
                    &card,
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
            cache::copy_images(&store.home, &paths, || {
                let owned = paths.clone();
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                app.run_on_main_thread(move || {
                    let _ = tx.send(native::clipboard(&owned));
                })
                .map_err(err)?;
                rx.recv().map_err(err)?
            })?;
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
                    let scan = scan::extract_checked(&root, main, None, &|| c.check())?;
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
                    snapshot_checked(&root, &stage, &v["sources"], &|| c.check())?;
                    let file = string(&v, "file");
                    safe(&stage, file)?;
                    let mut scan = scan::extract_checked(&stage, main, None, &|| c.check())?;
                    if !scan["files"].as_array().unwrap().iter().any(|s| s == file) {
                        scan = scan::extract_checked(&stage, main, Some(file), &|| c.check())?
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
        "pdf_navigate" | "pdf_drag_target" | "pdf_prepare_drag" => {
            let c = cancellation.unwrap_or_default();
            store.allowed_artifact(Path::new(string(&v["document"], "pdfPath")))?;
            store.allowed_artifact(Path::new(string(&v["document"], "sourceRoot")))?;
            let result = (|| {
                c.check()?;
                if op == "pdf_navigate" {
                    return compiler::navigate(&v["document"], &v, &c);
                }
                if op == "pdf_drag_target" {
                    return compiler::drag_target(&v["document"], &v, &c);
                }
                store.allowed_artifact(Path::new(string(&v["document"], "renderRoot")))?;
                let p = store.load(ident)?;
                let mut card = compiler::document_artwork(
                    &v["document"],
                    string(&v, "targetID"),
                    &store.project_dir(ident)?.join("Cache/PDFArtwork"),
                    &c,
                )?;
                let item = p["cards"]
                    .as_array()
                    .and_then(|cards| cards.iter().find(|item| item["id"] == card["id"]))
                    .unwrap_or(&Value::Null);
                let size = export::font_size(&v["fontSize"], item, &p, &runtime.settings)?;
                if string(&v, "format") == "svg" {
                    card["restoreMetadata"] = roundtrip::card_recipe(store, &p, &card, size)?;
                }
                let path = export::image_file(
                    &card,
                    string(&v, "format"),
                    size,
                    &store.project_dir(ident)?.join("Exports"),
                )?;
                c.check()?;
                Ok(json!({"card":card,"path":path}))
            })();
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
            let mut card = v["card"].clone();
            store.allowed_artifact(Path::new(string(&card, "svgPath")))?;
            let p = store.load(ident)?;
            let size = export::font_size(&v["fontSize"], &card, &p, &runtime.settings)?;
            if string(&v, "format") == "svg" {
                card["restoreMetadata"] = roundtrip::card_recipe(store, &p, &card, size)?;
            }
            Ok(json!(export::image_file(
                &card,
                string(&v, "format"),
                size,
                &store.project_dir(ident)?.join("Exports")
            )?))
        }
        "choose_tool" => {
            let name = string(&v, "name").to_owned();
            #[cfg(target_os = "macos")]
            let initial = string(&v, "path").to_owned();
            toolchain::validate(
                &json!({"schemaVersion":1,"kind":"mathlidelux.tools","paths":{name.clone():""}}),
            )?;
            #[cfg(target_os = "macos")]
            {
                let (tx, rx) = std::sync::mpsc::sync_channel(1);
                app.run_on_main_thread(move || {
                    let _ = tx.send(native::choose_executable(&name, &initial));
                })
                .map_err(err)?;
                Ok(json!(rx.recv().map_err(err)??))
            }
            #[cfg(not(target_os = "macos"))]
            {
                use tauri_plugin_dialog::DialogExt;
                let path = app
                    .dialog()
                    .file()
                    .set_title(format!("Choose {name} executable"))
                    .blocking_pick_file();
                Ok(json!(path.map(|path| path.to_string())))
            }
        }
        "tools" => {
            let _guard = store.writes.lock().unwrap();
            let mut status = runtime.tools.lock().unwrap().status();
            status["revision"] = json!(*runtime.tools_revision.lock().unwrap());
            Ok(status)
        }
        "check_tools" => Ok(toolchain::Toolchain::from_value(&v["value"])?.status()),
        "save_tools" | "reload_tools" => {
            let _guard = store.writes.lock().unwrap();
            let (tools, revision) = if op == "save_tools" {
                let tools = toolchain::Toolchain::from_value(&v["value"])?;
                tools.check_overrides()?;
                let revision =
                    runtime
                        .settings
                        .save("tools.json", &v["value"], string(&v, "revision"))?;
                (tools, revision)
            } else {
                let snapshot = runtime.settings.snapshot("tools.json", true)?;
                (
                    toolchain::Toolchain::from_value(&snapshot["value"])?,
                    string(&snapshot, "revision").to_owned(),
                )
            };
            let mut result = tools.status();
            result["revision"] = json!(revision);
            *runtime.tools_revision.lock().unwrap() = revision;
            *runtime.tools.lock().unwrap() = Arc::new(tools);
            Ok(result)
        }
        _ => Err(format!("Unknown operation: {op}")),
    }
}
pub fn run() {
    if automation::entry() || formatting::worker_requested() {
        return;
    }
    let mut context = tauri::generate_context!();
    if std::env::args().any(|a| a == "--automation-background") {
        for window in &mut context.config_mut().app.windows {
            window.visible = false;
        }
    }
    #[cfg(unix)]
    let guard = match single_instance::claim(&context.config().identifier) {
        Ok(single_instance::Claim::Primary(guard)) => guard,
        Ok(single_instance::Claim::Secondary(socket)) => {
            single_instance::secondary(context, socket);
            return;
        }
        Err(error) => {
            eprintln!("Unable to establish the FerMatter resident: {error}");
            return;
        }
    };
    let builder = tauri::Builder::default();
    #[cfg(windows)]
    let builder = builder.plugin(tauri_plugin_single_instance::init(|app, arguments, cwd| {
        windows_arguments(app, arguments.into_iter().skip(1), Path::new(&cwd));
    }));
    #[cfg(unix)]
    let builder = builder.manage(guard);
    builder
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: None,
                    }),
                ])
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .level_for("tauri", log::LevelFilter::Info)
                .level_for("tao", log::LevelFilter::Info)
                .level_for("wry", log::LevelFilter::Info)
                .max_file_size(5 * 1024 * 1024)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepSome(3))
                .timezone_strategy(tauri_plugin_log::TimezoneStrategy::UseLocal)
                .build(),
        )
        .manage(resident::Resident::default())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(resident::hotkey)
                .build(),
        )
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
            match cache::maintain(&store.home) {
                Ok(report) => {
                    let _ = json_write(&store.home.join("CacheMaintenance.json"), &report);
                }
                Err(error) => eprintln!("Cache cleanup skipped: {error}"),
            }
            let settings = settings::SettingsFiles::new(&store.home);
            #[cfg(target_os = "macos")]
            if override_home.is_none() {
                if let Some(home) = dirs::home_dir() {
                    settings.import_swift_preferences(
                        &home.join("Library/Preferences/local.slidemath.prototype.plist"),
                    )?;
                }
            }
            let tools_snapshot = settings.snapshot("tools.json", false)?;
            let tools = toolchain::Toolchain::from_value(&tools_snapshot["value"])?;
            let cache = cache::Cache::new(&store.home);
            let idle_cache = Arc::downgrade(&cache);
            std::thread::spawn(move || loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
                let Some(cache) = idle_cache.upgrade() else {
                    break;
                };
                if let Err(error) = cache.idle() {
                    eprintln!("Cache maintenance skipped: {error}");
                }
            });
            app.manage(Arc::new(Runtime {
                cache,
                settings,
                store,
                jobs: Mutex::new(HashMap::new()),
                tools: Mutex::new(Arc::new(tools)),
                tools_revision: Mutex::new(string(&tools_snapshot, "revision").to_owned()),
            }));
            app.manage(lifecycle::Lifecycle::default());
            #[cfg(unix)]
            {
                app.state::<single_instance::Guard>().listen(app.handle())?;
                app.manage(automation::transport::start(app.handle())?);
            }
            let menu = tauri::menu::Menu::default(app.handle())?;
            for item in menu.items()? {
                if let Some(submenu) = item.as_submenu() {
                    #[cfg(windows)]
                    if submenu.text()? == "Edit" {
                        // Windows predefined Undo/Redo reserve Ctrl+Z/Y and
                        // reinject those keys instead of using Monaco's history.
                        // Let the focused WebView editor handle its own undo stack.
                        for child in submenu.items()? {
                            if let Some(predefined) = child.as_predefined_menuitem() {
                                if matches!(predefined.text()?.as_str(), "Undo" | "Redo") {
                                    submenu.remove(&child)?;
                                }
                            }
                        }
                        if let Some(first) = submenu.items()?.first() {
                            if let Some(predefined) = first.as_predefined_menuitem() {
                                if predefined.text()?.is_empty() {
                                    submenu.remove(first)?;
                                }
                            }
                        }
                    }
                    if submenu.text()? == "File" {
                        submenu.insert(
                            &tauri::menu::MenuItem::with_id(
                                app,
                                "instant",
                                "New Instant",
                                true,
                                None::<&str>,
                            )?,
                            0,
                        )?;
                    }
                }
            }
            app.set_menu(menu)?;
            resident::install(app.handle())?;
            let runtime = app.state::<Arc<Runtime>>();
            if let Err(error) =
                resident::apply_keys(app.handle(), &runtime.settings.load("keybindings.json")?)
            {
                resident::report(app.handle(), error);
            }
            #[cfg(target_os = "macos")]
            if let Some(window) = app.get_webview_window("main") {
                // The CSS background controls translucency. A native material here
                // leaves an opaque tint behind it and defeats the opacity slider.
                window.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 0)))?;
                project_drop::install(&window)?;
            }
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "instant" {
                let runtime = app.state::<Arc<Runtime>>();
                if project_setup::required(&runtime.store).unwrap_or(true) {
                    lifecycle::cancel_close(app, "main");
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                    return;
                }
                if let Err(error) = instant::open(app, None) {
                    eprintln!("Could not open Instant: {error}");
                }
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                lifecycle::request_close(window.app_handle(), window.label());
            }
            if matches!(event, tauri::WindowEvent::Destroyed) {
                #[cfg(target_os = "macos")]
                project_drop::clear(window.label());
                let runtime = window.state::<Arc<Runtime>>();
                close_jobs(&runtime, window.label());
            }
        })
        .invoke_handler(tauri::generate_handler![request])
        .build(context)
        .expect("Unable to start FerMatter")
        .run(|app, event| match event {
            #[cfg(windows)]
            tauri::RunEvent::Ready => {
                windows_arguments(
                    app,
                    std::env::args().skip(1),
                    &std::env::current_dir().unwrap_or_default(),
                );
            }
            #[cfg(unix)]
            tauri::RunEvent::Ready => {
                #[cfg(target_os = "macos")]
                if let Err(error) =
                    native::set_application_icon(include_bytes!("../icons/icon.png"))
                {
                    log::warn!("Dock icon unchanged: {error}");
                }
                single_instance::primary_arguments(app);
            }
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Opened { urls } => resident::open_files(
                app,
                urls.into_iter()
                    .filter_map(|url| url.to_file_path().ok())
                    .collect(),
            ),
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { .. } => {
                resident::show_projects(app);
            }
            tauri::RunEvent::ExitRequested { api, .. } if !lifecycle::exiting(app) => {
                api.prevent_exit();
                lifecycle::request_quit(app);
            }
            _ => {}
        });
}

#[cfg(windows)]
fn windows_arguments(app: &tauri::AppHandle, arguments: impl Iterator<Item = String>, cwd: &Path) {
    let paths: Vec<_> = arguments
        .filter(|arg| !arg.starts_with('-'))
        .map(|arg| {
            let path = PathBuf::from(arg);
            if path.is_absolute() {
                path
            } else {
                cwd.join(path)
            }
        })
        .filter(|path| {
            path.extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("svg"))
        })
        .collect();
    if paths.is_empty() {
        resident::show_projects(app);
    } else {
        resident::open_files(app, paths);
    }
}

#[cfg(test)]
mod request_tests {
    use super::*;
    #[test]
    fn closing_instant_does_not_cancel_project_work() {
        let temp = tempfile::tempdir().unwrap();
        let runtime = Runtime {
            cache: cache::Cache::new(&temp.path().join("data")),
            store: storage::Storage::new(temp.path().join("data"), temp.path().join("legacy"))
                .unwrap(),
            settings: settings::SettingsFiles::new(temp.path()),
            jobs: Mutex::new(HashMap::new()),
            tools: Mutex::new(Arc::default()),
            tools_revision: Mutex::new(String::new()),
        };
        let main = job(&runtime, "main::render");
        let old = job(&runtime, "instant::render");
        let instant = job(&runtime, "instant::render");
        assert!(old.check().is_err());
        assert!(main.check().is_ok());
        close_jobs(&runtime, "instant");
        assert!(instant.check().is_err());
        assert!(main.check().is_ok());
        assert_eq!(runtime.jobs.lock().unwrap().len(), 1);
    }
    #[cfg(target_os = "macos")]
    #[test]
    fn reveal_selects_the_item_in_finder_on_macos() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("instant.json");
        let command = reveal_command(&file);
        assert_eq!(command.get_program(), "/usr/bin/open");
        let args: Vec<_> = command.get_args().collect();
        assert_eq!(args, [std::ffi::OsStr::new("-R"), file.as_os_str()]);
    }
    #[cfg(windows)]
    #[test]
    fn reveal_selects_the_item_in_explorer_with_conventional_windows_paths() {
        for (path, expected) in [
            (
                r"\\?\C:\Synthetic Folder\instant.json",
                r"/select,C:\Synthetic Folder\instant.json",
            ),
            (
                r"\\?\UNC\server\share\Synthetic Folder\instant.json",
                r"/select,\\server\share\Synthetic Folder\instant.json",
            ),
        ] {
            let command = reveal_command(Path::new(path));
            assert_eq!(command.get_program(), "explorer.exe");
            assert_eq!(
                command.get_args().collect::<Vec<_>>(),
                [std::ffi::OsStr::new(expected)]
            );
        }
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    #[test]
    fn reveal_opens_the_containing_folder_elsewhere() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("instant.json");
        fs::write(&file, "{}").unwrap();
        let for_file = reveal_command(&file);
        let for_folder = reveal_command(temp.path());
        assert_eq!(for_file.get_program(), "xdg-open");
        assert_eq!(for_file.get_args().collect::<Vec<_>>(), [temp.path()]);
        assert_eq!(for_folder.get_args().collect::<Vec<_>>(), [temp.path()]);
    }
}
