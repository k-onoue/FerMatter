//! User-editable Library extraction exclusions. Source and document scans stay unfiltered.
use crate::files::Result;
use serde_json::{json, Value};
use std::collections::HashSet;

pub const TABLE_ENVIRONMENTS: &[&str] = &[
    "table",
    "table*",
    "tabular",
    "tabular*",
    "tabularx",
    "longtable",
];

#[derive(Clone, Default)]
pub struct Options {
    pub ignore_math_in_tables: bool,
    pub latex_environments: HashSet<String>,
    pub typst_functions: HashSet<String>,
}

pub fn defaults() -> Value {
    json!({"schemaVersion":1,"kind":"mathlidelux.extraction","ignoreMathInTables":false,"excludedLatexEnvironments":[],"excludedTypstFunctions":[]})
}

impl Options {
    pub fn from_value(value: &Value) -> Result<Self> {
        if !value.is_object()
            || value["schemaVersion"] != 1
            || value["kind"] != "mathlidelux.extraction"
        {
            return Err("Use a version 1 mathlidelux.extraction document.".into());
        }
        let ignore_math_in_tables = value
            .get("ignoreMathInTables")
            .map(|v| {
                v.as_bool()
                    .ok_or("ignoreMathInTables must be true or false.")
            })
            .transpose()?
            .unwrap_or(false);
        let names = |key: &str, typst: bool| -> Result<HashSet<String>> {
            let Some(values) = value.get(key) else {
                return Ok(HashSet::new());
            };
            let values = values
                .as_array()
                .ok_or_else(|| format!("{key} must be a string array."))?;
            if values.len() > 128 {
                return Err(format!("{key} accepts at most 128 names."));
            }
            values.iter().map(|value| {
                let name = value.as_str().ok_or_else(|| format!("{key} must contain strings."))?;
                let valid = !name.is_empty() && name.len() <= 128 && if typst {
                    name.split('.').all(|part| {
                        part.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                            && part.chars().all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
                    })
                } else {
                    name.chars().all(|c| c.is_ascii_alphanumeric() || "_*@:-".contains(c))
                };
                if !valid { return Err(format!("Invalid name in {key}: use literal environment or function names (at most 128 characters).")); }
                Ok(name.to_owned())
            }).collect()
        };
        Ok(Self {
            ignore_math_in_tables,
            latex_environments: names("excludedLatexEnvironments", false)?,
            typst_functions: names("excludedTypstFunctions", true)?,
        })
    }
    pub fn excludes_latex(&self, name: &str) -> bool {
        self.latex_environments.contains(name)
            || self.ignore_math_in_tables && TABLE_ENVIRONMENTS.contains(&name)
    }
    pub fn excludes_typst(&self, name: &str) -> bool {
        self.typst_functions.contains(name) || self.ignore_math_in_tables && name == "table"
    }
    pub fn has_typst_exclusions(&self) -> bool {
        self.ignore_math_in_tables || !self.typst_functions.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_keep_tables_and_custom_names_are_independent() {
        let mut value = defaults();
        let defaults = Options::from_value(&value).unwrap();
        assert!(!defaults.excludes_latex("tabular"));
        assert!(!defaults.excludes_typst("table"));
        value["excludedLatexEnvironments"] = json!(["results", "custom-table*"]);
        value["excludedTypstFunctions"] = json!(["grid", "paper.results"]);
        let options = Options::from_value(&value).unwrap();
        assert!(options.excludes_latex("results"));
        assert!(options.excludes_typst("paper.results"));
        assert!(!options.excludes_latex("table"));
        value["ignoreMathInTables"] = json!(true);
        let options = Options::from_value(&value).unwrap();
        for name in TABLE_ENVIRONMENTS {
            assert!(options.excludes_latex(name));
        }
        assert!(options.excludes_typst("table"));
    }
    #[test]
    fn invalid_documents_are_rejected_without_ignoring_bad_fields() {
        for (key, bad) in [
            ("ignoreMathInTables", json!("true")),
            ("excludedLatexEnvironments", json!(["table}"])),
            ("excludedLatexEnvironments", json!(["a b"])),
            ("excludedTypstFunctions", json!(["table..cell"])),
            ("excludedTypstFunctions", json!(["grid()"])),
            ("excludedTypstFunctions", json!(null)),
        ] {
            let mut value = defaults();
            value[key] = bad;
            assert!(Options::from_value(&value).is_err(), "{key}");
        }
        let mut value = defaults();
        value["excludedLatexEnvironments"] = json!(vec!["table"; 129]);
        assert!(Options::from_value(&value).is_err());
    }
}
