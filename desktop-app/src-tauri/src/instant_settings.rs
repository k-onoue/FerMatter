//! Global reusable defaults are separate from Instant's working drafts/history.
use crate::{files::*, instant, settings::SettingsFiles, storage::Storage};
use serde_json::{json, Value};
use std::collections::HashSet;

pub fn defaults() -> Value {
    let templates: Value = serde_json::from_str(include_str!("../../src/template-defaults.json"))
        .expect("bundled templates");
    json!({"schemaVersion":1,"kind":"mathlidelux.instant","templates":templates,"defaultTemplateID":"builtin-latex","fontSize":24,"exportFormat":"svg","color":"#000000","inputMode":"block"})
}

pub fn validate(value: &Value) -> Result<()> {
    if value["schemaVersion"] != 1 || value["kind"] != "mathlidelux.instant" {
        return Err("Unsupported Instant settings format.".into());
    }
    let templates = value["templates"]
        .as_array()
        .ok_or("Instant needs a templates list.")?;
    if templates.is_empty() || templates.len() > 100 {
        return Err("Use between 1 and 100 Instant templates.".into());
    }
    let mut ids = HashSet::new();
    for template in templates {
        if string(template, "id").is_empty()
            || !ids.insert(string(template, "id"))
            || string(template, "name").trim().is_empty()
        {
            return Err("Each Instant template needs a unique ID and a name.".into());
        }
        instant::validate(
            &json!({"template":template,"source":"","kind":value["inputMode"],"fontSize":value["fontSize"],"exportFormat":value["exportFormat"],"color":value["color"]}),
        )?;
        if string(template, "source").matches("{{content}}").count() != 1 {
            return Err("Place {{content}} exactly once in each Instant template.".into());
        }
    }
    if !ids.contains(string(value, "defaultTemplateID")) {
        return Err("Choose an existing default Instant template.".into());
    }
    if value.to_string().len() > 1_000_000 {
        return Err("Instant settings exceed 1 MB.".into());
    }
    Ok(())
}

pub fn load(store: &Storage, settings: &SettingsFiles) -> Result<Value> {
    let _lock = store.writes.lock().unwrap();
    let mut seed = defaults();
    // Seed once from shared project templates; subsequent global edits are independent.
    if let Ok(shared) = json_read(&store.home.join("templates.json")) {
        if let Some(custom) = shared["custom"].as_array() {
            seed["templates"]
                .as_array_mut()
                .unwrap()
                .extend(custom.iter().cloned());
        }
        if seed["templates"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == shared["defaultID"])
        {
            seed["defaultTemplateID"] = shared["defaultID"].clone();
        }
        if validate(&seed).is_err() {
            seed = defaults();
        }
    }
    // Keep an existing pre-settings-file Instant draft intact and also seed its valid defaults.
    if let Ok(session) = json_read(&store.home.join("Instant/session.json")) {
        let draft = &session["draft"];
        if instant::validate(draft).is_ok()
            && string(&draft["template"], "source")
                .matches("{{content}}")
                .count()
                == 1
        {
            let mut migrated = seed.clone();
            let templates = migrated["templates"].as_array_mut().unwrap();
            templates.retain(|t| t["id"] != draft["template"]["id"]);
            templates.push(draft["template"].clone());
            migrated["defaultTemplateID"] = draft["template"]["id"].clone();
            for field in ["fontSize", "exportFormat", "color"] {
                migrated[field] = draft[field].clone();
            }
            migrated["inputMode"] = draft["kind"].clone();
            if validate(&migrated).is_ok() {
                seed = migrated;
            }
        }
    }
    settings.initialize("instant.json", &seed)?;
    settings.load("instant.json")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_are_valid_and_external_edits_are_not_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        let settings = SettingsFiles::new(temp.path());
        let value = defaults();
        settings.initialize("instant.json", &value).unwrap();
        settings.load("instant.json").unwrap();
        let file = temp.path().join("Settings/instant.json");
        std::fs::write(&file, "unfinished external edit").unwrap();
        assert!(settings
            .save(
                "instant.json",
                &value,
                string(
                    &settings.snapshot("instant.json", false).unwrap(),
                    "revision"
                )
            )
            .is_err());
        assert!(settings.reload("instant.json").is_err());
        assert_eq!(settings.load("instant.json").unwrap(), value);
        assert_eq!(
            std::fs::read_to_string(file).unwrap(),
            "unfinished external edit"
        );
        let mut invalid = value;
        invalid["defaultTemplateID"] = json!("missing");
        assert!(validate(&invalid).is_err());
    }
    #[test]
    fn seeds_legacy_defaults_once_without_rewriting_the_working_draft() {
        let temp = tempfile::tempdir().unwrap();
        let store = Storage::new(temp.path().join("data"), temp.path().join("legacy")).unwrap();
        let settings = SettingsFiles::new(&store.home);
        let template = defaults()["templates"][0].clone();
        let draft = json!({"source":"unfinished \\frac{","kind":"inline","fontSize":36,"exportFormat":"png","color":"#123456","template":template});
        instant::session(&store, &json!({"action":"save","draft":draft})).unwrap();
        let before = std::fs::read(store.home.join("Instant/session.json")).unwrap();
        let mut config = load(&store, &settings).unwrap();
        assert_eq!(config["fontSize"], 36);
        assert_eq!(config["inputMode"], "inline");
        assert_eq!(
            std::fs::read(store.home.join("Instant/session.json")).unwrap(),
            before
        );
        config["fontSize"] = json!(48);
        settings
            .save(
                "instant.json",
                &config,
                string(
                    &settings.snapshot("instant.json", false).unwrap(),
                    "revision",
                ),
            )
            .unwrap();
        assert_eq!(load(&store, &settings).unwrap()["fontSize"], 48);
        assert_eq!(
            std::fs::read(store.home.join("Instant/session.json")).unwrap(),
            before
        );
    }
}
