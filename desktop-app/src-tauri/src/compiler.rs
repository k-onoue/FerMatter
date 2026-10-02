use crate::{files::*, scan};
use log::{debug, error, info, warn};
use serde_json::{json, Value};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process::Stdio,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Condvar, LazyLock, Mutex,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Default)]
pub struct Cancellation(
    pub Arc<AtomicBool>,
    Arc<Mutex<std::collections::HashSet<u32>>>,
    Arc<crate::toolchain::Toolchain>,
);
impl Cancellation {
    pub fn with_tools(tools: Arc<crate::toolchain::Toolchain>) -> Self {
        Self(Arc::default(), Arc::default(), tools)
    }
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
        for id in self.1.lock().unwrap().iter() {
            terminate_group(*id);
        }
    }
    pub fn check(&self) -> Result<()> {
        if self.0.load(Ordering::SeqCst) {
            Err("Cancelled".into())
        } else {
            Ok(())
        }
    }
}
// Bound expensive compiler snapshots/raster buffers across all windows. Waiting
// requests observe cancellation before entering I/O, so obsolete caret jobs do not queue work.
static WORK: LazyLock<(Mutex<usize>, Condvar)> = LazyLock::new(|| (Mutex::new(0), Condvar::new()));
struct WorkPermit;
impl WorkPermit {
    fn acquire(cancel: &Cancellation) -> Result<Self> {
        let (lock, changed) = &*WORK;
        let mut active = lock.lock().unwrap();
        loop {
            cancel.check()?;
            if *active < 2 {
                *active += 1;
                return Ok(Self);
            }
            active = changed
                .wait_timeout(active, Duration::from_millis(25))
                .unwrap()
                .0;
        }
    }
}
impl Drop for WorkPermit {
    fn drop(&mut self) {
        let (lock, changed) = &*WORK;
        *lock.lock().unwrap() -= 1;
        changed.notify_one();
    }
}
fn terminate_group(id: u32) {
    #[cfg(unix)]
    unsafe {
        libc::kill(-(id as i32), libc::SIGKILL);
    }
}
struct ProcessRegistration {
    id: u32,
    cancellation: Cancellation,
}
impl Drop for ProcessRegistration {
    fn drop(&mut self) {
        self.cancellation.1.lock().unwrap().remove(&self.id);
    }
}
fn run_summary(program: &str, code: Option<i32>, elapsed: Duration, cwd: &Path) -> String {
    let exit = code.map_or("signal".to_string(), |code| code.to_string());
    format!(
        "{program} exit={exit} duration_ms={} cwd={}",
        elapsed.as_millis(),
        cwd.display()
    )
}
fn check_logged(cancel: &Cancellation, program: &str) -> Result<()> {
    cancel.check().inspect_err(|_| info!("{program} cancelled"))
}
pub fn run(program: &str, args: &[String], cwd: &Path, cancel: &Cancellation) -> Result<String> {
    run_captured(program, args, cwd, cancel, false)
}
// JSON-producing commands must not mix successful output with stderr warnings.
fn run_stdout(program: &str, args: &[String], cwd: &Path, cancel: &Cancellation) -> Result<String> {
    run_captured(program, args, cwd, cancel, true)
}
fn run_captured(
    program: &str,
    args: &[String],
    cwd: &Path,
    cancel: &Cancellation,
    stdout_only: bool,
) -> Result<String> {
    check_logged(cancel, program)?;
    debug!("{program} args={args:?} cwd={}", cwd.display());
    let mut log = tempfile::tempfile().map_err(err)?;
    let mut diagnostics = stdout_only
        .then(tempfile::tempfile)
        .transpose()
        .map_err(err)?;
    let mut command = cancel.2.command(program)?;
    command
        .args(args)
        .current_dir(cwd)
        .env_remove("SYNCTEX_EDITOR")
        .env_remove("SYNCTEX_VIEWER")
        .stdin(Stdio::null())
        .stdout(log.try_clone().map_err(err)?)
        .stderr(
            diagnostics
                .as_ref()
                .unwrap_or(&log)
                .try_clone()
                .map_err(err)?,
        );
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command.spawn().map_err(|e| {
        error!("{program} failed to spawn: {e}");
        format!("{program}: {e}")
    })?;
    cancel.1.lock().unwrap().insert(child.id());
    let registration = ProcessRegistration {
        id: child.id(),
        cancellation: cancel.clone(),
    };
    let start = Instant::now();
    let status = loop {
        let log_size = log.metadata().and_then(|m| {
            diagnostics.as_ref().map_or(Ok(m.len()), |d| {
                d.metadata().map(|e| m.len().saturating_add(e.len()))
            })
        });
        let oversized_log = log_size.as_ref().is_ok_and(|size| *size > 8 * 1024 * 1024);
        if cancel.check().is_err()
            || log_size.is_err()
            || oversized_log
            || start.elapsed() > Duration::from_secs(180)
        {
            #[cfg(unix)]
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.kill();
            let _ = child.wait();
            return Err(if cancel.check().is_err() {
                info!("{program} cancelled");
                "Cancelled".into()
            } else if let Err(error) = log_size {
                warn!("{program} output monitoring failed: {error}");
                format!("Unable to monitor compiler output: {error}")
            } else if oversized_log {
                warn!("{program} exceeded the 8 MB output limit");
                format!("{program} exceeded the 8 MB output limit.")
            } else {
                warn!(
                    "{program} timed out after 180 seconds cwd={}",
                    cwd.display()
                );
                format!("{program} timed out after 180 seconds.")
            });
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                terminate_group(child.id());
                let _ = child.kill();
                let _ = child.wait();
                return Err(err(error));
            }
        }
        std::thread::sleep(Duration::from_millis(25));
    };
    drop(registration);
    let summary = run_summary(program, status.code(), start.elapsed(), cwd);
    if status.success() {
        info!("{summary}");
    } else {
        warn!("{summary}");
    }
    check_logged(cancel, program)?;
    let mut bytes = read_output(&mut log, 8 * 1024 * 1024)?;
    if let Some(diagnostics) = &mut diagnostics {
        // Check the shared output budget again after process exit; a fast
        // command can finish between polling iterations. Keep stderr on failure.
        let stderr = read_output(diagnostics, 8 * 1024 * 1024 - bytes.len())?;
        if !status.success() && !stderr.is_empty() {
            bytes.push(b'\n');
            bytes.extend(stderr);
        }
    }
    let output = String::from_utf8_lossy(&bytes).into_owned();
    if !status.success() {
        return Err(output
            .chars()
            .rev()
            .take(16000)
            .collect::<String>()
            .chars()
            .rev()
            .collect());
    }
    check_logged(cancel, program)?;
    Ok(output)
}
fn read_output(file: &mut fs::File, limit: usize) -> Result<Vec<u8>> {
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0)).map_err(err)?;
    let mut bytes = vec![];
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(err)?;
    if bytes.len() > limit {
        return Err("Compiler output exceeds 8 MB.".into());
    }
    Ok(bytes)
}
pub fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}
pub fn path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn tex_args(stage: &Path, engine: &str, out: &Path, main: &str, pdf: bool) -> Result<Vec<String>> {
    let rc = ["latexmkrc", ".latexmkrc"]
        .into_iter()
        .find(|s| stage.join(s).is_file());
    let mut a = args(&["-norc"]);
    if let Some(rc) = rc {
        if engine == "project" {
            // A project rc may only set search paths. Default to direct PDF
            // output before loading it so explicit project modes still win.
            a.extend(args(&["-e", "$pdf_mode = 1;"]));
        }
        a.extend(args(&["-r", rc]))
    }
    match engine {
        "pdflatex" => a.push("-pdf".into()),
        "xelatex" => a.push("-xelatex".into()),
        "lualatex" => a.push("-lualatex".into()),
        "project" if rc.is_some() => {}
        _ => return Err("Choose a LaTeX compiler in Project Settings.".into()),
    }
    if pdf {
        a.push("-synctex=1".into())
    }
    a.extend(args(&[
        "-interaction=nonstopmode",
        "-halt-on-error",
        "-file-line-error",
        "-latexoption=-no-shell-escape",
    ]));
    a.push(format!("-outdir={}", out.display()));
    a.push(main.into());
    Ok(a)
}
fn ensure_pdf(pdf: &Path, stage: &Path, c: &Cancellation) -> Result<()> {
    if !pdf.exists() && pdf.with_extension("dvi").exists() {
        let ps = pdf.with_extension("ps");
        run(
            "dvips",
            &args(&["-o", &path(&ps), &path(&pdf.with_extension("dvi"))]),
            stage,
            c,
        )?;
        run("ps2pdf", &args(&[&path(&ps), &path(pdf)]), stage, c)?;
    }
    if !pdf.is_file() {
        return Err("The compiler did not produce a PDF.".into());
    }
    Ok(())
}
pub(crate) fn tex_body(card: &Value) -> String {
    let text = string(card, "tex").trim();
    if string(card, "kind") == "figure" {
        return text.into();
    }
    let t = scan::uncomment(text);
    let t = scan::re(r"\n[ \t]*\n+").replace_all(&t, "\n");
    let t = scan::re(r"\\(?:label|tag\*?)\{[^}]*\}").replace_all(&t, "");
    let t = scan::re(r"\\(begin|end)\{(equation|align|gather|multline)\*?\}")
        .replace_all(&t, "\\$1{$2*}");
    if scan::re(r"^\\begin\{(?:equation|align|gather|multline|displaymath|math)\*?\}")
        .is_match(t.trim())
    {
        return scan::re(r"\n[ \t]*\n+")
            .replace_all(t.trim(), "\n")
            .into_owned();
    }
    let mut t = t.trim().to_owned();
    for (a, b) in [("$$", "$$"), ("\\[", "\\]"), ("\\(", "\\)"), ("$", "$")] {
        if t.starts_with(a) && t.ends_with(b) && t.len() >= a.len() + b.len() {
            t = t[a.len()..t.len() - b.len()].trim().into();
            break;
        }
    }
    if string(card, "kind") == "inline" {
        format!("${t}$")
    } else {
        format!("\\[{t}\\]")
    }
}
pub(crate) fn typst_body(card: &Value) -> Result<String> {
    let mut t = string(card, "tex").trim().to_owned();
    if t.is_empty() {
        return Err("Enter an equation or drawing.".into());
    }
    if string(card, "kind") == "figure" {
        return Ok(t);
    }
    if t.starts_with('$') {
        if scan::math_end(&t, 0) != Some(t.len()) {
            return Err("Enter one Typst equation.".into());
        }
        t = t[1..t.len() - 1].trim().into()
    }
    Ok(if string(card, "kind") == "inline" {
        format!("#set text(top-edge: \"bounds\", bottom-edge: \"bounds\")\n${t}$")
    } else {
        format!("$ {t} $")
    })
}
fn ready(card: &Value, base: &Path, size: f64) -> Value {
    json!({"id":card["id"],"status":"ready","svgPath":base.with_extension("svg"),"pngPath":base.with_extension("png"),"pdfPath":base.with_extension("pdf"),"sourceFontSize":size})
}
pub fn raster(svg: &[u8], png: &Path, width: u32) -> Result<()> {
    let options = resvg::usvg::Options::default();
    let tree = resvg::usvg::Tree::from_data(svg, &options).map_err(err)?;
    let size = tree.size();
    let scale = width as f32 / size.width().max(size.height());
    let w = (size.width() * scale).ceil().max(1.) as u32;
    let h = (size.height() * scale).ceil().max(1.) as u32;
    if w > 16000 || h > 16000 || u64::from(w) * u64::from(h) > 16_000_000 {
        return Err(
            "Image exceeds 16,000 pixels per side or 16 megapixels. Choose a smaller export size."
                .into(),
        );
    }
    let mut pixmap = resvg::tiny_skia::Pixmap::new(w, h).ok_or("Image is too large")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.save_png(png).map_err(err)
}
fn cache_hit(card: &Value, base: &Path) -> Option<Value> {
    if ["svg", "png", "pdf"]
        .iter()
        .all(|e| base.with_extension(e).is_file())
    {
        let s = String::from_utf8(read_bounded(&base.with_extension("svg"), ARTIFACT_LIMIT).ok()?)
            .ok()?;
        let c = scan::re(r#"data-(?:tex|typst)-font-size="([0-9.]+)"#).captures(&s)?;
        Some(ready(card, base, c[1].parse().ok()?))
    } else {
        None
    }
}
pub(crate) fn card_color(card: &Value) -> Result<Option<&str>> {
    match card.get("instantColor").filter(|v| !v.is_null()) {
        None => Ok(None),
        Some(Value::String(color))
            if color.len() == 6 && color.bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            Ok(Some(color))
        }
        _ => Err("Invalid equation color.".into()),
    }
}

