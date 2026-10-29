//! Project-free scratch work. Drafts/history are durable; compiler inputs are disposable.
use crate::{compiler, export, files::*, storage::Storage};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn home(store: &Storage, project: &str) -> Result<PathBuf> {
    if project.is_empty() {
        Ok(store.home.clone())
    } else {
        store.load(project)?;
        store.project_dir(project)
    }
}

pub fn validate(draft: &Value) -> Result<()> {
    if let Some(ident) = draft.get("entryID") {
        let ident = ident.as_str().ok_or("Invalid Instant entry ID.")?;
        if ident.is_empty()
            || ident.len() > 64
            || !ident.bytes().all(|c| c.is_ascii_hexdigit() || c == b'-')
        {
            return Err("Invalid Instant entry ID.".into());
        }
    }
    let template = &draft["template"];
    if !["latex", "typst"].contains(&string(template, "format"))
        || !["inline", "block", "figure"].contains(&string(draft, "kind"))
        || !["svg", "pdf", "png", "jpeg"].contains(&string(draft, "exportFormat"))
        || !["pdflatex", "xelatex", "lualatex"].contains(&string(template, "compiler"))
    {
        return Err("Invalid Instant settings.".into());
    }
    if !draft["source"].is_string() || string(draft, "source").len() > 1_000_000 {
        return Err("Instant source must be text smaller than 1 MB.".into());
    }
    if string(template, "source").len() > 1_000_000 {
        return Err("Instant template is too large.".into());
    }
    let size = draft["fontSize"].as_f64().unwrap_or(0.);
    if !size.is_finite() || !(1. ..=200.).contains(&size) {
        return Err("Choose an output size between 1 and 200 pt.".into());
    }
    let color = string(draft, "color");
    if color.len() != 7
        || !color.starts_with('#')
        || !color[1..].bytes().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Choose a valid equation color.".into());
    }
    Ok(())
}

fn load(home: &Path) -> Result<Value> {
    let file = home.join("Instant/session.json");
    if !file.exists() {
        return Ok(json!({"schemaVersion":1,"draft":null,"history":[]}));
    }
    let state = json_read(&file)?;
    if state["schemaVersion"] != 1 || !state["history"].is_array() {
        return Err("Cannot read Instant history. The saved file has been preserved.".into());
    }
    if !state["draft"].is_null() {
        validate(&state["draft"])?;
    }
    for item in state["history"].as_array().unwrap() {
        validate(&item["draft"])?;
    }
    Ok(state)
}

// Older sessions used content hashes and saved entries only on export/switch.
// Preserve every old entry, then give the working draft a stable identity so
// ordinary typing and appearance edits update one reusable item.
fn migrate_library(state: &mut Value) {
    let draft_id = if state["draft"].is_null() {
        String::new()
    } else {
        state["draft"]["entryID"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| id(&state["draft"].to_string()))
    };
    for item in state["history"].as_array_mut().unwrap() {
        let ident = item["id"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .unwrap_or_else(|| id(&item["draft"].to_string()));
        item["id"] = json!(ident);
        item["draft"]["entryID"] = json!(ident);
    }
    if !draft_id.is_empty() {
        state["draft"]["entryID"] = json!(draft_id);
        remember(state, &state["draft"].clone());
    }
}

fn remember(state: &mut Value, draft: &Value) {
    if string(draft, "source").trim().is_empty() {
        return;
    }
    let ident = draft["entryID"]
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| id(&draft.to_string()));
    let mut draft = draft.clone();
    draft["entryID"] = json!(ident);
    let history = state["history"].as_array_mut().unwrap();
    if let Some(item) = history.iter_mut().find(|item| item["id"] == ident) {
        item["draft"] = draft;
    } else {
        history.insert(0, json!({"id":ident,"draft":draft}));
    }
}

fn is_pending_import(draft: &Value) -> bool {
    draft["requiresRender"] == true || draft["importAuthorization"].is_string()
}

fn same_import_snapshot(left: &Value, right: &Value) -> bool {
    let mut left = left.clone();
    let mut right = right.clone();
    for draft in [&mut left, &mut right] {
        if let Some(draft) = draft.as_object_mut() {
            draft.remove("entryID");
            draft.remove("requiresRender");
            draft.remove("importAuthorization");
        }
    }
    left == right
}

