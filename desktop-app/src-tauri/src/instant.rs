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

pub fn session(store: &Storage, request: &Value) -> Result<Value> {
    let _lock = store.writes.lock().unwrap();
    let home = home(store, string(request, "project"))?;
    let mut state = load(&home)?;
    match string(request, "action") {
        "save" => {
            validate(&request["draft"])?;
            state["draft"] = request["draft"].clone();
        }
        "remember" => {
            let draft = &request["draft"];
            validate(draft)?;
            if !string(draft, "source").trim().is_empty() {
                let ident = id(&draft.to_string());
                let history = state["history"].as_array_mut().unwrap();
                history.retain(|v| v["id"] != ident);
                history.insert(0, json!({"id":ident,"draft":draft}));
            }
        }
        "load" => return Ok(state),
        _ => return Err("Unknown Instant history operation.".into()),
    }
    json_write(&home.join("Instant/session.json"), &state)?;
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

pub fn render(home: &Path, draft: &Value, cancel: &compiler::Cancellation) -> Result<Value> {
    if draft["requiresRender"] == true {
        return Err("Press Render to compile imported source.".into());
    }
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
    if draft["requiresRender"] == true {
        return Err("Press Render to compile imported source.".into());
    }
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
            "Mathlidelux — {} Instant",
            Path::new(string(project, "root"))
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        )
    } else {
        "Mathlidelux — Instant".into()
    })
    .inner_size(780., 640.)
    .min_inner_size(480., 460.)
    .transparent(true)
    .disable_drag_drop_handler();
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    let window = builder.build()?;
    #[cfg(target_os = "macos")]
    crate::project_drop::install(&window)?;
    window.set_background_color(Some(tauri::utils::config::Color(0, 0, 0, 0)))?;
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
        assert_eq!(state["draft"], d);
        assert_eq!(state["history"].as_array().unwrap().len(), 1);
        let path = store.home.join("Instant/session.json");
        fs::write(&path, "broken history").unwrap();
        assert!(session(&store, &json!({"action":"save","draft":d})).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "broken history");
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
