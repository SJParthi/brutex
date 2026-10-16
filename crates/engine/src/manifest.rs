//! Which crates a `Cargo.toml` links, read as TOML rather than as lines.
//!
//! Test-only. It backs `tests::the_sweep_cannot_compute_a_condition_bit` (V-06),
//! which pins this crate's dependency set to exactly `vocab`.
//!
//! # Why a reader and not a line scan (D-1120)
//!
//! The line scan this replaces split every line at `#` and `=`, and took the
//! table from any line that began with `[`. Each of these linked a crate it did
//! not report:
//!
//! ```text
//! vocab = { package = "store", .. }   reported as `vocab`; `store` is what links
//! [ dependencies . store ]            spaces around the dot: no table matched
//! [dev_dependencies]                  cargo's underscore spelling: no table matched
//! dependencies.store = { .. }         a dotted key at the root, before any header
//! "store" = ".."                 an escaped key: reported with its escape
//! vocab.workspace = true              the rename lives in the ROOT manifest
//! ```
//!
//! and a continuation line of a multi-line array or string was read as a key.
//! The same holes were closed in the CI gates by D-1107, which reads every
//! pinned manifest with `.github/source_scan.rs`. This crate cannot call that
//! scanner and has no TOML crate in `Cargo.lock`, so this module walks the
//! grammar itself: keys are split into segments the way TOML splits them,
//! values are parsed (strings with their escapes, inline tables, arrays across
//! lines), and every key is joined to its full path before it is classified.
//!
//! Text the reader cannot parse is an `Err`, never a partial list. Cargo
//! accepts a few spellings this reader does not need to (it is permissive where
//! cargo is strict, which is safe: cargo refuses those manifests itself).

use std::collections::BTreeMap;

/// The tables whose keys are dependencies. Cargo still reads the underscore
/// spellings, with a deprecation warning, so they link exactly as the hyphen
/// ones do.
const DEPENDENCY_TABLES: [&str; 5] = [
    "dependencies",
    "dev-dependencies",
    "build-dependencies",
    "dev_dependencies",
    "build_dependencies",
];

/// One key the document defines: its full dotted path, and its value when the
/// value is a string or a bare scalar (`None` for a table or an array).
type Leaf = (Vec<String>, Option<String>);

/// A cursor over the manifest's characters.
struct Reader {
    chars: Vec<char>,
    at: usize,
    line: usize,
}

