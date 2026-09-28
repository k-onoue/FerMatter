use crate::{files::*, scan};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
pub struct SettingsFiles {
    dir: PathBuf,
    revisions: Mutex<HashMap<String, String>>,
}
impl SettingsFiles {
    pub fn new(home: &Path) -> Self {
        Self {
            dir: home.join("Settings"),
            revisions: Mutex::new(HashMap::new()),
        }
    }
    pub fn initialize(&self, name: &str, value: &Value) -> Result<()> {
        valid_name(name)?;
        if !self.dir.join(name).exists() && !self.dir.join(format!(".last-valid-{name}")).exists() {
            validate(name, value)?;
            json_write(&self.dir.join(name), value)?;
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    pub fn import_swift_preferences(&self, path: &Path) -> Result<()> {
        if self.dir.join("preferences.json").exists()
            || self.dir.join(".last-valid-preferences.json").exists()
            || !path.is_file()
        {
            return Ok(());
        }
        let old = plist::Value::from_file(path).map_err(err)?;
        let mut prefs = default_document("preferences.json");
        if let Some(v) = old.as_dictionary() {
            if let Some(appearance) = v.get("appearance").and_then(plist::Value::as_string) {
                if ["system", "light", "dark"].contains(&appearance) {
                    prefs["appearance"] = json!(appearance);
                }
            }
            if let Some(opacity) = v.get("opacity").and_then(plist::Value::as_real) {
                prefs["opacity"] = json!(opacity.clamp(0.45, 1.));
            }
            prefs["autoSave"] =
                json!(v.get("sourceSaveMode").and_then(plist::Value::as_string) != Some("manual"));
            prefs["pinned"] = json!(v
                .get("pinned")
                .and_then(plist::Value::as_boolean)
                .unwrap_or(false));
        }
        validate("preferences.json", &prefs)?;
        json_write(&self.dir.join("preferences.json"), &prefs)
    }
    pub fn load(&self, name: &str) -> Result<Value> {
        valid_name(name)?;
        let path = self.dir.join(name);
        let backup = self.dir.join(format!(".last-valid-{name}"));
        let value = match json_read(&path).and_then(|v| {
            validate(name, &v)?;
            Ok(v)
        }) {
            Ok(v) => v,
            Err(_) => match json_read(&backup).and_then(|v| {
                validate(name, &v)?;
                Ok(v)
            }) {
                Ok(v) => v,
                Err(_) => default_document(name),
            },
        };
        if !path.exists() {
            json_write(&path, &value)?
        }
        json_write(&backup, &value)?;
        self.revisions
            .lock()
            .unwrap()
            .insert(name.into(), hash(fs::read(&path).map_err(err)?));
        Ok(value)
    }
    pub fn reload(&self, name: &str) -> Result<Value> {
        valid_name(name)?;
        let path = self.dir.join(name);
        let value = json_read(&path)?;
        validate(name, &value)?;
        json_write(&self.dir.join(format!(".last-valid-{name}")), &value)?;
        self.revisions
            .lock()
            .unwrap()
            .insert(name.into(), hash(fs::read(&path).map_err(err)?));
        Ok(value)
    }
    pub fn save(&self, name: &str, value: &Value) -> Result<()> {
        valid_name(name)?;
        validate(name, value)?;
        if serde_json::to_vec(value).map_err(err)?.len() > 1_000_000 {
            return Err("Settings exceed 1 MB.".into());
        }
        let path = self.dir.join(name);
        let mut revisions = self.revisions.lock().unwrap();
        let actual = fs::read(&path).map(hash).unwrap_or_default();
        if revisions
            .get(name)
            .is_some_and(|revision| *revision != actual)
        {
            return Err("Settings changed on disk. Reload the file before saving.".into());
        }
        json_write(&path, value)?;
        json_write(&self.dir.join(format!(".last-valid-{name}")), value)?;
        revisions.insert(name.into(), hash(fs::read(path).map_err(err)?));
        Ok(())
    }
}
fn valid_name(name: &str) -> Result<()> {
    if [
        "preferences.json",
        "theme.json",
        "keybindings.json",
        "instant.json",
    ]
    .contains(&name)
    {
        Ok(())
    } else {
        Err("Unknown settings file".into())
    }
}
fn default_document(name: &str) -> Value {
    match name {
        "instant.json" => crate::instant_settings::defaults(),
        "theme.json" => {
            json!({"schemaVersion":1,"kind":"mathlidelux.theme","id":"org.mathlidelux.default","name":"Default","light":{},"dark":{}})
        }
        "keybindings.json" => {
            json!({"schemaVersion":1,"kind":"mathlidelux.keybindings","bindings":{}})
        }
        _ => {
            json!({"appearance":"dark","autoSave":true,"opacity":0.88,"pdfWidth":420,"projectView":"grid"})
        }
    }
}
pub fn validate(name: &str, v: &Value) -> Result<()> {
    if !v.is_object() {
        return Err("Settings must be a JSON object.".into());
    }
    match name {
        "instant.json" => crate::instant_settings::validate(v),
        "theme.json" => theme(v),
        "keybindings.json" => keybindings(v),
        "preferences.json" => {
            if let Some(path) = v.get("projectDirectory") {
                crate::project_location::validate_path(
                    path.as_str()
                        .ok_or("Project folder must be a path string.")?,
                )?;
            }
            if let Some(view) = v.get("projectView") {
                if !matches!(view.as_str(), Some("grid" | "list")) {
                    return Err("Project view must be grid or list.".into());
                }
            }
            if let Some(opacity) = v.get("opacity") {
                if !opacity.as_f64().is_some_and(|n| (0.45..=1.).contains(&n)) {
                    return Err("Background opacity must be between 0.45 and 1.".into());
                }
            }
            if !["system", "light", "dark"].contains(&string(v, "appearance")) {
                return Err("Invalid appearance".into());
            }
            if !v["autoSave"].is_boolean() {
                return Err("Auto Save must be a boolean.".into());
            }
            Ok(())
        }
        _ => Err("Unknown settings file".into()),
    }
}
fn theme(v: &Value) -> Result<()> {
    if v["schemaVersion"] != 1 || v["kind"] != "mathlidelux.theme" {
        return Err("Unsupported theme format".into());
    }
    if !scan::re(r"^[a-z0-9]+(?:[.-][a-z0-9]+)+$").is_match(string(v, "id"))
        || string(v, "id").len() > 120
        || string(v, "name").is_empty()
        || string(v, "name").chars().count() > 80
    {
        return Err("Invalid theme ID or name.".into());
    }
    for mode in ["light", "dark"] {
        let colors = v[mode]
            .as_object()
            .ok_or("Theme needs light and dark palettes")?;
        for (key, color) in colors {
            if ![
                "accent",
                "windowBackground",
                "surfaceBackground",
                "text",
                "command",
                "delimiter",
                "number",
                "comment",
            ]
            .contains(&key.as_str())
            {
                return Err(format!("Unknown theme token: {key}"));
            }
            let color = color.as_str().ok_or("Invalid color")?;
            if color != "system" && !scan::re(r"^#[0-9a-fA-F]{6}$").is_match(color) {
                return Err("Use opaque #RRGGBB colors.".into());
            }
        }
    }
    Ok(())
}
fn default_bindings() -> HashMap<&'static str, Value> {
    let mut map = HashMap::new();
    for (id, key, shift, option, control, command) in [
        ("open", "o", false, false, false, true),
        ("projects", "o", true, false, false, true),
        ("newProject", "n", true, false, false, true),
        ("instant", "k", true, false, false, true),
        ("newEquation", "n", false, false, false, true),
        ("newFigure", "n", false, true, false, true),
        ("find", "f", false, false, false, true),
        ("edit", "e", false, false, false, true),
        ("duplicate", "d", false, false, false, true),
        ("saveEquation", "s", false, false, false, true),
        ("copySVG", "c", true, false, false, true),
        ("exportSVG", "s", true, false, false, true),
        ("rescan", "r", false, false, false, true),
        ("togglePDF", "p", false, true, false, true),
        ("rebuildPDF", "r", true, false, false, true),
        ("previousPage", "[", false, false, false, true),
        ("nextPage", "]", false, false, false, true),
        ("zoomIn", "=", false, false, true, true),
        ("zoomOut", "-", false, false, true, true),
        ("fitPDF", "0", false, false, true, true),
        ("zoomWindowIn", "=", false, false, false, true),
        ("zoomWindowOut", "-", false, false, false, true),
        ("resetWindowZoom", "0", false, false, false, true),
        ("inspector", "i", false, true, false, true),
        ("pin", "p", true, false, false, true),
        ("fullScreen", "f", false, false, true, true),
        ("projectSource", "e", false, true, false, true),
        ("nextSourceTab", "\t", false, false, true, false),
        ("previousSourceTab", "\t", true, false, true, false),
        ("splitSource", "\\", false, false, false, true),
        ("splitSourceDown", "\\", false, true, false, true),
    ] {
        map.insert(
            id,
            json!({"key":key,"command":command,"control":control,"shift":shift,"option":option}),
        );
    }
    map
}
fn keybindings(v: &Value) -> Result<()> {
    if v["schemaVersion"] != 1 || v["kind"] != "mathlidelux.keybindings" {
        return Err("Unsupported shortcuts format".into());
    }
    let bindings = v["bindings"].as_object().ok_or("Missing bindings")?;
    let mut all = default_bindings();
    for (id, binding) in bindings {
        if !all.contains_key(id.as_str()) {
            return Err(format!("Unknown command: {id}"));
        }
        *all.get_mut(id.as_str()).unwrap() = binding.clone()
    }
    // Keep stored overrides usable when new defaults share their keys.
    // Only implicit defaults yield; explicit duplicates are still rejected.
    for id in [
        "instant",
        "zoomIn",
        "zoomOut",
        "fitPDF",
        "zoomWindowIn",
        "zoomWindowOut",
        "resetWindowZoom",
    ] {
        if !bindings.contains_key(id)
            && bindings.values().any(|saved| {
                ["key", "command", "control", "shift", "option"]
                    .iter()
                    .all(|field| saved[field] == all[id][field])
            })
        {
            all.get_mut(id).unwrap()["key"] = json!("");
        }
    }
    let mut seen = vec![];
    for (id, b) in all {
        let key = b["key"].as_str().ok_or("Shortcut key must be a string")?;
        if ["command", "control", "shift", "option"]
            .iter()
            .any(|key| !b[key].is_boolean())
        {
            return Err("Shortcut modifiers must be boolean.".into());
        }
        if key.is_empty() {
            continue;
        }
        let command = b["command"] == true;
        let shift = b["shift"] == true;
        let option = b["option"] == true;
        let control = b["control"] == true;
        if key.chars().count() != 1
            || !"abcdefghijklmnopqrstuvwxyz0123456789`-=[]\\;',./\t\u{f700}\u{f701}\u{f702}\u{f703}"
                .contains(key)
            || !command && !control
        {
            return Err("Use Command or Control with a letter, number, symbol, or arrow.".into());
        }
        if command
            && !control
            && !option
            && !shift
            && "qwmh,cvxazsp \t".contains(key)
            && !(id == "saveEquation" && key == "s")
            || command && !control && !option && shift && key == "z"
            || command && !control && option && !shift && key == "h"
        {
            return Err("Shortcut is reserved for macOS or text editing.".into());
        }
        let key = json!([key, command, control, shift, option]);
        if seen.contains(&key) {
            return Err(format!("Duplicate shortcut: {id}"));
        }
        seen.push(key);
    }
    Ok(())
}

pub fn validate_templates(value: &Value) -> Result<()> {
    let templates = value["custom"]
        .as_array()
        .ok_or("Templates require a custom list")?;
    if value["defaultID"].as_str().is_none() {
        return Err("Missing default template".into());
    }
    let mut ids = std::collections::HashSet::new();
    for template in templates {
        if string(template, "id").is_empty()
            || !ids.insert(string(template, "id"))
            || string(template, "id").starts_with("builtin-")
        {
            return Err("Invalid custom template identity".into());
        }
        if string(template, "name").trim().is_empty() {
            return Err("Enter a template name.".into());
        }
        if string(template, "source").matches("{{content}}").count() != 1 {
            return Err("Place {{content}} exactly once in the template.".into());
        }
        if !["latex", "typst"].contains(&string(template, "format")) {
            return Err("Unknown template format".into());
        }
        if string(template, "format") == "latex"
            && !["pdflatex", "xelatex", "lualatex"].contains(&string(template, "compiler"))
        {
            return Err("Choose a LaTeX compiler".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    #[test]
    fn swift_preferences_import_preserves_preview_settings() {
        let dir = tempfile::tempdir().unwrap();
        let legacy = dir.path().join("legacy.plist");
        let mut dict = plist::Dictionary::new();
        dict.insert("appearance".into(), plist::Value::String("light".into()));
        dict.insert("opacity".into(), plist::Value::Real(0.72));
        dict.insert(
            "sourceSaveMode".into(),
            plist::Value::String("manual".into()),
        );
        dict.insert("pinned".into(), plist::Value::Boolean(true));
        plist::Value::Dictionary(dict).to_file_xml(&legacy).unwrap();
        let settings = SettingsFiles::new(dir.path());
        settings.import_swift_preferences(&legacy).unwrap();
        let prefs = settings.load("preferences.json").unwrap();
        assert_eq!(prefs["appearance"], "light");
        assert_eq!(prefs["opacity"], 0.72);
        assert_eq!(prefs["autoSave"], false);
        assert_eq!(prefs["pinned"], true);
        let changed = json!({"appearance":"dark","opacity":0.9,"autoSave":true});
        settings.save("preferences.json", &changed).unwrap();
        settings.import_swift_preferences(&legacy).unwrap();
        assert_eq!(settings.load("preferences.json").unwrap(), changed);
        assert!(legacy.exists());
    }
    #[test]
    fn preserves_extension_and_recovers_invalid_theme() {
        let d = tempfile::tempdir().unwrap();
        let settings = SettingsFiles::new(d.path());
        let mut theme = settings.load("theme.json").unwrap();
        theme["extensions"] = json!({"community.example":{"author":"Example"}});
        settings.save("theme.json", &theme).unwrap();
        fs::write(d.path().join("Settings/theme.json"), "{").unwrap();
        assert!(settings.save("theme.json", &theme).is_err());
        assert_eq!(
            SettingsFiles::new(d.path()).load("theme.json").unwrap(),
            theme
        );
    }
    #[test]
    fn validates_combined_shortcuts() {
        let mut v = default_document("keybindings.json");
        assert!(keybindings(&v).is_ok());
        v["bindings"]["open"] =
            json!({"key":"z","command":true,"control":false,"option":false,"shift":false});
        assert!(keybindings(&v).is_err());
        v["bindings"]["open"]["key"] = json!("r");
        assert!(keybindings(&v).is_err());
        v["bindings"]["rescan"] =
            json!({"key":"o","command":true,"control":false,"option":false,"shift":false});
        assert!(keybindings(&v).is_ok());
    }
    #[test]
    fn preserves_legacy_zoom_overrides_and_rejects_explicit_duplicates() {
        let d = tempfile::tempdir().unwrap();
        let settings = SettingsFiles::new(d.path());
        let mut v = settings.load("keybindings.json").unwrap();
        v["bindings"]["zoomOut"] =
            json!({"key":"-","command":true,"control":false,"option":false,"shift":false});
        v["bindings"]["fitPDF"] =
            json!({"key":"0","command":true,"control":false,"option":false,"shift":false});
        v["bindings"]["open"] =
            json!({"key":"=","command":true,"control":true,"option":false,"shift":false});
        v["extensions"] = json!({"community.example":{"note":"keep me"}});
        settings.save("keybindings.json", &v).unwrap();
        assert_eq!(
            SettingsFiles::new(d.path())
                .load("keybindings.json")
                .unwrap(),
            v
        );
        v["bindings"]["zoomWindowOut"] = v["bindings"]["zoomOut"].clone();
        assert!(keybindings(&v).is_err());
    }
}
