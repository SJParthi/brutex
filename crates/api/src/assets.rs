//! The built front end, read from a directory at request time.
//!
//! # Why a directory and not the binary
//!
//! `CLAUDE.md` §2 and D-0053 say the same thing twice: no crate may depend on
//! the front end's toolchain, and `cargo build` must succeed on a machine that
//! has never installed one and has never run a `web/` build. CI gate 1e proves
//! it by moving `web/` aside and building the workspace.
//!
//! That rules out both obvious embeddings. `include_dir!` and `include_bytes!`
//! resolve at compile time against a path under `web/`, so with `web/` detached
//! the crate does not compile at all — the gate's own premise fails, and every
//! contributor without a front-end build inherits a red workspace. So the
//! assets are **read at run time from a directory this module names**, and the
//! only thing compiled in is that directory's default path, which is a string.
//!
//! # Where it looks
//!
//! [`WEB_ENV`], or `web/` beside the workspace this binary was built from. The
//! built assets are its `build` subdirectory, because that is what
//! `web/svelte.config.js` pins (`pages: 'build', assets: 'build'`) — the layout
//! is the front end's decision and is read here rather than restated.
//!
//! # When it is not there
//!
//! The server still starts, still binds, still answers every JSON route, and
//! says at `/` exactly which directory it looked in and what produces it.
//! `CLAUDE.md` §4: degrade loudly and name the reason. A blank 200 would be the
//! fallback that hides a failure; a refusal to start would take the JSON down
//! with the page, which is a second failure caused by the first.
//!
//! # Path safety
//!
//! Serving files off disk is the one thing in this crate that can hand an
//! attacker a file nobody meant to publish. The rules are in [`segments`] and
//! [`Assets::resolve`], and every one of them has a test:
//!
//! * the path is percent-decoded **once**, so a double-encoded `%252e%252e`
//!   decodes to the literal name `%2e%2e` and is a 404, never a traversal;
//! * a malformed escape, a null byte and non-UTF-8 are refusals, not guesses;
//! * every segment must be one ordinary file name — `..`, `.`, a leading `/`,
//!   a `\` separator and a drive prefix are all refused by the same rule;
//! * the resolved path is canonicalised and must still lie under the
//!   canonicalised root, which is what catches a symlink pointing out of it.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse as _, Response};

/// The environment variable naming the front-end directory.
///
/// Set it to point the server at a front end somewhere other than the
/// workspace it was built from — a packaged install, or a second checkout.
pub const WEB_ENV: &str = "BRUTEX_WEB";

/// The subdirectory of the front end holding its built output.
///
/// `web/svelte.config.js` pins `pages: 'build'` and `assets: 'build'`. Naming
/// it here rather than deriving it is deliberate: it is the front end's choice,
/// and a second spelling of it would be a second place to change.
const BUILD_DIR: &str = "build";

/// The shell every client-routed path falls back to.
const INDEX: &str = "index.html";

/// The built asset directory the bundler emits under [`BUILD_DIR`].
///
/// A request under it names a file the bundler produced, so an absent one is a
/// 404 and never the shell — see [`looks_like_asset`].
const IMMUTABLE_DIR: &str = "_app";

/// The type-ahead script, which is a source file rather than build output.
const TYPEAHEAD: &str = "typeahead.js";

/// The masters page's browser code, served from `web/` for the reason
/// `crate::mastersrun` records at length: 127 lines of it used to live inside a
/// Rust string literal, which is `CLAUDE.md` §2's "generated source in another
/// language, checked in".
///
/// UNLIKE [`TYPEAHEAD`], THIS ONE IS NOT PROGRESSIVE ENHANCEMENT. The instruments
/// page renders every row without its script; the masters page renders a table
/// whose cells are filled in ONLY by this file, so losing it loses the answer
/// rather than a convenience. The two refusals say different things for that
/// reason, and §4 is why they must: "degrade loudly and name the reason".
const MASTERS: &str = "masters.js";

/// The front end's own sources, beside the build rather than inside it.
///
/// Read for one purpose: [`Build::Stale`] compares the bundle's shell against
/// the newest file here. Nothing under it is ever served.
const SOURCES: &str = "src";

/// The front-end directory: [`WEB_ENV`], or `web/` beside this workspace.
#[must_use]
pub fn web_dir() -> PathBuf {
    web_dir_from(std::env::var_os(WEB_ENV))
}

/// The front-end directory implied by a value of [`WEB_ENV`].
///
/// Split from [`web_dir`] for the reason `server::masters_dir_from` is split
/// from `server::masters_dir`: both outcomes have to be testable, and a test
/// cannot set an environment variable — `set_var` is `unsafe` under edition
/// 2024, this crate forbids `unsafe`, and mutating process-wide state would
/// race every other test in the binary.
#[must_use]
fn web_dir_from(value: Option<std::ffi::OsString>) -> PathBuf {
    value.map_or_else(default_web_dir, PathBuf::from)
}

/// `web/` beside the workspace this binary was built from.
///
/// `CARGO_MANIFEST_DIR` is a string cargo defines at compile time. It is not a
/// file read and it does not require `web/` to exist, so gate 1e's build with
/// the tree detached is unaffected — which is exactly why the default is
/// expressed this way rather than as an `include_*` of anything.
///
/// It is an absolute path, so `cargo run` finds the front end from any working
/// directory. A binary copied off this machine will not, and that is what
/// [`WEB_ENV`] is for; the page below names the directory it looked in, so the
/// mistake reads itself.
fn default_web_dir() -> PathBuf {
    web_beside(Path::new(env!("CARGO_MANIFEST_DIR")))
}

/// `web/` two directories above `manifest`, which is `crates/api`.
///
/// Split out so the arm where the manifest path has no grandparent is reachable
/// from a test. It cannot happen for the real constant, and a branch no input
/// can enter is a branch the coverage gate cannot hold.
#[must_use]
fn web_beside(manifest: &Path) -> PathBuf {
    manifest
        .parent()
        .and_then(Path::parent)
        .map_or_else(|| PathBuf::from("web"), |root| root.join("web"))
}

/// Why a request for a file was refused rather than answered.
///
/// Each variant is a distinct rule with its own test. They are separate so the
/// response can say which one fired: "refused" with no reason is the shape
/// `CLAUDE.md` §4 bans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A `%` not followed by two hexadecimal digits.
    BadEscape,
    /// The decoded path is not UTF-8.
    NotUtf8,
    /// The decoded path contains a null byte.
    NullByte,
    /// A segment that is not one ordinary file name: `..`, `.`, a root, or a
    /// drive prefix.
    NotAName,
    /// The path resolved, and it resolved outside the root. A symlink.
    Escaped,
}

