//! R3-24 repair round 1 (20 Oct): every line the server writes that the
//! never-rolling state stream's lifecycle word rule takes is classified.
//!
//! The rule hands the state stream every line whose message holds a
//! lifecycle word (`telemetry::run_dirs::state_events::is_state_change`).
//! The second check of 20 Oct found two per-request lines it took that
//! `SAMPLED` did not name (`api.bars export abandoned; work stopped` and
//! `export stopped short`, one per CSV export a reader left or cut): five
//! abandoned exports wrote five lines that `/audit.json` then listed as
//! `run` state changes. A list kept by hand had missed them, so the list is
//! now checked against the server's source: every emit in `crates/api/src`
//! the rule takes is a row of `TABLE`, a row of `KEPT` below (a state
//! change, with how often it is written), or of `DYNAMIC` (a message no
//! literal spells), or is refused by `SAMPLED`. A new per-request emit with a
//! lifecycle word fails here until it is named.
//!
//! An integration test of `api`, not a unit test of `telemetry`: it reads
//! the server's source files, and gate 21 holds the logger's own crate to
//! the files it declares it opens.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use telemetry::Level;
use telemetry::run_dirs::state_events::{SAMPLED, is_state_change, row_of};

/// One `Event` constructor call the source scan found.
#[derive(Debug)]
struct Emit {
    /// The file, from the workspace root.
    file: String,
    /// The line the constructor's name is on, from 1.
    line: usize,
    /// The level, or `None` when the call's level is no literal `Level::`.
    level: Option<Level>,
    /// The target as a literal (through a `const` when named by one), or the
    /// expression's text when it cannot be resolved.
    target: Result<String, String>,
    /// The message as a literal, or the expression's text.
    message: Result<String, String>,
}