/// A character a bare key may contain.
fn bare(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

impl Reader {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn ahead(&self, k: usize) -> Option<char> {
        self.chars.get(self.at + k).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        if c == '\n' {
            self.line += 1;
        }
        self.at += 1;
        Some(c)
    }

    fn eat(&mut self, want: char) -> bool {
        let hit = self.peek() == Some(want);
        if hit {
            self.bump();
        }
        hit
    }

    fn fail(&self, what: &str) -> String {
        format!("line {}: {what}", self.line)
    }

    /// Spaces and tabs.
    fn spaces(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
    }

    /// The rest of a comment, up to and excluding its newline.
    fn comment(&mut self) {
        while !matches!(self.peek(), None | Some('\n')) {
            self.bump();
        }
    }

    /// Whitespace, newlines and comments.
    fn gaps(&mut self) {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n') => {
                    self.bump();
                }
                Some('#') => self.comment(),
                _ => return,
            }
        }
    }

    /// A basic or literal string, one-line or multi-line, escapes decoded.
    fn string(&mut self, quote: char) -> Result<String, String> {
        self.bump();
        let triple = self.peek() == Some(quote) && self.ahead(1) == Some(quote);
        if triple {
            self.bump();
            self.bump();
            // A newline straight after the opening delimiter is trimmed.
            if self.peek() == Some('\r') && self.ahead(1) == Some('\n') {
                self.bump();
            }
            self.eat('\n');
        }
        let mut out = String::new();
        loop {
            let c = self
                .peek()
                .ok_or_else(|| self.fail("unterminated string"))?;
            if c == quote && !triple {
                self.bump();
                return Ok(out);
            }
            if c == quote && self.ahead(1) == Some(quote) && self.ahead(2) == Some(quote) {
                // Up to two quotes may sit just inside the closing delimiter.
                while self.ahead(3) == Some(quote) {
                    out.push(quote);
                    self.bump();
                }
                self.bump();
                self.bump();
                self.bump();
                return Ok(out);
            }
            if c == '\n' && !triple {
                return Err(self.fail("newline inside a one-line string"));
            }
            self.bump();
            if c == '\\' && quote == '"' {
                self.escape(&mut out)?;
            } else {
                out.push(c);
            }
        }
    }

    /// The escape after a `\` in a basic string.
    fn escape(&mut self, out: &mut String) -> Result<(), String> {
        let e = self
            .bump()
            .ok_or_else(|| self.fail("unterminated escape"))?;
        let plain = match e {
            'b' => '\u{8}',
            't' => '\t',
            'n' => '\n',
            'f' => '\u{c}',
            'r' => '\r',
            '"' => '"',
            '\\' => '\\',
            'u' | 'U' => {
                let digits = if e == 'u' { 4 } else { 8 };
                let mut value: u32 = 0;
                for _ in 0..digits {
                    let d = self
                        .bump()
                        .and_then(|c| c.to_digit(16))
                        .ok_or_else(|| self.fail("malformed unicode escape"))?;
                    value = value * 16 + d;
                }
                char::from_u32(value).ok_or_else(|| self.fail("escape is not a character"))?
            }
            ' ' | '\t' | '\r' | '\n' => {
                // A line-ending backslash: the whitespace that follows is dropped.
                while matches!(self.peek(), Some(' ' | '\t' | '\r' | '\n')) {
                    self.bump();
                }
                return Ok(());
            }
            other => return Err(self.fail(&format!("unknown escape \\{other}"))),
        };
        out.push(plain);
        Ok(())
    }

    /// A dotted key, every segment unquoted and unescaped.
    fn key(&mut self) -> Result<Vec<String>, String> {
        let mut segments = Vec::new();
        loop {
            self.spaces();
            let segment = match self.peek() {
                Some(q @ ('"' | '\'')) => self.string(q)?,
                Some(c) if bare(c) => {
                    let mut s = String::new();
                    while let Some(c) = self.peek().filter(|c| bare(*c)) {
                        s.push(c);
                        self.bump();
                    }
                    s
                }
                _ => return Err(self.fail("expected a key")),
            };
            segments.push(segment);
            self.spaces();
            if !self.eat('.') {
                return Ok(segments);
            }
        }
    }

    /// One value at `path`, pushing a leaf for it and for everything inside it.
    fn value(&mut self, path: &[String], out: &mut Vec<Leaf>) -> Result<(), String> {
        match self.peek() {
            Some(q @ ('"' | '\'')) => {
                let s = self.string(q)?;
                out.push((path.to_vec(), Some(s)));
                Ok(())
            }
            Some('{') => {
                self.bump();
                out.push((path.to_vec(), None));
                self.gaps();
                if self.eat('}') {
                    return Ok(());
                }
                loop {
                    let key = self.key()?;
                    if !self.eat('=') {
                        return Err(self.fail("expected `=` in an inline table"));
                    }
                    self.spaces();
                    let mut inner = path.to_vec();
                    inner.extend(key);
                    self.value(&inner, out)?;
                    self.gaps();
                    if self.eat('}') {
                        return Ok(());
                    }
                    if !self.eat(',') {
                        return Err(self.fail("expected `,` or `}` in an inline table"));
                    }
                    self.gaps();
                    if self.eat('}') {
                        return Ok(());
                    }
                }
            }
            Some('[') => {
                self.bump();
                out.push((path.to_vec(), None));
                // An element's path ends in a segment no table can be named, so
                // nothing inside an array is ever read as a declaration.
                let mut element = path.to_vec();
                element.push("[]".to_owned());
                loop {
                    self.gaps();
                    if self.eat(']') {
                        return Ok(());
                    }
                    self.value(&element, out)?;
                    self.gaps();
                    if self.eat(']') {
                        return Ok(());
                    }
                    if !self.eat(',') {
                        return Err(self.fail("expected `,` or `]` in an array"));
                    }
                }
            }
            Some(_) => {
                let mut s = String::new();
                while let Some(c) = self
                    .peek()
                    .filter(|c| !matches!(c, ',' | ']' | '}' | '\r' | '\n' | '#'))
                {
                    s.push(c);
                    self.bump();
                }
                let s = s.trim();
                if s.is_empty() || s.contains('=') {
                    return Err(self.fail(&format!("malformed value {s:?}")));
                }
                out.push((path.to_vec(), Some(s.to_owned())));
                Ok(())
            }
            None => Err(self.fail("expected a value")),
        }
    }
}

