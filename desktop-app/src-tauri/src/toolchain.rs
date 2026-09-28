use crate::files::*;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

const TOOLS: &[(&str, &str)] = &[
    ("typst", "Compilers"),
    ("latexmk", "Compilers"),
    ("pdflatex", "Compilers"),
    ("xelatex", "Compilers"),
    ("lualatex", "Compilers"),
    ("latex", "Additional tools"),
    ("pdftex", "Additional tools"),
    ("pdfcrop", "Additional tools"),
    ("pdf2svg", "Additional tools"),
    ("pdfinfo", "Additional tools"),
    ("pdftocairo", "Additional tools"),
    ("synctex", "Additional tools"),
    ("dvips", "Additional tools"),
    ("ps2pdf", "Additional tools"),
    ("gs", "Additional tools"),
];

pub fn defaults() -> Value {
    json!({"schemaVersion":1,"kind":"mathlidelux.tools","paths":{}})
}

pub fn validate(value: &Value) -> Result<()> {
    if value["schemaVersion"] != 1 || value["kind"] != "mathlidelux.tools" {
        return Err("Unsupported compiler tools format.".into());
    }
    let paths = value["paths"]
        .as_object()
        .ok_or("Tool paths must be an object.")?;
    for (name, path) in paths {
        if !TOOLS.iter().any(|(tool, _)| tool == name) {
            return Err(format!("Unknown tool: {name}"));
        }
        let path = path.as_str().ok_or("Tool paths must be strings.")?;
        if (!path.is_empty() && !Path::new(path).is_absolute())
            || path.chars().any(char::is_control)
        {
            return Err(format!("{name}: choose an absolute executable path, or leave it empty for automatic detection."));
        }
    }
    Ok(())
}

#[derive(Default)]
pub struct Toolchain {
    paths: BTreeMap<String, String>,
    // Each job retains its own launchers until its subprocesses have finished.
    launchers: OnceLock<Result<tempfile::TempDir>>,
}

impl Toolchain {
    pub fn from_value(value: &Value) -> Result<Self> {
        validate(value)?;
        Ok(Self {
            paths: value["paths"]
                .as_object()
                .unwrap()
                .iter()
                .filter_map(|(name, path)| {
                    let path = path.as_str().unwrap();
                    (!path.is_empty()).then(|| (name.clone(), path.to_owned()))
                })
                .collect(),
            launchers: OnceLock::new(),
        })
    }
    pub fn value(&self) -> Value {
        json!({"schemaVersion":1,"kind":"mathlidelux.tools","paths":self.paths})
    }
    pub fn fingerprint(&self) -> String {
        hash(self.value().to_string())
    }
    fn search_paths(&self) -> Vec<PathBuf> {
        let mut paths: Vec<_> = self
            .paths
            .values()
            .filter_map(|p| Path::new(p).parent().map(Path::to_path_buf))
            .collect();
        paths.extend(
            [
                "/Library/TeX/texbin",
                "/opt/homebrew/bin",
                "/usr/local/bin",
                "/usr/bin",
                "/bin",
            ]
            .map(PathBuf::from),
        );
        paths.extend(
            std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
                .filter(|p| p.is_absolute()),
        );
        paths
    }
    fn resolve(&self, name: &str) -> Result<PathBuf> {
        if let Some(path) = self.paths.get(name) {
            let path = PathBuf::from(path);
            executable(&path).map_err(|e| format!("{name}: {e}"))?;
            return Ok(path);
        }
        self.search_paths()
            .into_iter()
            .map(|p| p.join(name))
            .find(|p| executable(p).is_ok())
            .ok_or_else(|| format!("{name}: not found"))
    }
    pub fn check_overrides(&self) -> Result<()> {
        for name in self.paths.keys() {
            self.resolve(name)?;
        }
        Ok(())
    }
    pub fn status(&self) -> Value {
        let tools: Vec<_> = TOOLS
            .iter()
            .map(|(name, group)| match self.resolve(name) {
                Ok(path) => json!({"name":name,"group":group,"path":path}),
                Err(error) => json!({"name":name,"group":group,"error":error}),
            })
            .collect();
        json!({"configuration":self.value(),"tools":tools})
    }
    fn environment_path(&self) -> Result<OsString> {
        let mut paths = self.search_paths();
        if !self.paths.is_empty() {
            let launchers = self
                .launchers
                .get_or_init(|| {
                    let dir = tempfile::tempdir().map_err(err)?;
                    for (name, path) in &self.paths {
                        // latexmk/pdfcrop/ps2pdf invoke other tools by name. A small
                        // launcher preserves exact overrides (including missing ones)
                        // instead of silently falling through to a different binary.
                        let quoted = path.replace('\'', "'\\''");
                        let file = dir.path().join(name);
                        fs::write(&file, format!("#!/bin/sh\nexec '{quoted}' \"$@\"\n"))
                            .map_err(err)?;
                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            fs::set_permissions(file, fs::Permissions::from_mode(0o700))
                                .map_err(err)?;
                        }
                    }
                    Ok(dir)
                })
                .as_ref()
                .map_err(Clone::clone)?;
            paths.insert(0, launchers.path().to_owned());
        }
        std::env::join_paths(paths).map_err(err)
    }
    pub fn command(&self, name: &str) -> Result<Command> {
        // Do not canonicalize TeX symlinks: argv[0] selects the engine format.
        let executable = if TOOLS.iter().any(|(tool, _)| *tool == name) {
            self.resolve(name)?
        } else {
            PathBuf::from(name)
        };
        let mut command = Command::new(executable);
        command.env("PATH", self.environment_path()?);
        Ok(command)
    }
}

