//! Portable SVG recipes. Never execute imported code or follow embedded paths.
use crate::{files::*, instant, storage::Storage};
use serde_json::{json, Value};
use std::{fs, io::Read, path::Path};

pub const SVG_LIMIT: u64 = 16 * 1024 * 1024;
const RECIPE_LIMIT: usize = 2 * 1024 * 1024;
const NAMESPACE: &str = "https://mathlidelux.app/ns/recipe/1";
const IDENTITY: &str = ".mathlidelux-project.json";

pub fn read_svg(path: &Path) -> Result<String> {
    if !path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"))
    {
        return Err("Choose an SVG exported by Mathlidelux.".into());
    }
    let file = fs::File::open(path).map_err(err)?;
    if !file.metadata().map_err(err)?.is_file() {
        return Err("Choose an SVG file.".into());
    }
    let mut text = String::new();
    file.take(SVG_LIMIT + 1)
        .read_to_string(&mut text)
        .map_err(err)?;
    if text.len() as u64 > SVG_LIMIT {
        return Err("SVG exceeds 16 MB.".into());
    }
    Ok(text)
}

pub fn project_uuid(root: &Path, create: bool) -> Result<Option<String>> {
    let path = root.join(IDENTITY);
    if path.exists() {
        let metadata = fs::symlink_metadata(&path).map_err(err)?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > 4096 {
            return Err("Invalid Mathlidelux project identity; the file was preserved.".into());
        }
        let value = json_read(&path)?;
        let uuid = string(&value, "uuid");
        if value["schemaVersion"] != 1
            || value["kind"] != "mathlidelux.project"
            || uuid::Uuid::parse_str(uuid).is_err()
        {
            return Err("Invalid Mathlidelux project identity; the file was preserved.".into());
        }
        return Ok(Some(uuid.to_owned()));
    }
    if !create {
        return Ok(None);
    }
    let uuid = uuid::Uuid::new_v4().to_string();
    json_write(
        &path,
        &json!({"schemaVersion":1,"kind":"mathlidelux.project","uuid":uuid}),
    )?;
    Ok(Some(uuid))
}

pub fn inherit_identity(source: &Path, target: &Path) -> Result<()> {
    if let Some(uuid) = project_uuid(source, false)? {
        json_write(
            &target.join(IDENTITY),
            &json!({"schemaVersion":1,"kind":"mathlidelux.project","uuid":uuid}),
        )?;
    }
    Ok(())
}

pub fn reset_identity(root: &Path) -> Result<()> {
    let path = root.join(IDENTITY);
    if path.exists() {
        fs::remove_file(&path).map_err(err)?;
    }
    project_uuid(root, true)?;
    Ok(())
}

fn relative(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains(['\\', ':', '\0'])
        && path
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != "..")
}

fn validate(value: &Value) -> Result<()> {
    if value["schemaVersion"] != 1 || value["kind"] != "mathlidelux.svg" {
        return Err("This SVG uses an unsupported Mathlidelux recipe version.".into());
    }
    instant::validate(&value["draft"])?;
    let template = &value["draft"]["template"];
    if ["id", "name", "source"]
        .iter()
        .any(|key| !template[key].is_string())
        || string(template, "id").len() > 512
        || string(template, "name").len() > 512
        || string(template, "source").matches("{{content}}").count() != 1
    {
        return Err("Invalid SVG template.".into());
    }
    if let Some(path) = value["draft"]["contextFile"].as_str() {
        if !relative(path) {
            return Err("Invalid relative context file.".into());
        }
    }
    if !value["project"].is_null()
        && (uuid::Uuid::parse_str(string(&value["project"], "uuid")).is_err()
            || string(&value["project"], "name").len() > 512)
    {
        return Err("Invalid SVG project identity.".into());
    }
    Ok(())
}

pub fn decode(svg: &str) -> Result<Value> {
    if svg.len() as u64 > SVG_LIMIT {
        return Err("SVG exceeds 16 MB.".into());
    }
    // roxmltree rejects DTDs by default and never resolves external entities.
    let doc = roxmltree::Document::parse(svg).map_err(err)?;
    if !doc
        .root_element()
        .has_tag_name(("http://www.w3.org/2000/svg", "svg"))
    {
        return Err("Invalid SVG document.".into());
    }
    let mut recipes = doc.descendants().filter(|node| {
        node.has_tag_name((NAMESPACE, "recipe"))
            && node
                .parent()
                .is_some_and(|p| p.has_tag_name(("http://www.w3.org/2000/svg", "metadata")))
    });
    let node = recipes.next().ok_or(
        "This SVG has no editable Mathlidelux source. Use the original SVG or Instant history.",
    )?;
    if recipes.next().is_some() {
        return Err("SVG contains more than one recipe.".into());
    }
    let text = node.text().ok_or("Empty SVG recipe")?;
    if text.len() > RECIPE_LIMIT {
        return Err("SVG recipe exceeds 2 MB.".into());
    }
    let value: Value = serde_json::from_str(text).map_err(err)?;
    validate(&value)?;
    Ok(value)
}

