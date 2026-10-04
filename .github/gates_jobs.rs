//! The decisions of the CI gates that run after the language-purity job: the
//! build job's probe and gates 13a, 13b, 13c, 6c, 5 and 6d, the coverage
//! job's probe and gate 20, gate 18's mutation plan, gate 8's bench count and
//! the browser job's gates W1, W3, W4, W5 and W6 (D-2315).
//!
//! Those steps used to decide with inline awk, sed and grep pipelines: a
//! second language in a tracked file outside `web/`, which CLAUDE.md section
//! 2 forbids. The rules live here now, std only, built by rustc in each job
//! that runs one, and pinned by the tests below, which that job runs first.
//!
//! WHAT STAYS IN THE WORKFLOW, AND WHY. Gate 0's `spawns` scan (D-2344) reads
//! every `.github/*.rs` and refuses a tool that starts any program but `git`.
//! So `cargo`, `rustc`, `rustfmt`, `clippy-driver`, `npm` and `node` are still
//! started by the workflow, each as one straight-line command writing to a
//! file, and this tool reads the file and renders the verdict. Every reader
//! fails closed: a missing or empty input is a refusal, never an empty answer.
//!
//! Every finding is printed to stdout, as the shell's `echo` printed it; a
//! refusal's reason goes to stderr, so it is seen even where stdout is
//! captured (`tools`), and the tool exits 1.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};

type Verdict = Result<(), String>;

// ------------------------------------------------------------- helpers --

fn read(p: &str) -> Result<String, String> {
    std::fs::read(p)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .map_err(|e| format!("{p}: {e}"))
}

/// The lines of `s` as awk, sed and grep split them: on `\n` only, so a `\r`
/// stays part of its line and a pattern anchored at `$` does not match it.
fn records(s: &str) -> Vec<&str> {
    let mut v: Vec<&str> = s.split('\n').collect();
    if v.last() == Some(&"") {
        v.pop();
    }
    v
}

/// `git` with `args`, its stdout, or why it failed.
fn git(args: &[&str]) -> Result<Vec<u8>, String> {
    let out = Command::new("git")
        .args(args)
        .stderr(Stdio::inherit())
        .output()
        .map_err(|e| format!("git {}: {e}", args.join(" ")))?;
    if out.status.success() {
        Ok(out.stdout)
    } else {
        Err(format!("git {} failed: {}", args.join(" "), out.status))
    }
}

/// Does `git` with `args`, its output discarded, exit 0?
fn git_ok(args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// The tracked files matching `pathspec`, one per entry.
fn tracked(pathspec: &str) -> Result<Vec<String>, String> {
    let raw = git(&["ls-files", "-z", "--", pathspec])?;
    Ok(String::from_utf8_lossy(&raw)
        .split('\0')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect())
}

/// Append `line` to the file `$var` names (`$GITHUB_OUTPUT`). An unset
/// variable is a refusal, as `set -u` made it.
fn append_to_env_file(var: &str, line: &str) -> Verdict {
    let path = std::env::var(var).map_err(|_| format!("{var} is not set"))?;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&path)
        .map_err(|e| format!("{path}: {e}"))?;
    writeln!(f, "{line}").map_err(|e| format!("{path}: {e}"))
}

/// Lines holding at least one character, as `grep -c .` counts them.
fn nonempty_lines(s: &str) -> usize {
    records(s).iter().filter(|l| !l.is_empty()).count()
}

// --------------------------------------------- probe: is there a crate? --

/// The build job's probe. Writes `has_crates` to `$GITHUB_OUTPUT`.
fn probe() -> Verdict {
    let n = tracked("crates/*/Cargo.toml")?.len();
    if n == 0 {
        append_to_env_file("GITHUB_OUTPUT", "has_crates=false")?;
        println!(
            "::warning title=No crate exists yet::The toolchain gates (fmt, build, clippy, deny, test) compiled NOTHING on this commit because the workspace has no members. This is not a passing build — it is an empty one. It stops being possible as soon as crates/core is merged."
        );
    } else {
        append_to_env_file("GITHUB_OUTPUT", "has_crates=true")?;
        println!("{n} crate manifest(s) tracked — the toolchain gates apply.");
    }
    Ok(())
}

/// The coverage job's probe: exit 0 (the step then exits 0) ONLY when no
/// crate manifest is tracked, so 100% is vacuous; any other outcome,
/// including a failure to ask git, exits non-zero and the step goes on to
/// measure. A broken probe can therefore never skip the measurement.
fn no_crates() -> ExitCode {
    match tracked("crates/*/Cargo.toml") {
        Ok(v) if v.is_empty() => {
            println!(
                "::warning title=Coverage measured nothing::No crate exists yet, so 100% coverage is vacuously true. See docs/06-limits.md section 7."
            );
            ExitCode::SUCCESS
        }
        Ok(v) => {
            println!("{} crate manifest(s) tracked — measuring.", v.len());
            ExitCode::FAILURE
        }
        Err(e) => {
            println!("{e}; measuring regardless.");
            ExitCode::FAILURE
        }
    }
}

// ------------------------------------------------------------ gate 13a --

/// The browser bindings gate 13a requires to be unreachable on the host.
const BINDINGS: [&str; 3] = ["wasm-bindgen", "js-sys", "web-sys"];

/// Gate 13a. `trees` holds, per binding, what `cargo tree -i <binding>
/// --prefix none` printed for the host, or None when that file is missing.
/// `cargo tree` prints no inverse tree when the package is unreachable.
fn gate_13a(trees: &[(&str, Option<String>)]) -> Verdict {
    for (binding, tree) in trees {
        let Some(tree) = tree else {
            return Err(format!(
                "GATE 13A READ NO INVERSE TREE FOR {binding}: the cargo tree line that writes it is gone."
            ));
        };
        // `$( ... )` strips trailing newlines and nothing else.
        let host = tree.trim_end_matches('\n');
        if !host.is_empty() {
            return Err(format!(
                "FORBIDDEN HOST-REACHABLE BINDING: {binding}\n{host}\nLock-only target-gated entries are allowed; a host path is not."
            ));
        }
    }
    println!("OK — wasm-bindgen, js-sys and web-sys are unreachable on the host graph.");
    Ok(())
}

fn gate_13a_cmd(dir: &str) -> Verdict {
    let trees: Vec<(&str, Option<String>)> = BINDINGS
        .iter()
        .map(|b| {
            (
                *b,
                std::fs::read_to_string(format!("{dir}/13a-{b}.txt")).ok(),
            )
        })
        .collect();
    gate_13a(&trees)
}

// ------------------------------------------------------------ gate 13b --

/// Does a `cargo tree --prefix none --format '{p}'` line name a package in
/// the family of `name`: the name itself, or the name, `-` or `_`, and one or
/// more of `[A-Za-z0-9_-]`, followed by ` v`? This is the old extended
/// pattern `^(name|name[-_][A-Za-z0-9_-]+) v`, matched without a regex.
fn in_family(line: &str, name: &str) -> bool {
    let Some(rest) = line.strip_prefix(name) else {
        return false;
    };
    if rest.starts_with(" v") {
        return true;
    }
    let Some(tail) = rest.strip_prefix(['-', '_']) else {
        return false;
    };
    let run = tail
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
        .count();
    run > 0 && tail[run..].starts_with(" v")
}

fn banned_line(line: &str, names: &[String]) -> bool {
    names.iter().any(|n| in_family(line, n))
}

/// The matcher proves itself on a fixture holding `ring` and `cc` among
/// near misses, so a list that matches nothing is a red build, not a pass.
fn gate_13b_self_test(names: &[String]) -> Verdict {
    let fixture = [
        "serde v1.0.0",
        "ring v0.17.14",
        "rustls v0.23.0",
        "cc v1.2.0",
        "ccache-sys v1.0.0",
        "luau v1.0.0",
    ];
    let caught = fixture.iter().filter(|l| banned_line(l, names)).count();
    if caught == 2 {
        Ok(())
    } else {
        Err(format!(
            "GATE 13B'S MATCHER IS BROKEN: it caught {caught} of the fixture's 2 banned lines."
        ))
    }
}

/// For each line of a `--prefix depth` tree naming `pkg`, the chain from it
/// up to its workspace root: the "why is it built" answer.
fn why_built(depth_tree: &str, pkg: &str) -> Vec<String> {
    let parsed: Vec<(usize, &str)> = records(depth_tree)
        .into_iter()
        .filter_map(|l| {
            let digits = l.bytes().take_while(u8::is_ascii_digit).count();
            let d = l[..digits].parse().ok()?;
            Some((d, &l[digits..]))
        })
        .collect();
    let mut out = Vec::new();
    for (i, (d, text)) in parsed.iter().enumerate() {
        if text.split(' ').next() != Some(pkg) {
            continue;
        }
        let mut chain = vec![*text];
        let mut want = *d;
        for (pd, ptext) in parsed[..i].iter().rev() {
            if want == 0 {
                break;
            }
            if *pd == want - 1 {
                chain.push(ptext);
                want -= 1;
            }
        }
        out.push(chain.join("  <-  "));
        if out.len() == 20 {
            break;
        }
    }
    out
}

