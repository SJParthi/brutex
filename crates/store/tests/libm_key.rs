//! `docs/05-decisions.md` D-0692, checked against the store it describes.
//!
//! D-0692 records that the `.grk` greeks sidecar persists `f64` bits that
//! depend on the platform's `exp` and `ln`, and keys a `.grk` resume to the
//! libm build that began the series. Re-verification found two sentences in it
//! wrong, and this file keeps both from coming back.
//!
//! **The key is an operating rule, and nothing enforces it.** The entry stated
//! the key in the present tense, as if something checked it. Nothing does:
//! neither the header nor the provenance word has a field for a libm build,
//! and `Header::advance` accepts a batch that follows the committed range
//! without reading any stored row or comparing any field but the timestamps.
//! The first two tests drive the store through exactly that, and the third
//! checks that D-0692 and `docs/06-limits.md` §29 say so.
//!
//! **None of them can see the key enforced.** The first fails if the store
//! comes to refuse that forward append, or if a byte of the `.grk` outside the
//! moved fields comes to depend on them. It cannot fail because the store
//! records or checks a libm build: both builds here are one real libm in one
//! process, so a stamp the store writes, into the header or into a file beside
//! the `.grk`, comes out the same in both files and a check against it passes,
//! and a stamp a caller packs into the provenance word never reaches these
//! tests, which build the provenance themselves. No other file under `crates/`
//! names D-0692, and the only others that cite §29 are four files of
//! `crates/greeks`, which cite it for what it records about that crate and
//! read no document, so the entry that enforces the key has to correct
//! D-0692's caveat and §29 itself. The fifth test checks that this doc, §29
//! and D-0692 all say so, the sixth which files under `crates/` name D-0692 or
//! cite §29, and the seventh that the sixth's walk reads only source files and
//! follows no link. The tenth checks that the sixth reads a citation wrapped
//! across two lines as one phrase, and a section number however the document
//! before it is named: it once matched raw text, and missed a greeks pointer
//! still citing §18 across a line break, and it then matched four spellings
//! that name `06-limits.md`, and missed "limits §29", which is how
//! `crates/api` names the register.
//!
//! **A fact is cited to the source that states it.** The entry cited D-0046
//! for "`x86_64` glibc, the CI target", and D-0046 never mentions glibc,
//! `x86_64` or the CI runner. The fourth test checks that the citation names
//! `docs/06-limits.md` §29, that §29 states the fact, that D-0046 still does
//! not, and that the CI workflow runs where §29 says it does. The eighth checks
//! against its source, verbatim, each quotation D-0692 takes from D-0046,
//! `CLAUDE.md` and `store::file`, and the two §29 takes from `store::file` and
//! `api`'s server, and the ninth that they cite `main`'s `96194c11` for the
//! code they read and the linkage they measured, and keep none of the wordings
//! D-0692's second correction lists.
//!
//! A second libm is modelled here as a one-ulp move in a computed `f64`, the
//! size of the first `exp` disagreement D-0046 reports between two real
//! libms. No second libm is linked or run, and nothing here measures how often
//! a real one moves a stored greek.
//!
//! The scratch root is a fresh directory per test under the host's temp
//! directory, named by process id and a counter, and removed on drop.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;

use store::file::{Appended, BarFile, Conflict, StoreError};
use store::format::{Greek, RATE_FROM_OPERATOR, VOL_FROM_SOLVED};
use store::layout::Layout;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

// ===========================================================================
// Scratch
// ===========================================================================

/// Distinguishes two scratch roots taken in the same process.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// A temporary directory tree that removes itself.
struct Scratch {
    root: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let serial = NEXT.fetch_add(1, Ordering::Relaxed);
        let mut root = std::env::temp_dir();
        root.push(format!(
            "brutex-libm-key-{}-{tag}-{serial}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("scratch root");
        Self { root }
    }

    fn root(&self) -> &Path {
        &self.root
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        // Best effort: a leaked scratch directory must never fail a test run.
        drop(fs::remove_dir_all(&self.root));
    }
}

// ===========================================================================
// Fixtures
// ===========================================================================

/// 2025-07-01 09:15 IST, the open of the session's first one-minute bar.
const T0: i64 = 1_751_341_500_000_000;
/// One minute, in microseconds.
const MINUTE: i64 = 60_000_000;
/// Rows per window. One window is one run of a backfill.
const WINDOW: i64 = 5;
/// The slot id every test opens with.
const SYMBOL: u32 = 13;
/// Where `delta` sits in a greeks record: after the stamp, the spot and the
/// volatility, eight bytes each (`Greek::image`).
const DELTA_AT: u64 = 24;

/// Which libm computed a row.
#[derive(Debug, Clone, Copy)]
enum Libm {
    /// The build that began the series.
    Began,
    /// Another build: a second machine, or this one after an OS update.
    Replaced,
}

/// One priced row, as `Libm` would compute it.
///
/// The replaced build moves `delta` by one ulp and leaves every other field
/// alone. D-0692 says one differing field in one row is enough to refuse an
/// overlap, so one field is all the model needs. The rate is the operator's,
/// not computed, and would not move under any libm.
fn greek(index: i64, libm: Libm) -> Greek {
    let began = Greek {
        ts_micros: T0 + index * MINUTE,
        spot: 2_400_000 + index,
        volatility: 0.142_537_891_2,
        delta: 0.523_9,
        gamma: 0.000_012_34,
        vega: 1_234.567_8,
        theta: -987.654_3,
        rho: 45.678_9,
        rate: 0.065_5,
        provenance: Greek::provenance_of(VOL_FROM_SOLVED, RATE_FROM_OPERATOR, true, 0),
    };
    match libm {
        Libm::Began => began,
        Libm::Replaced => Greek {
            delta: began.delta.next_up(),
            ..began
        },
    }
}

/// Window `which` of a backfill, every row computed under `libm`.
fn window(which: i64, libm: Libm) -> Vec<Greek> {
    (which * WINDOW..(which + 1) * WINDOW)
        .map(|index| greek(index, libm))
        .collect()
}

/// One window per entry of `libms`, from window 0, each under its entry.
fn month(libms: &[Libm]) -> Vec<Greek> {
    (0_i64..)
        .zip(libms)
        .flat_map(|(which, &libm)| window(which, libm))
        .collect()
}

fn greeks_path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: "FNO",
        symbol: "NIFTY",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: YearMonth::new(2025, 7).expect("2025-07"),
        file: FileKind::Greeks,
    })
    .expect("a legal greeks path")
}