impl Refusal {
    /// What to tell the client, and what a test asserts on.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::BadEscape => "malformed percent-escape in the path",
            Self::NotUtf8 => "the decoded path is not UTF-8",
            Self::NullByte => "the decoded path contains a null byte",
            Self::NotAName => "a path segment is not an ordinary file name",
            Self::Escaped => "the path resolves outside the asset root",
        }
    }

    /// The status this refusal answers with.
    ///
    /// A traversal that resolved is `403`: the request was well-formed and the
    /// answer is no. Everything else is `400`: the request itself is malformed.
    #[must_use]
    pub const fn status(self) -> StatusCode {
        match self {
            Self::Escaped => StatusCode::FORBIDDEN,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

/// One percent-encoded hexadecimal digit.
#[must_use]
fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Percent-decodes a request path **once**.
///
/// Once is the whole rule. Decoding until nothing changes is what turns
/// `%252e%252e` into `..`; decoding once turns it into the literal name
/// `%2e%2e`, which is not a directory this tree has and is therefore a 404.
///
/// # Errors
///
/// [`Refusal::BadEscape`] when a `%` is not followed by two hexadecimal
/// digits. A tolerated malformed escape is a second spelling of a byte, and two
/// spellings of a byte is how a filter is walked past.
fn percent_decode(raw: &str) -> Result<Vec<u8>, Refusal> {
    let mut out = Vec::with_capacity(raw.len());
    let mut bytes = raw.bytes();
    while let Some(b) = bytes.next() {
        if b == b'%' {
            let hi = bytes.next().and_then(hex).ok_or(Refusal::BadEscape)?;
            let lo = bytes.next().and_then(hex).ok_or(Refusal::BadEscape)?;
            out.push(hi * 16 + lo);
        } else {
            out.push(b);
        }
    }
    Ok(out)
}

/// Whether `segment` is exactly one ordinary file name.
///
/// This one predicate refuses `..`, `.`, an absolute `/x`, and a Windows drive
/// prefix, because [`Path::components`] classifies all four as something other
/// than a single [`Component::Normal`]. Writing it as a list of forbidden
/// spellings instead is how a list ends up one spelling short.
#[must_use]
fn is_one_name(segment: &str) -> bool {
    let mut parts = Path::new(segment).components();
    matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none()
}

/// The path split into the file names it names, or the rule that refused it.
///
/// Splits on `\` as well as `/`, so `..%5c` is refused by the same rule that
/// refuses `..%2f` rather than surviving as a segment this platform would then
/// hand to the filesystem.
///
/// # Errors
///
/// Any [`Refusal`] except [`Refusal::Escaped`], which cannot be decided until
/// the path has been resolved against a root.
fn segments(raw: &str) -> Result<Vec<String>, Refusal> {
    let decoded = percent_decode(raw)?;
    if decoded.contains(&0) {
        return Err(Refusal::NullByte);
    }
    let text = String::from_utf8(decoded).map_err(|_| Refusal::NotUtf8)?;
    let mut out = Vec::with_capacity(4);
    for segment in text.split(['/', '\\']) {
        if segment.is_empty() {
            continue;
        }
        if !is_one_name(segment) {
            return Err(Refusal::NotAName);
        }
        out.push(segment.to_owned());
    }
    Ok(out)
}

/// Whether an absent path must be a 404 rather than the client-routed shell.
///
/// A missing script that answers with `index.html` is worse than a 404: the
/// browser parses HTML as JavaScript and reports a syntax error at line 1 of a
/// file that was never the problem. So anything with a file extension, and
/// anything under the bundler's own directory, is a 404 when it is not there.
#[must_use]
fn looks_like_asset(segments: &[String]) -> bool {
    segments.first().is_some_and(|s| s == IMMUTABLE_DIR)
        || segments.last().is_some_and(|s| s.contains('.'))
}

/// The `Content-Type` for a file, decided by its extension and nothing else.
///
/// An unknown extension is `application/octet-stream`. Sniffing the bytes to
/// guess better is the fallback that hides a failure: a wrong guess is served
/// with the same confidence as a right one, and nobody can see which happened.
#[must_use]
pub fn content_type(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase);
    match ext.as_deref() {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        // A source map is JSON, and so is `_app/version.json`.
        Some("json" | "map") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml; charset=utf-8",
        Some("woff2") => "font/woff2",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// One response, built in one place.
fn answer(status: StatusCode, content_type: &'static str, body: Vec<u8>) -> Response {
    (status, [(header::CONTENT_TYPE, content_type)], body).into_response()
}

/// A plain page, for the two states a browser has to be able to read.
fn page(status: StatusCode, title: &str, body: &str) -> Response {
    let html = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\">\
         <meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">\
         <title>{}</title><style>\
         body{{font:15px/1.6 ui-monospace,SFMono-Regular,Menlo,monospace;\
         margin:0;padding:48px 24px;background:#0b0d10;color:#e6e9ef}}\
         main{{max-width:64ch;margin:0 auto}}\
         h1{{font-size:19px;margin:0 0 18px}}\
         code{{background:#171b21;padding:2px 6px;border-radius:4px;\
         word-break:break-all}}\
         a{{color:#7aa2f7}}\
         </style></head><body><main>{}</main></body></html>",
        crate::render::escape(title),
        body
    );
    answer(status, "text/html; charset=utf-8", html.into_bytes())
}

/// The most entries the startup walk visits before it stops counting.
///
/// A ceiling, not an expectation. The built front end is a few dozen files in
/// a handful of directories; a tree orders of magnitude past that means
/// [`WEB_ENV`] names something which is not a bundle, and walking it would make
/// the cost of *starting this server* a function of whatever happens to be on
/// the operator's disk. Bounded by structure and never by data — the walk is
/// once per process either way, and this is what keeps the once from being
/// expensive. Reaching it is reported, not hidden: see [`Tree::capped`].
const MAX_WALK: u64 = 100_000;

/// What the built asset directory actually holds, counted once at startup.
///
/// **Counts, never names.** The file list under a bundle grows with the front
/// end, and half of it is `_app/immutable/…` with a content hash in the name,
/// so a line carrying the list would be a field whose width is somebody else's
/// decision. Four integers answer the only question the log is being asked:
/// *is there a real build there, or a directory that merely exists.*
#[derive(Debug, Clone, Copy)]
struct Tree {
    /// Entries at any depth that are not directories.
    files: u64,
    /// Directories below the root, not counting the root itself.
    dirs: u64,
    /// Directories that would not open and entries the filesystem would not
    /// describe. Counted rather than swallowed: a walk that answered `files:0`
    /// for a bundle it could not read would be `CLAUDE.md` §4's banned shape,
    /// a failure reported as an ordinary empty result.
    unreadable: u64,
    /// Whether the walk stopped at its own ceiling, which makes every count
    /// above it a floor rather than a total. `CLAUDE.md` §3 rule 6.
    capped: bool,
}

/// Every entry under `root`, counted and never named.
///
/// Split from [`walk`] for the reason [`web_dir_from`] is split from
/// [`web_dir`]: both outcomes have to be testable, and no fixture a test can
/// build on a normal disk holds [`MAX_WALK`] entries.
///
/// Iterative rather than recursive, and keyed on [`std::fs::DirEntry::file_type`],
/// which does **not** follow a symlink — a link pointing at its own parent is
/// one entry here and not an infinite descent. That is the same fact
/// [`Assets::resolve`] canonicalises for: a symlink is the one thing in a
/// directory tree that turns a walk which terminates on paper into one that
/// does not.
fn walk_within(root: &Path, limit: u64) -> Tree {
    let mut tree = Tree {
        files: 0,
        dirs: 0,
        unreadable: 0,
        capped: false,
    };
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if tree.files.saturating_add(tree.dirs) >= limit {
            tree.capped = true;
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            tree.unreadable = tree.unreadable.saturating_add(1);
            continue;
        };
        for entry in entries {
            match entry.and_then(|found| found.file_type().map(|kind| (kind, found))) {
                Ok((kind, found)) if kind.is_dir() => {
                    tree.dirs = tree.dirs.saturating_add(1);
                    stack.push(found.path());
                }
                Ok(_) => tree.files = tree.files.saturating_add(1),
                Err(_) => tree.unreadable = tree.unreadable.saturating_add(1),
            }
        }
    }
    tree
}

/// Every entry under `root`, up to [`MAX_WALK`] of them.
fn walk(root: &Path) -> Tree {
    walk_within(root, MAX_WALK)
}

/// A count that was taken, or the absence of one.
///
/// `-1` and never `0`. A directory that was never walked does not hold zero
/// files — it holds no count, and `CLAUDE.md` §3 rule 1 is that the two are
/// different facts. An operator reading `files:0` off a run where the tree was
/// never there would go looking for an empty bundle that does not exist.
fn counted(value: Option<u64>) -> telemetry::Value<'static> {
    value.map_or(telemetry::Value::Int(-1), telemetry::Value::Uint)
}

/// A fact that was established, or the absence of one — see [`counted`].
///
/// `false` is a claim about the world and `null` is the admission that nobody
/// looked. `shell:false` on a run with no asset directory would say the shell
/// is missing from a build, when there is no build to be missing from.
fn known(value: Option<bool>) -> telemetry::Value<'static> {
    value.map_or(telemetry::Value::Null, telemetry::Value::Bool)
}

/// Where the front end was looked for and what was found there, once.
///
/// # What was invisible before
///
/// A front end that is silently not being served looks exactly like a broken
/// one from the browser: a blank page either way. The 503 page names the
/// directory, but only to whoever loads it — nothing outside the browser said
/// which path this process resolved, and `BRUTEX_WEB` pointing at a second
/// checkout, or a binary copied off the machine that built it, produced a
/// server that ran perfectly and served a page from nowhere. An operator can
/// now answer *which directory is this process serving, and does it hold a
/// build* from the log alone, before a browser is opened.
///
/// The counts are what separate the two failures that look identical. A
/// directory that exists with `files:1` is a checkout with an empty `build/`;
/// `shell:false` is a bundle whose `index.html` never got written; and
/// `unreadable` above zero is a permission problem wearing the costume of an
/// empty build.
///
/// # What it costs
///
/// One walk of a few dozen entries, once per process, bounded by [`MAX_WALK`]
/// rather than by the tree. Nothing per request, and nothing per file served —
/// `CLAUDE.md` §3 rule 4 is about the per-operation cost, and this is not on an
/// operation.
fn note_front_end(named: &Path, root: Option<&Path>, build: &Build) {
    let tree = root.map(walk);
    let shell = root.map(|dir| dir.join(INDEX).is_file());
    let _dropped_when_filtered = telemetry::emit(
        &telemetry::Event::new(
            if root.is_some() {
                telemetry::Level::Info
            } else {
                telemetry::Level::Warn
            },
            "api.assets",
            if root.is_some() {
                "front end found"
            } else {
                "front end NOT on disk — every page answers 503"
            },
        )
        .with("path", telemetry::Value::Str(&named.display().to_string()))
        .with("built", telemetry::Value::Bool(root.is_some()))
        .with("files", counted(tree.map(|held| held.files)))
        .with("dirs", counted(tree.map(|held| held.dirs)))
        .with("unreadable", counted(tree.map(|held| held.unreadable)))
        .with("capped", known(tree.map(|held| held.capped)))
        .with("shell", known(shell))
        // THE VERDICT, NOT ONLY ITS INPUTS. `built:true files:3 shell:false`
        // was four fields a reader had to combine to learn the one thing they
        // came for; this is that answer, in the words the banner uses.
        .with("state", telemetry::Value::Str(build.word()))
        .with("state_note", telemetry::Value::Str(&build.note())),
    );
}

/// What the named build directory actually is.
///
/// # The one bit this replaces was true for an empty directory
///
/// `built()` was `self.root.is_some()`, and `root` was `canonicalize().ok()`
/// filtered by `is_dir()`. Existence was the whole test, so a `build/` that a
/// half-finished `npm run build` left empty, a checkout whose bundle was never
/// built, and a `BRUTEX_WEB` pointing at last month's tree all printed the same
/// word on the banner: **serving**. The operator's only statement about the
/// front end was a bit that could not distinguish a working bundle from an
/// empty folder, while every page answered 503 — `CLAUDE.md` §4, a fallback
/// that hides a failure.
///
/// Four answers now, and each names the fix. `built()` is unchanged and still
/// means *the directory resolved*, because that is what [`Assets::respond`]
/// branches on; this is what the banner and the log say.
///
/// # Staleness is a comparison, not a guess
///
/// [`Build::Stale`] is `mtime(build/index.html) < mtime(newest file under
/// web/src)`. Both sides are measurements. When `web/src` is not there — a
/// binary shipped with a bundle and no sources — nothing is compared and
/// nothing is claimed: the answer is [`Build::Serving`], because a comparison
/// that cannot be made is not evidence of freshness *or* of staleness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Build {
    /// No directory resolved. Every page answers 503.
    Missing,
    /// The directory is there and holds no readable `index.html`.
    NoShell {
        /// Why the shell could not be read — absent, or the reason it could not
        /// be stat-ed.
        why: String,
    },
    /// The shell is older than a file under `web/src`.
    Stale {
        /// The newest source file, named — so the operator can see which edit
        /// the bundle does not carry.
        newer: String,
        /// How many seconds newer that file is than the shell.
        by_secs: u64,
    },
    /// A shell is there, and no source is newer than it.
    Serving,
}

impl Build {
    /// How many entries under `web/src` are looked at before the walk stops.
    ///
    /// The same bound and the same reason as [`MAX_WALK`]: a startup answer may
    /// not become a function of somebody's `node_modules`.
    const MAX_SOURCES: u64 = MAX_WALK;

    /// Reads the state of the build beside its sources.
    fn read(web: &Path, root: Option<&Path>) -> Self {
        let Some(root) = root else {
            return Self::Missing;
        };
        let shell = root.join(INDEX);
        let built_at = match std::fs::metadata(&shell).and_then(|m| m.modified()) {
            Ok(at) => at,
            Err(why) => {
                return Self::NoShell {
                    why: format!("{} — {why}", shell.display()),
                };
            }
        };
        // NOT THERE IS NOT A VERDICT. A tree with no `web/src` — a binary
        // shipped beside a bundle — has nothing to compare against, and an
        // absent comparison may not be reported as a fresh one OR as a stale
        // one. `CLAUDE.md` §3 rule 6.
        let src = web.join(SOURCES);
        let Some((newest, at)) = newest_under(&src, Self::MAX_SOURCES) else {
            return Self::Serving;
        };
        match at.duration_since(built_at) {
            Ok(gap) if gap.as_secs() > 0 => Self::Stale {
                newer: newest.display().to_string(),
                by_secs: gap.as_secs(),
            },
            // `Err` is the ordinary case: the shell is NEWER than every source,
            // so the subtraction runs backwards. Sub-second is not staleness —
            // a build writes its own output while the walk is running.
            _ => Self::Serving,
        }
    }

    /// Whether this build can answer a page.
    #[must_use]
    pub const fn serving(&self) -> bool {
        matches!(self, Self::Serving | Self::Stale { .. })
    }

    /// The word for a log field. One token, never a sentence.
    #[must_use]
    pub const fn word(&self) -> &'static str {
        match *self {
            Self::Missing => "missing",
            Self::NoShell { .. } => "no-shell",
            Self::Stale { .. } => "STALE",
            Self::Serving => "serving",
        }
    }

    /// The banner's parenthesis: the word, and the reason when there is one.
    #[must_use]
    pub fn note(&self) -> String {
        match *self {
            Self::Missing => "NOT BUILT — / says so and names the command".to_owned(),
            Self::NoShell { ref why } => format!(
                "NO SHELL — the directory is there and {INDEX} is not readable: {why}. \
                 Every page answers 503. Run `npm run build` in web/"
            ),
            Self::Stale { ref newer, by_secs } => format!(
                "STALE — {newer} is {by_secs}s newer than {INDEX}. \
                 The bundle being served does not carry that edit. \
                 Run `npm run build` in web/"
            ),
            Self::Serving => "serving".to_owned(),
        }
    }
}

/// The newest file under `dir`, and when it was written.
///
/// `None` when the directory is not there, holds no file, or holds none whose
/// modification time can be read — three absences, and not one of them is a
/// timestamp this may invent.
///
/// Iterative, bounded, and keyed on [`std::fs::DirEntry::file_type`] so a
/// symlink is one entry rather than a descent — the same rule, and the same
/// reason, as [`walk_within`].
fn newest_under(dir: &Path, limit: u64) -> Option<(PathBuf, std::time::SystemTime)> {
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut seen: u64 = 0;
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        if seen >= limit {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&next) else {
            continue;
        };
        for entry in entries.flatten() {
            seen = seen.saturating_add(1);
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(entry.path());
                continue;
            }
            let Ok(at) = entry.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            if best.as_ref().is_none_or(|(_, held)| at > *held) {
                best = Some((entry.path(), at));
            }
        }
    }
    best
}