/// Gate 13b. `tree` is `cargo tree --workspace -e normal,build,dev --prefix
/// none --format '{p}'`; `depth_tree` the same with `--prefix depth`, read
/// only to say why a refused package is built.
fn gate_13b(tree: &str, depth_tree: &str, names: &[String]) -> Verdict {
    gate_13b_self_test(names)?;
    let packages: BTreeSet<&str> = records(tree).into_iter().collect();
    if packages.iter().all(|l| l.is_empty()) {
        return Err("GATE 13B READ AN EMPTY BUILD GRAPH.".to_owned());
    }
    let hits: Vec<&str> = packages
        .iter()
        .copied()
        .filter(|l| banned_line(l, names))
        .collect();
    let count = packages.iter().filter(|l| !l.is_empty()).count();
    println!("host build graph: {count} package(s)");
    if hits.is_empty() {
        println!("OK — no banned runtime or native crate is built for the host.");
        return Ok(());
    }
    let mut msg = format!(
        "BANNED PACKAGE IN THE HOST BUILD GRAPH:\n{}\n",
        hits.join("\n")
    );
    for h in &hits {
        let p = h.split_whitespace().next().unwrap_or(h);
        msg.push_str(&format!("--- why {p} is built:\n"));
        for chain in why_built(depth_tree, p) {
            msg.push_str(&format!("  {chain}\n"));
        }
    }
    msg.push_str(
        "\nCLAUDE.md section 2 forbids a vendored binding to another language\nwithout exception. Lock-only optional entries are legal; a built\none is not.",
    );
    Err(msg)
}

// ------------------------------------------------------------ gate 13c --

const BUILD_SCRIPT: &str = "\"reason\":\"build-script-executed\"";

/// Every non-overlapping `"key":<open>...<close>` in `line`, as `grep -o
/// '"key":\[[^]]*\]'` (or the string form) prints it.
fn matches_of(line: &str, key: &str, open: char, close: char) -> Vec<String> {
    let head = format!("\"{key}\":{open}");
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(at) = line[from..].find(&head) {
        let start = from + at;
        let body = start + head.len();
        if let Some(end) = line[body..].find(close) {
            let stop = body + end + close.len_utf8();
            out.push(line[start..stop].to_owned());
            from = stop;
        } else {
            break;
        }
    }
    out
}

/// The non-empty `linked_libs` arrays of every build-script message.
fn linked_libs(msgs: &str) -> Vec<String> {
    records(msgs)
        .into_iter()
        .filter(|l| l.contains(BUILD_SCRIPT))
        .flat_map(|l| matches_of(l, "linked_libs", '[', ']'))
        .filter(|m| !m.contains("\"linked_libs\":[]"))
        .collect()
}

/// The `out_dir` of every build-script message.
fn out_dirs(msgs: &str) -> Vec<String> {
    records(msgs)
        .into_iter()
        .filter(|l| l.contains(BUILD_SCRIPT))
        .flat_map(|l| matches_of(l, "out_dir", '"', '"'))
        .map(|m| m["\"out_dir\":\"".len()..m.len() - 1].to_owned())
        .collect()
}

const NATIVE: [&str; 7] = [".o", ".obj", ".a", ".lib", ".so", ".dylib", ".dll"];

/// Every regular file under `dir` whose name ends in a native object or
/// archive suffix, symlinks not followed, as `find -type f -name` lists them.
fn native_objects(dir: &Path, out: &mut Vec<String>) -> Verdict {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in entries {
        let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
        let p = e.path();
        let ty = e.file_type().map_err(|e| format!("{}: {e}", p.display()))?;
        if ty.is_dir() {
            native_objects(&p, out)?;
        } else if ty.is_file() {
            let name = e.file_name().to_string_lossy().into_owned();
            if NATIVE.iter().any(|s| name.ends_with(s)) {
                out.push(p.display().to_string());
            }
        }
    }
    Ok(())
}

/// The extraction proves itself on two fixture messages, one clean and one
/// that links `ring_core`, so a pattern that matched nothing is a red build.
fn gate_13c_self_test() -> Verdict {
    let fx = concat!(
        "{\"reason\":\"build-script-executed\",\"package_id\":\"a\",\"linked_libs\":[],\"out_dir\":\"/x\"}\n",
        "{\"reason\":\"build-script-executed\",\"package_id\":\"ring\",\"linked_libs\":[\"static=ring_core\"],\"out_dir\":\"/y\"}\n",
    );
    if linked_libs(fx).len() == 1 && out_dirs(fx) == ["/x", "/y"] {
        Ok(())
    } else {
        Err("GATE 13C'S EXTRACTION IS BROKEN on its own fixture.".to_owned())
    }
}

/// Gate 13c over the messages of `cargo build --message-format=json`.
fn gate_13c(msgs: &str) -> Verdict {
    gate_13c_self_test()?;
    let n = records(msgs)
        .iter()
        .filter(|l| l.contains(BUILD_SCRIPT))
        .count();
    if n == 0 {
        return Err(
            "GATE 13C SAW NO BUILD SCRIPT RUN. libc has one; the read is broken.".to_owned(),
        );
    }
    let mut bad = false;
    let libs = linked_libs(msgs);
    if !libs.is_empty() {
        println!(
            "A BUILD SCRIPT LINKS A NATIVE LIBRARY:\n{}",
            libs.join("\n")
        );
        bad = true;
    }
    let mut objects = Vec::new();
    for dir in out_dirs(msgs) {
        let p = Path::new(&dir);
        if p.is_dir() {
            native_objects(p, &mut objects)?;
        }
    }
    if !objects.is_empty() {
        println!("A BUILD SCRIPT LEFT NATIVE OBJECTS IN ITS OUTPUT DIRECTORY:");
        for o in &objects {
            println!("  {o}");
        }
        bad = true;
    }
    println!("read {n} build-script run(s)");
    if bad {
        return Err("CLAUDE.md section 2 forbids a vendored binding to another language\nand native code compiled through a build script, without exception.".to_owned());
    }
    println!("OK — no build script linked or compiled native code.");
    Ok(())
}

// ------------------------------------------------------------- gate 6c --

/// The tracked `.github/*.rs` tools, one per line, for rustfmt and
/// clippy-driver to read. None is a refusal.
fn tools() -> Verdict {
    let tools = tracked(".github/*.rs")?;
    if tools.is_empty() {
        return Err("GATE 6c READ NO TOOL.".to_owned());
    }
    for t in tools {
        println!("{t}");
    }
    Ok(())
}

// -------------------------------------------------------------- gate 5 --

/// How many documented unsafe exceptions are allowed before each needs its
/// own decision entry.
const UNSAFE_CEILING: usize = 3;

/// Every `allow` or `expect` attribute opening on `unsafe_code` and closing
/// or continuing right after it, in `src`: the bare form, the form with a
/// `reason`, and the `expect` form alike (D-1610), one count per occurrence.
fn unsafe_exceptions(src: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    for (n, line) in records(src).into_iter().enumerate() {
        let needle = "unsafe_code";
        let mut from = 0;
        while let Some(at) = line[from..].find(needle) {
            let start = from + at;
            let end = start + needle.len();
            let before = &line[..start];
            let opened = before.ends_with("allow(") || before.ends_with("expect(");
            if opened && line[end..].starts_with([',', ')']) {
                out.push((n + 1, line));
            }
            from = end;
        }
    }
    out
}

fn gate_5(files: &[(String, String)]) -> Verdict {
    let mut hits = Vec::new();
    for (f, src) in files {
        for (n, line) in unsafe_exceptions(src) {
            hits.push(format!("{f}:{n}:{line}"));
        }
    }
    println!("documented unsafe exceptions: {}", hits.len());
    if hits.len() > UNSAFE_CEILING {
        // Counted per occurrence, listed per line, as `grep -n` lists them.
        hits.dedup();
        return Err(format!(
            "More than {UNSAFE_CEILING} unsafe exceptions. Each needs a decision entry.\n{}",
            hits.join("\n")
        ));
    }
    Ok(())
}

fn gate_5_cmd() -> Verdict {
    let mut files = Vec::new();
    for f in tracked("*.rs")? {
        let src = read(&f)?;
        files.push((f, src));
    }
    gate_5(&files)
}

