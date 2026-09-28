//! Project-free scratch work. Drafts/history are durable; compiler inputs are disposable.
use crate::{compiler, export, files::*, storage::Storage};
use serde_json::{json, Value};
use std::{fs, path::Path};

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
    let mut state = load(&store.home)?;
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
    json_write(&store.home.join("Instant/session.json"), &state)?;
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
    let card = json!({"id":"instant","name":"Instant","tex":draft["source"],"kind":draft["kind"],"origin":"custom","sourceOrder":0,"status":"pending","typstContext":[{"file":main,"offset":prefix.chars().count()}]});
    let body = if typst {
        compiler::typst_body(&card)?
    } else {
        compiler::tex_body(&card)
    };
    Ok((main.into(), format!("{prefix}\n{body}\n{after}"), card))
}

pub fn render(home: &Path, draft: &Value, cancel: &compiler::Cancellation) -> Result<Value> {
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
    let path = export::image_file(
        &card,
        string(draft, "exportFormat"),
        draft["fontSize"].as_f64().unwrap(),
        &home.join("Instant/Exports"),
    )?;
    cancel.check()?;
    Ok(json!({"card":card,"path":path}))
}

pub fn open(app: &tauri::AppHandle) -> tauri::Result<()> {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("instant") {
        window.show()?;
        window.unminimize()?;
        return window.set_focus();
    }
    let builder = tauri::WebviewWindowBuilder::new(
        app,
        "instant",
        tauri::WebviewUrl::App("index.html#instant".into()),
    )
    .title("Mathlidelux — Instant")
    .inner_size(780., 640.)
    .min_inner_size(480., 460.)
    .transparent(true)
    .disable_drag_drop_handler();
    #[cfg(target_os = "macos")]
    let builder = builder
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true);
    let window = builder.build()?;
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
}
