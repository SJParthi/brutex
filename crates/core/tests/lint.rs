//! The lint that keeps a float away from a price, checked rather than assumed.
//!
//! # Why this is a source check, and why that is the honest shape
//!
//! `CLAUDE.md` §7 says prices are paisa integers and never a float. Nothing
//! about a *type* can state that: `Paisa` holds an `i64` and always did, and
//! [`brutex_core::price`]'s own tests already prove the arithmetic. What was
//! never checked is the thing that keeps the SECOND float path from arriving —
//! `clippy::float_arithmetic` — and invariant X-02 named a test for it that
//! existed in no file.
//!
//! It turned out the lint did not say what six comments in this workspace said
//! it said. `Cargo.toml` carried `float_arithmetic = "warn"`, so the rule held
//! only because CI passes `-D warnings`; a `warn` in the table and a `deny` in
//! the prose is a rule that is one flag away from not being one. D-0061
//! tightens it, and this file is what stops it drifting back — delete the line
//! or soften it to `warn` and `cargo test` fails, which is a shorter loop than
//! waiting for a CI flag to notice.
//!
//! # What this proves, and what it does not
//!
//! It proves three things about the source of `crates/core`, which is the crate
//! that owns the price type:
//!
//!   1. the workspace lint table **denies** `clippy::float_arithmetic`;
//!   2. exactly one `#[allow]` overrides it, it is item-level rather than
//!      module-wide, and it sits on `Paisa::from_rupees_half_up`;
//!   3. every float named anywhere else in this crate is handed straight to
//!      that one conversion.
//!
//! It does **not** prove the same of the other seven crates. A Rust test cannot
//! walk a tree without depending on the directory it was run from, and
//! `include_str!` reaches only paths written down here. What it does instead is
//! close the list: the modules scanned below are checked against the `pub mod`
//! lines in `lib.rs`, so a new module in this crate is a failing test until
//! someone adds it here and looks at it. `docs/06-limits.md` records the rest.
//!
//! `crates/greeks` holds four module-wide allows for the same lint and they are
//! not a violation of anything: a delta of `0.00017142680429549402` is a
//! statistical value, and `CLAUDE.md` §7 keeps those at full precision on
//! purpose. The line this file defends is between a PRICE and a statistic, not
//! between an integer and a float.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

/// The workspace manifest, which is where the lint either is or is not denied.
const MANIFEST: &str = include_str!("../../../Cargo.toml");

/// The crate root, so the module list below can be checked against it.
const LIB: &str = include_str!("../src/lib.rs");

/// The one module allowed to name a float, and its text.
const PRICE: &str = include_str!("../src/price.rs");

/// Every other module this crate declares.
///
/// Listed by hand because `include_str!` takes a literal, and checked against
/// `lib.rs` by [`the_module_list_is_the_whole_crate`] so the hand-written part
/// cannot fall behind the crate.
const OTHERS: [(&str, &str); 7] = [
    ("blake3.rs", include_str!("../src/blake3.rs")),
    ("error.rs", include_str!("../src/error.rs")),
    ("instrument.rs", include_str!("../src/instrument.rs")),
    ("isin.rs", include_str!("../src/isin.rs")),
    ("symbol.rs", include_str!("../src/symbol.rs")),
    ("universe.rs", include_str!("../src/universe.rs")),
    ("vendor.rs", include_str!("../src/vendor.rs")),
];

/// The one conversion a float in this crate is allowed to be on its way to.
const THE_BOUNDARY: &str = "from_rupees_half_up";