// ------------------------------------------------------------- gate 6d --

/// The libraries the native tests under `web/` link, by crate name.
const WEB_EXTERNS: [&str; 9] = [
    "serde",
    "serde_json",
    "axum",
    "tower",
    "tokio",
    "api",
    "cli",
    "brutex_core",
    "vocab",
];

/// The distinct `.rlib` files cargo reported for the library crate `name`,
/// read exactly as the old grep chain did: a `compiler-artifact` line, from
/// `"crate_types":["lib"],"name":"<name>"` on, its first `"filenames":[` entry
/// that ends `.rlib`.
fn rlibs(art: &str, name: &str) -> Vec<String> {
    let anchor = format!("\"crate_types\":[\"lib\"],\"name\":\"{name}\"");
    let head = "\"filenames\":[\"";
    let mut out = BTreeSet::new();
    for line in records(art) {
        if !line.contains("\"reason\":\"compiler-artifact\"") {
            continue;
        }
        let Some(at) = line.find(&anchor) else {
            continue;
        };
        let rest = &line[at..];
        let mut from = 0;
        while let Some(h) = rest[from..].find(head) {
            let body = from + h + head.len();
            let Some(end) = rest[body..].find('"') else {
                break;
            };
            let entry = &rest[body..body + end];
            if entry.ends_with(".rlib") {
                out.insert(entry.to_owned());
                from = body + end + 1;
            } else {
                from = from + h + 1;
            }
        }
    }
    out.into_iter().collect()
}

/// SHA-256 (FIPS 180-4), lowercase hex.
fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    let bits = (data.len() as u64).wrapping_mul(8);
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bits.to_be_bytes());
    for block in msg.chunks(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh] = h;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (x, y) in h.iter_mut().zip([a, b, c, d, e, f, g, hh]) {
            *x = x.wrapping_add(y);
        }
    }
    h.iter().map(|x| format!("{x:08x}")).collect()
}

/// The `--extern` arguments for rustc, one argument per line (rustc's
/// `@file` form), or the crate that has no single rlib.
fn web_externs(art: &str) -> Result<(String, String), String> {
    let mut args = String::new();
    let mut api = String::new();
    for n in WEB_EXTERNS {
        let found = rlibs(art, n);
        let [p] = found.as_slice() else {
            return Err(format!(
                "GATE 6d: no single rlib for {n}: {}",
                found.join("\n")
            ));
        };
        args.push_str(&format!("--extern\n{n}={p}\n"));
        if n == "api" {
            api.clone_from(p);
        }
    }
    Ok((args, api))
}

/// Gate 6d's preparation: `<dir>/externs` for rustc's `@file`, and
/// `<dir>/sha.txt` holding the api rlib's, the inspector source's and the
/// viewer source's SHA-256, space-separated, for the shell to `read`.
fn gate_6d(art_path: &str, dir: &str) -> Verdict {
    let art = read(art_path)?;
    let (externs, api) = web_externs(&art)?;
    let sha = |p: &str| {
        std::fs::read(p)
            .map(|b| sha256_hex(&b))
            .map_err(|e| format!("{p}: {e}"))
    };
    let line = format!(
        "{} {} {}\n",
        sha(&api)?,
        sha("web/sweep-readiness/main-inspector.rs")?,
        sha("web/saved-backtest/viewer.rs")?
    );
    std::fs::write(format!("{dir}/externs"), externs).map_err(|e| format!("{dir}: {e}"))?;
    std::fs::write(format!("{dir}/sha.txt"), line).map_err(|e| format!("{dir}: {e}"))?;
    println!(
        "linking the native web tests against {} libraries",
        WEB_EXTERNS.len()
    );
    Ok(())
}

// ------------------------------------------------------------- gate 20 --

/// Does a `cargo llvm-cov report --text` line open a file: an absolute path
/// at column zero ending `.rs:`?
fn is_file_header(l: &str) -> bool {
    l.starts_with('/') && l.ends_with(".rs:")
}

/// Does `l` carry `telemetry/src/<[a-z_]+>.rs:` anywhere?
fn names_telemetry_file(l: &str) -> bool {
    let head = "telemetry/src/";
    l.match_indices(head).any(|(at, _)| {
        let rest = &l[at + head.len()..];
        let run = rest
            .bytes()
            .take_while(|b| b.is_ascii_lowercase() || *b == b'_')
            .count();
        run > 0 && rest[run..].starts_with(".rs:")
    })
}

/// The execution count of a counted line, `^ *[0-9]+\| *[0-9]+\|`.
fn exec_count(l: &str) -> Option<&str> {
    let l = l.trim_start_matches(' ');
    let n = l.bytes().take_while(u8::is_ascii_digit).count();
    let l = l[n..].strip_prefix('|').filter(|_| n > 0)?;
    let l = l.trim_start_matches(' ');
    let c = l.bytes().take_while(u8::is_ascii_digit).count();
    l[c..].starts_with('|').then(|| &l[..c]).filter(|_| c > 0)
}

/// The file a telemetry header names: everything after the last
/// `/crates/telemetry/src/`, with its first `:` removed.
fn telemetry_name(l: &str) -> String {
    let marker = "/crates/telemetry/src/";
    let s = l.rfind(marker).map_or(l, |at| &l[at + marker.len()..]);
    s.replacen(':', "", 1)
}

/// Counted lines, and of them uncovered, in the telemetry crate's files.
/// `f` is cleared by ANY file header, not only a telemetry one (D-0167).
fn telemetry_totals(cov: &str) -> (usize, usize) {
    let (mut f, mut t, mut z) = (false, 0, 0);
    for l in records(cov) {
        if is_file_header(l) {
            f = false;
        }
        if names_telemetry_file(l) {
            f = true;
            continue;
        }
        if f && let Some(c) = exec_count(l) {
            t += 1;
            if c == "0" {
                z += 1;
            }
        }
    }
    (t, z)
}

/// `file count` per telemetry file with an uncovered line, sorted.
fn telemetry_uncovered(cov: &str) -> Vec<String> {
    let mut f: Option<String> = None;
    let mut n: std::collections::BTreeMap<String, usize> = Default::default();
    for l in records(cov) {
        if is_file_header(l) {
            f = None;
        }
        if names_telemetry_file(l) {
            f = Some(telemetry_name(l));
            continue;
        }
        if let Some(name) = &f
            && exec_count(l) == Some("0")
        {
            *n.entry(name.clone()).or_default() += 1;
        }
    }
    let mut v: Vec<String> = n.into_iter().map(|(k, c)| format!("{k} {c}")).collect();
    v.sort();
    v
}

/// The declared `file count` pairs, sorted. An odd word count is a refusal.
fn declared(words: &[String]) -> Result<Vec<String>, String> {
    if !words.len().is_multiple_of(2) {
        return Err(format!(
            "GATE 20: the declared list has {} words; it must be `file count` pairs.",
            words.len()
        ));
    }
    let mut v: Vec<String> = words
        .chunks(2)
        .map(|p| format!("{} {}", p[0], p[1]))
        .collect();
    v.sort();
    Ok(v)
}

fn gate_20(cov: &str, words: &[String]) -> Verdict {
    let want = declared(words)?;
    let (total, zero) = telemetry_totals(cov);
    if total < 500 || zero * 2 > total {
        return Err(format!(
            "NO USABLE COVERAGE PROFILE — {zero} of {total} telemetry lines\nread as uncovered. A real profile leaves about 19. The measure\nstep did not produce usable data, which usually means the\nworkspace failed to BUILD or the target directory was cleared\nunderneath it. This gate is reporting THAT, not a coverage\nregression. Measured: a stale profile read 13.96% where a\nreal one read 96.80%."
        ));
    }
    let actual = telemetry_uncovered(cov);
    println!("declared:");
    for w in &want {
        println!("  {w}");
    }
    println!("measured:");
    for a in &actual {
        println!("  {a}");
    }
    if want == actual {
        println!("OK — every uncovered line in crates/telemetry is declared.");
        return Ok(());
    }
    let mut msg = String::from("\nTHE LOGGER'S UNCOVERED SET IS NOT THE DECLARED ONE.\n\n");
    for w in want.iter().filter(|w| !actual.contains(w)) {
        msg.push_str(&format!("< {w}\n"));
    }
    for a in actual.iter().filter(|a| !want.contains(a)) {
        msg.push_str(&format!("> {a}\n"));
    }
    msg.push_str("\nA file with MORE uncovered lines than declared has a new gap:\ncover it, or add it to docs/06-limits.md section 54 with the\nreason it cannot run and raise the count here in the same\ncommit. A file with FEWER has a declaration that is now false:\nlower the count and delete the prose that justified it.");
    Err(msg)
}

