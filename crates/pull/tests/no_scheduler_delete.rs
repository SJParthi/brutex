//! No automatic deletion (R4-06 item 2, AUT-05): no module a scheduler can
//! reach removes a file, a folder, or renames one to anything but its own
//! sibling final name.
//!
//! # The closed list
//!
//! [`DECLARED`] is every module the round-3 plan names as scheduler-reachable,
//! existing or not. [`LISTED`] holds exactly the declared members that exist
//! at this head, each read by `include_str!`, so a listed file that is moved
//! or deleted fails the BUILD naming its path — never a silent skip
//! (`CLAUDE.md` §4). A member created later adds itself to [`LISTED`] in its
//! creating change (a shared line); until it does,
//! [`every_declared_module_that_exists_is_listed`] fails naming it.
//!
//! # What is refused
//!
//! In production code — comments, string and character literals, and every
//! top-level `#[cfg(test)]` item stripped — every site [`is_site`] names:
//! the identifiers of [`DELETES`] (std's and tokio's removes, the C
//! library's `unlink`, `unlinkat` and `rmdir`, `set_len`, `ftruncate`), any
//! call, turbofish, path or import of `rename` (its `(` may be on a later
//! line), `truncate(true)`, `File::create`, `libc::remove`, and `Command`,
//! unless the site is one of [`ALLOWED`], each a named, counted exception with its
//! reason. A rename that publishes a verified temporary under its sibling
//! final name is the one rename an append-only writer needs; the scan cannot
//! prove sibling-ness from text, so each such site is enumerated, and an
//! edit to the line fails here by name until it is justified again.
//!
//! # What it cannot see
//!
//! A deletion reached through a helper in a module that is NOT listed (the
//! list is the scheduler's own modules, not their whole call graph), and one
//! spelled through an alias the `use` line of which is itself refused here.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

/// Every module the plan declares scheduler-reachable (R4-06 item 2).
const DECLARED: [&str; 12] = [
    "crates/api/src/autopilot.rs",
    "crates/pull/src/upkeep.rs",
    "crates/api/src/watcher.rs",
    "crates/pull/src/gdfl_dropbox.rs",
    "crates/pull/src/tickstore_write.rs",
    "crates/api/src/scrub_schedule.rs",
    "crates/api/src/log_archive.rs",
    "crates/api/src/gdfl_dropwatch.rs",
    "crates/api/src/ingest_chain.rs",
    "crates/api/src/schedule.rs",
    "crates/api/src/keep_awake.rs",
    "crates/api/src/gap_worker.rs",
];

/// The declared members that exist at this head, read at compile time.
const LISTED: [(&str, &str); 3] = [
    (
        "crates/api/src/autopilot.rs",
        include_str!("../../api/src/autopilot.rs"),
    ),
    (
        "crates/pull/src/gdfl_dropbox.rs",
        include_str!("../src/gdfl_dropbox.rs"),
    ),
    (
        "crates/pull/src/tickstore_write.rs",
        include_str!("../src/tickstore_write.rs"),
    ),
];

/// The named exceptions: (module, the site's code line trimmed, how many
/// times it may appear, why).
const ALLOWED: [(&str, &str, usize, &str); 4] = [
    (
        "crates/api/src/autopilot.rs",
        "let mut file = std::fs::File::create(path)?;",
        1,
        "store_writable's probe creates its own dot-file (STORE_PROBE_PREFIX + \
         vendor) to learn whether the disk takes a write; File::create would \
         empty a file of that name, and no store file carries it",
    ),
    (
        "crates/api/src/autopilot.rs",
        "std::fs::remove_file(path)",
        1,
        "store_writable's probe removes the one dot-file it created a moment \
         before (STORE_PROBE_PREFIX + vendor), never a store file; making the \
         probe overwrite in place instead is R4-12's (autopilot.rs owner)",
    ),
    (
        "crates/pull/src/tickstore_write.rs",
        "std::fs::rename(temp, final_path).map_err(|why| Refusal::WriteFailed {",
        1,
        "publish renames the verified temporary to the day's final name in the \
         same folder, then fsyncs that folder",
    ),
    (
        "crates/pull/src/gdfl_dropbox.rs",
        "std::fs::rename(&part, &target)",
        1,
        "fetch_one promotes a verified `.part` to the day zip's name in the \
         part's own run folder (`target` is `part.parent()` joined with the \
         name), after refusing `name_taken` when that name exists; the staging \
         lock keeps any second press out (R4-07, D-4986)",
    ),
];