/// **X-02: a price never touches a float, and the lint that says so is a
/// `deny`.**
///
/// Three assertions, in the order a reader should meet them: the rule exists,
/// exactly one thing is excepted from it, and nothing else in the crate holds a
/// float that is not on its way to that exception.
#[test]
fn no_float_in_price() {
    // 1. THE RULE. Read out of the lint table rather than trusted, because the
    //    table is an ordinary tracked file and a one-word edit disarms it
    //    across every crate at once with every other gate still green.
    let table = MANIFEST
        .split_once("[workspace.lints.clippy]")
        .expect("the workspace denies lints in one table")
        .1;
    let table = &table[..table.find("\n[").unwrap_or(table.len())];
    let line = table
        .lines()
        .find(|l| l.trim_start().starts_with("float_arithmetic"))
        .expect(
            "clippy::float_arithmetic is the only mechanism that stops a \
             second float path from compiling silently; a table without it is \
             CLAUDE.md section 7 enforced by nothing",
        );
    assert!(
        line.contains("\"deny\""),
        "float_arithmetic must be DENIED, not warned. A warn holds only while \
         CI passes -D warnings, and six comments in this workspace already \
         tell the reader it is denied: {line}",
    );

    // 2. THE ONE EXCEPTION. The vendor sends rupees as an IEEE double, so
    //    something has to accept one; the whole design is that exactly one
    //    thing does.
    assert_eq!(
        PRICE.matches("allow(clippy::float_arithmetic)").count(),
        1,
        "a second allow in price.rs is a second float path, and it must be a \
         visible addition rather than a quiet one",
    );
    let (before, after) = PRICE
        .split_once("allow(clippy::float_arithmetic)")
        .expect("counted one above");
    assert!(
        before.ends_with("#["),
        "the allow must be item-level. A module-wide `#![allow]` here would \
         except the whole file, which is every future function in it too",
    );
    let next_fn = after
        .split_once("fn ")
        .expect("an allow sits on a function")
        .1;
    assert!(
        next_fn.starts_with(THE_BOUNDARY),
        "the exception belongs to {THE_BOUNDARY} and to nothing else; it now \
         sits on: {}",
        &next_fn[..next_fn.find('(').unwrap_or(0)],
    );

    // 3. EVERY OTHER FLOAT IN THE CRATE. There is one — `parse_strike` reads a
    //    vendor's rupee strike — and it does no arithmetic: it parses and hands
    //    the value straight to the conversion above. That is the shape any
    //    float here must have, so it is asserted as a shape rather than as a
    //    count that would have to be edited every time one is added.
    for (name, text) in OTHERS {
        assert!(
            !text.contains("allow(clippy::float_arithmetic)"),
            "{name}: only the boundary conversion is excepted from the lint",
        );
        // COMMENTS ARE STRIPPED FIRST, and this is the fourth guard in this workspace to
        // need that line. `contains` reads TEXT, and text in a comment is text -- so a doc
        // comment on `PriceError::NotDecimal` mentioning the float type failed this test
        // while introducing no float at all. The same defect was just fixed in
        // `vocab::mask::hits_does_the_same_work_for_every_input`,
        // `engine::the_sweep_cannot_compute_a_condition_bit` and gate 22 clause A.
        //
        // Everything from the first `//` to the end of the line goes, which is exact here:
        // `core` has no string literal containing `//`.
        // AND STRING LITERALS GO TOO, WHICH IS THE SAME LESSON ONE STEP FURTHER.
        //
        // The paragraph above records that text in a COMMENT is text. Text in a
        // STRING is text for exactly the same reason, and it arrived exactly the
        // same way: `blake3.rs` carries the published BLAKE3 vectors as 64-character
        // hex literals, and the digest of the empty input ends `...cae41f3262`,
        // which contains `f32`. That line named no float, performed no arithmetic
        // and could never reach `from_rupees_half_up` -- there is nothing there to
        // be on its way anywhere.
        //
        // Stripping is the fix rather than an exception list, because an exception
        // list would have to name `blake3.rs` and would then stop scanning the very
        // file it excused. This still refuses `let x: f32` anywhere; it only stops
        // reading the inside of a quoted literal as code.
        let lines: Vec<String> = text
            .lines()
            .map(|l| l.split_once("//").map_or(l, |(code, _)| code))
            .map(|code| {
                let mut out = String::with_capacity(code.len());
                let mut inside = false;
                let mut escaped = false;
                for c in code.chars() {
                    if escaped {
                        escaped = false;
                    } else if inside && c == '\\' {
                        escaped = true;
                    } else if c == '"' {
                        inside = !inside;
                    } else if !inside {
                        out.push(c);
                    }
                }
                out
            })
            .collect();
        for (i, line) in lines.iter().enumerate() {
            if !line.contains("f64") && !line.contains("f32") {
                continue;
            }
            let reaches = lines[i..lines.len().min(i + 3)]
                .iter()
                .any(|l| l.contains(THE_BOUNDARY));
            assert!(
                reaches,
                "{name}:{}: a float in this crate is a value on its way to \
                 {THE_BOUNDARY} and nothing else. This one is not handed to it \
                 within two lines, so it is either arithmetic or a price that \
                 stayed a float: {line}",
                i + 1,
            );
        }
    }
}

/// The hand-written module list above is the whole crate, not most of it.
///
/// Without this, adding `pub mod pivot;` to `lib.rs` would silently be a module
/// [`no_float_in_price`] never opens — and a scan that quietly stops covering
/// what it claims to cover is the failure this repository's gate 10 exists to
/// catch. The failure is deliberately loud and says which name is missing.
#[test]
fn the_module_list_is_the_whole_crate() {
    let declared: Vec<String> = LIB
        .lines()
        .filter_map(|l| l.trim().strip_prefix("pub mod "))
        .filter_map(|l| l.strip_suffix(';'))
        .map(|m| format!("{m}.rs"))
        .collect();
    assert!(
        !declared.is_empty(),
        "the `pub mod` pattern stopped matching, so this check measures \
         nothing — a silent zero is not a pass",
    );

    let scanned: Vec<&str> = OTHERS
        .iter()
        .map(|(n, _)| *n)
        .chain(std::iter::once("price.rs"))
        .collect();

    for module in &declared {
        assert!(
            scanned.contains(&module.as_str()),
            "{module} is declared in lib.rs and no_float_in_price never opens \
             it. Add it to OTHERS and read it first.",
        );
    }
    for module in &scanned {
        assert!(
            declared.iter().any(|d| d == module),
            "{module} is scanned and lib.rs no longer declares it",
        );
    }
}