// ------------------------------------------------------------- gate 18 --

/// D-0677: the integration commit PR #13 is planned against. Self-expiring
/// by ancestry: an unreachable object makes `merge-base` fail, which reads as
/// "not an ancestor".
const INTEGRATION: &str = "0d4fef1391db280b11492e0cae40c9d38720a798";

/// The file whose one-line change must plan a mutant (D-1119).
const PROBE: &str = "crates/cli/build_provenance.rs";

/// The base mutations are planned against, and the diff against it.
fn mutant_diff(out: &str) -> Verdict {
    let mut base = std::env::var("BASE_REF").unwrap_or_default();
    if !base.is_empty()
        && git_ok(&["merge-base", "--is-ancestor", INTEGRATION, "HEAD"])
        && !git_ok(&["merge-base", "--is-ancestor", INTEGRATION, &base])
    {
        println!("D-0677: planning mutations against {INTEGRATION}, not {base}");
        base = INTEGRATION.to_owned();
    }
    if base.is_empty() {
        // A root commit has no HEAD~1; git's complaint is not a finding.
        base = Command::new("git")
            .args(["rev-parse", "--verify", "HEAD~1"])
            .stderr(Stdio::null())
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .trim_end_matches('\n')
                    .to_owned()
            })
            .unwrap_or_default();
    }
    let diff = if base.is_empty() {
        git(&[
            "diff-tree",
            "--no-commit-id",
            "--root",
            "-p",
            "HEAD",
            "--",
            "crates/*.rs",
        ])?
    } else {
        git(&["diff", &format!("{base}...HEAD"), "--", "crates/*.rs"])?
    };
    std::fs::write(out, diff).map_err(|e| format!("{out}: {e}"))
}

fn mutant_walked(walked: &str) -> Verdict {
    match std::fs::metadata(walked) {
        Ok(m) if m.len() > 0 => Ok(()),
        _ => Err("GATE 18: cargo-mutants walked no file".to_owned()),
    }
}

/// `src` with ` // gate 18 self-test` appended to the line after the first
/// line beginning `fn delta_varint(`, or None when there is no such line. A
/// function on the last line leaves the file unchanged, as `sed` did, and
/// the self-test then plans nothing and refuses.
fn mark_probe(src: &str) -> Option<String> {
    let lines: Vec<&str> = src.split_inclusive('\n').collect();
    let at = lines
        .iter()
        .position(|l| l.starts_with("fn delta_varint("))?
        + 1;
    let mut out = String::with_capacity(src.len() + 24);
    for (i, l) in lines.iter().enumerate() {
        if i == at {
            let body = l.strip_suffix('\n').unwrap_or(l);
            out.push_str(body);
            out.push_str(" // gate 18 self-test");
            if l.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push_str(l);
        }
    }
    Some(out)
}

fn mutant_probe_mark(out: &str) -> Verdict {
    let src = read(PROBE)?;
    let marked =
        mark_probe(&src).ok_or_else(|| format!("GATE 18 SELF-TEST: no delta_varint in {PROBE}"))?;
    std::fs::write(PROBE, marked).map_err(|e| format!("{PROBE}: {e}"))?;
    let diff = git(&["diff", "--", PROBE])?;
    std::fs::write(out, diff).map_err(|e| format!("{out}: {e}"))
}

/// The self-test's verdict over the mutants listed for the marked probe.
fn probe_planned(listed: &str) -> Verdict {
    if !listed.contains("build_provenance.rs:") {
        return Err(format!(
            "GATE 18 SELF-TEST: a change to {PROBE} plans no mutant.\ncargo-mutants no longer walks it, or names it in a way respell\ndoes not reach. Either way the plan below would be blind to it."
        ));
    }
    println!(
        "self-test: {} mutant(s) planned for a one-line change in {PROBE}",
        nonempty_lines(listed)
    );
    Ok(())
}

fn mutant_probe_check(listed_path: &str) -> Verdict {
    git(&["checkout", "--", PROBE])?;
    probe_planned(&read(listed_path)?)
}

/// `key=<the file's text>` appended to `$GITHUB_OUTPUT`, trailing newlines
/// stripped as `$( ... )` stripped them. A value spanning lines would break
/// the output file's format and is refused.
fn output_value(text: &str) -> Result<&str, String> {
    let v = text.trim_end_matches('\n');
    if v.contains('\n') {
        return Err("the value spans more than one line".to_owned());
    }
    Ok(v)
}

fn output(key: &str, path: &str) -> Verdict {
    let text = read(path)?;
    let v = output_value(&text).map_err(|e| format!("{path}: {e}"))?;
    append_to_env_file("GITHUB_OUTPUT", &format!("{key}={v}"))
}

// -------------------------------------------------------------- gate 8 --

fn gate_8(benches: usize) -> Verdict {
    if benches == 0 {
        return Err("GATE 8 MEASURED NOTHING — no crate carries a bench.\nWrite one, or record abandoning the invariant in\ndocs/05-decisions.md. This step does not skip.".to_owned());
    }
    println!("{benches} bench source file(s) tracked.");
    Ok(())
}

// ------------------------------------------------------------- gate W1 --

fn w1() -> Verdict {
    let same = Command::new("git")
        .args(["diff", "--exit-code", "--stat", "--", "web/build"])
        .status()
        .map_err(|e| format!("git diff: {e}"))?
        .success();
    if !same {
        return Err("\nThe committed bundle is not what this source builds.\n\nweb/build is served from disk by the Rust binary, so the\nartifact in this repository is what an operator actually\nruns. A source change committed without a rebuild ships a\npage that does not match its own source, and nothing else\nin CI would notice.\n\nFix: npm --prefix web run build, then commit web/build.".to_owned());
    }
    println!("The bundle corresponds to the source. Fingerprint:");
    print!("{}", read("web/build/_app/version.json")?);
    Ok(())
}

// ------------------------------------------------------- gates W3, W4 --

fn ceiling(arg: &str) -> Result<u64, String> {
    arg.parse()
        .map_err(|_| format!("the ceiling `{arg}` is not a whole number"))
}

/// The error count of svelte-check's SUMMARY: the last `<digits> error`
/// (any case, `s` optional) in the log, in either of its two formats.
fn svelte_errors(log: &str) -> Option<&str> {
    let mut last = None;
    for line in records(log) {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if !b[i].is_ascii_digit() {
                i += 1;
                continue;
            }
            let start = i;
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            let tail = &b[i..];
            if tail.len() >= 6 && tail[0] == b' ' && tail[1..6].eq_ignore_ascii_case(b"error") {
                last = Some(&line[start..i]);
                i += 6;
            }
        }
    }
    last
}

fn w3(log: &str, ceiling_arg: &str) -> Verdict {
    let ceiling = ceiling(ceiling_arg)?;
    let Some(found) = svelte_errors(log) else {
        return Err("svelte-check produced no ERRORS line — it did not run, and a\ngate that cannot run must fail rather than pass.".to_owned());
    };
    let found: u64 = found
        .parse()
        .map_err(|_| format!("svelte-check reported an error count past any limit: {found}"))?;
    println!("svelte-check errors: {found} (ceiling {ceiling})");
    if found > ceiling {
        return Err(format!(
            "The tree types clean at {ceiling} and this change does not.\n\nLook for the ROOT, not the reported line. Every error this\ngate has ever counted in bulk came from a handful of untyped\ndeclarations — an empty array inside $state(...) inferring\nnever[], or a callback parameter with no annotation — and\nthe errors surface at the READERS, never at the cause.\n\nA JSDoc cast belongs INSIDE $state(...), not above it:\n  let x = $state(/** @type {{{{ rows: any[] }}}} */ ({{ rows: [] }}));\nand the block must open with /** — a plain /* is not read.\n\nIf this genuinely cannot reach zero, raise CEILING in this\nfile and say why in docs/06-limits.md."
        ));
    }
    if found < ceiling {
        println!(
            "Fewer than the ceiling. Lower CEILING to {found} in this file so\nthe ratchet keeps its meaning."
        );
    }
    Ok(())
}

/// The selector of a vite `Unused CSS selector "x"` warning: the LAST
/// `selector "<one or more non-quotes>"` on the line, or the whole line
/// when there is none, as `sed -E 's/.*selector "([^"]+)".*/\1/'` left it.
fn unused_selector(line: &str) -> &str {
    let head = "selector \"";
    for (at, _) in line.rmatch_indices(head) {
        let rest = &line[at + head.len()..];
        if let Some(end) = rest.find('"')
            && end > 0
        {
            return &rest[..end];
        }
    }
    line
}

