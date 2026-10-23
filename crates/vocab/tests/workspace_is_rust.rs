//! **The Rust-only rule, enforced mechanically over the whole dependency tree.**
//!
//! `CLAUDE.md` §2 forbids "any vendored binding to another language" and "any
//! `build.rs` that invokes an external process", **without exception**, and CI
//! gate 13's own title is "no foreign runtime, *not even through a dependency*".
//!
//! # Why this test exists when gate 13 already does
//!
//! Gate 13 is a name scan against a short list, and its own comment admits the
//! hole: *"A binding published under a name that is not on the list. The list
//! is finite and the ecosystem is not."* `ring` is not on that list, and `ring`
//! ships **135 non-Rust files** — 17 `.c`, 28 `.h`, 73 `.S`, 17 `.asm` — and
//! compiles them through its own `build.rs`. It passed every gate in the
//! repository.
//!
//! This test inverts the logic. Instead of listing what is forbidden, it lists
//! what is **known and declared**, and fails on anything else. A new native
//! dependency cannot arrive quietly; it arrives as a failing test naming itself.
//!
//! # Where this belongs
//!
//! It guards the workspace, not this crate. It lives here because it has to run
//! under `cargo test` and `vocab` is the crate with no other tenant. It should
//! move to a dedicated gate the day one exists — recorded here rather than left
//! for somebody to wonder about.
//!
//! # What it cannot see
//!
//! Said plainly, because a gate that oversells itself is worse than no gate:
//!
//! * A crate that shells out from `build.rs` **without shipping source** — it
//!   would have no `.c` to find. The declared list is by name, so a rename
//!   defeats it exactly as it defeats gate 13.
//! * Whether a declared crate is actually **compiled** on this target, read
//!   from the build itself. `Cargo.lock` records the edges of every target and
//!   of weak (`dep?/feature`) features, so the reachability walk below skips
//!   exactly the edges [`LOCK_ONLY_EDGES`] names, each with its reason, and
//!   every other edge counts as compiled. Target `cfg` expressions are not
//!   evaluated: a new target-gated path to a declared crate fails here and has
//!   to be argued into that table. D-1435.
//! * Anything in `web/`, which `CLAUDE.md` §2 exempts by path.

use std::collections::{BTreeMap, BTreeSet};

/// Every dependency known to ship non-Rust source, with what it ships and
/// whether it reaches the shipped binary.
///
/// Produced by walking every dependency source tree in
/// `~/.cargo/registry/src` and counting files by extension — 182 of them on
/// 2026-08-10, and 192 on 2026-08-26 after the `gzip` feature. **This is a
/// measurement, not a guess.** The count lives in the assertion below rather
/// than in this sentence, because a number written twice goes stale in one
/// place first.
struct Declared {
    name: &'static str,
    ships: &'static str,
    why: &'static str,
}

const DECLARED: &[Declared] = &[
    Declared {
        name: "ring",
        ships: "17 .c, 28 .h, 73 .S, 17 .asm, plus build.rs",
        why: "IF THIS PRINTS, RING IS REACHABLE FROM A WORKSPACE MEMBER THROUGH AN \
              EDGE LOCK_ONLY_EDGES DOES NOT SKIP. History: as of 2026-08-26 it \
              was not reached, and this row once said the opposite. It read \
              `THE OPEN §2 BREACH. Measured: nm on the release binary returns 72 \
              ring_core symbols, so this is linked, not dormant` — which was true \
              when written and stopped being true the moment `reqwest` moved to \
              `rustls-tls-webpki-roots-no-provider` and `rustls-graviola` took \
              over, the change the 189 -> 176 note below records. The sentence \
              outlived it and directly contradicted the table it sat in. \
              Re-measured 2026-08-26: `cargo tree -i ring --target all` prints \
              nothing, so it is resolved and not built. It stays in Cargo.lock \
              as an OPTIONAL dependency of `rustls-webpki`, recorded there only \
              because of the weak `ring?/alloc` feature (LOCK_ONLY_EDGES), which \
              is why it is still declared here rather than deleted — the day \
              something turns that feature on, a new edge such as `rustls -> \
              ring` appears and this row is what says what arrived.",
    },
    Declared {
        name: "cc",
        ships: "1 .c",
        why: "The compiler driver ring builds through. Its own .c is a probe fixture, \
              but the crate's PURPOSE is invoking a C compiler from a build script.",
    },
    Declared {
        name: "wit-bindgen",
        ships: "3 .c, plus build.rs",
        why: "WebAssembly component tooling, reached only on wasm targets. Not built \
              for the host. UNVERIFIED that no wasm build path pulls it in.",
    },
    Declared {
        name: "iana-time-zone-haiku",
        ships: "1 .cc, plus build.rs",
        why: "Target-gated to Haiku OS. Never compiled on macOS or Linux. Present in \
              the lockfile because cargo resolves every target.",
    },
];

/// Names that must not appear at all: a foreign runtime, or a binding to one.
/// Distinct from DECLARED — these are not tolerated, they are absent.
///
/// # One list for three mechanisms (D-1108)
///
/// Gate 13's `banned`, this list and `deny.toml`'s `deny` were written apart and
/// disagreed: gate 13, which runs first, did not name `napi`, the two crates that
/// embed or bind the best-known interpreted language, `rhai`, `mozjs`, `j4rs`,
/// `quickjs` or `lua`; this list did not name eleven that gate 13 did; and one
/// entry here, the `lib`-prefixed `-sys` shim, is not a crate at all (crates.io
/// answered 404 on 2026-10-02), so it stopped nothing while reading as
/// protection. The real crate is the unprefixed one, and it is named instead.
/// `the_three_banned_lists_agree` in `crates/core/tests/banned_lists.rs` now reads
/// this list, gate 13's and `deny.toml`'s and holds them together. It lives in
/// `core` because gate 22 clause D refuses a sweep crate's compile-time include of
/// the workflow file. Every name here answered 200 from
/// `https://crates.io/api/v1/crates/<name>` on that date, except the two
/// D-2350 family entries the comment beside them names.
///
/// Every entry is matched with its family -- the name itself, or the name
/// followed by `-` or `_` and more -- exactly as gate 13 matches it.
const FORBIDDEN: &[&str] = &[
    "pyo3",
    concat!("rust", "py", "thon", "-vm"),
    concat!("c", "py", "thon"),
    concat!("py", "thon", "3-sys"),
    "napi",
    "neon",
    "node-bindgen",
    "mlua",
    "rlua",
    "hlua",
    "lua",
    "lua-src",
    "luajit-src",
    "duktape",
    "quickjs",
    "quickjs-rs",
    "rquickjs",
    "boa_engine",
    "v8",
    "rusty_v8",
    "deno_core",
    "mozjs",
    "rhai",
    "jni",
    "j4rs",
    "ruby-sys",
    "magnus",
    "rutie",
    "ext-php-rs",
    "openssl-sys",
    // D-2350 (RO-9): WebAssembly runtimes and further embedded interpreters.
    // `extendr` and `perl-sys` answered 404 on 2026-10-04 and are kept for
    // their families; every other name below answered 200.
    "wasmtime",
    "wasmer",
    "wasmi",
    concat!("rust", "py", "thon"),
    "deno_runtime",
    "quick-js",
    "rb-sys",
    "extendr",
    "extendr-api",
    "extendr-engine",
    "jlrs",
    "starlark",
    "rune",
    "gluon",
    "mun",
    "koto",
    "steel-core",
    "piccolo",
    "libR-sys",
    "perl-sys",
    "libperl-sys",
    "tcl",
];