/// The identifiers that delete or cut a file wherever they appear: std's,
/// tokio's and the C library's (`libc::unlink`, `unlinkat`, `rmdir`), and
/// the two that cut a file to a length (`set_len`, `ftruncate`).
const DELETES: [&str; 8] = [
    "remove_file",
    "remove_dir",
    "remove_dir_all",
    "unlink",
    "unlinkat",
    "rmdir",
    "set_len",
    "ftruncate",
];

/// Whether the identifier `word` at byte `at` of `code` is a site: one of
/// [`DELETES`]; a `rename` that is called (its `(` on this line or a later
/// one), turbofished, reached as a path segment (`fs::rename`) or imported;
/// a `truncate(true)` (an open that empties the file) or `libc::truncate`;
/// `File::create` (which empties an existing file); the C library's
/// `remove`; or `Command`, since a spawned process can delete anything (`rm`,
/// `unlink`, a shell) and its program name is a string this scan blanks.
fn is_site(code: &str, at: usize, word: &str) -> bool {
    let after = code[at + word.len()..].trim_start();
    let before = &code[..at];
    let line_start = before.rfind('\n').map_or(0, |n| n + 1);
    match word {
        w if DELETES.contains(&w) => true,
        "rename" => {
            after.starts_with('(')
                || after.starts_with("::")
                || before.ends_with("::")
                || code[line_start..].trim_start().starts_with("use ")
        }
        "truncate" => {
            before.ends_with("libc::")
                || after
                    .strip_prefix('(')
                    .is_some_and(|args| args.trim_start().starts_with("true"))
        }
        "create" => before.ends_with("File::"),
        "remove" => before.ends_with("libc::"),
        "Command" => true,
        _ => false,
    }
}

/// `source` with comments, string and character literals blanked (newlines
/// kept, so line numbers hold).
fn code_only(source: &str) -> String {
    let chars: Vec<char> = source.chars().collect();
    let mut out = String::with_capacity(source.len());
    let mut i = 0;
    let ident = |c: char| c.is_alphanumeric() || c == '_';
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c == '/' && next == Some('/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0;
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    i += 2;
                } else if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    if chars[i] == '\n' {
                        out.push('\n');
                    }
                    i += 1;
                }
            }
        } else if (c == 'r' || (c == 'b' && next == Some('r')))
            && (i == 0 || !ident(chars[i - 1]))
            && {
                let mut j = i + if c == 'b' { 2 } else { 1 };
                while chars.get(j) == Some(&'#') {
                    j += 1;
                }
                chars.get(j) == Some(&'"')
            }
        {
            let mut j = i + if c == 'b' { 2 } else { 1 };
            let mut hashes = 0;
            while chars[j] == '#' {
                hashes += 1;
                j += 1;
            }
            j += 1;
            out.push_str("\"\"");
            loop {
                if j >= chars.len() {
                    break;
                }
                if chars[j] == '"' && (0..hashes).all(|h| chars.get(j + 1 + h) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                if chars[j] == '\n' {
                    out.push('\n');
                }
                j += 1;
            }
            i = j;
        } else if c == '"' {
            out.push_str("\"\"");
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                }
                if chars.get(i) == Some(&'\n') {
                    out.push('\n');
                }
                i += 1;
            }
            i += 1;
        } else if c == '\'' && next == Some('\\') {
            i += 2;
            while i < chars.len() && chars[i] != '\'' {
                i += 1;
            }
            out.push_str("' '");
            i += 1;
        } else if c == '\'' && chars.get(i + 2) == Some(&'\'') {
            out.push_str("' '");
            i += 3;
        } else {
            out.push(c);
            i += 1;
        }
    }
    out
}

/// `code` with every top-level `#[cfg(test)]` item removed: its attributes,
/// then a `mod x;` line or a body up to the next column-0 `}` (lines kept as
/// blanks, so numbers hold).
fn without_tests(code: &str) -> String {
    let mut out = Vec::new();
    // While skipping: the brace depth, and whether the item opened a body.
    let mut skipping: Option<(usize, bool)> = None;
    for line in code.lines() {
        let Some((mut depth, mut opened)) = skipping else {
            if line.trim_end() == "#[cfg(test)]" {
                skipping = Some((0, false));
                out.push("");
            } else {
                out.push(line);
            }
            continue;
        };
        out.push("");
        // Braces and `;` are counted in code with literals blanked, so a
        // brace inside a string cannot end the item early.
        let mut ended = false;
        for c in line.chars() {
            match c {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => {
                    depth = depth.saturating_sub(1);
                    ended |= opened && depth == 0;
                }
                ';' => ended |= !opened && depth == 0,
                _ => {}
            }
        }
        skipping = (!ended).then_some((depth, opened));
    }
    out.join("\n")
}