/// Whether `c` can be part of an identifier.
fn ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Per char of `text`: whether it is code, rather than inside a comment, a
/// string literal (raw or not) or a char literal. A brace in a string is no
/// brace, and an emit in a comment is no emit.
fn code_mask(text: &[char]) -> Vec<bool> {
    let mut mask = vec![false; text.len()];
    let at = |i: usize| text.get(i).copied();
    let mut i = 0;
    while i < text.len() {
        let c = text[i];
        let next = at(i + 1);
        if c == '/' && next == Some('/') {
            while i < text.len() && text[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && next == Some('*') {
            let mut depth = 0usize;
            while i < text.len() {
                if text[i] == '/' && at(i + 1) == Some('*') {
                    depth += 1;
                    i += 2;
                } else if text[i] == '*' && at(i + 1) == Some('/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    i += 1;
                }
            }
            continue;
        }
        let before = |back: usize| i.checked_sub(back).and_then(at);
        let raw = c == 'r'
            && (before(1).is_none_or(|p| !ident_char(p))
                || (before(1) == Some('b') && before(2).is_none_or(|p| !ident_char(p))));
        if raw {
            let mut j = i + 1;
            while at(j) == Some('#') {
                j += 1;
            }
            if at(j) == Some('"') {
                let hashes = j - i - 1;
                j += 1;
                while j < text.len() {
                    if text[j] == '"' && (1..=hashes).all(|h| at(j + h) == Some('#')) {
                        j += 1 + hashes;
                        break;
                    }
                    j += 1;
                }
                i = j;
                continue;
            }
        }
        if c == '"' {
            let mut j = i + 1;
            while j < text.len() && text[j] != '"' {
                if text[j] == '\\' {
                    j += 1;
                }
                j += 1;
            }
            i = j + 1;
            continue;
        }
        if c == '\'' {
            if next == Some('\\') {
                let mut j = i + 3;
                while j < text.len() && text[j] != '\'' {
                    j += 1;
                }
                i = j + 1;
                continue;
            }
            if at(i + 2) == Some('\'') {
                i += 3;
                continue;
            }
        }
        mask[i] = true;
        i += 1;
    }
    mask
}

/// Whether `text` holds `word` as code at `i`.
fn code_at(text: &[char], mask: &[bool], i: usize, word: &str) -> bool {
    word.chars()
        .enumerate()
        .all(|(k, w)| text.get(i + k) == Some(&w) && mask.get(i + k) == Some(&true))
}

/// The end (exclusive) of the item that starts at `from`: its first `;`
/// outside brackets, or the end of its first `{ … }` block outside them.
fn item_end(text: &[char], mask: &[bool], from: usize) -> usize {
    let mut depth = 0usize;
    let mut j = from;
    while j < text.len() {
        if mask[j] {
            match text[j] {
                '(' | '[' => depth += 1,
                ')' | ']' => depth = depth.saturating_sub(1),
                ';' if depth == 0 => return j + 1,
                '{' if depth == 0 => {
                    let mut braces = 0usize;
                    while j < text.len() {
                        if mask[j] && text[j] == '{' {
                            braces += 1;
                        } else if mask[j] && text[j] == '}' {
                            braces -= 1;
                            if braces == 0 {
                                return j + 1;
                            }
                        }
                        j += 1;
                    }
                    return j;
                }
                _ => {}
            }
        }
        j += 1;
    }
    j
}

/// Per char: whether it is in an item `#[cfg(test)]` annotates; and the
/// module files those items name (`#[path = "…"]`, or `mod name;`).
fn test_items(text: &[char], mask: &[bool]) -> (Vec<bool>, Vec<String>) {
    const CFG: &str = "#[cfg(test)]";
    let mut out = vec![false; text.len()];
    let mut files = Vec::new();
    let mut i = 0;
    while i < text.len() {
        if code_at(text, mask, i, CFG) {
            let end = item_end(text, mask, i + CFG.len());
            let item: String = text[i..end].iter().collect();
            if let Some(path) = item
                .split("#[path = \"")
                .nth(1)
                .and_then(|rest| rest.split('"').next())
            {
                files.push(path.to_owned());
            } else if let Some(name) = item
                .split_whitespace()
                .skip_while(|word| *word != "mod")
                .nth(1)
                .and_then(|name| name.strip_suffix(';'))
            {
                files.push(format!("{name}.rs"));
            }
            for flag in &mut out[i..end] {
                *flag = true;
            }
            i = end;
            continue;
        }
        i += 1;
    }
    (out, files)
}

/// The text of a string literal argument, or `None` for any other
/// expression. Escapes and line continuations are undone.
fn literal(arg: &str) -> Option<String> {
    let arg = arg.trim();
    if let Some(rest) = arg.strip_prefix('r') {
        let fence = "#".repeat(rest.len() - rest.trim_start_matches('#').len());
        return rest
            .strip_prefix(fence.as_str())?
            .strip_prefix('"')?
            .strip_suffix(fence.as_str())?
            .strip_suffix('"')
            .map(str::to_owned);
    }
    let inner = arg.strip_prefix('"')?.strip_suffix('"')?;
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            return None;
        }
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            '\n' => {
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
            }
            'n' => out.push('\n'),
            't' => out.push('\t'),
            other => out.push(other),
        }
    }
    Some(out)
}

/// Every string literal in `text`, in order: each run of non-code chars
/// that opens with a quote or a raw string's `r`.
fn literals_of(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    let mask = code_mask(&chars);
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if mask[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !mask[i] {
            i += 1;
        }
        let run: String = chars[start..i].iter().collect();
        if let Some(text) = literal(&run) {
            out.push(text);
        }
    }
    out
}

/// The value of `const NAME: &str = "…";` (or `&'static str`) in `text`.
fn const_str(text: &str, name: &str) -> Option<String> {
    let rest = text.split(&format!("const {name}: &")).nth(1)?;
    literal(rest.split_once('=')?.1.split_once(';')?.0)
}

/// A target named by a `const`: its value in `text`, else the one value every
/// file of the crate that spells that `const` gives it. A path through another
/// module (`a::B`) or a name two files spell differently is not resolved.
fn resolve(expr: &str, text: &str, crate_texts: &[String]) -> Option<String> {
    if !expr.chars().all(ident_char) {
        return None;
    }
    if let Some(value) = const_str(text, expr) {
        return Some(value);
    }
    let mut values: Vec<String> = crate_texts
        .iter()
        .filter_map(|file| const_str(file, expr))
        .collect();
    values.sort();
    values.dedup();
    (values.len() == 1).then(|| values.remove(0))
}