fn same_import_content(left: &Value, right: &Value) -> bool {
    ["source", "template", "kind", "contextFile"]
        .iter()
        .all(|key| left.get(key) == right.get(key))
}

fn pending_import<'a>(state: &'a Value, draft: &Value) -> Option<&'a Value> {
    let ident = string(draft, "entryID");
    std::iter::once(&state["draft"])
        .chain(
            state["history"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|item| &item["draft"]),
        )
        .filter(|known| is_pending_import(known))
        .find(|known| known["entryID"].as_str() == Some(ident) || same_import_content(known, draft))
}

fn preserve_pending_import(state: &Value, draft: &mut Value) {
    if let Some(pending) = pending_import(state, draft) {
        if pending["importAuthorization"].is_string() {
            draft["importAuthorization"] = pending["importAuthorization"].clone();
        }
        draft["requiresRender"] = json!(true);
    }
}

fn reject_pending_import(store: &Storage, project: &str, draft: &Value) -> Result<()> {
    if draft["requiresRender"] == true || draft["importAuthorization"].is_string() {
        return Err("Press Render to compile imported source.".into());
    }
    let state = load(&home(store, project)?)?;
    if pending_import(&state, draft).is_some() {
        return Err("Press Render to compile imported source.".into());
    }
    Ok(())
}

