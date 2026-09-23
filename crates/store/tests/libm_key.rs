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
//! without comparing a field of any row. The first two tests drive the store
//! through exactly that, and the third checks that D-0692 and
//! `docs/06-limits.md` §29 say so. If the store comes to refuse that forward
//! append, or to write a byte that tells the two builds apart outside the bits
//! they computed, the first test fails, and the entry that makes the change
//! has to say what changed. A stamp a caller packs into the provenance word
//! would not be seen here, because these tests build the provenance
//! themselves.
//!
//! **A fact is cited to the source that states it.** The entry cited D-0046
//! for "`x86_64` glibc, the CI target", and D-0046 never mentions glibc,
//! `x86_64` or the CI runner. The fourth test checks that the citation names
//! `docs/06-limits.md` §29, that §29 states the fact, that D-0046 still does
//! not, and that the CI workflow runs where §29 says it does.
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

use store::file::{Appended, BarFile, StoreError};
use store::format::{FormatError, Greek, RATE_FROM_OPERATOR, VOL_FROM_SOLVED};
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

/// Whether `appended` is the refusal D-0692 describes: a timestamp fault, for
/// timestamps the file already holds.
fn refused_as_a_timestamp_fault(appended: &Result<Appended, StoreError>) -> bool {
    matches!(
        appended,
        Err(StoreError::Format {
            source: FormatError::TimestampsOutOfOrder { .. },
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
/// computed bits themselves says two builds wrote the file.
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
             accepted without comment; if a libm check now refuses it, the \
             entry that added the check must say so"
        );
    }

    let (mixed, single) = (image(mixed.root()), image(single.root()));
    assert_eq!(mixed.len(), single.len());

    let header = usize::try_from(Layout::GREEKS.header_len()).expect("header fits");
    assert_eq!(
        mixed[..header],
        single[..header],
        "the header now differs by the libm that wrote a window, which D-0692 \
         and docs/06-limits.md §29 say nothing records"
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
            "byte {at} differs and is not a delta of window two: something in \
             the file now records the build"
        );
    }
}

/// **AFTER A MIXED RESUME, NO ONE LIBM BUILD RE-FILES THE WHOLE MONTH.**
///
/// Each build re-files the window it wrote as `AlreadyPresent`, and refuses
/// the window the other wrote with `TimestampsOutOfOrder`, for timestamps the
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
