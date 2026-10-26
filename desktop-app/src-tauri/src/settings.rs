use crate::{files::*, scan};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};
pub struct SettingsFiles {
    dir: PathBuf,
    writes: Mutex<()>,
}
impl SettingsFiles {
    pub fn new(home: &Path) -> Self {
        Self {
            dir: home.join("Settings"),
            writes: Mutex::new(()),
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
        Ok(self.snapshot(name, false)?["value"].clone())
    }
    /// Return the revision of exactly the bytes read, never a shared caller revision.
    pub fn snapshot(&self, name: &str, strict: bool) -> Result<Value> {
        let _guard = self.writes.lock().unwrap();
        valid_name(name)?;
        let path = self.dir.join(name);
        let backup = self.dir.join(format!(".last-valid-{name}"));
        let bytes = match read_document(&path) {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(_) => {
                return Err(format!(
                    "Cannot read settings file {}. It has been kept.",
                    path.display()
                ))
            }
        };
        let parse = |bytes: &[u8]| -> Result<Value> {
            if bytes.len() > 1_000_000 {
                return Err("Settings exceed 1 MB.".into());
            }
            let value: Value =
                serde_json::from_slice(bytes).map_err(|_| "Invalid settings JSON.".to_string())?;
            validate(name, &value)?;
            Ok(value)
        };
        let current = bytes
            .as_deref()
            .ok_or_else(|| "Settings file is missing.".to_string())
            .and_then(parse);
        let valid = current.is_ok();
        let value = match current {
            Ok(value) => value,
            Err(error) if strict => return Err(error),
            Err(_) => read_document(&backup)
                .ok()
                .and_then(|b| parse(&b).ok())
                .unwrap_or_else(|| default_document(name)),
        };
        let revision = if let Some(bytes) = bytes {
            // An invalid active document can supply fallback values, but never a
            // writable revision. Reload must succeed before replacing the original.
            if valid {
                hash(bytes)
            } else {
                String::new()
            }
        } else {
            let bytes = serde_json::to_vec_pretty(&value).map_err(err)?;
            atomic(&path, &bytes)?;
            hash(bytes)
        };
        json_write(&backup, &value)?;
        Ok(json!({"value":value,"revision":revision}))
    }
    #[cfg(test)]
    pub fn reload(&self, name: &str) -> Result<Value> {
        Ok(self.snapshot(name, true)?["value"].clone())
    }
    pub fn save(&self, name: &str, value: &Value, expected: &str) -> Result<String> {
        let _guard = self.writes.lock().unwrap();
        valid_name(name)?;
        validate(name, value)?;
        if serde_json::to_vec(value).map_err(err)?.len() > 1_000_000 {
            return Err("Settings exceed 1 MB.".into());
        }
        let path = self.dir.join(name);
        let actual = read_document(&path).map(hash).unwrap_or_default();
        if expected.is_empty() || expected != actual {
            return Err("Settings changed on disk. Reload the file before saving.".into());
        }
        let bytes = serde_json::to_vec_pretty(value).map_err(err)?;
        atomic(&path, &bytes)?;
        atomic(&self.dir.join(format!(".last-valid-{name}")), &bytes)?;
        Ok(hash(bytes))
    }
}
fn read_document(path: &Path) -> std::io::Result<Vec<u8>> {
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x0020_0000);
    }
    let file = options.open(path)?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if file.metadata()?.file_attributes() & 0x400 != 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Settings cannot be symbolic links or reparse points.",
            ));
        }
    }
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Settings must be a regular file.",
        ));
    }
    let mut bytes = Vec::new();
    file.take(1_000_001).read_to_end(&mut bytes)?;
    if bytes.len() > 1_000_000 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "Settings exceed 1 MB.",
        ));
    }
    Ok(bytes)
}