/// An edge `Cargo.lock` records that a normal build on this host does not
/// compile. Matched by exact `(parent, child)` names, so a NEW path to the same
/// child is not covered and fails [`no_declared_native_dependency_is_compiled_any_more`].
struct LockOnlyEdge {
    parent: &'static str,
    child: &'static str,
    why: &'static str,
}

/// The lock records more than the build compiles, in two named ways, and these
/// are the only edges the reachability walk skips. Each is checked to still be
/// in the lock, so a row cannot outlive its edge.
const LOCK_ONLY_EDGES: &[LockOnlyEdge] = &[
    LockOnlyEdge {
        parent: "rustls-webpki",
        child: "ring",
        why: "rustls-webpki's `alloc` feature is `ring?/alloc`, a WEAK feature: it \
              does not enable `ring`, but cargo's lock resolver still records the \
              optional dependency. Re-enabling ring for real (rustls's `ring` \
              feature) adds a `rustls -> ring` edge, which is not this row.",
    },
    LockOnlyEdge {
        parent: "iana-time-zone",
        child: "iana-time-zone-haiku",
        why: "a `[target.'cfg(target_os = \"haiku\")'.dependencies]` entry; the lock \
              resolves every target, the build compiles only the host's.",
    },
    LockOnlyEdge {
        parent: "wasip2",
        child: "wit-bindgen",
        why: "wasip2 is itself reached only on wasm32-wasip2 targets (getrandom's \
              target table); the host build never compiles it.",
    },
];

/// One `[[package]]` stanza of `Cargo.lock`, as written.
struct LockPackage {
    name: String,
    version: String,
    /// No `source` line: a workspace member (or a path dependency).
    workspace: bool,
    /// The `dependencies` entries verbatim: `"name"`, `"name version"` or
    /// `"name version (source)"`.
    deps: Vec<String>,
}

fn quoted(s: &str) -> Option<&str> {
    s.trim()
        .trim_end_matches(',')
        .strip_prefix('"')?
        .strip_suffix('"')
}

/// Parses every `[[package]]` stanza. A line it does not understand is ignored;
/// the callers' size checks are what turn a format change into a failure.
fn parse_lock(text: &str) -> Vec<LockPackage> {
    let mut out: Vec<LockPackage> = Vec::new();
    let mut in_package = false;
    let mut in_deps = false;
    for line in text.lines() {
        let t = line.trim();
        if t.starts_with('[') && !in_deps {
            in_package = t == "[[package]]";
            if in_package {
                out.push(LockPackage {
                    name: String::new(),
                    version: String::new(),
                    workspace: true,
                    deps: Vec::new(),
                });
            }
            continue;
        }
        let Some(pkg) = out.last_mut().filter(|_| in_package) else {
            continue;
        };
        if in_deps {
            if t == "]" {
                in_deps = false;
            } else if let Some(dep) = quoted(t) {
                pkg.deps.push(dep.to_owned());
            }
        } else if let Some(v) = t.strip_prefix("name = ").and_then(quoted) {
            v.clone_into(&mut pkg.name);
        } else if let Some(v) = t.strip_prefix("version = ").and_then(quoted) {
            v.clone_into(&mut pkg.version);
        } else if t.starts_with("source = ") {
            pkg.workspace = false;
        } else if t == "dependencies = [" {
            in_deps = true;
        }
    }
    out
}

/// Names of every package a workspace member reaches through the lock's
/// dependency edges, skipping exactly the `skip` edges. O(V + E) in the lock.
///
/// A dependency entry naming no package in the lock is corrupt input and is an
/// `Err` naming it: a walk that silently dropped it would under-report.
fn reachable_from_workspace(
    pkgs: &[LockPackage],
    skip: &[LockOnlyEdge],
) -> Result<BTreeSet<String>, String> {
    let mut by_name: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (i, p) in pkgs.iter().enumerate() {
        by_name.entry(p.name.as_str()).or_default().push(i);
    }
    let mut seen: Vec<bool> = pkgs.iter().map(|p| p.workspace).collect();
    let mut stack: Vec<&LockPackage> = pkgs.iter().filter(|p| p.workspace).collect();
    let mut out = BTreeSet::new();
    while let Some(parent) = stack.pop() {
        out.insert(parent.name.clone());
        for dep in &parent.deps {
            let mut parts = dep.split(' ');
            let name = parts.next().unwrap_or_default();
            let version = parts.next();
            if skip
                .iter()
                .any(|e| e.parent == parent.name && e.child == name)
            {
                continue;
            }
            let targets: Vec<usize> = by_name
                .get(name)
                .into_iter()
                .flatten()
                .copied()
                .filter(|&j| version.is_none_or(|v| pkgs.get(j).is_some_and(|p| p.version == v)))
                .collect();
            if targets.is_empty() {
                return Err(format!(
                    "`{}` depends on `{dep}`, which no [[package]] in the lock provides",
                    parent.name
                ));
            }
            for j in targets {
                if let (Some(flag), Some(child)) = (seen.get_mut(j), pkgs.get(j))
                    && !*flag
                {
                    *flag = true;
                    stack.push(child);
                }
            }
        }
    }
    Ok(out)
}

/// The [`DECLARED`] native crates a workspace member reaches in `lock_text`,
/// with [`LOCK_ONLY_EDGES`] skipped.
fn declared_native_reached(lock_text: &str) -> Result<Vec<&'static Declared>, String> {
    let reached = reachable_from_workspace(&parse_lock(lock_text), LOCK_ONLY_EDGES)?;
    Ok(DECLARED
        .iter()
        .filter(|d| reached.contains(d.name))
        .collect())
}

/// FNV-1a over every package's name, version AND dependency list, sorted by
/// name and version, with separators so `ab`+`c` and `a`+`bc` cannot collide.
/// Hand-rolled because this crate takes no dependencies.
///
/// The edges are in it because the name set alone missed the one change that
/// matters most: a feature flip that turns an optional native crate back on
/// leaves every name in place and adds one line to a `dependencies` list.
fn fingerprint(pkgs: &[LockPackage]) -> u64 {
    let mut rows: Vec<&LockPackage> = pkgs.iter().collect();
    rows.sort_unstable_by(|a, b| (&a.name, &a.version).cmp(&(&b.name, &b.version)));
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |bytes: &[u8], sep: u8| {
        for b in bytes.iter().chain(std::iter::once(&sep)) {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
    };
    for p in rows {
        eat(p.name.as_bytes(), 0x1f);
        // D-1611: the version is hashed, not only sorted on, so a version-only
        // change moves the pin (audit-20261003 rustonly2-6).
        eat(p.version.as_bytes(), 0x1c);
        for d in &p.deps {
            eat(d.as_bytes(), 0x1e);
        }
        eat(&[], 0x1d);
    }
    h
}

/// Is `package` the crate `name` or one of its `name-*` / `name_*` family?
fn in_family(package: &str, name: &str) -> bool {
    package == name
        || package
            .strip_prefix(name)
            .is_some_and(|rest| rest.len() > 1 && (rest.starts_with('-') || rest.starts_with('_')))
}

fn lock_text() -> String {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock");
    let lock = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        !lock.is_empty(),
        "the workspace lockfile is not readable at {path} — this gate cannot run, \
         and a gate that cannot run must fail rather than pass"
    );
    lock
}