/// Every key the document defines, as a full path joined to its table.
fn leaves(manifest: &str) -> Result<Vec<Leaf>, String> {
    let mut r = Reader {
        chars: manifest.chars().collect(),
        at: 0,
        line: 1,
    };
    let mut out: Vec<Leaf> = Vec::new();
    let mut table: Vec<String> = Vec::new();
    loop {
        r.gaps();
        if r.peek().is_none() {
            return Ok(out);
        }
        if r.eat('[') {
            let array = r.eat('[');
            let mut header = r.key()?;
            if !r.eat(']') || (array && !r.eat(']')) {
                return Err(r.fail("malformed table header"));
            }
            if array {
                // `[[bench]]` and its kin: an element, never a dependency table.
                header.push("[]".to_owned());
            }
            out.push((header.clone(), None));
            table = header;
        } else {
            let key = r.key()?;
            if !r.eat('=') {
                return Err(r.fail("expected `=` after a key"));
            }
            r.spaces();
            let mut path = table.clone();
            path.extend(key);
            r.value(&path, &mut out)?;
        }
        r.spaces();
        if r.peek() == Some('#') {
            r.comment();
        }
        if r.peek() == Some('\r') {
            r.bump();
        }
        if !matches!(r.peek(), None | Some('\n')) {
            return Err(r.fail("trailing text after a value"));
        }
    }
}

/// The dependency a full key path declares: (its table, its name, the path
/// below the name). `dependencies.NAME..`, its two siblings, their underscore
/// spellings, and `target.CFG.<any of them>.NAME..`.
fn dependency_of(path: &[String]) -> Option<(Vec<String>, &str, &[String])> {
    let is_table = |t: &String| DEPENDENCY_TABLES.contains(&t.as_str());
    match path {
        [t, name, rest @ ..] if is_table(t) => Some((vec![t.clone()], name, rest)),
        [target, cfg, t, name, rest @ ..] if target == "target" && is_table(t) => {
            Some((vec![target.clone(), cfg.clone(), t.clone()], name, rest))
        }
        _ => None,
    }
}

/// `workspace.dependencies.NAME` → the package it names, from the root manifest.
fn workspace_packages(root: &str) -> Result<BTreeMap<String, String>, String> {
    let mut packages: BTreeMap<String, String> = BTreeMap::new();
    for (path, value) in leaves(root)? {
        if let [w, d, name, rest @ ..] = path.as_slice()
            && w == "workspace"
            && d == "dependencies"
        {
            let package = packages.entry(name.clone()).or_insert_with(|| name.clone());
            if let ([key], Some(renamed)) = (rest, value)
                && key == "package"
            {
                *package = renamed;
            }
        }
    }
    Ok(packages)
}

/// Every PACKAGE `manifest` links, in any table and any spelling cargo accepts,
/// sorted and deduplicated. A `NAME.workspace = true` declaration resolves
/// through `root`'s `[workspace.dependencies]`, which is where its `package`
/// rename would be written.
///
/// The package and not the key, because the key is only a local alias:
/// `vocab = { package = "store", .. }` links `store`.
pub(crate) fn linked_packages(manifest: &str, root: &str) -> Result<Vec<String>, String> {
    let inherited = workspace_packages(root)?;
    // Keyed by (table, name): one alias may be declared in two tables and
    // renamed differently in each.
    let mut declared: BTreeMap<(Vec<String>, String), (Option<String>, bool)> = BTreeMap::new();
    for (path, value) in leaves(manifest)? {
        let Some((table, name, rest)) = dependency_of(&path) else {
            continue;
        };
        let entry = declared
            .entry((table, name.to_owned()))
            .or_insert((None, false));
        match (rest, value) {
            ([key], Some(package)) if key == "package" => entry.0 = Some(package),
            ([key], Some(flag)) if key == "workspace" && flag == "true" => entry.1 = true,
            _ => {}
        }
    }
    let mut found: Vec<String> = Vec::with_capacity(declared.len());
    for ((_, name), (package, from_workspace)) in declared {
        let linked = match (package, from_workspace) {
            (Some(package), _) => package,
            (None, true) => inherited.get(&name).cloned().unwrap_or(name),
            (None, false) => name,
        };
        found.push(linked);
    }
    found.sort();
    found.dedup();
    Ok(found)
}
