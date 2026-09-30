//! Stable, window-independent commands shared by local automation transports.
mod cli;
mod mcp;
mod schema;
pub(crate) mod transport;
use crate::{
    compiler, export, files::*, instant, instant_settings, roundtrip, settings::SettingsFiles,
    storage::Storage,
};
pub use cli::entry;
pub(crate) use schema::tools;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
}
impl ApiError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl From<String> for ApiError {
    fn from(message: String) -> Self {
        Self::new(
            if message == "Cancelled" {
                "cancelled"
            } else {
                "operation_failed"
            },
            message,
        )
    }
}
pub type ApiResult = std::result::Result<Value, ApiError>;
pub struct Service<'a> {
    pub store: &'a Storage,
    pub settings: &'a SettingsFiles,
}
fn parse<T: serde::de::DeserializeOwned>(value: Value) -> std::result::Result<T, ApiError> {
    serde_json::from_value(value).map_err(|e| ApiError::new("invalid_arguments", e.to_string()))
}
pub fn revision(project: &Value) -> String {
    hash(project.to_string())
}

impl Service<'_> {
    fn project(&self, id: &str) -> std::result::Result<Value, ApiError> {
        let projects = self.store.recent()?;
        let saved = projects.iter().find(|p| p["id"] == id).ok_or_else(|| {
            ApiError::new(
                "project_not_found",
                "Open this project in Mathlidelux first.",
            )
        })?;
        let directory = self.store.project_dir(id)?;
        if !crate::project_data::present(&directory.join("project.json"))
            && !crate::project_data::present(&crate::project_data::backup(
                &directory.join("project.json"),
            ))
        {
            let _guard = self.store.writes.lock().unwrap();
            return self
                .store
                .open(
                    Path::new(string(saved, "root")),
                    Some(string(saved, "main")),
                )
                .map_err(Into::into);
        }
        self.store.load(id).map_err(Into::into)
    }
    pub fn call(
        &self,
        operation: &str,
        value: Value,
        cancel: &compiler::Cancellation,
        progress: &mut impl FnMut(Value),
    ) -> ApiResult {
        cancel.check()?;
        match operation {
            "list_templates" => {
                let args: schema::Page = parse(value)?;
                let (offset, limit) = args.bounds()?;
                let settings = instant_settings::load(self.store, self.settings)?;
                let templates = settings["templates"].as_array().unwrap();
                Ok(
                    json!({"apiVersion":1,"defaultTemplateId":settings["defaultTemplateID"],"total":templates.len(),"templates":templates.iter().skip(offset).take(limit).collect::<Vec<_>>()}),
                )
            }
            "list_projects" => {
                let args: schema::Page = parse(value)?;
                let (offset, limit) = args.bounds()?;
                let projects = self.store.recent()?;
                Ok(
                    json!({"apiVersion":1,"total":projects.len(),"projects":projects.iter().skip(offset).take(limit).map(|p| json!({"id":p["id"],"name":Path::new(string(p,"root")).file_name().unwrap_or_default(),"path":p["root"],"format":p["format"],"equationCount":p["cards"].as_array().map_or(0,Vec::len),"revision":revision(p)})).collect::<Vec<_>>() }),
                )
            }
            "search_equations" => {
                let args: schema::Search = parse(value)?;
                let p = self.project(&args.project_id)?;
                let query = args.query.unwrap_or_default().to_lowercase();
                let (offset, limit) = schema::Page {
                    offset: args.offset,
                    limit: args.limit,
                }
                .bounds()?;
                let cards: Vec<_> = p["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|c| {
                        let section = c["outlinePath"]
                            .as_array()
                            .map(|ids| {
                                ids.iter()
                                    .filter_map(|id| {
                                        p["outline"].as_array()?.iter().find(|h| h["id"] == *id)
                                    })
                                    .map(|h| string(h, "title"))
                                    .collect::<Vec<_>>()
                                    .join(" ")
                            })
                            .unwrap_or_default();
                        let text = format!(
                            "{} {} {} {section}",
                            string(c, "name"),
                            string(c, "tex"),
                            string(c, "file")
                        )
                        .to_lowercase();
                        (args
                            .kind
                            .as_ref()
                            .is_none_or(|k| k.as_str() == string(c, "kind")))
                            && text.contains(&query)
                    })
                    .collect();
                Ok(
                    json!({"apiVersion":1,"projectId":args.project_id,"revision":revision(&p),"sourceState":"saved_library","total":cards.len(),"equations":cards.iter().skip(offset).take(limit).map(|c|json!({"id":c["id"],"name":c["name"],"kind":c["kind"],"origin":c["origin"],"file":c["file"],"line":c["line"],"sourcePreview":string(c,"tex").chars().take(1000).collect::<String>()})).collect::<Vec<_>>()}),
                )
            }
            "get_equation" => {
                let args: schema::Equation = parse(value)?;
                let p = self.project(&args.project_id)?;
                let c = find_card(&p, &args.equation_id)?;
                Ok(
                    json!({"apiVersion":1,"projectId":args.project_id,"revision":revision(&p),"sourceState":"saved_library","equation":c}),
                )
            }
            "render_equation" => {
                let args: schema::Render = parse(value)?;
                let p = args
                    .project_id
                    .as_deref()
                    .map(|id| self.project(id))
                    .transpose()?;
                self.output_directory(&args.output_directory)?;
                let size = self.size(args.font_size, p.as_ref(), None)?;
                let draft = self.draft(&args, p.as_ref(), if size > 0. { size } else { 30. })?;
                let result = if let Some(p) = &p {
                    instant::render_project(self.store, string(p, "id"), &draft, cancel)
                } else {
                    instant::render(&self.store.home, &draft, cancel)
                }
                .map_err(|e| {
                    if cancel.check().is_err() {
                        ApiError::new("cancelled", "Cancelled")
                    } else {
                        ApiError::new("render_failed", e)
                    }
                })?;
                let _lease = crate::cache::Lease::new(&self.store.home, &result);
                self.publish(
                    &args.output_directory,
                    &[(result["card"].clone(), size)],
                    args.format.as_str(),
                    p.as_ref(),
                    cancel,
                    progress,
                )
            }
            "export_equations" => {
                let args: schema::Export = parse(value)?;
                if args.equation_ids.is_empty() || args.equation_ids.len() > 64 {
                    return Err(ApiError::new(
                        "invalid_arguments",
                        "Export between 1 and 64 equations.",
                    ));
                }
                let p = self.project(&args.project_id)?;
                self.output_directory(&args.output_directory)?;
                if args
                    .equation_ids
                    .iter()
                    .collect::<std::collections::HashSet<_>>()
                    .len()
                    != args.equation_ids.len()
                {
                    return Err(ApiError::new(
                        "invalid_arguments",
                        "Each equationId must appear once.",
                    ));
                }
                let cards = args
                    .equation_ids
                    .iter()
                    .map(|id| find_card(&p, id).cloned())
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                let mut rendered = Vec::new();
                compiler::render(
                    &self.store.root(&args.project_id)?,
                    string(&p, "main"),
                    string(&p, "engine"),
                    &cards,
                    &self.store.project_dir(&args.project_id)?.join("Cache"),
                    cancel,
                    |event| {
                        progress(
                            json!({"completed":rendered.len()+1,"total":cards.len(),"equationId":event["id"]}),
                        );
                        let mut card = cards
                            .iter()
                            .find(|c| c["id"] == event["id"])
                            .unwrap()
                            .clone();
                        for (k, v) in event.as_object().unwrap() {
                            card[k] = v.clone();
                        }
                        rendered.push(card);
                    },
                )?;
                let _lease = crate::cache::Lease::new(&self.store.home, &json!(rendered));
                rendered.sort_by_key(|card| {
                    args.equation_ids
                        .iter()
                        .position(|id| card["id"] == *id)
                        .unwrap_or(usize::MAX)
                });
                let sized = rendered
                    .into_iter()
                    .map(|c| {
                        self.size(args.font_size, Some(&p), Some(&c))
                            .map(|size| (c, size))
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                self.publish(
                    &args.output_directory,
                    &sized,
                    args.format.as_str(),
                    Some(&p),
                    cancel,
                    progress,
                )
            }
            "create_variant" => {
                let args: schema::Variant = parse(value.clone())?;
                if args.request_id.is_empty()
                    || args.request_id.len() > 128
                    || args.name.trim().is_empty()
                    || args.name.len() > 300
                {
                    return Err(ApiError::new(
                        "invalid_arguments",
                        "Provide a name and a requestId of 1–128 characters.",
                    ));
                }
                let p = self.project(&args.project_id)?;
                let id = format!("custom-api-{}", crate::files::id(&args.request_id));
                let mut intent = value;
                intent.as_object_mut().unwrap().remove("expectedRevision");
                let request_hash = hash(intent.to_string());
                if let Some(existing) = p["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"] == id)
                {
                    if existing["automationRequestHash"] != request_hash {
                        return Err(ApiError::new(
                            "request_id_conflict",
                            "This requestId was used with different arguments.",
                        ));
                    }
                    return Ok(
                        json!({"apiVersion":1,"projectId":args.project_id,"equation":existing,"revision":revision(&p),"created":false}),
                    );
                }
                if revision(&p) != args.expected_revision {
                    return Err(ApiError::new(
                        "project_conflict",
                        "The library changed. Search again before creating the variant.",
                    ));
                }
                let base = find_card(&p, &args.equation_id)?;
                if args.source.trim().is_empty() || args.source.len() > 1_000_000 {
                    return Err(ApiError::new(
                        "invalid_arguments",
                        "Source must contain 1–1,000,000 bytes.",
                    ));
                }
                let stamp =
                    fingerprint_checked(&self.store.root(&args.project_id)?, &|| cancel.check())?;
                let mut card = base.clone();
                for key in ["svgPath", "pngPath", "pdfPath", "error", "restoreMetadata"] {
                    card.as_object_mut().unwrap().remove(key);
                }
                card["id"] = json!(id);
                card["parentID"] = base["id"].clone();
                card["name"] = json!(args.name);
                card["tex"] = json!(args.source);
                card["origin"] = json!("custom");
                card["requiresRender"] = json!(false);
                if let Some(color) = args.color {
                    schema::validate_color(&color)?;
                    card["instantColor"] = json!(&color[1..]);
                }
                let size = self.size(args.font_size, Some(&p), Some(base))?;
                card["exportFontSize"] = json!(size);
                compiler::render(
                    &self.store.root(&args.project_id)?,
                    string(&p, "main"),
                    string(&p, "engine"),
                    &[card.clone()],
                    &self.store.project_dir(&args.project_id)?.join("Cache"),
                    cancel,
                    |event| {
                        for (k, v) in event.as_object().unwrap() {
                            card[k] = v.clone();
                        }
                    },
                )?;
                let _lease = crate::cache::Lease::new(&self.store.home, &card);
                if card["status"] != "ready" {
                    return Err(ApiError::new("render_failed", string(&card, "error")));
                }
                cancel.check()?;
                let _lock = self.store.writes.lock().unwrap();
                let mut latest = self.store.load(&args.project_id)?;
                if let Some(existing) = latest["cards"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|c| c["id"] == id)
                {
                    if existing["automationRequestHash"] != request_hash {
                        return Err(ApiError::new(
                            "request_id_conflict",
                            "This requestId was used with different arguments.",
                        ));
                    }
                    return Ok(
                        json!({"apiVersion":1,"projectId":args.project_id,"equation":existing,"revision":revision(&latest),"created":false}),
                    );
                }
                if revision(&latest) != args.expected_revision
                    || fingerprint_checked(&self.store.root(&args.project_id)?, &|| cancel.check())?
                        != stamp
                {
                    return Err(ApiError::new(
                        "project_conflict",
                        "The project changed during rendering. Retry with its current revision.",
                    ));
                }
                let generation = latest["automationGeneration"]
                    .as_u64()
                    .unwrap_or(0)
                    .checked_add(1)
                    .ok_or_else(|| {
                        ApiError::new("operation_failed", "Variant generation overflow")
                    })?;
                card["automationGeneration"] = json!(generation);
                card["automationRequestHash"] = json!(request_hash);
                card["sourceOrder"] = json!(latest["cards"].as_array().unwrap().len());
                latest["cards"].as_array_mut().unwrap().push(card.clone());
                latest["automationGeneration"] = json!(generation);
                self.store.save(&args.project_id, &latest)?;
                Ok(
                    json!({"apiVersion":1,"projectId":args.project_id,"equation":card,"revision":revision(&self.store.load(&args.project_id)?),"created":true,"generation":generation}),
                )
            }
            _ => Err(ApiError::new(
                "unknown_operation",
                format!("Unknown operation: {operation}"),
            )),
        }
    }
    fn size(
        &self,
        requested: Option<f64>,
        project: Option<&Value>,
        card: Option<&Value>,
    ) -> std::result::Result<f64, ApiError> {
        let defaults = self.settings.load("preferences.json")?;
        if requested.is_some_and(|size| !size.is_finite() || !(1. ..=200.).contains(&size)) {
            return Err(ApiError::new(
                "invalid_arguments",
                "fontSize must be between 1 and 200 pt.",
            ));
        }
        let size = requested
            .or_else(|| card.and_then(|c| c["exportFontSize"].as_f64()))
            .or_else(|| project.and_then(|p| p["exportFontSize"].as_f64()))
            .or_else(|| defaults["exportFontSize"].as_f64())
            .unwrap_or(30.);
        if !size.is_finite() || !(0. ..=200.).contains(&size) {
            return Err(ApiError::new(
                "invalid_arguments",
                "fontSize must be between 1 and 200 pt.",
            ));
        }
        Ok(size)
    }
    fn draft(
        &self,
        args: &schema::Render,
        project: Option<&Value>,
        size: f64,
    ) -> std::result::Result<Value, ApiError> {
        let configuration = instant_settings::load(self.store, self.settings)?;
        let templates = configuration["templates"].as_array().unwrap();
        let selected = args
            .template_id
            .as_ref()
            .map(|id| {
                templates.iter().find(|t| t["id"] == *id).ok_or_else(|| {
                    ApiError::new(
                        "invalid_arguments",
                        "Unknown templateId. Use list_templates.",
                    )
                })
            })
            .transpose()?;
        if project.is_some() && selected.is_some() {
            return Err(ApiError::new(
                "invalid_arguments",
                "Use projectId or templateId, not both.",
            ));
        }
        let format = project
            .map(|p| string(p, "format"))
            .or_else(|| args.language.as_ref().map(schema::Language::as_str))
            .or_else(|| selected.map(|t| string(t, "format")))
            .unwrap_or("latex");
        let template = selected
            .or_else(|| {
                templates.iter().find(|t| {
                    string(t, "format") == format && t["id"] == configuration["defaultTemplateID"]
                })
            })
            .or_else(|| templates.iter().find(|t| string(t, "format") == format))
            .ok_or_else(|| ApiError::new("invalid_arguments", "No matching Instant template."))?;
        if string(template, "format") != format
            || args
                .language
                .as_ref()
                .is_some_and(|language| language.as_str() != format)
        {
            return Err(ApiError::new(
                "invalid_arguments",
                "language must match the project/template format.",
            ));
        }
        let color = args
            .color
            .as_deref()
            .unwrap_or_else(|| string(&configuration, "color"));
        schema::validate_color(color)?;
        let draft = json!({"source":args.source,"kind":args.kind.as_str(),"template":template,"fontSize":size,"color":color,"exportFormat":args.format.as_str(),"contextFile":args.context_file});
        instant::validate(&draft)?;
        Ok(draft)
    }
    fn output_directory(&self, directory: &Path) -> std::result::Result<PathBuf, ApiError> {
        if !directory.is_absolute() {
            return Err(ApiError::new(
                "invalid_arguments",
                "outputDirectory must be an existing absolute directory.",
            ));
        }
        let directory = directory.canonicalize().map_err(|_| {
            ApiError::new(
                "invalid_arguments",
                "Create the outputDirectory before exporting.",
            )
        })?;
        if !directory.is_dir()
            || self.store.recent()?.iter().any(|p| {
                self.store
                    .root(string(p, "id"))
                    .is_ok_and(|root| directory.starts_with(root))
            })
        {
            return Err(ApiError::new(
                "invalid_arguments",
                "Choose an outputDirectory outside all registered project sources.",
            ));
        }
        Ok(directory)
    }
    fn publish(
        &self,
        directory: &Path,
        cards: &[(Value, f64)],
        format: &str,
        project: Option<&Value>,
        cancel: &compiler::Cancellation,
        progress: &mut impl FnMut(Value),
    ) -> ApiResult {
        let directory = self.output_directory(directory)?;
        let output = tempfile::Builder::new()
            .prefix("mathlidelux-")
            .tempdir_in(directory)
            .map_err(err)?;
        let mut results = Vec::new();
        for (index, (card, size)) in cards.iter().enumerate() {
            cancel.check()?;
            if card["status"] != "ready" {
                results.push(json!({"equationId":card["id"],"status":"failed","error":{"code":"render_failed","message":card["error"]}}));
                continue;
            }
            let size = if *size > 0. {
                *size
            } else {
                card["sourceFontSize"].as_f64().unwrap_or(11.)
            };
            let mut card = card.clone();
            if card["restoreMetadata"].is_null() {
                if let Some(p) = project {
                    card["restoreMetadata"] = roundtrip::card_recipe(self.store, p, &card, size)?;
                }
            }
            if card["restoreMetadata"]["draft"].is_object() {
                card["restoreMetadata"]["draft"]["fontSize"] = json!(size);
            }
            let source = export::image_file(
                &card,
                format,
                size,
                &self.store.home.join("Instant/Exports"),
            )?;
            let _lease = crate::cache::Lease::new(&self.store.home, &json!(source));
            let dest = output.path().join(format!(
                "{:03}.{}",
                index + 1,
                if format == "jpeg" { "jpg" } else { format }
            ));
            fs::copy(&source, &dest).map_err(err)?;
            let preview = output.path().join(format!("{:03}-preview.png", index + 1));
            fs::copy(string(&card, "pngPath"), &preview).map_err(err)?;
            let svg = read_bounded(Path::new(string(&card, "svgPath")), ARTIFACT_LIMIT)?;
            let tree = resvg::usvg::Tree::from_data(&svg, &resvg::usvg::Options::default())
                .map_err(err)?;
            let scale = size / card["sourceFontSize"].as_f64().unwrap_or(11.);
            results.push(json!({"equationId":card["id"],"status":"ready","path":dest,"previewPath":preview,"format":format,"fontSize":size,"widthPt":tree.size().width() as f64*0.75*scale,"heightPt":tree.size().height() as f64*0.75*scale}));
            progress(json!({"completed":index+1,"total":cards.len()}));
        }
        let result = json!({"apiVersion":1,"sourceState":if project.is_some(){"saved_project_context"}else{"supplied_source"},"outputDirectory":output.path(),"manifestPath":output.path().join("manifest.json"),"items":results});
        json_write(&output.path().join("manifest.json"), &result)?;
        cancel.check()?;
        let _ = output.keep();
        Ok(result)
    }
}
fn find_card<'a>(project: &'a Value, id: &str) -> std::result::Result<&'a Value, ApiError> {
    project["cards"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["id"] == id)
        .ok_or_else(|| {
            ApiError::new(
                "equation_not_found",
                "Equation not found. Search the library first.",
            )
        })
}

/// Preserve only variants created after this GUI snapshot was taken. Deleting
/// an already-observed variant remains a normal, persistent Library operation.
pub fn merge_unseen_variants(incoming: &Value, current: &Value) -> Value {
    let mut next = incoming.clone();
    let seen = incoming["automationGeneration"].as_u64().unwrap_or(0);
    if let (Some(cards), Some(saved)) = (next["cards"].as_array_mut(), current["cards"].as_array())
    {
        for card in saved {
            if card["automationGeneration"].as_u64().unwrap_or(0) > seen
                && !cards.iter().any(|c| c["id"] == card["id"])
            {
                cards.push(card.clone());
            }
        }
    }
    next["automationGeneration"] =
        json!(seen.max(current["automationGeneration"].as_u64().unwrap_or(0)));
    next
}

#[cfg(test)]
mod tests;
pub(crate) mod verification;