/// The identifier at each position of `line` (ASCII identifiers).
fn identifiers(line: &str) -> Vec<(usize, &str)> {
    let bytes = line.as_bytes();
    let word = |b: u8| b.is_ascii_alphanumeric() || b == b'_';
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if word(bytes[i]) {
            let start = i;
            while i < bytes.len() && word(bytes[i]) {
                i += 1;
            }
            out.push((start, &line[start..i]));
        } else {
            i += 1;
        }
    }
    out
}

/// Every deleting, renaming, truncating or spawning site ([`is_site`]) of
/// `module`'s production code that no [`ALLOWED`] row covers, as
/// `module:line: code`. The whole text is read at once, so a call whose `(`
/// is on the next line is still a call; a line with two sites is one site.
fn refused_sites(module: &str, source: &str, allowed: &[(&str, &str, usize, &str)]) -> Vec<String> {
    let code = without_tests(&code_only(source));
    let lines: Vec<&str> = code.lines().collect();
    let mut hit_lines: Vec<usize> = identifiers(&code)
        .into_iter()
        .filter(|(at, word)| is_site(&code, *at, word))
        .map(|(at, _)| code[..at].matches('\n').count())
        .collect();
    hit_lines.dedup();
    let mut used = vec![0_usize; allowed.len()];
    let mut out = Vec::new();
    for number in hit_lines {
        let trimmed = lines.get(number).map_or("", |line| line.trim());
        match allowed
            .iter()
            .position(|(m, site, _, _)| *m == module && *site == trimmed)
        {
            Some(row) if used[row] < allowed[row].2 => used[row] += 1,
            _ => out.push(format!("{module}:{}: {trimmed}", number + 1)),
        }
    }
    out
}

/// The listed modules' sources, or the first listed name `read` cannot
/// supply — by name.
fn listed_sources<'a>(
    names: &[&'a str],
    read: impl Fn(&str) -> Option<String>,
) -> Result<Vec<(&'a str, String)>, String> {
    names
        .iter()
        .map(|name| {
            read(name)
                .map(|text| (*name, text))
                .ok_or_else(|| format!("listed scheduler-reachable module {name} is missing"))
        })
        .collect()
}

fn workspace() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn no_scheduler_path_deletes_a_file() {
    let names: Vec<&str> = LISTED.iter().map(|(name, _)| *name).collect();
    let sources = listed_sources(&names, |name| {
        LISTED
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, text)| (*text).to_owned())
    })
    .unwrap();
    let refused: Vec<String> = sources
        .iter()
        .flat_map(|(name, text)| refused_sites(name, text, &ALLOWED))
        .collect();
    assert!(
        refused.is_empty(),
        "a scheduler path deletes or renames: {refused:#?}"
    );
    // EVERY EXCEPTION IS STILL SPENT: a row whose line is gone is a stale
    // allowance a later deletion could hide behind.
    for (module, site, times, why) in ALLOWED {
        let text = LISTED.iter().find(|(n, _)| *n == module).unwrap().1;
        let code = without_tests(&code_only(text));
        let found = code.lines().filter(|l| l.trim() == site).count();
        assert_eq!(
            found, times,
            "{module}: allowed site {site:?} ({why}) appears {found} times"
        );
    }
}