fn executable(path: &Path) -> Result<()> {
    let metadata = fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a file.", path.display()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(format!("{} is not executable.", path.display()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compiler, settings::SettingsFiles};
    use std::{os::unix::fs::PermissionsExt, sync::Arc};

    fn script(base: &Path, name: &str, body: &str) -> PathBuf {
        let file = base.join(name);
        fs::write(&file, format!("#!/bin/sh\n{body}\n")).unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o700)).unwrap();
        file
    }
    fn config(paths: Value) -> Value {
        json!({"schemaVersion":1,"kind":"mathlidelux.tools","paths":paths})
    }
    #[test]
    fn validates_paths_and_recovers_saved_configuration_without_overwriting_external_edits() {
        for paths in [
            json!({"bad":"/bin/sh"}),
            json!({"typst":"relative"}),
            json!({"typst":1}),
            json!({"typst":"/bad\npath"}),
        ] {
            assert!(validate(&config(paths)).is_err());
        }
        let base = tempfile::tempdir().unwrap();
        let settings = SettingsFiles::new(base.path());
        assert_eq!(settings.load("tools.json").unwrap(), defaults());
        let value = config(json!({"typst":"/bin/sh"}));
        settings
            .save(
                "tools.json",
                &value,
                string(&settings.snapshot("tools.json", false).unwrap(), "revision"),
            )
            .unwrap();
        let file = base.path().join("Settings/tools.json");
        fs::write(&file, "invalid").unwrap();
        assert!(settings
            .save(
                "tools.json",
                &defaults(),
                string(&settings.snapshot("tools.json", false).unwrap(), "revision")
            )
            .is_err());
        assert!(settings.reload("tools.json").is_err());
        assert_eq!(
            SettingsFiles::new(base.path()).load("tools.json").unwrap(),
            value
        );
        assert_eq!(fs::read_to_string(&file).unwrap(), "invalid");
        json_write(&file, &defaults()).unwrap();
        assert_eq!(settings.reload("tools.json").unwrap(), defaults());
    }
    #[test]
    fn selected_executables_and_child_engines_keep_literal_paths_and_arguments() {
        let base = tempfile::tempdir().unwrap();
        let engine = script(
            base.path(),
            "custom engine's $(untrusted)",
            "printf '<%s>\\n' \"$@\"",
        );
        let driver = script(base.path(), "custom latexmk", "pdflatex \"$@\"");
        let tools =
            Toolchain::from_value(&config(json!({"latexmk":driver,"pdflatex":engine}))).unwrap();
        tools.check_overrides().unwrap();
        let cancel = compiler::Cancellation::with_tools(Arc::new(tools));
        let args = compiler::args(&["a b", "$(no-command)", "quote'\"", ""]);
        let result = compiler::run("latexmk", &args, base.path(), &cancel).unwrap();
        assert_eq!(result, "<a b>\n<$(no-command)>\n<quote'\">\n<>\n");
        assert_eq!(
            compiler::run("pdflatex", &args, base.path(), &cancel).unwrap(),
            result
        );
    }
    #[test]
    fn missing_or_non_executable_overrides_never_fall_back_including_child_tools() {
        let base = tempfile::tempdir().unwrap();
        let missing = base.path().join("missing-engine");
        let driver = script(base.path(), "driver", "pdflatex --version");
        let tools =
            Toolchain::from_value(&config(json!({"latexmk":driver,"pdflatex":missing}))).unwrap();
        assert!(tools.check_overrides().is_err());
        let cancel = compiler::Cancellation::with_tools(Arc::new(tools));
        assert!(compiler::run("pdflatex", &[], base.path(), &cancel)
            .unwrap_err()
            .contains("missing-engine"));
        assert!(compiler::run("latexmk", &[], base.path(), &cancel)
            .unwrap_err()
            .contains("missing-engine"));
        fs::write(&missing, "not executable").unwrap();
        assert!(compiler::run("pdflatex", &[], base.path(), &cancel)
            .unwrap_err()
            .contains("not executable"));
    }
    #[test]
    fn concurrent_jobs_keep_independent_tool_choices_and_reset_restores_detection() {
        let base = tempfile::tempdir().unwrap();
        let driver = script(base.path(), "driver", "pdflatex");
        let first = script(base.path(), "one", "printf first");
        let second = script(base.path(), "two", "printf second");
        let first =
            Toolchain::from_value(&config(json!({"latexmk":driver,"pdflatex":first}))).unwrap();
        let second =
            Toolchain::from_value(&config(json!({"latexmk":driver,"pdflatex":second}))).unwrap();
        assert_ne!(first.fingerprint(), second.fingerprint());
        let run = |tools| {
            let cwd = base.path().to_owned();
            std::thread::spawn(move || {
                compiler::run(
                    "latexmk",
                    &[],
                    &cwd,
                    &compiler::Cancellation::with_tools(Arc::new(tools)),
                )
                .unwrap()
            })
        };
        let a = run(first);
        let b = run(second);
        assert_eq!(a.join().unwrap(), "first");
        assert_eq!(b.join().unwrap(), "second");
        let reset = Toolchain::from_value(&config(json!({"pdflatex":""}))).unwrap();
        assert_eq!(reset.fingerprint(), Toolchain::default().fingerprint());
        assert_eq!(reset.value(), defaults());
    }
    #[test]
    fn changing_a_tool_path_invalidates_rendered_artwork_with_the_same_version() {
        let base = tempfile::tempdir().unwrap();
        let source = base.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("main.typ"), "$ x $").unwrap();
        let body = r#"case "$1" in
--version) printf 'same version';;
query) printf '[11]';;
compile) for target do :; done
case "$target" in
*.svg) printf '<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10"/></svg>' > "$target";;
*.pdf) printf 'test fixture' > "$target";;
esac;;
esac"#;
        let a = script(base.path(), "typst-one", body);
        let b = script(base.path(), "typst-two", body);
        let render = |tool| {
            let tools = Toolchain::from_value(&config(json!({"typst":tool}))).unwrap();
            let c = compiler::Cancellation::with_tools(Arc::new(tools));
            let mut output = Value::Null;
            compiler::render(
                &source,
                "main.typ",
                "typst",
                &[json!({"id":"sample", "kind":"inline", "tex":"x"})],
                &base.path().join("cache"),
                &c,
                |event| output = event,
            )
            .unwrap();
            assert_eq!(output["status"], "ready", "{output}");
            output["svgPath"].clone()
        };
        let first = render(&a);
        assert_eq!(first, render(&a));
        assert_ne!(first, render(&b));
    }
}
