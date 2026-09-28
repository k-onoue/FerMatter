//! Text-only Typst formatting. The parser runs in an isolated copy of this
//! executable: malformed/deep syntax must never overflow the resident's stack.
use crate::{compiler::Cancellation, files::*};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::{Condvar, LazyLock, Mutex},
    time::{Duration, Instant},
};
use typst_syntax::{LinkedNode, Source, SyntaxKind};

const SOURCE_LIMIT: usize = 1024 * 1024;
const JSON_LIMIT: usize = 8 * SOURCE_LIMIT;
const WORKER_FLAG: &str = "--mathlidelux-format-typst";
const TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Mode {
    Document,
    Math,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Input {
    source: String,
    mode: Mode,
}
#[derive(Deserialize, Serialize)]
struct Output {
    result: Option<String>,
    error: Option<String>,
}
impl From<Result<String>> for Output {
    fn from(value: Result<String>) -> Self {
        match value {
            Ok(result) => Self {
                result: Some(result),
                error: None,
            },
            Err(error) => Self {
                result: None,
                error: Some(error),
            },
        }
    }
}

static WORK: LazyLock<(Mutex<usize>, Condvar)> = LazyLock::new(|| (Mutex::new(0), Condvar::new()));
struct Permit;
impl Permit {
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
                .wait_timeout(active, Duration::from_millis(20))
                .unwrap()
                .0;
        }
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        let (lock, changed) = &*WORK;
        *lock.lock().unwrap() -= 1;
        changed.notify_one();
    }
}
fn input(value: &Value) -> Result<Input> {
    let source = value["source"]
        .as_str()
        .ok_or("Expected Typst source text.")?;
    if source.len() > SOURCE_LIMIT {
        return Err("Format up to 1 MiB of Typst source at once.".into());
    }
    let mode = match string(value, "mode") {
        "document" => Mode::Document,
        "math" => Mode::Math,
        _ => return Err("Choose document or math formatting.".into()),
    };
    Ok(Input {
        source: source.into(),
        mode,
    })
}

pub fn format(value: &Value, cancel: &Cancellation) -> Result<Value> {
    let input = input(value)?;
    let _permit = Permit::acquire(cancel)?;
    let result = run(
        &std::env::current_exe().map_err(err)?,
        &input,
        cancel,
        TIMEOUT,
    )?;
    Ok(Value::String(result))
}
fn run(
    program: &std::path::Path,
    input: &Input,
    cancel: &Cancellation,
    timeout: Duration,
) -> Result<String> {
    cancel.check()?;
    let bytes = serde_json::to_vec(input).map_err(err)?;
    let mut child = Command::new(program)
        .arg(WORKER_FLAG)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start Typst formatter: {e}"))?;
    let mut stdin = child.stdin.take().ok_or("Formatter input unavailable.")?;
    let stdout = child.stdout.take().ok_or("Formatter output unavailable.")?;
    std::thread::scope(|scope| {
        let writer = scope.spawn(move || stdin.write_all(&bytes));
        let reader = scope.spawn(move || -> Result<Vec<u8>> {
            let mut output = Vec::new();
            stdout
                .take(JSON_LIMIT as u64 + 1)
                .read_to_end(&mut output)
                .map_err(err)?;
            if output.len() > JSON_LIMIT {
                return Err("Formatted Typst output is too large.".into());
            }
            Ok(output)
        });
        let started = Instant::now();
        let status = loop {
            if let Err(error) = cancel.check() {
                let _ = child.kill();
                let _ = child.wait();
                break Err(error);
            }
            if started.elapsed() > timeout {
                let _ = child.kill();
                let _ = child.wait();
                break Err("Typst formatting timed out. The source was not changed.".into());
            }
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(10)),
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break Err(err(error));
                }
            }
        };
        // The worker never launches descendants. Killing it closes both pipes,
        // allowing these bounded readers/writers to finish before returning.
        let written = writer
            .join()
            .map_err(|_| "Formatter input thread failed.")?;
        let output = reader
            .join()
            .map_err(|_| "Formatter output thread failed.")??;
        let status = status?;
        if !status.success() {
            return Err("Typst formatting failed. The source was not changed.".into());
        }
        written.map_err(err)?;
        let output: Output =
            serde_json::from_slice(&output).map_err(|_| "Invalid Typst formatter response.")?;
        cancel.check()?;
        match (output.result, output.error) {
            (Some(result), None) if result.len() <= SOURCE_LIMIT => Ok(result),
            (_, Some(error)) => Err(error),
            _ => Err("Invalid or oversized Typst formatter output.".into()),
        }
    })
}