fn lockfile_packages() -> BTreeSet<String> {
    let out: BTreeSet<String> = parse_lock(&lock_text())
        .into_iter()
        .map(|p| p.name)
        .collect();
    assert!(
        out.len() > 100,
        "parsed only {} packages — the lockfile format changed and this test stopped testing",
        out.len()
    );
    out
}

/// No interpreted runtime and no binding to one, at any depth.
#[test]
fn no_foreign_runtime_reaches_the_workspace() {
    let packages = lockfile_packages();
    let found: Vec<&String> = packages
        .iter()
        .filter(|p| FORBIDDEN.iter().any(|f| in_family(p, f)))
        .collect();
    assert!(
        found.is_empty(),
        "CLAUDE.md §2 forbids a foreign runtime without exception, and the lockfile holds: {found:?}"
    );
}

/// The family rule matches the name and its `-`/`_` relatives, and nothing that
/// merely starts with the same letters.
#[test]
fn a_family_is_the_name_and_its_dashed_relatives() {
    assert!(in_family("lua", "lua"));
    assert!(in_family("lua-src", "lua"));
    assert!(in_family("napi_derive", "napi"));
    assert!(!in_family("luau", "lua"));
    assert!(!in_family("lua-", "lua"));
    assert!(!in_family("v", "v8"));
    assert!(!in_family("cc", "c"));
}