/// The arguments of the call whose `(` is at `open`, split at top-level
/// commas, as text.
fn split_args(chars: &[char], mask: &[bool], open: usize) -> Vec<String> {
    let (mut args, mut depth, mut start) = (Vec::new(), 0usize, open + 1);
    for j in open + 1..chars.len() {
        if !mask[j] {
            continue;
        }
        match chars[j] {
            '(' | '[' | '{' => depth += 1,
            ')' if depth == 0 => {
                args.push(chars[start..j].iter().collect());
                return args;
            }
            ')' | ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                args.push(chars[start..j].iter().collect());
                start = j + 1;
            }
            _ => {}
        }
    }
    args
}

/// How a literal level reads in a call to `Event::new`.
const LEVEL_WORDS: [(&str, Level); 5] = [
    ("Level::Trace", Level::Trace),
    ("Level::Debug", Level::Debug),
    ("Level::Info", Level::Info),
    ("Level::Warn", Level::Warn),
    ("Level::Error", Level::Error),
];

/// Every `Event::{trace,debug,info,warn,error,new}(…)` call in `text`
/// outside test items, with its level, target and message. A target named
/// by a `const` is looked up in `text`, then in `crate_texts`.
fn emits_in(file: &str, text: &str, crate_texts: &[String]) -> Vec<Emit> {
    let chars: Vec<char> = text.chars().collect();
    let mask = code_mask(&chars);
    let (tests, _) = test_items(&chars, &mask);
    let mut out = Vec::new();
    for i in 0..chars.len() {
        if tests[i] || !code_at(&chars, &mask, i, "Event::") {
            continue;
        }
        let name_at = i + "Event::".len();
        let name: String = chars[name_at..]
            .iter()
            .take_while(|c| ident_char(**c))
            .collect();
        let open = name_at + name.chars().count();
        if chars.get(open) != Some(&'(') {
            continue;
        }
        let mut level = match name.as_str() {
            "trace" => Some(Level::Trace),
            "debug" => Some(Level::Debug),
            "info" => Some(Level::Info),
            "warn" => Some(Level::Warn),
            "error" => Some(Level::Error),
            "new" => None,
            _ => continue,
        };
        let mut args = split_args(&chars, &mask, open);
        if name == "new" {
            if args.is_empty() {
                continue;
            }
            let named = args.remove(0);
            level = LEVEL_WORDS
                .iter()
                .find(|(word, _)| named.trim().ends_with(word))
                .map(|(_, level)| *level);
        }
        let (Some(target), Some(message)) = (args.first(), args.get(1)) else {
            continue;
        };
        let target = literal(target)
            .ok_or_else(|| target.trim().to_owned())
            .or_else(|expr| resolve(&expr, text, crate_texts).ok_or(expr));
        out.push(Emit {
            file: file.to_owned(),
            line: chars[..i].iter().filter(|c| **c == '\n').count() + 1,
            level,
            target,
            message: literal(message).ok_or_else(|| message.trim().to_owned()),
        });
    }
    out
}

/// Every `.rs` file under `dir`, recursively, sorted, as `(path, text)`.
fn sources(dir: &std::path::Path, out: &mut Vec<(std::path::PathBuf, String)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .map(|entry| entry.expect("an entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            out.push((path, text));
        }
    }
}

/// The source files of `src` (from the workspace root) that are not test
/// code, as `(name from the workspace root, text)`, and every file's text.
/// A file only a `#[cfg(test)]` item names, and every `*_tests.rs`, is test
/// code.
fn crate_sources(src: &str) -> (Vec<(String, String)>, Vec<String>) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let dir = root.join(src);
    let mut files = Vec::new();
    sources(&dir, &mut files);
    let mut test_files: Vec<String> = Vec::new();
    for (_, text) in &files {
        let chars: Vec<char> = text.chars().collect();
        let mask = code_mask(&chars);
        test_files.extend(test_items(&chars, &mask).1);
    }
    let texts = files.iter().map(|(_, text)| text.clone()).collect();
    let kept = files
        .into_iter()
        .filter(|(path, _)| {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            !name.ends_with("_tests.rs") && !test_files.iter().any(|t| t == name)
        })
        .map(|(path, text)| {
            let shown = format!("{src}/{}", path.strip_prefix(&dir).unwrap().display());
            (shown, text)
        })
        .collect();
    (kept, texts)
}