/// The front end on disk, and everything that decides what a path answers.
#[derive(Debug)]
pub struct Assets {
    /// The built asset directory as it was named, whether or not it exists.
    /// This is what the honest page prints, so the operator can see the
    /// directory that was actually looked in rather than the one they assumed.
    named: PathBuf,
    /// The canonicalised root, when it is there and is a directory. `None` is
    /// the not-built state, and it is a state the server runs in.
    root: Option<PathBuf>,
    /// What was found inside that root, read once at startup. See [`Build`].
    build: Build,
    /// The type-ahead script. A source file beside the build, not inside it.
    typeahead: PathBuf,
    /// `web/masters.js` -- see [`MASTERS`], which records why it is not embedded
    /// and why its absence is not the same failure as [`TYPEAHEAD`]'s.
    masters: PathBuf,
    /// How many requests have named an asset that is not on disk. Held so the
    /// warning in [`Assets::note_missing`] can fire on a doubling rather than
    /// on every request, which is the difference between a diagnostic and a
    /// denial of service against this server's own log.
    missing: AtomicU64,
}

impl Assets {
    /// Reads where the front end is, once, at startup.
    ///
    /// The root is canonicalised here rather than per request, so the
    /// comparison every request makes is against a path with no `..` and no
    /// symlink left in it. One `canonicalize` at startup; one per request for
    /// the target, which is unavoidable — it is the check itself.
    #[must_use]
    pub fn new(web: &Path) -> Self {
        let named = web.join(BUILD_DIR);
        let root = std::fs::canonicalize(&named)
            .ok()
            .filter(|resolved| resolved.is_dir());
        // WHAT IS IN IT, NOT MERELY THAT IT IS THERE. See [`Build`].
        let build = Build::read(web, root.as_deref());
        // ONCE PER PROCESS, AND THE ONLY PLACE THIS ANSWER EXISTS. See
        // [`note_front_end`]: the directory this server resolved was written
        // nowhere outside the 503 page, so "the front end is not being served"
        // and "the front end is broken" were the same observation.
        note_front_end(&named, root.as_deref(), &build);
        Self {
            named,
            root,
            build,
            typeahead: web.join(TYPEAHEAD),
            masters: web.join(MASTERS),
            missing: AtomicU64::new(0),
        }
    }