/// **The load-bearing one, rewritten because the first version did not work.**
///
/// The original asserted that every DECLARED crate was still present and that
/// `DECLARED.len() == 4`. That is a constant compared to a constant. It never
/// looked at the lockfile for an **undeclared** native crate, which is the only
/// thing it existed to catch — a reviewer described its sole real check as "the
/// number 4 still equals 4", and that was accurate.
///
/// # Why a fingerprint rather than a scan
///
/// The honest check is "does any dependency ship C that is not declared", and a
/// registry walk cannot answer it here: `~/.cargo/registry` is not present in a
/// clean CI checkout, and a vendored or `[patch]`ed crate has no registry entry
/// at all. So this pins the **whole dependency set** instead.
///
/// A new native-code crate is necessarily a **new package**. Pinning the set
/// therefore catches it — along with every other dependency change, which is a
/// feature and not noise: `CLAUDE.md` §2 makes the dependency graph part of the
/// law, so it should not move without somebody looking.
///
/// When this fails, the fix is not to update the number. It is to run the
/// registry scan, decide whether the new crate ships non-Rust source, update
/// [`DECLARED`] if it does, and only then re-pin.
///
/// **It has already earned its keep.** Landing `crates/indicators` moved the set
/// from 185 packages to 186 and this test went red — correctly. The scan was run
/// (the new package holds one `.rs` and one `.toml`, no `build.rs`, no C), and
/// only then was the pin moved to 186 / `0x53A9_30D0_795D_5D5E`. That sequence,
/// not the number, is the guarantee.
#[test]
fn the_dependency_set_has_not_moved_without_review() {
    let packages = lockfile_packages();

    // Every declared crate must still be present — a stale declaration implies a
    // breach that is gone, which is its own kind of lie.
    for d in DECLARED {
        assert!(
            packages.contains(d.name),
            "`{}` is declared as a native-code dependency but is no longer in the \
             lockfile. Remove the declaration rather than leaving it to imply a \
             breach that no longer exists.",
            d.name
        );
    }

    let names: Vec<&str> = packages.iter().map(String::as_str).collect();
    let h = fingerprint(&parse_lock(&lock_text()));

    // 188 -> 189 IS `crates/cli`, AND THE SCAN WAS RUN BEFORE THIS NUMBER MOVED.
    //
    // Same discipline as the move below it, recorded the same way. The delta
    // against `d6a8457^` is exactly one name -- `cli` -- with nothing removed.
    // It is a workspace member, not a registry crate: `Cargo.toml`, `src/lib.rs`,
    // `src/main.rs` and `tests/binary.rs`, no `.c`/`.cc`/`.h`/`.S`/`.asm` and no
    // `build.rs`. Its dependencies are `core`, `engine`, `indicators` and `vocab`,
    // every one already in this lock, so NO third-party code entered the tree with
    // it and `DECLARED` is unchanged because there is nothing new to declare.
    //
    // 187 -> 188 IS `crates/runner`, AND THE SCAN WAS RUN BEFORE THAT NUMBER MOVED.
    //
    // The message below says what the discipline is and why -- "that is how `ring`
    // got in" -- so here is the scan it asks for, recorded rather than claimed.
    // The new package is a workspace member, not a registry crate: it holds two
    // `.rs` files and one `.toml`, ships no `.c`/`.cc`/`.h`/`.S`/`.asm`, carries no
    // `build.rs`, and its four dependencies are `core`, `engine`, `indicators` and
    // `vocab` -- every one already in this lock. So the delta is exactly the member
    // itself and NO third-party code entered the tree with it. `DECLARED` is
    // unchanged because there is nothing new to declare.
    // 189 -> 176 IS THE REMOVAL OF `ring`, AND THE SCAN WAS RUN BEFORE THIS
    // NUMBER MOVED. `reqwest` moved from the `rustls-tls` feature to
    // `rustls-tls-webpki-roots-no-provider`, which drops rustls's default
    // `ring` provider, and `rustls-graviola` replaces it. Thirteen names left
    // the lockfile and none arrived that ships non-Rust source: the registry
    // scan over the resulting build graph found ZERO crates shipping .c, .h,
    // .S, .asm, .cc, .cpp or .js. graviola itself ships none of those and has
    // no build.rs -- its assembly is `core::arch::asm!`, ordinary stable Rust.
    // 176 -> 182 BY `rayon`, AND THE SCAN THIS TEST DEMANDS WAS RUN.
    //
    // `c0bc3eb` gave `crates/cli` a `rayon` arrow so the whole-store sweep walks
    // instrument-months in parallel. Six packages arrived and no others:
    // `rayon`, `rayon-core`, `either`, `crossbeam-deque`, `crossbeam-epoch`,
    // `crossbeam-utils`. The count moved by exactly six, which is itself the
    // check that nothing else came with them.
    //
    // THE SCAN, because this test's own message says not to re-pin without it —
    // *"any new crate may ship C, and DECLARED is the record"* — and because the
    // sentence after it names what happens when somebody does: *"that is how
    // `ring` got in."*
    //
    // Measured over the vendored sources of all six:
    //   * **zero** files matching .c .h .S .asm .cc .cpp .js — so none is added
    //     to DECLARED, because DECLARED records crates that ship non-Rust source
    //     and none of these does;
    //   * four carry a `build.rs` — crossbeam-deque, crossbeam-epoch,
    //     crossbeam-utils and rayon-core — and `CLAUDE.md` §2 forbids not a
    //     build script but *"any `build.rs` that invokes an external process"*.
    //     Each was read: they are `env::var` reads and `println!("cargo:...")`
    //     cfg emission, nothing else. Grepped for `Command`, `process::`,
    //     `Stdio`, `spawn`, `.output()` and `.status()` across all four: zero
    //     hits. `rayon` and `either` carry no build script at all.
    //
    // So §2 holds without an exception, and the pin moves. D-0232.
    //
    // 182 -> 192 BY `gzip`, AND THIS PIN WAS RED FOR A COMMIT BEFORE ANYBODY
    // NOTICED. `bae4148` added the `gzip` feature to `reqwest` so a vendor
    // serving a compressed instrument master would not land as garbage, ran
    // `cargo deny check`, and never ran the workspace suite — so the count
    // moved to 192 and the pin stayed at 182 and the commit went in red. That
    // is worth writing down beside the number rather than quietly corrected:
    // this test is the only thing standing between a feature flag and a C
    // dependency, and it can only work if the suite is actually run.
    //
    // Ten packages arrived and no others: `adler2`, `async-compression`,
    // `compression-codecs`, `compression-core`, `crc32fast`, `flate2`,
    // `futures-sink`, `miniz_oxide`, `simd-adler32`, `tokio-util`. The count
    // moved by exactly ten, which is itself the check that nothing else came
    // with them.
    //
    // THE SCAN, because this test's own message demands it. Measured over the
    // vendored sources of all ten:
    //   * **zero** files matching .c .h .S .asm .cc .cpp .js across every one
    //     of them -- so none is added to DECLARED, which records crates that
    //     ship non-Rust source. `flate2` is here on its `rust_backend`, and
    //     `miniz_oxide` is that backend: a pure-Rust DEFLATE, which is the
    //     whole reason this feature could be taken at all. Nothing links
    //     `zlib`.
    //   * exactly one carries a `build.rs` -- `crc32fast` -- and it runs
    //     `$RUSTC --version` to decide whether the stable ARM CRC32 intrinsics
    //     exist. That is the same feature-probe `serde`, `libc`, `proc-macro2`,
    //     `quote`, `httparse`, `ahash` and `generic-array` already perform in
    //     this tree; `docs/06-limits.md` §96 measures the set and records that
    //     §2's dependency-level build-script ban is not held by the workspace
    //     and is not holdable. It is not a new exception, and it is not being
    //     treated as one.
    //
    // The cookie feature that would have arrived alongside it was NOT taken --
    // `cargo deny` refused the `time 0.3.45` it dragged in -- so `cookie`,
    // `cookie_store` and `time` are absent from this count. D-0311.
    //
    // 192 -> 194 BY `rustix`, AND THE SCAN WAS RUN BEFORE THIS NUMBER MOVED.
    //
    // `api` names `rustix` 1.1.4 (`std`, `fs`, no defaults) so `/disk.json`
    // measures its filesystem with one `statvfs(2)` instead of starting
    // `/bin/df` (D-5240). Two packages arrived and no others: `rustix` and
    // `linux-raw-sys` 0.12.1; the four others `rustix` names (`bitflags`,
    // `errno`, `libc`, `windows-sys` 0.61.2) were already locked at the version
    // it takes. Measured over the registry sources of both:
    //   * **zero** files matching .c .cc .cpp .h .S .s .asm .js -- `rustix`
    //     ships 318 `.rs` files and `linux-raw-sys` 462, the rest licences,
    //     manifests and Markdown -- so neither is added to DECLARED. On Linux
    //     `rustix` makes its system calls through `core::arch::asm!`, ordinary
    //     stable Rust, as `graviola` does;
    //   * `linux-raw-sys` has no `build.rs`. `rustix`'s runs `$RUSTC` (or a
    //     `RUSTC_WRAPPER`) on feature probes and prints `cfg`s, the probe
    //     `docs/06-limits.md` §96 records for thirteen packages already; §96
    //     now lists fourteen. It is not a new exception and not treated as one.
    assert_eq!(
        names.len(),
        194,
        "the dependency count changed. Run the registry scan for non-Rust source \
         before re-pinning: any new crate may ship C, and DECLARED is the record."
    );
    // 0x354E_E31C_8B93_4AEA -> 0xA685_8949_AA8A_D9CE IS THE HASH WIDENING, NOT A LOCK CHANGE.
    // The old pin hashed the NAME set only, so re-enabling `ring` through
    // rustls's `ring` feature -- one new line in one `dependencies` list, every
    // name unchanged -- left it green (UC-7, UC-13). The fingerprint now covers
    // every package's dependency list too. `Cargo.lock` is byte-identical to
    // the commit before, so there is no new package to scan. D-1435.
    // 0xA685_8949_AA8A_D9CE -> 0xFDB9_D257_60EF_53D3 IS THE VERSION BEING HASHED
    // (D-1611), NOT A LOCK CHANGE: a version-only bump used to leave the pin
    // equal. `Cargo.lock` is byte-identical to the commit before, the count is
    // still 192, so there is no new package to scan.
    // 0xFDB9_D257_60EF_53D3 -> 0xAAD0_3620_6F60_10EA IS ONE NEW ARROW, NOT A NEW
    // PACKAGE (D-1510): `api` names `http-body` 1.1.0 directly to wrap a request
    // body in its deadline. The package was already in the lock under `axum` and
    // `hyper`, the count is still 192, and its registry source was scanned: only
    // `.rs`, `.md`, `.toml`, the lock and LICENSE, and no `build.rs`.
    // 0xAAD0_3620_6F60_10EA -> 0x8ACF_7E51_E4A6_F150 IS ONE NEW ARROW, NOT A NEW
    // PACKAGE (D-2801): `pull` names `ruzstd` 0.8.2 to read the GDFL tick store.
    // The package was already in the lock under `lake` with the same features,
    // the count is unchanged, and it is pure Rust with no `build.rs` (D-0056).
    // 0x8ACF_7E51_E4A6_F150 -> 0xFFDD_76EA_1ADA_B2F7 IS TWO VERSION BUMPS, NOT A
    // NEW PACKAGE (D-5090): `ruzstd` 0.8.2 -> 0.9.1 (RUSTSEC-2026-0344) and
    // `rustls` 0.23.45 -> 0.23.46 (RUSTSEC-2026-0346..0349). The count is still
    // 195 and neither dependency list moved. The scan: `ruzstd` 0.9.1 ships no
    // `build.rs` and no .c/.cc/.h/.S/.asm (its only new non-Rust files are a
    // LICENSE and three `.zst` test fixtures); `rustls` 0.23.46's file set
    // outside .rs/.md/.toml/.json equals 0.23.45's and its `build.rs` is
    // byte-identical to the one already in the tree.
    // 0xFFDD_76EA_1ADA_B2F7 -> 0x8070_52EA_72E5_1027 IS `rustix` 1.1.4 AND
    // `linux-raw-sys` 0.12.1 ARRIVING WITH THE ONE NEW ARROW `api -> rustix`
    // (D-5240), merged onto the two bumps above; on its own branch, from
    // 0x8ACF_7E51_E4A6_F150, the same two packages pinned 0x0D06_2B04_5053_8A00.
    // The scan is recorded beside the count above, and no other package's
    // `dependencies` list moved.
    // 0x8070_52EA_72E5_1027 -> 0xEF62_60E1_BE35_175C IS ONE NEW ARROW, NOT A NEW
    // PACKAGE (IA-16, CONC-031): `pull` names `rustix` 1.1.4 for the GDFL
    // import's free-space floor (one `statfs(2)`, the call `/disk.json` makes),
    // in place of the `/bin/df` child the no-runtime-spawn gate refuses. The
    // package was already in the lock under `api` with the same features
    // (`std`, `fs`), the count is unchanged, and its scan is the one D-5240
    // recorded: no .c/.cc/.h/.S/.asm, and a `build.rs` that runs only `rustc`
    // feature probes. Only `pull`'s `dependencies` list moved.
    // 0x8070_52EA_72E5_1027 -> 0x57F8_69FF_1E0D_4A5C IS TWO VERSION BUMPS, NOT A
    // NEW PACKAGE (D-7120): `rustls-webpki` 0.103.15 -> 0.103.16
    // (RUSTSEC-2026-0382) and `webpki-roots` 1.0.9 -> 1.0.10
    // (RUSTSEC-2026-0381), in `Cargo.lock` only. The count is still 194 and
    // neither `dependencies` list moved. The scan, over both registry sources
    // before and after: neither ships a `build.rs` or any
    // .c/.cc/.cpp/.h/.S/.s/.asm/.js file; `webpki-roots` 1.0.10's file set
    // equals 1.0.9's; `rustls-webpki` 0.103.16 adds only eight `.der` public
    // keys under `src/data/distrusted/`, data it compiles in with
    // `include_bytes!`. Each manifest moves only `version` (and
    // `rustls-webpki`'s `rust-version`, 1.70 -> 1.71): same features, so
    // `ring` stays off by default and the skipped `rustls-webpki -> ring` edge
    // above is still a weak one. DECLARED is unchanged.
    // R3-17 (D-6271) made the same two bumps on its own branch, to the same
    // `Cargo.lock` bytes and the same pin, with the same scan result.
    // R3-24 repair round 3 (D-5890) made the same two bumps and its own scan
    // of both registry sources independently, and reached this same pin: its
    // repair round 2 had bumped the lock without the re-pin (B10).
    // 0x57F8_69FF_1E0D_4A5C -> 0xADFB_F4EE_30AF_1851 IS THE TWO CHANGES ABOVE,
    // TOGETHER, NOT A THIRD (IA-16 repair 2, merging feat/db-fix): the two bumps
    // (D-7120) and IA-16's one arrow `pull -> rustix`. No package was added,
    // no version moved beyond D-7120's, and both scans stand as recorded.
    assert_eq!(
        h, 0xADFB_F4EE_30AF_1851,
        "the dependency SET or GRAPH changed — a package was added, removed, \
         renamed or re-versioned, or a `dependencies` list in Cargo.lock moved (a feature flip \
         that turns an optional native crate on shows up only there). \
         Scan the new set for .c/.cc/.h/.S/.asm and build.rs, update DECLARED if \
         anything ships non-Rust source, then re-pin this fingerprint. Do not \
         update the number without doing the scan; that is how `ring` got in."
    );
}

