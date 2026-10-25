use super::ApiError;
use rmcp::{
    model::{Tool, ToolAnnotations},
    schemars::{self, JsonSchema},
};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Default, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    Latex,
    Typst,
}
impl Language {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Latex => "latex",
            Self::Typst => "typst",
        }
    }
}
#[derive(Default, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Inline,
    #[default]
    Block,
    Figure,
}
impl Kind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Inline => "inline",
            Self::Block => "block",
            Self::Figure => "figure",
        }
    }
}
#[derive(Default, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    #[default]
    Svg,
    Png,
    Pdf,
    Jpeg,
}
impl Format {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Svg => "svg",
            Self::Png => "png",
            Self::Pdf => "pdf",
            Self::Jpeg => "jpeg",
        }
    }
}
#[derive(Default, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Page {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}
impl Page {
    pub fn bounds(&self) -> Result<(usize, usize), ApiError> {
        let limit = self.limit.unwrap_or(50);
        if !(1..=200).contains(&limit) {
            return Err(ApiError::new(
                "invalid_arguments",
                "limit must be between 1 and 200.",
            ));
        }
        Ok((self.offset.unwrap_or(0), limit))
    }
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Search {
    pub project_id: String,
    pub query: Option<String>,
    pub kind: Option<Kind>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Equation {
    pub project_id: String,
    pub equation_id: String,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Render {
    /// Omit to use an Instant template; provide a registered project ID to use its packages/definitions.
    pub project_id: Option<String>,
    /// Standalone template ID from list_templates. Omit to use the configured default for the language.
    pub template_id: Option<String>,
    /// Defaults to the project's language, or LaTeX for standalone rendering.
    pub language: Option<Language>,
    /// Equation body without math delimiters; figures use their full drawing code.
    pub source: String,
    #[serde(default)]
    pub kind: Kind,
    pub context_file: Option<String>,
    /// Output size in points, 1–200. Omit to inherit app/project settings.
    pub font_size: Option<f64>,
    /// Six-digit #RRGGBB foreground.
    pub color: Option<String>,
    #[serde(default)]
    pub format: Format,
    /// Existing absolute output folder outside project sources. A new child folder is created; files are never overwritten.
    pub output_directory: PathBuf,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Export {
    pub project_id: String,
    /// 1–64 equation IDs, in output order.
    pub equation_ids: Vec<String>,
    pub font_size: Option<f64>,
    #[serde(default)]
    pub format: Format,
    pub output_directory: PathBuf,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Variant {
    pub project_id: String,
    pub equation_id: String,
    pub name: String,
    pub source: String,
    /// Revision returned by search_equations/get_equation. Conflicts never overwrite an existing edit.
    pub expected_revision: String,
    /// Stable caller-generated ID. Repeating the same request does not add another card.
    pub request_id: String,
    pub font_size: Option<f64>,
    pub color: Option<String>,
}
#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Open {
    /// Absolute path to an exported FerMatter SVG containing its editable recipe.
    #[cfg_attr(not(unix), allow(dead_code))] // Read by the Unix resident transport.
    pub path: PathBuf,
}
pub fn validate_color(color: &str) -> Result<(), ApiError> {
    if color.len() != 7
        || !color.starts_with('#')
        || !color[1..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return Err(ApiError::new("invalid_arguments", "color must be #RRGGBB."));
    }
    Ok(())
}
fn tool<T: JsonSchema>(name: &str, description: &str, read_only: bool, idempotent: bool) -> Tool {
    let schema = schemars::schema_for!(T);
    Tool::new(
        name.to_owned(),
        description.to_owned(),
        schema.as_object().unwrap().clone(),
    )
    .annotate(
        ToolAnnotations::new()
            .read_only(read_only)
            .destructive(false)
            .idempotent(idempotent)
            .open_world(false),
    )
}
pub fn tools() -> Vec<Tool> {
    vec![
        tool::<Page>("list_templates", "List configured Instant templates, including TikZ and CeTZ. Returns IDs, languages, and source templates for standalone render_equation.", true, true),
    tool::<Page>("list_projects","List registered local projects. Paginated; no source changes.",true,true),
    tool::<Search>("search_equations","Search saved Library equations by name, source, file, or outline heading. Does not scan unsaved editor changes. Returns a revision for create_variant.",true,true),
    tool::<Equation>("get_equation","Read an equation's complete source and metadata from the saved Library.",true,true),
    tool::<Render>("render_equation","Render LaTeX/Typst (including TikZ/CeTZ) standalone or using a project's saved definitions. Export an image, PNG preview, physical size, and manifest to a new output folder. Does not edit the manuscript or Instant draft.",false,false),
    tool::<Export>("export_equations","Render and export selected saved Library equations in order to a new folder. Returns per-item results, preview paths, and physical sizes. Does not edit sources.",false,false),
    tool::<Variant>("create_variant","Render and add a presentation variant to the Library, preserving the original equation and manuscript. Requires the latest Library revision and a unique requestId. Retry the same requestId/arguments after uncertain completion.",false,true),
    tool::<Open>("open_in_app","Open an exported FerMatter SVG for human review through the app's normal import flow. Opens the matching project editor or Instant; preserves existing drafts. Returns queued, not proof of review.",false,false),
]
}