fn w4(log: &str, ceiling_arg: &str) -> Verdict {
    let ceiling = ceiling(ceiling_arg)?;
    let built = records(log).iter().any(|l| {
        l.match_indices("built in ")
            .any(|(at, _)| l[at + 9..].starts_with(|c: char| c.is_ascii_digit()))
    });
    if !built {
        let lines = records(log);
        let tail = lines[lines.len().saturating_sub(20)..].join("\n");
        return Err(format!(
            "GATE W4 READ NO BUILD OUTPUT. A silent zero is not a pass.\n{tail}"
        ));
    }
    let found = records(log)
        .into_iter()
        .filter(|l| l.contains("Unused CSS selector"))
        .map(unused_selector)
        .collect::<BTreeSet<&str>>()
        .len() as u64;
    println!("distinct unused CSS selectors: {found} (ceiling {ceiling})");
    if found > ceiling {
        return Err("\nA rule was added for markup that does not exist, or markup was\ndeleted and its styling left behind. The second is how this\nrepository loses safeguards: the comment beside the rule keeps\nasserting a guarantee the page no longer delivers.".to_owned());
    }
    if found < ceiling {
        println!(
            "Fewer than the ceiling. Lower CEILING to {found} in this file so\nthe ratchet keeps its meaning."
        );
    }
    Ok(())
}

// ------------------------------------------------------------- gate W5 --

/// Every path under `dir` whose name ends `.svelte` or `.css`, files and
/// directories alike, symlinks not followed, as `find -name` lists them.
fn style_sources(dir: &Path, out: &mut Vec<String>) -> Verdict {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for e in entries {
        let e = e.map_err(|e| format!("{}: {e}", dir.display()))?;
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.ends_with(".svelte") || name.ends_with(".css") {
            out.push(p.display().to_string());
        }
        if e.file_type()
            .map_err(|e| format!("{}: {e}", p.display()))?
            .is_dir()
        {
            style_sources(&p, out)?;
        }
    }
    Ok(())
}

/// The sorted file list gate W5's front-end program reads. The program
/// refuses an empty list itself; this refuses one first.
fn w5_files() -> Verdict {
    let mut v = Vec::new();
    style_sources(Path::new("web/src"), &mut v)?;
    if v.is_empty() {
        return Err("GATE W5 FOUND NO .svelte OR .css FILE UNDER web/src.".to_owned());
    }
    v.sort();
    for f in v {
        println!("{f}");
    }
    Ok(())
}

// ------------------------------------------------------------- gate W6 --

const MASTERS: &str = "web/masters.js";
const MASTERS_TEST: &str = "web/tests/masters-load.test.js";

/// Does a line of `src` begin with `prefix`?
fn line_starts(src: &str, prefix: &str) -> bool {
    records(src).iter().any(|l| l.starts_with(prefix))
}

/// Gate W6's text checks over `web/masters.js`: every complaint, or none.
fn w6_findings(src: &str) -> Vec<String> {
    let mut bad = Vec::new();
    for f in ["refresh", "verify"] {
        if !src.contains(&format!("addEventListener('click', {f})")) {
            bad.push(format!("`{f}` is not bound to a press in {MASTERS}."));
        }
        // A BARE `fn();` AT COLUMN ZERO IS A CALL ON LOAD.
        if line_starts(src, &format!("{f}();")) {
            bad.push(format!("`{f}()` is INVOKED ON LOAD in {MASTERS}, which spends work nobody\nasked for. Bind it to the press and let the press call it."));
        }
    }
    for fact in [
        "/indexmap.json?feed=",
        "Unconfirmed",
        "Which ones",
        "no exchange confirms",
    ] {
        if !src.contains(fact) {
            bad.push(format!(
                "The cross-verification script no longer carries: {fact}"
            ));
        }
    }
    if !src.contains("Restart the server") {
        bad.push("A successful refresh no longer says the running parse is stale.".to_owned());
    }
    if !line_starts(src, "status();") {
        bad.push("NOTE: status() is no longer called on load — the table will be\nempty until something presses. If that is deliberate, update\nthis gate; it is here because the empty table looked like a bug.".to_owned());
    }
    bad
}

fn w6() -> Verdict {
    if !Path::new(MASTERS).is_file() {
        return Err(format!(
            "{MASTERS} is missing. /masters loads it by name; without it the page\nrenders and every status cell stays empty."
        ));
    }
    if !git_ok(&["ls-files", "--error-unmatch", MASTERS_TEST]) {
        return Err(format!(
            "{MASTERS_TEST} is not tracked. It is what proves opening /masters spends\nno vendor request; the text checks below see one spelling."
        ));
    }
    let bad = w6_findings(&read(MASTERS)?);
    if !bad.is_empty() {
        return Err(bad.join("\n"));
    }
    println!(
        "OK — refresh and verify are bound to a press and called by\n     nothing on load; only the socket-free status() runs."
    );
    Ok(())
}

// ---------------------------------------------------------------- main --

fn arg<'a>(rest: &'a [String], i: usize, what: &str) -> Result<&'a str, String> {
    rest.get(i)
        .map(String::as_str)
        .ok_or_else(|| format!("missing argument: {what}"))
}