/// **§2 is satisfied without exception, and this test is what keeps it that way.**
///
/// This replaces `the_open_breach_is_named_and_not_papered_over`, which asserted
/// the opposite: that at least one declared crate was still compiled, so a green
/// suite could not imply purity while `ring` was in the tree. That test carried
/// its own retirement instruction — *"When `ring` leaves the tree, this test
/// fails and tells you to delete it, which is the moment the workspace actually
/// becomes Rust-only"* — and this is that moment, so the assertion is inverted
/// rather than deleted. Deleting it would leave the invariant unguarded in the
/// exact direction that now matters.
///
/// The `DECLARED` table stays. Every entry is still in the lockfile as an
/// unactivated optional dependency, and
/// `the_dependency_set_has_not_moved_without_review` reads it to prove a
/// declaration never outlives the crate it describes.
#[test]
fn no_declared_native_dependency_is_compiled_any_more() {
    // Read from the lock's dependency EDGES, walked from the workspace members.
    // The version this replaces filtered on a hand-written
    // `compiled_on_this_target: false` in every row, so it passed with ring
    // re-enabled and compiled (UC-7, UC-13, ET-rust-only-purity-5; D-1435).
    let lock = lock_text();
    let pkgs = parse_lock(&lock);
    let roots = pkgs.iter().filter(|p| p.workspace).count();
    assert_eq!(
        roots, 13,
        "the walk starts from the workspace members, and CLAUDE.md §5 names thirteen; \
         a walk from no roots would reach nothing and pass"
    );
    // A skipped edge that has left the lock is a stale exemption: remove it.
    for e in LOCK_ONLY_EDGES {
        assert!(
            pkgs.iter().any(|p| p.name == e.parent
                && p.deps.iter().any(|d| d.split(' ').next() == Some(e.child))),
            "LOCK_ONLY_EDGES skips `{} -> {}`, which the lock no longer records ({}); \
             delete the row",
            e.parent,
            e.child,
            e.why
        );
    }
    // A lock-format change that renamed `dependencies = [` would leave the walk
    // with no edges, and a walk over no edges reaches nothing and passes. The
    // lock records 474 edge lines as of D-1435; 300 leaves room to shrink.
    let edges: usize = pkgs.iter().map(|p| p.deps.len()).sum();
    assert!(
        edges > 300,
        "parsed only {edges} dependency edges — the lockfile format changed and \
         the reachability walk stopped testing"
    );
    let reached = declared_native_reached(&lock);
    assert!(reached.is_ok(), "{:?}", reached.as_ref().err());
    let compiled = reached.unwrap_or_default();

    // The failure text carries `ships` and `why` from the table, not just the
    // name. A reader who trips this needs to know WHAT came back and HOW it got
    // in -- "ring is compiled again" is a fact, "ring ships 17 .c ..." is an
    // instruction.
    let named: Vec<String> = compiled
        .iter()
        .map(|d| format!("{} (ships {}) — {}", d.name, d.ships, d.why))
        .collect();

    assert!(
        compiled.is_empty(),
        "native code is compiled into this workspace again:\n  {}\n\n§2 forbids \
         a vendored binding to another language and a build.rs that invokes an \
         external process, both without exception. Either remove the dependency \
         or record the breach in DECLARED and change this assertion back — but \
         do not let a green suite imply purity that is gone. See D-0211, D-1435.",
        named.join("\n  ")
    );
}

/// The declared names a synthetic lock reaches; asserts the walk did not refuse it.
fn names_reached(lock: &str) -> Vec<&'static str> {
    let r = declared_native_reached(lock).map(|v| v.iter().map(|d| d.name).collect::<Vec<_>>());
    assert!(r.is_ok(), "{r:?}");
    r.unwrap_or_default()
}

/// A synthetic lock: `app` is the one workspace member (no `source`), and the
/// caller supplies every other stanza.
fn synthetic(rest: &str) -> String {
    let src = "source = \"registry+https://github.com/rust-lang/crates.io-index\"";
    let mut lock = String::from("version = 4\n\n");
    for stanza in rest.split("\n\n").filter(|s| !s.trim().is_empty()) {
        lock.push_str("[[package]]\n");
        lock.push_str(&stanza.replace("SRC", src));
        lock.push_str("\n\n");
    }
    lock
}

const APP_REACHES_RING_THROUGH_RUSTLS: &str = "name = \"app\"\nversion = \"0.1.0\"\ndependencies = [\n \"rustls\",\n]\n\n\
name = \"rustls\"\nversion = \"0.23.45\"\nSRC\ndependencies = [\n \"ring 0.17.14\",\n \"rustls-webpki\",\n]\n\n\
name = \"rustls-webpki\"\nversion = \"0.103.15\"\nSRC\ndependencies = [\n \"ring\",\n]\n\n\
name = \"ring\"\nversion = \"0.17.14\"\nSRC\ndependencies = [\n \"cc\",\n]\n\n\
name = \"cc\"\nversion = \"1.2.0\"\nSRC";