fn open(root: &Path) -> BarFile {
    BarFile::open_or_create(root, greeks_path(), SYMBOL).expect("open the greeks sidecar")
}

/// The whole `.grk` file, as bytes.
fn image(root: &Path) -> Vec<u8> {
    fs::read(greeks_path().to_path_buf(root)).expect("the greeks file")
}

/// Whether `appended` is the refusal D-0692 describes, for timestamps the file
/// already holds. It was reported as a timestamp fault; since D-1525 it is
/// named as what it is, a restatement of a held row.
fn refused_as_a_timestamp_fault(appended: &Result<Appended, StoreError>) -> bool {
    matches!(
        appended,
        Err(StoreError::OverlapDisagrees {
            conflict: Conflict::Restated,
            ..
        })
    )
}

// ===========================================================================
// The store: nothing records or checks the libm build
// ===========================================================================

/// **A FORWARD APPEND UNDER ANOTHER LIBM IS COMMITTED, AND THE FILE KEEPS NO
/// TRACE OF WHICH BUILD WROTE WHICH WINDOW.**
///
/// Two files hold the same two windows. In one, the second window was computed
/// under a replaced libm. Both appends commit, and the only bytes that differ
/// between the files are the `delta` fields of that window: the header, the
/// stamps, the spots and the provenance words are identical. So nothing but the
/// computed bits themselves says two builds wrote the file. Both builds are one
/// real libm, though, so a record of that libm would be the same in both files
/// and pass here unseen; the module doc says what that leaves unpinned.
#[test]
fn a_forward_append_under_another_libm_is_committed_and_names_no_build() {
    let mixed = Scratch::new("mixed");
    let single = Scratch::new("single");

    for (root, second) in [(mixed.root(), Libm::Replaced), (single.root(), Libm::Began)] {
        let mut file = open(root);
        assert_eq!(
            file.append(&window(0, Libm::Began)),
            Ok(Appended::Committed {
                first_index: 0,
                n_valid: 5,
            }),
        );
        assert_eq!(
            file.append(&window(1, second)),
            Ok(Appended::Committed {
                first_index: 5,
                n_valid: 10,
            }),
            "the second window, under {second:?}, was not committed. D-0692 and \
             docs/06-limits.md §29 say a resume that does not overlap is \
             accepted without comment, and the store now refuses this forward \
             append"
        );
    }

    let (mixed, single) = (image(mixed.root()), image(single.root()));
    assert_eq!(mixed.len(), single.len());

    let header = usize::try_from(Layout::GREEKS.header_len()).expect("header fits");
    assert_eq!(
        mixed[..header],
        single[..header],
        "the two headers differ, though the two files were given the same \
         rows but for window two's `delta`: the header now depends on that \
         `delta`, or on something that is not a row"
    );

    // Every differing byte is a byte of window two's `delta`, and window two's
    // `delta` does differ: the model moved it, and the store kept the move.
    let deltas: Vec<std::ops::Range<usize>> = (5..10)
        .map(|index| {
            let at = Layout::GREEKS.offset_of(index).expect("in range") + DELTA_AT;
            let at = usize::try_from(at).expect("offset fits");
            at..at + 8
        })
        .collect();
    let differing: Vec<usize> = (0..mixed.len())
        .filter(|&at| mixed[at] != single[at])
        .collect();
    assert!(
        !differing.is_empty(),
        "the replaced build's bits were not stored as computed"
    );
    for at in differing {
        assert!(
            deltas.iter().any(|delta| delta.contains(&at)),
            "byte {at} differs and is not a delta of window two, though the two \
             files were given the same rows but for those: that byte now \
             depends on a computed field, or on something that is not a row"
        );
    }
}

/// **AFTER A MIXED RESUME, NO ONE LIBM BUILD RE-FILES THE WHOLE MONTH.**
///
/// Each build re-files the window it wrote as `AlreadyPresent`, and refuses
/// the window the other wrote as a restatement (`OverlapDisagrees`), for timestamps the
/// file already holds. Offering the whole month is refused under either build,
/// and a refused batch files none of the new days it carries.
#[test]
fn after_a_mixed_resume_neither_libm_build_re_files_the_whole_month() {
    let scratch = Scratch::new("refile");
    let mut file = open(scratch.root());
    assert!(matches!(
        file.append(&window(0, Libm::Began)),
        Ok(Appended::Committed { .. })
    ));
    assert!(matches!(
        file.append(&window(1, Libm::Replaced)),
        Ok(Appended::Committed { .. })
    ));
    let before = image(scratch.root());

    assert_eq!(
        file.append(&window(0, Libm::Began)),
        Ok(Appended::AlreadyPresent {
            first_index: 0,
            n_valid: 10,
        }),
        "the build that began the series re-files its own window"
    );
    assert_eq!(
        file.append(&window(1, Libm::Replaced)),
        Ok(Appended::AlreadyPresent {
            first_index: 5,
            n_valid: 10,
        }),
        "the replaced build re-files its own window"
    );

    for (which, libm) in [(0, Libm::Replaced), (1, Libm::Began)] {
        let appended = file.append(&window(which, libm));
        assert!(
            refused_as_a_timestamp_fault(&appended),
            "window {which} re-offered under {libm:?}: {appended:?}"
        );
    }
    for libm in [Libm::Began, Libm::Replaced] {
        let appended = file.append(&month(&[libm, libm]));
        assert!(
            refused_as_a_timestamp_fault(&appended),
            "the whole month re-offered under {libm:?}: {appended:?}"
        );
    }

    // A refused overlap takes its new days with it: window two under the
    // first build, followed by a window the file does not hold yet.
    let appended = file.append(&month(&[Libm::Began, Libm::Began, Libm::Began])[5..]);
    assert!(
        refused_as_a_timestamp_fault(&appended),
        "the overlap plus a new window: {appended:?}"
    );
    assert_eq!(
        image(scratch.root()),
        before,
        "a refusal, or a window already present, wrote a byte"
    );
}

