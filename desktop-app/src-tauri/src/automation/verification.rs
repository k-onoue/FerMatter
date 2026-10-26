//! Real compiler coverage through exactly the service used by CLI and MCP.
use super::*;
pub fn run(output: &Path) -> Result<Value> {
    let home = output.join("Automation");
    let store = Storage::new(home.clone(), home.join("Legacy"))?;
    let settings = SettingsFiles::new(&home);
    let service = Service {
        store: &store,
        settings: &settings,
    };
    let cancellation = compiler::Cancellation::default();
    let mut report = vec![];
    let call = |name: &str, args: Value| {
        service
            .call(name, args, &cancellation, &mut |_| {})
            .map_err(|e| format!("{}: {}", e.code, e.message))
    };
    for language in ["latex", "typst"] {
        // Project roots cannot live inside application storage.
        let root = output.join(format!("automation-{language}-source"));
        fs::create_dir_all(&root).map_err(err)?;
        let (main, text, source) = if language == "latex" {
            ("main.tex","\\documentclass[12pt]{article}\n\\usepackage{amsmath}\n\\newcommand{\\projectValue}{x+7}\n\\begin{document}\n\\[\\projectValue\\]\n\\end{document}",r"\projectValue+1")
        } else {
            (
                "main.typ",
                "#let projectValue = $x+7$\n= Model\n$projectValue$\n",
                "projectValue+1",
            )
        };
        fs::write(root.join(main), text).map_err(err)?;
        let mut p = store.open(&root, Some(main))?;
        p["cards"] = crate::scan::extract(&root, main, None)?["cards"].clone();
        store.save(string(&p, "id"), &p)?;
        let stamp = fingerprint(&root)?;
        let rendered = call(
            "render_equation",
            json!({"projectId":p["id"],"source":source,"fontSize":30,"color":"#2356ab","outputDirectory":output}),
        )?;
        if rendered["items"][0]["fontSize"].as_f64() != Some(30.)
            || rendered["items"][0]["widthPt"].as_f64().unwrap_or(0.) <= 0.
        {
            return Err("API render omitted output size".into());
        }
        roundtrip::decode(
            &fs::read_to_string(string(&rendered["items"][0], "path")).map_err(err)?,
        )?;
        let found = call("search_equations", json!({"projectId":p["id"]}))?;
        let args = json!({"projectId":p["id"],"equationId":p["cards"][0]["id"],"source":source,"name":"Slide variant","requestId":"verification","expectedRevision":found["revision"],"fontSize":30});
        let variant = call("create_variant", args.clone())?;
        let retried = call("create_variant", args)?;
        if variant["created"] != true
            || retried["created"] != false
            || variant["equation"]["id"] != retried["equation"]["id"]
        {
            return Err("Variant retries duplicated data".into());
        }
        let mut formats = vec![];
        for format in ["svg", "png", "pdf", "jpeg"] {
            let exported = call(
                "export_equations",
                json!({"projectId":p["id"],"equationIds":[variant["equation"]["id"],p["cards"][0]["id"]],"format":format,"fontSize":30,"outputDirectory":output}),
            )?;
            let items = exported["items"].as_array().ok_or("Missing export items")?;
            if items.len() != 2 || items[0]["equationId"] != variant["equation"]["id"] {
                return Err("Batch export order changed".into());
            }
            for item in items {
                if item["status"] != "ready"
                    || fs::metadata(string(item, "path")).map_err(err)?.len() == 0
                {
                    return Err("API exported an empty file".into());
                }
            }
            formats.push(exported);
        }
        if fingerprint(&root)? != stamp {
            return Err("Automation changed manuscript sources".into());
        }
        if home.join("Instant/session.json").exists() {
            return Err("Automation modified an Instant draft".into());
        }
        report.push(
            json!({"language":language,"render":rendered,"exports":formats,"variant":variant}),
        );
    }
    let standalone = call(
        "render_equation",
        json!({"language":"typst","source":"frac(a,b)","outputDirectory":output}),
    )?;
    for (template, source) in [
        (
            "builtin-tikz",
            r"\begin{tikzpicture}\draw (0,0)--(1,1);\end{tikzpicture}",
        ),
        (
            "builtin-cetz",
            "#cetz.canvas({ import cetz.draw: *; line((0,0),(1,1)) })",
        ),
    ] {
        call(
            "render_equation",
            json!({"templateId":template,"kind":"figure","source":source,"outputDirectory":output}),
        )?;
    }
    json_write(
        &output.join("automation-verification.json"),
        &json!({"projects":report,"standalone":standalone}),
    )?;
    Ok(json!({"projects":report.len(),"standalone":true,"home":home}))
}
