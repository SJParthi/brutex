//! `docs/02-store-format.md` against the code it describes.
//!
//! An engine-truth audit found four sentences in that document, and one in
//! `BarFile::read_record`'s own doc, that the code had stopped supporting:
//! reads through a read-only memory mapping, a header that outran its data
//! falling back a generation, no bench timing a syscall, and C-07 enforcing
//! per-read verification. None of them was read by a test, so each one drifted
//! in silence. Each test below reads the text AND the fact it names. D-0790.

/// The format document.
const FORMAT_DOC: &str = include_str!("../../../docs/02-store-format.md");
/// The limits document, which repeated the syscall sentence.
const LIMITS_DOC: &str = include_str!("../../../docs/06-limits.md");
/// The one module that opens, reads and writes a bar file.
const FILE_RS: &str = include_str!("../src/file.rs");
/// The crate root, where `unsafe` is forbidden.
const LIB_RS: &str = include_str!("../src/lib.rs");
/// This crate's manifest.
const MANIFEST: &str = include_str!("../Cargo.toml");
/// The ratio bench that carries C-07, C-28 and C-29.
const BENCH: &str = include_str!("../benches/ratio.rs");
/// The integration tests the corrected §5 cites.
const WRITE_TESTS: &str = include_str!("write.rs");

/// Collapse every run of whitespace to one space, so a sentence is found
/// whatever column the document wrapped it at.
fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text of the `## N.` section: from its heading to the next `## `.
fn section(number: &str) -> String {
    let heading = format!("\n## {number}. ");
    let (_, rest) = FORMAT_DOC
        .split_once(heading.as_str())
        .unwrap_or_else(|| panic!("docs/02-store-format.md has no `## {number}.` section"));
    let body = rest.split("\n## ").next().unwrap_or(rest);
    flat(body)
}

#[test]
fn the_helpers_find_a_section_and_flatten_its_wrapping() {
    assert_eq!(flat(" a\n  b\tc "), "a b c");
    let four = section("4");
    assert!(
        four.starts_with("Reading"),
        "§4 is the reading section, read from its heading: {four:.60}"
    );
    assert!(
        !four.contains("## 5."),
        "a section stops at the next heading"
    );
}

/// **§4 names the read the store performs: one positional read, no mapping.**
#[test]
fn section_4_names_the_positional_read_and_claims_no_mapping() {
    // The facts, from the code.
    assert!(
        FILE_RS.contains("FileExt::read_at(self, buf, offset)"),
        "the read path is one positional read"
    );
    assert!(
        LIB_RS.contains("#![forbid(unsafe_code)]"),
        "the crate forbids unsafe code, which a memory mapping needs"
    );
    for (name, text) in [("file.rs", FILE_RS), ("Cargo.toml", MANIFEST)] {
        for word in ["memmap", "Mmap", "mmap("] {
            assert!(
                !text.contains(word),
                "{name} names `{word}`: a mapping arrived, and §4 must say so"
            );
        }
    }

    // The document, against them.
    let four = section("4");
    for stale in [
        "The file is mapped **read-only**",
        "pointer arithmetic against resident pages",
        "Reading — read-only mapping",
    ] {
        assert!(
            !four.contains(stale),
            "§4 still says `{stale}`, and no crate maps a file: {four:.200}"
        );
    }
    for fact in ["FileExt::read_at", "`pread`", "#![forbid(unsafe_code)]"] {
        assert!(
            four.contains(fact),
            "§4 must name `{fact}`, the read the store performs: {four:.200}"
        );
    }
}

/// **§5 says a header ahead of its data is REFUSED, and cites the tests.**
#[test]
fn section_5_says_a_header_ahead_of_its_data_is_refused() {
    let five = section("5");
    assert!(
        !five.contains("the reader falls back to the previous generation rather than refusing"),
        "§5 still promises the fallback `BarFile::validated` refuses"
    );
    assert!(
        five.contains("`FormatError::CounterExceedsFile`"),
        "§5 must name the refusal every open door returns"
    );
    for test in [
        "a_truncation_back_to_the_header_is_refused_rather_than_silently_accepted",
        "a_counter_behind_its_bytes_opens_and_a_counter_ahead_of_them_is_refused",
    ] {
        assert!(five.contains(test), "§5 must cite `store::write::{test}`");
        assert!(
            WRITE_TESTS.contains(&format!("fn {test}()")),
            "§5 cites `{test}`, which must exist in crates/store/tests/write.rs"
        );
    }
    // The file.rs comment that said the same thing through a test name that
    // was renamed when its assertion was inverted.
    assert!(
        !flat(FILE_RS).contains("is that file and it still opens"),
        "`BarFile::validated`'s comment still says an exact-extent truncation opens"
    );
}

