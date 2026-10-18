//! The bar routes over both forms of a month (SC-3, D-4965; design §4.4 and
//! §6 row "Old and new forms in one request").
//!
//! A sealed month (`.sbar`) holds the same records bit for bit as its raw
//! form, and the read door opens it first. So every window — time order both
//! ways, a page across a month boundary, a value sort, Go to a minute, the
//! extremes — and every page answers the same over raw months, sealed months
//! and a mixture. Prices are invented and absurd.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]

use super::*;

const SYMBOL: &str = "SCTHREESEAL";
const SEGMENT: &str = "TESTSEG";

fn scratch(tag: &str) -> std::path::PathBuf {
    let root =
        std::env::temp_dir().join(format!("brutex-sealed-window-{}-{tag}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a scratch root");
    root
}

fn month_path(month: YearMonth) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Dhan,
        exchange: "NSE",
        segment: SEGMENT,
        symbol: SYMBOL,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    })
    .expect("a legal path")
}

#[expect(clippy::cast_possible_truncation, reason = "the store's own fold")]
fn symbol_id() -> u32 {
    brutex_core::universe::fnv1a(SYMBOL) as u32
}

/// `n` one-minute bars from 09:15 IST on the 2nd of `month`; the close
/// carries the index and wanders, so a value sort has ties and order to keep.
fn write_month(root: &std::path::Path, month: YearMonth, n: i64, base: i64) {
    let start = month.ist_bounds_micros().0 + (86_400 + 555 * 60) * 1_000_000;
    let rows: Vec<Bar> = (0..n)
        .map(|i| {
            let close = base + (i * 37) % 101;
            Bar {
                ts_micros: start + i * 60_000_000,
                open: close,
                high: close + 10,
                low: close - 10,
                close,
                volume: 1 + i % 7,
                open_interest: if i % 3 == 0 { OI_NULL } else { 5_000_000 + i },
            }
        })
        .collect();
    let mut file = store::file::BarFile::open_or_create(root, month_path(month), symbol_id())
        .expect("a bar file");
    file.append(&rows).expect("the batch appends");
}

/// Writes `month`'s sealed form beside its raw one, by rename.
fn seal(root: &std::path::Path, month: YearMonth) {
    let path = month_path(month);
    let raw = store::file::BarFile::open_existing(root, path, symbol_id()).expect("raw");
    assert_eq!(raw.form(), store::file::Form::Raw);
    let bars = raw.read_range(0, raw.records()).expect("every row");
    let built = store::sealed::build(path, &raw.header(), &bars, None, None).expect("seals");
    let at = path.with_file(FileKind::SealedBars).to_path_buf(root);
    let staged = at.with_extension("staged");
    std::fs::write(&staged, built.to_bytes()).expect("staged");
    std::fs::rename(&staged, &at).expect("published");
    let sealed = store::file::BarFile::open_existing(root, path, symbol_id()).expect("sealed");
    assert_eq!(sealed.form(), store::file::Form::Sealed);
}

fn ym(month: u8) -> YearMonth {
    YearMonth::new(2031, month).expect("a month")
}

fn queries() -> Vec<WindowQuery> {
    let key = SeriesKey::new("NSE", SEGMENT, SYMBOL, None);
    let q = |sort, desc, offset, limit, extremes| {
        WindowQuery::single(
            Vendor::Dhan,
            key.clone(),
            Timeframe::MINUTE_1,
            ym(1),
            ym(3),
            sort,
            desc,
            offset,
            limit,
            extremes,
        )
    };
    let go_to = ym(2).ist_bounds_micros().0 + (86_400 + 555 * 60 + 17 * 60) * 1_000_000;
    vec![
        // Newest first and oldest first, at both ends and across a boundary.
        q(SortKey::Ts, true, 0, 100, true),
        q(SortKey::Ts, false, 0, 100, false),
        q(SortKey::Ts, false, 250, 100, false),
        q(SortKey::Ts, true, 520, 100, false),
        // A value sort at the first page and at a depth.
        q(SortKey::Close, true, 0, 50, false),
        q(SortKey::Close, false, 400, 50, false),
        q(SortKey::Volume, true, 123, 25, false),
        // Go to a minute.
        WindowQuery {
            at: Some(go_to),
            offset: None,
            ..q(SortKey::Ts, false, 0, 40, false)
        },
    ]
}

fn snapshot(root: &std::path::Path) -> Vec<String> {
    queries()
        .iter()
        .map(|query| {
            let got = window(root, query).expect("a legal window");
            format!(
                "total={} offset={} read={} missing={} bars={:?} extremes={:?} faults={:?} \
                 faulted={}",
                got.total,
                got.offset,
                got.months_read,
                got.months_missing,
                got.bars,
                got.extremes,
                got.faults,
                got.faulted_records
            )
        })
        .collect()
}