// Restored variants need definitions after the original equation as well. Find
// include ancestry even when the selected file contains no extracted equations.
fn restored_typst_context(
    root: &Path,
    main: &str,
    file: &str,
    c: &Cancellation,
) -> Result<Vec<Value>> {
    if !file.ends_with(".typ") || file.contains(['\\', ':', '\0']) {
        return Err("Choose a Typst context file inside the project.".into());
    }
    let root = root.canonicalize().map_err(err)?;
    let selected = safe(&root, file)?;
    let target = selected.strip_prefix(&root).map_err(err)?.to_string_lossy();
    let text = read(&selected)?;
    let final_frame = json!({"file":target,"offset":text.chars().count()});
    struct ContextScan<'a> {
        root: &'a Path,
        target: &'a str,
        cancel: &'a Cancellation,
        bytes: usize,
        visited: std::collections::HashSet<String>,
    }
    impl ContextScan<'_> {
        fn walk(&mut self, name: &str, depth: usize) -> Result<Option<Vec<Value>>> {
            self.cancel.check()?;
            if name == self.target {
                return Ok(Some(vec![]));
            }
            if depth >= 32 || !self.visited.insert(name.to_owned()) {
                return Ok(None);
            }
            if self.visited.len() > 30_000 {
                return Err("Typst context exceeds 30,000 files.".into());
            }
            let text = read(&safe(self.root, name)?)?;
            self.bytes += text.len();
            if self.bytes > 100 * 1024 * 1024 {
                return Err("Typst context exceeds 100 MB.".into());
            }
            for token in scan::tokens_checked(&text, &|| self.cancel.check())? {
                if token.kind != "include" {
                    continue;
                }
                let child = token.extra.as_str().unwrap_or("");
                let path = if child.starts_with('/') {
                    self.root.join(child.trim_start_matches('/'))
                } else {
                    self.root.join(name).parent().unwrap().join(child)
                };
                let Ok(path) = path.canonicalize() else {
                    continue;
                };
                let Ok(relative) = path.strip_prefix(self.root) else {
                    continue;
                };
                let relative = relative.to_string_lossy();
                if relative.is_empty() || !path.is_file() {
                    continue;
                }
                if let Some(mut frames) = self.walk(&relative, depth + 1)? {
                    frames.insert(
                        0,
                        json!({"file":name,"offset":text[..token.start].chars().count()}),
                    );
                    return Ok(Some(frames));
                }
            }
            Ok(None)
        }
    }
    let mut frames = ContextScan {
        root: &root,
        target: &target,
        cancel: c,
        bytes: text.len(),
        visited: Default::default(),
    }
    .walk(main, 0)?
    .unwrap_or_default();
    frames.push(final_frame);
    Ok(frames)
}