/// **No document says no bench times a syscall: C-28 and C-29 time
/// `read_record`, whose read is a `pread`.**
#[test]
fn no_document_says_no_bench_times_a_syscall() {
    assert!(
        BENCH.contains("file.read_record(black_box(9_999))"),
        "C-29 times `read_record` end to end"
    );
    assert!(
        BENCH.contains("black_box(f).read_record(black_box(0))"),
        "C-28 times `read_record` end to end"
    );
    for (name, text) in [
        ("crates/store/src/file.rs", FILE_RS),
        ("docs/06-limits.md", LIMITS_DOC),
        ("docs/02-store-format.md", FORMAT_DOC),
    ] {
        let text = flat(text).replace("/// ", "");
        assert!(
            !text.contains("no bench in this repository times a syscall"),
            "{name} still says no bench times a syscall"
        );
    }
}

/// **§6 does not cite C-07 for the read path's verification.** C-07 seals a
/// block held in memory; the read path verifies through `verify_block_of`.
#[test]
fn section_6_does_not_cite_c07_for_per_read_verification() {
    assert!(
        BENCH.contains("block::seal(Layout::V2, n_valid, 0, black_box(&bytes))"),
        "C-07 times `block::seal` over an in-memory buffer"
    );
    assert!(
        FILE_RS.contains("fn verify_block_of(&self, index: u64)"),
        "the read path verifies through `verify_block_of`"
    );
    let six = section("6");
    assert!(
        !six.contains("Verification is O(1) per read: one block CRC, not a file scan. Enforced by"),
        "§6 still says C-07 enforces the per-read bound"
    );
    for fact in ["`block::seal`", "`verify_block_of`"] {
        assert!(six.contains(fact), "§6 must name {fact}");
    }
}

/// The module that decodes and commits header slots.
const HEADER_RS: &str = include_str!("../src/header.rs");
/// The invariants document, whose C-01 paragraph is append-only.
const INVARIANTS_DOC: &str = include_str!("../../../docs/04-invariants.md");
/// The decisions ledger.
const DECISIONS_DOC: &str = include_str!("../../../docs/05-decisions.md");

/// **`header.rs` does not head its module doc with a mapping no build makes.**
#[test]
fn the_header_module_doc_claims_no_mapping() {
    let doc: String = HEADER_RS
        .lines()
        .filter_map(|line| line.strip_prefix("//!"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !doc.contains("# It is still a read-only mapping plus `pwrite`"),
        "header.rs still heads a section with a read-only mapping"
    );
    assert!(
        doc.contains("# Positional reads plus `pwrite`, and no mapping"),
        "header.rs must head that section with the read it performs"
    );
    assert!(
        flat(&doc).contains("a positional read (`FileExt::read_at`)"),
        "header.rs must name the positional read"
    );
}

/// **The C-01 paragraph's syscall sentence is superseded where it stands.**
///
/// `docs/04-invariants.md` is append-only, so the paragraph under C-01 that
/// says no bench times a syscall stays as written. While it does, D-0790 must
/// name it superseded and row C4-DOCS-WEB-07 must exist.
#[test]
fn the_c01_syscall_sentence_is_superseded_where_it_stands() {
    assert!(
        BENCH.contains("C-28 read_record") && BENCH.contains("C-29 read_record"),
        "C-28 and C-29 time `read_record`"
    );
    assert!(
        FILE_RS.contains("FileExt::read_at(self, buf, offset)"),
        "the read `read_record` performs is a positional read"
    );
    if !flat(INVARIANTS_DOC).contains("no bench in this repository times a syscall") {
        return;
    }
    let entry = DECISIONS_DOC
        .split("\n### D-0790 ")
        .nth(1)
        .and_then(|rest| rest.split("\n### ").next())
        .map(flat)
        .expect("docs/05-decisions.md has a D-0790 entry");
    assert!(
        entry.contains(
            "The paragraph under C-01 in `docs/04-invariants.md` also says no bench times a syscall"
        ) && entry.contains("this entry supersedes that sentence"),
        "D-0790 must name the C-01 sentence superseded: {entry:.300}"
    );
    assert!(
        INVARIANTS_DOC.contains("\n| C4-DOCS-WEB-07 | While the C-01 paragraph"),
        "row C4-DOCS-WEB-07 must record the supersession"
    );
}