// ===========================================================================
// The documents: what D-0692 and docs/06-limits.md §29 say about it
// ===========================================================================

/// The ledger. Read at compile time so a rename fails the build rather than
/// skipping the check.
const DECISIONS: &str = include_str!("../../../docs/05-decisions.md");
/// The limits register, for §29.
const LIMITS: &str = include_str!("../../../docs/06-limits.md");
/// The CI workflow, for the runner §29 names.
const CI: &str = include_str!("../../../.github/workflows/ci.yml");

/// The text from `start` up to the next `end`, whitespace collapsed.
///
/// Markdown wraps prose at the column, so a phrase can sit on two physical
/// lines. Collapsing every run of whitespace to one space lets a phrase be
/// looked up whole, wherever the wrap falls.
fn section(doc: &str, name: &str, start: &str, end: &str) -> String {
    let from = doc
        .find(start)
        .unwrap_or_else(|| panic!("`{name}` does not contain `{start}`"));
    let rest = &doc[from..];
    let to = rest[start.len()..]
        .find(end)
        .map_or(rest.len(), |at| at + start.len());
    rest[..to].split_whitespace().collect::<Vec<_>>().join(" ")
}

fn d_0692() -> String {
    section(DECISIONS, "docs/05-decisions.md", "### D-0692 ", "\n### D-")
}

fn d_0046() -> String {
    section(DECISIONS, "docs/05-decisions.md", "## D-0046 ", "\n## D-")
}

fn limits_29() -> String {
    section(LIMITS, "docs/06-limits.md", "## 29. ", "\n## 30. ")
}

/// Where `needle` starts in `text`, or a failure naming the phrase that is gone.
fn offset(text: &str, name: &str, needle: &str) -> usize {
    text.find(needle)
        .unwrap_or_else(|| panic!("{name} no longer says `{needle}`"))
}

/// **D-0692 SAYS THE LIBM KEY IS NOT ENFORCED, RIGHT AFTER IT STATES THE KEY.**
///
/// The decision paragraph keys a `.grk` resume to the libm build that began the
/// series, in the present tense. The paragraph after it says nothing records or
/// checks that build, which is what the two tests above drive. It has to come
/// after the decision and before the rejected options, so a reader of the rule
/// meets the caveat before anything else.
#[test]
fn d_0692_and_limits_29_say_nothing_enforces_the_libm_key() {
    let entry = d_0692();
    let name = "D-0692";
    let decision = offset(
        &entry,
        name,
        "a `.grk` series resumes under the libm build that began it.",
    );
    let caveat = offset(
        &entry,
        name,
        "**That rule is operational, and nothing enforces it.**",
    );
    let rejected = offset(&entry, name, "**Rejected.**");
    assert!(
        decision < caveat && caveat < rejected,
        "D-0692's caveat no longer follows the rule it qualifies: rule at \
         {decision}, caveat at {caveat}, rejected options at {rejected}"
    );
    for claim in [
        "No code records or checks the libm build.",
        "Neither the header nor the provenance word has a field for it",
        "a plain forward append under another libm or after an OS update, is \
         accepted without comment",
        "with nothing recording which rows came from which",
        "Hardening 2 below is what would close this.",
        "**Stamp the libm build into the `.grk` provenance.**",
    ] {
        offset(&entry, name, claim);
    }

    let limits = limits_29();
    let name = "docs/06-limits.md §29";
    for claim in [
        "**A resume that does not overlap is not checked at all.**",
        "neither the header nor the provenance word has a field for it",
        "as an operating rule, not an enforced one",
    ] {
        offset(&limits, name, claim);
    }
}

/// **D-0692 CITES THE GLIBC CI TARGET TO A SOURCE THAT STATES IT.**
///
/// It once cited D-0046, which never mentions glibc, `x86_64` or the CI
/// runner. The citation now names `docs/06-limits.md` §29, which does, and the
/// CI workflow is read to confirm the runner §29 names.
#[test]
fn d_0692_cites_the_glibc_ci_target_to_a_source_that_states_it() {
    let entry = d_0692();
    let lead = "nothing here was measured on x86_64 glibc, the CI target (";
    let from = offset(&entry, "D-0692", lead) + lead.len();
    let to = from
        + entry[from..]
            .find(')')
            .expect("the citation closes its parenthesis");
    assert_eq!(
        &entry[from..to],
        "`docs/06-limits.md` §29",
        "D-0692 cites the glibc CI target to a source other than the one that \
         states it"
    );

    let limits = limits_29();
    for fact in [
        "CI runs `ubuntu-24.04` on x86_64 with glibc's libm",
        "**No measurement was taken on glibc**",
    ] {
        offset(&limits, "docs/06-limits.md §29", fact);
    }

    // The source first cited says none of it. The ledger is append-only, so
    // this holds for as long as the ledger does.
    let first_cited = d_0046().to_lowercase();
    for word in ["glibc", "x86", "ubuntu"] {
        assert!(
            !first_cited.contains(word),
            "D-0046 now mentions `{word}`; re-read it before citing it"
        );
    }

    // And the fact itself: every CI job runs on the runner §29 names.
    let runners: Vec<&str> = CI
        .lines()
        .filter_map(|line| line.trim().strip_prefix("runs-on:"))
        .map(str::trim)
        .collect();
    assert!(!runners.is_empty(), "ci.yml names no runner");
    assert!(
        runners.iter().all(|runner| *runner == "ubuntu-24.04"),
        "CI no longer runs only where docs/06-limits.md §29 says: {runners:?}"
    );
}

/// This file, for its own module doc.
const HERE: &str = include_str!("libm_key.rs");

