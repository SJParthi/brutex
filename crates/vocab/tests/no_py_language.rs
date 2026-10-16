//! **The Py-language appears nowhere in this repository: not as a file, not as
//! a shebang, and not as its name.**
//!
//! The operator's rule of 16 October 2026 (D-5250) is stricter than `CLAUDE.md`
//! §2. §2 refuses the language by extension outside `web/`; the rule refuses it
//! everywhere, `web/` included, and refuses its NAME in any tracked file, in
//! any case. CI gate 15 already walks every tracked file for the name, but it
//! is a workflow step that `cargo test` never runs, its escape hatch is a count
//! per file, and its own comment says it cannot see the interpreter's binary
//! name or a shebang line. This test is all of that, under `cargo test`.
//!
//! # The needle is assembled, and it must stay assembled
//!
//! This file is tracked, and it reads itself. Spelled whole, the name would
//! refuse this file on its first run, and the only repair would be to exempt
//! the scanner: the one file best placed to hide things. [`NEEDLE`] is two
//! halves that mean nothing apart, and [`NEEDLE_LEN`] is the tripwire for a
//! half edited away, which would leave a needle matching far too much or one
//! matching nothing.
//!
//! # What is walked
//!
//! `git ls-files` from the workspace root: exactly the tracked set, read from
//! the working tree, so an edit is seen before it is committed. A tree with no
//! `.git` (`cargo-mutants` copies the tree without one), or a host where `git`
//! cannot be run, is walked as a directory instead, skipping `.git`, the root
//! `target` and every `node_modules`, and the run says so on stdout. That walk
//! is a superset of the tracked set, so it is stricter, never weaker. A `.git`
//! that `git` cannot list is a failure, not a walk.
//!
//! # No exception
//!
//! There is none: not by file, not by pattern, not by line. D-5250 first kept
//! one, D-0207's line naming the language of the vendor's own SDK, because the
//! ledger is append-only and nothing then allowed an old entry to be edited.
//! The operator allowed that one redaction on 16 October 2026 (D-5251), the
//! line now says "the vendor's own SDK", and the line it used to be is refused
//! here like any other (`no_line_is_excused_not_even_the_one_d0207_held`).
//!
//! # What it cannot see
//!
//! The name split across two lines, written with a zero-width character inside
//! it, or encoded. Anything untracked, when `git` is present. A determined
//! author beats any text rule; this stops the accident and the habit.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The name, assembled from two halves at compile time. If you are here to
/// tidy this into one literal, stop: read the module comment above.
const NEEDLE: &str = concat!("py", "thon");

/// The tripwire for an edited half.
const NEEDLE_LEN: usize = 6;

/// The line D-0207 held until D-5251, whole: the last occurrence of the name in
/// the tree, kept here only so a test can prove it is no longer excused.
/// Assembled for the same reason [`NEEDLE`] is.
const FORMER_EXCEPTION: &str = concat!(
    "second in the vendor's own Py",
    "thon SDK. On the status axis they are one event."
);

/// The extensions of the language's own files: source, stub, bytecode (both
/// kinds), a Windows script, a compiled extension module, a zip application,
/// and a notebook.
const EXTENSIONS: [&str; 8] = ["py", "pyi", "pyc", "pyo", "pyw", "pyd", "pyz", "ipynb"];

/// Interpreter programs whose names do not already contain [`NEEDLE`], which
/// a shebang would name instead of it. Compared after a trailing `.exe` and a
/// trailing version (digits and dots) are taken off.
const INTERPRETERS: [&str; 4] = ["py", "pypy", "jython", "graalpy"];

/// Why one file is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Breach {
    /// The path itself holds the name.
    Path,
    /// The file is one of the language's own, by this extension (lower case).
    Extension(String),
    /// The name is on this line, counted from 1.
    Name(usize),
    /// A shebang on this line, counted from 1, runs the interpreter.
    Shebang(usize),
}