/// **The proof the checker can fail.** A workspace member reaching `ring`
/// through `rustls`'s `ring` feature is reported, and so is the `cc` it builds
/// through. The same lock without that one dependency line — ring recorded
/// only through the weak `rustls-webpki` edge, which is today's real lock —
/// reports nothing, and the fingerprint tells the two apart.
#[test]
fn a_synthetic_lock_where_a_member_reaches_ring_is_reported() {
    let breached = synthetic(APP_REACHES_RING_THROUGH_RUSTLS);
    let names: Vec<&str> = names_reached(&breached);
    assert_eq!(names, ["ring", "cc"]);

    let clean = breached.replace(" \"ring 0.17.14\",\n", "");
    assert_ne!(clean, breached);
    assert!(names_reached(&clean).is_empty());

    // The name sets are identical; only one dependency line differs.
    let set = |t: &str| -> BTreeSet<String> { parse_lock(t).into_iter().map(|p| p.name).collect() };
    assert_eq!(set(&clean), set(&breached));
    assert_ne!(
        fingerprint(&parse_lock(&clean)),
        fingerprint(&parse_lock(&breached))
    );
    // Rerun: same input, same fingerprint (CLAUDE.md §3 rule 5).
    assert_eq!(
        fingerprint(&parse_lock(&clean)),
        fingerprint(&parse_lock(&clean))
    );
}

/// The skip list is matched by exact edge. `ring` reached from any parent other
/// than `rustls-webpki` is reported; a declared crate no member reaches is not;
/// a version-qualified entry resolves only to that version; a dangling entry is
/// corrupt input and refused by name; an empty lock reaches nothing.
#[test]
fn the_reachability_walk_skips_only_the_named_edges_and_refuses_a_dangling_one() {
    let other_parent = synthetic(
        "name = \"app\"\nversion = \"0.1.0\"\ndependencies = [\n \"tls\",\n]\n\n\
         name = \"tls\"\nversion = \"1.0.0\"\nSRC\ndependencies = [\n \"ring\",\n]\n\n\
         name = \"ring\"\nversion = \"0.17.14\"\nSRC",
    );
    let names: Vec<&str> = names_reached(&other_parent);
    assert_eq!(names, ["ring"]);

    let orphan = synthetic(
        "name = \"app\"\nversion = \"0.1.0\"\n\n\
         name = \"ring\"\nversion = \"0.17.14\"\nSRC",
    );
    assert!(names_reached(&orphan).is_empty());
    let reached = reachable_from_workspace(&parse_lock(&orphan), &[]);
    assert_eq!(
        reached.map(|r| r.into_iter().collect::<Vec<_>>()),
        Ok(vec!["app".to_owned()])
    );

    let two_versions = synthetic(
        "name = \"app\"\nversion = \"0.1.0\"\ndependencies = [\n \"shim 2.0.0\",\n]\n\n\
         name = \"shim\"\nversion = \"1.0.0\"\nSRC\ndependencies = [\n \"cc\",\n]\n\n\
         name = \"shim\"\nversion = \"2.0.0\"\nSRC\n\n\
         name = \"cc\"\nversion = \"1.2.0\"\nSRC",
    );
    assert!(names_reached(&two_versions).is_empty());
    let one = two_versions.replace("\"shim 2.0.0\"", "\"shim 1.0.0\"");
    let names: Vec<&str> = names_reached(&one);
    assert_eq!(names, ["cc"]);

    let dangling =
        synthetic("name = \"app\"\nversion = \"0.1.0\"\ndependencies = [\n \"ghost 9.9.9\",\n]");
    let err = declared_native_reached(&dangling).err().unwrap_or_default();
    assert!(err.contains("`app` depends on `ghost 9.9.9`"), "{err}");

    assert!(parse_lock("").is_empty());
    assert!(names_reached("").is_empty());
}

/// **A version-only change moves the fingerprint** (audit-20261003 rustonly2-6,
/// D-1611). The fingerprint hashed each package's name and dependency list
/// and used the version only to sort, so `ring 0.17.14` becoming `0.17.99`
/// left it equal: a new release of a crate that starts vendoring C under the
/// same name was invisible to the pin.
#[test]
fn a_version_only_bump_changes_the_fingerprint() {
    let before = synthetic(APP_REACHES_RING_THROUGH_RUSTLS);
    let after = before.replace(
        "name = \"cc\"\nversion = \"1.2.0\"",
        "name = \"cc\"\nversion = \"1.2.99\"",
    );
    assert_ne!(after, before);
    let set = |t: &str| -> BTreeSet<String> { parse_lock(t).into_iter().map(|p| p.name).collect() };
    assert_eq!(set(&after), set(&before), "the name set is unchanged");
    assert_ne!(
        fingerprint(&parse_lock(&after)),
        fingerprint(&parse_lock(&before)),
        "a version-only change must move the fingerprint"
    );
}

// ---------------------------------------------------------------------------
// Advisory floors (D-7120, BASE-REDS-1021).
// ---------------------------------------------------------------------------

/// A package a `RustSec` advisory names, and the versions that advisory clears.
///
/// `cargo deny check` (CI gate 3) reads the advisory database when it runs and
/// is the authority on what is vulnerable. This table is the offline half of
/// the two bumps D-7120 made: it holds the lock above their floors, so a merge
/// that settles a `Cargo.lock` conflict by taking the older side, or a
/// `cargo update --precise` downgrade, fails `cargo test` on a machine where
/// `cargo deny` is not installed or cannot fetch. The ranges are the advisory
/// files' own `patched` and `unaffected` lines, recorded with their digests in
/// `docs/00-charter.md`. A row is added by the decision that bumps for a new
/// advisory and is never edited to fit a lock.
struct Floor {
    package: &'static str,
    advisory: &'static str,
    /// Cleared when every comparator of any one set holds.
    patched: &'static [&'static [&'static str]],
    /// Never affected: cleared on the same rule.
    unaffected: &'static [&'static [&'static str]],
}

const FLOORS: &[Floor] = &[
    Floor {
        package: "rustls-webpki",
        advisory: "RUSTSEC-2026-0382",
        patched: &[
            &[">= 0.103.16", "< 0.104.0-alpha.1"],
            &[">= 0.104.0-alpha.8"],
        ],
        unaffected: &[],
    },
    Floor {
        package: "webpki-roots",
        advisory: "RUSTSEC-2026-0381",
        patched: &[&[">= 1.0.10"]],
        unaffected: &[&["= 0.26.11"]],
    },
];

/// One pre-release identifier. The derived order is the one semver 2.0.0 §11
/// gives: a numeric identifier sorts below an alphanumeric one, numbers by
/// value, the rest byte by byte in ASCII.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum PreIdent {
    Num(u64),
    Alpha(String),
}

/// A version as precedence sees it: build metadata is read and dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Semver {
    core: (u64, u64, u64),
    pre: Vec<PreIdent>,
}

impl Ord for Semver {
    /// A release sorts above every pre-release of its own `MAJOR.MINOR.PATCH`;
    /// two pre-releases compare identifier by identifier, and a shorter list
    /// that is a prefix of a longer one sorts first (`Vec`'s own order).
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering;
        self.core
            .cmp(&other.core)
            .then_with(|| match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            })
    }
}

impl PartialOrd for Semver {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// A numeric part: digits only, no leading zero, fits `u64`.
fn semver_number(part: &str, whole: &str) -> Result<u64, String> {
    if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("`{whole}`: `{part}` is not a number"));
    }
    if part.len() > 1 && part.starts_with('0') {
        return Err(format!("`{whole}`: `{part}` has a leading zero"));
    }
    part.parse()
        .map_err(|_| format!("`{whole}`: `{part}` does not fit in 64 bits"))
}