    /// What the named directory actually holds — the answer [`Assets::built`]
    /// could not give. Read once, at startup, and never per request.
    #[must_use]
    pub fn build(&self) -> &Build {
        &self.build
    }

    /// The built asset directory this was pointed at, existing or not.
    #[must_use]
    pub fn named(&self) -> &Path {
        &self.named
    }

    /// Whether a front-end build was found.
    #[must_use]
    pub fn built(&self) -> bool {
        self.root.is_some()
    }

    /// The file a request names, once it is proven to be inside the root.
    ///
    /// `Ok(None)` means nothing is there — which is not an error, because an
    /// absent path is either a 404 or the client-routed shell and this function
    /// does not decide which.
    ///
    /// # Errors
    ///
    /// [`Refusal::Escaped`] when the path resolves outside `root`. That is the
    /// symlink case: every textual rule has already passed, the filesystem
    /// answered, and the answer is somewhere else.
    fn resolve(root: &Path, segments: &[String]) -> Result<Option<PathBuf>, Refusal> {
        let joined = segments
            .iter()
            .fold(root.to_path_buf(), |path, segment| path.join(segment));
        match std::fs::canonicalize(&joined) {
            Err(_) => Ok(None),
            Ok(real) if real.starts_with(root) => Ok(Some(real)),
            Ok(_) => Err(Refusal::Escaped),
        }
    }

    /// `/favicon.ico` is answered by `favicon.svg`, and the alias lives here.
    ///
    /// # Why an alias rather than a second file
    ///
    /// A browser asks for `/favicon.ico` by convention whether or not the page
    /// declares an icon, and `web/build` holds `favicon.svg` and no `.ico`. The
    /// miss is not free: it costs TWO warn-level events per page load — one
    /// `api.request` 404 and one `api.assets` "no such asset" — and measured on
    /// the running binary that was 28 of the last 200 events, 17% of the log,
    /// all of it the console talking about itself on the page an operator opens
    /// to read that log.
    ///
    /// Shipping a real `.ico` was tried first and rejected on evidence: a
    /// hand-built PNG-in-ICO passed `file(1)`, reported itself as a valid 64x64,
    /// and decoded to fully transparent in the browser. A binary asset nobody in
    /// this repository can render is one nobody can check.
    ///
    /// # Why here and not in the router
    ///
    /// This function is already the one place that answers "which file on disk
    /// answers this path". A route would put a second answer to that question in
    /// `server.rs`, and the two would drift the day the build output moves.
    ///
    /// The substitution is on the SEGMENTS, before `resolve`, so the alias
    /// inherits every rule that follows it — the traversal guard, the symlink
    /// escape check, and `content_type`, which reads the RESOLVED name and
    /// therefore answers `image/svg+xml` rather than claiming to be an icon.
    /// A missing `favicon.svg` still 404s exactly as before; this redirects the
    /// question, it does not invent an answer.
    /// `first()` AND NOT `[0]`: this workspace denies `indexing_slicing`, and
    /// the lint is right even where the length check makes the index provably
    /// safe — the guarantee lives in a different expression from the access, and
    /// the two drift when one is edited.
    fn favicon_alias(segments: Vec<String>) -> Vec<String> {
        if segments.len() == 1 && segments.first().is_some_and(|only| only == "favicon.ico") {
            return vec!["favicon.svg".to_owned()];
        }
        segments
    }

    /// Answers one request for a static path.
    ///
    /// The order is the whole contract, and it is the order of this function:
    /// a refusal beats everything; a real file beats the shell; the shell beats
    /// a 404 only for a path that does not look like an asset.
    #[must_use]
    pub fn respond(&self, method: &axum::http::Method, raw_path: &str) -> Response {
        if method != axum::http::Method::GET && method != axum::http::Method::HEAD {
            return answer(
                StatusCode::METHOD_NOT_ALLOWED,
                "text/plain; charset=utf-8",
                format!("{method} is not a method the front end answers\n").into_bytes(),
            );
        }
        let Some(root) = self.root.as_deref() else {
            return self.not_built();
        };
        let segments = match segments(raw_path) {
            Ok(segments) => segments,
            Err(refusal) => return refused(refusal),
        };
        // AFTER the textual rules and BEFORE the filesystem, so the alias is
        // subject to every guard that follows rather than stepping around them.
        let segments = Self::favicon_alias(segments);
        let target = match Self::resolve(root, &segments) {
            Ok(target) => target,
            Err(refusal) => return refused(refusal),
        };
        // READ RATHER THAN ASK. `is_file()` then `read()` is two answers to one
        // question with a window between them, and a directory, a file deleted
        // in that window and a file this process cannot open all end in the
        // same place anyway: there is nothing here to send.
        if let Some(real) = target
            && let Ok(bytes) = std::fs::read(&real)
        {
            return answer(StatusCode::OK, content_type(&real), bytes);
        }
        if looks_like_asset(&segments) {
            return self.not_found(raw_path);
        }
        Self::shell(root)
    }

    /// The type-ahead script, read at run time like everything else.
    ///
    /// It used to be `include_str!("../../../web/typeahead.js")`, which made
    /// this crate unbuildable the moment `web/` was not beside it — the exact
    /// coupling gate 1e detaches the tree to find. It is a source file rather
    /// than build output, so it sits beside the build directory rather than
    /// inside it, and it is served by the same code path as everything else.
    #[must_use]
    pub fn typeahead(&self) -> Response {
        match std::fs::read(&self.typeahead) {
            Ok(bytes) => answer(StatusCode::OK, content_type(&self.typeahead), bytes),
            Err(e) => answer(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                format!(
                    "{} could not be read: {e}\n\
                     The instruments page renders every row without it; the \
                     type-ahead is progressive enhancement and nothing else \
                     depends on this file.\n",
                    self.typeahead.display()
                )
                .into_bytes(),
            ),
        }
    }