/// What reading one file found.
#[derive(Debug, Default)]
struct Scan {
    /// Each refusal, with its evidence with the name masked out.
    breaches: Vec<(Breach, String)>,
}

impl Scan {
    /// The refusals without their evidence, for comparing in a test.
    fn kinds(&self) -> Vec<Breach> {
        self.breaches.iter().map(|(b, _)| b.clone()).collect()
    }
}

/// Does `hay` hold the name, in any case? Every byte is read: a NUL or bytes
/// that are not UTF-8 do not stop it, the lesson D-1104 records for gate 15.
fn names_it(hay: &[u8]) -> bool {
    hay.windows(NEEDLE.len())
        .any(|w| w.eq_ignore_ascii_case(NEEDLE.as_bytes()))
}

/// `text` with the name replaced, so a refusal never prints what it refuses.
fn masked(text: &[u8]) -> String {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        if text
            .get(i..i + NEEDLE.len())
            .is_some_and(|w| w.eq_ignore_ascii_case(NEEDLE.as_bytes()))
        {
            out.extend_from_slice(b"[THE NAME]");
            i += NEEDLE.len();
        } else {
            out.push(text[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// The lower-cased text after a path's last dot, or none when its last
/// segment has no dot. A file named only `.py` counts as that extension.
fn extension_of(path: &str) -> Option<String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    name.rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
}

/// The last segment of a program path, either separator.
fn basename(word: &str) -> &str {
    word.rsplit(['/', '\\']).next().unwrap_or(word)
}

/// The program a `#!` line runs, lower-cased, or none when the line is not a
/// shebang. `#!` followed by `[` is a Rust inner attribute, not a shebang:
/// rustc's own rule. Through `env`, the program is the first word that is
/// neither a flag (`-S`) nor an assignment (`NAME=value`).
fn shebang_program(line: &[u8]) -> Option<String> {
    let rest = line.trim_ascii_start().strip_prefix(b"#!")?;
    if rest.trim_ascii_start().starts_with(b"[") {
        return None;
    }
    let rest = String::from_utf8_lossy(rest);
    let mut words = rest.split_whitespace();
    let first = basename(words.next()?);
    let program = if first.eq_ignore_ascii_case("env") {
        basename(words.find(|w| !w.starts_with('-') && !w.contains('='))?)
    } else {
        first
    };
    Some(program.to_ascii_lowercase())
}

/// Does this program name run the language?
fn runs_the_language(program: &str) -> bool {
    let program = program.strip_suffix(".exe").unwrap_or(program);
    let stem = program.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    names_it(program.as_bytes()) || INTERPRETERS.contains(&stem)
}

/// Every refusal one file earns. `path` is relative to the workspace root,
/// with `/` separators, exactly as `git ls-files` prints it.
fn scan(path: &str, bytes: &[u8]) -> Scan {
    let mut out = Scan::default();
    if names_it(path.as_bytes()) {
        out.breaches.push((Breach::Path, masked(path.as_bytes())));
    }
    if let Some(ext) = extension_of(path).filter(|e| EXTENSIONS.contains(&e.as_str())) {
        out.breaches
            .push((Breach::Extension(ext), masked(path.as_bytes())));
    }
    for (index, raw) in bytes.split(|b| *b == b'\n').enumerate() {
        let line = raw.strip_suffix(b"\r").unwrap_or(raw);
        if names_it(line) {
            out.breaches.push((Breach::Name(index + 1), masked(line)));
        }
        if shebang_program(line).is_some_and(|p| runs_the_language(&p)) {
            out.breaches
                .push((Breach::Shebang(index + 1), masked(line)));
        }
    }
    out
}

/// The workspace root.
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `git -C root ARGS`, with the variables that would point it at another
/// repository removed.
fn git(root: &Path, args: &[&str]) -> std::io::Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
}

/// Every file under `root`, relative and `/`-separated, sorted, skipping
/// `.git`, the root's `target` and every `node_modules`. A symlink is listed
/// and never followed.
fn walk(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![PathBuf::new()];
    while let Some(relative) = stack.pop() {
        let at_root = relative.as_os_str().is_empty();
        for entry in
            std::fs::read_dir(root.join(&relative)).expect("a walked directory is readable")
        {
            let entry = entry.expect("a directory entry is readable");
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = relative.join(&name);
            if entry
                .file_type()
                .expect("an entry's type is readable")
                .is_dir()
            {
                let skipped =
                    name == ".git" || name == "node_modules" || (at_root && name == "target");
                if !skipped {
                    stack.push(path);
                }
            } else {
                out.push(path.to_string_lossy().replace('\\', "/"));
            }
        }
    }
    out.sort();
    out
}

/// The files to read under `root`, and one sentence saying how they were
/// found.
fn files_to_read(root: &Path) -> (Vec<String>, String) {
    if std::fs::symlink_metadata(root.join(".git")).is_err() {
        let how = format!(
            "WALKED AS A DIRECTORY: {} holds no `.git` (`cargo-mutants` copies the tree \
             without one), so every file but `.git`, `target` and `node_modules` was read",
            root.display()
        );
        return (walk(root), how);
    }
    let listed = match git(root, &["ls-files", "-z"]) {
        Ok(listed) => listed,
        Err(why) => {
            let how = format!(
                "WALKED AS A DIRECTORY: `git` cannot be run here ({why}), so every file but \
                 `.git`, `target` and `node_modules` was read"
            );
            return (walk(root), how);
        }
    };
    assert!(
        listed.status.success(),
        "{} holds a `.git` and `git ls-files` failed there, so the tracked set is \
         unknown and a walk would pass unread: {}",
        root.display(),
        String::from_utf8_lossy(&listed.stderr).trim()
    );
    let files = listed
        .stdout
        .split(|b| *b == 0)
        .filter(|name| !name.is_empty())
        .map(|name| {
            String::from_utf8(name.to_vec())
                .expect("a tracked path is not UTF-8, so this test cannot name what it reads")
        })
        .collect();
    (files, "`git ls-files`: exactly the tracked set".to_owned())
}

/// A listed file's bytes: a symlink's are its target, which is what git
/// tracks. `None` when the file is listed and absent from the working tree.
fn contents(root: &Path, relative: &str) -> Option<Vec<u8>> {
    let path = root.join(relative);
    let meta = match std::fs::symlink_metadata(&path) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return None,
        meta => meta.expect("a listed file's metadata is readable"),
    };
    if meta.file_type().is_symlink() {
        let target = std::fs::read_link(&path).expect("a listed symlink is readable");
        return Some(target.as_os_str().as_encoded_bytes().to_vec());
    }
    Some(std::fs::read(&path).expect("a listed file is readable as bytes, and a directory is not"))
}

/// Files every walk of this repository reads. The count alone cannot say a
/// walk was real: CI gate 1e tests a copy of the tree with `web/` moved aside,
/// where `git ls-files` still lists those files and fewer than half are on
/// disk. A walk that misses one of these did not read this repository.
const ANCHORS: [&str; 4] = [
    "CLAUDE.md",
    "Cargo.toml",
    "docs/05-decisions.md",
    "crates/vocab/tests/no_py_language.rs",
];

/// What one walk of a tree found.
#[derive(Debug)]
struct Audit {
    /// How the files were found.
    how: String,
    /// Files read.
    read: usize,
    /// How many of [`ANCHORS`] were read.
    anchors: usize,
    /// Files listed and absent from the working tree.
    absent: Vec<String>,
    /// Every refusal, `path: kind: masked evidence`, one a line.
    refused: Vec<String>,
}

/// Read every file `files_to_read` names under `root`, and scan each.
fn audit(root: &Path) -> Audit {
    let (files, how) = files_to_read(root);
    let mut audit = Audit {
        how,
        read: 0,
        anchors: 0,
        absent: Vec::new(),
        refused: Vec::new(),
    };
    for file in files {
        let Some(bytes) = contents(root, &file) else {
            audit.absent.push(file);
            continue;
        };
        audit.read += 1;
        if ANCHORS.contains(&file.as_str()) {
            audit.anchors += 1;
        }
        let found = scan(&file, &bytes);
        for (breach, evidence) in found.breaches {
            audit
                .refused
                .push(format!("{file}: {breach:?}: {evidence}"));
        }
    }
    audit
}

/// A fresh empty directory for one proof.
fn scratch(tag: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("no-py-language-{tag}-{}", std::process::id()));
    let _stale = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

/// Write `text` at `relative` under `dir`, making its parents.
fn plant(dir: &Path, relative: &str, text: &[u8]) {
    let path = dir.join(relative);
    std::fs::create_dir_all(path.parent().expect("a planted file has a parent"))
        .expect("a planted file's parent");
    std::fs::write(path, text).expect("a planted file");
}

// ---------------------------------------------------------------- the gate --

/// THE GATE. No tracked file is the language's, runs its interpreter, or
/// names it. Nothing is excused (D-5251).
#[test]
fn no_tracked_file_is_runs_or_names_the_py_language() {
    let found = audit(&root());
    println!("{}", found.how);
    if !found.absent.is_empty() {
        println!(
            "{} listed file(s) absent from the working tree, so nothing of theirs was \
             read; the first ten: {:?}",
            found.absent.len(),
            found.absent.iter().take(10).collect::<Vec<_>>()
        );
    }
    let listed = found.read + found.absent.len();
    assert!(
        listed > 1_000,
        "listed {listed} file(s), which is not this repository. A silent near-zero is not \
         a pass ({})",
        found.how
    );
    assert_eq!(
        found.anchors,
        ANCHORS.len(),
        "the walk read {} file(s) and missed one of {ANCHORS:?}, so it did not read this \
         repository ({})",
        found.read,
        found.how
    );
    assert!(
        found.refused.is_empty(),
        "The operator's rule of 16 October 2026 (D-5250): the Py-language is used \
         nowhere in this repository and its name appears in no file. Rewrite each line \
         so its meaning stays and the name goes (`the vendor's own SDK`, or name the \
         exact rule); a file of the language is deleted, not renamed. The name is \
         masked below so this message is not the leak:\n{}",
        found.refused.join("\n")
    );
}

// ------------------------------------------------------------- its proofs --

/// The needle is the name, whole, in lower case, and this file never spells it.
#[test]
fn the_needle_is_six_letters_and_this_file_never_spells_it() {
    assert_eq!(NEEDLE.len(), NEEDLE_LEN);
    assert!(NEEDLE.bytes().all(|b| b.is_ascii_lowercase()));
    let me = include_bytes!("no_py_language.rs");
    assert!(!names_it(me), "this file spells the name it refuses");
    let found = scan("crates/vocab/tests/no_py_language.rs", me);
    assert!(found.breaches.is_empty(), "{:?}", found.breaches);
}

/// A planted name is refused in any case, mid-word, after a CR, beside a NUL
/// and in bytes that are not UTF-8, on the right line, and the evidence is
/// masked. The name split by a space is the stated limit and passes.
#[test]
fn a_planted_name_is_refused_in_any_case_and_any_bytes() {
    let upper = NEEDLE.to_ascii_uppercase();
    let mixed = format!("P{}", &NEEDLE[1..]);
    let cases: [(Vec<u8>, usize); 5] = [
        (format!("// the {NEEDLE} sdk\n").into_bytes(), 1),
        (
            format!("line one\r\nPorted from {upper}\r\n").into_bytes(),
            2,
        ),
        (format!("a\nb\nabc{mixed}def").into_bytes(), 3),
        (
            [b"\x00\xff\xfe ".as_slice(), NEEDLE.as_bytes(), b"\x00"].concat(),
            1,
        ),
        (format!("x\n\n\n#!/usr/bin/env {NEEDLE}3\n").into_bytes(), 4),
    ];
    for (bytes, line) in cases {
        let found = scan("crates/x/src/lib.rs", &bytes);
        assert_eq!(
            found.kinds().first(),
            Some(&Breach::Name(line)),
            "{found:?}"
        );
        for (_, evidence) in &found.breaches {
            assert!(
                !names_it(evidence.as_bytes()),
                "the evidence leaks the name"
            );
            assert!(evidence.contains("[THE NAME]"), "{evidence}");
        }
    }
    let split = format!("{} {}", &NEEDLE[..3], &NEEDLE[3..]);
    assert!(scan("docs/x.md", split.as_bytes()).breaches.is_empty());
    assert!(scan("docs/x.md", b"").breaches.is_empty());
}

/// Nothing is excused. D-0207's former line, in the ledger itself and anywhere
/// else, with CRLF or not, is refused at its line; the line as D-5251 left it
/// passes. The ledger is read like every other file.
#[test]
fn no_line_is_excused_not_even_the_one_d0207_held() {
    assert!(names_it(FORMER_EXCEPTION.as_bytes()));
    let former = format!("before\n{FORMER_EXCEPTION}\r\nafter\n");
    for path in [
        "docs/05-decisions.md",
        "docs/04-invariants.md",
        "crates/pull/src/kite.rs",
    ] {
        let found = scan(path, former.as_bytes());
        assert_eq!(found.kinds(), [Breach::Name(2)], "{path}");
        assert!(!names_it(found.breaches[0].1.as_bytes()));
    }
    let redacted = format!(
        "before\n{}\nafter\n",
        FORMER_EXCEPTION.replace(concat!("Py", "thon "), "")
    );
    assert!(redacted.contains("the vendor's own SDK. On the status axis"));
    let found = scan("docs/05-decisions.md", redacted.as_bytes());
    assert!(found.breaches.is_empty(), "{:?}", found.breaches);
    let ledger = include_str!("../../../docs/05-decisions.md");
    assert!(
        ledger.contains("second in the vendor's own SDK. On the status axis they are one event."),
        "D-0207's line is not the one D-5251 left; read it before changing this test"
    );
    assert!(
        scan("docs/05-decisions.md", ledger.as_bytes())
            .breaches
            .is_empty()
    );
}

/// Every extension of the language's own files is refused, in any case and as
/// a bare dotfile; other extensions, and a path naming `py` alone, are not. A
/// path that names the language is refused whatever the file holds.
#[test]
fn a_file_of_the_language_is_refused_by_extension_and_by_path() {
    for ext in EXTENSIONS {
        let upper = ext.to_ascii_uppercase();
        for path in [
            format!("tools/helper.{ext}"),
            format!("web/src/lib/x.{upper}"),
            format!(".{ext}"),
        ] {
            let found = scan(&path, b"");
            assert_eq!(found.kinds(), [Breach::Extension(ext.to_owned())], "{path}");
        }
    }
    for clean in [
        "crates/x/src/lib.rs",
        "web/src/app.js",
        "notes.pyramid",
        "x.pyrs",
        "Makefile",
        ".gitignore",
        "tools/py/readme.md",
    ] {
        assert!(scan(clean, b"").breaches.is_empty(), "{clean}");
    }
    let named = format!("tools/{NEEDLE}_helper/readme.md");
    let found = scan(&named, b"clean text\n");
    assert_eq!(found.kinds(), [Breach::Path]);
    assert!(!names_it(found.breaches[0].1.as_bytes()));
}

/// A shebang that runs the interpreter is refused on any line, directly or
/// through `env`, by any listed name or one holding the needle. A Rust inner
/// attribute, another interpreter, and a name that merely starts like one are
/// not.
#[test]
fn a_shebang_naming_the_interpreter_is_refused() {
    for runs in [
        "#!/usr/bin/env pypy3",
        "#!/usr/bin/pypy",
        "#! /usr/local/bin/jython",
        "#!/usr/bin/env -S py -3",
        "#!/usr/bin/env PATH=/opt/bin graalpy",
        "#!C:\\Launcher\\py.exe",
        "  #!/usr/bin/env pypy3.10",
    ] {
        let text = format!("first\n{runs}\n");
        let found = scan("docs/x.md", text.as_bytes());
        assert_eq!(found.kinds(), [Breach::Shebang(2)], "{runs}");
    }
    let named = format!("#!/usr/bin/env {NEEDLE}3.12\n");
    let found = scan("tools/run", named.as_bytes());
    assert_eq!(found.kinds(), [Breach::Name(1), Breach::Shebang(1)]);
    for passes in [
        "#![forbid(unsafe_code)]",
        "#! [allow(dead_code)]",
        "#!/bin/sh",
        "#!/usr/bin/env bash",
        "#!/usr/bin/env node",
        "#!/usr/bin/env pyramid",
        "#!/usr/bin/env -S cargo +nightly -Zscript",
        "# !/usr/bin/pypy",
        "#!",
        "#!/usr/bin/env",
    ] {
        assert!(
            scan("docs/x.md", passes.as_bytes()).breaches.is_empty(),
            "{passes}"
        );
    }
}

/// The gate's own walk, end to end on planted trees. With no `.git` the tree is
/// walked whole, `target`, `.git` and `node_modules` aside, and the planted
/// name is refused. With a `.git`, only what `git ls-files` lists is read: the
/// staged plant is refused and the untracked one is never opened.
#[test]
fn a_planted_tree_is_refused_through_the_same_walk_the_gate_takes() {
    let planted = format!("ported from {NEEDLE}\n").into_bytes();
    let dir = scratch("walk");
    plant(&dir, "keep.md", &planted);
    plant(&dir, "sub/keep.rs", b"// clean\n");
    plant(
        &dir,
        "sub/target/kept.md",
        b"only the root's target is skipped\n",
    );
    plant(&dir, "target/debug/build.py", &planted);
    plant(&dir, "web/node_modules/pkg/index.py", &planted);
    plant(&dir, "sub/node_modules/x.py", &planted);
    assert_eq!(walk(&dir), ["keep.md", "sub/keep.rs", "sub/target/kept.md"]);
    let found = audit(&dir);
    assert!(
        found.how.starts_with("WALKED AS A DIRECTORY"),
        "{}",
        found.how
    );
    assert_eq!(found.read, 3);
    assert_eq!(found.refused.len(), 1, "{:?}", found.refused);
    assert!(
        found.refused[0].starts_with("keep.md: Name(1): "),
        "{:?}",
        found.refused
    );
    assert!(!names_it(found.refused[0].as_bytes()));

    let repo = scratch("git");
    if let Err(why) = git(&repo, &["--version"]) {
        println!("SKIPPING the git half: `git` cannot be run here ({why}).");
        return;
    }
    let init = git(&repo, &["init", "-q"]).expect("git ran a moment ago");
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    plant(&repo, "tracked.md", &planted);
    plant(
        &repo,
        "tools/run.py",
        b"clean, and still the language's own file\n",
    );
    plant(&repo, "untracked.md", &planted);
    let add = git(&repo, &["add", "tracked.md", "tools/run.py"]).expect("git ran");
    assert!(
        add.status.success(),
        "{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let found = audit(&repo);
    assert!(found.how.contains("git ls-files"), "{}", found.how);
    assert_eq!(found.read, 2);
    assert_eq!(
        found.refused,
        [
            "tools/run.py: Extension(\"py\"): tools/run.py",
            "tracked.md: Name(1): ported from [THE NAME]",
        ]
    );
    std::fs::remove_file(repo.join("tracked.md")).expect("the plant is removable");
    let found = audit(&repo);
    assert_eq!(found.absent, ["tracked.md"]);
    assert_eq!(found.read, 1);
}