/// A line of the server the word rule takes that IS a state change, and how
/// often it is written: once per run, start or stop, never per request or
/// per minute. `(target, message, how often)`.
const KEPT: [(&str, &str, &str); 12] = [
    (
        "api.audit",
        "journal index build finished",
        "once per journal index built from scratch",
    ),
    (
        "pull.recovery",
        "recovery ended",
        "once per recovery plan driven",
    ),
    (
        "api.pull",
        "broker request stopped",
        "once per hand-pull basket the broker stopped",
    ),
    ("pull.run", "started", "once per pull run"),
    ("pull.run", "finished", "once per pull run"),
    (
        "pull.run",
        "finished — BOOKS DO NOT BALANCE",
        "once per pull run",
    ),
    (
        "api.recovery",
        "recovery not resumed",
        "once per server start with a recovery it does not resume",
    ),
    ("pull.roll", "walk finished", "once per F&O roll walk"),
    (
        "api.server",
        "the server stopped after serving a DEGRADED universe",
        "once per server life",
    ),
    (
        "api.server",
        "the server stopped on an error",
        "once per server life",
    ),
    (
        "api.sweep",
        "an engine task finished",
        "once per engine task (a sweep, a descent or a command)",
    ),
    (
        "cli.audit",
        "sweep attempt started",
        "once per sweep attempt the browser starts",
    ),
];

/// An emit whose message (or level) is no literal, so the word rule's
/// verdict is the message's at run time: `(file, target, the expression's
/// first word, how often it is written)`. Its literal alternatives, when the
/// expression spells them, are checked as [`KEPT`]'s rows are.
const DYNAMIC: [(&str, &str, &str, &str); 6] = [
    (
        "crates/api/src/assets.rs",
        "api.assets",
        "if",
        "once per server start: whether the front end is on disk",
    ),
    (
        "crates/api/src/credential_law.rs",
        "pull.credential",
        "what",
        "once per credential re-read, at most one per pull",
    ),
    (
        "crates/api/src/logs.rs",
        "api.request",
        "suppression_message",
        "at most one per ration window: its words are checked below",
    ),
    (
        "crates/api/src/main.rs",
        "api.main",
        "said",
        "once per process exit",
    ),
    (
        "crates/api/src/server.rs",
        "pull.run",
        "if",
        "once per pull run",
    ),
    (
        "crates/api/src/server.rs",
        "api.serve",
        "message",
        "once per connection cut or closed: its words are checked below",
    ),
];

/// The first word of an expression: what [`DYNAMIC`] names it by.
fn head(expr: &str) -> &str {
    let expr = expr.trim();
    let end = expr.find(|c: char| !ident_char(c)).unwrap_or(expr.len());
    expr.get(..end).unwrap_or(expr)
}

/// What the server's emits the word rule takes are, against the three lists:
/// each line names an emit no list classifies. Empty when every one is a row
/// of `TABLE`, of [`KEPT`] or of [`DYNAMIC`] (or refused by `SAMPLED`, in
/// which case the rule does not take it).
fn unclassified(emits: &[Emit]) -> Vec<String> {
    let mut out = Vec::new();
    let classified = |target: &str, message: &str, level: Level| {
        !is_state_change(level, target, message)
            || row_of(target, message).is_some()
            || KEPT.iter().any(|(t, m, _)| *t == target && *m == message)
    };
    for emit in emits {
        // A LEVEL NOT SPELT is taken at its worst: `error` is audited.
        let level = emit.level.unwrap_or(Level::Error);
        match (&emit.target, &emit.message) {
            (Ok(target), Ok(message)) => {
                if !classified(target, message, level) {
                    out.push(format!(
                        "{}:{} {target} / {message:?} at {}",
                        emit.file,
                        emit.line,
                        level.label()
                    ));
                }
            }
            (target, message) => {
                let target = target.as_ref().unwrap_or_else(|expr| expr);
                let expr = message.as_ref().unwrap_or_else(|text| text);
                let named = DYNAMIC
                    .iter()
                    .any(|(f, t, h, _)| *f == emit.file && t == target && *h == head(expr));
                if !named {
                    out.push(format!(
                        "{}:{} {target} / the expression {:?} is no row of DYNAMIC",
                        emit.file,
                        emit.line,
                        head(expr)
                    ));
                }
                for alternative in literals_of(expr) {
                    if !classified(target, &alternative, level) {
                        out.push(format!(
                            "{}:{} {target} / {alternative:?} at {}",
                            emit.file,
                            emit.line,
                            level.label()
                        ));
                    }
                }
            }
        }
    }
    out
}