#[test]
fn a_window_over_both_forms_equals_the_same_window_over_one() {
    let root = scratch("both-forms");
    write_month(&root, ym(1), 300, 41_000_000_000);
    write_month(&root, ym(2), 300, 42_000_000_000);
    write_month(&root, ym(3), 300, 43_000_000_000);
    let raw = snapshot(&root);
    assert!(raw.iter().all(|line| line.contains("faults=[]")), "{raw:?}");

    // One month sealed: a window over BOTH forms.
    seal(&root, ym(2));
    assert_eq!(snapshot(&root), raw, "raw and sealed months together");

    // Every month sealed; then the raw form of one damaged where only a
    // reader of the `.bin` would see it: the answers do not move, because the
    // door reads the sealed form.
    seal(&root, ym(1));
    seal(&root, ym(3));
    assert_eq!(snapshot(&root), raw, "every month sealed");
    let bin = month_path(ym(3)).to_path_buf(&root);
    let mut bytes = std::fs::read(&bin).expect("the raw form");
    let at = bytes.len() - 20;
    bytes[at] ^= 0xFF;
    std::fs::write(&bin, bytes).expect("damaged raw form");
    assert_eq!(snapshot(&root), raw, "the sealed form is what is served");
    // THE RAW FORM RETIRED (SC-9, off today): the month is listed and read
    // from its sealed form alone.
    std::fs::rename(&bin, bin.with_extension("bin-retired")).expect("retired");
    assert_eq!(snapshot(&root), raw, "a month held only sealed");

    // The page route's reader: one range read for a sealed month.
    let file =
        store::file::BarFile::open_existing(&root, month_path(ym(2)), symbol_id()).expect("sealed");
    let (rows, faults) = page(&file, 250, 100);
    assert_eq!(rows.len(), 50, "a page past the end is cut at the commit");
    assert!(faults.sentences().is_empty());
    assert_eq!(
        rows[0].ts_micros,
        file.read_record(250).expect("row").ts_micros
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_damaged_sealed_frame_is_named_row_by_row_on_a_page() {
    let root = scratch("damaged-frame");
    write_month(&root, ym(2), 600, 42_000_000_000);
    seal(&root, ym(2));
    let at = month_path(ym(2))
        .with_file(FileKind::SealedBars)
        .to_path_buf(&root);
    let mut bytes = std::fs::read(&at).expect("sealed");
    let last = bytes.len() - 2;
    bytes[last] ^= 0x08;
    std::fs::write(&at, bytes).expect("damaged");
    let file = store::file::BarFile::open_existing(&root, month_path(ym(2)), symbol_id())
        .expect("the head still opens");
    // Rows 512..600 are the last frame. The page reads 500..600: twelve rows
    // read, eighty-eight refused, each named, none invented.
    let (rows, faults) = page(&file, 500, 100);
    assert_eq!(rows.len(), 12);
    let said = faults.sentences();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("(88)"), "{said:?}");
    assert!(said[0].contains("sealed month is refused"), "{said:?}");
    let _ = std::fs::remove_dir_all(&root);
}

/// `docs/06-limits.md` L-SC-24 and L-SC-25, held to what was measured and to
/// the source they name (verify-SC-3-r1). L-SC-24 once quoted `/calendar.json`
/// at 0.25 ms over sealed months when every timed answer had been a 202 or a
/// 503, and said every max was under 50 ms when the first request after a
/// start costs more in both forms. So: the warm row says its figures count
/// only answers of 200 and no longer denies a max over 50 ms; the first-request
/// row names that cost in both forms, the function that pays its floor, the
/// auction row it shares, and the bound this module enforces, read from the
/// constant itself so the bound cannot drift from the code.
#[test]
fn the_sealed_route_limits_time_answers_and_name_first_requests() {
    const LIMITS: &str = include_str!("../../../docs/06-limits.md");
    const LINEAGE: &str = include_str!("bars_change.rs");
    const SORT_INDEX: &str = include_str!("../../store/src/sort_index.rs");
    let row = |id: &str| {
        LIMITS
            .lines()
            .find(|line| line.starts_with(&format!("| {id} |")))
            .unwrap_or_else(|| panic!("docs/06-limits.md has no row {id}"))
    };
    let warm = row("L-SC-24");
    assert!(warm.contains("every answer 200"), "{warm}");
    assert!(warm.contains("HTTP status"), "{warm}");
    assert!(!warm.contains("every max under 50 ms"), "{warm}");
    assert!(warm.contains("L-SC-25"), "{warm}");
    let first = row("L-SC-25");
    for named in [
        "first request",
        "sealed",
        "raw",
        "`bars::change::Lineage::read`",
        "`imports/gdfl.lineage`",
        "L-AG04-02",
        "every answer 200",
        "over 50 ms",
    ] {
        assert!(first.contains(named), "L-SC-25 names {named}: {first}");
    }
    let bound = format!("`MAX_WINDOW_MONTHS` = {MAX_WINDOW_MONTHS}");
    assert!(first.contains(&bound), "L-SC-25 quotes {bound}: {first}");
    // The floor it names is still the source's shape: a ledger whose stamp is
    // new is read and parsed whole, once, and kept until the stamp moves.
    assert!(
        LINEAGE.contains("let ledger = Arc::new(parse_ledger(&text));"),
        "bars_change.rs no longer parses the whole lineage ledger on a new stamp: L-SC-25 is stale"
    );
    // A cost paid on EVERY request is not a first request's, and a shadow's
    // artefact is not the form's (the final re-check): on the standard linked
    // shadow the sorted pages over 97 months read the rows of the months whose
    // repair revision the shadow cannot see, because `shadow.sh` does not clone
    // `bar-generations-v1` and the original `.bin` has no `.srt`. The row says
    // so in the words the answer uses, and those words are still the store's.
    for named in [
        "every request",
        "`bar-generations-v1`",
        "the month has no .srt sort index",
        "`.srt`",
    ] {
        assert!(first.contains(named), "L-SC-25 names {named}: {first}");
    }
    assert!(
        SORT_INDEX.contains(r#"Self::Absent => f.write_str("the month has no .srt sort index")"#),
        "store::sort_index no longer names a month with no .srt so: L-SC-25 quotes stale words"
    );
}