pub fn embed(svg: &str, value: &Value) -> Result<String> {
    validate(value)?;
    let text = serde_json::to_string(value).map_err(err)?;
    if text.len() > RECIPE_LIMIT {
        return Err("SVG recipe exceeds 2 MB.".into());
    }
    let escaped = text
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;");
    let end = svg.rfind("</svg>").ok_or("Invalid SVG document")?;
    Ok(format!(
        "{}<metadata><ml:recipe xmlns:ml=\"{NAMESPACE}\">{escaped}</ml:recipe></metadata>{}",
        &svg[..end],
        &svg[end..]
    ))
}

pub fn recipe(draft: &Value, project: Value) -> Value {
    let mut draft = draft.clone();
    if let Some(object) = draft.as_object_mut() {
        object.remove("requiresRender");
        object.remove("restorationProject");
    }
    json!({"schemaVersion":1,"kind":"mathlidelux.svg","draft":draft,"project":project})
}

pub fn reference(store: &Storage, project: &Value) -> Result<Value> {
    let _guard = store.writes.lock().unwrap();
    let root = store.root(string(project, "id"))?;
    let uuid = match project_uuid(&root, true) {
        Ok(uuid) => uuid,
        Err(_) if !root.join(IDENTITY).exists() => return Ok(Value::Null),
        Err(error) => return Err(error),
    };
    Ok(json!({"uuid":uuid,"name":root.file_name().unwrap_or_default().to_string_lossy()}))
}

pub fn fallback_template(project: &Value, root: &Path) -> Value {
    let templates: Value =
        serde_json::from_str(include_str!("../../src/template-defaults.json")).unwrap();
    let mut template = templates
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["format"] == project["format"])
        .unwrap()
        .clone();
    // Only the LaTeX preamble is embedded, never the document body. Typst
    // definitions can contain arbitrary prose/imports, so use the base template.
    if project["format"] == "latex" {
        if let Ok(text) = safe(root, string(project, "main")).and_then(|p| read(&p)) {
            if let Some(mark) =
                crate::scan::re(r"\\begin\s*\{document\}").find(&crate::scan::uncomment(&text))
            {
                if mark.start() < 1_000_000 {
                    template["source"] = json!(format!(
                        "{}\n\\begin{{document}}\n{{{{content}}}}\n\\end{{document}}",
                        &text[..mark.start()]
                    ));
                    template["id"] = json!("restored");
                    template["name"] = json!("Restored template");
                }
            }
        }
        if ["pdflatex", "xelatex", "lualatex"].contains(&string(project, "engine")) {
            template["compiler"] = project["engine"].clone();
        }
    }
    template
}

pub fn card_recipe(store: &Storage, p: &Value, card: &Value, size: f64) -> Result<Value> {
    let root = store.root(string(p, "id"))?;
    let draft = json!({"source":card["tex"],"kind":card["kind"],
        "template":fallback_template(p,&root),"fontSize":if size > 0. {size} else {card["sourceFontSize"].as_f64().unwrap_or(11.)},
        "color":"#000000","exportFormat":"svg","contextFile":card.get("file").unwrap_or(&p["main"])});
    let value = recipe(&draft, reference(store, p)?);
    validate(&value)?;
    Ok(value)
}