#[test]
fn a_fixture_deletion_is_refused_and_a_comment_or_string_is_not() {
    let fixture = r##"
//! remove_file in a doc comment is allowed.
fn tidy(path: &Path) {
    // std::fs::remove_file(path) in a comment is allowed.
    let words = "remove_dir_all(\"/\") in a string is allowed";
    let raw = r#"rename(a, b) in a raw string"#;
    let c = '"';
    /* remove_dir in a block comment */
    std::fs::remove_file(path).ok();
}
fn tidy_dir(path: &Path) {
    let _ = tokio::fs::remove_dir_all(path);
    std::fs::rename(path, "/elsewhere").unwrap();
    use std::fs::remove_dir as gone;
}
#[cfg(test)]
mod tests {
    fn cleanup() { let _ = std::fs::remove_dir_all("/tmp/x"); }
    const LONG: &str = "a string over two lines
}; whose second line opens at column 0";
    fn later() { let _ = std::fs::remove_dir_all("/tmp/y"); }
}
#[cfg(test)]
#[path = "x_tests.rs"]
mod x_tests;
fn after_tests() { std::fs::remove_file("late").ok(); }
"##;
    let refused = refused_sites("fixture.rs", fixture, &[]);
    assert_eq!(refused.len(), 5, "{refused:#?}");
    for (at, needle) in [
        (0, "std::fs::remove_file(path).ok();"),
        (1, "tokio::fs::remove_dir_all"),
        (2, "std::fs::rename(path"),
        (3, "use std::fs::remove_dir as gone;"),
        (4, "std::fs::remove_file(\"\").ok();"),
    ] {
        assert!(refused[at].contains(needle), "{at}: {}", refused[at]);
    }
    // AN ALLOWANCE COVERS ITS COUNT, NOT ONE MORE.
    let twice = "fn a(p: &Path) {\n    std::fs::remove_file(p)\n}\nfn b(p: &Path) {\n    std::fs::remove_file(p)\n}\n";
    let one = [("fixture.rs", "std::fs::remove_file(p)", 1, "the first only")];
    assert_eq!(refused_sites("fixture.rs", twice, &one).len(), 1);
    assert_eq!(
        refused_sites("other.rs", twice, &one).len(),
        2,
        "an allowance is per module"
    );
    // `rename` that is not a call, and an identifier that only contains the
    // word, are not sites.
    let names = "fn f() { let renamed = 1; let rename_count = 2; let x = s.rename_all; }\n";
    assert!(refused_sites("fixture.rs", names, &[]).is_empty());
}

/// Each evasion the second agent tried on a copy of `autopilot.rs` (16 Oct,
/// `verify-R4-06-r1.json` attacks) is a site, by line; the near misses that
/// cut nothing are not.
#[test]
fn every_evasion_the_second_agent_tried_is_refused() {
    let fixture = "fn a(p: &Path) {
    let _ = std::process::Command::new(\"rm\").arg(\"-rf\").arg(p).status();
    unsafe { libc::unlink(c.as_ptr()) };
    unsafe { libc::rmdir(c.as_ptr()) };
    unsafe { libc::remove(c.as_ptr()) };
    std::fs::rename::<&str, &str>(\"a\", \"b\").ok();
    std::fs::rename
        (p, \"b\").ok();
    let f = std::fs::OpenOptions::new().write(true).truncate(true).open(p);
    let g = std::fs::OpenOptions::new().write(true).truncate(
        true,
    ).open(p);
    file.set_len(0).ok();
    let h = std::fs::File::create(p);
    use std::fs::rename as mv;
    let _ = nix::unistd::unlink(p);
}
fn fine(p: &Path) {
    failures.truncate(MAX_FAILURES);
    let t = std::fs::File::create_new(p);
    map.remove(&key);
    let renamed = rename_count;
}
";
    let refused = refused_sites("fixture.rs", fixture, &[]);
    let lines: Vec<usize> = refused
        .iter()
        .map(|site| site.split(':').nth(1).unwrap().parse().unwrap())
        .collect();
    assert_eq!(
        lines,
        [2, 3, 4, 5, 6, 7, 9, 10, 13, 14, 15, 16],
        "{refused:#?}"
    );
    assert!(refused[5].contains("std::fs::rename"), "{}", refused[5]);
}

#[test]
fn a_missing_listed_module_fails_by_name() {
    let names = ["crates/api/src/autopilot.rs", "crates/api/src/watcher.rs"];
    let read = |name: &str| (name == "crates/api/src/autopilot.rs").then(String::new);
    let why = listed_sources(&names, read).unwrap_err();
    assert_eq!(
        why,
        "listed scheduler-reachable module crates/api/src/watcher.rs is missing"
    );
}

#[test]
fn every_declared_module_that_exists_is_listed() {
    let root = workspace();
    for name in DECLARED {
        let exists = root.join(name).is_file();
        let listed = LISTED.iter().any(|(n, _)| *n == name);
        assert_eq!(
            exists, listed,
            "{name}: exists={exists} listed={listed}; a scheduler-reachable module \
             adds itself to LISTED in the change that creates it"
        );
    }
    for (name, _) in LISTED {
        assert!(
            DECLARED.contains(&name),
            "{name} is listed but not declared"
        );
    }
}
