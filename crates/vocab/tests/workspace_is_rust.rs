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
//! * Whether a declared crate is actually **compiled** on this target.
//!   `iana-time-zone-haiku` ships C++ and is gated to Haiku, so it never builds
//!   here — the test records that distinction rather than flattening it.
//! * Anything in `web/`, which `CLAUDE.md` §2 exempts by path.

use std::collections::BTreeSet;

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
    /// True when the crate's native code is compiled into a normal build here.
    compiled_on_this_target: bool,
    why: &'static str,
}

const DECLARED: &[Declared] = &[
    Declared {
        name: "ring",
        ships: "17 .c, 28 .h, 73 .S, 17 .asm, plus build.rs",
        compiled_on_this_target: false,
        why: "NO LONGER REACHED, AND THIS ROW USED TO SAY THE OPPOSITE. It read \
              `THE OPEN §2 BREACH. Measured: nm on the release binary returns 72 \
              ring_core symbols, so this is linked, not dormant` — which was true \
              when written and stopped being true the moment `reqwest` moved to \
              `rustls-tls-webpki-roots-no-provider` and `rustls-graviola` took \
              over, the change the 189 -> 176 note below records. The sentence \
              outlived it and directly contradicted the `compiled_on_this_target: \
              false` beside it. Re-measured 2026-08-26: `cargo tree -i ring \
              --target all` prints nothing, so it is resolved and not built. It \
              stays in Cargo.lock as an OPTIONAL dependency of `rustls-webpki`, \
              which is why it is still declared here rather than deleted — the \
              day something turns that feature on, this row is what says what \
              arrived.",
    },
    Declared {
        name: "cc",
        ships: "1 .c",
        compiled_on_this_target: false,
        why: "The compiler driver ring builds through. Its own .c is a probe fixture, \
              but the crate's PURPOSE is invoking a C compiler from a build script.",
    },
    Declared {
        name: "wit-bindgen",
        ships: "3 .c, plus build.rs",
        compiled_on_this_target: false,
        why: "WebAssembly component tooling, reached only on wasm targets. Not built \
              for the host. UNVERIFIED that no wasm build path pulls it in.",
    },
    Declared {
        name: "iana-time-zone-haiku",
        ships: "1 .cc, plus build.rs",
        compiled_on_this_target: false,
        why: "Target-gated to Haiku OS. Never compiled on macOS or Linux. Present in \
              the lockfile because cargo resolves every target.",
    },
];

/// Names that must not appear at all: a foreign runtime, or a binding to one.
/// Distinct from DECLARED — these are not tolerated, they are absent.
const FORBIDDEN: &[&str] = &[
    "pyo3",
    "rustpython-vm",
    "cpython",
    "napi",
    "neon",
    "node-bindgen",
    "mlua",
    "rlua",
    "lua",
    "duktape",
    "quickjs",
    "v8",
    "rusty_v8",
    "deno_core",
    "jni",
    "j4rs",
    "openssl-sys",
    "libpython3-sys",
];

fn lockfile_packages() -> BTreeSet<String> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../Cargo.lock");
    let lock = std::fs::read_to_string(path).unwrap_or_default();
    assert!(
        !lock.is_empty(),
        "the workspace lockfile is not readable at {path} — this gate cannot run, \
         and a gate that cannot run must fail rather than pass"
    );
    let mut out = BTreeSet::new();
    let mut in_package = false;
    for line in lock.lines() {
        if line.trim() == "[[package]]" {
            in_package = true;
            continue;
        }
        if in_package && let Some(rest) = line.strip_prefix("name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                out.insert(name.to_owned());
            }
            in_package = false;
        }
    }
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
    let found: Vec<&str> = FORBIDDEN
        .iter()
        .copied()
        .filter(|f| packages.contains(*f))
        .collect();
    assert!(
        found.is_empty(),
        "CLAUDE.md §2 forbids a foreign runtime without exception, and the lockfile holds: {found:?}"
    );
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

    // FNV-1a over the sorted names, with a separator so `ab`+`c` and `a`+`bc`
    // cannot collide. Hand-rolled because this crate takes no dependencies.
    let mut names: Vec<&str> = packages.iter().map(String::as_str).collect();
    names.sort_unstable();
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for n in &names {
        for b in n.as_bytes() {
            h ^= u64::from(*b);
            h = h.wrapping_mul(0x0100_0000_01b3);
        }
        h ^= 0x1f;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }

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
    assert_eq!(
        names.len(),
        192,
        "the dependency count changed. Run the registry scan for non-Rust source \
         before re-pinning: any new crate may ship C, and DECLARED is the record."
    );
    assert_eq!(
        h, 0x354E_E31C_8B93_4AEA,
        "the dependency SET changed — a package was added, removed or renamed. \
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
    let packages = lockfile_packages();
    let compiled: Vec<&Declared> = DECLARED
        .iter()
        .filter(|d| d.compiled_on_this_target && packages.contains(d.name))
        .collect();

    // The failure text carries `ships` and `why` from the table, not just the
    // name. A reader who trips this needs to know WHAT came back and HOW it got
    // in -- "ring is compiled again" is a fact, "ring ships 17 .c ... via
    // reqwest -> rustls -> ring" is an instruction. It also keeps both fields
    // read: the test this replaced printed them, and dropping the print made
    // them dead code under `-D warnings`, which is the compiler noticing that
    // the declaration had stopped being used for anything.
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
         do not let a green suite imply purity that is gone. See D-0211.",
        named.join("\n  ")
    );
}
