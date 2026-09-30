//! Headless checks use only the repository's synthetic examples.
use crate::{compiler, files::*, scan};
use serde_json::{json, Value};
use std::{fs, path::Path};
pub fn verify_examples(repository: &Path, output: &Path) -> Result<Value> {
    let report_folder = output.canonicalize().map_err(err)?;
    // A retained run directory gives inspection artifacts without ever reusing an
    // older fixture or render cache. Failures retain their workspace as evidence.
    let output = fresh_workspace(&report_folder)?;
    let _cache = crate::cache::Cache::new(&output);
    let expected: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/extraction.json")).map_err(err)?;
    let mut results = vec![];
    for name in ["demo-paper", "demo-typst", "demo-tikz", "demo-cetz"] {
        let source = repository.join("examples").join(name);
        let target = output.join("ProjectsSource").join(name);
        snapshot(&source, &target, &Value::Null)?;
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
        // Reclaimed synthetic images must be regenerable, including when the
        // source card still carries its old cached paths.
        if name == "demo-typst" {
            let card = cards[0].clone();
            for key in ["svgPath", "pngPath", "pdfPath"] {
                fs::remove_file(string(&card, key)).map_err(err)?;
            }
            let mut regenerated = Value::Null;
            compiler::render(&root, main, "project", &[card], &cache, &c, |event| {
                regenerated = event
            })?;
            if regenerated["status"] != "ready" {
                return Err(format!("Cache regeneration failed: {regenerated}"));
            }
            for key in ["svgPath", "pngPath", "pdfPath"] {
                if regular_file(Path::new(string(&regenerated, key)))?
                    .metadata()
                    .map_err(err)?
                    .len()
                    == 0
                {
                    return Err("Regenerated artifact is empty".into());
                }
            }
        }
        let document = compiler::document(
            &root,
            main,
            "project",
            &project_dir.join("Documents"),
            &Value::Null,
            &c,
        )?;
        // PDF drags must use the compiled snapshot, even when live sources
        // have since changed. Also catches Typst navigation metadata leaking
        // into the snapshot used to render its standalone artwork.
        let drag_card = cards
            .iter()
            .find(|card| string(card, "kind") == "figure")
            .unwrap_or(&cards[0]);
        let entry = root.join(main);
        let original = fs::read(&entry).map_err(err)?;
        fs::write(
            &entry,
            "deliberately invalid live source for PDF drag verification",
        )
        .map_err(err)?;
        let artwork = compiler::document_artwork(
            &document,
            string(drag_card, "id"),
            &cache.join("PDFArtwork"),
            &c,
        );
        fs::write(&entry, original).map_err(err)?;
        let artwork = artwork?;
        if artwork["tex"] != drag_card["tex"] || artwork["status"] != "ready" {
            return Err("PDF drag did not preserve its compiled source".into());
        }
        for format in ["svg", "png", "jpeg", "pdf"] {
            crate::export::image_file(&artwork, format, 24., &project_dir.join("Exports"))?;
        }
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
            for card in cards.iter().filter(|card| {
                ["Trajectory", "Velocity", "Ridge solution"].contains(&string(card, "name"))
            }) {
                let anchor = compiler::navigate(
                    &document,
                    &json!({"file":card["file"],"line":card["line"]}),
                    &c,
                )?;
                let reverse = compiler::navigate(
                    &document,
                    &json!({"reverse":true,"page":anchor["page"],
                    "x":anchor["x"].as_f64().unwrap() + anchor["width"].as_f64().unwrap() / 2.,
                    "y":anchor["y"].as_f64().unwrap() + anchor["height"].as_f64().unwrap() / 2.}),
                    &c,
                )?;
                let line = reverse["line"].as_u64().unwrap();
                if reverse["file"] != card["file"]
                    || line < card["line"].as_u64().unwrap()
                    || line > card["endLine"].as_u64().unwrap()
                {
                    return Err(format!(
                        "SyncTeX equation round trip failed: {} -> {}",
                        card["name"], reverse
                    ));
                }
                let hit = compiler::drag_target(
                    &document,
                    &json!({"reverse":true,"page":anchor["page"],
                    "x":anchor["x"].as_f64().unwrap() + anchor["width"].as_f64().unwrap() / 2.,
                    "y":anchor["y"].as_f64().unwrap() + anchor["height"].as_f64().unwrap() / 2.}),
                    &c,
                )?;
                if hit["targetID"] != card["id"] {
                    return Err(format!(
                        "PDF drag target mismatch: {} -> {}",
                        card["id"], hit
                    ));
                }
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
    verify_instant(&output)?;
    verify_project_instant(&output)?;
    verify_zip_compiler_defaults(&output)?;
    let report = json!(results);
    json_write(&output.join("verification.json"), &report)?;
    json_write(
        &report_folder.join("verification.json"),
        &json!({"workspace": output, "results": report}),
    )?;
    Ok(report)
}
fn fresh_workspace(output: &Path) -> Result<std::path::PathBuf> {
    Ok(tempfile::Builder::new()
        .prefix("verify-")
        .tempdir_in(output)
        .map_err(err)?
        .keep())
}

fn verify_zip_compiler_defaults(output: &Path) -> Result<()> {
    use crate::storage::{import_zip, Storage};
    use std::io::Write;
    let workspace = output.join("zip-compiler-defaults");
    fs::create_dir_all(&workspace).map_err(err)?;
    let store = Storage::new(workspace.join("data"), workspace.join("legacy"))?;
    let mut reports = vec![];
    // Search-path-only rc files must use pdfLaTeX. Explicit LuaLaTeX and
    // latex/DVI settings must override that fallback, for either rc filename.
    for (name, rc_name, mode, engine_check) in [
        (
            "default",
            "latexmkrc",
            "",
            r"\ifdefined\pdfoutput\ifnum\pdfoutput>0\else\errmessage{Expected direct PDF}\fi\else\errmessage{Expected pdfTeX}\fi",
        ),
        (
            "lua",
            ".latexmkrc",
            "$pdf_mode = 4;",
            r"\ifdefined\directlua\else\errmessage{Expected LuaLaTeX}\fi",
        ),
        (
            "dvi",
            "latexmkrc",
            "$pdf_mode = 0;",
            r"\ifnum\pdfoutput=0\else\errmessage{Expected DVI}\fi",
        ),
    ] {
        let archive = workspace.join(format!("{name}.zip"));
        let mut zip = zip::ZipWriter::new(fs::File::create(&archive).map_err(err)?);
        let rc =
            format!("$ENV{{'TEXINPUTS'}} = './styles//:' . ($ENV{{'TEXINPUTS'}} // '');\n{mode}\n");
        let main = format!("\\documentclass{{article}}\n\\input{{definitions}}\n{engine_check}\n\\begin{{document}}\n$\\sample{{x}}$\n\\end{{document}}\n");
        for (path, text) in [
            ("main.tex", main.as_str()),
            (rc_name, rc.as_str()),
            (
                "styles/definitions.tex",
                r"\newcommand{\sample}[1]{#1^{2}+1}",
            ),
        ] {
            zip.start_file(
                format!("project/{path}"),
                zip::write::SimpleFileOptions::default(),
            )
            .map_err(err)?;
            zip.write_all(text.as_bytes()).map_err(err)?;
        }
        zip.finish().map_err(err)?;
        let root = import_zip(&archive, &workspace.join("imports"))?;
        let project = store.open(&root, Some("main.tex"))?;
        let engine = string(&project, "engine");
        if engine != "project" {
            return Err("Imported rc was not selected as the compiler configuration.".into());
        }
        let before = fingerprint(&root)?;
        let scanned = scan::extract(&root, "main.tex", None)?;
        let cancel = compiler::Cancellation::default();
        let mut rendered = vec![];
        compiler::render(
            &root,
            "main.tex",
            engine,
            scanned["cards"].as_array().unwrap(),
            &workspace.join(name).join("Cache"),
            &cancel,
            |card| rendered.push(card),
        )?;
        if rendered.len() != 1 || rendered[0]["status"] != "ready" {
            return Err(format!("ZIP compiler {name}: {rendered:?}"));
        }
        for key in ["svgPath", "pngPath", "pdfPath"] {
            if fs::metadata(string(&rendered[0], key)).map_err(err)?.len() == 0 {
                return Err(format!("ZIP compiler {name}: empty {key}"));
            }
        }
        let document = compiler::document(
            &root,
            "main.tex",
            engine,
            &workspace.join(name).join("Documents"),
            &Value::Null,
            &cancel,
        )?;
        if fingerprint(&root)? != before {
            return Err("ZIP compilation changed project sources.".into());
        }
        reports.push(json!({"mode":name,"cards":rendered.len(),"pdf":document["pdfPath"]}));
    }
    json_write(&workspace.join("verification.json"), &json!(reports))
}
fn verify_project_instant(output: &Path) -> Result<()> {
    use crate::{instant, storage::Storage};
    let temp = tempfile::tempdir().map_err(err)?;
    let store = Storage::new(temp.path().join("data"), temp.path().join("legacy"))?;
    let mut reports = vec![];
    for format in ["latex", "typst"] {
        let root = temp.path().join(format);
        fs::create_dir_all(root.join("sections")).map_err(err)?;
        let (main, source) = if format == "latex" {
            fs::write(root.join("main.tex"), "\\documentclass[12pt]{article}\n\\input{definitions}\n\\begin{document}Project body\\end{document}").map_err(err)?;
            fs::write(
                root.join("definitions.tex"),
                "\\usepackage{mathtools,bm}\n\\newcommand{\\projectvec}[1]{\\bm{#1}}\n",
            )
            .map_err(err)?;
            fs::write(root.join("latexmkrc"), "$pdf_mode = 1;\n").map_err(err)?;
            ("main.tex", r"\projectvec{x}+\mathclap{1}")
        } else {
            fs::write(root.join("main.typ"), "#set text(size: 13pt)\n#import \"symbols.typ\": projectSum\n= Project\n#include \"sections/model.typ\"\n").map_err(err)?;
            fs::write(root.join("symbols.typ"), "#let projectSum(x) = $x + 7$\n").map_err(err)?;
            fs::write(root.join("sections/model.typ"), "#import \"../symbols.typ\": projectSum\n$ x $\n#let lastDef(x) = $projectSum(x) + 3$\n").map_err(err)?;
            ("main.typ", "projectSum(x)")
        };
        let p = store.open(&root, Some(main))?;
        let id = string(&p, "id");
        let before = fingerprint(&root)?;
        let context = instant::context(&store, id)?;
        let mut draft = json!({"source":source,"kind":"block","fontSize":24,"color":"#2356ab","exportFormat":"svg","template":{"id":"project","name":"Project","format":format,"source":"","compiler":"pdflatex"}});
        let cancel = compiler::Cancellation::default();
        let result = instant::render_project(&store, id, &draft, &cancel)?;
        let mut nested = Value::Null;
        if format == "typst" {
            draft["contextFile"] = json!("sections/model.typ");
            draft["source"] = json!("lastDef(x)");
            nested = instant::render_project(&store, id, &draft, &cancel)?;
        }
        if fingerprint(&root)? != before {
            return Err("Project Instant changed source files.".into());
        }
        draft["color"] = json!("#aa2244");
        let colored = instant::render_project(&store, id, &draft, &cancel)?;
        let previous = if nested.is_null() { &result } else { &nested };
        if colored["card"]["svgPath"] == previous["card"]["svgPath"] {
            return Err("Project Instant reused stale colors.".into());
        }
        // The same restored recipe must work as a Library variant, where the
        // ordinary batch renderer (rather than Instant's template wrapper) runs.
        let context_file = draft["contextFile"].as_str().unwrap_or(main);
        let mut variants = vec![];
        for (name, color) in [
            ("blue", Some("2356ab")),
            ("red", Some("aa2244")),
            ("default", None),
        ] {
            let mut card = json!({"id":name,"tex":draft["source"],"kind":"block",
                "file":context_file,"restoredContextFile":context_file,"origin":"custom",
                "requiresRender":false,"exportFontSize":24});
            if let Some(color) = color {
                card["instantColor"] = json!(color);
            }
            variants.push(card);
        }
        compiler::render(
            &root,
            main,
            "project",
            &variants.clone(),
            &temp.path().join(format!("{format}-restored")),
            &cancel,
            |event| {
                let card = variants
                    .iter_mut()
                    .find(|v| v["id"] == event["id"])
                    .unwrap();
                for (key, value) in event.as_object().unwrap() {
                    card[key] = value.clone();
                }
            },
        )?;
        for (card, expected) in variants
            .iter()
            .zip([[35u8, 86, 171], [170, 34, 68], [0, 0, 0]])
        {
            if card["status"] != "ready" {
                return Err(format!(
                    "Restored {format} variant failed: {}",
                    card["error"]
                ));
            }
            let pixels = image::open(string(card, "pngPath"))
                .map_err(err)?
                .into_rgba8();
            if !pixels
                .pixels()
                .any(|p| p[3] > 240 && (0..3).all(|i| p[i].abs_diff(expected[i]) <= 4))
            {
                return Err(format!(
                    "Restored {format} variant lost its color or leaked the previous card's color."
                ));
            }
            if format == "typst" && card["sourceFontSize"] != 13. {
                return Err("Restored Typst variant lost its parent context.".into());
            }
            let recipe = crate::roundtrip::card_recipe(&store, &p, card, 24.)?;
            if recipe["draft"]["contextFile"] != context_file || recipe["draft"]["fontSize"] != 24.
            {
                return Err("Restored variant lost its portable context or size.".into());
            }
        }
        if variants[0]["svgPath"] == variants[1]["svgPath"] {
            return Err("Restored variants reused a cache entry for different colors.".into());
        }
        if fingerprint(&root)? != before {
            return Err("Restored Library variants changed source files.".into());
        }
        let definitions = root.join(if format == "latex" {
            "definitions.tex"
        } else {
            "symbols.typ"
        });
        let old = read(&definitions)?;
        fs::write(
            &definitions,
            format!(
                "{old}\n{} changed\n",
                if format == "latex" { "%" } else { "//" }
            ),
        )
        .map_err(err)?;
        if instant::context(&store, id)?["revision"] == context["revision"] {
            return Err("Project Instant missed source changes.".into());
        }
        cancel.cancel();
        if instant::render_project(&store, id, &draft, &cancel).is_ok() {
            return Err("Cancelled Project Instant returned artwork.".into());
        }
        reports.push(json!({"format":format,"status":"ready","nestedTypstContext":!nested.is_null(),"restoredLibraryContext":true,"restoredLibraryColors":true,"sourceUnchanged":true,"externalChangesDetected":true}));
    }
    json_write(
        &output.join("instant-project-verification.json"),
        &json!(reports),
    )
}
fn verify_instant(output: &Path) -> Result<()> {
    use crate::{export, instant};
    let latex = "\\documentclass[12pt]{article}\n\\usepackage{amsmath,amssymb}\n\\newcommand{\\vect}[1]{\\mathbf{#1}}\n\\begin{document}\n{{content}}\n\\end{document}";
    let typst = "#set text(size: 12pt)\n#let square(x) = $x^2$\n{{content}}\n";
    let tikz = latex.replace("\\begin{document}", "\\usepackage{tikz}\n\\begin{document}");
    let cetz = format!("#import \"@preview/cetz:0.5.2\"\n{typst}");
    let mut reports = vec![];
    for (name, format, template, source, kind) in [
        ("latex", "latex", latex, r"\frac{\vect{x}}{2}", "block"),
        (
            "multiline",
            "latex",
            latex,
            "\\begin{aligned}a&=b+c\\\\\nd&=e+f\\end{aligned}",
            "block",
        ),
        (
            "typst-inline",
            "typst",
            typst,
            "(dif x(t))/(dif t)",
            "inline",
        ),
        ("typst-display", "typst", typst, "square(x) + 1", "block"),
        (
            "tikz",
            "latex",
            &tikz,
            r"\begin{tikzpicture}\draw (0,0)--(1,0)--(0.5,1)--cycle;\end{tikzpicture}",
            "figure",
        ),
        (
            "cetz",
            "typst",
            &cetz,
            "#cetz.canvas({ import cetz.draw: *; line((0,0),(1,0),(0.5,1),close: true) })",
            "figure",
        ),
    ] {
        let mut draft = json!({"source":source,"kind":kind,"fontSize":24,"color":"#2356ab","exportFormat":"svg","template":{"id":name,"name":name,"format":format,"source":template,"compiler":"pdflatex"}});
        let c = compiler::Cancellation::default();
        let result = instant::render(output, &draft, &c)?;
        let card = &result["card"];
        if card["status"] != "ready" || card["sourceFontSize"].as_f64().unwrap_or(0.) <= 0. {
            return Err(format!("Instant {name}: no measured artwork"));
        }
        for format in ["svg", "pdf", "png", "jpeg"] {
            let file = export::image_file(card, format, 36., &output.join("Instant/Exports"))?;
            if file.metadata().map_err(err)?.len() == 0 {
                return Err(format!("Instant {name}: empty {format}"));
            }
        }
        if name == "latex" || name == "typst-display" {
            let mut colored = draft.clone();
            colored["color"] = json!("#dc2626");
            let red = instant::render(output, &colored, &c)?;
            if red["path"] == result["path"] || red["card"]["svgPath"] == card["svgPath"] {
                return Err(format!("Instant {name}: color reused old artwork"));
            }
            for (image, expected) in [(&result, [35u8, 86, 171]), (&red, [220u8, 38, 38])] {
                let png = export::image_file(
                    &image["card"],
                    "png",
                    36.,
                    &output.join("Instant/Exports"),
                )?;
                let pixels = image::open(png).map_err(err)?.into_rgba8();
                if !pixels
                    .pixels()
                    .any(|p| p[3] > 240 && (0..3).all(|i| p[i].abs_diff(expected[i]) <= 4))
                {
                    return Err(format!(
                        "Instant {name}: preview/export lost equation color"
                    ));
                }
            }
            let svg = crate::roundtrip::read_svg(Path::new(string(&red, "path")))?;
            let recipe = crate::roundtrip::decode(&svg)?;
            let temp = tempfile::tempdir().map_err(err)?;
            let store =
                crate::storage::Storage::new(temp.path().join("data"), temp.path().join("legacy"))?;
            let (_, restored) = crate::roundtrip::resolve(&store, &recipe)?;
            if restored["color"] != colored["color"] || restored["requiresRender"] != false {
                return Err(format!(
                    "Instant {name}: restored draft is not ready to preview"
                ));
            }
            instant::render(output, &restored, &c)?;
        }
        // A failed render must not return the previous successful image.
        draft["source"] = json!(if format == "latex" {
            "\\undefinedInstantCommand{"
        } else {
            "#unknown-instant-command("
        });
        if instant::render(output, &draft, &c).is_ok() {
            return Err(format!(
                "Instant {name}: invalid input unexpectedly rendered"
            ));
        }
        draft["source"] = json!(source);
        instant::render(output, &draft, &c)?;
        c.cancel();
        if instant::render(output, &draft, &c).is_ok() {
            return Err("Instant: cancelled work returned an image".into());
        }
        reports.push(json!({"name":name,"card":card,"path":result["path"]}));
    }
    json_write(&output.join("instant-verification.json"), &json!(reports))
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
    fn each_verification_has_fresh_sources_and_cache() {
        let parent = tempfile::tempdir().unwrap();
        let first = fresh_workspace(parent.path()).unwrap();
        fs::create_dir_all(first.join("ProjectsSource")).unwrap();
        fs::write(first.join("ProjectsSource/stale.typ"), "old source").unwrap();
        let second = fresh_workspace(parent.path()).unwrap();
        assert_ne!(first, second);
        assert!(second.read_dir().unwrap().next().is_none());
        assert!(first.join("ProjectsSource/stale.typ").exists());
    }
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