pub fn resolve(store: &Storage, value: &Value) -> Result<(Option<Value>, Value)> {
    validate(value)?;
    let uuid = string(&value["project"], "uuid");
    let mut matches = vec![];
    if !uuid.is_empty() {
        for p in store.recent()? {
            let root = store.root(string(&p, "id"))?;
            if p["format"] == value["draft"]["template"]["format"]
                && project_uuid(&root, false).ok().flatten().as_deref() == Some(uuid)
            {
                matches.push(p);
            }
        }
    }
    let project = if matches.len() == 1 {
        matches.pop()
    } else {
        None
    };
    let mut draft = value["draft"].clone();
    draft["requiresRender"] = json!(true);
    if let Some(p) = &project {
        let root = store.root(string(p, "id"))?;
        let context = string(&draft, "contextFile");
        if context.is_empty() || safe(&root, context).is_err() || !root.join(context).is_file() {
            draft["contextFile"] = p["main"].clone();
        }
    } else {
        draft.as_object_mut().unwrap().remove("contextFile");
        draft["restorationProject"] = value["project"].clone();
    }
    Ok((project, draft))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft() -> Value {
        json!({"source":"a < b & x_{日}","kind":"block","fontSize":24,"color":"#123456","exportFormat":"svg","template":{"id":"latex","name":"LaTeX","format":"latex","compiler":"pdflatex","source":"\\documentclass{article}\n\\begin{document}\n{{content}}\n\\end{document}"}})
    }
    #[test]
    fn svg_preserves_source_settings_and_rejects_invalid_recipes() {
        let value = recipe(&draft(), Value::Null);
        let svg = embed(
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><path d=\"M0 0\"/></svg>",
            &value,
        )
        .unwrap();
        assert_eq!(decode(&svg).unwrap(), value);
        assert!(decode(&svg.replace("&lt;", "&unknown;")).is_err());
        assert!(decode("<svg xmlns=\"http://www.w3.org/2000/svg\"/>").is_err());
        assert!(decode(&embed(&svg, &value).unwrap()).is_err());
        let mut bad = value.clone();
        bad["schemaVersion"] = json!(2);
        assert!(embed(&svg, &bad).is_err());
        bad = value;
        bad["draft"]["contextFile"] = json!("../../private.tex");
        assert!(embed(&svg, &bad).is_err());
    }
    #[test]
    fn identity_survives_move_and_duplicates_are_independent() {
        let dir = tempfile::tempdir().unwrap();
        let first = dir.path().join("first");
        fs::create_dir(&first).unwrap();
        let uuid = project_uuid(&first, true).unwrap();
        let moved = dir.path().join("moved");
        fs::rename(first, &moved).unwrap();
        assert_eq!(project_uuid(&moved, false).unwrap(), uuid);
        reset_identity(&moved).unwrap();
        assert_ne!(project_uuid(&moved, false).unwrap(), uuid);
    }
    #[test]
    fn fallback_never_runs_imported_source() {
        let dir = tempfile::tempdir().unwrap();
        let store = Storage::new(dir.path().join("data"), dir.path().join("legacy")).unwrap();
        let (_, value) = resolve(&store, &recipe(&draft(), Value::Null)).unwrap();
        assert_eq!(value["requiresRender"], true);
        assert_eq!(value["source"], draft()["source"]);
    }

    #[test]
    fn registered_project_restores_locally_and_ambiguous_copy_falls_back() {
        let dir = tempfile::tempdir().unwrap();
        let store = Storage::new(dir.path().join("data"), dir.path().join("legacy")).unwrap();
        let root = dir.path().join("project");
        fs::create_dir(&root).unwrap();
        fs::write(
            root.join("main.tex"),
            "\\documentclass{article}\\begin{document}$x$\\end{document}",
        )
        .unwrap();
        let p = store.open(&root, Some("main.tex")).unwrap();
        let reference = reference(&store, &p).unwrap();
        let value = recipe(&draft(), reference);
        let (resolved, restored) = resolve(&store, &value).unwrap();
        assert_eq!(resolved.unwrap()["id"], p["id"]);
        assert_eq!(restored["contextFile"], "main.tex");
        let other = dir.path().join("other");
        fs::create_dir(&other).unwrap();
        fs::copy(root.join("main.tex"), other.join("main.tex")).unwrap();
        fs::copy(root.join(IDENTITY), other.join(IDENTITY)).unwrap();
        store.open(&other, Some("main.tex")).unwrap();
        let (resolved, restored) = resolve(&store, &value).unwrap();
        assert!(resolved.is_none());
        assert_eq!(restored["restorationProject"], value["project"]);
        reset_identity(&other).unwrap();
        assert!(resolve(&store, &value).unwrap().0.is_some());
    }
    #[test]
    fn embeds_only_the_preamble_and_keeps_broken_identity() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.tex"),"\\documentclass{article}\n% begin comment\n\\newcommand{\\R}{R}\n\\begin{document}\nPRIVATE BODY\n\\end{document}").unwrap();
        let p = json!({"main":"main.tex","format":"latex","engine":"lualatex"});
        let template = fallback_template(&p, dir.path());
        assert!(string(&template, "source").contains("\\newcommand"));
        assert!(!string(&template, "source").contains("PRIVATE BODY"));
        assert_eq!(template["compiler"], "lualatex");
        fs::write(dir.path().join(IDENTITY), "broken").unwrap();
        assert!(project_uuid(dir.path(), true).is_err());
        assert_eq!(
            fs::read_to_string(dir.path().join(IDENTITY)).unwrap(),
            "broken"
        );
    }
}