/// Runs before Tauri, storage, windows or single-instance registration. It reads
/// bounded stdin and emits JSON; source/project paths are never accepted.
pub fn worker_requested() -> bool {
    if std::env::args_os().nth(1).as_deref() != Some(std::ffi::OsStr::new(WORKER_FLAG)) {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        // Never write a core dump containing editor text on a parser abort.
        libc::setrlimit(
            libc::RLIMIT_CORE,
            &libc::rlimit {
                rlim_cur: 0,
                rlim_max: 0,
            },
        );
    }
    std::panic::set_hook(Box::new(|_| {}));
    let result = std::panic::catch_unwind(|| {
        let mut bytes = Vec::new();
        std::io::stdin()
            .take(JSON_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(err)?;
        if bytes.len() > JSON_LIMIT {
            return Err("Typst formatter input is too large.".into());
        }
        let input: Input =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid Typst formatter request.")?;
        format_text(&input)
    })
    .unwrap_or_else(|_| Err("Typst formatting failed. The source was not changed.".into()));
    let _ = serde_json::to_writer(std::io::stdout().lock(), &Output::from(result));
    true
}

fn lone_equation(source: &Source) -> Option<LinkedNode<'_>> {
    let mut found = None;
    for node in LinkedNode::new(source.root()).children() {
        match node.kind() {
            SyntaxKind::Space
            | SyntaxKind::Parbreak
            | SyntaxKind::LineComment
            | SyntaxKind::BlockComment => {}
            SyntaxKind::Equation if found.is_none() => found = Some(node),
            _ => return None,
        }
    }
    found
}
fn checked(source: String) -> Result<Source> {
    let source = Source::detached(source);
    if source.root().diagnosis().errors {
        return Err("Typst has syntax errors. The source was not changed.".into());
    }
    // The parser itself caps recursive parsing at 256. Its output may still
    // contain deep left-associative chains; reject them before pretty-printing.
    let mut nodes = vec![(source.root(), 0)];
    while let Some((node, depth)) = nodes.pop() {
        if depth > 128 {
            return Err("Typst source is too deeply nested to format.".into());
        }
        nodes.extend(node.children().map(|child| (child, depth + 1)));
    }
    Ok(source)
}
fn formatted(source: Source) -> Result<String> {
    typstyle_core::Typstyle::default()
        .format_source(source)
        .render()
        .map_err(err)
}
fn final_newline(mut output: String, input: &str) -> String {
    if !input.ends_with('\n') && !input.ends_with('\r') {
        while output.ends_with('\n') {
            output.pop();
        }
    } else if !output.ends_with('\n') {
        output.push('\n');
    }
    output
}
fn format_text(input: &Input) -> Result<String> {
    if input.source.len() > SOURCE_LIMIT {
        return Err("Format up to 1 MiB of Typst source at once.".into());
    }
    let source = input.source.replace("\r\n", "\n").replace('\r', "\n");
    let output = match input.mode {
        Mode::Document => final_newline(formatted(checked(source.clone())?)?, &source),
        Mode::Math => {
            // Keep an explicit equation's delimiters and inline/block style.
            let document = checked(source.clone());
            if let Ok(document) = document.as_ref() {
                if lone_equation(document).is_some() {
                    let output = final_newline(formatted(document.clone())?, &source);
                    if output.len() > SOURCE_LIMIT {
                        return Err("Formatted Typst source exceeds 1 MiB.".into());
                    }
                    return Ok(output);
                }
            }
            // Newlines ensure a final // comment cannot consume our closing $.
            let wrapped = checked(format!("$\n{source}\n$"))?;
            let equation = lone_equation(&wrapped).ok_or("Enter one Typst equation to format.")?;
            if equation.range() != (0..wrapped.text().len()) {
                return Err("Enter one Typst equation to format.".into());
            }
            let output = checked(formatted(wrapped)?)?;
            let equation =
                lone_equation(&output).ok_or("Formatting changed the equation boundaries.")?;
            let mut delimiters = equation
                .children()
                .filter(|node| node.kind() == SyntaxKind::Dollar);
            let start = delimiters
                .next()
                .ok_or("Missing Typst opening delimiter.")?
                .range()
                .end;
            let end = delimiters
                .next()
                .ok_or("Missing Typst closing delimiter.")?
                .range()
                .start;
            if delimiters.next().is_some() {
                return Err("Formatting changed the equation boundaries.".into());
            }
            // Comments next to $ belong to the equation node, outside its Math
            // child. Keep the whole delimiter interior, not only the Math range.
            let text = output.text()[start..end].trim().to_owned();
            final_newline(text, &source)
        }
    };
    if output.len() > SOURCE_LIMIT {
        return Err("Formatted Typst source exceeds 1 MiB.".into());
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn apply(source: &str, mode: Mode) -> Result<String> {
        format_text(&Input {
            source: source.into(),
            mode,
        })
    }
    #[test]
    fn formats_typst_documents_and_cetz_without_compiling_or_importing_packages() {
        let source="#import \"@preview/cetz:0.3.4\":canvas,draw\n#canvas({import draw:*;line((0,0),(1,2),stroke:blue)})\n";
        let formatted = apply(source, Mode::Document).unwrap();
        assert!(formatted.contains("(0, 0), (1, 2)"));
        assert!(formatted.contains("stroke: blue"));
        assert_eq!(apply(&formatted, Mode::Document).unwrap(), formatted);
    }
    #[test]
    fn bare_math_is_unwrapped_and_comments_strings_and_alignment_are_retained() {
        for source in [
            "frac( a , b ) + x^2",
            "a &= b \\\n &= c",
            "x // final comment",
            "/* before */ cases(1, \"yes\", 0, \"no\")",
            "text(\"cost $5\") + x",
            "x \\$ y",
        ] {
            let formatted = apply(source, Mode::Math).unwrap();
            assert!(!formatted.starts_with('$'));
            assert_eq!(
                apply(&formatted, Mode::Math).unwrap(),
                formatted,
                "{source}"
            );
            if source.contains("comment") {
                assert!(formatted.contains("// final comment"));
            }
        }
    }
    #[test]
    fn explicit_math_preserves_delimiters_and_display_style() {
        for source in ["$frac( a ,b )$", "$ frac( a ,b ) $", "$ x // comment\n $\n"] {
            let formatted = apply(source, Mode::Math).unwrap();
            let before = Source::detached(source);
            let after = Source::detached(formatted.clone());
            let equation = |source: &Source| {
                lone_equation(source)
                    .unwrap()
                    .get()
                    .cast::<typst_syntax::ast::Equation>()
                    .unwrap()
                    .block()
            };
            assert_eq!(equation(&before), equation(&after));
            assert_eq!(apply(&formatted, Mode::Math).unwrap(), formatted);
        }
    }
    #[test]
    fn invalid_syntax_or_math_boundaries_are_rejected_without_guessing() {
        for (source, mode) in [
            ("#let x = (1,", Mode::Document),
            ("frac(a,", Mode::Math),
            ("x $ y $ z", Mode::Math),
            ("$x$ $y$", Mode::Math),
        ] {
            assert!(apply(source, mode).is_err(), "{source}");
        }
        assert!(input(&serde_json::json!({"source":false,"mode":"math"})).is_err());
        assert!(input(&serde_json::json!({"source":"x","mode":"unknown"})).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn isolated_workers_are_cancelled_timed_out_and_crashes_do_not_escape() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let program = directory.path().join("formatter");
        // Shell builtins only: these fixtures must not leave descendants holding
        // stdout after cancellation. The real helper never spawns any process.
        std::fs::write(&program, "#!/bin/sh\nwhile :; do :; done\n").unwrap();
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700)).unwrap();
        let input = Input {
            source: "x".into(),
            mode: Mode::Math,
        };
        let cancel = Cancellation::default();
        let started = Instant::now();
        assert!(run(&program, &input, &cancel, Duration::from_millis(30))
            .unwrap_err()
            .contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(2));
        let cancellation = cancel.clone();
        std::thread::scope(|scope| {
            scope.spawn(move || {
                std::thread::sleep(Duration::from_millis(30));
                cancellation.cancel();
            });
            assert_eq!(
                run(&program, &input, &cancel, TIMEOUT).unwrap_err(),
                "Cancelled"
            );
        });
        std::fs::write(&program, "#!/bin/sh\nexit 7\n").unwrap();
        assert!(run(&program, &input, &Cancellation::default(), TIMEOUT)
            .unwrap_err()
            .contains("source was not changed"));
    }
    #[test]
    fn size_nesting_and_newline_policies_are_explicit() {
        assert!(apply(&"x".repeat(SOURCE_LIMIT + 1), Mode::Document).is_err());
        assert!(apply(
            &format!("#{}1{}", "(".repeat(300), ")".repeat(300)),
            Mode::Document
        )
        .is_err());
        assert!(apply("#let x=(1,2)", Mode::Document)
            .unwrap()
            .ends_with(')'));
        let formatted = apply("#let x=(1,2)\r\n", Mode::Document).unwrap();
        assert!(formatted.ends_with('\n'));
        assert!(!formatted.contains('\r'));
    }
}