/// This file's module doc: its `//!` lines, whitespace collapsed as
/// [`section`] collapses it. Only the `//!` lines are read, so a phrase the
/// tests below look for, or keep out, is never found in the test itself.
fn module_doc() -> String {
    HERE.lines()
        .filter_map(|line| line.strip_prefix("//!"))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

/// The source of the test named `test` in this file, from its `fn` line to the
/// next `#[test]`, with string continuations joined and whitespace collapsed,
/// so an assertion message can be looked up whole.
fn source_of(test: &str) -> String {
    let start = format!("fn {test}()");
    let from = HERE
        .find(&start)
        .unwrap_or_else(|| panic!("this file has no test `{test}`"));
    let rest = &HERE[from..];
    let to = rest.find("\n#[test]").unwrap_or(rest.len());
    rest[..to]
        .replace("\\\n", " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// **NO TEST HERE CAN SEE THE LIBM KEY ENFORCED, AND ALL THREE TEXTS SAY SO.**
///
/// Both modelled builds are one real libm in one process, so a stamp of the
/// build comes out the same in both files and a check against it passes. §29
/// and this file's module doc once said the first test fails the day the store
/// refuses a forward append under another libm, or writes a byte that tells
/// the builds apart. Both were read as promising that the test would catch the
/// store recording or checking a libm build, and it cannot fail for that,
/// though it does fail for either as the model has them. D-0692's correction,
/// §29 and the module doc now say which, and that the entry which enforces the
/// key has to correct the caveat itself, because no test here will fail to
/// make it. The first test's own messages name no cause it cannot see. All
/// three say the four greeks files cite §29 for what it records about that
/// crate, not for its measurement: `lib.rs` cites it for the solver's bound on
/// model evaluations, and `tests/vendor_anchor.rs` for vendor parameters §29
/// leaves UNVERIFIED.
#[test]
fn nothing_here_sees_the_libm_key_enforced_and_the_three_texts_say_so() {
    let entry = d_0692();
    for claim in [
        "**Correction, 2026-09-24: what `libm_key.rs` can see.**",
        "As the model has the two builds, it does fail for either",
        "It cannot fail because the store records or checks a libm build, \
         which is what both were read as promising.",
        "Both modelled builds are one real libm in one process",
        "comes out the same in both files and a check against it passes",
        "which build the provenance themselves",
        "every test in the `store` suite passed, these four included",
        "No file under `crates/` but `libm_key.rs` names this entry, and the \
         only others that cite §29 are four files of `crates/greeks`",
        "has to supersede the paragraph that begins \"That rule is \
         operational, and nothing enforces it\" and correct §29 itself",
    ] {
        offset(&entry, "D-0692", claim);
    }

    let doc = module_doc();
    let limits = limits_29();
    for (text, name, alone, owed) in [
        (
            &doc,
            "libm_key.rs's module doc",
            "No other file under `crates/` names D-0692",
            "has to correct D-0692's caveat and §29 itself",
        ),
        (
            &limits,
            "docs/06-limits.md §29",
            "No file under `crates/` but `libm_key.rs` names D-0692",
            "has to correct this paragraph and D-0692's caveat itself",
        ),
    ] {
        for claim in [
            "It cannot fail because the store records or checks a libm build",
            "one real libm in one process",
            "comes out the same in both files and a check against it passes",
            "which build the provenance themselves",
            "the only others that cite §29 are four files of `crates/greeks`",
            alone,
            owed,
        ] {
            offset(text, name, claim);
        }
        assert!(
            !text.contains("tells the two builds apart"),
            "{name} again says the first test would see a byte that tells the \
             two builds apart, which reads as a record of the libm build, and \
             a record of one real libm is the same in both files"
        );
    }
    offset(
        &limits,
        "docs/06-limits.md §29",
        "every test in the `store` suite passed",
    );
    for (text, name) in [
        (&entry, "D-0692"),
        (&limits, "docs/06-limits.md §29"),
        (&doc, "libm_key.rs's module doc"),
    ] {
        assert!(
            !text.contains("cannot fail for either"),
            "{name} again says the first test cannot fail for either reason, \
             and it fails for both as the model has them; what it cannot \
             see is the store recording or checking a libm build"
        );
        offset(
            text,
            name,
            "which cite it for what it records about that crate and read no \
             document",
        );
        for gone in ["cite it for the measurement", "cite it for its measurement"] {
            assert!(
                !text.contains(gone),
                "{name} again says the greeks files {gone}, and two of them \
                 cite §29 for the solver's evaluation bound and for vendor \
                 parameters it leaves UNVERIFIED"
            );
        }
    }

    // The first test's messages give no cause it cannot see: a refusal made by
    // position alone fails it with the same message as one made by content.
    let first = source_of("a_forward_append_under_another_libm_is_committed_and_names_no_build");
    offset(
        &first,
        "the first test",
        "and the store now refuses this forward append",
    );
    for cause in [
        "for what its rows hold",
        "has come to depend on a computed field",
    ] {
        assert!(
            !first.contains(cause),
            "a message of the first test again gives a cause it cannot see: \
             `{cause}`"
        );
    }
}

/// `crates/`, the directory this crate sits in.
fn crates_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/store sits under crates/")
}

/// Every `.rs`, `.toml` and `.md` file under `dir`, at any depth, in no
/// particular order.
///
/// Nothing else is read, because nothing else is source: an editor's backup,
/// a merge's `.orig` or a swap file of a source file holds its text without
/// being it. A link is not followed, to a file or to a directory, because a
/// linked directory can loop and a linked file is read where it lives.
fn sources_under(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|error| panic!("{}: {error}", dir.display())) {
        let entry = entry.expect("a directory entry");
        // `DirEntry::file_type` does not follow a link, so a link is neither.
        let kind = entry.file_type().expect("a file type");
        let path = entry.path();
        if kind.is_dir() {
            sources_under(&path, into);
        } else if kind.is_file()
            && path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "rs" | "toml" | "md"))
        {
            into.push(path);
        }
    }
}