/// **EVERY LINE THE SERVER WRITES THAT THE WORD RULE TAKES IS CLASSIFIED**
/// (R3-24 repair round 1): failed on `crates/api/src/bars_export.rs`'s
/// `export stopped short` and `export abandoned; work stopped` while SAMPLED
/// named two rows, and passes now SAMPLED names them. The scan's premises
/// are checked too: it finds the server's emits (more than 60, among them a
/// TABLE row's and both SAMPLED sites'), and every list row names an emit the
/// scan found, so no row lingers after its emit moved.
#[test]
fn every_lifecycle_word_emit_in_the_server_is_classified() {
    let (files, texts) = crate_sources("crates/api/src");
    let emits: Vec<Emit> = files
        .iter()
        .flat_map(|(file, text)| emits_in(file, text, &texts))
        .collect();
    assert!(emits.len() > 60, "premise: {} emits found", emits.len());
    let found = |target: &str, message: &str| {
        emits.iter().any(|emit| {
            emit.target.as_deref() == Ok(target)
                && (emit.message.as_deref() == Ok(message)
                    || emit
                        .message
                        .as_ref()
                        .is_err_and(|expr| literals_of(expr).iter().any(|m| m == message)))
        })
    };
    assert!(
        found("api.serve", "listening"),
        "premise: a TABLE row's emit"
    );
    for row in &SAMPLED {
        assert!(found(row.target, row.message), "premise: {row:?} is found");
    }
    for (target, message, why) in &KEPT {
        assert!(!why.is_empty());
        assert!(
            found(target, message),
            "KEPT names {target} / {message:?}, which the server no longer emits"
        );
    }
    for (file, target, expr, why) in &DYNAMIC {
        assert!(!why.is_empty());
        assert!(
            emits.iter().any(|emit| emit.file == *file
                && emit.target.as_deref() == Ok(*target)
                && emit.message.as_ref().is_err_and(|e| head(e) == *expr)),
            "DYNAMIC names {file} {target} {expr}, which is no longer there"
        );
    }
    let missed = unclassified(&emits);
    assert!(
        missed.is_empty(),
        "lines the never-rolling state stream would take and no list classifies; name each \
         in SAMPLED if it is written per request or per minute, else in TABLE or KEPT: \
         {missed:#?}"
    );
}

/// **THE TWO PER-REQUEST DYNAMIC LINES CARRY NO LIFECYCLE WORD**: the
/// connection's one line (`api.serve`, a message its callers pass to
/// `report`) and the ration's summary (`api.request`, the words of
/// `suppression_message`). Each is written per connection or per window, so
/// each of their messages is checked to be no state change at any level.
#[test]
fn the_per_request_dynamic_lines_carry_no_lifecycle_word() {
    let (files, _) = crate_sources("crates/api/src");
    let text = |name: &str| {
        files
            .iter()
            .find(|(file, _)| file == name)
            .map_or_else(|| panic!("{name} is read"), |(_, text)| text.clone())
    };
    let server: Vec<char> = text("crates/api/src/server.rs").chars().collect();
    let mask = code_mask(&server);
    let mut said = Vec::new();
    for i in 0..server.len() {
        if code_at(&server, &mask, i, ".report(") {
            let args = split_args(&server, &mask, i + ".report".len());
            said.extend(args.get(1).and_then(|arg| literal(arg)));
        }
    }
    let logs = text("crates/api/src/logs.rs");
    let summary = logs
        .split("fn suppression_message")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("suppression_message is in logs.rs");
    let summaries = literals_of(summary);
    assert!(said.len() >= 3, "premise: report's messages, {said:?}");
    assert!(
        summaries.len() >= 2,
        "premise: the summaries, {summaries:?}"
    );
    for (target, message) in said
        .iter()
        .map(|m| ("api.serve", m))
        .chain(summaries.iter().map(|m| ("api.request", m)))
    {
        for level in [Level::Info, Level::Warn, Level::Error] {
            assert!(
                !is_state_change(level, target, message),
                "{target} / {message:?} is written per connection or per window and would \
                 grow the never-rolling stream"
            );
        }
    }
}