pub fn session(store: &Storage, request: &Value) -> Result<Value> {
    let _lock = store.writes.lock().unwrap();
    let home = home(store, string(request, "project"))?;
    let mut state = load(&home)?;
    let previous = state.clone();
    migrate_library(&mut state);
    match string(request, "action") {
        "save" => {
            let mut draft = request["draft"].clone();
            validate(&draft)?;
            preserve_pending_import(&state, &mut draft);
            state["draft"] = draft.clone();
            remember(&mut state, &draft);
        }
        "remember" => {
            let mut draft = request["draft"].clone();
            validate(&draft)?;
            preserve_pending_import(&state, &mut draft);
            remember(&mut state, &draft);
        }
        "authorize_render" => {
            let requested = &request["draft"];
            let ident = string(requested, "entryID");
            let stored = std::iter::once(&state["draft"])
                .chain(
                    state["history"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .map(|item| &item["draft"]),
                )
                .find(|draft| is_pending_import(draft) && draft["entryID"].as_str() == Some(ident))
                .ok_or("Imported source changed; reopen the SVG.")?;
            let stored_token = stored["importAuthorization"].as_str();
            let requested_token = requested["importAuthorization"].as_str();
            if !same_import_snapshot(stored, requested)
                || (stored_token.is_some() && stored_token != requested_token)
            {
                return Err("Imported source changed; reopen the SVG.".into());
            }
            let mut authorized = requested.clone();
            validate(&authorized)?;
            authorized
                .as_object_mut()
                .unwrap()
                .remove("importAuthorization");
            authorized["requiresRender"] = json!(false);
            if state["draft"]["entryID"].as_str() == Some(ident) {
                state["draft"] = authorized.clone();
            }
            remember(&mut state, &authorized);
        }
        "load" => {}
        _ => return Err("Unknown Instant history operation.".into()),
    }
    if state != previous {
        json_write(&home.join("Instant/session.json"), &state)?;
    }
    Ok(state)
}

pub fn source(draft: &Value) -> Result<(String, String, Value)> {
    validate(draft)?;
    let template = &draft["template"];
    let source = string(template, "source");
    if source.matches("{{content}}").count() != 1 {
        return Err("Place {{content}} exactly once in the template.".into());
    }
    let (before, after) = source.split_once("{{content}}").unwrap();
    let typst = string(template, "format") == "typst";
    let main = if typst { "main.typ" } else { "main.tex" };
    let color = &string(draft, "color")[1..];
    let prefix = if typst {
        format!("{before}\n#set text(fill: rgb(\"{color}\"))\n")
    } else {
        let mark = crate::scan::re(r"\\begin\s*\{document\}");
        let position = mark
            .find(before)
            .ok_or("Put {{content}} inside the document environment.")?
            .start();
        format!(
            "{}\n\\usepackage{{xcolor}}\n\\AtBeginDocument{{\\color[HTML]{{{color}}}}}\n{}",
            &before[..position],
            &before[position..]
        )
    };
    // The compiler renders isolated preview environments, which may reset the
    // document's current color. Apply color to the artwork itself as well.
    let card = json!({"id":"instant","name":"Instant","tex":draft["source"],"kind":draft["kind"],"origin":"custom","sourceOrder":0,"status":"pending","instantColor":color,"typstContext":[{"file":main,"offset":prefix.chars().count()}]});
    let body = if typst {
        compiler::typst_body(&card)?
    } else {
        compiler::tex_body(&card)
    };
    Ok((main.into(), format!("{prefix}\n{body}\n{after}"), card))
}

pub fn render(store: &Storage, draft: &Value, cancel: &compiler::Cancellation) -> Result<Value> {
    let home = &store.home;
    reject_pending_import(store, "", draft)?;
    let (main, content, mut card) = source(draft)?;
    if string(draft, "source").trim().is_empty() {
        return Err("Enter an equation or drawing.".into());
    }
    let temp = tempfile::tempdir().map_err(err)?;
    fs::write(temp.path().join(&main), content).map_err(err)?;
    compiler::render(
        temp.path(),
        &main,
        string(&draft["template"], "compiler"),
        &[card.clone()],
        &home.join("Instant/Cache"),
        cancel,
        |event| {
            for (key, value) in event.as_object().unwrap() {
                card[key] = value.clone();
            }
        },
    )?;
    cancel.check()?;
    if card["status"] != "ready" {
        return Err(string(&card, "error").into());
    }
    card["restoreMetadata"] = crate::roundtrip::recipe(draft, draft["restorationProject"].clone());
    let path = export::image_file(
        store,
        &card,
        string(draft, "exportFormat"),
        draft["fontSize"].as_f64().unwrap(),
        &home.join("Instant/Exports"),
    )?;
    cancel.check()?;
    Ok(json!({"card":card,"path":path}))
}

pub fn context(store: &Storage, project: &str) -> Result<Value> {
    if project.is_empty() {
        return Ok(Value::Null);
    }
    let p = store.load(project)?;
    let root = store.root(project)?;
    let sources: Vec<_> = entries(&root)?
        .into_iter()
        .filter(|(name, _, dir)| {
            !dir && [".tex", ".sty", ".cls", ".typ"]
                .iter()
                .any(|ext| name.ends_with(ext))
        })
        .take(200)
        .filter_map(|(name, path, _)| {
            read(&path)
                .ok()
                .map(|text| json!({"path":name,"text":text}))
        })
        .collect();
    Ok(
        json!({"id":project,"root":root,"main":p["main"],"format":p["format"],"engine":p["engine"],"sources":sources,"revision":format!("{}|{}|{}",fingerprint(&root)?,p["main"],p["engine"])}),
    )
}

pub fn render_project(
    store: &Storage,
    project: &str,
    draft: &Value,
    cancel: &compiler::Cancellation,
) -> Result<Value> {
    validate(draft)?;
    reject_pending_import(store, project, draft)?;
    let p = store.load(project)?;
    if draft["template"]["format"] != p["format"] {
        return Err("The project format changed. Start a new equation.".into());
    }
    cancel.check()?;
    let root = store.root(project)?;
    let temp = tempfile::tempdir().map_err(err)?;
    let stage = temp.path().join("source");
    snapshot_checked(&root, &stage, &Value::Null, &|| cancel.check())?;
    let main = string(&p, "main");
    let color = &string(draft, "color")[1..];
    let mut card = json!({"id":"instant","name":"Instant","tex":draft["source"],"kind":draft["kind"],"origin":"custom","sourceOrder":0,"instantColor":color});
    if main.ends_with(".typ") {
        let file = draft["contextFile"]
            .as_str()
            .filter(|s| !s.is_empty())
            .unwrap_or(main);
        if !file.ends_with(".typ") {
            return Err("Choose a Typst context file.".into());
        }
        let text = read(&safe(&stage, file)?)?;
        let scan = crate::scan::extract(&stage, main, None)?;
        let mut frames = scan["cards"]
            .as_array()
            .unwrap()
            .iter()
            .find(|card| string(card, "file") == file)
            .and_then(|card| card["typstContext"].as_array().cloned())
            .unwrap_or_else(|| vec![json!({"file":file})]);
        // Use all top-level definitions in the selected file, including ones after its last equation.
        if let Some(frame) = frames.last_mut() {
            frame["offset"] = json!(text.chars().count());
        }
        card["typstContext"] = json!(frames);
    } else {
        let path = safe(&stage, main)?;
        let source = read(&path)?;
        let marker = crate::scan::re(r"\\begin\s*\{document\}");
        let position = marker
            .find(&crate::scan::uncomment(&source))
            .ok_or("Main file has no document environment")?
            .start();
        fs::write(
            path,
            format!(
                "{}\n\\usepackage{{xcolor}}\n\\AtBeginDocument{{\\color[HTML]{{{color}}}}}\n{}",
                &source[..position],
                &source[position..]
            ),
        )
        .map_err(err)?;
    }
    let home = home(store, project)?;
    compiler::render(
        &stage,
        main,
        string(&p, "engine"),
        &[card.clone()],
        &home.join("Instant/Cache"),
        cancel,
        |event| {
            for (key, value) in event.as_object().unwrap() {
                card[key] = value.clone();
            }
        },
    )?;
    cancel.check()?;
    if card["status"] != "ready" {
        return Err(string(&card, "error").into());
    }
    let mut portable = draft.clone();
    portable["template"] = crate::roundtrip::fallback_template(&p, &root);
    card["restoreMetadata"] =
        crate::roundtrip::recipe(&portable, crate::roundtrip::reference(store, &p)?);
    let path = export::image_file(
        store,
        &card,
        string(draft, "exportFormat"),
        draft["fontSize"].as_f64().unwrap(),
        &home.join("Instant/Exports"),
    )?;
    cancel.check()?;
    Ok(json!({"card":card,"path":path}))
}

pub fn open(app: &tauri::AppHandle, project: Option<&Value>) -> Result<()> {
    let project = project.cloned();
    crate::lifecycle::on_main(app, move |app| {
        open_window(&app, project.as_ref()).map_err(err)
    })
}
fn open_window(app: &tauri::AppHandle, project: Option<&Value>) -> tauri::Result<()> {
    use tauri::Manager;
    if crate::lifecycle::quitting(app) {
        return Err(std::io::Error::other("Finish saving before opening another editor.").into());
    }
    let ident = project.map(|p| string(p, "id")).unwrap_or("");
    let label = if ident.is_empty() {
        "instant".into()
    } else {
        format!("instant-{ident}")
    };
    crate::lifecycle::cancel_close(app, &label);
    if let Some(window) = app.get_webview_window(&label) {
        window.show()?;
        window.unminimize()?;
        return window.set_focus();
    }
    let builder = tauri::WebviewWindowBuilder::new(
        app,
        &label,
        tauri::WebviewUrl::App(format!("index.html#instant?project={ident}").into()),
    )
    .title(if let Some(project) = project {
        format!(
            "FerMatter — {} Instant",
            Path::new(string(project, "root"))
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        )
    } else {
        "FerMatter — Instant".into()
    })
    .inner_size(780., 640.)
    .min_inner_size(480., 460.)
    .transparent(cfg!(target_os = "macos"))
    .disable_drag_drop_handler();
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    #[cfg(target_os = "macos")]
    let builder = if let Some(id) = crate::identity::isolated_store_id() {
        builder.data_store_identifier(id)
    } else {
        builder
    };
    let _window = builder.build()?;
    #[cfg(target_os = "macos")]
    crate::project_drop::install(&_window)?;
    #[cfg(target_os = "macos")]
    _window.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 0)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> Value {
        json!({"source":"\\frac{a}{b}","kind":"block","fontSize":24,"color":"#000000","exportFormat":"svg","template":{"id":"latex","name":"LaTeX","format":"latex","compiler":"pdflatex","source":"\\documentclass{article}\n\\begin{document}\n{{content}}\n\\end{document}"}})
    }
    #[test]
    fn multiline_input_is_one_item_and_keeps_custom_definitions() {
        let mut d = draft();
        d["source"] = json!("\\begin{aligned}a&=b\\\\c&=d\\end{aligned}");
        let (_, content, card) = source(&d).unwrap();
        assert_eq!(content.matches("\\[").count(), 1);
        assert_eq!(card["tex"], d["source"]);
        assert!(content.contains("\\AtBeginDocument{\\color[HTML]{000000}}"));
        d["template"]["source"] = json!("missing placeholder");
        assert!(source(&d).is_err());
    }
    #[test]
    fn history_deduplicates_and_invalid_files_are_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let d = draft();
        session(&store, &json!({"action":"save","draft":d})).unwrap();
        for _ in 0..2 {
            session(&store, &json!({"action":"remember","draft":d})).unwrap();
        }
        let state = session(&store, &json!({"action":"load"})).unwrap();
        assert_eq!(state["draft"]["source"], d["source"]);
        assert_eq!(state["history"].as_array().unwrap().len(), 1);
        let path = store.home.join("Instant/session.json");
        fs::write(&path, "broken history").unwrap();
        assert!(session(&store, &json!({"action":"save","draft":d})).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "broken history");
    }
    #[test]
    fn autosaves_reusable_items_without_export_and_keeps_their_identity() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let mut first = draft();
        first["entryID"] = json!("aaaa");
        for (source, color, size) in [("x", "#000000", 24), ("x^2", "#123456", 30)] {
            first["source"] = json!(source);
            first["color"] = json!(color);
            first["fontSize"] = json!(size);
            session(&store, &json!({"action":"save", "draft":first})).unwrap();
        }
        let mut second = first.clone();
        second["entryID"] = json!("bbbb");
        second["color"] = json!("#ffffff");
        session(&store, &json!({"action":"save", "draft":second})).unwrap();
        let reopened = Storage::new(store.home.clone(), temp.path().join("legacy")).unwrap();
        let state = session(&reopened, &json!({"action":"load"})).unwrap();
        assert_eq!(state["history"].as_array().unwrap().len(), 2);
        assert_eq!(state["history"][0]["draft"], second);
        assert_eq!(state["history"][1]["draft"], first);
        assert!(!store.home.join("Setup.json").exists());
        // Clearing an editor does not erase its last nonempty saved item.
        second["source"] = json!("");
        let state = session(&store, &json!({"action":"save", "draft":second})).unwrap();
        assert_eq!(state["history"].as_array().unwrap().len(), 2);
        assert_eq!(state["history"][0]["draft"]["source"], "x^2");
    }
    #[test]
    fn pending_import_survives_generic_saves_until_explicit_authorization() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let mut imported = draft();
        imported["entryID"] = json!("aaaa");
        imported["requiresRender"] = json!(true);
        imported["importAuthorization"] = json!("token");
        session(&store, &json!({"action":"save","draft":imported})).unwrap();
        let mut bypass = imported.clone();
        bypass["source"] = json!("edited");
        bypass["requiresRender"] = json!(false);
        bypass
            .as_object_mut()
            .unwrap()
            .remove("importAuthorization");
        let state = session(&store, &json!({"action":"save","draft":bypass})).unwrap();
        assert_eq!(state["draft"]["requiresRender"], true);
        assert_eq!(state["draft"]["importAuthorization"], "token");
        assert!(render(&store, &bypass, &compiler::Cancellation::default())
            .unwrap_err()
            .contains("Press Render"));
        let mut changed_id = state["draft"].clone();
        changed_id["entryID"] = json!("bbbb");
        changed_id["color"] = json!("#2563eb");
        changed_id["fontSize"] = json!(42);
        changed_id["requiresRender"] = json!(false);
        changed_id
            .as_object_mut()
            .unwrap()
            .remove("importAuthorization");
        assert!(
            render(&store, &changed_id, &compiler::Cancellation::default())
                .unwrap_err()
                .contains("Press Render")
        );

        let root = temp.path().join("project");
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("main.tex"),
            "\\documentclass{article}\\begin{document}x\\end{document}",
        )
        .unwrap();
        let project = store.open(&root, Some("main.tex")).unwrap();
        let project_id = string(&project, "id");
        session(
            &store,
            &json!({"project":project_id,"action":"save","draft":imported}),
        )
        .unwrap();
        assert!(render_project(
            &store,
            project_id,
            &bypass,
            &compiler::Cancellation::default(),
        )
        .unwrap_err()
        .contains("Press Render"));
        let mut project_changed_id = imported.clone();
        project_changed_id["entryID"] = json!("bbbb");
        project_changed_id["color"] = json!("#2563eb");
        project_changed_id["requiresRender"] = json!(false);
        project_changed_id
            .as_object_mut()
            .unwrap()
            .remove("importAuthorization");
        assert!(render_project(
            &store,
            project_id,
            &project_changed_id,
            &compiler::Cancellation::default(),
        )
        .unwrap_err()
        .contains("Press Render"));

        let mut authorize = state["draft"].clone();
        authorize["source"] = json!("edited");
        let state = session(
            &store,
            &json!({"action":"authorize_render","draft":authorize}),
        )
        .unwrap();
        assert_eq!(state["draft"]["requiresRender"], false);
        assert!(state["draft"].get("importAuthorization").is_none());

        let mut local = draft();
        local["entryID"] = json!("cccc");
        local["source"] = json!("new local source");
        assert!(pending_import(&state, &local).is_none());
    }
    #[test]
    fn legacy_pending_draft_without_token_is_authorized_once_and_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let mut legacy = draft();
        legacy["entryID"] = json!("aaaa");
        legacy["source"] = json!("legacy source");
        legacy["requiresRender"] = json!(true);
        let path = store.home.join("Instant/session.json");
        json_write(
            &path,
            &json!({"schemaVersion":1,"draft":legacy,"history":[{"id":"aaaa","draft":legacy}]}),
        )
        .unwrap();

        let loaded = session(&store, &json!({"action":"load"})).unwrap();
        assert_eq!(loaded["draft"]["source"], "legacy source");
        assert_eq!(loaded["draft"]["requiresRender"], true);
        let authorized = session(
            &store,
            &json!({"action":"authorize_render","draft":loaded["draft"]}),
        )
        .unwrap();
        assert_eq!(authorized["draft"]["source"], "legacy source");
        assert_eq!(authorized["draft"]["requiresRender"], false);
        assert_eq!(authorized["history"][0]["draft"], authorized["draft"]);
    }
    #[test]
    fn upgrades_old_history_once_without_dropping_styles_or_creating_duplicates() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let first = draft();
        let mut second = first.clone();
        second["color"] = json!("#ffffff");
        let ident = id(&first.to_string());
        let second_id = id(&second.to_string());
        let path = store.home.join("Instant/session.json");
        json_write(
            &path,
            &json!({"schemaVersion":1,"draft":first,"history":[
            {"id":ident,"draft":first},{"id":second_id,"draft":second}
        ], "unknown":"preserved"}),
        )
        .unwrap();
        for _ in 0..3 {
            let state = session(&store, &json!({"action":"load"})).unwrap();
            assert_eq!(state["history"].as_array().unwrap().len(), 2);
            assert_eq!(state["history"][0]["id"], ident);
            assert_eq!(state["draft"]["entryID"], ident);
            assert_eq!(state["history"][1]["draft"]["color"], "#ffffff");
            assert_eq!(state["unknown"], "preserved");
        }
    }
    #[test]
    fn rejects_unsafe_options_but_saves_unfinished_templates() {
        let mut d = draft();
        d["template"]["source"] = json!("still editing");
        assert!(validate(&d).is_ok());
        d["color"] = json!("#000000}\\input{other}");
        assert!(validate(&d).is_err());
        d = draft();
        d["fontSize"] = json!(-1);
        assert!(validate(&d).is_err());
    }
    #[test]
    fn project_drafts_and_history_are_separate_from_global_and_other_projects() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let mut ids = vec![String::new()];
        for name in ["one", "two"] {
            let root = temp.path().join(name);
            fs::create_dir_all(&root).unwrap();
            fs::write(
                root.join("main.tex"),
                "\\documentclass{article}\\begin{document}x\\end{document}",
            )
            .unwrap();
            ids.push(string(&store.open(&root, Some("main.tex")).unwrap(), "id").to_owned());
        }
        for (i, project) in ids.iter().enumerate() {
            let mut value = draft();
            value["source"] = json!(format!("x_{i}"));
            for action in ["save", "remember"] {
                session(
                    &store,
                    &json!({"project":project,"action":action,"draft":value}),
                )
                .unwrap();
            }
        }
        for (i, project) in ids.iter().enumerate() {
            let state = session(&store, &json!({"project":project,"action":"load"})).unwrap();
            assert_eq!(state["draft"]["source"], format!("x_{i}"));
            assert_eq!(state["history"].as_array().unwrap().len(), 1);
            assert_eq!(
                state["history"][0]["draft"]["source"],
                state["draft"]["source"]
            );
        }
        assert!(home(&store, "../other").is_err());
    }
}