/// A dot-separated identifier list: none empty, each `[0-9A-Za-z-]+`.
fn semver_idents(list: &str) -> bool {
    list.split('.')
        .all(|g| !g.is_empty() && g.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'))
}

/// Parses `MAJOR.MINOR.PATCH[-PRE][+BUILD]`. Anything else is an `Err` naming
/// the text: a version this test cannot read is never taken as cleared.
fn parse_semver(text: &str) -> Result<Semver, String> {
    let body = match text.split_once('+') {
        Some((body, build)) if semver_idents(build) => body,
        Some(_) => return Err(format!("`{text}`: the build tag is empty or malformed")),
        None => text,
    };
    let (core, pre) = match body.split_once('-') {
        Some((core, pre)) => (core, Some(pre)),
        None => (body, None),
    };
    let mut parts = core.split('.');
    let (Some(major), Some(minor), Some(patch), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(format!("`{text}` is not MAJOR.MINOR.PATCH"));
    };
    let core = (
        semver_number(major, text)?,
        semver_number(minor, text)?,
        semver_number(patch, text)?,
    );
    let mut idents = Vec::new();
    if let Some(pre) = pre {
        if !semver_idents(pre) {
            return Err(format!("`{text}`: the pre-release is empty or malformed"));
        }
        for g in pre.split('.') {
            idents.push(if g.bytes().all(|b| b.is_ascii_digit()) {
                PreIdent::Num(semver_number(g, text)?)
            } else {
                PreIdent::Alpha(g.to_owned())
            });
        }
    }
    Ok(Semver { core, pre: idents })
}

/// One comparator as the advisory files write them: `>= v`, `< v` or `= v`.
/// Any other operator is refused by name rather than guessed at.
fn comparator_holds(comparator: &str, version: &Semver) -> Result<bool, String> {
    if let Some(bound) = comparator.strip_prefix(">= ") {
        Ok(*version >= parse_semver(bound)?)
    } else if let Some(bound) = comparator.strip_prefix("< ") {
        Ok(*version < parse_semver(bound)?)
    } else if let Some(bound) = comparator.strip_prefix("= ") {
        Ok(*version == parse_semver(bound)?)
    } else {
        Err(format!(
            "comparator `{comparator}` is not one this test reads (`>= `, `< `, `= `)"
        ))
    }
}

/// Does any one set hold whole? Every comparator is evaluated, so a malformed
/// one is refused even when an earlier set already cleared the version.
fn any_set_holds(sets: &[&[&str]], version: &Semver) -> Result<bool, String> {
    let mut cleared = false;
    for set in sets {
        let mut all = true;
        for comparator in *set {
            all &= comparator_holds(comparator, version)?;
        }
        cleared |= all;
    }
    Ok(cleared)
}

/// Whether `floor`'s advisory clears `version`. A pre-release is refused, not
/// judged: cargo matches a pre-release only against a comparator on its own
/// `MAJOR.MINOR.PATCH`, which plain precedence does not model.
fn clears(floor: &Floor, version: &str) -> Result<bool, String> {
    let parsed = parse_semver(version)?;
    if !parsed.pre.is_empty() {
        return Err(format!(
            "`{version}` is a pre-release, which this test does not judge: read {} \
             for it and extend the test",
            floor.advisory
        ));
    }
    Ok(any_set_holds(floor.patched, &parsed)? || any_set_holds(floor.unaffected, &parsed)?)
}

/// Every locked copy of a floored package its advisory does not clear, and
/// every copy whose version cannot be read, each named. Every copy is judged,
/// not only the first: a lock may hold one package at two versions.
fn below_floor(pkgs: &[LockPackage], floors: &[Floor]) -> Vec<String> {
    let mut out = Vec::new();
    for floor in floors {
        for pkg in pkgs.iter().filter(|p| p.name == floor.package) {
            match clears(floor, &pkg.version) {
                Ok(true) => {}
                Ok(false) => out.push(format!(
                    "{} {} is inside {}: patched {:?}, unaffected {:?}",
                    pkg.name, pkg.version, floor.advisory, floor.patched, floor.unaffected
                )),
                Err(why) => out.push(format!(
                    "{} {:?} cannot be judged against {}, so it is refused: {why}",
                    pkg.name, pkg.version, floor.advisory
                )),
            }
        }
    }
    out
}

/// Floored packages the lock holds no copy of. A floor over a package that is
/// gone checks nothing, so its absence is named rather than passed.
fn floors_not_held<'a>(pkgs: &[LockPackage], floors: &'a [Floor]) -> Vec<&'a str> {
    floors
        .iter()
        .filter(|f| !pkgs.iter().any(|p| p.name == f.package))
        .map(|f| f.package)
        .collect()
}

/// **The real lock is above both floors**, and holds both packages. It fails
/// on `feat/db-fix` at 8c67456ac (`rustls-webpki` 0.103.15, `webpki-roots`
/// 1.0.9) and passes after D-7120's bump.
#[test]
fn the_lock_is_above_every_advisory_floor() {
    let pkgs = parse_lock(&lock_text());
    let below = below_floor(&pkgs, FLOORS);
    assert!(
        below.is_empty(),
        "Cargo.lock holds versions a RustSec advisory names (D-7120). Bump them \
         with `cargo update -p <name>`, never by editing this table: {below:#?}"
    );
    let gone = floors_not_held(&pkgs, FLOORS);
    assert!(
        gone.is_empty(),
        "the lock no longer holds {gone:?}, so its floor checks nothing. Remove the \
         row in the change that dropped the package, with a decision"
    );
}

/// The versions D-7120 bumped from are refused, each naming its advisory, and
/// the versions it bumped to are cleared. Fields in either order are read.
#[test]
fn the_floors_refuse_the_versions_d7120_bumped_from() {
    let before = synthetic(
        "name = \"rustls-webpki\"\nversion = \"0.103.15\"\nSRC\n\n\
         version = \"1.0.9\"\nname = \"webpki-roots\"\nSRC",
    );
    let below = below_floor(&parse_lock(&before), FLOORS);
    assert_eq!(below.len(), 2, "{below:#?}");
    assert!(
        below
            .iter()
            .any(|b| b.starts_with("rustls-webpki 0.103.15 is inside RUSTSEC-2026-0382"))
    );
    assert!(
        below
            .iter()
            .any(|b| b.starts_with("webpki-roots 1.0.9 is inside RUSTSEC-2026-0381"))
    );

    let after = before
        .replace("0.103.15", "0.103.16")
        .replace("1.0.9", "1.0.10");
    assert!(below_floor(&parse_lock(&after), FLOORS).is_empty());
    assert!(floors_not_held(&parse_lock(&after), FLOORS).is_empty());
}