/// Each source file under `root` whose prose satisfies `names`, as a path
/// relative to `root` with `/` between its parts, sorted. Each file is read
/// through [`prose`], so a phrase wrapped across two comment lines, or across
/// a continued string, is looked up whole.
fn sources_in_that(root: &Path, names: impl Fn(&str) -> bool) -> Vec<String> {
    let mut files = Vec::new();
    sources_under(root, &mut files);
    let mut hits: Vec<String> = files
        .iter()
        .filter(|path| names(&prose(&fs::read_to_string(path).expect("source is UTF-8"))))
        .map(|path| {
            path.strip_prefix(root)
                .expect("under the root walked")
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    hits.sort();
    hits
}

/// Each source file under `crates/` whose prose satisfies `names`, as
/// [`sources_in_that`] gives it.
fn sources_that(names: impl Fn(&str) -> bool) -> Vec<String> {
    sources_in_that(crates_dir(), names)
}

/// `source` read as prose: each line's leading comment marker (`//!`, `///`,
/// `//` or `#`) removed, a string's `\` line continuation joined, and every
/// run of whitespace collapsed to one space.
///
/// A citation wraps where its comment or its string wraps, so
/// `docs/06-limits.md` can end one line and its section begin the next. Read
/// raw, that citation is two fragments and matches no spelling of it; read as
/// prose, it is the phrase a reader sees.
fn prose(source: &str) -> String {
    collapse(
        &source
            .replace("\\\n", "\n")
            .lines()
            .map(|line| {
                let line = line.trim_start();
                ["//!", "///", "//", "#"]
                    .iter()
                    .find_map(|marker| line.strip_prefix(marker))
                    .unwrap_or(line)
            })
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// Whether `text` mentions a section `number` of any document: `§29`,
/// `§ 29`, `section 29` or `sections 29`, in any case, where the number does
/// not run on into a longer one, as §290 runs on from 29.
///
/// It does not ask which document the section is in. `crates/` names
/// `docs/06-limits.md` before a section in more ways than a list keeps up
/// with: `` `docs/06-limits.md` §98 ``, `docs/06-limits.md section 18`, and,
/// in `crates/api`, `limits §125`. A list of four spellings, each naming
/// `06-limits.md`, once stood here, and read no citation of the last kind.
/// Every mention of the number is read instead, so a citation of the register
/// is seen however the register is named, and another document's section of
/// that number is seen too, for whoever reads the failure to list. A number
/// apart from its `§` or `section`, as the 29 in `sections 18 and 29`, is not
/// read.
///
/// Give it text read through [`prose`]: raw, a citation wrapped across two
/// lines is split, and a line can end with `section` and the next begin with
/// its number.
fn mentions_section(text: &str, number: &str) -> bool {
    let text = text.to_ascii_lowercase();
    ["§", "§ ", "section ", "sections "]
        .iter()
        .map(|lead| format!("{lead}{number}"))
        .any(|mention| {
            text.match_indices(&mention).any(|(at, _)| {
                !text[at + mention.len()..]
                    .chars()
                    .next()
                    .is_some_and(|next| next.is_ascii_digit())
            })
        })
}

/// **ONLY THIS FILE NAMES D-0692, AND ONLY `crates/greeks` ALSO CITES §29.**
///
/// D-0692's correction and §29 say the key can be enforced without failing a
/// test that would send its author back to either text, because this file is
/// the only one that names D-0692, and the only others that cite §29 are four
/// files of `crates/greeks` that cite it for what it records about that crate
/// and read no document. A file that comes to name either, or a greeks file
/// that comes to read a document, fails here, so that sentence is re-read the
/// day it could stop being true. The greeks files are listed rather than
/// exempted: they cited §18, the number §29 had before it was renumbered on
/// merge, until the review that corrected them, and a list is what caught
/// that.
///
/// Every file is read as [`prose`], every mention of a section 29 is taken
/// for a citation of §29 whatever document it names, and "section 18" is as
/// stale as "§18". This test once matched the raw text, and stayed green while
/// `crates/greeks/src/solver.rs` still cited "`docs/06-limits.md` section" on
/// one comment line and "18" on the next. It then matched four spellings, each
/// naming `06-limits.md`, and stayed green with "limits §29" in a module doc
/// of `crates/core` and "limits §18" in a greeks comment, which is how
/// `crates/api` names the register. A file that mentions another document's
/// §29 fails here too, and is listed once read. A file reads a document
/// through a string that names it, so any string ending in `.md` counts as a
/// read, wherever the call around it closes.
#[test]
fn only_this_file_names_d_0692_and_only_greeks_also_cites_limits_29() {
    assert_eq!(
        sources_that(|text| text.contains("D-0692")),
        ["store/tests/libm_key.rs"],
        "the files under crates/ that name D-0692 changed. D-0692's correction \
         and docs/06-limits.md §29 say only libm_key.rs does; re-read both"
    );

    let greeks = [
        "greeks/src/bsm.rs",
        "greeks/src/lib.rs",
        "greeks/src/solver.rs",
        "greeks/tests/vendor_anchor.rs",
    ];
    let mut expected = greeks.to_vec();
    expected.push("store/tests/libm_key.rs");
    assert_eq!(
        sources_that(|text| mentions_section(text, "29")),
        expected,
        "the files under crates/ that mention a section 29, of any document, \
         changed. D-0692's correction and docs/06-limits.md §29 say only \
         libm_key.rs and four greeks files cite §29; re-read both, and list a \
         file here once its mention is read"
    );

    for file in greeks {
        let text = fs::read_to_string(crates_dir().join(file)).expect("a greeks source");
        assert!(
            !text.contains(".md\""),
            "{file} now names a document in a string, which is how a file reads \
             one. D-0692's correction and docs/06-limits.md §29 say it cites \
             §29 and reads none; re-read both"
        );
    }
    let stale: Vec<String> = sources_that(|text| mentions_section(text, "18"))
        .into_iter()
        .filter(|file| file.starts_with("greeks/"))
        .collect();
    assert!(
        stale.is_empty(),
        "{stale:?} mention a section 18, however its document is named and \
         perhaps across a line break. docs/06-limits.md §18 is \"What CI \
         cannot prove about a credential path\", and this crate's measurement \
         is §29; a section 18 of another document is excluded here by name \
         once read"
    );
}

/// **THE WALK ABOVE READS SOURCE, AND FOLLOWS NO LINK.**
///
/// A copy of this file left beside it by an editor or a merge names D-0692
/// without being source, and failed the test above when the walk read every
/// file. A linked directory that points back up the tree would recurse until
/// the operating system refused the path.
#[test]
fn the_crates_walk_reads_only_source_files_and_follows_no_link() {
    let scratch = Scratch::new("walk");
    let root = scratch.root();
    let tree = root.join("tree");
    fs::create_dir_all(tree.join("nested")).expect("the scratch tree");
    for (name, text) in [
        ("keep.rs", "D-0692"),
        ("Cargo.toml", "D-0692"),
        ("nested/notes.md", "D-0692"),
        ("keep.rs.orig", "D-0692"),
        (".keep.rs.swp", "D-0692"),
        ("keep.rs~", "D-0692"),
        ("nested/image.bin", "D-0692"),
    ] {
        fs::write(tree.join(name), text).expect("a scratch file");
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&tree, tree.join("nested").join("loop"))
            .expect("a directory link");
        std::os::unix::fs::symlink(tree.join("keep.rs"), tree.join("linked.rs"))
            .expect("a file link");
    }

    let mut found = Vec::new();
    sources_under(&tree, &mut found);
    let mut found: Vec<String> = found
        .iter()
        .map(|path| {
            path.strip_prefix(&tree)
                .expect("under the scratch tree")
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        })
        .collect();
    found.sort();
    assert_eq!(found, ["Cargo.toml", "keep.rs", "nested/notes.md"]);
}

/// Every run of whitespace in `text` collapsed to one space.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The text of `source`'s `//` comments, markers removed, whitespace collapsed.
fn comments(source: &str) -> String {
    collapse(
        &source
            .lines()
            .filter_map(|line| line.trim_start().strip_prefix("//"))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// `CLAUDE.md`, for the sentences D-0692 quotes from it.
const CLAUDE_MD: &str = include_str!("../../../CLAUDE.md");
/// `store::file`, for the comment D-0692 and §29 quote from `append`.
const FILE_RS: &str = include_str!("../src/file.rs");
/// `api`'s server, for the message §29 quotes from the Groww chain path.
const SERVER_RS: &str = include_str!("../../api/src/server.rs");

/// **D-0692 AND §29 QUOTE THEIR SOURCES VERBATIM.**
///
/// Each quotation is looked up, opening quote mark included, in the text that
/// quotes it and, whitespace collapsed, in the source it names. The list is
/// every quotation D-0692 takes from D-0046, `CLAUDE.md` and `store::file`,
/// and the two §29 takes from `store::file` and `api`'s server. D-0692 once
/// quoted `CLAUDE.md` §4 with a lowercase first letter, and this list once
/// left out the two quotations D-0692 opens with, D-0046's section title and
/// its "no consumer".
#[test]
fn d_0692_and_limits_29_quote_their_sources_verbatim() {
    let entry = d_0692();
    let limits = limits_29();
    let (claude, d_0046, append, server) = (
        collapse(CLAUDE_MD),
        d_0046(),
        comments(FILE_RS),
        collapse(SERVER_RS),
    );

    for (text, name, quote, source, source_name) in [
        (&entry, "D-0692", "no consumer", &d_0046, "D-0046"),
        (
            &entry,
            "D-0692",
            "Reproducible within one target, and not across two",
            &d_0046,
            "D-0046",
        ),
        (
            &entry,
            "D-0692",
            "must never enter the blake3 run identity of `CLAUDE.md` §3 rule 3, \
             and must never be compared byte-for-byte across machines",
            &d_0046,
            "D-0046",
        ),
        (
            &entry,
            "D-0692",
            "Nothing in this repository does either today — `greeks` is a leaf \
             with no consumer here — and this paragraph is what a future \
             consumer has to read first.",
            &d_0046,
            "D-0046",
        ),
        (&entry, "D-0692", "within one target", &d_0046, "D-0046"),
        (
            &entry,
            "D-0692",
            "would replace a measured 4.2e-14 disagreement with several \
             hundred lines of transcendental code carrying its own errors, for \
             a property nothing currently needs.",
            &d_0046,
            "D-0046",
        ),
        (
            &entry,
            "D-0692",
            "no clock, no randomness, no hash iteration and no threading \
             anywhere in its non-test code",
            &d_0046,
            "D-0046",
        ),
        (
            &entry,
            "D-0692",
            "keep full precision and are never rounded for storage",
            &claude,
            "CLAUDE.md",
        ),
        (
            &entry,
            "D-0692",
            "A new field is a new file version at its own stride.",
            &claude,
            "CLAUDE.md",
        ),
        (
            &entry,
            "D-0692",
            "a genuine conflict. The vendor restated history, or bars arrived \
             out of order",
            &append,
            "crates/store/src/file.rs",
        ),
        (
            &limits,
            "docs/06-limits.md §29",
            "a genuine conflict. The vendor restated history, or bars arrived \
             out of order",
            &append,
            "crates/store/src/file.rs",
        ),
        (
            &limits,
            "docs/06-limits.md §29",
            "the greeks were computed and could not be filed",
            &server,
            "crates/api/src/server.rs",
        ),
    ] {
        offset(text, name, &format!("\"{quote}"));
        assert!(
            source.contains(quote),
            "{name} quotes {source_name} as saying `{quote}`, and it does not"
        );
    }

    the_texts_say_which_quotations_the_eighth_test_checks(&entry);
}

/// D-0692 and this file's module doc say which quotations the eighth test
/// checks, and neither says it checks every quotation in both texts, which it
/// once said while leaving out the two D-0692 opens with.
fn the_texts_say_which_quotations_the_eighth_test_checks(entry: &str) {
    let doc = module_doc();
    for (text, name, who) in [
        (entry, "D-0692", "this entry"),
        (doc.as_str(), "libm_key.rs's module doc", "D-0692"),
    ] {
        offset(
            text,
            name,
            &format!(
                "The eighth checks against its source, verbatim, each quotation \
                 {who} takes from D-0046, `CLAUDE.md` and `store::file`, and the \
                 two §29 takes from `store::file` and `api`'s server"
            ),
        );
        assert!(
            !text.contains("quote their sources verbatim"),
            "{name} again says the eighth test checks every quotation in D-0692 \
             and §29, and it checks the ones it lists"
        );
    }
}

/// **D-0692 AND §29 CITE WHAT `main` HOLDS, AND KEEP NONE OF THE WORDINGS
/// D-0692'S SECOND CORRECTION LISTS.**
///
/// The code reading and the linkage measurement cite `96194c11`, on `main`,
/// where they were re-taken: the commit first cited is in neither `main`'s
/// history nor this branch's. And each wording that correction lists stays
/// gone: that `Header::advance` compares no field of any row, when it compares
/// the timestamps; that the two hardenings only name the refusal better, when
/// the second also refuses a forward append; that G-10's test proves, when it
/// checks the inputs it runs, in either of the two sentences that said so; and
/// that the §4 sentence it quotes begins with a lowercase letter. So does
/// §29's question whether a resume under one build is always
/// `AlreadyPresent`, when a window whose inputs changed is refused under any.
/// Each is refused as it was written.
#[test]
fn d_0692_and_limits_29_cite_main_and_keep_none_of_the_corrected_wordings() {
    let entry = d_0692();
    let limits = limits_29();
    let doc = module_doc();

    for (text, name) in [(&entry, "D-0692"), (&limits, "docs/06-limits.md §29")] {
        offset(text, name, "from the source at `96194c11`");
        for gone in [
            "from the source at `b1d9ac70`",
            "from the code at `b1d9ac70`",
        ] {
            assert!(
                !text.contains(gone),
                "{name} again cites `{gone}`, a commit in neither main's \
                 history nor this branch's"
            );
        }
    }
    offset(&entry, "D-0692", "read from the code at `96194c11`.");

    for (text, name) in [
        (&entry, "D-0692"),
        (&limits, "docs/06-limits.md §29"),
        (&doc, "libm_key.rs's module doc"),
    ] {
        offset(
            text,
            name,
            "without reading any stored row or comparing any field but the \
             timestamps",
        );
        assert!(
            !text.contains("without comparing a field of any row"),
            "{name} again says Header::advance compares no field of any row, \
             and it compares the batch's timestamps"
        );
    }

    offset(
        &entry,
        "D-0692",
        "Neither hardening below removes the refusal: the first names it \
         better, and the second would refuse a resume under another libm up \
         front, by name, whether or not it overlaps.",
    );
    assert!(
        !entry.contains("only name the refusal better"),
        "D-0692 again says both hardenings only name the refusal better, and \
         the second would also refuse a forward append"
    );

    offset(
        &entry,
        "D-0692",
        "checks that the same inputs give the same solved volatility",
    );
    assert!(
        !entry.contains("proves that the same inputs"),
        "D-0692 again says G-10's test proves what it checks on its inputs"
    );
    offset(
        &entry,
        "D-0692",
        "A resume in a new process is outside what it checks.",
    );
    assert!(
        !entry.contains("outside what it proves"),
        "D-0692 again says what G-10's test proves, where it checks inputs"
    );

    assert!(
        !entry.contains("\"a new field is a new file version"),
        "D-0692 again quotes CLAUDE.md §4 with a lowercase first letter"
    );

    offset(
        &limits,
        "docs/06-limits.md §29",
        "recomputes the same bits and so is `AlreadyPresent`",
    );
    assert!(
        !limits.contains("always `AlreadyPresent`"),
        "docs/06-limits.md §29 again asks whether a same-build resume is \
         always AlreadyPresent, and a window whose inputs changed is refused \
         under any build"
    );

    offset(
        &entry,
        "D-0692",
        "the ninth that they cite `96194c11` and keep none of the wordings \
         corrected here, each refused as it was written",
    );
    offset(
        &doc,
        "libm_key.rs's module doc",
        "keep none of the wordings D-0692's second correction lists",
    );
    assert!(
        !doc.contains("keep none of the wordings a review corrected"),
        "libm_key.rs's module doc again says the ninth test refuses every \
         wording a review corrected, and it refuses those D-0692's second \
         correction lists"
    );
}

/// **THE SIXTH TEST READS A SECTION WHERE IT WRAPS, HOWEVER ITS DOCUMENT IS
/// NAMED.**
///
/// The sixth test once matched raw text for four unwrapped spellings of §29
/// and one of §18. `crates/greeks/src/solver.rs` cited
/// "`docs/06-limits.md` section" on one comment line and "18" on the next,
/// and the repair that corrected six other greeks pointers left it, because
/// that test stayed green. A module doc citing §29 across a line break in a
/// crate outside `crates/greeks` would have left it green too. The repair that
/// read wrapped text still matched four spellings, each naming
/// `06-limits.md`, and `crates/api` cites the register as "limits §98" and
/// "limits §125": a review wrote "limits §29" into `crates/core` and
/// "limits §18" into `crates/greeks`, and the sixth test stayed green. All
/// four are written into a scratch tree here and read through the same walk,
/// beside a citation that names its document after the section, and each
/// shape `crates/` wraps a citation in is read as prose. Each of those shapes
/// wraps between the `§` or `section` and the number: they once wrapped before
/// it, where the mention is whole on one line, and stayed green with the
/// continuation join, the `//!` and `///` stripping or the `#` stripping
/// removed from [`prose`]. D-0692, §29 and this file's module doc say what the
/// sixth test reads, and D-0692 that two of its readings missed a citation.
#[test]
fn the_citation_check_reads_a_wrapped_citation_however_its_document_is_named() {
    let scratch = Scratch::new("wrapped");
    let tree = scratch.root().join("tree");
    for dir in ["api/src", "core/src", "greeks/src"] {
        fs::create_dir_all(tree.join(dir)).expect("the scratch tree");
    }
    for (file, text) in [
        (
            "core/src/lib.rs",
            "//! The libm hazard is in `docs/06-limits.md`\n//! §29.\n",
        ),
        (
            "greeks/src/normal.rs",
            "fn f() {\n    // Invariant G-10 is narrowed to match and `docs/06-limits.md` \
             section\n    // 18 carries the measurement. D-0046.\n}\n",
        ),
        // The review's break: the register named as `crates/api` names it.
        (
            "core/src/vendor.rs",
            "//! Greek bits depend on the libm. D-0046 and limits §29.\n",
        ),
        (
            "greeks/src/moneyness.rs",
            "// The measurement is in D-0046 and limits §18.\n",
        ),
        // The same, wrapped between the register's name and the section.
        (
            "api/src/trades.rs",
            "//! saturation answers 429 before another task is queued. D-0435 and \
             limits\n//! §29.\n",
        ),
        // The section before the document.
        (
            "core/src/error.rs",
            "/// §29 of the limits register carries it.\npub struct E;\n",
        ),
    ] {
        fs::write(tree.join(file), text).expect("a scratch file");
    }
    assert_eq!(
        sources_in_that(&tree, |text| mentions_section(text, "29")),
        [
            "api/src/trades.rs",
            "core/src/error.rs",
            "core/src/lib.rs",
            "core/src/vendor.rs"
        ],
        "a file citing docs/06-limits.md §29, across a line break or with the \
         register named another way, is not seen"
    );
    assert_eq!(
        sources_in_that(&tree, |text| mentions_section(text, "18")),
        ["greeks/src/moneyness.rs", "greeks/src/normal.rs"],
        "a comment citing docs/06-limits.md §18, across a line break or with \
         the register named another way, is not seen"
    );

    // Each wrap falls between the `§` or `section` and its number. A wrap
    // before the sign or the word leaves the mention whole on one line, so it
    // is found even when `prose` removes no marker and joins no continuation,
    // and the case holds none of them.
    for (source, number, mentions) in [
        // An item doc, indented.
        (
            "    /// good to this many digits*. `docs/06-limits.md` section\n    /// 29 carries",
            "29",
            true,
        ),
        // A module doc.
        (
            "//! The libm hazard is in `docs/06-limits.md` §\n//! 29.",
            "29",
            true,
        ),
        // A string continued with a backslash.
        (
            "\"cite docs/06-limits.md section \\\n         29 for it\"",
            "29",
            true,
        ),
        // A TOML comment, and a Markdown line.
        (
            "# registered in docs/06-limits.md section\n# 29",
            "29",
            true,
        ),
        ("see `docs/06-limits.md` section\n29 for it", "29", true),
        // The register named as `crates/api` names it, wrapped and not.
        ("//! D-0435 and limits section\n//! 29.", "29", true),
        ("//! D-0404 and limits §29 name the cold bound", "29", true),
        ("# limits §\n# 29", "29", true),
        // Any case, a space after the sign, and the plural.
        ("`docs/06-limits.md` Section 29", "29", true),
        ("Limits § 29", "29", true),
        ("SECTIONS 29 and 30", "29", true),
        // Another document's §29 is read too, for its reader to list.
        ("`docs/05-decisions.md` §29", "29", true),
        // Not a section 29.
        ("`docs/06-limits.md` §290", "29", false),
        ("limits §129", "29", false),
        ("limits §18", "29", false),
        ("`docs/06-limits.md` section 29", "18", false),
        ("29 sections", "29", false),
    ] {
        assert_eq!(
            mentions_section(&prose(source), number),
            mentions,
            "`{source}`, read as prose, {} a mention of a section {number}",
            if mentions { "is not" } else { "is" }
        );
    }

    the_texts_say_what_the_sixth_test_reads();
}

/// D-0692, §29 and this file's module doc say what the sixth test reads, and
/// D-0692 that two of its readings missed a citation; none of the three says
/// it reads each spelling `crates/` uses, which it once said while reading
/// four of five.
fn the_texts_say_what_the_sixth_test_reads() {
    let entry = d_0692();
    for claim in [
        "at seven places, the number §29 had before it was renumbered on \
         merge, and all seven now cite §29.",
        "The first pass of this correction changed six and said all four files \
         were done.",
        "counts \"section 18\" as stale beside \"§18\"",
        "That reading still found a citation only in four spellings, each \
         naming `06-limits.md`, and `crates/api` also cites the register as \
         \"limits §98\" and \"limits §125\".",
        "which made the first correction's sentence about §29 false, and the \
         sixth test stayed green.",
        "it takes every \"§29\", \"§ 29\", \"section 29\" or \"sections 29\" \
         under `crates/`, in any case and whatever document it names, as a \
         possible citation of §29",
        "as the 29 in \"sections 18 and 29\", is not read.",
        "the tenth that the sixth test's reading finds a citation wrapped \
         across two lines, and a section number however the document before \
         it is named.",
    ] {
        offset(&entry, "D-0692", claim);
    }
    assert!(
        !entry.contains("renumbered on merge, and they now cite §29"),
        "D-0692 again says the greeks files now cite §29 without saying its \
         first pass missed the pointer wrapped across a line in solver.rs"
    );
    let limits = limits_29();
    for claim in [
        "reads a citation that wraps across two lines as one phrase",
        "for a citation, whatever document it names",
    ] {
        offset(&limits, "docs/06-limits.md §29", claim);
    }
    let doc = module_doc();
    offset(
        &doc,
        "libm_key.rs's module doc",
        "The tenth checks that the sixth reads a citation wrapped across two \
         lines as one phrase, and a section number however the document before \
         it is named",
    );
    for (text, name) in [
        (&entry, "D-0692"),
        (&limits, "docs/06-limits.md §29"),
        (&doc, "libm_key.rs's module doc"),
    ] {
        assert!(
            !text.contains("in each spelling `crates/` uses"),
            "{name} again says the sixth test reads each spelling `crates/` \
             uses for a citation, and it once read four while `crates/api` \
             used a fifth"
        );
    }
}