fn prepare_cards(root: &Path, main: &str, cards: &[Value], c: &Cancellation) -> Result<Vec<Value>> {
    // This gate must precede cache hits, filesystem reads, and compiler startup.
    for card in cards {
        if card["requiresRender"] == true {
            return Err("Press Render to compile imported source.".into());
        }
        card_color(card)?;
    }
    let mut contexts = std::collections::HashMap::new();
    cards
        .iter()
        .map(|card| {
            c.check()?;
            let mut card = card.clone();
            if main.ends_with(".typ") {
                if let Some(value) = card.get("restoredContextFile").filter(|v| !v.is_null()) {
                    let file = value.as_str().ok_or("Invalid restored context file.")?;
                    if !contexts.contains_key(file) {
                        contexts.insert(
                            file.to_owned(),
                            restored_typst_context(root, main, file, c)?,
                        );
                    }
                    card["typstContext"] = json!(contexts[file]);
                }
            }
            Ok(card)
        })
        .collect()
}

pub fn render(
    root: &Path,
    main: &str,
    engine: &str,
    cards: &[Value],
    cache: &Path,
    c: &Cancellation,
    mut emit: impl FnMut(Value),
) -> Result<()> {
    let _permit = WorkPermit::acquire(c)?;
    let cards = prepare_cards(root, main, cards, c)?;
    let lease = crate::cache::Lease::new(cache, &Value::Null);
    let staging = tempfile::tempdir().map_err(err)?;
    let mut deliver = |mut event: Value| {
        if string(&event, "status") == "ready" {
            let paths: Vec<_> = ["svgPath", "pngPath", "pdfPath"]
                .iter()
                .map(|key| {
                    let source = PathBuf::from(string(&event, key));
                    let dest = cache.join(source.file_name().unwrap());
                    (source, dest)
                })
                .collect();
            match crate::cache::publish_checked(&paths, &|| c.check()) {
                Ok(()) => {
                    for (key, (_, dest)) in ["svgPath", "pngPath", "pdfPath"].iter().zip(&paths) {
                        event[*key] = json!(dest);
                    }
                    lease.add(&event);
                }
                Err(error) => {
                    event = json!({"id":event["id"],"status":"failed","error":error});
                }
            }
            for (source, _) in paths {
                if source.starts_with(staging.path()) {
                    let _ = fs::remove_file(source);
                }
            }
        } else {
            // Diagnostics are in the response. Do not accumulate intermediate
            // images or logs for failed cards in a large render batch.
            let log = Path::new(string(&event, "logPath"));
            if log.starts_with(staging.path()) {
                for ext in ["svg", "png", "pdf", "log"] {
                    let _ = fs::remove_file(log.with_extension(ext));
                }
            }
            event.as_object_mut().unwrap().remove("logPath");
        }
        if c.check().is_ok() {
            emit(event);
        }
    };
    fs::create_dir_all(cache).map_err(err)?;
    let stamp = fingerprint_checked(root, &|| c.check())?;
    let typst = main.ends_with(".typ");
    let version = if typst {
        run("typst", &args(&["--version"]), root, c)?
    } else {
        run("latexmk", &args(&["-v"]), root, c)?
    };
    let mut pending = vec![];
    let toolchain = c.2.fingerprint();
    for card in &cards {
        c.check()?;
        let mut cache_key = format!(
            "rust-2|{stamp}|{main}|{engine}|{version}|{toolchain}|{}|{}|{}",
            card["kind"], card["tex"], card["typstContext"]
        );
        if let Some(color) = card["instantColor"].as_str() {
            cache_key.push_str(&format!("|instant-color:{color}"));
        }
        let key = id(&cache_key);
        let base = cache.join(key);
        let paths = json!([
            base.with_extension("svg"),
            base.with_extension("png"),
            base.with_extension("pdf")
        ]);
        lease.add(&paths);
        if let Some(e) = cache_hit(card, &base) {
            deliver(e)
        } else {
            pending.push((card.clone(), staging.path().join(base.file_name().unwrap())))
        }
    }
    if pending.is_empty() {
        return c.check();
    }
    let temp = tempfile::tempdir().map_err(err)?;
    let stage = temp.path().join("source");
    snapshot_checked(root, &stage, &Value::Null, &|| c.check())?;
    let stage = stage.canonicalize().map_err(err)?;
    let output = temp.path().join("output");
    fs::create_dir_all(&output).map_err(err)?;
    if typst {
        for (card, base) in pending {
            c.check()?;
            let result = render_typst(&stage, main, &card, &base, c);
            match result {
                Ok(e) => deliver(e),
                Err(e) => {
                    c.check()?;
                    let log = base.with_extension("log");
                    fs::write(&log, &e).map_err(err)?;
                    deliver(json!({"id":card["id"],"status":"failed","error":e,"logPath":log}));
                }
            }
        }
    } else {
        let source = read(&safe(&stage, main)?)?;
        let begin = scan::re(r"\\begin\s*\{document\}")
            .find(&source)
            .ok_or("Main file has no document environment")?
            .start();
        let preamble = &source[..begin];
        for group in pending.chunks(32) {
            render_tex_group(&stage, &output, engine, preamble, group, c, &mut deliver)?
        }
    }
    c.check()
}
fn render_typst(
    stage: &Path,
    main: &str,
    card: &Value,
    base: &Path,
    c: &Cancellation,
) -> Result<Value> {
    let original = read(&safe(stage, main)?)?;
    let default_offset = scan::tokens_checked(&original, &|| c.check())?
        .iter()
        .find(|t| matches!(t.kind, "math" | "figure" | "include"))
        .map(|t| original[..t.start].chars().count())
        .unwrap_or(original.chars().count());
    let default = vec![json!({"file":main,"offset":default_offset})];
    let frames = card["typstContext"]
        .as_array()
        .filter(|v| !v.is_empty())
        .unwrap_or(&default);
    let token = format!("mathlidelux-{}", uuid::Uuid::new_v4());
    let mut child: Option<PathBuf> = None;
    let mut generated = vec![];
    let result = (|| {
        for (i, frame) in frames.iter().enumerate().rev() {
            let p = safe(stage, string(frame, "file"))?;
            let text = read(&p)?;
            let prelude = scan::setup_before(
                &text,
                scan::offset(&text, frame["offset"].as_u64().unwrap_or(0) as usize),
            );
            let target = p.parent().unwrap().join(format!("{token}-{i}.typ"));
            let content = if let Some(ref child) = child {
                format!(
                    "#include {}",
                    json!(format!("/{}", child.strip_prefix(stage).unwrap().display()))
                )
            } else {
                let color = card_color(card)?
                    .map(|s| format!("#set text(fill: rgb(\"{s}\"))\n"))
                    .unwrap_or_default();
                format!("#set page(width: auto, height: auto, margin: 2pt, fill: none, header: none, footer: none, background: none, foreground: none, numbering: none)\n#set math.equation(numbering: none)\n{color}#context [#metadata(text.size.pt()) <slide-math-font>]\n{}\n",typst_body(card)?)
            };
            fs::write(&target, format!("{prelude}\n{content}")).map_err(err)?;
            generated.push(target.clone());
            child = Some(target)
        }
        let child = child.ok_or("Missing Typst context")?;
        let common = args(&["--root", &path(stage), "--font-path", &path(stage)]);
        for ext in ["pdf", "svg"] {
            let mut a = args(&["compile"]);
            a.extend(common.clone());
            a.extend(args(&[&path(&child), &path(&base.with_extension(ext))]));
            run("typst", &a, stage, c)?;
        }
        let mut a = args(&["query"]);
        a.extend(common);
        a.extend(args(&[
            &path(&child),
            "<slide-math-font>",
            "--field",
            "value",
        ]));
        let sizes: Vec<f64> =
            serde_json::from_str(&run_stdout("typst", &a, stage, c)?).map_err(err)?;
        let size = *sizes
            .first()
            .filter(|s| s.is_finite() && **s > 0.)
            .ok_or("Could not measure font size")?;
        let svg = String::from_utf8(read_bounded(&base.with_extension("svg"), ARTIFACT_LIMIT)?)
            .map_err(err)?
            .replacen(
                "<svg ",
                &format!("<svg data-typst-font-size=\"{size}\" data-slide-math-bounds=\"glyph\" "),
                1,
            );
        raster(svg.as_bytes(), &base.with_extension("png"), 900)?;
        fs::write(base.with_extension("svg"), svg).map_err(err)?;
        Ok(ready(card, base, size))
    })();
    for p in generated {
        let _ = fs::remove_file(p);
    }
    result
}
fn render_tex_group(
    stage: &Path,
    output: &Path,
    engine: &str,
    preamble: &str,
    group: &[(Value, PathBuf)],
    c: &Cancellation,
    emit: &mut impl FnMut(Value),
) -> Result<()> {
    c.check()?;
    let parent_output = output;
    let temporary = tempfile::tempdir_in(parent_output).map_err(err)?;
    let output = temporary.path();
    let job = format!("mathlidelux-{}", uuid::Uuid::new_v4());
    let result = (|| -> Result<f64> {
        let figures = group.iter().any(|(v, _)| string(v, "kind") == "figure");
        let mut previews = vec![];
        let mut colored = false;
        for (card, _) in group {
            let body = if let Some(color) = card_color(card)? {
                colored = true;
                format!("{{\\color[HTML]{{{color}}}\n{}\n}}", tex_body(card))
            } else {
                tex_body(card)
            };
            previews.push(format!("\\begin{{preview}}\n{body}\n\\end{{preview}}"));
        }
        let document=format!("\\PassOptionsToPackage{{active,tightpage}}{{preview}}\n{preamble}\n{}\n{}\n\\usepackage{{preview}}\n\\pagestyle{{empty}}\n\\begin{{document}}\n\\ifdefined\\ClearShipoutPictureBG\\ClearShipoutPictureBG\\fi\n\\ifdefined\\ClearShipoutPictureFG\\ClearShipoutPictureFG\\fi\n\\ifdefined\\nolinenumbers\\nolinenumbers\\fi\n\\ifdefined\\tikzexternaldisable\\tikzexternaldisable\\fi\n\\makeatletter\\typeout{{SLIDEMATHFONT:\\f@size}}\\makeatother\n{}\n\\end{{document}}\n",if figures{"\\usepackage{tikz}"}else{""},if colored{"\\usepackage{xcolor}"}else{""},previews.join("\n"));
        fs::write(stage.join(format!("{job}.tex")), document).map_err(err)?;
        run(
            "latexmk",
            &tex_args(stage, engine, output, &format!("{job}.tex"), false)?,
            stage,
            c,
        )?;
        let pdf = output.join(format!("{job}.pdf"));
        ensure_pdf(&pdf, stage, c)?;
        let cropped = output.join(format!("{job}-crop.pdf"));
        run(
            "pdfcrop",
            &args(&["--margins", "2", &path(&pdf), &path(&cropped)]),
            stage,
            c,
        )?;
        let info = run("pdfinfo", &args(&[&path(&cropped)]), stage, c)?;
        let pages = scan::re(r"(?m)^Pages:\s+(\d+)")
            .captures(&info)
            .and_then(|v| v[1].parse::<usize>().ok());
        if pages != Some(group.len()) {
            return Err("Rendered page count does not match equation count.".into());
        }
        let log = String::from_utf8(read_bounded(
            &output.join(format!("{job}.log")),
            8 * 1024 * 1024,
        )?)
        .map_err(err)?;
        let size = scan::re(r"SLIDEMATHFONT:([0-9.]+)")
            .captures(&log)
            .and_then(|v| v[1].parse::<f64>().ok())
            .ok_or("Could not measure font size")?;
        for (index, (card, base)) in group.iter().enumerate() {
            c.check()?;
            let page = (index + 1).to_string();
            let pdf = base.with_extension("pdf");
            run(
                "pdftocairo",
                &args(&[
                    "-pdf",
                    "-origpagesizes",
                    "-f",
                    &page,
                    "-l",
                    &page,
                    &path(&cropped),
                    &path(&pdf),
                ]),
                stage,
                c,
            )?;
            let svg = base.with_extension("svg");
            run(
                "pdf2svg",
                &args(&[&path(&cropped), &path(&svg), &page]),
                stage,
                c,
            )?;
            let text = String::from_utf8(read_bounded(&svg, ARTIFACT_LIMIT)?)
                .map_err(err)?
                .replacen("<svg ", &format!("<svg data-tex-font-size=\"{size}\" "), 1);
            raster(text.as_bytes(), &base.with_extension("png"), 900)?;
            fs::write(svg, text).map_err(err)?;
            emit(ready(card, base, size));
        }
        Ok(size)
    })();
    let _ = fs::remove_file(stage.join(format!("{job}.tex")));
    drop(temporary);
    if let Err(e) = result {
        c.check()?;
        if group.len() > 1 {
            let half = group.len() / 2;
            render_tex_group(
                stage,
                parent_output,
                engine,
                preamble,
                &group[..half],
                c,
                emit,
            )?;
            render_tex_group(
                stage,
                parent_output,
                engine,
                preamble,
                &group[half..],
                c,
                emit,
            )?
        } else {
            let (card, base) = &group[0];
            let log = base.with_extension("log");
            fs::write(&log, &e).map_err(err)?;
            emit(json!({"id":card["id"],"status":"failed","error":e,"logPath":log}));
        }
    }
    Ok(())
}
pub fn document(
    root: &Path,
    main: &str,
    engine: &str,
    cache: &Path,
    drafts: &Value,
    c: &Cancellation,
) -> Result<Value> {
    let _permit = WorkPermit::acquire(c)?;
    let dest = cache.join(format!("document-{}", uuid::Uuid::new_v4()));
    let work = crate::cache::DocumentWork::new(&dest);
    let stage = dest.join("Source");
    let out = dest.join("Output");
    fs::create_dir_all(&out).map_err(err)?;
    let result = (|| {
        snapshot_checked(root, &stage, drafts, &|| c.check())?;
        let targets = scan::extract_checked(&stage, main, None, &|| c.check())?["cards"].clone();
        // Typst navigation inserts metadata into its build sources. Keep an
        // untouched snapshot for exporting the artwork visible in this PDF.
        let render_root = if main.ends_with(".typ") {
            let original = dest.join("Original");
            snapshot_checked(&stage, &original, &Value::Null, &|| c.check())?;
            original
        } else {
            stage.clone()
        };
        let (pdf, anchors) = if main.ends_with(".typ") {
            typst_document(&stage, main, &out, c)?
        } else {
            run(
                "latexmk",
                &tex_args(&stage, engine, &out, main, true)?,
                &stage,
                c,
            )?;
            let pdf = out
                .join(Path::new(main).file_stem().unwrap())
                .with_extension("pdf");
            ensure_pdf(&pdf, &stage, c)?;
            (pdf, vec![])
        };
        c.check()?;
        if regular_file(&pdf)?.metadata().map_err(err)?.len() > ARTIFACT_LIMIT {
            return Err("Compiled PDF exceeds the 100 MiB limit.".into());
        }
        work.commit()?;
        Ok(
            json!({"pdfPath":pdf,"sourceRoot":stage,"renderRoot":render_root,"main":main,"engine":engine,"format":if main.ends_with(".typ"){"typst"}else{"latex"},"anchors":anchors,"targets":targets}),
        )
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&dest);
    }
    result
}
pub fn document_artwork(
    document: &Value,
    target_id: &str,
    cache: &Path,
    c: &Cancellation,
) -> Result<Value> {
    c.check()?;
    let mut card = document["targets"]
        .as_array()
        .and_then(|targets| targets.iter().find(|t| string(t, "id") == target_id))
        .cloned()
        .ok_or("No exportable equation at this PDF position.")?;
    let root = Path::new(string(document, "renderRoot"));
    let main = string(document, "main");
    if main.is_empty() || !root.is_dir() {
        return Err("Rebuild the PDF before dragging its artwork.".into());
    }
    render(
        root,
        main,
        string(document, "engine"),
        &[card.clone()],
        cache,
        c,
        |event| {
            for (key, value) in event.as_object().unwrap() {
                card[key] = value.clone();
            }
        },
    )?;
    c.check()?;
    if string(&card, "status") != "ready" {
        return Err(string(&card, "error").to_owned());
    }
    Ok(card)
}