fn run(args: &[String]) -> Verdict {
    let (cmd, rest) = args.split_first().ok_or(
        "usage: gates_jobs <probe|gate-13a|gate-13b|gate-13c|tools|gate-5|gate-6d|gate-20|mutant-diff|mutant-walked|mutant-probe-mark|mutant-probe-check|output|gate-8|w1|w3|w4|w5-files|w6> ARGS",
    )?;
    match cmd.as_str() {
        "probe" => probe(),
        "gate-13a" => gate_13a_cmd(arg(rest, 0, "the directory of inverse trees")?),
        "gate-13b" => {
            let tree = read(arg(rest, 0, "the host build graph")?)?;
            let depth = read(arg(rest, 1, "the depth-prefixed build graph")?)?;
            gate_13b(&tree, &depth, &rest[2..])
        }
        "gate-13c" => gate_13c(&read(arg(rest, 0, "the build messages")?)?),
        "tools" => tools(),
        "gate-5" => gate_5_cmd(),
        "gate-6d" => gate_6d(
            arg(rest, 0, "the artifact messages")?,
            arg(rest, 1, "the output directory")?,
        ),
        "gate-20" => gate_20(&read(arg(rest, 0, "the coverage report")?)?, &rest[1..]),
        "mutant-diff" => mutant_diff(arg(rest, 0, "the diff to write")?),
        "mutant-walked" => mutant_walked(arg(rest, 0, "the walked file list")?),
        "mutant-probe-mark" => mutant_probe_mark(arg(rest, 0, "the probe diff to write")?),
        "mutant-probe-check" => mutant_probe_check(arg(rest, 0, "the probe's mutant list")?),
        "output" => output(arg(rest, 0, "the key")?, arg(rest, 1, "the value file")?),
        "gate-8" => gate_8(tracked("crates/*/benches/*.rs")?.len()),
        "w1" => w1(),
        "w3" => w3(
            &read(arg(rest, 0, "the log")?)?,
            arg(rest, 1, "the ceiling")?,
        ),
        "w4" => w4(
            &read(arg(rest, 0, "the log")?)?,
            arg(rest, 1, "the ceiling")?,
        ),
        "w5-files" => w5_files(),
        "w6" => w6(),
        other => Err(format!("unknown command `{other}`")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("no-crates") {
        return no_crates();
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("{why}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| (*x).to_owned()).collect()
    }

    /// Gate 13b's list as the workflow passes it, the banned word assembled
    /// from two halves as gate 15 requires.
    fn banned() -> Vec<String> {
        let w = concat!("py", "thon");
        let list = format!(
            "pyo3 c{w} rust{w}-vm {w}3-sys mlua rlua hlua lua lua-src luajit-src rquickjs quickjs quickjs-rs duktape boa_engine deno_core v8 rusty_v8 mozjs rhai napi neon node-bindgen jni j4rs ruby-sys magnus rutie ext-php-rs wasmtime wasmer wasmi rust{w} deno_runtime quick-js rb-sys jlrs extendr extendr-api extendr-engine starlark rune gluon mun koto steel-core piccolo libR-sys perl-sys libperl-sys tcl ring aws-lc-rs aws-lc-sys cc openssl-sys"
        );
        list.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn records_keep_carriage_returns_and_drop_only_the_final_empty_piece() {
        assert_eq!(records("a\r\nb\n"), ["a\r", "b"]);
        assert_eq!(records("a\n\nb"), ["a", "", "b"]);
        assert!(records("").is_empty());
        assert_eq!(nonempty_lines("a\n\nb\n"), 2);
    }

    #[test]
    fn gate_13a_refuses_a_host_path_and_a_missing_tree() {
        let clean = [
            ("wasm-bindgen", Some(String::new())),
            ("js-sys", Some("\n\n".to_owned())),
            ("web-sys", Some(String::new())),
        ];
        assert!(gate_13a(&clean).is_ok());
        let mut reach = clean.clone();
        reach[1].1 = Some("js-sys v0.3.77\nweb v0.1.0 (/w)\n".to_owned());
        let e = gate_13a(&reach).unwrap_err();
        assert!(e.starts_with("FORBIDDEN HOST-REACHABLE BINDING: js-sys\njs-sys v0.3.77"));
        // One space is not an empty tree: `$( )` strips newlines only.
        let mut space = clean.clone();
        space[2].1 = Some(" \n".to_owned());
        assert!(gate_13a(&space).unwrap_err().contains("web-sys"));
        let mut gone = clean.clone();
        gone[0].1 = None;
        assert!(
            gate_13a(&gone)
                .unwrap_err()
                .contains("NO INVERSE TREE FOR wasm-bindgen")
        );
    }

    #[test]
    fn gate_13b_family_matching_is_the_old_pattern() {
        let names = banned();
        for hit in [
            "ring v0.17.14",
            "ring-core v1 (*)",
            "ring_x v1",
            "cc v1.2.0",
            "aws-lc-sys v0.1",
            "lua-src v5",
            "lua v5",
            "wasmtime-jit v1",
        ] {
            assert!(banned_line(hit, &names), "{hit}");
        }
        for miss in [
            "rustls v0.23.0",
            "ccache-sys v1.0.0",
            "luau v1.0.0",
            "ring- v1",
            "ringo v1",
            " ring v1",
            "ring",
            "ring-a.b v1",
            "serde v1.0.0",
        ] {
            assert!(!banned_line(miss, &names), "{miss}");
        }
        assert!(banned_line(
            &format!("{}3-sys v1", concat!("py", "thon")),
            &names
        ));
    }

    #[test]
    fn gate_13b_refuses_a_banned_package_an_empty_graph_and_a_broken_list() {
        let names = banned();
        let depth = "0core v0.1.0 (/w/core)\n1tls v1\n2ring v0.17.14\n0api v0.1.0 (/w/api)\n";
        assert!(gate_13b("serde v1.0.0\ncore v0.1.0 (/w)\n", depth, &names).is_ok());
        let e = gate_13b(
            "serde v1.0.0\nring v0.17.14\nring v0.17.14\n",
            depth,
            &names,
        )
        .unwrap_err();
        assert!(e.starts_with("BANNED PACKAGE IN THE HOST BUILD GRAPH:\nring v0.17.14\n--- why ring is built:\n  ring v0.17.14  <-  tls v1  <-  core v0.1.0 (/w/core)\n"), "{e}");
        assert_eq!(
            gate_13b("\n", depth, &names).unwrap_err(),
            "GATE 13B READ AN EMPTY BUILD GRAPH."
        );
        assert!(gate_13b("", depth, &names).is_err());
        // A list that lost `ring` or `cc` fails its own fixture first.
        let no_cc: Vec<String> = names.iter().filter(|n| *n != "cc").cloned().collect();
        assert!(
            gate_13b("serde v1\n", depth, &no_cc)
                .unwrap_err()
                .contains("caught 1 of")
        );
        assert!(
            gate_13b("serde v1\n", depth, &[])
                .unwrap_err()
                .contains("caught 0 of")
        );
        let wide = s(&["ring", "cc", "luau"]);
        assert!(
            gate_13b("serde v1\n", depth, &wide)
                .unwrap_err()
                .contains("caught 3 of")
        );
    }

    #[test]
    fn why_built_walks_to_the_root_and_stops_at_twenty() {
        let tree = "0a v1\n1b v1\n2x v1\n1c v1\n2d v1\n3x v1 (*)\n";
        assert_eq!(
            why_built(tree, "x"),
            [
                "x v1  <-  b v1  <-  a v1",
                "x v1 (*)  <-  d v1  <-  c v1  <-  a v1"
            ]
        );
        let many: String = std::iter::repeat_n("0x v1\n", 30).collect();
        assert_eq!(why_built(&many, "x").len(), 20);
        assert!(why_built("not a depth line\n", "not").is_empty());
    }

    fn msg(pkg: &str, libs: &str, out: &str) -> String {
        format!(
            "{{\"reason\":\"build-script-executed\",\"package_id\":\"{pkg}\",\"linked_libs\":{libs},\"linked_paths\":[],\"cfgs\":[],\"env\":[],\"out_dir\":\"{out}\"}}"
        )
    }

    #[test]
    fn gate_13c_extraction_matches_the_old_grep() {
        let m = format!(
            "{}\n{}\n{{\"reason\":\"compiler-artifact\",\"linked_libs\":[\"x\"],\"out_dir\":\"/z\"}}\n",
            msg("a", "[]", "/a"),
            msg("b", "[\"static=ring_core\",\"dylib=m\"]", "/b")
        );
        assert_eq!(
            linked_libs(&m),
            ["\"linked_libs\":[\"static=ring_core\",\"dylib=m\"]"]
        );
        assert_eq!(out_dirs(&m), ["/a", "/b"]);
        assert!(gate_13c_self_test().is_ok());
    }

    fn scratch(tag: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("gates-jobs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn gate_13c_refuses_a_linked_library_an_object_and_no_build_script() {
        let d = scratch("13c");
        let clean = d.join("clean");
        let dirty = d.join("dirty with space/deep");
        std::fs::create_dir_all(&clean).unwrap();
        std::fs::create_dir_all(&dirty).unwrap();
        std::fs::write(clean.join("bindings.rs"), "").unwrap();
        std::fs::write(clean.join("a.out"), "").unwrap();
        let ok = format!("{}\n", msg("a", "[]", &clean.display().to_string()));
        assert!(gate_13c(&ok).is_ok());
        // An out_dir that does not exist is skipped, as `[ -d ]` skipped it.
        let gone = format!("{}\n", msg("a", "[]", "/nonexistent/gates-jobs"));
        assert!(gate_13c(&gone).is_ok());
        let linked = format!("{}\n", msg("r", "[\"static=ring_core\"]", "/nonexistent"));
        assert!(gate_13c(&linked).unwrap_err().contains("native code"));
        for obj in [
            "x.o", "x.obj", "libx.a", "x.lib", "x.so", "x.dylib", "x.dll", ".a",
        ] {
            let f = dirty.join(obj);
            std::fs::write(&f, "").unwrap();
            let one = format!(
                "{}\n",
                msg("c", "[]", &d.join("dirty with space").display().to_string())
            );
            assert!(gate_13c(&one).is_err(), "{obj}");
            std::fs::remove_file(&f).unwrap();
        }
        // A directory named like an object is not a file.
        std::fs::create_dir_all(dirty.join("x.o")).unwrap();
        let dir_named = format!(
            "{}\n",
            msg("c", "[]", &d.join("dirty with space").display().to_string())
        );
        assert!(gate_13c(&dir_named).is_ok());
        let none = "{\"reason\":\"compiler-artifact\"}\n";
        assert!(
            gate_13c(none)
                .unwrap_err()
                .contains("SAW NO BUILD SCRIPT RUN")
        );
        assert!(gate_13c("").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn gate_5_counts_every_exception_form() {
        let allow = concat!("#![al", "low(unsafe_code)]");
        let reasoned = concat!("#[al", "low(unsafe_code, reason = \"x\")]");
        let expect = concat!("#[ex", "pect(unsafe_code)]");
        let src = format!(
            "{allow}\n{reasoned} {expect}\n// forbid(unsafe_code) deny(unsafe_code)\n{}\n{}\n",
            concat!("al", "low(unsafe_codes)"),
            concat!("al", "low( unsafe_code)")
        );
        let hits = unsafe_exceptions(&src);
        assert_eq!(hits.iter().map(|(n, _)| *n).collect::<Vec<_>>(), [1, 2, 2]);
        let one = vec![("a.rs".to_owned(), src.clone())];
        assert!(gate_5(&one).is_ok());
        let two = vec![
            ("a.rs".to_owned(), src.clone()),
            ("b c.rs".to_owned(), format!("{allow}\r\n")),
        ];
        let e = gate_5(&two).unwrap_err();
        assert!(e.contains("More than 3 unsafe exceptions") && e.contains("b c.rs:1:"));
        // Two on one line count twice and are listed once, as `grep -n` lists.
        assert_eq!(e.matches("a.rs:2:").count(), 1);
        // Inside a string or a comment it still counts, as grep counted it.
        let quoted = vec![(
            "q.rs".to_owned(),
            format!("// {allow}\nlet s = \"{expect}{expect}{expect}\";\n"),
        )];
        assert!(gate_5(&quoted).is_err());
    }

    fn art(name: &str, files: &str) -> String {
        format!(
            "{{\"reason\":\"compiler-artifact\",\"package_id\":\"x\",\"target\":{{\"kind\":[\"lib\"],\"crate_types\":[\"lib\"],\"name\":\"{name}\",\"src_path\":\"/s\"}},\"filenames\":[{files}],\"fresh\":true}}"
        )
    }

    #[test]
    fn gate_6d_finds_exactly_one_rlib_per_crate() {
        let mut all: Vec<String> = WEB_EXTERNS
            .iter()
            .map(|n| art(n, &format!("\"/t/lib{n}.rlib\",\"/t/lib{n}.rmeta\"")))
            .collect();
        // `serde` must not read `serde_json`'s line, nor a test target's.
        all.push(
            "{\"reason\":\"compiler-artifact\",\"target\":{\"crate_types\":[\"bin\"],\"name\":\"api\"},\"filenames\":[\"/t/api-test\"]}".to_owned(),
        );
        all.push(art("api", "\"/t/libapi.rlib\""));
        let joined = all.join("\n");
        assert_eq!(rlibs(&joined, "serde"), ["/t/libserde.rlib"]);
        assert_eq!(rlibs(&joined, "api"), ["/t/libapi.rlib"]);
        let (args, api) = web_externs(&joined).unwrap();
        assert_eq!(api, "/t/libapi.rlib");
        assert!(args.starts_with("--extern\nserde=/t/libserde.rlib\n--extern\nserde_json="));
        assert_eq!(args.lines().count(), 18);
        // Two distinct rlibs for one crate, or none, is refused.
        let two = format!("{joined}\n{}", art("tokio", "\"/u/libtokio.rlib\""));
        assert!(
            web_externs(&two)
                .unwrap_err()
                .contains("no single rlib for tokio")
        );
        let none: String = all
            .iter()
            .filter(|l| !l.contains("\"name\":\"vocab\""))
            .cloned()
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            web_externs(&none)
                .unwrap_err()
                .contains("no single rlib for vocab")
        );
        // A first entry that is not an rlib is not read as one.
        assert!(rlibs(&art("x", "\"/t/libx.rmeta\""), "x").is_empty());
        let stray = "{\"reason\":\"compiler-message\",\"crate_types\":[\"lib\"],\"name\":\"x\",\"filenames\":[\"/t/x.rlib\"]}";
        assert!(rlibs(stray, "x").is_empty());
    }

    #[test]
    fn sha256_matches_the_standard_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256_hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
        assert_eq!(
            sha256_hex(&[b'a'; 1000]),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    /// A report in `cargo llvm-cov report --text`'s shape: per file, an
    /// absolute header and `line| count|` rows.
    fn report(files: &[(&str, usize, usize)]) -> String {
        let mut r = String::new();
        for (path, covered, zero) in files {
            r.push_str(&format!("{path}:\n"));
            for i in 0..*covered {
                r.push_str(&format!("{:>5}|{:>6}|    let x = {i};\n", i + 1, 3));
            }
            for i in 0..*zero {
                r.push_str(&format!("{:>5}|{:>6}|    let y = {i};\n", i + 1, 0));
            }
            r.push_str("    6|      |}\n\n");
        }
        r
    }

    fn decl(v: &[&str]) -> Vec<String> {
        s(v)
    }

    #[test]
    fn gate_20_accepts_exactly_the_declared_set() {
        let cov = report(&[
            ("/w/crates/api/src/a.rs", 5, 9),
            ("/w/crates/telemetry/src/lib.rs", 400, 6),
            ("/w/crates/telemetry/src/sink.rs", 200, 20),
            ("/w/crates/vocab/src/mask.rs", 5, 1),
        ]);
        assert_eq!(telemetry_totals(&cov), (626, 26));
        assert_eq!(telemetry_uncovered(&cov), ["lib.rs 6", "sink.rs 20"]);
        assert!(gate_20(&cov, &decl(&["sink.rs", "20", "lib.rs", "6"])).is_ok());
        // One more, one fewer, a missing file, a duplicate: all refused.
        for bad in [
            decl(&["sink.rs", "21", "lib.rs", "6"]),
            decl(&["sink.rs", "19", "lib.rs", "6"]),
            decl(&["sink.rs", "20"]),
            decl(&["sink.rs", "20", "lib.rs", "6", "lib.rs", "6"]),
            decl(&["sink.rs", "20", "lib.rs", "6", "tail.rs", "3"]),
        ] {
            let e = gate_20(&cov, &bad).unwrap_err();
            assert!(e.contains("NOT THE DECLARED ONE"), "{bad:?}");
        }
        assert!(
            gate_20(&cov, &decl(&["sink.rs"]))
                .unwrap_err()
                .contains("pairs")
        );
    }

    #[test]
    fn gate_20_clears_the_file_on_any_header_d0167() {
        // vocab sorts after telemetry: its uncovered line is not telemetry's.
        let cov = report(&[
            ("/w/crates/telemetry/src/value.rs", 600, 0),
            ("/w/crates/vocab/src/mask.rs", 5, 1),
        ]);
        assert_eq!(telemetry_uncovered(&cov), Vec::<String>::new());
        assert_eq!(telemetry_totals(&cov), (600, 0));
        // A subdirectory file is not a telemetry file and clears it too.
        let sub = report(&[
            ("/w/crates/telemetry/src/lib.rs", 600, 1),
            ("/w/crates/telemetry/src/sub/x.rs", 5, 7),
        ]);
        assert_eq!(telemetry_uncovered(&sub), ["lib.rs 1"]);
    }

    #[test]
    fn gate_20_refuses_a_profile_nobody_can_read() {
        let thin = report(&[("/w/crates/telemetry/src/lib.rs", 400, 6)]);
        assert!(
            gate_20(&thin, &decl(&["lib.rs", "6"]))
                .unwrap_err()
                .contains("NO USABLE")
        );
        let stale = report(&[("/w/crates/telemetry/src/lib.rs", 300, 301)]);
        assert!(
            gate_20(&stale, &decl(&["lib.rs", "301"]))
                .unwrap_err()
                .contains("301 of 601")
        );
        // Exactly half uncovered is still read; one past half is not.
        let half = report(&[("/w/crates/telemetry/src/lib.rs", 300, 300)]);
        assert!(gate_20(&half, &decl(&["lib.rs", "300"])).is_ok());
        assert!(gate_20("", &[]).unwrap_err().contains("0 of 0"));
    }

    #[test]
    fn gate_20_line_shapes_are_the_old_patterns() {
        assert!(is_file_header("/w/crates/telemetry/src/lib.rs:"));
        assert!(!is_file_header("/w/crates/telemetry/src/lib.rs:\r"));
        assert!(!is_file_header("w/lib.rs:"));
        assert!(names_telemetry_file(
            "  12|  0| // telemetry/src/sink.rs: note"
        ));
        assert!(!names_telemetry_file("/w/crates/telemetry/src/Sink.rs:"));
        assert!(!names_telemetry_file("/w/crates/telemetry/src/.rs:"));
        assert!(!names_telemetry_file("/w/crates/telemetry/src/a1.rs:"));
        assert_eq!(exec_count("   12|     0|x"), Some("0"));
        assert_eq!(exec_count("12|00|x"), Some("00"));
        assert_eq!(exec_count("   12|  1.2k|x"), None);
        assert_eq!(exec_count("   12|      |x"), None);
        assert_eq!(exec_count("\t12|0|x"), None);
        assert_eq!(exec_count("|0|x"), None);
        assert_eq!(
            telemetry_name("/a/crates/telemetry/src/x/crates/telemetry/src/sink.rs:"),
            "sink.rs"
        );
        assert_eq!(
            telemetry_name("rel/telemetry/src/a.rs:"),
            "rel/telemetry/src/a.rs"
        );
        // A telemetry mention inside a counted row sets the file and is not
        // itself counted, exactly as awk's `next` skipped it.
        let cov = "/w/crates/api/src/a.rs:\n    1|    0| telemetry/src/sink.rs: x\n    2|    0|y\n";
        assert_eq!(
            telemetry_uncovered(cov),
            ["    1|    0| telemetry/src/sink.rs x 1"]
        );
    }

    #[test]
    fn mark_probe_appends_to_the_line_after_the_function() {
        let src = "a\nfn delta_varint(b: &[u8]) {\n    body\n}\n";
        assert_eq!(
            mark_probe(src).unwrap(),
            "a\nfn delta_varint(b: &[u8]) {\n    body // gate 18 self-test\n}\n"
        );
        let crlf = "fn delta_varint(x) {\r\n    body\r\n";
        assert_eq!(
            mark_probe(crlf).unwrap(),
            "fn delta_varint(x) {\r\n    body\r // gate 18 self-test\n"
        );
        assert_eq!(
            mark_probe("fn delta_varint(x)").unwrap(),
            "fn delta_varint(x)"
        );
        assert!(mark_probe("    fn delta_varint(x) {\n}\n").is_none());
        assert!(mark_probe("fn delta_varints(x) {\n}\n").is_none());
        assert!(mark_probe("pub fn delta_varint(x) {\n}\n").is_none());
        // The first definition only.
        let two = "fn delta_varint(a)\n1\nfn delta_varint(b)\n2\n";
        assert_eq!(
            mark_probe(two).unwrap(),
            "fn delta_varint(a)\n1 // gate 18 self-test\nfn delta_varint(b)\n2\n"
        );
    }

    #[test]
    fn the_probe_must_plan_a_mutant_in_its_own_file() {
        assert!(probe_planned("crates/cli/src/../build_provenance.rs:613:5: replace x\n").is_ok());
        assert!(probe_planned("").is_err());
        assert!(probe_planned("crates/cli/src/lib.rs:1:1: replace\n").is_err());
        assert!(probe_planned("build_provenance.rs 1\n").is_err());
    }

    #[test]
    fn walked_and_output_refuse_what_the_shell_refused() {
        let d = scratch("walk");
        let empty = d.join("empty");
        std::fs::write(&empty, "").unwrap();
        assert!(mutant_walked(&empty.display().to_string()).is_err());
        assert!(mutant_walked(&d.join("absent").display().to_string()).is_err());
        let one = d.join("one");
        std::fs::write(&one, "crates/a/src/lib.rs\n").unwrap();
        assert!(mutant_walked(&one.display().to_string()).is_ok());
        assert_eq!(output_value("{\"include\":[]}\n\n"), Ok("{\"include\":[]}"));
        assert!(output_value("a\nb\n").is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn gate_8_refuses_no_bench() {
        assert!(
            gate_8(0)
                .unwrap_err()
                .starts_with("GATE 8 MEASURED NOTHING")
        );
        assert!(gate_8(3).is_ok());
    }

    #[test]
    fn w3_reads_both_summary_formats_and_refuses_above_the_ceiling() {
        let machine =
            "x\n1680000000 COMPLETED 325 FILES 61 ERRORS 1 WARNINGS 11 FILES_WITH_PROBLEMS\n";
        let human = "Error: thing\nsvelte-check found 64 errors and 1 warning in 11 files\n";
        assert_eq!(svelte_errors(machine), Some("61"));
        assert_eq!(svelte_errors(human), Some("64"));
        assert_eq!(svelte_errors("found 1 Error\n"), Some("1"));
        assert_eq!(svelte_errors("a12 errors then 3 ERRORSx\n"), Some("3"));
        assert_eq!(svelte_errors("svelte-check found 0 errors\r\n"), Some("0"));
        assert_eq!(svelte_errors("no count\n12  errors\n12 err\n"), None);
        assert!(w3("svelte-check found 0 errors and 0 warnings\n", "0").is_ok());
        assert!(w3(human, "0").unwrap_err().contains("types clean at 0"));
        assert!(
            w3("nothing ran\n", "0")
                .unwrap_err()
                .contains("no ERRORS line")
        );
        assert!(w3("99999999999999999999999 errors\n", "0").is_err());
        assert!(w3("0 errors\n", "x").is_err());
        // The LAST count is the summary, earlier ones are diagnostics.
        assert!(w3("5 errors\nfound 0 errors\n", "0").is_ok());
        assert!(w3("found 0 errors\n5 errors\n", "0").is_err());
        assert!(w3("found 2 errors\n", "3").is_ok());
    }

    #[test]
    fn w4_counts_distinct_selectors_and_refuses_a_log_without_a_build() {
        let ok = "vite v5\n✓ built in 2.31s\n";
        assert!(w4(ok, "0").is_ok());
        let unused = "warn: Unused CSS selector \".a\"\nwarn: Unused CSS selector \".a\"\nUnused CSS selector \"div > .b\" (x.svelte:3)\nbuilt in 1s\n";
        assert!(
            w4(unused, "0")
                .unwrap_err()
                .contains("markup that does not exist")
        );
        assert!(w4(unused, "2").is_ok());
        assert!(w4(unused, "1").is_err());
        assert!(
            w4("Unused CSS selector \".a\"\n", "5")
                .unwrap_err()
                .contains("READ NO BUILD OUTPUT")
        );
        assert!(w4("built in x\n", "5").is_err());
        assert!(w4("", "5").is_err());
        assert_eq!(
            unused_selector("Unused CSS selector \"a\" selector \"b\""),
            "b"
        );
        assert_eq!(
            unused_selector("Unused CSS selector \"a\" selector \"\""),
            "a"
        );
        assert_eq!(
            unused_selector("Unused CSS selector .a"),
            "Unused CSS selector .a"
        );
        assert_eq!(unused_selector("Unused CSS selector \"é ü\"\r"), "é ü");
    }

    #[test]
    fn w5_lists_svelte_and_css_files_sorted() {
        let d = scratch("w5");
        std::fs::create_dir_all(d.join("lib/x.css")).unwrap();
        for f in [
            "b.svelte",
            "a.css",
            "lib/c.svelte",
            "lib/d.js",
            "lib/e.svelte.js",
            ".css",
        ] {
            std::fs::write(d.join(f), "").unwrap();
        }
        let mut v = Vec::new();
        style_sources(&d, &mut v).unwrap();
        v.sort();
        let base = d.display().to_string();
        let rel: Vec<String> = v.iter().map(|p| p[base.len() + 1..].to_owned()).collect();
        assert_eq!(
            rel,
            [".css", "a.css", "b.svelte", "lib/c.svelte", "lib/x.css"]
        );
        assert!(style_sources(&d.join("absent"), &mut v).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }

    const GOOD_MASTERS: &str = "function refresh() { fetch('/masters/refresh'); }\nfunction verify() { fetch('/indexmap.json?feed=' + f); }\ngo.addEventListener('click', refresh);\nv.addEventListener('click', verify);\n// Unconfirmed, Which ones, no exchange confirms, Restart the server\nstatus();\n";

    #[test]
    fn w6_accepts_the_bound_script_and_refuses_each_breach() {
        assert!(w6_findings(GOOD_MASTERS).is_empty());
        let cases = [
            ("refresh();\n", "INVOKED ON LOAD"),
            ("verify();\r\n", "INVOKED ON LOAD"),
        ];
        for (extra, why) in cases {
            let f = w6_findings(&format!("{GOOD_MASTERS}{extra}"));
            assert!(f.iter().any(|x| x.contains(why)), "{extra}");
        }
        // An indented call is reached only through a press, as before.
        assert!(w6_findings(&format!("{GOOD_MASTERS}  refresh();\n")).is_empty());
        for (from, to, why) in [
            (
                "addEventListener('click', refresh)",
                "addEventListener('click',refresh)",
                "`refresh` is not bound",
            ),
            (
                "addEventListener('click', verify)",
                "x",
                "`verify` is not bound",
            ),
            (
                "/indexmap.json?feed=",
                "/indexmap.json",
                "no longer carries: /indexmap.json?feed=",
            ),
            (
                "Unconfirmed",
                "unconfirmed",
                "no longer carries: Unconfirmed",
            ),
            ("Which ones", "Which", "no longer carries: Which ones"),
            (
                "no exchange confirms",
                "x",
                "no longer carries: no exchange confirms",
            ),
            ("Restart the server", "Restart", "running parse is stale"),
            (
                "\nstatus();",
                "\n  status();",
                "status() is no longer called",
            ),
        ] {
            let f = w6_findings(&GOOD_MASTERS.replace(from, to));
            assert!(f.iter().any(|x| x.contains(why)), "{from} -> {to}: {f:?}");
        }
    }

    #[test]
    fn ceilings_and_usage_refuse_bad_arguments() {
        assert!(ceiling("-1").is_err());
        assert_eq!(ceiling("7"), Ok(7));
        assert!(run(&[]).is_err());
        assert!(run(&s(&["nope"])).unwrap_err().contains("unknown command"));
        assert!(
            run(&s(&["gate-13c"]))
                .unwrap_err()
                .contains("missing argument")
        );
        assert!(
            run(&s(&["gate-20"]))
                .unwrap_err()
                .contains("missing argument")
        );
        assert!(
            run(&s(&["output", "k"]))
                .unwrap_err()
                .contains("missing argument")
        );
    }
}