/// What [`the_scan_reads_rust_not_text`] compares of an [`Emit`]: its level,
/// target, message and line.
type Seen<'a> = (
    Option<Level>,
    Result<&'a str, &'a str>,
    Result<&'a str, &'a str>,
    usize,
);

/// **THE SCAN READS RUST, NOT TEXT**: an emit in a comment, in a string or in
/// a `#[cfg(test)]` item is no emit; a brace in a string does not end an
/// item; a target named by a `const` is resolved; a multi-line call, a raw
/// string and an `Event::new` with a spelt level are read; and the classifier
/// names a per-request line SAMPLED does not, by file and line.
#[test]
fn the_scan_reads_rust_not_text() {
    let source = r##"
const TARGET: &str = "api.fake";
// Event::info("api.fake", "in a comment stopped")
/* Event::info("api.fake", "in a block /* nested */ comment stopped") */
fn live() {
    let _s = "Event::info(\"api.fake\", \"in a string stopped\") {";
    let _c = '{';
    let _e = telemetry::Event::warn(
        TARGET,
        "export stopped short",
    );
    let _r = telemetry::Event::info("api.fake", r#"raw "quoted" ended"#);
    let _n = telemetry::Event::new(telemetry::Level::Info, "pull.run", "started");
    let _d = telemetry::Event::new(level, "api.fake", if x { "a finished" } else { "b" });
}
#[cfg(test)]
mod tests {
    fn f() { let _ = "}"; telemetry::Event::info("api.fake", "in a test stopped"); }
}
#[cfg(test)]
#[path = "fake_tests.rs"]
mod more;
"##;
    let emits = emits_in("crates/api/src/fake.rs", source, &[]);
    let seen: Vec<Seen<'_>> = emits
        .iter()
        .map(|e| {
            (
                e.level,
                e.target
                    .as_ref()
                    .map(String::as_str)
                    .map_err(String::as_str),
                e.message
                    .as_ref()
                    .map(String::as_str)
                    .map_err(String::as_str),
                e.line,
            )
        })
        .collect();
    assert_eq!(
        seen,
        [
            (
                Some(Level::Warn),
                Ok("api.fake"),
                Ok("export stopped short"),
                8
            ),
            (
                Some(Level::Info),
                Ok("api.fake"),
                Ok("raw \"quoted\" ended"),
                12
            ),
            (Some(Level::Info), Ok("pull.run"), Ok("started"), 13),
            (
                None,
                Ok("api.fake"),
                Err("if x { \"a finished\" } else { \"b\" }"),
                14
            ),
        ]
    );
    let chars: Vec<char> = source.chars().collect();
    let (_, files) = test_items(&chars, &code_mask(&chars));
    assert_eq!(files, ["fake_tests.rs"]);
    let missed = unclassified(&emits);
    assert_eq!(
        missed,
        [
            "crates/api/src/fake.rs:8 api.fake / \"export stopped short\" at warn",
            "crates/api/src/fake.rs:12 api.fake / \"raw \\\"quoted\\\" ended\" at info",
            "crates/api/src/fake.rs:14 api.fake / the expression \"if\" is no row of DYNAMIC",
            "crates/api/src/fake.rs:14 api.fake / \"a finished\" at error",
        ]
    );
    assert_eq!(
        literal(
            r#""a \"b\" \
        c""#
        ),
        Some("a \"b\" c".to_owned())
    );
    assert_eq!(literal("x"), None);
    assert_eq!(const_str(source, "TARGET"), Some("api.fake".to_owned()));
}