pub fn drag_target(document: &Value, request: &Value, c: &Cancellation) -> Result<Value> {
    let location = navigate(document, request, c)?;
    let line = location["line"].as_u64().unwrap_or(0);
    let targets: Vec<_> = document["targets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|target| {
            target["file"] == location["file"]
                && target["line"].as_u64().unwrap_or(u64::MAX) <= line
                && target["endLine"].as_u64().unwrap_or(0) >= line
        })
        .collect();
    // SyncTeX can report only a line. Never export an arbitrary item when
    // several equations share it; ordinary click-to-source still works.
    if targets.len() != 1 {
        return Ok(Value::Null);
    }
    let target = targets[0];
    let mut anchor = navigate(
        document,
        &json!({"file":target["file"],"line":target["line"]}),
        c,
    )?;
    let x = request["x"].as_f64().unwrap_or(-1.);
    let y = request["y"].as_f64().unwrap_or(-1.);
    let left = anchor["x"].as_f64().unwrap_or(0.);
    let top = anchor["y"].as_f64().unwrap_or(0.);
    let width = anchor["width"].as_f64().unwrap_or(0.);
    let height = anchor["height"].as_f64().unwrap_or(0.);
    if anchor["page"] != request["page"]
        || width <= 0.
        || height <= 0.
        || x < left - 3.
        || x > left + width + 3.
        || y < top - 3.
        || y > top + height + 3.
    {
        return Ok(Value::Null);
    }
    anchor["targetID"] = target["id"].clone();
    Ok(anchor)
}
fn typst_document(
    stage: &Path,
    main: &str,
    out: &Path,
    c: &Cancellation,
) -> Result<(PathBuf, Vec<Value>)> {
    let scan = scan::extract_checked(stage, main, None, &|| c.check())?;
    let mut changes: std::collections::BTreeMap<String, std::collections::BTreeMap<usize, String>> =
        Default::default();
    let mut targets: std::collections::HashMap<String, Vec<Value>> = Default::default();
    let ns = format!("ml-{}", uuid::Uuid::new_v4());
    let label = format!("map-{}", uuid::Uuid::new_v4());
    for card in scan["cards"].as_array().unwrap() {
        let file = string(card, "file");
        let text = read(&safe(stage, file)?)?;
        let start = scan::offset(&text, card["sourceOffset"].as_u64().unwrap() as usize);
        let end = scan::offset(&text, card["sourceEnd"].as_u64().unwrap() as usize);
        let key = format!("{file}:{start}");
        targets
            .entry(key.clone())
            .or_default()
            .push(card["id"].clone());
        let edit = changes.entry(file.into()).or_default();
        if string(card, "kind") == "figure" {
            edit.insert(start, "context { let ml_drawing = ".into());
            edit.insert(end,format!("; let ml_size = measure(ml_drawing); metadata(({}, {}, ml_size.width.pt(), ml_size.height.pt())); ml_drawing }}",json!(ns),json!(key)));
        } else {
            let mut pos = start + 1;
            while pos < text.len() && text[pos..].starts_with(char::is_whitespace) {
                pos = scan::next(&text, pos)
            }
            edit.insert(pos, format!("#metadata(({}, {})) ", json!(ns), json!(key)));
        }
    }
    for h in scan["outline"].as_array().unwrap() {
        let file = string(h, "file");
        let text = read(&safe(stage, file)?)?;
        let pos = text
            .split_inclusive('\n')
            .take(h["line"].as_u64().unwrap_or(1) as usize - 1)
            .map(str::len)
            .sum();
        let key = format!("{file}:heading:{pos}");
        targets
            .entry(key.clone())
            .or_default()
            .push(h["id"].clone());
        changes
            .entry(file.into())
            .or_default()
            .insert(pos, format!("#metadata(({}, {}))\n", json!(ns), json!(key)));
    }
    for (file, edits) in changes {
        let p = safe(stage, &file)?;
        let mut text = read(&p)?;
        for (i, s) in edits.iter().rev() {
            text.insert_str(*i, s)
        }
        fs::write(p, text).map_err(err)?;
    }
    let entry = safe(stage, main)?;
    let query=format!("\n#context [#metadata(query(metadata).filter(it => type(it.value) == array and (it.value.len() == 2 or it.value.len() == 4) and it.value.at(0) == {}).map(it => {{let p = it.location().position(); (key: it.value.at(1), page: p.page, x: p.x.pt(), y: p.y.pt(), width: if it.value.len() == 4 {{it.value.at(2)}} else {{0}}, height: if it.value.len() == 4 {{it.value.at(3)}} else {{0}})}})) <{label}>]\n",json!(ns));
    let text = read(&entry)? + &query;
    fs::write(&entry, text).map_err(err)?;
    let pdf = out.join("document.pdf");
    let common = args(&["--root", &path(stage), "--font-path", &path(stage)]);
    let mut a = args(&["compile"]);
    a.extend(common.clone());
    a.extend(args(&[&path(&entry), &path(&pdf)]));
    run("typst", &a, stage, c)?;
    let mut a = args(&["query"]);
    a.extend(common);
    a.extend(args(&[
        &path(&entry),
        &format!("<{label}>"),
        "--field",
        "value",
        "--one",
    ]));
    let records: Vec<Value> =
        serde_json::from_str(&run_stdout("typst", &a, stage, c)?).map_err(err)?;
    let mut anchors = vec![];
    for mut record in records {
        if let Some(ids) = targets.get_mut(string(&record, "key")) {
            if !ids.is_empty() {
                record["targetID"] = ids.remove(0);
                anchors.push(record);
            }
        }
    }
    Ok((pdf, anchors))
}