    /// `web/masters.js`, read from disk at request time.
    ///
    /// # Why this is not [`Self::typeahead`] with a different path
    ///
    /// The two files fail differently and the refusals have to say so.
    /// `/typeahead.js` is progressive enhancement: the instruments page renders
    /// every row without it, so its 404 correctly tells the reader nothing
    /// depends on it. `/masters.js` fills the status cells of a table that is
    /// otherwise empty and drives the verify button, so its absence is the
    /// difference between an answer and a blank page.
    ///
    /// `CLAUDE.md` §4 bans "a fallback that hides a failure" and prescribes the
    /// alternative in the same row: degrade loudly and name the reason. Naming
    /// the wrong reason is the same defect one step further on, which is why
    /// this is a second function rather than a shared one.
    #[must_use]
    pub fn masters(&self) -> Response {
        match std::fs::read(&self.masters) {
            Ok(bytes) => answer(StatusCode::OK, content_type(&self.masters), bytes),
            Err(e) => answer(
                StatusCode::NOT_FOUND,
                "text/plain; charset=utf-8",
                format!(
                    "{} could not be read: {e}\n\
                     The masters page cannot fill its status cells or run a \
                     verification without this file. Unlike the type-ahead, it \
                     is not an enhancement — the table stays empty. The page \
                     itself still renders, so the nav and the refusal are both \
                     visible rather than a blank screen.\n",
                    self.masters.display()
                )
                .into_bytes(),
            ),
        }
    }

    /// The client-routed shell, for a path that is not an asset.
    fn shell(root: &Path) -> Response {
        let index = root.join(INDEX);
        match std::fs::read(&index) {
            Ok(bytes) => answer(StatusCode::OK, "text/html; charset=utf-8", bytes),
            Err(e) => page(
                StatusCode::SERVICE_UNAVAILABLE,
                "brutex · no front-end shell",
                &format!(
                    "<h1>The asset directory is there and <code>{}</code> is not.</h1>\
                     <p>Looked in <code>{}</code> and the read said: {}.</p>\
                     <p>Every JSON route on this server still answers, and so \
                     does every server-rendered page: \
                     <a href=\"/dashboard\">/dashboard</a>, \
                     <a href=\"/instruments\">/instruments</a>, \
                     <a href=\"/pull\">/pull</a>, \
                     <a href=\"/store\">/store</a>, \
                     <a href=\"/audit\">/audit</a>.</p>",
                    crate::render::escape(INDEX),
                    crate::render::escape(&index.display().to_string()),
                    crate::render::escape(&e.to_string()),
                ),
            ),
        }
    }

    /// One request that named an asset the build did not emit.
    ///
    /// # What was invisible before
    ///
    /// A stale `index.html` pointing at a hashed chunk the bundler no longer
    /// writes is the front end's quietest failure: the shell loads, the page
    /// stays blank, and the whole record is a 404 in a browser console nobody
    /// is watching. The server said nothing at all — the 404 body names the
    /// path, and then it is gone with the response. An operator can now answer
    /// *did the browser ask this server for a file that is not in the build*
    /// after the fact, which is when the question is usually asked.
    ///
    /// # Why it is not one line per request
    ///
    /// [`Assets::respond`] is reachable by anything that can open a socket, so
    /// the rate of this event is set by the caller rather than by this
    /// repository. A scanner walking a wordlist would write a line per probe
    /// and roll the run's own beginning out of the sink's 64 MiB window —
    /// evidence destroying itself, which is the shape D-0072 forbids and the
    /// reason a per-request `warn` is not simply added here.
    ///
    /// So the line fires on a **doubling**: the 1st miss, the 2nd, the 4th,
    /// the 8th. A million probes cost twenty lines, the first miss is still
    /// immediate rather than batched, and each line carries the running total
    /// so the count is read off the line instead of counted from the file.
    /// `docs/06-limits.md` should record that misses between doublings are
    /// summarised and not individually named — the path on the line is the
    /// path of *that* miss, and the ones in between are only a number.
    ///
    /// The counter is one relaxed atomic add on a path that has already failed.
    /// A request that is served never touches it.
    fn note_missing(&self, raw_path: &str) {
        let seen = self
            .missing
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1);
        if !seen.is_power_of_two() {
            return;
        }
        let _dropped_when_filtered = telemetry::emit(
            &telemetry::Event::warn("api.assets", "no such asset")
                .with("path", telemetry::Value::Str(raw_path))
                .with(
                    "root",
                    telemetry::Value::Str(&self.named.display().to_string()),
                )
                .with("seen", telemetry::Value::Uint(seen)),
        );
    }

    /// A path that looks like an asset, and is not there.
    ///
    /// Honest about which path, because a 404 that does not name what it could
    /// not find sends the reader to the wrong file.
    fn not_found(&self, raw_path: &str) -> Response {
        self.note_missing(raw_path);
        answer(
            StatusCode::NOT_FOUND,
            "text/plain; charset=utf-8",
            format!(
                "no such asset: {raw_path}\nasset root: {}\n",
                self.named.display()
            )
            .into_bytes(),
        )
    }

    /// There is no front end on disk, and this says exactly that.
    ///
    /// `503`, not `200`. The server is up and the JSON is fine; the page is
    /// not, and a monitor reads the status code and nothing else. A `200` with
    /// an apology in the body is the fallback that hides a failure —
    /// `CLAUDE.md` §4.
    fn not_built(&self) -> Response {
        page(
            StatusCode::SERVICE_UNAVAILABLE,
            "brutex · front end not built",
            &format!(
                "<h1>The front end is not on disk.</h1>\
                 <p>Looked in <code>{}</code>.</p>\
                 <p>That directory is committed to this repository, so a clean \
                 checkout has it. If it is missing, either this is not a clean \
                 checkout or <code>{}</code> points somewhere else.</p>\
                 <p>What produces it, from the repository root:</p>\
                 <p><code>{}</code></p>\
                 <p>The server itself is up. Every JSON route answers, the \
                 backfill is running, and every server-rendered page still \
                 works: <a href=\"/dashboard\">/dashboard</a>, \
                 <a href=\"/instruments\">/instruments</a>, \
                 <a href=\"/pull\">/pull</a>, \
                 <a href=\"/store\">/store</a>, \
                 <a href=\"/audit\">/audit</a>, \
                 <a href=\"/autopilot.json\">/autopilot.json</a>.</p>",
                crate::render::escape(&self.named.display().to_string()),
                crate::render::escape(WEB_ENV),
                crate::render::escape(BUILD_COMMAND),
            ),
        )
    }
}

/// The one command that produces the asset directory.
///
/// Named once, printed once, and never run by this crate — `CLAUDE.md` §2 and
/// D-0053: nothing here may invoke a front-end toolchain, at build time or at
/// run time. This is a string an operator reads, not a process this starts.
const BUILD_COMMAND: &str = "npm --prefix web ci && npm --prefix web run build";

