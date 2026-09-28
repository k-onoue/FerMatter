//! Literal source extraction. Positions stored in project JSON use Unicode scalars,
//! matching the original worker; editor positions additionally use UTF-16.
use crate::files::*;
use regex::Regex;
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{LazyLock, Mutex},
};
static REGEX: LazyLock<Mutex<HashMap<String, Regex>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub fn re(pattern: &str) -> Regex {
    REGEX
        .lock()
        .unwrap()
        .entry(pattern.into())
        .or_insert_with(|| Regex::new(pattern).unwrap())
        .clone()
}
pub fn next(s: &str, i: usize) -> usize {
    i + s[i..].chars().next().map(char::len_utf8).unwrap_or(0)
}
pub fn offset(s: &str, n: usize) -> usize {
    s.char_indices().nth(n).map(|(i, _)| i).unwrap_or(s.len())
}
pub fn line(s: &str, i: usize) -> usize {
    s[..i].bytes().filter(|c| *c == b'\n').count() + 1
}
fn skip_space(s: &str, mut i: usize) -> usize {
    while i < s.len() && s[i..].starts_with(char::is_whitespace) {
        i = next(s, i)
    }
    i
}
fn balanced(s: &str, start: usize, open: u8, close: u8) -> usize {
    let mut depth = 0;
    let mut i = start;
    while i < s.len() {
        let b = s.as_bytes()[i];
        if b == b'\\' {
            i = next(s, i + 1);
            continue;
        }
        if open == b'[' && b == b'{' {
            i = balanced(s, i, b'{', b'}');
            continue;
        }
        if b == open {
            depth += 1
        }
        if b == close {
            depth -= 1;
            if depth == 0 {
                return i + 1;
            }
        }
        i = next(s, i);
    }
    s.len()
}
pub fn uncomment(s: &str) -> String {
    let mut bytes = s.as_bytes().to_vec();
    let mut escaped = false;
    let mut comment = false;
    for (i, c) in s.char_indices() {
        if c == '\n' {
            comment = false
        }
        if c == '%' && !escaped {
            comment = true
        }
        if comment {
            bytes[i..i + c.len_utf8()].fill(b' ')
        }
        escaped = c == '\\' && !escaped;
    }
    String::from_utf8(bytes).unwrap()
}
#[derive(Clone, Debug)]
pub struct Token {
    pub kind: &'static str,
    pub start: usize,
    pub end: usize,
    pub extra: Value,
}
fn string_end(s: &str, start: usize) -> usize {
    let mut i = start + 1;
    while i < s.len() {
        match s.as_bytes()[i] {
            b'\\' => i = next(s, (i + 1).min(s.len())),
            b'"' => return i + 1,
            _ => i = next(s, i),
        }
    }
    s.len()
}
fn comment_end(s: &str, start: usize) -> usize {
    if s[start..].starts_with("//") {
        return s[start..].find('\n').map(|n| n + start).unwrap_or(s.len());
    }
    if !s[start..].starts_with("/*") {
        return start;
    }
    let mut i = start + 2;
    let mut depth = 1;
    while i < s.len() {
        if s[i..].starts_with("/*") {
            depth += 1;
            i += 2
        } else if s[i..].starts_with("*/") {
            depth -= 1;
            i += 2;
            if depth == 0 {
                return i;
            }
        } else {
            i = next(s, i)
        }
    }
    s.len()
}
fn raw_end(s: &str, start: usize) -> usize {
    let n = s[start..].bytes().take_while(|c| *c == b'`').count();
    s[start + n..]
        .find(&"`".repeat(n))
        .map(|i| start + n + i + n)
        .unwrap_or(s.len())
}
pub fn math_end(s: &str, start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < s.len() {
        let end = comment_end(s, i);
        if end > i {
            i = end;
            continue;
        }
        match s.as_bytes()[i] {
            b'\\' => i = next(s, (i + 1).min(s.len())),
            b'"' => i = string_end(s, i),
            b'`' => i = raw_end(s, i),
            b'$' => return Some(i + 1),
            _ => i = next(s, i),
        }
    }
    None
}
fn code_end(s: &str, start: usize, mut expr: bool) -> usize {
    let mut stack = vec![];
    let mut i = start;
    while i < s.len() {
        let end = comment_end(s, i);
        if end > i {
            i = end;
            continue;
        }
        let c = s.as_bytes()[i];
        match c {
            b'"' => {
                i = string_end(s, i);
                continue;
            }
            b'`' => {
                i = raw_end(s, i);
                continue;
            }
            b'$' => {
                i = math_end(s, i).unwrap_or(s.len());
                continue;
            }
            b'(' | b'[' | b'{' => stack.push(c),
            b')' | b']' | b'}' => {
                let expected = match c {
                    b')' => b'(',
                    b']' => b'[',
                    _ => b'{',
                };
                if stack.pop() != Some(expected) {
                    return i;
                }
                if expr && stack.is_empty() {
                    let tail = &s[i + 1..];
                    if tail.starts_with('[') || tail.starts_with('(') {
                    } else if re(r"^[ \t]+if\b").is_match(tail) {
                        expr = false
                    } else {
                        return i + 1;
                    }
                }
            }
            b';' if stack.is_empty() => return i + 1,
            b'\n' if stack.is_empty() => {
                let t = s[start..i].trim_end();
                if !t.ends_with('=') && !t.ends_with("=>") && !t.ends_with(',') {
                    return i;
                }
            }
            _ => {}
        }
        i = next(s, i)
    }
    i
}
pub fn tokens(s: &str) -> Vec<Token> {
    let mut out = vec![];
    let mut i = 0;
    let mut names = HashSet::from(["cetz.canvas".to_owned()]);
    while i < s.len() {
        let end = comment_end(s, i);
        if end > i {
            i = end;
            continue;
        }
        let tail = &s[i..];
        let c = s.as_bytes()[i];
        if c == b'\\' {
            i = next(s, (i + 1).min(s.len()));
            continue;
        }
        if c == b'`' {
            i = raw_end(s, i);
            continue;
        }
        if c == b'=' && s[..i].rsplit('\n').next().unwrap_or("").trim().is_empty() {
            if let Some(h) = re(r"^(={1,6})[ \t]+([^\n]*)").captures(tail) {
                let title = re(r"\s*<[^<>]+>\s*$")
                    .replace(h[2].split("//").next().unwrap_or(""), "")
                    .replace(['*', '_', '`', '$'], "");
                let end = i + h[0].len();
                out.push(Token {
                    kind: "heading",
                    start: i,
                    end,
                    extra: json!([h[1].len(), title]),
                });
                i = end;
                continue;
            }
        }
        if c == b'$' {
            let end = math_end(s, i);
            out.push(Token {
                kind: if end.is_some() { "math" } else { "unclosed" },
                start: i,
                end: end.unwrap_or(s.len()),
                extra: Value::Null,
            });
            i = end.unwrap_or(s.len());
            continue;
        }
        if c == b'#' {
            if let Some(cap) = re(r##"^#include\s+("(?:\\.|[^"\\])*")"##).captures(tail) {
                let end = i + cap[0].len();
                out.push(Token {
                    kind: "include",
                    start: i,
                    end,
                    extra: serde_json::from_str(&cap[1]).unwrap_or(Value::Null),
                });
                i = end;
                continue;
            }
            if let Some(cap) =
                re(r"^#(let|set|import|show|include|for|if|while|context)\b").captures(tail)
            {
                let kind = &cap[1];
                let end = code_end(s, i, kind == "set");
                let directive = &s[i..end];
                if kind == "import" {
                    if let Some(c) = re(r##"^#import\s+"@(?:preview|local)/cetz:[^"]+"(.*)"##)
                        .captures(directive)
                    {
                        let tail = c[1].trim();
                        if let Some(a) = re(r"^as\s+([\w-]+)").captures(tail) {
                            names.insert(format!("{}.canvas", &a[1]));
                        }
                        if let Some(items) = tail.strip_prefix(':') {
                            if items.trim().trim_end_matches(';') == "*" {
                                names.insert("canvas".into());
                            }
                            for item in items.split(',') {
                                if let Some(a) =
                                    re(r"^canvas(?:\s+as\s+([\w-]+))?;?$").captures(item.trim())
                                {
                                    names.insert(
                                        a.get(1).map(|m| m.as_str()).unwrap_or("canvas").into(),
                                    );
                                }
                            }
                        }
                    }
                }
                let supported = matches!(kind, "let" | "set" | "import")
                    || kind == "show" && re(r"^#show\s+math\.equation\b").is_match(directive);
                out.push(Token {
                    kind: if supported { "setup" } else { "unsupported" },
                    start: i,
                    end,
                    extra: json!(kind),
                });
                i = end.max(i + 1);
                continue;
            }
            if re(r"^#(?:[A-Za-z_][\w.-]*(?:\s*\(|\s*\[)|[({])").is_match(tail) {
                let end = code_end(s, i, true);
                let mut j = i + 1;
                let mut found = false;
                while j < end {
                    let skip = comment_end(s, j);
                    if skip > j {
                        j = skip;
                        continue;
                    }
                    match s.as_bytes()[j] {
                        b'"' => {
                            j = string_end(s, j);
                            continue;
                        }
                        b'`' => {
                            j = raw_end(s, j);
                            continue;
                        }
                        b'$' => {
                            j = math_end(s, j).unwrap_or(end);
                            continue;
                        }
                        _ => {}
                    }
                    let mut matched = false;
                    for name in &names {
                        if s[j..].starts_with(name)
                            && (j == 0
                                || !s[..j]
                                    .chars()
                                    .next_back()
                                    .is_some_and(|c| c.is_alphanumeric() || "_.-".contains(c)))
                            && s[j + name.len()..].trim_start().starts_with('(')
                        {
                            let last = code_end(s, j, true);
                            if last <= end && s[..last].ends_with(')') {
                                out.push(Token {
                                    kind: "figure",
                                    start: j,
                                    end: last,
                                    extra: json!(i),
                                });
                                j = last;
                                matched = true;
                                found = true;
                                break;
                            }
                        }
                    }
                    if !matched {
                        j = next(s, j)
                    }
                }
                if !found {
                    out.push(Token {
                        kind: "unsupported",
                        start: i,
                        end,
                        extra: json!("computed content"),
                    })
                }
                i = end.max(i + 1);
                continue;
            }
        }
        i = next(s, i)
    }
    out
}
pub fn setup_before(s: &str, pos: usize) -> String {
    tokens(s)
        .iter()
        .filter(|t| t.kind == "setup" && t.end <= pos)
        .map(|t| s[t.start..t.end].to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}
#[derive(Default)]
struct Scan {
    cards: Vec<Value>,
    outline: Vec<Value>,
    stack: Vec<Value>,
    warnings: Vec<String>,
    files: HashSet<String>,
    counts: HashMap<String, usize>,
    head_counts: HashMap<String, usize>,
}
impl Scan {
    fn heading(&mut self, title: String, level: usize, file: &str, line: usize) {
        while self
            .stack
            .last()
            .is_some_and(|h| h["level"].as_u64().unwrap_or(0) >= level as u64)
        {
            self.stack.pop();
        }
        let title = title.split_whitespace().collect::<Vec<_>>().join(" ");
        let title = if title.is_empty() {
            "Untitled".into()
        } else {
            title
        };
        let mut path = self.path();
        let key = format!(
            "{}\0{}\0{}{}",
            file,
            level,
            title,
            path.iter()
                .map(|p| format!("\0{}", p.as_str().unwrap()))
                .collect::<String>()
        );
        let count = self.head_counts.entry(key.clone()).or_default();
        *count += 1;
        let ident = format!("heading-{}", id(&format!("{key}\0{count}")));
        path.push(json!(ident));
        let h = json!({"id":ident,"title":title,"level":level,"file":file,"line":line,"path":path});
        self.stack.push(h.clone());
        self.outline.push(h);
    }
    fn path(&self) -> Vec<Value> {
        self.stack.iter().map(|h| h["id"].clone()).collect()
    }
    fn card(
        &mut self,
        s: &str,
        file: &str,
        span: std::ops::Range<usize>,
        body: String,
        kind: &str,
        frames: Option<Vec<Value>>,
    ) {
        let start = span.start;
        let end = span.end;
        let typst = frames.is_some();
        let body = body.trim().to_owned();
        let l = line(s, start);
        let last = line(s, end);
        let key = format!(
            "{}\0{}",
            file,
            body.split_whitespace().collect::<Vec<_>>().join(" ")
        );
        let count = self.counts.entry(key.clone()).or_default();
        *count += 1;
        let ident = format!(
            "{}-{}",
            if typst { "typst" } else { "source" },
            id(&format!("{key}{count}"))
        );
        let label = if typst {
            re(r"^\s*<([\w:.-]+)>")
                .captures(&s[end..])
                .map(|c| c[1].to_owned())
        } else {
            re(r"\\label\{([^}]+)\}")
                .captures(&body)
                .map(|c| c[1].to_owned())
        };
        let name = label
            .map(|s| {
                let t = s
                    .split_once(':')
                    .map(|(_, b)| b)
                    .unwrap_or(&s)
                    .replace(['_', '-'], " ")
                    .to_lowercase();
                let mut ch = t.chars();
                ch.next()
                    .map(|c| c.to_uppercase().collect::<String>() + ch.as_str())
                    .unwrap_or_default()
            })
            .unwrap_or_else(|| match kind {
                "figure" => format!("{} · {l}", if typst { "CeTZ" } else { "TikZ" }),
                "inline" => format!("Inline · {}", body.chars().take(45).collect::<String>()),
                _ => format!("Equation · {l}"),
            });
        let mut card = json!({"id":ident,"name":name,"tex":body,"kind":kind,"origin":"source","file":file,"line":l,"endLine":last,"sourceOffset":s[..start].chars().count(),"sourceEnd":s[..end].chars().count(),"startUTF16":s[..start].encode_utf16().count(),"endUTF16":s[..end].encode_utf16().count(),"column":s[..start].rsplit('\n').next().unwrap_or("").chars().count()+1,"endColumn":s[..end].rsplit('\n').next().unwrap_or("").chars().count()+1,"sourceOrder":self.cards.len() as f64,"contextStart":l.saturating_sub(5).max(1),"context":s.lines().skip(l.saturating_sub(6)).take(last+5-l.saturating_sub(6)).collect::<Vec<_>>(),"outlinePath":self.path(),"status":"pending"});
        if typst {
            card["typstContext"] = json!(frames)
        }
        self.cards.push(card);
    }
    fn typst(
        &mut self,
        root: &Path,
        name: &str,
        ancestors: Vec<Value>,
        stack: Vec<String>,
    ) -> Result<()> {
        if stack.iter().any(|n| n == name) || stack.len() >= 32 {
            self.warnings
                .push(format!("Skipped cyclic include: {name}"));
            return Ok(());
        }
        let p = safe(root, name)?;
        let s = read(&p)?;
        self.files.insert(name.into());
        let mut chain = stack;
        chain.push(name.into());
        for t in tokens(&s) {
            match t.kind {
                "heading" => self.heading(
                    t.extra[1].as_str().unwrap_or("").into(),
                    t.extra[0].as_u64().unwrap_or(1) as usize,
                    name,
                    line(&s, t.start),
                ),
                "include" => {
                    let child = t.extra.as_str().unwrap_or("");
                    let rel = if child.starts_with('/') {
                        child.trim_start_matches('/').to_owned()
                    } else {
                        Path::new(name)
                            .parent()
                            .unwrap_or(Path::new(""))
                            .join(child)
                            .to_string_lossy()
                            .into_owned()
                    };
                    let child_path = root.join(&rel).canonicalize();
                    match child_path {
                        Ok(p) if p.starts_with(root) => {
                            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                            let mut frames = ancestors.clone();
                            frames.push(json!({"file":name,"offset":s[..t.start].chars().count()}));
                            if let Err(e) = self.typst(root, &rel, frames, chain.clone()) {
                                self.warnings.push(e)
                            }
                        }
                        _ => self
                            .warnings
                            .push(format!("Referenced file not found inside project: {child}")),
                    }
                }
                "math" | "figure" => {
                    let figure = t.kind == "figure";
                    let body = if figure {
                        format!("#{}", &s[t.start..t.end])
                    } else {
                        s[t.start + 1..t.end - 1].into()
                    };
                    let kind = if figure {
                        "figure"
                    } else if body.starts_with(char::is_whitespace)
                        && body.ends_with(char::is_whitespace)
                    {
                        "block"
                    } else {
                        "inline"
                    };
                    let mut frames = ancestors.clone();
                    let start = if figure {
                        t.extra.as_u64().unwrap_or(t.start as u64) as usize
                    } else {
                        t.start
                    };
                    frames.push(json!({"file":name,"offset":s[..start].chars().count()}));
                    self.card(&s, name, t.start..t.end, body, kind, Some(frames))
                }
                "unclosed" => self
                    .warnings
                    .push(format!("{name}:{} Unclosed equation.", line(&s, t.start))),
                "unsupported" => self.warnings.push(format!(
                    "{name}:{} Skipped {} (literal extraction only).",
                    line(&s, t.start),
                    t.extra.as_str().unwrap_or("expression")
                )),
                _ => {}
            }
        }
        Ok(())
    }
    fn tex(&mut self, root: &Path, name: &str, stack: Vec<String>, start: usize) -> Result<()> {
        if stack.iter().any(|n| n == name) || stack.len() > 32 {
            self.warnings.push(format!("Skipped cyclic input: {name}"));
            return Ok(());
        }
        let path = safe(root, name)?;
        let raw = read(&path)?;
        let text = uncomment(&raw); // Preserve byte offsets, including Unicode comments.
        self.files.insert(name.into());
        let mut chain = stack;
        chain.push(name.into());
        let mut i = start;
        while i < text.len() {
            let tail = &text[i..];
            if !tail.starts_with(['\\', '$']) {
                i += tail.find(['\\', '$']).unwrap_or(tail.len());
                continue;
            }
            if tail.starts_with("\\end{document}") {
                break;
            }
            if let Some(m) = re(r"^\\(?:newcommand|renewcommand|providecommand|DeclareMathOperator|DeclareRobustCommand)\*?\s*").find(tail) {
                let mut j = i + m.end();
                if text[j..].starts_with('{') {
                    j = balanced(&text, j, b'{', b'}');
                } else if let Some(m) = re(r"^\\[A-Za-z@]+").find(&text[j..]) {
                    j += m.end();
                }
                j = skip_space(&text, j);
                while text[j..].starts_with('[') {
                    j = balanced(&text, j, b'[', b']');
                    j = skip_space(&text, j);
                }
                i = if text[j..].starts_with('{') {
                    balanced(&text, j, b'{', b'}')
                } else {
                    j
                };
                continue;
            }
            if re(r"^\\(?:def|gdef|edef|xdef)\b").is_match(tail) {
                i = tail
                    .find('{')
                    .map(|n| balanced(&text, i + n, b'{', b'}'))
                    .unwrap_or(text.len());
                continue;
            }
            if let Some(h) = re(r"^\\(part|chapter|section|subsection|subsubsection|paragraph|subparagraph)\b\*?\s*").captures(tail) {
                let mut j = i + h[0].len();
                if text[j..].starts_with('[') {
                    j = skip_space(&text, balanced(&text, j, b'[', b']'));
                }
                if text[j..].starts_with('{') {
                    let end = balanced(&text, j, b'{', b'}');
                    let title = tex_title(&text[j + 1..end.saturating_sub(1)]);
                    let level = ["part", "chapter", "section", "subsection", "subsubsection", "paragraph", "subparagraph"]
                        .iter().position(|v| *v == &h[1]).unwrap();
                    self.heading(title, level, name, line(&raw, i));
                    i = end;
                    continue;
                }
            }
            if let Some(c) =
                re(r"^\\(?:input|include)\b\s*(?:\{([^}]+)\}|([^\s{}]+))").captures(tail)
            {
                let child = c.get(1).or_else(|| c.get(2)).unwrap().as_str().trim();
                let mut rel = Path::new(child).to_path_buf();
                if rel.extension().is_none() {
                    rel.set_extension("tex");
                }
                let found = [path.parent().unwrap().join(&rel), root.join(&rel)]
                    .iter()
                    .find_map(|p| {
                        p.canonicalize()
                            .ok()
                            .filter(|p| p.starts_with(root) && p.is_file())
                    });
                if let Some(p) = found {
                    let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
                    if let Err(e) = self.tex(root, &rel, chain.clone(), 0) {
                        self.warnings.push(e)
                    }
                } else {
                    self.warnings
                        .push(format!("Referenced file not found: {child}"))
                }
                i += c[0].len();
                continue;
            }
            if let Some(c) = re(r"^\\verb\*?([^A-Za-z\s])").captures(tail) {
                let j = i + c[0].len();
                i = text[j..]
                    .find(&c[1])
                    .map(|n| j + n + c[1].len())
                    .unwrap_or(text.len());
                continue;
            }
            let env = re(r"^\\begin\s*\{([^}]+)\}").captures(tail);
            let mut kind = "";
            let mut ending = None;
            let mut body = String::new();
            if let Some(ref c) = env {
                let en = &c[1];
                if [
                    "verbatim",
                    "verbatim*",
                    "Verbatim",
                    "lstlisting",
                    "minted",
                    "comment",
                ]
                .contains(&en)
                {
                    let pat = format!(r"\\end\s*\{{{}\}}", regex::escape(en));
                    i = re(&pat)
                        .find(&text[i + c[0].len()..])
                        .map(|m| i + c[0].len() + m.end())
                        .unwrap_or(text.len());
                    continue;
                }
                if [
                    "equation",
                    "equation*",
                    "align",
                    "align*",
                    "gather",
                    "gather*",
                    "multline",
                    "multline*",
                    "displaymath",
                    "math",
                    "tikzpicture",
                ]
                .contains(&en)
                {
                    kind = if en == "tikzpicture" {
                        "figure"
                    } else if en == "math" {
                        "inline"
                    } else {
                        "block"
                    };
                    let pat = format!(r"\\(begin|end)\s*\{{{}\}}", regex::escape(en));
                    let mut depth = 1;
                    let j = i + c[0].len();
                    for c in re(&pat).captures_iter(&text[j..]) {
                        depth += if &c[1] == "begin" { 1 } else { -1 };
                        if depth == 0 {
                            ending = Some(j + c.get(0).unwrap().end());
                            break;
                        }
                    }
                    if let Some(end) = ending {
                        body = raw[i..end].into()
                    }
                }
            }
            if kind.is_empty() {
                for (open, close) in [("$$", "$$"), ("\\[", "\\]"), ("\\(", "\\)"), ("$", "$")] {
                    if tail.starts_with(open) {
                        kind = if open == "$" || open == "\\(" {
                            "inline"
                        } else {
                            "block"
                        };
                        let mut j = i + open.len();
                        while j < text.len() {
                            if text[j..].starts_with(close) {
                                ending = Some(j + close.len());
                                body = raw[i + open.len()..j].into();
                                break;
                            }
                            j = if text[j..].starts_with('\\') {
                                next(&text, (j + 1).min(text.len()))
                            } else {
                                next(&text, j)
                            }
                        }
                        break;
                    }
                }
            }
            if let Some(end) = ending {
                self.card(&raw, name, i..end, body, kind, None);
                i = end;
                continue;
            }
            if !kind.is_empty() {
                self.warnings
                    .push(format!("{name}:{} Unclosed equation.", line(&raw, i)))
            }
            i = if tail.starts_with('\\') && tail.chars().nth(1).is_some_and(|c| !c.is_alphabetic())
            {
                next(&text, i + 1)
            } else {
                next(&text, i)
            };
        }
        Ok(())
    }
}
fn tex_title(s: &str) -> String {
    let t = re(r"\\texorpdfstring\{[^{}]*\}\{([^{}]*)\}").replace_all(s, "$1");
    let t = re(r"\\(?:label|index)\{[^{}]*\}").replace_all(&t, "");
    let t = re(r"\\([&%_$#{}])").replace_all(&t, "$1");
    re(r"\\[A-Za-z@]+\*?")
        .replace_all(&t, "")
        .replace(['{', '}', '$'], "")
        .replace('~', " ")
}
pub fn extract(root: &Path, main: &str, focus: Option<&str>) -> Result<Value> {
    let canonical = root.canonicalize().map_err(err)?;
    let root = canonical.as_path();
    let mut scan = Scan::default();
    if main.ends_with(".typ") {
        scan.typst(root, focus.unwrap_or(main), vec![], vec![])?
    } else {
        let s = read(&safe(root, main)?)?;
        let start = re(r"\\begin\s*\{document\}")
            .find(&uncomment(&s))
            .ok_or("Main file has no begin{document}.")?
            .end();
        scan.tex(
            root,
            focus.unwrap_or(main),
            vec![],
            if focus.is_some() { 0 } else { start },
        )?
    }
    let mut files: Vec<_> = scan.files.into_iter().collect();
    files.sort();
    Ok(json!({"cards":scan.cards,"outline":scan.outline,"warnings":scan.warnings,"files":files}))
}
pub fn candidates(root: &Path) -> Result<Vec<String>> {
    let mut out = vec![];
    for (name, p, dir) in entries(root)? {
        if dir {
            continue;
        }
        if name.ends_with(".typ")
            || name.ends_with(".tex")
                && read(&p).is_ok_and(|s| re(r"\\begin\s*\{document\}").is_match(&uncomment(&s)))
        {
            out.push(name)
        }
    }
    out.sort_by_key(|s| {
        (
            !matches!(s.as_str(), "main.tex" | "main.typ"),
            s.matches('/').count(),
            s.clone(),
        )
    });
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typst_literals_and_utf16() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("main.typ"),
            "= Model\n😀 $x^2$\n`$ignored$`\n#let f(x) = [$ignored$]\n$ (a/b) $",
        )
        .unwrap();
        let v = extract(d.path(), "main.typ", None).unwrap();
        assert_eq!(v["cards"].as_array().unwrap().len(), 2);
        assert_eq!(v["outline"][0]["title"], "Model");
        assert_eq!(v["cards"][0]["startUTF16"], 11);
    }
    #[test]
    fn tex_skips_macros_comments() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("main.tex"),"\\documentclass{article}\n\\begin{document}\n% 日本語 $no$\n\\newcommand{\\a}{$no$}\n\\section{Model}\n$x$\\[y\\]\\begin{tikzpicture}a\\end{tikzpicture}\\end{document}").unwrap();
        let v = extract(d.path(), "main.tex", None).unwrap();
        assert_eq!(v["cards"].as_array().unwrap().len(), 3);
        assert_eq!(v["cards"][2]["kind"], "figure");
        assert_eq!(v["outline"][0]["title"], "Model");
    }
    #[test]
    fn cetz_aliases() {
        let s="#import \"@preview/cetz:0.4.2\": canvas as draw\n#figure(draw({ import cetz.draw: *; line((0,0),(1,1)) }))";
        assert_eq!(tokens(s).iter().filter(|t| t.kind == "figure").count(), 1);
    }
    #[test]
    fn tex_optional_heading_respects_brace_groups() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("main.tex"),
            r"\begin{document}\section[short {]}]{Long title}$x$\end{document}",
        )
        .unwrap();
        let v = extract(d.path(), "main.tex", None).unwrap();
        assert_eq!(v["outline"][0]["title"], "Long title");
        assert_eq!(v["cards"].as_array().unwrap().len(), 1);
    }
}