pub fn navigate(document: &Value, request: &Value, c: &Cancellation) -> Result<Value> {
    let file = string(request, "file");
    let mut line = request["line"].as_u64().unwrap_or(1);
    // TeX emits display/align nodes at the end of the environment. Its opening
    // line (and even early body lines in align) can map to preceding prose.
    if let Some(target) = document["targets"].as_array().and_then(|targets| {
        targets
            .iter()
            .find(|t| t["file"] == file && t["line"].as_u64() == Some(line))
    }) {
        if string(target, "kind") != "inline" {
            line = target["endLine"].as_u64().unwrap_or(line);
        }
    }
    let page = request["page"].as_u64().unwrap_or(1);
    let x = request["x"].as_f64().unwrap_or(0.);
    let y = request["y"].as_f64().unwrap_or(0.);
    let reverse = request["reverse"].as_bool().unwrap_or(false);
    let stage = Path::new(string(document, "sourceRoot"));
    let pdf = string(document, "pdfPath");
    let output = if reverse {
        run(
            "synctex",
            &args(&["edit", "-o", &format!("{page}:{x}:{y}:{pdf}")]),
            stage,
            c,
        )?
    } else {
        let file = safe(stage, file)?;
        run(
            "synctex",
            &args(&[
                "view",
                "-i",
                &format!("{line}:1:{}", file.display()),
                "-o",
                pdf,
            ]),
            stage,
            c,
        )?
    };
    let mut records = vec![];
    let mut record = std::collections::HashMap::new();
    for line in output.lines() {
        if let Some((key, value)) = line.split_once(':') {
            if key == "Output" && record.contains_key("Page") {
                records.push(std::mem::take(&mut record));
            }
            record
                .entry(key.to_owned())
                .or_insert(value.trim().to_owned());
        }
    }
    records.push(record);
    let record = &records[0];
    if reverse {
        let file = record
            .get("Input")
            .ok_or("No source location at this PDF position.")?;
        let file = stage.join(file).canonicalize().map_err(err)?;
        let root = stage.canonicalize().map_err(err)?;
        if !file.starts_with(&root) {
            return Err("PDF source is outside the snapshot".into());
        }
        Ok(
            json!({"file":file.strip_prefix(root).unwrap(),"line":record.get("Line").and_then(|s|s.parse::<u64>().ok()).unwrap_or(1),"column":record.get("Column").and_then(|s|s.parse::<i64>().ok()).unwrap_or(-1)}),
        )
    } else {
        let number = |r: &std::collections::HashMap<String, String>, key: &str| {
            r.get(key).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.)
        };
        let x = number(record, "x");
        let y = number(record, "y");
        // Prefer the tight equation box sharing the first hit's left edge;
        // the first outer box can include the distant equation number.
        let bounds = records
            .iter()
            .filter(|r| {
                r.get("Page") == record.get("Page")
                    && (number(r, "h") - x).abs() < 0.2
                    && number(r, "W") > 0.
                    && number(r, "H") > 0.
                    && number(r, "v") >= y
                    && number(r, "v") - number(r, "H") <= y
            })
            .min_by(|a, b| number(a, "W").total_cmp(&number(b, "W")))
            .unwrap_or(record);
        Ok(
            json!({"page":record.get("Page").and_then(|s|s.parse::<u64>().ok()).ok_or("No PDF location for this source line.")?,"x":number(bounds,"h"),"y":number(bounds,"v")-number(bounds,"H"),"width":number(bounds,"W"),"height":number(bounds,"H")}),
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn query_stdout_remains_json_when_stderr_contains_warnings() {
        let root = tempfile::tempdir().unwrap();
        let cancel = super::Cancellation::default();
        for json in [
            "[1.0,2.0]",
            r#"[{"key":"equation","page":1,"x":10,"y":20}]"#,
        ] {
            let script = format!(
                "printf '%s\\n' '{json}'; printf '%s\\n' 'warning: query is deprecated' >&2"
            );
            let args = super::args(&["-c", &script]);
            let output = super::run_stdout("/bin/sh", &args, root.path(), &cancel).unwrap();
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&output).unwrap(),
                serde_json::from_str::<serde_json::Value>(json).unwrap()
            );
            // Other compiler commands still return their combined diagnostics.
            assert!(super::run("/bin/sh", &args, root.path(), &cancel)
                .unwrap()
                .contains("warning: query is deprecated"));
        }
    }

    #[test]
    fn query_failure_keeps_stderr_and_invalid_stdout_is_not_silently_trimmed() {
        let root = tempfile::tempdir().unwrap();
        let cancel = super::Cancellation::default();
        let error = super::run_stdout(
            "/bin/sh",
            &super::args(&[
                "-c",
                "printf 'partial'; printf 'unknown variable' >&2; exit 1",
            ]),
            root.path(),
            &cancel,
        )
        .unwrap_err();
        assert!(error.contains("partial") && error.contains("unknown variable"));
        let output = super::run_stdout(
            "/bin/sh",
            &super::args(&["-c", "printf '[1] invalid'"]),
            root.path(),
            &cancel,
        )
        .unwrap();
        assert!(serde_json::from_str::<serde_json::Value>(&output).is_err());
    }

    #[test]
    fn query_output_limit_counts_both_streams() {
        let root = tempfile::tempdir().unwrap();
        let cancel = super::Cancellation::default();
        let error = super::run_stdout(
            "/bin/sh",
            &super::args(&[
                "-c",
                "head -c 5000000 /dev/zero; head -c 4000000 /dev/zero >&2",
            ]),
            root.path(),
            &cancel,
        )
        .unwrap_err();
        assert!(error.contains("8 MB"), "{error}");
        // Also cover the final read after a fast child exits between polls.
        let mut file = tempfile::tempfile().unwrap();
        use std::io::Write;
        file.write_all(b"12345").unwrap();
        assert!(super::read_output(&mut file, 4).is_err());
    }

    #[test]
    fn run_summary_reports_program_exit_duration_and_cwd() {
        let cwd = std::path::Path::new("/tmp/work");
        let ok = super::run_summary(
            "typst",
            Some(0),
            std::time::Duration::from_millis(1234),
            cwd,
        );
        assert_eq!(ok, "typst exit=0 duration_ms=1234 cwd=/tmp/work");
        let killed = super::run_summary("latexmk", None, std::time::Duration::ZERO, cwd);
        assert_eq!(killed, "latexmk exit=signal duration_ms=0 cwd=/tmp/work");
    }

    #[test]
    fn imported_cards_require_render_before_files_or_compilers_are_accessed() {
        let temp = tempfile::tempdir().unwrap();
        let cache = temp.path().join("unused-cache");
        let error = super::render(
            &temp.path().join("missing-project"),
            "missing.typ",
            "project",
            &[serde_json::json!({"requiresRender":true})],
            &cache,
            &super::Cancellation::default(),
            |_| panic!("unexpected artwork"),
        )
        .unwrap_err();
        assert!(error.contains("Press Render"));
        assert!(!cache.exists());
    }

    #[test]
    fn restored_typst_context_includes_ancestors_and_late_definitions_without_equations() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("sections")).unwrap();
        let main = "#set text(size: 19pt)\n日本語\n#include \"sections/definitions.typ\"\n";
        let definitions = "#let early = 1\n#let last(x) = x + early\n";
        fs::write(temp.path().join("main.typ"), main).unwrap();
        fs::write(temp.path().join("sections/definitions.typ"), definitions).unwrap();
        let input = json!({"tex":"last(2)","kind":"block","restoredContextFile":"sections/definitions.typ","typstContext":[{"file":"missing.typ","offset":0}]});
        let cards =
            prepare_cards(temp.path(), "main.typ", &[input], &Cancellation::default()).unwrap();
        assert_eq!(
            cards[0]["typstContext"],
            json!([
                {"file":"main.typ","offset":main[..main.find("#include").unwrap()].chars().count()},
                {"file":"sections/definitions.typ","offset":definitions.chars().count()}
            ])
        );
        for path in [
            "../main.typ",
            "/main.typ",
            "sections\\definitions.typ",
            "missing.typ",
        ] {
            assert!(restored_typst_context(
                temp.path(),
                "main.typ",
                path,
                &Cancellation::default()
            )
            .is_err());
        }
    }

    #[cfg(unix)]
    #[test]
    fn restored_context_rejects_symlinks_outside_the_project() {
        let project = tempfile::tempdir().unwrap();
        let external = tempfile::tempdir().unwrap();
        fs::write(external.path().join("private.typ"), "#let private = 1").unwrap();
        std::os::unix::fs::symlink(
            external.path().join("private.typ"),
            project.path().join("context.typ"),
        )
        .unwrap();
        assert!(restored_typst_context(
            project.path(),
            "main.typ",
            "context.typ",
            &Cancellation::default()
        )
        .is_err());
    }

    #[test]
    fn restored_color_rejects_markup_and_accepts_hex() {
        assert_eq!(card_color(&json!({})).unwrap(), None);
        assert_eq!(
            card_color(&json!({"instantColor":"aB1234"})).unwrap(),
            Some("aB1234")
        );
        for color in [json!("#123456"), json!("123456}\\input{secret}"), json!(1)] {
            assert!(card_color(&json!({"instantColor":color})).is_err());
        }
    }

    #[test]
    fn oversized_raster_is_rejected_before_pixel_allocation() {
        let temp = tempfile::tempdir().unwrap();
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"/>"#;
        assert!(super::raster(svg, &temp.path().join("large.png"), 16000)
            .unwrap_err()
            .contains("16 megapixels"));
        assert!(!temp.path().join("large.png").exists());
    }
    #[test]
    fn queued_work_stops_when_cancelled() {
        let c = super::Cancellation::default();
        c.cancel();
        assert!(super::WorkPermit::acquire(&c).is_err());
    }

    use super::*;
    #[test]
    fn cancellation_kills_process_group() {
        for stdout_only in [false, true] {
            let d = tempfile::tempdir().unwrap();
            let c = Cancellation::default();
            let other = c.clone();
            let root = d.path().to_owned();
            let thread = std::thread::spawn(move || {
                run_captured(
                    "/bin/sh",
                    &args(&["-c", "sleep 30 & wait"]),
                    &root,
                    &other,
                    stdout_only,
                )
            });
            std::thread::sleep(Duration::from_millis(100));
            c.cancel();
            assert_eq!(thread.join().unwrap().unwrap_err(), "Cancelled");
        }
    }
    #[test]
    fn body_preserves_math_style() {
        assert_eq!(
            tex_body(&json!({"kind":"block","tex":"\\begin{align}a&=b\\label{x}\\end{align}"})),
            "\\begin{align*}a&=b\\end{align*}"
        );
        assert!(typst_body(&json!({"kind":"inline","tex":"$a$ $b$"})).is_err());
    }
}