fn valid_name(name: &str) -> Result<()> {
    if [
        "preferences.json",
        "theme.json",
        "keybindings.json",
        "instant.json",
        "tools.json",
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
        "tools.json" => crate::toolchain::defaults(),
        "instant.json" => crate::instant_settings::defaults(),
        "theme.json" => {
            json!({"schemaVersion":1,"kind":"mathlidelux.theme","id":"org.mathlidelux.default","name":"Default","light":{},"dark":{}})
        }
        "keybindings.json" => {
            json!({"schemaVersion":1,"kind":"mathlidelux.keybindings","bindings":{}})
        }
        _ => {
            json!({"appearance":"dark","autoSave":true,"opacity":0.88,"pdfWidth":420,"projectView":"grid","exportFontSize":30})
        }
    }
}
pub fn validate(name: &str, v: &Value) -> Result<()> {
    if !v.is_object() {
        return Err("Settings must be a JSON object.".into());
    }
    match name {
        "tools.json" => crate::toolchain::validate(v),
        "instant.json" => crate::instant_settings::validate(v),
        "theme.json" => theme(v),
        "keybindings.json" => keybindings(v),
        "preferences.json" => {
            // Preserve valid legacy selections on import/export, without applying them.
            if let Some(icon) = v.get("dockIcon") {
                if !matches!(
                    icon.as_str(),
                    Some("automatic" | "detailed" | "standard" | "smile" | "frown")
                ) {
                    return Err("Invalid Dock icon selection.".into());
                }
            }
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
            if let Some(size) = v.get("exportFontSize") {
                if !size
                    .as_f64()
                    .is_some_and(|n| n == 0. || (1. ..=200.).contains(&n))
                {
                    return Err(
                        "Export font size must be between 1 and 200 pt (or 0 for the source size)."
                            .into(),
                    );
                }
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
        ("globalInstant", "n", false, true, true, true),
        ("globalInstantClipboard", "v", false, true, true, true),
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
        let (control, command) =
            if cfg!(windows) && matches!(id, "nextSourceTab" | "previousSourceTab") {
                (false, true)
            } else {
                (control, command)
            };
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
        "globalInstant",
        "globalInstantClipboard",
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
            && (if cfg!(windows) {
                "qwmh,cvxazsp "
            } else {
                "qwmh,cvxazsp \t"
            })
            .contains(key)
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
    #[test]
    fn legacy_dock_icon_settings_remain_valid_for_round_trips() {
        let home = tempfile::tempdir().unwrap();
        let settings = super::SettingsFiles::new(home.path());
        let mut preferences = super::default_document("preferences.json");
        assert!(super::validate("preferences.json", &preferences).is_ok());
        for choice in ["automatic", "detailed", "standard", "smile", "frown"] {
            preferences["dockIcon"] = serde_json::json!(choice);
            assert!(super::validate("preferences.json", &preferences).is_ok());
            let snapshot = settings.snapshot("preferences.json", false).unwrap();
            settings
                .save(
                    "preferences.json",
                    &preferences,
                    super::string(&snapshot, "revision"),
                )
                .unwrap();
            assert_eq!(
                settings.load("preferences.json").unwrap()["dockIcon"],
                choice
            );
        }
        for invalid in [
            serde_json::json!(null),
            serde_json::json!(3),
            serde_json::json!("unknown"),
        ] {
            preferences["dockIcon"] = invalid;
            assert!(super::validate("preferences.json", &preferences).is_err());
        }
    }
    use super::*;
    #[test]
    fn output_size_accepts_decimals_and_legacy_source_sizes_but_rejects_invalid_values() {
        for size in [json!(1), json!(30.5), json!(200), json!(0)] {
            assert!(validate(
                "preferences.json",
                &json!({"appearance":"light","autoSave":true,"exportFontSize":size})
            )
            .is_ok());
        }
        for size in [json!(-1), json!(0.5), json!(201), json!("30"), Value::Null] {
            assert!(validate(
                "preferences.json",
                &json!({"appearance":"light","autoSave":true,"exportFontSize":size})
            )
            .is_err());
        }
    }
    #[test]
    fn oversized_settings_remain_untouched_and_cannot_be_saved_over() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsFiles::new(dir.path());
        let first = settings.snapshot("preferences.json", false).unwrap();
        let path = dir.path().join("Settings/preferences.json");
        let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len(32 * 1024 * 1024).unwrap();
        assert!(settings.snapshot("preferences.json", false).is_err());
        assert!(settings
            .save(
                "preferences.json",
                &first["value"],
                string(&first, "revision")
            )
            .is_err());
        assert_eq!(fs::metadata(path).unwrap().len(), 32 * 1024 * 1024);
    }
    #[test]
    fn another_reader_cannot_authorize_a_stale_writer() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsFiles::new(dir.path());
        for name in [
            "preferences.json",
            "theme.json",
            "keybindings.json",
            "instant.json",
            "tools.json",
        ] {
            let first = settings.snapshot(name, false).unwrap();
            let mut external = first["value"].clone();
            external["extensions"] = json!({"community.example": "external change"});
            json_write(&dir.path().join("Settings").join(name), &external).unwrap();
            let second = settings.snapshot(name, false).unwrap();
            assert_eq!(second["value"], external);
            assert!(settings
                .save(name, &first["value"], string(&first, "revision"))
                .is_err());
            let revision = settings
                .save(name, &external, string(&second, "revision"))
                .unwrap();
            assert!(!revision.is_empty());
            assert_eq!(settings.load(name).unwrap(), external);
        }
    }
    #[test]
    fn invalid_active_settings_cannot_be_overwritten_using_fallback_revision() {
        let dir = tempfile::tempdir().unwrap();
        let settings = SettingsFiles::new(dir.path());
        let initial = settings.snapshot("preferences.json", false).unwrap();
        let path = dir.path().join("Settings/preferences.json");
        fs::write(&path, "{unfinished").unwrap();
        let fallback = settings.snapshot("preferences.json", false).unwrap();
        assert_eq!(fallback["value"], initial["value"]);
        assert_eq!(fallback["revision"], "");
        assert!(settings
            .save(
                "preferences.json",
                &fallback["value"],
                string(&fallback, "revision")
            )
            .is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "{unfinished");
    }
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
        settings
            .save(
                "preferences.json",
                &changed,
                string(
                    &settings.snapshot("preferences.json", false).unwrap(),
                    "revision",
                ),
            )
            .unwrap();
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
        settings
            .save(
                "theme.json",
                &theme,
                string(&settings.snapshot("theme.json", false).unwrap(), "revision"),
            )
            .unwrap();
        fs::write(d.path().join("Settings/theme.json"), "{").unwrap();
        assert!(settings
            .save(
                "theme.json",
                &theme,
                string(&settings.snapshot("theme.json", false).unwrap(), "revision")
            )
            .is_err());
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
    #[cfg(windows)]
    #[test]
    fn windows_source_tab_defaults_use_control_without_the_windows_key() {
        let defaults = default_bindings();
        for id in ["nextSourceTab", "previousSourceTab"] {
            assert_eq!(defaults[id]["command"], true);
            assert_eq!(defaults[id]["control"], false);
        }
        let mut value = default_document("keybindings.json");
        value["bindings"]["nextSourceTab"] = defaults["nextSourceTab"].clone();
        assert!(keybindings(&value).is_ok());
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
        settings
            .save(
                "keybindings.json",
                &v,
                string(
                    &settings.snapshot("keybindings.json", false).unwrap(),
                    "revision",
                ),
            )
            .unwrap();
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