/// Each range is cut exactly at its published bounds, the older lines below
/// them are refused, and the one unaffected compatibility version is cleared.
#[test]
fn each_range_is_cut_at_its_published_bounds() {
    let judge = |package: &str, version: &str| -> Result<bool, String> {
        let floor = FLOORS.iter().find(|f| f.package == package);
        floor.map_or_else(
            || Err(format!("no floor for {package}")),
            |f| clears(f, version),
        )
    };
    for (version, cleared) in [
        ("0.0.0", false),
        ("0.102.8", false),
        ("0.103.0", false),
        ("0.103.15", false),
        ("0.103.16", true),
        ("0.103.999", true),
        ("0.104.0", true),
        ("0.104.1", true),
        ("1.0.0", true),
    ] {
        assert_eq!(
            judge("rustls-webpki", version),
            Ok(cleared),
            "rustls-webpki {version}"
        );
    }
    for (version, cleared) in [
        ("0.26.10", false),
        ("0.26.11", true),
        ("0.26.11+compat", true),
        ("0.26.12", false),
        ("1.0.0", false),
        ("1.0.9", false),
        ("1.0.10", true),
        ("1.1.0", true),
        ("2.0.0", true),
    ] {
        assert_eq!(
            judge("webpki-roots", version),
            Ok(cleared),
            "webpki-roots {version}"
        );
    }
    assert!(
        judge("webpki", "0.22.4").is_err(),
        "an unfloored name is not judged"
    );
}

/// Precedence is semver 2.0.0 §11's: its own example chain, the bounds this
/// table uses in order, and build metadata dropped.
#[test]
fn semver_precedence_is_the_specs() {
    let chain = [
        "1.0.0-alpha",
        "1.0.0-alpha.1",
        "1.0.0-alpha.beta",
        "1.0.0-beta",
        "1.0.0-beta.2",
        "1.0.0-beta.11",
        "1.0.0-rc.1",
        "1.0.0",
        "1.0.1",
        "1.1.0",
        "2.0.0",
    ];
    let parsed: Result<Vec<Semver>, String> = chain.iter().map(|v| parse_semver(v)).collect();
    assert!(parsed.is_ok(), "{parsed:?}");
    let parsed = parsed.unwrap_or_default();
    assert_eq!(parsed.len(), chain.len());
    for pair in parsed.windows(2) {
        if let [a, b] = pair {
            assert!(a < b, "{a:?} must sort below {b:?}");
        }
    }
    let bounds: Result<Vec<Semver>, String> =
        ["0.103.16", "0.104.0-alpha.1", "0.104.0-alpha.8", "0.104.0"]
            .iter()
            .map(|v| parse_semver(v))
            .collect();
    let bounds = bounds.unwrap_or_default();
    assert_eq!(bounds.len(), 4);
    for pair in bounds.windows(2) {
        if let [a, b] = pair {
            assert!(a < b, "{a:?} must sort below {b:?}");
        }
    }
    assert_eq!(
        parse_semver("0.11.1+wasi-snapshot-preview1"),
        parse_semver("0.11.1"),
        "build metadata has no precedence"
    );
}

/// A version this test cannot read is refused by name and never cleared:
/// empty, short, long, non-numeric, a leading zero, an overflow, an empty
/// pre-release or build tag, a stray dot, a pre-release, and a stanza with no
/// version line at all.
#[test]
fn a_version_it_cannot_read_is_refused_by_name_never_cleared() {
    for garbage in [
        "",
        "1.0",
        "1.0.10.1",
        "1.0.x",
        "01.0.10",
        "1.00.10",
        "1..10",
        "-1.0.10",
        "1.0.10-",
        "1.0.10+",
        "1.0.10-alpha..1",
        "1.0.99999999999999999999",
        " 1.0.10",
    ] {
        let parsed = parse_semver(garbage);
        assert!(parsed.is_err(), "`{garbage}` must not parse: {parsed:?}");
        let lock = synthetic(&format!(
            "name = \"webpki-roots\"\nversion = \"{garbage}\"\nSRC"
        ));
        let below = below_floor(&parse_lock(&lock), FLOORS);
        assert_eq!(below.len(), 1, "`{garbage}`: {below:?}");
        assert!(
            below
                .iter()
                .all(|b| b.contains("cannot be judged against RUSTSEC-2026-0381")),
            "{below:?}"
        );
    }
    let pre = synthetic("name = \"rustls-webpki\"\nversion = \"0.104.0-alpha.9\"\nSRC");
    let below = below_floor(&parse_lock(&pre), FLOORS);
    assert_eq!(below.len(), 1, "{below:?}");
    assert!(
        below.iter().all(|b| b.contains("is a pre-release")),
        "{below:?}"
    );

    let no_version = synthetic("name = \"rustls-webpki\"\nSRC");
    let below = below_floor(&parse_lock(&no_version), FLOORS);
    assert_eq!(below.len(), 1, "{below:?}");
    assert!(
        below
            .iter()
            .all(|b| b.contains("rustls-webpki \"\" cannot be judged")),
        "{below:?}"
    );
}

/// Every copy is judged, in either order, and a name that merely starts with a
/// floored one is not the floored package.
#[test]
fn every_locked_copy_is_judged_not_only_the_first() {
    for stanzas in [
        "name = \"webpki-roots\"\nversion = \"1.0.10\"\nSRC\n\n\
         name = \"webpki-roots\"\nversion = \"0.26.10\"\nSRC",
        "name = \"webpki-roots\"\nversion = \"0.26.10\"\nSRC\n\n\
         name = \"webpki-roots\"\nversion = \"1.0.10\"\nSRC",
    ] {
        let below = below_floor(&parse_lock(&synthetic(stanzas)), FLOORS);
        assert_eq!(below.len(), 1, "{below:?}");
        assert!(
            below
                .iter()
                .all(|b| b.starts_with("webpki-roots 0.26.10 is inside")),
            "{below:?}"
        );
    }
    let relatives = synthetic(
        "name = \"rustls-webpki-extra\"\nversion = \"0.1.0\"\nSRC\n\n\
         name = \"webpki\"\nversion = \"0.22.4\"\nSRC",
    );
    let pkgs = parse_lock(&relatives);
    assert!(below_floor(&pkgs, FLOORS).is_empty());
    assert_eq!(
        floors_not_held(&pkgs, FLOORS),
        ["rustls-webpki", "webpki-roots"]
    );
}

/// An empty lock holds nothing below a floor, and that is named as both floors
/// checking nothing rather than passed in silence.
#[test]
fn an_empty_lock_is_named_as_checking_nothing() {
    let pkgs = parse_lock("");
    assert!(pkgs.is_empty());
    assert!(below_floor(&pkgs, FLOORS).is_empty());
    assert_eq!(
        floors_not_held(&pkgs, FLOORS),
        ["rustls-webpki", "webpki-roots"]
    );
}

/// Every comparator in the table is one this test reads, so a typo in a row
/// is refused here rather than read as a set that never holds; an operator it
/// does not read is refused by name.
#[test]
fn every_comparator_in_the_table_is_read_and_others_are_refused() {
    let probe = parse_semver("1.0.0");
    assert!(probe.is_ok(), "{probe:?}");
    let Ok(probe) = probe else { return };
    let mut read = 0;
    for floor in FLOORS {
        for set in floor.patched.iter().chain(floor.unaffected) {
            for comparator in *set {
                let held = comparator_holds(comparator, &probe);
                assert!(held.is_ok(), "{}: {comparator}: {held:?}", floor.advisory);
                read += 1;
            }
        }
    }
    assert_eq!(read, 5, "the table holds five comparators");
    for foreign in [
        "> 1.0.0", "<= 1.0.0", "~1.0", "^1", "1.0.0", ">=1.0.0", "= 1.0",
    ] {
        assert!(
            comparator_holds(foreign, &probe).is_err(),
            "`{foreign}` must be refused"
        );
    }
}
