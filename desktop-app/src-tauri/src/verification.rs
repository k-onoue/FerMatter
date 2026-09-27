//! Headless checks use only the repository's synthetic examples.
use crate::{compiler, files::*, scan};
use serde_json::{json, Value};
use std::{fs, path::Path};
pub fn verify_examples(repository: &Path, output: &Path) -> Result<Value> {
    let output = output.canonicalize().map_err(err)?;
    let expected: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/extraction.json")).map_err(err)?;
    let mut results = vec![];
    for name in ["demo-paper", "demo-typst", "demo-tikz", "demo-cetz"] {
        let source = repository.join("examples").join(name);
        let target = output.join("ProjectsSource").join(name);
        if !target.exists() {
            snapshot(&source, &target, &Value::Null)?
        }
        let root = target.canonicalize().map_err(err)?;
        let main = if name == "demo-typst" || name == "demo-cetz" {
            "main.typ"
        } else {
            "main.tex"
        };
        let stamp = fingerprint(&root)?;
        let scanned = scan::extract(&root, main, None)?;
        let normalized = normalize(&scanned);
        if normalized != expected[name] {
            json_write(&output.join(format!("{name}-actual.json")), &normalized)?;
            return Err(format!(
                "{name}: scanner differs from the Swift worker fixture."
            ));
        }
        let ident = id(&root.to_string_lossy());
        let project_dir = output.join("Projects").join(&ident);
        let cache = project_dir.join("Cache");
        let c = compiler::Cancellation::default();
        let mut cards = scanned["cards"].as_array().unwrap().clone();
        compiler::render(
            &root,
            main,
            "project",
            &cards.clone(),
            &cache,
            &c,
            |event| {
                if let Some(card) = cards.iter_mut().find(|v| v["id"] == event["id"]) {
                    for (k, v) in event.as_object().unwrap() {
                        card[k] = v.clone()
                    }
                }
            },
        )?;
        let failed: Vec<_> = cards.iter().filter(|v| v["status"] != "ready").collect();
        if !failed.is_empty() {
            return Err(format!(
                "{name}: {}",
                failed
                    .iter()
                    .map(|c| format!(
                        "{}: {}",
                        c["name"],
                        string(c, "error")
                            .chars()
                            .rev()
                            .take(1300)
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect::<String>()
                    ))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        let document = compiler::document(
            &root,
            main,
            "project",
            &project_dir.join("Documents"),
            &Value::Null,
            &c,
        )?;
        for card in &cards {
            for format in ["svg", "png", "jpeg", "pdf"] {
                let path =
                    crate::export::image_file(card, format, 0., &project_dir.join("Exports"))?;
                if fs::metadata(path).map_err(err)?.len() == 0 {
                    return Err("Empty export".into());
                }
            }
        }
        if main.ends_with(".tex") {
            let card = &cards[0];
            let anchor = compiler::navigate(
                &document,
                &json!({"file":card["file"],"line":card["line"]}),
                &c,
            )?;
            if anchor["page"].as_u64().unwrap_or(0) == 0 {
                return Err("SyncTeX forward navigation failed".into());
            }
            let reverse = compiler::navigate(
                &document,
                &json!({"reverse":true,"page":anchor["page"],"x":anchor["x"],"y":anchor["y"]}),
                &c,
            )?;
            if reverse["file"].as_str().is_none() {
                return Err("SyncTeX reverse navigation failed".into());
            }
        }
        // Verify resized vector PDF output independently of the UI.
        let scaled =
            crate::export::image_file(&cards[0], "pdf", 24., &project_dir.join("Exports"))?;
        let scaled_pdf = lopdf::Document::load(scaled).map_err(err)?;
        if scaled_pdf.get_pages().len() != 1 {
            return Err("Scaled PDF must contain one page".into());
        }
        if fingerprint(&root)? != stamp {
            return Err(format!("{name}: original files changed"));
        }
        let project = json!({"id":ident,"root":root,"main":main,"format":if main.ends_with(".typ"){"typst"}else{"latex"},"engine":"project","cards":cards,"outline":scanned["outline"],"warnings":scanned["warnings"],"files":scanned["files"],"customOrder":[],"sections":[],"sectionAssignments":{},"exportFormat":"svg","exportFontSize":0,"candidates":[main]});
        json_write(&project_dir.join("project.json"), &project)?;
        json_write(&project_dir.join("verification-document.json"), &document)?;
        results.push(json!({"project":name,"cards":cards.len(),"headings":scanned["outline"].as_array().unwrap().len(),"anchors":document["anchors"].as_array().unwrap().len(),"pdf":document["pdfPath"]}));
    }
    let report = json!(results);
    json_write(&output.join("verification.json"), &report)?;
    Ok(report)
}
fn normalize(scan: &Value) -> Value {
    let mut cards = vec![];
    for c in scan["cards"].as_array().unwrap() {
        let mut card = serde_json::Map::new();
        for key in [
            "id",
            "tex",
            "kind",
            "file",
            "line",
            "endLine",
            "sourceOffset",
            "sourceEnd",
            "outlinePath",
            "typstContext",
        ] {
            if let Some(v) = c.get(key) {
                card.insert(key.into(), v.clone());
            }
        }
        cards.push(Value::Object(card));
    }
    json!({"cards":cards,"outline":scan["outline"],"files":scan["files"]})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scanner_matches_existing_worker() {
        let repository = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .unwrap();
        let expected: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/extraction.json")).unwrap();
        for name in ["demo-paper", "demo-tikz", "demo-typst", "demo-cetz"] {
            let root = repository.join("examples").join(name);
            let main = if name == "demo-typst" || name == "demo-cetz" {
                "main.typ"
            } else {
                "main.tex"
            };
            assert_eq!(
                normalize(&scan::extract(&root, main, None).unwrap()),
                expected[name],
                "{name}"
            )
        }
    }
}