/// The response for a path this server will not resolve.
fn refused(refusal: Refusal) -> Response {
    answer(
        refusal.status(),
        "text/plain; charset=utf-8",
        format!("refused: {}\n", refusal.reason()).into_bytes(),
    )
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// A front-end directory holding a build, named after the test.
    ///
    /// Returns the *web* directory — the one [`Assets::new`] takes — so a test
    /// gets `<dir>/build` for the assets and `<dir>/typeahead.js` beside it.
    fn web(name: &str) -> PathBuf {
        let dir = crate::scratch::path(&format!("assets-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(BUILD_DIR)).expect("mkdir");
        dir
    }

    /// Writes one file under the build directory, creating its parents.
    fn put(dir: &Path, rel: &str, body: &str) {
        let path = dir.join(BUILD_DIR).join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        let mut f = std::fs::File::create(&path).expect("create");
        f.write_all(body.as_bytes()).expect("write");
    }

    /// A directory with a shell and one real asset.
    fn furnished(name: &str) -> PathBuf {
        let dir = web(name);
        put(&dir, INDEX, "<!doctype html><title>shell</title>");
        put(&dir, "_app/immutable/entry/app.js", "export const x = 1;");
        dir
    }

    /// The status and body of a response, read to the end.
    async fn read(response: Response) -> (StatusCode, String, String) {
        let status = response.status();
        let mime = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
            .await
            .expect("body");
        (status, mime, String::from_utf8_lossy(&bytes).into_owned())
    }

    /// A GET of one path against a furnished directory.
    async fn get(assets: &Assets, path: &str) -> (StatusCode, String, String) {
        read(assets.respond(&axum::http::Method::GET, path)).await
    }

    #[test]
    fn the_directory_is_the_environment_or_web_beside_the_workspace() {
        assert_eq!(
            web_dir_from(Some("/somewhere/else".into())),
            PathBuf::from("/somewhere/else"),
            "the environment wins when it is set"
        );
        // Compared against `default_web_dir` rather than only against
        // `web_dir_from(None)`, which calls it: a function compared only
        // against itself cannot be falsified.
        assert_eq!(web_dir_from(None), default_web_dir());
        assert!(
            default_web_dir().ends_with("web"),
            "the default names the front-end directory: {}",
            default_web_dir().display()
        );
        assert!(
            default_web_dir().is_absolute(),
            "so `cargo run` finds it from any working directory"
        );
        assert_eq!(
            web_beside(Path::new("/a/crates/api")),
            PathBuf::from("/a/web"),
            "two directories up from crates/api, then web"
        );
        assert_eq!(
            web_beside(Path::new("")),
            PathBuf::from("web"),
            "a manifest path with no grandparent still names something"
        );
        assert_eq!(
            web_dir(),
            web_dir_from(std::env::var_os(WEB_ENV)),
            "one reader, one answer"
        );
        assert_eq!(WEB_ENV, "BRUTEX_WEB", "the name is documented and pinned");
    }

    // ---------------------------------------------------------------------
    // 4. PATH TRAVERSAL. One test per spelling, and each of them fails if the
    //    rule it names is removed.
    // ---------------------------------------------------------------------

    #[tokio::test]
    async fn a_parent_directory_is_refused_in_every_spelling() {
        let dir = furnished("traversal");
        // The file the traversals are reaching for: one level above the root,
        // which is inside the scratch directory and therefore really readable.
        let secret = dir.join("secret.txt");
        std::fs::write(&secret, "SECRET").expect("write");
        let assets = Assets::new(&dir);

        for path in [
            "/../secret.txt",
            "/%2e%2e/secret.txt",
            "/..%2fsecret.txt",
            "/..%5csecret.txt",
            "/a/../../secret.txt",
            "/..",
            "/%2E%2E/secret.txt",
            "/_app/../../secret.txt",
        ] {
            let (status, _, body) = get(&assets, path).await;
            assert_eq!(
                status,
                StatusCode::BAD_REQUEST,
                "{path} must be refused, not answered: {body}"
            );
            assert!(
                body.contains(Refusal::NotAName.reason()),
                "{path} names the rule that refused it: {body}"
            );
            assert!(!body.contains("SECRET"), "{path} leaked the file: {body}");
        }
    }

    #[tokio::test]
    async fn a_double_encoded_traversal_is_a_name_and_not_a_traversal() {
        let dir = furnished("double");
        std::fs::write(dir.join("secret.txt"), "SECRET").expect("write");
        let assets = Assets::new(&dir);
        // Decoded ONCE this is the literal name `%2e%2e`. Decoded twice it is
        // `..`, which is the defect. It must be a 404 and never a file above
        // the root.
        let (status, _, body) = get(&assets, "/%252e%252e/secret.txt").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert!(!body.contains("SECRET"), "{body}");
        assert_eq!(
            percent_decode("/%252e%252e").expect("decodes"),
            b"/%2e%2e".to_vec(),
            "one pass, not to a fixed point"
        );
    }

    #[tokio::test]
    async fn a_null_byte_is_refused() {
        let dir = furnished("null");
        let assets = Assets::new(&dir);
        let (status, _, body) = get(&assets, "/index.html%00.js").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(body.contains(Refusal::NullByte.reason()), "{body}");
        assert_eq!(segments("/a%00b"), Err(Refusal::NullByte));
    }

    #[tokio::test]
    async fn a_malformed_escape_is_refused_rather_than_taken_literally() {
        let dir = furnished("escape");
        let assets = Assets::new(&dir);
        for path in ["/%zz", "/%2", "/%"] {
            let (status, _, body) = get(&assets, path).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {body}");
            assert!(body.contains(Refusal::BadEscape.reason()), "{body}");
        }
        assert_eq!(percent_decode("%zz"), Err(Refusal::BadEscape));
        assert_eq!(percent_decode("%4"), Err(Refusal::BadEscape));
        assert_eq!(percent_decode("%41"), Ok(b"A".to_vec()));
        assert_eq!(percent_decode("%4a"), Ok(b"J".to_vec()));
        assert_eq!(percent_decode("%4A"), Ok(b"J".to_vec()));
    }

    #[tokio::test]
    async fn a_path_that_is_not_utf8_is_refused() {
        let dir = furnished("utf8");
        let assets = Assets::new(&dir);
        let (status, _, body) = get(&assets, "/%ff%fe").await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
        assert!(body.contains(Refusal::NotUtf8.reason()), "{body}");
    }

    #[test]
    fn a_leading_separator_is_not_an_ordinary_name_and_a_backslash_is_a_separator() {
        assert!(is_one_name("app.js"), "an ordinary name passes");
        assert!(!is_one_name(".."), "the parent directory does not");
        assert!(!is_one_name("."), "nor the current one");
        assert!(!is_one_name("/etc"), "nor an absolute path");
        assert!(!is_one_name("a/b"), "nor two names in one segment");
        assert!(!is_one_name(""), "nor nothing");
        // A BACKSLASH IS A SEPARATOR HERE, and that is a decision this module
        // takes rather than one the platform takes for it. On a unix target
        // `\` is an ordinary character in a file name, so `..\x` would arrive
        // as ONE component that `is_one_name` accepts — [`segments`] splits on
        // it first, which is what makes `..%5c` refusable at all.
        assert_eq!(
            segments("/a\\b").expect("split"),
            vec!["a".to_owned(), "b".to_owned()],
            "a backslash separates"
        );
        assert_eq!(segments("/..\\x"), Err(Refusal::NotAName));
        // A doubled separator is empty rather than illegal, and is skipped.
        assert_eq!(
            segments("//a///b").expect("skips empties"),
            vec!["a".to_owned(), "b".to_owned()]
        );
        assert_eq!(segments("/").expect("root"), Vec::<String>::new());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlink_pointing_out_of_the_root_is_refused() {
        let dir = furnished("symlink");
        let outside = dir.join("outside.txt");
        std::fs::write(&outside, "SECRET").expect("write");
        std::os::unix::fs::symlink(&outside, dir.join(BUILD_DIR).join("out.txt")).expect("symlink");
        let assets = Assets::new(&dir);
        let (status, _, body) = get(&assets, "/out.txt").await;
        assert_eq!(
            status,
            StatusCode::FORBIDDEN,
            "a symlink out of the root is refused: {body}"
        );
        assert!(body.contains(Refusal::Escaped.reason()), "{body}");
        assert!(!body.contains("SECRET"), "and it leaks nothing: {body}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_symlink_staying_inside_the_root_is_served() {
        // The other half of the rule: the check is "outside the root", not
        // "is a symlink". Without this a mutant refusing every symlink passes
        // the test above.
        let dir = furnished("symlink-ok");
        put(&dir, "real.js", "export const ok = 1;");
        std::os::unix::fs::symlink(
            dir.join(BUILD_DIR).join("real.js"),
            dir.join(BUILD_DIR).join("alias.js"),
        )
        .expect("symlink");
        let assets = Assets::new(&dir);
        let (status, mime, body) = get(&assets, "/alias.js").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(mime, "text/javascript; charset=utf-8");
        assert!(body.contains("export const ok"), "{body}");
    }

    // ---------------------------------------------------------------------
    // 2. ROUTING ORDER, the half this module owns: a real asset, then the
    //    shell, then an honest 404.
    // ---------------------------------------------------------------------

    #[tokio::test]
    async fn a_real_asset_is_served_and_a_missing_one_is_a_404_not_the_shell() {
        let dir = furnished("order");
        let assets = Assets::new(&dir);

        let (status, mime, body) = get(&assets, "/_app/immutable/entry/app.js").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(mime, "text/javascript; charset=utf-8");
        assert!(body.contains("export const x"), "{body}");

        // THE DEFECT THIS TEST EXISTS FOR. A missing script that answers with
        // `index.html` makes the browser report a syntax error in a file that
        // was never the problem.
        let (status, mime, body) = get(&assets, "/assets/missing.js").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert!(!body.contains("<!doctype"), "not the shell: {body}");
        assert_eq!(mime, "text/plain; charset=utf-8");
        assert!(body.contains("/assets/missing.js"), "names it: {body}");

        // A missing file under the bundler's own directory is a 404 even
        // without an extension, because everything under it is generated.
        let (status, _, body) = get(&assets, "/_app/immutable/nope").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");

        // And the bundler's directory itself is not a page.
        let (status, _, body) = get(&assets, "/_app").await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    }

    #[tokio::test]
    async fn a_client_routed_path_gets_the_shell_and_so_does_the_root() {
        let dir = furnished("shell");
        let assets = Assets::new(&dir);
        for path in ["/", "/db", "/ingest", "/autopilot", "/db/anything"] {
            let (status, mime, body) = get(&assets, path).await;
            assert_eq!(status, StatusCode::OK, "{path}: {body}");
            assert_eq!(mime, "text/html; charset=utf-8", "{path}");
            assert!(body.contains("<title>shell</title>"), "{path}: {body}");
        }
    }

    #[tokio::test]
    async fn a_prerendered_page_beats_the_shell() {
        let dir = furnished("prerendered");
        put(&dir, "db.html", "<!doctype html><title>db</title>");
        let assets = Assets::new(&dir);
        // `/db.html` is the file; `/db` is client-routed and gets the shell.
        let (status, _, body) = get(&assets, "/db.html").await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert!(body.contains("<title>db</title>"), "{body}");
    }

    #[tokio::test]
    async fn only_get_and_head_reach_the_front_end() {
        let dir = furnished("method");
        let assets = Assets::new(&dir);
        let (status, _, body) = read(assets.respond(&axum::http::Method::POST, "/db")).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED, "{body}");
        assert!(body.contains("POST"), "it names the method: {body}");
        let (status, _, _) = read(assets.respond(&axum::http::Method::HEAD, "/")).await;
        assert_eq!(status, StatusCode::OK, "HEAD is a GET without a body");
    }

    // ---------------------------------------------------------------------
    // 3. CONTENT TYPES.
    // ---------------------------------------------------------------------

    #[test]
    fn every_extension_the_front_end_emits_has_a_type_and_the_rest_do_not_guess() {
        for (file, expected) in [
            ("a.html", "text/html; charset=utf-8"),
            ("a.js", "text/javascript; charset=utf-8"),
            ("a.mjs", "text/javascript; charset=utf-8"),
            ("a.css", "text/css; charset=utf-8"),
            ("a.json", "application/json; charset=utf-8"),
            ("a.map", "application/json; charset=utf-8"),
            ("a.svg", "image/svg+xml; charset=utf-8"),
            ("a.woff2", "font/woff2"),
            ("a.png", "image/png"),
            ("a.ico", "image/x-icon"),
            ("a.txt", "text/plain; charset=utf-8"),
            ("a.HTML", "text/html; charset=utf-8"),
            ("a.wasm", "application/octet-stream"),
            ("a", "application/octet-stream"),
            ("a.", "application/octet-stream"),
        ] {
            assert_eq!(
                content_type(Path::new(file)),
                expected,
                "{file} is typed by its extension and nothing else"
            );
        }
    }

    // ---------------------------------------------------------------------
    // 5. MISSING ROOT.
    // ---------------------------------------------------------------------

    #[tokio::test]
    async fn a_missing_root_is_named_loudly_and_the_server_still_runs() {
        let dir = crate::scratch::path("assets-absent");
        let _ = std::fs::remove_dir_all(&dir);
        let assets = Assets::new(&dir);
        assert!(!assets.built(), "nothing is there");
        assert!(
            assets.named().ends_with(BUILD_DIR),
            "and it still knows what it looked for: {}",
            assets.named().display()
        );

        let (status, mime, body) = get(&assets, "/").await;
        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "loud, not a blank 200: {body}"
        );
        assert_eq!(mime, "text/html; charset=utf-8");
        assert!(
            body.contains(&crate::render::escape(
                &assets.named().display().to_string()
            )),
            "it names the directory it looked in: {body}"
        );
        assert!(
            body.contains(&crate::render::escape(BUILD_COMMAND)),
            "and the command that creates it: {body}"
        );
        assert!(body.contains(WEB_ENV), "and the override: {body}");
        assert!(
            body.contains("/autopilot.json"),
            "and says the rest of the server is fine: {body}"
        );
        // Every path answers the same way. There is one reason and it is stated
        // once, rather than a 404 that sends the reader looking for a file.
        let (status, _, _) = get(&assets, "/_app/immutable/entry/app.js").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn a_root_that_is_a_file_is_not_a_root() {
        let dir = crate::scratch::path("assets-file");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(dir.join(BUILD_DIR), "not a directory").expect("write");
        let assets = Assets::new(&dir);
        assert!(!assets.built(), "a file named `build` is not a build");
        let (status, _, _) = get(&assets, "/").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    }

    /// **`built()` was true for any directory that exists, and the banner said
    /// `serving` for all of them.**
    ///
    /// Four states, one word each. The empty-directory case is the repository's
    /// own green test one screen down: it asserts `built()` and then asserts
    /// `503` from every page — a build that is *serving* and answers nothing.
    /// A stale one is worse, because it answers, with the bundle from before
    /// the edit.
    #[test]
    fn a_build_directory_that_exists_is_not_the_same_as_a_build() {
        // 1. NO DIRECTORY AT ALL.
        let bare = crate::scratch::path("assets-build-missing");
        let _ = std::fs::remove_dir_all(&bare);
        std::fs::create_dir_all(&bare).expect("mkdir");
        let missing = Assets::new(&bare);
        assert_eq!(*missing.build(), Build::Missing);
        assert!(!missing.build().serving());
        assert_eq!(missing.build().word(), "missing");
        assert!(missing.build().note().contains("NOT BUILT"));

        // 2. THE DIRECTORY IS THERE AND EMPTY — the case that used to print
        //    `serving` while every page answered 503.
        let dir = web("build-empty");
        let empty = Assets::new(&dir);
        assert!(
            empty.built(),
            "the directory resolved, which is all that meant"
        );
        assert!(
            !empty.build().serving(),
            "and it cannot serve a page, which is what the operator is told"
        );
        assert_eq!(empty.build().word(), "no-shell");
        let note = empty.build().note();
        assert!(note.contains("NO SHELL") && note.contains(INDEX), "{note}");
        assert!(note.contains("npm run build"), "and the fix: {note}");

        // 3. A SHELL, AND NO SOURCES TO COMPARE IT WITH. Nothing is claimed
        //    about freshness, because nothing was measured.
        let fresh = furnished("build-fresh");
        assert_eq!(*Assets::new(&fresh).build(), Build::Serving);

        // 4. A SOURCE NEWER THAN THE SHELL.
        let stale = furnished("build-stale");
        let src = stale.join(SOURCES).join("routes");
        std::fs::create_dir_all(&src).expect("mkdir");
        let edited = src.join("+page.svelte");
        std::fs::write(&edited, "<h1>the edit the bundle does not carry</h1>").expect("write");
        // The shell was written first, but a filesystem's resolution is not
        // this test's to assume: the timestamps are set explicitly, so the
        // comparison is over values this test chose.
        let shell = std::fs::File::options()
            .write(true)
            .open(stale.join(BUILD_DIR).join(INDEX))
            .expect("the shell");
        let long_ago =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        shell
            .set_modified(long_ago)
            .expect("a filesystem that carries mtimes");
        let source = std::fs::File::options()
            .write(true)
            .open(&edited)
            .expect("the source");
        source
            .set_modified(long_ago + std::time::Duration::from_mins(10))
            .expect("a filesystem that carries mtimes");

        let built = Assets::new(&stale);
        let Build::Stale { ref newer, by_secs } = *built.build() else {
            panic!(
                "a source newer than the shell is STALE, not {:?}",
                built.build()
            );
        };
        assert!(
            newer.ends_with("+page.svelte"),
            "it names the file: {newer}"
        );
        assert_eq!(by_secs, 600, "and by how much");
        assert!(
            built.build().serving(),
            "a stale bundle does answer pages — that is exactly why it is dangerous"
        );
        let note = built.build().note();
        assert!(
            note.contains("STALE") && note.contains("does not carry that edit"),
            "{note}"
        );
    }

    #[tokio::test]
    async fn a_build_with_no_shell_says_so_rather_than_answering_blank() {
        let dir = web("no-shell");
        let assets = Assets::new(&dir);
        assert!(assets.built(), "the directory is there");
        let (status, mime, body) = get(&assets, "/db").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        assert_eq!(mime, "text/html; charset=utf-8");
        assert!(body.contains(INDEX), "it names the file: {body}");
        assert!(body.contains("/dashboard"), "and where to go: {body}");
    }

    // ---------------------------------------------------------------------
    // 6. THE TYPE-AHEAD, on the same run-time path.
    // ---------------------------------------------------------------------

    #[tokio::test]
    async fn the_typeahead_is_read_from_disk_and_says_so_when_it_is_not_there() {
        let dir = web("typeahead");
        let assets = Assets::new(&dir);

        let (status, _, body) = read(assets.typeahead()).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
        assert!(body.contains(TYPEAHEAD), "it names the file: {body}");
        assert!(
            body.contains("progressive enhancement"),
            "and says nothing breaks: {body}"
        );

        std::fs::write(dir.join(TYPEAHEAD), "export const t = 1;").expect("write");
        let (status, mime, body) = read(assets.typeahead()).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        assert_eq!(mime, "text/javascript; charset=utf-8");
        assert!(body.contains("export const t"), "{body}");
    }

    #[test]
    fn a_refusal_carries_its_own_reason_and_status() {
        for refusal in [
            Refusal::BadEscape,
            Refusal::NotUtf8,
            Refusal::NullByte,
            Refusal::NotAName,
        ] {
            assert_eq!(refusal.status(), StatusCode::BAD_REQUEST, "{refusal:?}");
            assert!(!refusal.reason().is_empty(), "{refusal:?}");
        }
        assert_eq!(Refusal::Escaped.status(), StatusCode::FORBIDDEN);
        // Five distinct sentences, so a response can never say the wrong one.
        let reasons = [
            Refusal::BadEscape.reason(),
            Refusal::NotUtf8.reason(),
            Refusal::NullByte.reason(),
            Refusal::NotAName.reason(),
            Refusal::Escaped.reason(),
        ];
        for (i, a) in reasons.iter().enumerate() {
            for b in reasons.iter().skip(i + 1) {
                assert_ne!(a, b, "two refusals must not read the same");
            }
        }
    }

    // ---------------------------------------------------------------------
    // 7. WHAT THE LOG GETS TOLD. The counts behind `api.assets`, and the gate
    //    that keeps a scanner from writing the run's own beginning away.
    // ---------------------------------------------------------------------

    /// The startup walk counts every depth, stops where it is told to, and says
    /// so when it stopped.
    #[test]
    fn the_walk_counts_every_depth_and_reports_its_own_ceiling() {
        let dir = furnished("walk");
        let root = dir.join(BUILD_DIR);

        let whole = walk_within(&root, MAX_WALK);
        assert_eq!(whole.files, 2, "index.html and app.js");
        assert_eq!(whole.dirs, 3, "_app, immutable, entry");
        assert_eq!(whole.unreadable, 0);
        assert!(!whole.capped, "nothing this small reaches the ceiling");
        assert_eq!(
            walk(&root).files,
            whole.files,
            "one reader, one answer — `walk` is `walk_within` at MAX_WALK"
        );

        // THE CEILING FIRES AND SAYS SO. A count that stopped early and looked
        // like a total is `CLAUDE.md` §3 rule 6's banned shape.
        let clipped = walk_within(&root, 1);
        assert!(clipped.capped, "the walk stopped at its own limit");
        assert!(
            clipped.files.saturating_add(clipped.dirs) < whole.files + whole.dirs,
            "and stopped short: {clipped:?} against {whole:?}"
        );

        // A ROOT THAT WILL NOT OPEN IS NOT AN EMPTY ROOT. Counted, so the line
        // cannot read `files:0` for a bundle nobody could look inside.
        let refused = walk_within(&root.join(INDEX), MAX_WALK);
        assert_eq!(refused.unreadable, 1, "{refused:?}");
        assert_eq!(refused.files, 0, "and it counted nothing it did not see");
    }

    /// A count nobody took is `-1`, and a fact nobody established is `null`.
    ///
    /// Zero and false are claims about the world. `CLAUDE.md` §3 rule 1: an
    /// absent measurement is not a measurement of zero.
    #[test]
    fn a_count_that_was_never_taken_is_not_a_count_of_zero() {
        assert_eq!(counted(Some(0)), telemetry::Value::Uint(0));
        assert_eq!(counted(Some(36)), telemetry::Value::Uint(36));
        assert_eq!(counted(None), telemetry::Value::Int(-1));
        assert_eq!(known(Some(true)), telemetry::Value::Bool(true));
        assert_eq!(known(Some(false)), telemetry::Value::Bool(false));
        assert_eq!(known(None), telemetry::Value::Null);
    }

    /// **Every miss is counted; only a doubling is written.**
    ///
    /// The counter is what makes the warning bounded by structure rather than
    /// by whoever is opening sockets, and the response is what proves the gate
    /// never reaches the client: a probe that happens to land on the third miss
    /// must read exactly like one that lands on the fourth.
    #[tokio::test]
    async fn a_missing_asset_is_counted_every_time_and_the_answer_never_varies() {
        let dir = furnished("missing-count");
        let assets = Assets::new(&dir);
        let mut answers = Vec::new();
        for n in 1..=5u64 {
            let (status, mime, body) = get(&assets, "/gone.js").await;
            assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
            assert_eq!(
                assets.missing.load(Ordering::Relaxed),
                n,
                "miss {n} was not counted, so the total on the line would lie"
            );
            answers.push((mime, body));
        }
        assert!(
            answers.windows(2).all(|pair| pair[0] == pair[1]),
            "the doubling gate decides what is LOGGED and must not decide what \
             is SERVED: {answers:?}"
        );
    }

    #[test]
    fn what_looks_like_an_asset_is_decided_by_the_name() {
        let seg = |s: &str| -> Vec<String> {
            s.split('/')
                .filter(|p| !p.is_empty())
                .map(str::to_owned)
                .collect()
        };
        assert!(looks_like_asset(&seg("/a/b.js")), "an extension");
        assert!(
            looks_like_asset(&seg("/_app/anything")),
            "the bundler's own"
        );
        assert!(!looks_like_asset(&seg("/db")), "a client route");
        assert!(!looks_like_asset(&seg("/")), "and the root");
    }

    #[test]
    fn the_icon_a_browser_asks_for_is_answered_by_the_one_that_exists() {
        let own = |s: &str| vec![s.to_owned()];

        assert_eq!(
            Assets::favicon_alias(own("favicon.ico")),
            own("favicon.svg"),
            "a browser asks for /favicon.ico whether or not the page declares an \
             icon, and web/build holds only the SVG — the miss cost two \
             warn-level events per page load"
        );

        // EVERY OTHER PATH IS UNTOUCHED, and the three below are the ways this
        // could have been written too loosely: a suffix match, a segment match
        // at any depth, and a name that merely contains it.
        assert_eq!(
            Assets::favicon_alias(own("favicon.svg")),
            own("favicon.svg"),
            "the real file still answers for itself"
        );
        assert_eq!(
            Assets::favicon_alias(vec!["_app".to_owned(), "favicon.ico".to_owned()]),
            vec!["_app".to_owned(), "favicon.ico".to_owned()],
            "the alias is the ROOT icon, not any file of that name at any depth"
        );
        assert_eq!(
            Assets::favicon_alias(own("not-a-favicon.ico")),
            own("not-a-favicon.ico"),
            "an equality test and never a suffix one"
        );
        assert!(
            Assets::favicon_alias(Vec::new()).is_empty(),
            "the root path has no segment to alias and must not gain one"
        );
    }
}
