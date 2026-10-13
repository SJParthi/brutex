#![cfg(test)]
//! Tests of the GDFL import runtime. Every value is invented at run time;
//! no vendor row is quoted (`gdfl_cm::tests::fixtures_are_built_not_pasted`).

#![allow(
    clippy::expect_used,
    clippy::panic,
    clippy::case_sensitive_file_extension_comparisons,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brutex_core::instrument::Contract;
use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe};

use super::*;
use crate::gdfl_archive::Archive;
use crate::gdfl_fixtures::{Method, bts, csv, mon, put, row, scratch, unrun, zip};
use crate::gdfl_nfo::{NfoTickStore, NfoZips};
use crate::gdfl_tickstore::TickStore;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// Three regular sessions and the Saturday after them.
fn d1() -> Day {
    day(2024, 4, 1)
}
fn d2() -> Day {
    day(2024, 4, 2)
}
fn d3() -> Day {
    day(2024, 4, 3)
}

const NINE_FIFTEEN: u32 = 9 * 3_600 + 15 * 60;
const CLOSE: u32 = 15 * 3_600 + 30 * 60;

/// An invented build commit in the shape a provenance record takes: forty
/// lower-case hexadecimal digits (D-3334).
const COMMIT: &str = "0123abcd0123abcd0123abcd0123abcd0123abcd";

fn tick(sod: u32, ltp: i64, ltq: u64) -> Tick {
    Tick {
        sod,
        ltp,
        ltq,
        oi: None,
    }
}

// ── the bar: row rule, placement, fold ─────────────────────────────────────

#[test]
fn an_index_second_uses_every_row_and_carries_no_volume() {
    let ticks = [
        tick(NINE_FIFTEEN, 1_000, 0),
        tick(NINE_FIFTEEN, 1_030, 0),
        tick(NINE_FIFTEEN, 990, 0),
        tick(NINE_FIFTEEN + 1, 1_010, 7),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(got.seconds.len(), 2);
    let first = got.seconds[0];
    assert_eq!(
        (first.open, first.high, first.low, first.close),
        (1_000, 1_030, 990, 990)
    );
    assert_eq!(first.volume, 0, "index volume is 0, never null");
    assert_eq!(
        got.seconds[1].volume, 0,
        "even where a row states a quantity"
    );
    assert_eq!(first.open_interest, OI_NULL);
    // 2024-04-01 09:15:00 IST is 03:45:00 UTC.
    assert_eq!(first.ts_micros, 1_711_943_100 * 1_000_000);
    assert_eq!(got.placement.ltq_zero_dropped, 0);
}

#[test]
fn a_stock_or_option_second_uses_traded_rows_only_and_sums_their_quantity() {
    let ticks = [
        tick(NINE_FIFTEEN, 500, 0),
        tick(NINE_FIFTEEN, 510, 10),
        tick(NINE_FIFTEEN, 505, 0),
        tick(NINE_FIFTEEN, 520, 5),
        tick(NINE_FIFTEEN, 495, 1),
        tick(NINE_FIFTEEN + 3, 600, 0),
    ];
    for kind in [ImportKind::Stocks, ImportKind::Options] {
        let got = convert(kind, d1(), &ticks).unwrap();
        assert_eq!(
            got.seconds.len(),
            1,
            "a second with no traded row has no bar"
        );
        let bar = got.seconds[0];
        assert_eq!(
            (bar.open, bar.high, bar.low, bar.close),
            (510, 520, 495, 495)
        );
        assert_eq!(bar.volume, 16);
        assert_eq!(got.placement.ltq_zero_dropped, 3);
        assert_eq!(got.placement.rows, 6);
    }
}

/// D-3342 (the operator's answer: bar-close semantics), `BAR_DEFINITION` 3: an
/// option second's open interest is the OI of the LAST row placed in it,
/// traded or not; its prices and volume stay the traded rows'. The shape of
/// DB-083's BANKNIFTY28MAR1926500CE at 14:14:30, a later untraded row in the
/// second carrying another OI, with invented values.
#[test]
fn an_option_second_carries_the_open_interest_of_its_last_row_traded_or_not() {
    let with = |sod, ltp, ltq, oi| Tick {
        sod,
        ltp,
        ltq,
        oi: Some(oi),
    };
    let got = convert(
        ImportKind::Options,
        d1(),
        &[
            with(NINE_FIFTEEN, 5, 1, 100),
            with(NINE_FIFTEEN, 6, 2, 0),
            with(NINE_FIFTEEN, 7, 0, 999),
            // A second whose only rows are untraded has no bar, whatever its
            // OI says.
            with(NINE_FIFTEEN + 1, 8, 0, 4_242),
            // A late untraded row lands in the next in-order row's second
            // and is that second's last row.
            with(NINE_FIFTEEN + 3, 9, 4, 300),
            with(NINE_FIFTEEN + 2, 1, 0, 777),
            with(NINE_FIFTEEN + 3, 10, 1, 301),
            with(NINE_FIFTEEN + 3, 11, 0, 302),
        ],
    )
    .unwrap();
    let ois: Vec<i64> = got.seconds.iter().map(|bar| bar.open_interest).collect();
    assert_eq!(ois, [999, 302], "{:?}", got.seconds);
    let closes: Vec<i64> = got.seconds.iter().map(|bar| bar.close).collect();
    assert_eq!(closes, [6, 10], "prices are the traded rows'");
    let volumes: Vec<i64> = got.seconds.iter().map(|bar| bar.volume).collect();
    assert_eq!(volumes, [3, 5], "volume is the traded rows'");
    // Zero is zero: an untraded last row of OI 0 sets it.
    let zero = convert(
        ImportKind::Options,
        d1(),
        &[with(NINE_FIFTEEN, 5, 1, 100), with(NINE_FIFTEEN, 6, 0, 0)],
    )
    .unwrap();
    assert_eq!(zero.seconds[0].open_interest, 0);
    // An index or a share is untouched: no row carries an OI.
    let spot = convert(
        ImportKind::Stocks,
        d1(),
        &[tick(NINE_FIFTEEN, 5, 1), tick(NINE_FIFTEEN, 6, 0)],
    )
    .unwrap();
    assert_eq!(spot.seconds[0].open_interest, OI_NULL);
    assert_eq!(
        BAR_DEFINITION, 4,
        "D-3342 changed the bars an option file builds (3), and the closing \
         auction's band an eligible share's from 2026-08-03 (4, D-4878 item 10)"
    );
}

#[test]
fn a_late_row_is_deferred_to_the_next_in_order_second_never_earlier() {
    let ticks = [
        tick(NINE_FIFTEEN, 100, 0),
        tick(NINE_FIFTEEN + 5, 105, 0),
        // Two late rows, back-stepping 3 s and 5 s.
        tick(NINE_FIFTEEN + 2, 999, 0),
        tick(NINE_FIFTEEN, 1, 0),
        tick(NINE_FIFTEEN + 7, 107, 0),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    let secs: Vec<i64> = got.seconds.iter().map(|b| b.ts_micros).collect();
    let at = |s: u32| micros_at(d1(), s);
    assert_eq!(
        secs,
        vec![at(NINE_FIFTEEN), at(NINE_FIFTEEN + 5), at(NINE_FIFTEEN + 7)]
    );
    let landed = got.seconds[2];
    assert_eq!(
        (landed.open, landed.high, landed.low, landed.close),
        (999, 999, 1, 107)
    );
    assert_eq!(got.placement.late_rows, 2);
    assert_eq!(got.placement.max_back_s, 5);
    assert_eq!(got.placement.late_unresolved, 0);
    // The 09:15:05 second is untouched by the deferred prices.
    assert_eq!(got.seconds[1].high, 105);
}

#[test]
fn a_late_row_with_nothing_after_it_is_dropped_and_counted() {
    let ticks = [
        tick(NINE_FIFTEEN + 10, 100, 0),
        tick(NINE_FIFTEEN, 1, 0),
        tick(NINE_FIFTEEN + 1, 2, 0),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(got.seconds.len(), 1);
    assert_eq!(got.placement.late_unresolved, 2);
    assert_eq!(got.placement.max_back_s, 10);
}

#[test]
fn a_utc_stamped_row_is_deferred_like_any_late_row() {
    let ticks = [
        tick(NINE_FIFTEEN + 19_800, 100, 0),
        tick(NINE_FIFTEEN, 7, 0),
        tick(NINE_FIFTEEN + 19_801, 101, 0),
    ];
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(got.placement.max_back_s, 19_800);
    assert_eq!(got.seconds[1].low, 7, "carried to the next in-order second");
}

#[test]
fn duplicate_stamps_halts_and_frozen_runs_are_kept_as_printed() {
    let mut ticks = vec![tick(NINE_FIFTEEN, 100, 0); 50];
    // A halt: nothing for ten minutes, then a frozen run of one price.
    for s in 0..30 {
        ticks.push(tick(NINE_FIFTEEN + 600 + s, 100, 0));
    }
    let got = convert(ImportKind::Indices, d1(), &ticks).unwrap();
    assert_eq!(
        got.seconds.len(),
        31,
        "one bar for fifty rows of one second, none for the halt"
    );
    assert!(got.seconds.iter().all(|b| b.open == 100 && b.close == 100));
}

#[test]
fn empty_and_untraded_files_make_no_bar_and_no_refusal() {
    assert!(
        convert(ImportKind::Stocks, d1(), &[])
            .unwrap()
            .seconds
            .is_empty()
    );
    let untraded = [tick(NINE_FIFTEEN, 1, 0), tick(NINE_FIFTEEN + 1, 1, 0)];
    let got = convert(ImportKind::Options, d1(), &untraded).unwrap();
    assert!(got.seconds.is_empty());
    assert_eq!(got.placement.ltq_zero_dropped, 2);
}

#[test]
fn a_quantity_past_i64_is_refused_never_saturated() {
    let big = u64::try_from(i64::MAX).unwrap() + 1;
    assert_eq!(
        convert(ImportKind::Stocks, d1(), &[tick(NINE_FIFTEEN, 1, big)]),
        Err(ImportRefusal::VolumeOverflow { sod: NINE_FIFTEEN })
    );
    let max = u64::try_from(i64::MAX).unwrap();
    let one = convert(ImportKind::Stocks, d1(), &[tick(NINE_FIFTEEN, 1, max)]).unwrap();
    assert_eq!(one.seconds[0].volume, i64::MAX);
    assert_eq!(
        convert(
            ImportKind::Stocks,
            d1(),
            &[tick(NINE_FIFTEEN + 9, 1, max), tick(NINE_FIFTEEN + 9, 1, 1)]
        ),
        Err(ImportRefusal::VolumeOverflow {
            sod: NINE_FIFTEEN + 9
        })
    );
}

#[test]
fn a_negative_price_reaches_the_fold_and_is_refused_by_it() {
    // No reader hands one on (both refuse below one tick); the fold's own
    // refusals still name themselves here.
    let got = convert(ImportKind::Indices, d1(), &[tick(NINE_FIFTEEN, -5, 0)]);
    assert!(got.is_ok(), "the fold folds what it is given: {got:?}");
    assert_eq!(second_of(micros_at(d1(), 77), d1()), 77);
    assert_eq!(second_of(i64::MIN, d1()), 0);
}

#[test]
fn kinds_name_their_words_segments_and_venues() {
    for kind in ImportKind::ALL {
        assert_eq!(ImportKind::parse(kind.as_str()), Ok(kind));
    }
    assert_eq!(
        ImportKind::parse("futures"),
        Err(ImportRefusal::KindUnknown {
            word: "futures".to_owned()
        })
    );
    assert_eq!(
        ImportKind::ALL.map(ImportKind::segment),
        ["INDEX", "CASH", "FNO"]
    );
    assert_eq!(
        ImportKind::ALL.map(ImportKind::listing),
        [Listing::Index, Listing::Equity, Listing::Derivative]
    );
    assert_eq!(
        ImportKind::ALL.map(ImportKind::every_row_counts),
        [true, false, false]
    );
}

#[test]
fn every_refusal_says_what_it_is() {
    let all = [
        ImportRefusal::KindUnknown { word: "w".into() },
        ImportRefusal::RangeBackwards {
            from: d2(),
            to: d1(),
        },
        ImportRefusal::VolumeOverflow { sod: 3 },
        ImportRefusal::Fold { why: "f".into() },
        ImportRefusal::Journal { why: "j".into() },
        ImportRefusal::FilterName { name: "a b".into() },
        ImportRefusal::StampPastTheDay { sod: 86_400 },
        refused_at_d3(Ok(room(MAX_ENTRIES - 1, 3, 3))),
        refused_at_d3(Ok(room(10, 3, 3))),
        refused_at_d3(Err("unreadable".into())),
        ImportRefusal::Filing { why: "s".into() },
        ImportRefusal::DayNotHeld { day: d2() },
    ];
    let texts: Vec<String> = all.iter().map(ToString::to_string).collect();
    let unique: std::collections::HashSet<&String> = texts.iter().collect();
    assert_eq!(unique.len(), texts.len());
    assert!(texts[1].contains("2024-04-02"));
    // Each variant's code is its snake-case name, but the full census's,
    // which is the census's own refusal (D-3317 items 6 and 7).
    assert_eq!(
        all.each_ref().map(ImportRefusal::code),
        [
            "kind_unknown",
            "range_backwards",
            "volume_overflow",
            "fold",
            "journal",
            "filter_name",
            "stamp_past_the_day",
            "too_many_entries",
            "census_refused",
            "census_refused",
            "filing",
            "day_not_held",
        ]
    );
    // The ceiling names the FILE's numbers, the batch and what was left.
    let full = &texts[7];
    for part in [
        "cannot take options 2024-04-03",
        "n_valid=2097151",
        "limit=2097152",
        "free=1",
        "batch=3",
        "new=3",
        "does not fit",
        "(the census's own words: words)",
        "nothing of the day's batch was published",
        "the run stopped there after 2 day(s) imported",
    ] {
        assert!(full.contains(part), "{full} lacks {part:?}");
    }
    assert!(
        texts[8].contains("although the day's batch fits") && texts[8].contains("free=2097142"),
        "{}",
        texts[8]
    );
    assert!(
        texts[9].contains("its room could not be measured (unreadable)"),
        "{}",
        texts[9]
    );
    // The journal's `reason=` is the refusal's code, by the same test.
    for (at, measured) in [
        (7, Ok(room(MAX_ENTRIES - 1, 3, 3))),
        (8, Ok(room(10, 3, 3))),
        (9, Err("unreadable".to_owned())),
    ] {
        assert_eq!(census_code(&measured), all[at].code());
    }
    // Only a census refusal carries what the run did before it stopped.
    assert_eq!(all[7].report().map(|r| r.days_imported), Some(2));
    assert_eq!(all[9].report().map(|r| r.days_imported), Some(2));
    assert!(all[0].report().is_none() && all[11].report().is_none());
}

// ── D-3317: the census ceiling, the day ledger, the provenance ─────────────

fn room(n_valid: u64, batch: u64, new: u64) -> CensusRoom {
    CensusRoom {
        n_valid,
        limit: MAX_ENTRIES,
        batch,
        new,
    }
}

/// The census refusal of options on day three, through the runtime's own
/// choice of variant ([`census_refusal`]), after two days imported.
fn refused_at_d3(room: Result<CensusRoom, String>) -> ImportRefusal {
    let done = Report {
        days_imported: 2,
        ..Report::default()
    };
    census_refusal((ImportKind::Options, d3()), "words".into(), room, done)
}

/// A census double filled to `n_valid` of the real [`MAX_ENTRIES`]: it
/// publishes a batch only when the entries it does not already hold fit, and
/// otherwise refuses in the real census's own words, quoting -- as the real
/// one does -- its counter in memory at the first append past the ceiling. A
/// census at the ceiling is a 268 MB file, which no unit test builds.
struct Tight {
    n_valid: u64,
    held: HashMap<crate::manifest::EntryKey, Held>,
    rooms: std::cell::Cell<usize>,
}

impl Tight {
    fn below_the_ceiling_by(free: u64) -> Self {
        Self {
            n_valid: MAX_ENTRIES - free,
            held: HashMap::new(),
            rooms: std::cell::Cell::new(0),
        }
    }

    fn new_in(&self, held: &[Held]) -> Vec<Held> {
        held.iter()
            .filter(|one| self.held.get(&one.entry.key) != Some(*one))
            .copied()
            .collect()
    }
}

impl Census for Tight {
    fn publish(&mut self, _: &Path, held: &[Held]) -> Option<String> {
        let new = self.new_in(held);
        let free = MAX_ENTRIES - self.n_valid;
        if new.len() as u64 > free {
            // The census's own up-front refusal (D-3316 item 5, WP-10): the
            // file's n_valid, the batch and the limit, before any row is
            // recorded.
            return Some(
                crate::manifest::ManifestError::TooManyEntries {
                    n_valid: self.n_valid,
                    batch: new.len() as u64,
                    limit: MAX_ENTRIES,
                }
                .to_string(),
            );
        }
        for one in new {
            self.held.insert(one.entry.key, one);
            self.n_valid += 1;
        }
        None
    }

    fn room(&self, _: &Path, held: &[Held]) -> Result<CensusRoom, String> {
        self.rooms.set(self.rooms.get() + 1);
        Ok(room(
            self.n_valid,
            held.len() as u64,
            self.new_in(held).len() as u64,
        ))
    }
}

fn d4() -> Day {
    day(2024, 4, 4)
}
fn d5() -> Day {
    day(2024, 4, 5)
}

/// The `run=` id every closing line of `text` carries, all of them one id.
fn run_ids(text: &str) -> std::collections::BTreeSet<u64> {
    text.lines()
        .filter(|line| !line.starts_with("run ") && !line.starts_with("begin "))
        .map(|line| {
            let field = line
                .split(' ')
                .find_map(|field| field.strip_prefix("run="))
                .unwrap_or_else(|| panic!("a closing line without run=: {line}"));
            field.parse().unwrap()
        })
        .collect()
}

/// THE PLAN'S FIXTURE (WP-11a; DB-G01, DB-G10, DB-058): the census preset
/// ten entries below its ceiling, three new contract-months a day. Days one
/// to three fit (ten, seven, four free); the fourth's batch of three meets
/// one free slot, and the run stops THERE: the refusal names the file's
/// `n_valid`, the batch and the free slots, nothing of the batch is published,
/// the day is journalled `incomplete ... stage=census reason=too_many_entries`,
/// it is not counted imported, and the fifth day is never read.
#[test]
fn the_run_stops_at_the_first_day_whose_batch_does_not_fit_the_census() {
    let root = scratch("import-ceiling");
    world(&root, &[d1(), d2(), d3(), d4(), d5()]);
    let store = root.join("S");
    let source = NfoTickStore::new(&root.join("ts"));
    let run = Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d5(),
        only: &[],
        store_root: &store,
    };
    let mut census = Tight::below_the_ceiling_by(10);
    let mut read = Vec::new();
    let got = drive_with(&run, RunOptions::default(), &mut census, |on, sink| {
        read.push(on);
        nfo_day(&source, &run, on, sink)
    })
    .unwrap_err();
    assert_eq!(
        read,
        vec![d1(), d2(), d3(), d4()],
        "the fifth day is never read"
    );
    assert_eq!(
        census.rooms.get(),
        1,
        "the room is measured once, on the refusal"
    );
    assert_eq!(
        census.n_valid,
        MAX_ENTRIES - 1,
        "nothing of day four published"
    );
    assert_eq!(got.code(), "too_many_entries");
    let ImportRefusal::CensusFull {
        kind,
        day: at,
        why,
        room: measured,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *at), (ImportKind::Options, d4()));
    assert_eq!(*measured, room(MAX_ENTRIES - 1, 3, 3));
    assert!(!measured.fits() && measured.free() == 1);
    assert_eq!(
        why,
        "n_valid is 2097151 of a capacity of 2097152 (1 free); a batch of 3 entries does not fit",
        "the census's own words, kept"
    );
    assert_eq!(done.days_imported, 3, "day four is not counted imported");
    assert!(done.resumed.is_empty());
    assert!(
        done.closed_incomplete.is_empty(),
        "the census day ends the run; the refusal names it"
    );
    let text = got.to_string();
    for part in [
        "options 2024-04-04",
        "n_valid=2097151",
        "free=1",
        "batch=3",
        "new=3",
        "does not fit",
        "nothing of the day's batch was published",
        "after 3 day(s) imported",
    ] {
        assert!(text.contains(part), "{text} lacks {part:?}");
    }
    // Day four's bars are on disk, uncounted: the report says so, and they are.
    let ce = bars(&store, "FNO", "NIFTY", Some(nifty_ce()));
    assert!(
        ce.iter()
            .any(|b| b.ts_micros >= micros_at(d4(), 0) && b.ts_micros < micros_at(d5(), 0)),
        "day four's bars landed"
    );
    assert_eq!(done.files, 12, "four days of three files: {done:?}");
    let census_failure = done
        .failures
        .iter()
        .find(|f| f.instrument == "2024-04-04 census")
        .unwrap();
    assert!(
        census_failure.why.starts_with(
            "the census is full, so nothing of the batch was published: n_valid=2097151 \
             limit=2097152 free=1 batch=3 new=3 (the census's own words: n_valid is 2097151 \
             of a capacity of 2097152 (1 free); a batch of 3 entries does not fit)"
        ),
        "{}",
        census_failure.why
    );
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    let lines: Vec<&str> = journal.lines().collect();
    assert!(lines[0].starts_with("run "), "{journal}");
    for d in ["2024-04-01", "2024-04-02", "2024-04-03"] {
        assert!(
            lines.iter().any(|l| l.starts_with(&format!(
                "done options {d} * definition={BAR_DEFINITION} files=3 "
            )) && !l.contains(" stage=")
                && !l.contains(" vendor_refused=")),
            "{journal}"
        );
    }
    let last = lines.last().unwrap();
    assert!(
        last.starts_with(&format!(
            "incomplete options 2024-04-04 * definition={BAR_DEFINITION} files=3 "
        )) && last.contains(" failures=1 ")
            && last.contains(" stage=census reason=too_many_entries run="),
        "{journal}"
    );
    assert!(!journal.contains("2024-04-05"), "{journal}");
    assert_eq!(run_ids(&journal).len(), 1, "one run: {journal}");

    // With room, the next run takes day four up again from the journal and
    // carries on: the stop is resumable, never a loss.
    let mut roomy = Tight::below_the_ceiling_by(1_000);
    let again = drive_with(&run, RunOptions::default(), &mut roomy, |on, sink| {
        nfo_day(&source, &run, on, sink)
    })
    .unwrap();
    assert_eq!(again.days_skipped, 3);
    assert_eq!(again.resumed, vec![d4()]);
    assert_eq!(again.days_imported, 2, "{again:?}");
    assert_eq!(roomy.rooms.get(), 0, "no refusal, no second census read");
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert_eq!(run_ids(&journal).len(), 2, "two runs, two ids: {journal}");
    assert!(
        journal.contains("\ndone options 2024-04-04 * ")
            && journal.contains("\ndone options 2024-04-05 * "),
        "{journal}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A census that refuses a batch that FITS still stops the run (D-3317): a
/// lock another run holds, an install that fails. Continuing would file the
/// next day's bars uncounted as well.
#[test]
fn a_census_refusal_that_is_not_the_ceiling_stops_the_run_too() {
    struct Busy;
    impl Census for Busy {
        fn publish(&mut self, _: &Path, _: &[Held]) -> Option<String> {
            Some("another ingest holds the census lock".to_owned())
        }
        fn room(&self, _: &Path, held: &[Held]) -> Result<CensusRoom, String> {
            Ok(room(10, held.len() as u64, held.len() as u64))
        }
    }
    let root = scratch("import-busy");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    let source = TickStore::new(&root.join("ts"));
    let run = Run {
        kind: ImportKind::Stocks,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: &store,
    };
    let got = drive_with(&run, RunOptions::default(), &mut Busy, |on, sink| {
        cm_day(&source, &run, CmKind::Stocks, on, sink)
    })
    .unwrap_err();
    assert_eq!(got.code(), "census_refused");
    let text = got.to_string();
    assert!(
        text.contains("refused stocks 2024-04-01 although the day's batch fits")
            && text.contains("another ingest holds the census lock"),
        "{text}"
    );
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        journal
            .lines()
            .last()
            .unwrap()
            .contains(" stage=census reason=census_refused run="),
        "{journal}"
    );
    assert!(!journal.contains("2024-04-02"));
    std::fs::remove_dir_all(root).unwrap();
}

/// The real census's room, read from its file (D-3317; DB-G10): after a run
/// it holds the entries the run counted; offered again, those are not new; an
/// absent census is empty; a file that is not one is named; a store that has
/// moved to D-3316's version-4 census is not measured from the frozen file.
#[test]
fn the_census_room_is_read_from_the_census_file_itself() {
    let root = scratch("import-room");
    world(&root, &[d1()]);
    let store = root.join("S");
    assert_eq!(
        census_room(&store, &[]),
        Ok(room(0, 0, 0)),
        "absent is empty"
    );
    import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    let bytes = std::fs::read(manifest_path(&store, Vendor::Gdfl)).unwrap();
    let census = Manifest::open_image(Vendor::Gdfl, &bytes).unwrap();
    let held: Vec<Held> = census
        .held_keys()
        .map(|key| census.held(key).unwrap())
        .collect();
    assert_eq!(held.len(), 3, "three contract-months");
    let got = census_room(&store, &held).unwrap();
    assert_eq!(
        got,
        room(census.entries(), 3, 0),
        "held exactly: nothing new"
    );
    assert!(got.fits() && got.free() == MAX_ENTRIES - census.entries());
    let mut moved = held.clone();
    moved[0].entry.rows += 1;
    assert_eq!(
        census_room(&store, &moved).unwrap().new,
        1,
        "one changed row"
    );
    let v4 = store.join("manifest/v4/gdfl");
    std::fs::create_dir_all(&v4).unwrap();
    assert!(
        census_room(&store, &held).is_err_and(|why| why.contains("version-4 census")),
        "a version-4 store names its own room"
    );
    std::fs::remove_dir_all(&v4).unwrap();
    std::fs::remove_file(manifest_path(&store, Vendor::Gdfl)).unwrap();
    std::fs::create_dir_all(manifest_path(&store, Vendor::Gdfl)).unwrap();
    assert!(
        census_room(&store, &held).is_err_and(|why| why.contains("not a regular file")),
        "a directory is not a census"
    );
    std::fs::remove_dir_all(manifest_path(&store, Vendor::Gdfl)).unwrap();
    std::fs::write(manifest_path(&store, Vendor::Gdfl), b"short").unwrap();
    assert!(
        census_room(&store, &held).is_err(),
        "a torn census is named"
    );
    assert_eq!(CENSUS_CAP, HEADER_LEN + MAX_ENTRIES * ENTRY_STRIDE);
    std::fs::remove_dir_all(root).unwrap();
}

/// The journal line a day closes with, for the day `on` of `kind`.
fn line_of(store: &Path, kind: &str, on: &str) -> String {
    let text = std::fs::read_to_string(journal_path(store)).unwrap();
    text.lines()
        .rev()
        .find(|l| {
            !l.starts_with("begin ")
                && l.split(' ').nth(1) == Some(kind)
                && l.split(' ').nth(2) == Some(on)
        })
        .unwrap_or_else(|| panic!("no closing line for {kind} {on} in {text}"))
        .to_owned()
}

/// The closing line `line` less its `run=` field, which names the run.
fn without_run(line: &str) -> String {
    let (head, run) = line.rsplit_once(" run=").unwrap();
    assert!(run.parse::<u64>().is_ok(), "{line}");
    head.to_owned()
}

/// DB-025: a day whose every file was refused gets a `refused` line naming
/// the stage and the reason, where it used to get none. It is not done, so
/// the next run reads it again and names it resumed; refused again the same
/// way, its line is not written twice.
#[test]
fn a_day_whose_every_file_is_refused_is_journalled_refused_with_its_reason() {
    let root = scratch("import-all-refused");
    let folder = crate::gdfl_cm::day_folder_name(CmKind::Indices, d1());
    // Both index files stop inside the session: the reader refuses each.
    let cut = |stem: &str| csv(&[row(stem, d1(), NINE_FIFTEEN, "100.00", 0, 0)]);
    let entries = vec![
        (
            format!("{folder}/NIFTY 50.NSE_IDX.csv"),
            cut("NIFTY 50.NSE_IDX"),
        ),
        (
            format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
            cut("NIFTY BANK.NSE_IDX"),
        ),
    ];
    put(
        &root,
        "ts/cm/INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert_eq!((got.files, got.files_refused), (0, 2), "{:?}", got.failures);
    assert_eq!(got.closed_refused, vec![d1()]);
    let line = line_of(&store, "indices", "2024-04-01");
    assert_eq!(
        without_run(&line),
        format!(
            "refused indices 2024-04-01 * definition={BAR_DEFINITION} stage=vendor \
             reason=index_ends_in_session failures=2"
        ),
    );
    assert!(got.closed_with_refusals.is_empty());
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(!journal.contains("begin "), "it wrote nothing: {journal}");
    let again = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert_eq!(
        (again.days_skipped, again.days_imported),
        (0, 1),
        "read again"
    );
    assert_eq!(again.resumed, vec![d1()], "a refused day is named resumed");
    assert_eq!(
        std::fs::read_to_string(journal_path(&store)).unwrap(),
        journal,
        "refused again the same way: nothing written, not even a run line"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// DB-025: a day the calendar declines and a day the source does not hold
/// are holes with a reason in the journal, each closed `refused`; a re-run
/// that meets them again writes nothing. (A special session was the
/// calendar's example until D-3319 read it inside its windows; a day before
/// the calendar's spans is one now.)
#[test]
fn a_calendar_refused_day_and_a_missing_day_each_leave_a_refused_line() {
    let root = scratch("import-holes");
    world(&root, &[d1()]);
    let store = root.join("S");
    let special = import(
        &root,
        Src::Store,
        ImportKind::Indices,
        day(2017, 12, 29),
        day(2017, 12, 29),
        &store,
    );
    assert_eq!(special.days_refused, 1);
    assert_eq!(
        without_run(&line_of(&store, "indices", "2017-12-29")),
        format!(
            "refused indices 2017-12-29 * definition={BAR_DEFINITION} stage=calendar \
             reason=calendar_unmeasured failures=1"
        )
    );
    let missing = import(&root, Src::Store, ImportKind::Stocks, d1(), d2(), &store);
    assert_eq!(missing.days_missing, vec![d2()]);
    assert!(
        missing.failures.is_empty(),
        "a missing day names no failure"
    );
    assert_eq!(
        without_run(&line_of(&store, "stocks", "2024-04-02")),
        format!(
            "refused stocks 2024-04-02 * definition={BAR_DEFINITION} stage=vendor \
             reason=day_not_held failures=0"
        )
    );
    assert!(line_of(&store, "stocks", "2024-04-01").starts_with("done stocks 2024-04-01 "));
    let before = std::fs::read_to_string(journal_path(&store)).unwrap();
    let rerun = import(&root, Src::Store, ImportKind::Stocks, d1(), d2(), &store);
    assert_eq!(rerun.days_skipped, 1);
    assert_eq!(rerun.resumed, vec![d2()], "taken up again and named");
    let special = import(
        &root,
        Src::Zips,
        ImportKind::Indices,
        day(2017, 12, 29),
        day(2017, 12, 29),
        &store,
    );
    assert_eq!(special.days_refused, 1);
    assert_eq!(
        std::fs::read_to_string(journal_path(&store)).unwrap(),
        before,
        "the same holes, the same journal, byte for byte"
    );
    // A day refused for another reason this time is written again.
    let unmeasured = import(
        &root,
        Src::Store,
        ImportKind::Stocks,
        day(2011, 1, 3),
        day(2011, 1, 3),
        &store,
    );
    assert_eq!(unmeasured.days_refused, 1);
    assert_eq!(
        without_run(&line_of(&store, "stocks", "2011-01-03")),
        "refused stocks 2011-01-03 * definition=4 stage=calendar reason=calendar_unmeasured \
         failures=1"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The options world of `d1` with one more contract whose LTP has three
/// places: the vendor's bytes, refused by the reader's named price rule.
fn with_a_bad_price(root: &Path) {
    let folder = crate::gdfl_nfo::day_folder_name(d1());
    let mut entries = nfo_entries(d1());
    entries.push((
        format!("{folder}\\Options\\NIFTY04APR2422100CE.NFO.csv"),
        option_file("NIFTY04APR2422100CE", d1(), "101.505"),
    ));
    put(
        root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
}

/// DB-G09: a day with only the vendor's own named refusals closes DONE WITH
/// REFUSALS, and a re-run skips it instead of rereading it for ever; given
/// `retry_refused` it is read again, named resumed, and closes the same way.
#[test]
fn a_day_with_only_vendor_refusals_closes_done_and_is_not_read_again() {
    let root = scratch("import-done-refusals");
    with_a_bad_price(&root);
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!((got.files, got.files_refused), (3, 1), "{:?}", got.failures);
    assert_eq!(got.closed_with_refusals, vec![d1()]);
    assert!(got.closed_incomplete.is_empty());
    let line = line_of(&store, "options", "2024-04-01");
    let want = format!(
        "done options 2024-04-01 * definition={BAR_DEFINITION} files=3 seconds={} failures=1 late={} \
         late_unresolved={} max_back_s={} vendor_refused=1 reason=price_refused",
        got.seconds, got.late_rows, got.late_unresolved, got.max_back_s
    );
    assert_eq!(without_run(&line), want);
    let again = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!((again.days_skipped, again.days_imported), (1, 0));
    // `retry_refused` reads it again: how a day is re-read after a reader
    // learns to accept what it refused.
    let run = Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    let retry = RunOptions {
        origin: None,
        retry_refused: true,
        ..RunOptions::default()
    };
    let retried = run_nfo_from(&NfoTickStore::new(&root.join("ts")), &run, retry).unwrap();
    assert_eq!((retried.days_skipped, retried.days_imported), (0, 1));
    assert_eq!(retried.resumed, vec![d1()]);
    assert!(retried.restated.is_empty());
    assert_eq!(retried.seconds_committed, 0, "nothing written twice");
    assert_eq!(retried.closed_with_refusals, vec![d1()]);
    // A clean day is not read again under the flag: it has nothing to retry.
    world(&root, &[d1()]);
    let clean = root.join("C");
    import(&root, Src::Store, ImportKind::Options, d1(), d1(), &clean);
    let run = Run {
        store_root: &clean,
        ..run
    };
    let skipped = run_nfo_from(&NfoTickStore::new(&root.join("ts")), &run, retry).unwrap();
    assert_eq!((skipped.days_skipped, skipped.days_imported), (1, 0));
    std::fs::remove_dir_all(root).unwrap();
}

/// The loader reads every line D-3317 writes, and still refuses any other
/// (D-2814): the last closing line of a key wins.
#[test]
fn the_loader_reads_every_line_the_journal_writes_and_refuses_any_other() {
    let root = scratch("import-loader");
    let key = |d: u8| format!("stocks 2024-04-0{d} *");
    let text = [
        "run 7 source=zip archive=%2Fx archive_bytes=3 commit=unstamped started=7 \
         archive_cd_blake3=unmeasured"
            .to_owned(),
        format!("begin {}", key(1)),
        format!(
            "done {} definition=2 files=1 failures=1 vendor_refused=1 reason=price_refused run=7",
            key(1)
        ),
        format!("done {} definition=2 files=1 failures=0 run=7", key(2)),
        format!("begin {}", key(3)),
        format!(
            "incomplete {} definition=2 files=1 stage=file reason=filing run=7",
            key(3)
        ),
        format!(
            "refused {} definition=2 stage=vendor reason=day_not_held failures=0 run=7",
            key(4)
        ),
        format!("recounted {} definition=2 months=3 run=8", key(3)),
        format!(
            "refused {} definition=2 stage=calendar reason=x failures=1 run=7",
            key(5)
        ),
        format!("begin {}", key(5)),
        format!("recounted {} definition=1 months=1 run=8", key(6)),
        "recou".to_owned(),
    ]
    .join("\n");
    put(&root, "imports/gdfl.journal", text.as_bytes());
    let journal = Journal::load(journal_path(&root)).unwrap();
    assert!(journal.runs.contains(&7));
    assert!(journal.done.contains(&key(1)) && journal.with_refusals.contains(&key(1)));
    assert!(journal.done.contains(&key(2)) && !journal.with_refusals.contains(&key(2)));
    assert!(
        journal.done.contains(&key(3)) && !journal.incomplete.contains(&key(3)),
        "a recount closes an incomplete day"
    );
    assert!(journal.incomplete.contains(&key(4)) && !journal.done.contains(&key(4)));
    assert_eq!(
        journal.refused_close.get(&key(4)).map(String::as_str),
        Some(
            "refused stocks 2024-04-04 * definition=2 stage=vendor reason=day_not_held failures=0"
        )
    );
    assert!(
        journal.open.contains(&key(5)) && !journal.refused_close.contains_key(&key(5)),
        "a begin after a refused line must be closed again"
    );
    assert!(journal.restated.contains(&key(6)) && !journal.done.contains(&key(6)));
    assert!(
        std::fs::read_to_string(journal_path(&root))
            .unwrap()
            .ends_with("recou (torn)\n"),
        "the torn start of a recounted line is closed as torn"
    );
    for foreign in [
        "run x source=zip",
        "run 07 source=zip",
        "run 7 source",
        "done stocks 2024-04-01 * definition=2 vendor_refused=x",
        "done stocks 2024-04-01 * definition=2 run=-1",
        "incomplete stocks 2024-04-01 * definition=2 run=1 run=2",
        "refused stocks 2024-04-01 * definition=x",
        "refused stocks",
        "recounted stocks 2024-04-01 * definition=2 months=x",
        "recounted stocks 2024-04-01 * definition=2 run=x",
    ] {
        put(
            &root,
            "imports/gdfl.journal",
            format!("{foreign}\n").as_bytes(),
        );
        assert!(
            matches!(
                Journal::load(journal_path(&root)),
                Err(ImportRefusal::Journal { .. })
            ),
            "{foreign}"
        );
    }
    for torn in ["r", "ru", "run", "ref", "recounted st"] {
        assert!(torn_fragment(torn), "{torn}");
    }
    assert!(!torn_fragment("x"));
    assert_eq!(count_of(&["a=1", "b=2"], "b"), Ok(Some(2)));
    assert_eq!(count_of(&["a=1"], "b"), Ok(None));
    assert_eq!(count_of(&["b=01"], "b"), Err(()));
    assert_eq!(count_of(&["b=1", "b=1"], "b"), Err(()));
    assert_eq!(
        count_of(&["bb=1"], "b"),
        Ok(None),
        "a longer name is not it"
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn staged_at(cause: Cause, about: &str) -> Staged {
    Staged {
        cause,
        failure: failure(about, "w".into()),
    }
}

/// How a day closes, by its failures' causes and whether a file read clean
/// (D-3317 items 2, 3 and 7).
#[test]
fn a_days_close_follows_its_failures() {
    let vendor = staged_at(Cause::vendor("price_refused"), "A");
    let other_vendor = staged_at(Cause::vendor("malformed_row"), "B");
    let file = staged_at(Cause::open(Stage::File, "filing"), "C");
    let copy = staged_at(Cause::open(Stage::Vendor, "source_crc_mismatch"), "D");
    let calendar = staged_at(Cause::open(Stage::Calendar, "expiry_passed"), "E");
    let census = staged_at(Cause::open(Stage::Census, "too_many_entries"), "F");
    assert_eq!(close_of(&[], false), Close::Done);
    assert_eq!(close_of(&[], true), Close::Done);
    assert_eq!(
        close_of(&[vendor.clone(), other_vendor.clone()], true),
        Close::WithRefusals("price_refused"),
        "every failure closes: the first names the reason"
    );
    assert_eq!(
        close_of(std::slice::from_ref(&vendor), false),
        Close::Refused(vendor.cause),
        "nothing read clean: refused"
    );
    assert_eq!(
        close_of(&[vendor.clone(), file.clone(), copy.clone()], true),
        Close::Incomplete(file.cause),
        "the first failure that keeps the day open decides"
    );
    assert_eq!(
        close_of(&[vendor.clone(), copy.clone()], true),
        Close::Incomplete(copy.cause),
        "a damaged copy never closes a day"
    );
    assert_eq!(
        close_of(std::slice::from_ref(&calendar), false),
        Close::Refused(calendar.cause)
    );
    assert_eq!(
        close_of(&[vendor.clone(), file.clone(), census.clone()], true),
        Close::Incomplete(census.cause),
        "a census failure decides before any other"
    );
    assert_eq!(
        [
            Close::Done,
            Close::WithRefusals("x"),
            Close::Incomplete(file.cause),
            Close::Refused(file.cause)
        ]
        .map(Close::outcome)
        .map(Outcome::as_str),
        ["done", "done_with_refusals", "incomplete", "refused"]
    );
    assert_eq!(
        Stage::ALL.map(Stage::as_str),
        ["census", "journal", "file", "calendar", "vendor"]
    );
    let counts = Counts {
        files: 2,
        seconds: 9,
        failures: 1,
        late: 3,
        late_unresolved: 0,
        max_back_s: 4,
    };
    let stats = format!(
        "definition={BAR_DEFINITION} files=2 seconds=9 failures=1 late=3 late_unresolved=0 \
         max_back_s=4"
    );
    assert_eq!(
        closing_line("k 1 *", Close::Done, &counts, ""),
        format!("done k 1 * {stats}")
    );
    assert_eq!(
        closing_line("k 1 *", Close::WithRefusals("price_refused"), &counts, ""),
        format!("done k 1 * {stats} vendor_refused=1 reason=price_refused")
    );
    assert_eq!(
        closing_line("k 1 *", Close::Incomplete(copy.cause), &counts, ""),
        format!("incomplete k 1 * {stats} stage=vendor reason=source_crc_mismatch")
    );
    assert_eq!(
        closing_line("k 1 *", Close::Refused(calendar.cause), &counts, ""),
        format!(
            "refused k 1 * definition={BAR_DEFINITION} stage=calendar reason=expiry_passed failures=1"
        )
    );
    // A RECORDED RUN'S SCOPE FOLLOWS THE COUNTS (WP-12f, D-4878), so the
    // `stage=`, `reason=` and `run=` every reader knows keep their places.
    assert_eq!(
        closing_line("k 1 *", Close::Done, &counts, " scope=ab12"),
        format!("done k 1 * {stats} scope=ab12")
    );
    assert_eq!(
        closing_line(
            "k 1 *",
            Close::WithRefusals("price_refused"),
            &counts,
            " scope=ab12"
        ),
        format!("done k 1 * {stats} scope=ab12 vendor_refused=1 reason=price_refused")
    );
    assert_eq!(
        closing_line(
            "k 1 *",
            Close::Incomplete(copy.cause),
            &counts,
            " scope=ab12"
        ),
        format!("incomplete k 1 * {stats} scope=ab12 stage=vendor reason=source_crc_mismatch")
    );
}

/// Each refusal has the cause that decides whether a day can close with it
/// (D-3317 item 2): the vendor's own bytes refused by a named rule close; the
/// copy, the calendar, the store and the census never do.
#[test]
fn every_reader_refusal_has_its_cause() {
    let crc = CmRefusal::SourceCrcMismatch { file: 1, member: 2 };
    for (got, stage, code, closes) in [
        (
            cm_cause(&CmRefusal::PriceRefused { line: 2 }),
            Stage::Vendor,
            "price_refused",
            true,
        ),
        (
            cm_cause(&CmRefusal::NoSessionRows),
            Stage::Vendor,
            "no_session_rows",
            true,
        ),
        (
            cm_cause(&CmRefusal::CalendarUnmeasured),
            Stage::Calendar,
            "calendar_unmeasured",
            false,
        ),
        (
            cm_cause(&CmRefusal::NotSwept),
            Stage::File,
            "not_swept",
            false,
        ),
        (cm_cause(&crc), Stage::Vendor, "source_crc_mismatch", false),
        (
            cm_cause(&CmRefusal::TickStoreUnavailable {
                kind: std::io::ErrorKind::NotFound,
            }),
            Stage::Vendor,
            "tick_store_unavailable",
            false,
        ),
        (
            nfo_cause(&NfoRefusal::MalformedRow { line: 451 }),
            Stage::Vendor,
            "malformed_row",
            true,
        ),
        (
            nfo_cause(&NfoRefusal::ContractNamedTwice { ticker: "T".into() }),
            Stage::Vendor,
            "contract_named_twice",
            true,
        ),
        (
            nfo_cause(&NfoRefusal::StrikeImplausible {
                ticker: "T".into(),
                strike: 10,
                median: 1,
            }),
            Stage::Vendor,
            "strike_implausible",
            true,
        ),
        (
            nfo_cause(&NfoRefusal::ExpiryPassed { ticker: "T".into() }),
            Stage::Calendar,
            "expiry_passed",
            false,
        ),
        (
            nfo_cause(&NfoRefusal::UnderlyingRefused { ticker: "T".into() }),
            Stage::File,
            "underlying_refused",
            false,
        ),
        (
            nfo_cause(&NfoRefusal::Source(CmRefusal::ArchiveMemberCorrupt {
                member: "m".into(),
            })),
            Stage::Vendor,
            "archive_member_corrupt",
            false,
        ),
        (
            nfo_cause(&NfoRefusal::Source(CmRefusal::HeaderUnknown)),
            Stage::Vendor,
            "header_unknown",
            true,
        ),
    ] {
        assert_eq!((got.stage, got.code, got.closes), (stage, code, closes));
    }
    let named = Refused::cm("SBIN.NSE.csv", &crc);
    assert_eq!(named.instrument, "SBIN.NSE.csv", "it reads as its failure");
    assert_eq!(named.cause, cm_cause(&crc));
    assert_eq!(named.why, crc.to_string());
}

/// A journal value holds no space, no newline and no byte it cannot give
/// back: everything outside `A-Z a-z 0-9 - . _ ~ /` is `%XX` (D-3317).
#[test]
fn a_journal_value_is_percent_encoded() {
    assert_eq!(encoded(b"a b%c"), "a%20b%25c");
    assert_eq!(encoded(b"tab\there\n"), "tab%09here%0A");
    assert_eq!(encoded("₹5".as_bytes()), "%E2%82%B95");
    assert_eq!(
        encoded(b"/Volumes/WD_BLACK/Options-1.~x"),
        "/Volumes/WD_BLACK/Options-1.~x"
    );
    assert_eq!(encoded(b""), "");
}

/// The central directory a zip names: an end record (with a comment), or a
/// ZIP64 end record its locator points at.
fn zip64_with(directory: &[u8]) -> Vec<u8> {
    let mut out = directory.to_vec();
    let size = u64::try_from(directory.len()).unwrap();
    let record_at = size;
    // ZIP64 end record: signature, then size of the rest (44), versions, disk
    // numbers, entry counts, and the directory's size and offset (APPNOTE
    // 4.3.14).
    out.extend_from_slice(&0x0606_4b50_u32.to_le_bytes());
    out.extend_from_slice(&44_u64.to_le_bytes());
    out.extend_from_slice(&[0; 4 + 4 + 4]);
    out.extend_from_slice(&[0; 8 + 8]);
    out.extend_from_slice(&size.to_le_bytes());
    out.extend_from_slice(&0_u64.to_le_bytes());
    // Locator: signature, disk, the record's offset, disks (4.3.15).
    out.extend_from_slice(&0x0706_4b50_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(&record_at.to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    // End record, its size and offset saturated, as a ZIP64 writer leaves them.
    out.extend_from_slice(&0x0605_4b50_u32.to_le_bytes());
    out.extend_from_slice(&[0; 2 + 2 + 2 + 2]);
    out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.extend_from_slice(&u32::MAX.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out
}

#[test]
fn the_central_directory_is_read_as_stored_and_damage_is_named() {
    let root = scratch("import-cd");
    let one = zip(&[("a.csv", b"invented", Method::Stored)]);
    let path = put(&root, "one.zip", &one);
    let directory = central_directory_bytes(&path).unwrap();
    assert!(!directory.is_empty() && one.ends_with(&[0, 0]));
    assert_eq!(
        &directory[..4],
        &0x0201_4b50_u32.to_le_bytes(),
        "a central directory header"
    );
    // A comment after the end record is skipped to find it.
    let mut commented = one.clone();
    let at = commented.len() - 2;
    commented[at..].copy_from_slice(&3_u16.to_le_bytes());
    commented.extend_from_slice(b"hi!");
    let path = put(&root, "commented.zip", &commented);
    assert_eq!(central_directory_bytes(&path).unwrap(), directory);
    // ZIP64: the directory the locator's record names.
    let path = put(&root, "z64.zip", &zip64_with(b"DIRECTORY"));
    assert_eq!(central_directory_bytes(&path).unwrap(), b"DIRECTORY");
    let path = put(&root, "z64-moved.zip", &{
        let mut bytes = zip64_with(b"DIRECTORY");
        // The locator names an offset where no ZIP64 record is.
        let locator = bytes.len() - 22 - 20;
        bytes[locator + 8..locator + 16].copy_from_slice(&1_u64.to_le_bytes());
        bytes
    });
    assert!(
        central_directory_bytes(&path).is_err_and(|why| why.contains("no ZIP64 end record")),
        "{path:?}"
    );
    let path = put(&root, "z64-past.zip", &{
        let mut bytes = zip64_with(b"DIRECTORY");
        let locator = bytes.len() - 22 - 20;
        bytes[locator + 8..locator + 16].copy_from_slice(&u64::MAX.to_le_bytes());
        bytes
    });
    assert!(central_directory_bytes(&path).is_err_and(|why| why.contains("lies past the end")));
    // A directory said to run past the end of the file.
    let mut past = one.clone();
    let end = past.len() - 22;
    past[end + 16..end + 20].copy_from_slice(&u32::MAX.to_le_bytes());
    let path = put(&root, "past.zip", &past);
    assert!(central_directory_bytes(&path).is_err_and(|why| why.contains("lies past the end")));
    let path = put(&root, "plain.zip", b"not a zip at all");
    assert!(central_directory_bytes(&path).is_err_and(|why| why.contains("no end-of-central")));
    assert!(central_directory_bytes(&root.join("absent.zip")).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

/// DB-G56: the origin a caller measures names its source, the archive's path
/// and bytes, an identity that is the same for the same archive and another
/// for another, and the commit; what it cannot measure it names.
#[test]
fn an_origin_names_the_source_archive_bytes_digest_and_commit() {
    let root = scratch("import-origin");
    let a = zip(&[("a.csv", b"one", Method::Stored)]);
    let b = zip(&[("b.csv", b"two!", Method::Deflated)]);
    put(&root, "years/2018.zip", &a);
    put(&root, "years/2019.zip", &b);
    put(&root, "years/.DS_Store", b"finder");
    put(&root, "years/notes.txt", b"not an archive");
    put(&root, "years/20190.zip", b"not a year");
    let years = root.join("years");
    let one = Origin::measure(SourceKind::Zip, &years, Some("abc123"));
    assert_eq!(one.fault, None, "{one:?}");
    assert_eq!(one.source, SourceKind::Zip);
    assert_eq!(one.path, years);
    assert_eq!(one.archive_bytes, u64::try_from(a.len() + b.len()).unwrap());
    assert_eq!(one.commit.as_deref(), Some("abc123"));
    assert_eq!(one.digest_hex().len(), 64);
    assert!(one.started > 1_700_000_000, "{}", one.started);
    let same = Origin::measure(SourceKind::Zip, &years, None);
    assert_eq!(same.digest, one.digest, "the same zips, the same identity");
    put(&root, "years/.DS_Store", b"visited again");
    assert_eq!(
        Origin::measure(SourceKind::Zip, &years, None).digest,
        one.digest,
        "a dotfile is not data"
    );
    put(&root, "years/2019.zip", &a);
    let changed = Origin::measure(SourceKind::Zip, &years, None);
    assert_ne!(
        changed.digest, one.digest,
        "another member is another archive"
    );
    // One outer zip: its length and its own directory's digest.
    let file = Origin::measure(SourceKind::Zip, &years.join("2018.zip"), None);
    assert_eq!(file.archive_bytes, u64::try_from(a.len()).unwrap());
    let directory = central_directory_bytes(&years.join("2018.zip")).unwrap();
    assert_eq!(file.digest, Some(brutex_core::blake3::hash(&directory)));
    // What cannot be measured is named, and the digest is `unmeasured`.
    put(&root, "years/2020.zip", b"not a zip");
    let damaged = Origin::measure(SourceKind::Zip, &years, None);
    assert!(
        damaged.digest.is_none()
            && damaged
                .fault
                .as_ref()
                .is_some_and(|w| w.contains("2020.zip")),
        "{damaged:?}"
    );
    assert_eq!(damaged.digest_hex(), "unmeasured");
    std::fs::create_dir_all(root.join("bare")).unwrap();
    let bare = Origin::measure(SourceKind::Zip, &root.join("bare"), None);
    assert!(
        bare.fault
            .is_some_and(|w| w.contains("holds no yearly zip"))
    );
    let absent = Origin::measure(SourceKind::Zip, &root.join("nowhere"), None);
    assert_eq!(absent.archive_bytes, 0);
    assert!(absent.fault.is_some_and(|why| why.contains("nowhere")));
    // A tick store: every `.bts` under its root, and its `FORMAT.md`.
    put(&root, "ts/FORMAT.md", b"# an invented format");
    put(&root, "ts/cm/2024/x.bts", b"12345");
    put(&root, "ts/options/2024/y.bts", b"123");
    put(&root, "ts/options/2024/notes.md", b"ignored");
    put(&root, "ts/.hidden/z.bts", b"ignored too");
    let ts = Origin::measure(SourceKind::TickStore, &root.join("ts"), Some("feed"));
    assert_eq!((ts.archive_bytes, ts.fault.clone()), (8, None), "{ts:?}");
    {
        use sha2::Digest as _;
        let want: [u8; 32] = sha2::Sha256::digest(b"# an invented format").into();
        assert_eq!(ts.digest, Some(want));
    }
    std::fs::remove_file(root.join("ts/FORMAT.md")).unwrap();
    let unformatted = Origin::measure(SourceKind::TickStore, &root.join("ts"), None);
    assert_eq!(unformatted.archive_bytes, 8);
    assert!(
        unformatted.digest.is_none()
            && unformatted
                .fault
                .is_some_and(|why| why.contains("FORMAT.md"))
    );
    let gone = Origin::measure(SourceKind::TickStore, &root.join("nowhere"), None);
    assert!(gone.fault.is_some_and(|why| why.contains("nowhere")));
    assert_eq!(
        [SourceKind::Zip, SourceKind::TickStore].map(SourceKind::digest_field),
        ["archive_cd_blake3", "format_sha256"]
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// The two events that carry the most fields keep every one of them: the
/// sink counts a field past [`telemetry::MAX_FIELDS`] instead of keeping it,
/// and the field it would lose is the room's last number or the archive's
/// fault, exactly the ones they exist to say.
#[test]
fn the_stop_and_start_events_keep_every_field() {
    let cause = Cause::open(Stage::Census, "too_many_entries");
    let measured = room(MAX_ENTRIES - 1, 463, 463);
    let full = stopped_event(
        (ImportKind::Options, "2020-09-16", cause),
        ("words", 7),
        Some(&measured),
    );
    assert_eq!(full.dropped_fields(), 0);
    assert_eq!(full.level(), telemetry::Level::Error);
    let keys: Vec<&str> = full.fields().iter().map(|(key, _)| *key).collect();
    assert_eq!(
        keys,
        [
            "kind",
            "day",
            "stage",
            "code",
            "why",
            "days_imported",
            "published",
            "n_valid",
            "limit",
            "free",
            "batch",
            "new"
        ]
    );
    let unmeasured = stopped_event((ImportKind::Options, "2020-09-16", cause), ("w", 0), None);
    assert_eq!(
        (unmeasured.fields().len(), unmeasured.dropped_fields()),
        (7, 0)
    );
    let root = scratch("import-events");
    let mut origin = Origin::measure(SourceKind::Zip, &root.join("absent"), Some("abc"));
    assert!(origin.fault.is_some());
    let faulted = started_event(
        (ImportKind::Options, "a", "b", "*"),
        (1, true),
        Some((&origin, "p", "unmeasured")),
    );
    assert_eq!(faulted.dropped_fields(), 0);
    assert_eq!(faulted.level(), telemetry::Level::Warn);
    assert_eq!(
        faulted.fields().last().map(|(key, _)| *key),
        Some("archive_fault")
    );
    origin.fault = None;
    let clean = started_event(
        (ImportKind::Options, "a", "b", "*"),
        (1, false),
        Some((&origin, "p", "d")),
    );
    assert_eq!(clean.level(), telemetry::Level::Info);
    let bare = started_event((ImportKind::Stocks, "a", "b", "*"), (1, false), None);
    assert_eq!(
        bare.fields().last().map(|(key, _)| *key),
        Some("source"),
        "unrecorded, named"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// DB-G56 and D-3317 item 4: a run's first line names where its data came
/// from and what built it; every line it closes carries its id; two runs
/// started in one second are two runs; a run that changes nothing writes
/// nothing; a run nobody measured says `unrecorded`, never nothing.
#[test]
fn every_line_a_run_closes_carries_its_run_and_the_run_line_names_the_origin() {
    let root = scratch("import-provenance");
    world(&root, &[d1()]);
    let store = root.join("S");
    let ts = root.join("ts");
    let mut origin = Origin::measure(SourceKind::TickStore, &ts, Some(COMMIT));
    origin.started = 1_750_000_000;
    let run = Run {
        kind: ImportKind::Stocks,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: &store,
    };
    let options = RunOptions {
        origin: Some(&origin),
        retry_refused: false,
        ..RunOptions::default()
    };
    run_cm_from(&TickStore::new(&ts), &run, options).unwrap();
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        format!(
            "run 1750000000 source=tickstore archive={} archive_bytes={} commit={COMMIT} \
             started=1750000000 format_sha256=unmeasured",
            encoded(ts.as_os_str().as_encoded_bytes()),
            origin.archive_bytes
        ),
        "the world's tick store has no FORMAT.md: unmeasured, named"
    );
    assert!(origin.archive_bytes > 0);
    assert!(lines[1].starts_with("begin stocks 2024-04-01 *"));
    assert!(line_of(&store, "stocks", "2024-04-01").ends_with(" run=1750000000"));
    assert!(line_of(&store, "stocks", "2024-04-02").ends_with(" run=1750000000"));
    // The same second again: the id is raised past the one the journal holds.
    let opt = Run {
        kind: ImportKind::Options,
        ..run
    };
    let mut zips = Origin::measure(SourceKind::Zip, &root.join("zips/options"), None);
    zips.started = 1_750_000_000;
    let options = RunOptions {
        origin: Some(&zips),
        retry_refused: false,
        ..RunOptions::default()
    };
    run_nfo_from(&NfoZips::new(&root.join("zips/options")), &opt, options).unwrap();
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    let second = text
        .lines()
        .find(|l| l.starts_with("run 1750000001 "))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(
        second.contains(" source=zip ")
            && second.contains(" commit=unstamped ")
            && second.contains(" started=1750000000 ")
            && second.ends_with(&format!(" archive_cd_blake3={}", zips.digest_hex())),
        "{second}"
    );
    assert_eq!(zips.fault, None, "{zips:?}");
    assert!(line_of(&store, "options", "2024-04-01").ends_with(" run=1750000001"));
    // A run that skips every day and meets the same hole writes nothing.
    let before = text.clone();
    run_cm_from(&TickStore::new(&ts), &run, RunOptions::default()).unwrap();
    assert_eq!(
        std::fs::read_to_string(journal_path(&store)).unwrap(),
        before
    );
    // Nobody measured the origin: every part of it is `unrecorded`.
    let plain = root.join("T");
    import(&root, Src::Store, ImportKind::Stocks, d1(), d1(), &plain);
    let text = std::fs::read_to_string(journal_path(&plain)).unwrap();
    let first = text.lines().next().unwrap();
    assert!(
        first.starts_with("run ")
            && first.contains(
                " source=unrecorded archive=unrecorded archive_bytes=unrecorded \
                 commit=unrecorded started="
            ),
        "{first}"
    );
    assert_eq!(run_ids(&text).len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}

/// A store refusal of a file is this import's (`stage=file`), and a source
/// that hands on a damaged block is the copy's (`stage=vendor`, a code that
/// never closes): neither closes the day.
///
/// The store refusal was a day older than what its month holds, which is
/// back-filled as a new generation since WP-12d (DB-N26); a file that would
/// change a held bar is refused by the store and by the repair alike.
#[test]
fn a_file_or_copy_refusal_journals_its_stage_and_leaves_the_day_incomplete() {
    let root = scratch("import-stages");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    let restated = root.join("restated");
    world_of(
        &restated,
        &[(
            d1(),
            Tree::Indices,
            vec![(
                entry_of(Tree::Indices, d1(), "NIFTY 50.NSE_IDX"),
                index_file("NIFTY 50.NSE_IDX", d1(), 22_001),
            )],
        )],
    );
    let late = import_only(
        &restated,
        Src::Store,
        ImportKind::Indices,
        d1(),
        d1(),
        &store,
        &["NIFTY".to_owned()],
    );
    assert_eq!(late.closed_incomplete, vec![d1()]);
    let line = line_of(&store, "indices", "2024-04-01");
    assert!(
        line.starts_with("incomplete indices 2024-04-01 ")
            && line.contains(" stage=file reason=filing run="),
        "{line}"
    );
    let path = root.join("ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[12] ^= 0xFF;
    std::fs::write(&path, bytes).unwrap();
    let damaged = root.join("D");
    let got = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &damaged);
    assert_eq!(got.files_refused, 1);
    let line = line_of(&damaged, "options", "2024-04-01");
    assert!(
        line.starts_with("incomplete options 2024-04-01 ")
            && line.contains(" stage=vendor reason="),
        "{line}"
    );
    assert!(got.closed_with_refusals.is_empty());
    assert_eq!(got.closed_incomplete, vec![d1()]);
    std::fs::remove_dir_all(root).unwrap();
}

// ── a world of invented days, in both sources ──────────────────────────────

fn ddmmyyyy(d: Day) -> String {
    format!("{:02}{:02}{:04}", d.day(), d.month(), d.year())
}

/// One invented index file: pre-open rows, three rows in one second, a late
/// row, session rows and post-close rows (so the reader's end check holds).
fn index_file(stem: &str, d: Day, base: u32) -> Vec<u8> {
    let p = |v: u32| format!("{}.{:02}", base + v / 100, v % 100);
    let mut rows = vec![row(stem, d, NINE_FIFTEEN - 300, &p(1), 0, 0)];
    rows.push(row(stem, d, NINE_FIFTEEN, &p(5), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(55), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN, &p(15), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 2, &p(20), 0, 0));
    rows.push(row(stem, d, NINE_FIFTEEN + 1, &p(30), 0, 0));
    for s in (60..22_500).step_by(997) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, &p(s % 500), 0, 0));
    }
    rows.push(row(stem, d, CLOSE + 60, &p(7), 0, 0));
    csv(&rows)
}

/// One invented stock file: traded and untraded rows.
fn stock_file(stem: &str, d: Day) -> Vec<u8> {
    let mut rows = Vec::new();
    for s in (0..22_500).step_by(1_201) {
        rows.push(row(stem, d, NINE_FIFTEEN + s, "731.40", 0, 0));
        rows.push(row(
            stem,
            d,
            NINE_FIFTEEN + s,
            &format!("73{}.05", s % 10),
            u64::from(s % 7 + 1),
            0,
        ));
    }
    csv(&rows)
}

/// One invented option file.
fn option_file(ticker: &str, d: Day, ltp: &str) -> Vec<u8> {
    let stem = format!("{ticker}.NFO");
    let mut rows = Vec::new();
    for s in (0..22_500).step_by(2_003) {
        rows.push(row(
            &stem,
            d,
            NINE_FIFTEEN + s,
            ltp,
            u64::from(s % 3) * 25,
            1_000 + u64::from(s),
        ));
    }
    csv(&rows)
}

/// The capital-market entries of one tree on one day.
fn cm_entries(kind: CmKind, d: Day) -> Vec<(String, Vec<u8>)> {
    let folder = crate::gdfl_cm::day_folder_name(kind, d);
    match kind {
        CmKind::Indices => vec![
            (format!("{folder}/"), Vec::new()),
            (
                format!("{folder}/NIFTY 50.NSE_IDX.csv"),
                index_file("NIFTY 50.NSE_IDX", d, 22_000),
            ),
            (
                format!("{folder}/NIFTY BANK.NSE_IDX.csv"),
                index_file("NIFTY BANK.NSE_IDX", d, 47_000),
            ),
            (
                format!("{folder}/INDIA VIX.NSE_IDX.csv"),
                index_file("INDIA VIX.NSE_IDX", d, 13),
            ),
        ],
        CmKind::Stocks => vec![
            (
                format!("{folder}/RELIANCE.NSE.csv"),
                stock_file("RELIANCE.NSE", d),
            ),
            (format!("{folder}/SBIN.NSE.csv"), stock_file("SBIN.NSE", d)),
            (
                format!("{folder}/RELIANCE.BE.NSE.csv"),
                stock_file("RELIANCE.BE.NSE", d),
            ),
        ],
    }
}

/// The options entries of one day.
fn nfo_entries(d: Day) -> Vec<(String, Vec<u8>)> {
    let folder = crate::gdfl_nfo::day_folder_name(d);
    [
        "NIFTY04APR2422000CE",
        "NIFTY04APR2422000PE",
        "BANKNIFTY10APR2447000CE",
    ]
    .iter()
    .map(|t| {
        (
            format!("{folder}\\Options\\{t}.NFO.csv"),
            option_file(t, d, "101.5"),
        )
    })
    .collect()
}

fn as_refs(entries: &[(String, Vec<u8>)]) -> Vec<(&str, &[u8])> {
    entries
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect()
}

/// Writes `days` of every tree into a tick store and into zips under `root`.
fn world(root: &Path, days: &[Day]) {
    let mut cm_outer: Vec<(String, Vec<u8>)> = Vec::new();
    let mut nfo_outer: BTreeMap<u16, Vec<(String, Vec<u8>)>> = BTreeMap::new();
    for &d in days {
        let month = format!("{}_{:04}", mon(d), d.year());
        for (kind, tree, prefix) in [
            (CmKind::Indices, "INDICES", "GFDLCM_INDICES_TICK_"),
            (CmKind::Stocks, "STOCKS", "GFDLCM_STOCK_TICK_"),
        ] {
            let entries = cm_entries(kind, d);
            let rel = format!("{tree}/{:04}/{month}/{prefix}{}", d.year(), ddmmyyyy(d));
            put(root, &format!("ts/cm/{rel}.bts"), &bts(&as_refs(&entries)));
            let methods: Vec<(&str, &[u8], Method)> = entries
                .iter()
                .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
                .collect();
            cm_outer.push((format!("{rel}.zip"), zip(&methods)));
        }
        let entries = nfo_entries(d);
        let rel = format!("{:04}/{month}/GFDLNFO_TICK_{}", d.year(), ddmmyyyy(d));
        put(
            root,
            &format!("ts/options/{rel}.bts"),
            &bts(&as_refs(&entries)),
        );
        let methods: Vec<(&str, &[u8], Method)> = entries
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
            .collect();
        nfo_outer.entry(d.year()).or_default().push((
            format!("{month}/GFDLNFO_TICK_{}.zip", ddmmyyyy(d)),
            zip(&methods),
        ));
    }
    let stored: Vec<(&str, &[u8], Method)> = cm_outer
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(root, "zips/cm.zip", &zip(&stored));
    for (year, inner) in nfo_outer {
        let stored: Vec<(&str, &[u8], Method)> = inner
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
            .collect();
        put(root, &format!("zips/options/{year:04}.zip"), &zip(&stored));
    }
}

/// Which source a run reads.
#[derive(Clone, Copy)]
enum Src {
    Store,
    Zips,
}

fn import(root: &Path, src: Src, kind: ImportKind, from: Day, to: Day, store: &Path) -> Report {
    import_only(root, src, kind, from, to, store, &[])
}

fn import_only(
    root: &Path,
    src: Src,
    kind: ImportKind,
    from: Day,
    to: Day,
    store: &Path,
    only: &[String],
) -> Report {
    let run = Run {
        kind,
        from,
        to,
        only,
        store_root: store,
    };
    match (src, kind) {
        (Src::Store, ImportKind::Options) => run_nfo(&NfoTickStore::new(&root.join("ts")), &run),
        (Src::Zips, ImportKind::Options) => {
            run_nfo(&NfoZips::new(&root.join("zips/options")), &run)
        }
        (Src::Store, _) => run_cm(&TickStore::new(&root.join("ts")), &run),
        (Src::Zips, _) => run_cm(&Archive::open(&root.join("zips/cm.zip")).unwrap(), &run),
    }
    .unwrap()
}

/// Every file under `dir`, relative path to bytes.
fn tree(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.insert(
                    path.strip_prefix(dir).unwrap().to_path_buf(),
                    std::fs::read(&path).unwrap(),
                );
            }
        }
    }
    out
}

/// Every one-second bar of one instrument-month.
fn bars(store: &Path, segment: &str, symbol: &str, contract: Option<Contract>) -> Vec<Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract,
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let file = BarFile::open_existing(store, path, id).unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

fn nifty_ce() -> Contract {
    Contract::parse("2024-04-04-2200000-CE").unwrap()
}

#[test]
fn every_kind_lands_at_one_second_and_both_sources_leave_the_same_store() {
    let root = scratch("import-both");
    world(&root, &[d1()]);
    let (a, b) = (root.join("A"), root.join("B"));
    for kind in ImportKind::ALL {
        let ra = import(&root, Src::Store, kind, d1(), d1(), &a);
        let rb = import(&root, Src::Zips, kind, d1(), d1(), &b);
        assert!(ra.failures.is_empty(), "{kind:?}: {:?}", ra.failures);
        assert_eq!(ra, rb, "{kind:?}: the two sources report the same run");
        assert_eq!(ra.days_imported, 1);
        assert!(
            ra.seconds > 0 && ra.seconds_committed == ra.seconds,
            "{kind:?}: {ra:?}"
        );
    }
    // ADAPTED (D-3317 item 4): byte for byte, census included; the journal
    // too but for its `run` lines and `run=` fields, which name the second
    // each run started.
    let journal = |root: &Path| std::fs::read_to_string(journal_path(root)).unwrap();
    assert_eq!(unrun(&journal(&a)), unrun(&journal(&b)));
    let mut ta = tree(&a);
    let mut tb = tree(&b);
    let at = PathBuf::from("imports/gdfl.journal");
    assert!(ta.remove(&at).is_some() && tb.remove(&at).is_some());
    // ADAPTED (D-3335): the day ledger's `member_bytes` is the day zip's
    // size, which the tick store cannot measure; every other field agrees.
    let flags = PathBuf::from("imports/gdfl.dayflags");
    let (fa, fb) = (ta.remove(&flags).unwrap(), tb.remove(&flags).unwrap());
    assert_eq!(
        crate::gdfl_fixtures::as_compared(&flags, fa.clone()),
        crate::gdfl_fixtures::as_compared(&flags, fb.clone())
    );
    let (fa, fb) = (
        String::from_utf8(fa).unwrap(),
        String::from_utf8(fb).unwrap(),
    );
    assert!(
        fa.lines()
            .all(|l| l.contains(" member_bytes=0 ") && l.ends_with(" member_src=none")),
        "{fa}"
    );
    assert!(
        fb.lines()
            .all(|l| !l.contains(" member_bytes=0 ") && l.ends_with(" member_src=zip")),
        "{fb}"
    );
    assert_eq!(ta, tb, "byte for byte, census included");
    // Only the one-second rung exists, for every kind (D-2802).
    let files = tree(&a);
    for path in files.keys() {
        let text = path.to_string_lossy();
        if text.starts_with("bars/") {
            assert!(text.contains("/1s/"), "{text}");
        }
    }
    // The import files through the store's one writer, so every one-second
    // month it lands carries the `.tix` time index that writer keeps on each
    // append (D-2329): no side door leaves a month the index does not cover.
    let months: Vec<_> = files
        .keys()
        .filter(|p| p.starts_with("bars") && p.to_string_lossy().ends_with(".bin"))
        .collect();
    assert!(!months.is_empty(), "the import landed no bar month");
    for bin in months {
        let tix = bin.with_extension("tix");
        assert!(
            files.get(&tix).is_some_and(|bytes| !bytes.is_empty()),
            "{} has no time index beside it",
            bin.display()
        );
    }
    let nifty = bars(&a, "INDEX", "NIFTY", None);
    // Pre-open and post-close rows are not session bars.
    assert_eq!(nifty[0].ts_micros, micros_at(d1(), NINE_FIFTEEN));
    assert!(
        nifty
            .iter()
            .all(|b| b.volume == 0 && b.open_interest == OI_NULL)
    );
    // The late 09:15:01 row joined the 09:15:02 second; no 09:15:01 bar.
    assert_eq!(nifty[1].ts_micros, micros_at(d1(), NINE_FIFTEEN + 2));
    assert_eq!(
        (nifty[0].open, nifty[0].high, nifty[0].low),
        (2_200_005, 2_200_055, 2_200_005)
    );
    let ce = bars(&a, "FNO", "NIFTY", Some(nifty_ce()));
    assert!(ce.iter().all(|b| b.volume > 0 && b.open_interest >= 1_000));
    assert!(
        bars(&a, "CASH", "RELIANCE", None)
            .iter()
            .all(|b| b.volume > 0)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_rerun_writes_nothing_and_a_new_day_appends() {
    let root = scratch("import-rerun");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    let first = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    let before = tree(&store);
    let again = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!(again.days_skipped, 1);
    assert_eq!(again.seconds, 0);
    assert_eq!(tree(&store), before, "byte for byte");
    let held = bars(&store, "FNO", "NIFTY", Some(nifty_ce())).len();
    let next = import(&root, Src::Store, ImportKind::Options, d1(), d2(), &store);
    assert_eq!((next.days_skipped, next.days_imported), (1, 1));
    let now = bars(&store, "FNO", "NIFTY", Some(nifty_ce()));
    assert!(now.len() > held, "the second day appended");
    assert!(
        now.windows(2).all(|w| w[0].ts_micros < w[1].ts_micros),
        "strictly increasing"
    );
    assert_eq!(first.seconds + next.seconds, first.seconds * 2);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_crashed_run_is_resumed_without_a_duplicate() {
    let root = scratch("import-crash");
    world(&root, &[d1()]);
    let clean = root.join("clean");
    import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &clean);
    // The same run, crashed after its bars landed and before its census and
    // its `done` line: the journal holds only `begin`, the census is gone,
    // and one bar file carries a torn tail past its committed records.
    let crashed = root.join("crashed");
    import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &crashed);
    let journal = journal_path(&crashed);
    let text = std::fs::read_to_string(&journal).unwrap();
    // ADAPTED (D-3317 item 4): the run's `run` line comes first, then the
    // day's `begin`; the crash kept both.
    let mut lines = text.lines();
    let run = lines.next().unwrap().to_owned();
    assert!(run.starts_with("run "), "{run}");
    let begin = lines.next().unwrap().to_owned();
    assert!(begin.starts_with("begin indices 2024-04-01 *"), "{begin}");
    std::fs::write(&journal, format!("{run}\n{begin}\n")).unwrap();
    std::fs::remove_dir_all(crashed.join("manifest")).unwrap();
    let bin = tree(&crashed)
        .into_keys()
        .find(|p| p.to_string_lossy().ends_with(".bin"))
        .unwrap();
    let mut torn = std::fs::read(crashed.join(&bin)).unwrap();
    torn.extend_from_slice(&[0xAB; 37]);
    std::fs::write(crashed.join(&bin), torn).unwrap();
    let resumed = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &crashed);
    assert_eq!(resumed.resumed, vec![d1()]);
    assert_eq!(resumed.seconds_committed, 0, "nothing written twice");
    assert!(resumed.failures.is_empty(), "{:?}", resumed.failures);
    assert_eq!(
        bars(&crashed, "INDEX", "NIFTY", None),
        bars(&clean, "INDEX", "NIFTY", None)
    );
    let census = |s: &Path| std::fs::read(s.join("manifest/gdfl.man")).unwrap();
    assert_eq!(
        census(&crashed),
        census(&clean),
        "the census is re-counted whole"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_torn_journal_line_is_ignored_and_a_foreign_one_refuses_the_run() {
    let root = scratch("import-journal");
    world(&root, &[d1()]);
    let store = root.join("S");
    put(
        &store,
        "imports/gdfl.journal",
        // `definition=` names the bar definition that built the day (D-2822).
        b"done indices 2024-04-01 * definition=2 files=2\ndone ind",
    );
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert_eq!(got.days_skipped, 1);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        text.ends_with("done ind (torn)\n"),
        "the torn line is closed with its mark, not rewritten (D-2814)"
    );
    put(&store, "imports/gdfl.journal", b"something else\n");
    let run = Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    assert!(matches!(
        run_cm(&TickStore::new(&root.join("ts")), &run),
        Err(ImportRefusal::Journal { .. })
    ));
    // A store root that is a file: the journal cannot be read.
    let file_root = put(&root, "plain", b"x");
    let run = Run {
        store_root: &file_root,
        ..run
    };
    assert!(matches!(
        run_cm(&TickStore::new(&root.join("ts")), &run),
        Err(ImportRefusal::Journal { .. })
    ));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_journal_that_cannot_be_written_fails_the_day_by_name() {
    let root = scratch("import-journal-write");
    world(&root, &[d1()]);
    let store = root.join("S");
    // A journal that reads and will not take a line.
    let journal = put(&store, "imports/gdfl.journal", b"");
    let mut perms = std::fs::metadata(&journal).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o444);
    std::fs::set_permissions(&journal, perms).unwrap();
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert_eq!(got.seconds, 0, "nothing is written before its begin line");
    assert!(
        got.failures
            .iter()
            .any(|f| f.instrument.ends_with("journal")),
        "{:?}",
        got.failures
    );
    // A clean day that wrote nothing still records `done`, and that line
    // failing is named too.
    let folder = crate::gdfl_nfo::day_folder_name(d1());
    let untraded = csv(&[row(
        "NIFTY04APR2422000CE.NFO",
        d1(),
        NINE_FIFTEEN,
        "0",
        0,
        5,
    )]);
    let entries = vec![(
        format!("{folder}\\Options\\NIFTY04APR2422000CE.NFO.csv"),
        untraded,
    )];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let quiet = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!(quiet.files, 1);
    assert!(
        quiet
            .failures
            .iter()
            .any(|f| f.instrument.ends_with("journal")),
        "{:?}",
        quiet.failures
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn closed_unmeasured_special_and_missing_days_are_each_named() {
    let root = scratch("import-days");
    world(&root, &[d1()]);
    let store = root.join("S");
    // 2024-04-06/07 are a weekend; 2024-04-02 is a trading day nobody wrote.
    let got = import(
        &root,
        Src::Store,
        ImportKind::Stocks,
        d1(),
        day(2024, 4, 7),
        &store,
    );
    assert_eq!(got.days_imported, 1);
    assert_eq!(got.days_closed, 2);
    assert_eq!(
        got.days_missing,
        vec![d2(), d3(), day(2024, 4, 4), day(2024, 4, 5)]
    );
    // The disaster-recovery Saturday is a special session read inside its
    // windows (D-3319, WP-12a): admitted, and this world does not hold it.
    let special = import(
        &root,
        Src::Zips,
        ImportKind::Indices,
        day(2024, 3, 2),
        day(2024, 3, 2),
        &store,
    );
    assert_eq!(special.days_refused, 0);
    assert_eq!(special.days_missing, vec![day(2024, 3, 2)]);
    let unmeasured = import(
        &root,
        Src::Zips,
        ImportKind::Options,
        day(2011, 1, 3),
        day(2011, 1, 3),
        &store,
    );
    assert_eq!(unmeasured.days_refused, 1);
    let run = Run {
        kind: ImportKind::Indices,
        from: d2(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    assert_eq!(
        run_cm(&TickStore::new(&root.join("ts")), &run).unwrap_err(),
        ImportRefusal::RangeBackwards {
            from: d2(),
            to: d1()
        }
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A day older than what its month holds is back-filled as a NEW GENERATION
/// of the month (D-3350; DB-N26, WP-12d), never inserted in place. It was
/// refused by the store and stayed `incomplete` for ever: the append-only
/// month cannot take a day before its first. `gdfl_backfill_tests` proves
/// the shape the real store has, a day between held ones.
#[test]
fn a_day_older_than_what_the_month_holds_lands_as_a_new_generation() {
    let root = scratch("import-order");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &store);
    let held = bars(&store, "INDEX", "NIFTY", None);
    let originals = tree(&store.join("bars"));
    let late = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert!(late.failures.is_empty(), "{:?}", late.failures);
    assert_eq!(late.months_repaired, late.files);
    assert!(late.seconds_committed > 0);
    let now = bars(&store, "INDEX", "NIFTY", None);
    assert_eq!(&now[now.len() - held.len()..], held.as_slice(), "held kept");
    assert!(
        now[0].ts_micros < held[0].ts_micros,
        "the earlier day first"
    );
    assert_eq!(tree(&store.join("bars")), originals, "never in place");
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(text.contains("\ndone indices 2024-04-01 *"), "{text}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_corrupt_file_is_refused_and_its_neighbours_still_land() {
    let root = scratch("import-corrupt");
    world(&root, &[d1()]);
    // Flip a byte inside the tick store's first options block.
    let path = root.join("ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts");
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[12] ^= 0xFF;
    std::fs::write(&path, bytes).unwrap();
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Options, d1(), d1(), &store);
    assert_eq!(got.files_refused, 1);
    assert_eq!(got.files, 2, "the other two contracts landed");
    assert!(got.failures[0].instrument.contains("NIFTY04APR2422000CE"));
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        text.contains("incomplete options"),
        "a day with a refusal is retried"
    );
    // An unreadable listing is one named failure for the day.
    std::fs::write(&path, b"not a day file").unwrap();
    let again = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("T"),
    );
    assert!(again.failures[0].instrument.ends_with("listing"));
    std::fs::write(
        root.join("ts/cm/INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024.bts"),
        b"no",
    )
    .unwrap();
    let cm = import(
        &root,
        Src::Store,
        ImportKind::Indices,
        d1(),
        d1(),
        &root.join("U"),
    );
    assert!(cm.failures[0].instrument.ends_with("listing"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_reader_refusal_in_a_capital_market_file_is_named() {
    let root = scratch("import-cm-refusal");
    world(&root, &[d1()]);
    // A stock file with no traded session row: the reader refuses it.
    let folder = crate::gdfl_cm::day_folder_name(CmKind::Stocks, d1());
    let dead = csv(&[row("SBIN.NSE", d1(), NINE_FIFTEEN, "1.00", 0, 0)]);
    let entries = vec![
        (
            format!("{folder}/RELIANCE.NSE.csv"),
            stock_file("RELIANCE.NSE", d1()),
        ),
        (format!("{folder}/SBIN.NSE.csv"), dead),
    ];
    put(
        &root,
        "ts/cm/STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let got = import(
        &root,
        Src::Store,
        ImportKind::Stocks,
        d1(),
        d1(),
        &root.join("S"),
    );
    assert_eq!((got.files, got.files_refused), (1, 1));
    assert!(got.failures[0].instrument.contains("SBIN.NSE.csv"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn two_tickers_naming_one_contract_are_both_refused() {
    let root = scratch("import-ambiguous");
    // Before the switch an index monthly can be named in both forms: the
    // day form `NIFTY27DEC1810500CE` and the monthly `NIFTY18DEC10500CE`
    // (expiry Thu 27 Dec 2018) name one contract.
    let d = day(2018, 12, 3);
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let entries: Vec<(String, Vec<u8>)> = [
        "NIFTY27DEC1810500CE",
        "NIFTY18DEC10500CE",
        "NIFTY18DEC10600CE",
    ]
    .iter()
    .map(|t| {
        (
            format!("{folder}\\Options\\{t}.NFO.csv"),
            option_file(t, d, "5"),
        )
    })
    .collect();
    put(
        &root,
        "ts/options/2018/DEC_2018/GFDLNFO_TICK_03122018.bts",
        &bts(&as_refs(&entries)),
    );
    let got = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d,
        d,
        &root.join("S"),
    );
    assert_eq!((got.files, got.files_refused), (1, 2), "{:?}", got.failures);
    assert!(got.failures.iter().all(|f| f.why.contains("same contract")));
    // A filter that excludes them refuses nothing.
    let none = import_only(
        &root,
        Src::Store,
        ImportKind::Options,
        d,
        d,
        &root.join("T"),
        &["BANKNIFTY".to_owned()],
    );
    assert!(none.failures.is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_filter_takes_only_the_named_instruments_and_keys_the_journal() {
    let root = scratch("import-only");
    world(&root, &[d1()]);
    let store = root.join("S");
    let only = ["BANKNIFTY".to_owned(), "BANKNIFTY".to_owned()];
    let got = import_only(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &store,
        &only,
    );
    assert_eq!((got.files, got.files_skipped), (1, 2));
    let spot = import_only(
        &root,
        Src::Zips,
        ImportKind::Indices,
        d1(),
        d1(),
        &store,
        &["NIFTY".to_owned()],
    );
    assert_eq!(spot.files, 1);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(
        text.contains("done options 2024-04-01 BANKNIFTY "),
        "{text}"
    );
    assert!(text.contains("done indices 2024-04-01 NIFTY "), "{text}");
    // An undecodable ticker is refused when it could be one of the filter's.
    let folder = crate::gdfl_nfo::day_folder_name(d1());
    let entries = vec![
        (
            format!("{folder}\\Options\\NIFTYJUNK.NFO.csv"),
            option_file("NIFTYJUNK", d1(), "5"),
        ),
        (
            format!("{folder}\\Options\\ACCJUNK.NFO.csv"),
            option_file("ACCJUNK", d1(), "5"),
        ),
        (format!("{folder}\\Futures\\NIFTY-I.NFO.csv"), Vec::new()),
    ];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let junk = import_only(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("T"),
        &["NIFTY".to_owned()],
    );
    assert_eq!((junk.files_refused, junk.files_skipped), (1, 2));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_source_of_the_wrong_kind_is_refused_by_name() {
    let store = scratch("import-kind");
    let run = Run {
        kind: ImportKind::Options,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    assert!(matches!(
        run_cm(&TickStore::new(&store), &run),
        Err(ImportRefusal::KindUnknown { .. })
    ));
    let spot = Run {
        kind: ImportKind::Stocks,
        ..run
    };
    assert!(matches!(
        run_nfo(&NfoTickStore::new(&store), &spot),
        Err(ImportRefusal::KindUnknown { .. })
    ));
    std::fs::remove_dir_all(store).unwrap();
}

/// ADAPTED (D-3317; DB-G01). A census that will not publish STOPS the run at
/// that day, by name: it was one more failure of the day and the run went on,
/// filing every later day's bars where no census row counts them. The census
/// file here cannot even be read, so its room is the refusal's own words.
#[test]
fn a_census_that_will_not_publish_stops_the_run_at_that_day_by_name() {
    let root = scratch("import-census");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    // A directory where the census file should be.
    std::fs::create_dir_all(store.join("manifest/gdfl.man")).unwrap();
    let run = Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d2(),
        only: &[],
        store_root: &store,
    };
    let got = run_cm(&TickStore::new(&root.join("ts")), &run).unwrap_err();
    let ImportRefusal::CensusRefused {
        kind,
        day,
        why,
        room,
        done,
    } = &got
    else {
        panic!("{got:?}");
    };
    assert_eq!((*kind, *day), (ImportKind::Indices, d1()));
    assert!(!why.is_empty());
    assert!(
        room.as_ref()
            .is_err_and(|unread| unread.contains("gdfl.man") && unread.contains("not a regular")),
        "{room:?}"
    );
    assert_eq!(done.days_imported, 0, "the refused day is not imported");
    assert!(done.seconds_committed > 0, "its bars are on disk: {done:?}");
    assert!(
        done.failures
            .iter()
            .any(|f| f.instrument == "2024-04-01 census" && f.why.contains("nothing of the batch")),
        "{:?}",
        done.failures
    );
    let text = got.to_string();
    assert!(
        text.contains("indices 2024-04-01")
            && text.contains("its room could not be measured")
            && text.contains("the run stopped there after 0 day(s) imported"),
        "{text}"
    );
    // The day's line says census; the next day was never read or journalled.
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    let last = journal.lines().last().unwrap();
    assert!(
        last.starts_with(&format!(
            "incomplete indices 2024-04-01 * definition={BAR_DEFINITION} files=2 "
        )) && last.contains(" stage=census reason=census_refused run="),
        "{journal}"
    );
    assert!(!journal.contains("2024-04-02"), "{journal}");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_gdfl_bar_is_folded_into_no_coarser_rung_on_any_ingest_door() {
    assert!(!crate::ingest::derives_for(Vendor::Gdfl));
    assert!(crate::ingest::derives_for(Vendor::Zerodha));
    assert_eq!(
        crate::ingest::derived_count_for(Vendor::Gdfl, None, Timeframe::MINUTE_1),
        0
    );
    assert_eq!(
        crate::ingest::derived_count_for(Vendor::Zerodha, None, Timeframe::MINUTE_1),
        crate::ingest::derived_count(Timeframe::MINUTE_1)
    );
    // Through `from_members` at one second: one file, no failure, no rung.
    let store = scratch("import-derive");
    let window = Window::new(d1(), d1()).unwrap();
    let request = BarRequest {
        instrument_id: String::new(),
        listing: Listing::Index,
        window,
        granularity: Granularity::Second1,
    };
    let secs =
        i64::from(d1().days_from_epoch()) * 86_400 + i64::from(NINE_FIFTEEN) - IST_OFFSET_SECS;
    let member = crate::archive::Member {
        path: PathBuf::from("invented"),
        instrument: "NIFTY".to_owned(),
        rows: (0..120)
            .map(|s| crate::fetch::RawRow {
                timestamp: secs + s,
                open: 100,
                high: 100,
                low: 100,
                close: 100,
                volume: 0,
                open_interest: None,
            })
            .collect(),
    };
    let done =
        crate::ingest::from_members(&[member], &store, plan(ImportKind::Indices, &request, None));
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(done.derived_files, 0);
    let rungs: Vec<String> = tree(&store)
        .into_keys()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| p.starts_with("bars/") && p.ends_with(".bin"))
        .collect();
    assert_eq!(rungs.len(), 1, "{rungs:?}");
    assert!(rungs[0].contains("/1s/"));
    std::fs::remove_dir_all(store).unwrap();
}

// ── properties of the one-second build ─────────────────────────────────────

/// A seeded generator, so every run tests the same sequences.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

/// The rule written the slow, obvious way: keep (every row for an index,
/// traded rows otherwise), place each late row at the next in-order stamp,
/// drop late rows with none after them, then per placed second: first,
/// max, min, last, sum, last OI.
fn reference(kind: ImportKind, day: Day, ticks: &[Tick]) -> Vec<Bar> {
    // The clock is every row; the bar is the kept rows (D-2812).
    let kept = ticks;
    let mut placed: Vec<(u32, Tick)> = Vec::new();
    // Every row at its placed second, untraded ones included (D-3342).
    let mut every: Vec<(u32, Tick)> = Vec::new();
    for (i, t) in kept.iter().enumerate() {
        let max_before = kept[..i].iter().map(|x| x.sod).max();
        let late = max_before.is_some_and(|m| t.sod < m);
        let second = if late {
            // The next in-order row after it.
            let mut running = max_before.unwrap();
            let mut found = None;
            for x in &kept[i + 1..] {
                if x.sod >= running {
                    found = Some(x.sod);
                    break;
                }
                running = running.max(x.sod);
            }
            found
        } else {
            Some(t.sod)
        };
        if let Some(s) = second {
            every.push((s, *t));
        }
        if let Some(s) = second.filter(|_| kind.every_row_counts() || t.ltq > 0) {
            placed.push((s, *t));
        }
    }
    let mut out: Vec<Bar> = Vec::new();
    for (s, t) in placed {
        let volume = if kind.every_row_counts() {
            0
        } else {
            i64::try_from(t.ltq).unwrap()
        };
        let oi = t.oi.unwrap_or(OI_NULL);
        match out.last_mut() {
            Some(bar) if bar.ts_micros == micros_at(day, s) => {
                bar.high = bar.high.max(t.ltp);
                bar.low = bar.low.min(t.ltp);
                bar.close = t.ltp;
                bar.volume += volume;
                if oi != OI_NULL {
                    bar.open_interest = oi;
                }
            }
            _ => out.push(Bar {
                ts_micros: micros_at(day, s),
                open: t.ltp,
                high: t.ltp,
                low: t.ltp,
                close: t.ltp,
                volume,
                open_interest: oi,
            }),
        }
    }
    // D-3342: an option second's open interest is its LAST placed row's,
    // traded or not.
    if kind == ImportKind::Options {
        for bar in &mut out {
            if let Some(oi) = every
                .iter()
                .filter(|(s, _)| micros_at(day, *s) == bar.ts_micros)
                .filter_map(|(_, t)| t.oi)
                .next_back()
            {
                bar.open_interest = oi;
            }
        }
    }
    out
}

#[test]
fn the_one_second_build_equals_the_rule_written_the_slow_way() {
    let mut rng = Rng(0xD1B5_4A32_D192_ED03);
    for case in 0..2_000 {
        let kind = ImportKind::ALL[usize::try_from(rng.below(3)).unwrap()];
        let n = rng.below(120);
        let mut sod = NINE_FIFTEEN - 5 + u32::try_from(rng.below(10)).unwrap();
        let mut ticks = Vec::new();
        for _ in 0..n {
            match rng.below(10) {
                // Back-steps of every size, duplicates and forward jumps.
                0 => sod = sod.saturating_sub(u32::try_from(rng.below(40)).unwrap()),
                1 | 2 => {}
                3 => sod += u32::try_from(rng.below(600)).unwrap(),
                _ => sod += 1,
            }
            let ltq = if rng.below(3) == 0 {
                0
            } else {
                rng.below(1_000)
            };
            let oi =
                (kind == ImportKind::Options).then(|| i64::try_from(rng.below(1_000_000)).unwrap());
            ticks.push(Tick {
                sod,
                ltp: 5 + i64::try_from(rng.below(100_000)).unwrap(),
                ltq,
                oi,
            });
        }
        let got = convert(kind, d1(), &ticks).unwrap();
        assert_eq!(
            got.seconds,
            reference(kind, d1(), &ticks),
            "case {case} {kind:?}"
        );
        assert!(
            got.seconds
                .windows(2)
                .all(|w| w[0].ts_micros < w[1].ts_micros),
            "strictly increasing"
        );
        let p = got.placement;
        assert_eq!(p.rows, ticks.len());
        assert!(p.ltq_zero_dropped + p.late_unresolved <= p.rows);
    }
}

#[test]
fn the_session_edges_keep_0915_00_and_1529_59_and_count_what_is_outside() {
    let root = scratch("import-edges");
    let d = d1();
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let stem = "NIFTY04APR2422000CE.NFO";
    let rows = csv(&[
        row(stem, d, NINE_FIFTEEN - 1, "1", 5, 1),
        row(stem, d, NINE_FIFTEEN, "2", 5, 2),
        row(stem, d, CLOSE - 1, "3", 5, 3),
        row(stem, d, CLOSE, "4", 5, 4),
    ]);
    let entries = vec![(
        format!("{folder}\\Options\\NIFTY04APR2422000CE.NFO.csv"),
        rows,
    )];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Options, d, d, &store);
    assert_eq!(
        (got.seconds_committed, got.outside_session),
        (2, 2),
        "{got:?}"
    );
    let held = bars(&store, "FNO", "NIFTY", Some(nifty_ce()));
    assert_eq!(held[0].ts_micros, micros_at(d, NINE_FIFTEEN));
    assert_eq!(held[1].ts_micros, micros_at(d, CLOSE - 1));
    assert_eq!(
        (held[0].close, held[1].close, held[1].open_interest),
        (200, 300, 3)
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_fold_refusal_other_than_volume_is_named_in_the_folds_words() {
    let why = crate::fold::FoldError::OutOfOrder {
        at: 1,
        previous: 2,
        found: 1,
    };
    assert!(
        matches!(fold_refusal(why, d1()), ImportRefusal::Fold { why } if why.contains("OutOfOrder"))
    );
}

#[test]
fn an_incomplete_day_is_closed_in_the_journal_and_retried() {
    let root = scratch("import-incomplete");
    world(&root, &[d1()]);
    let store = root.join("S");
    put(
        &store,
        "imports/gdfl.journal",
        b"begin indices 2024-04-01 *\nincomplete indices 2024-04-01 * files=1\n",
    );
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    // ADAPTED (D-2821): an `incomplete` day taken up again is named in
    // `resumed`, as a crashed one is; it was in no list before.
    assert_eq!(got.resumed, vec![d1()], "an incomplete day is named");
    assert!(got.restated.is_empty(), "closed incomplete, not restated");
    assert_eq!(got.days_imported, 1, "and imported again");
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_quantity_past_i64_in_a_real_file_refuses_that_file_by_name() {
    let root = scratch("import-huge");
    let folder = crate::gdfl_cm::day_folder_name(CmKind::Stocks, d1());
    let huge = csv(&[row(
        "SBIN.NSE",
        d1(),
        NINE_FIFTEEN,
        "1.00",
        9_223_372_036_854_775_808,
        0,
    )]);
    let entries = vec![
        (
            format!("{folder}/RELIANCE.NSE.csv"),
            stock_file("RELIANCE.NSE", d1()),
        ),
        (format!("{folder}/SBIN.NSE.csv"), huge),
        // Two names for one ticker: refused once, the twin skipped.
        (
            format!("{folder}/SBIN.NSE.CSV"),
            stock_file("SBIN.NSE", d1()),
        ),
    ];
    put(
        &root,
        "ts/cm/STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let got = import(
        &root,
        Src::Store,
        ImportKind::Stocks,
        d1(),
        d1(),
        &root.join("S"),
    );
    assert_eq!(got.files, 1, "{:?}", got.failures);
    assert_eq!(got.files_refused, 1, "{:?}", got.failures);
    // Options: a trading day the source does not hold.
    let none = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("T"),
    );
    assert_eq!(none.days_missing, vec![d1()]);
    // The huge quantity, through the options reader into the converter.
    let ofolder = crate::gdfl_nfo::day_folder_name(d1());
    let t = "NIFTY04APR2422000CE";
    let entries = vec![(
        format!("{ofolder}\\Options\\{t}.NFO.csv"),
        csv(&[row(
            &format!("{t}.NFO"),
            d1(),
            NINE_FIFTEEN,
            "5",
            9_223_372_036_854_775_808,
            1,
        )]),
    )];
    put(
        &root,
        "ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts",
        &bts(&as_refs(&entries)),
    );
    let big = import(
        &root,
        Src::Store,
        ImportKind::Options,
        d1(),
        d1(),
        &root.join("U"),
    );
    assert_eq!((big.files, big.files_refused), (0, 1), "{:?}", big.failures);
    assert!(big.failures[0].why.contains("does not fit an i64"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn the_done_line_states_the_days_exact_counts() {
    let root = scratch("import-done-line");
    world(&root, &[d1()]);
    let store = root.join("S");
    let got = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    let want = format!(
        "done indices 2024-04-01 * definition={BAR_DEFINITION} files={} seconds={} failures=0 late={} late_unresolved={} max_back_s={}",
        got.files, got.seconds, got.late_rows, got.late_unresolved, got.max_back_s
    );
    assert!(text.contains(&want), "{text} lacks {want}");
    assert_eq!(
        (got.files, got.late_rows),
        (2, 2),
        "two files, one late row each"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ══ WP-11b: the day ledger (D-3335), the lineage (D-3331, D-3343) and the
// provenance (D-3334). Every row below is invented at run time; the shapes
// follow the audit's measured days (DB-026, DB-029, DB-030, DB-056, DB-G56,
// DB-G71, DB-G72, DB-G77, DB-G96) without copying a vendor row, and the
// prices are ones no instrument of those days traded at.

/// Which tree a fixture file belongs to.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum Tree {
    Indices,
    Stocks,
    Options,
}

/// One day of one tree and its files, as [`world_of`] writes it.
type DayPlan = (Day, Tree, Vec<(String, Vec<u8>)>);

/// The second of the day `h:m:s`.
const fn hms_at(h: u32, m: u32, s: u32) -> u32 {
    h * 3_600 + m * 60 + s
}

/// One invented capital-market file of `(second, LTP text, LTQ)` rows, its
/// quotes zero.
fn cm_csv(stem: &str, d: Day, rows: &[(u32, String, u64)]) -> Vec<u8> {
    let lines: Vec<String> = rows
        .iter()
        .map(|(sod, ltp, ltq)| row(stem, d, *sod, ltp, *ltq, 0))
        .collect();
    csv(&lines)
}

/// One invented row of `stem` with its quotes.
fn quoted_row(
    stem: &str,
    d: Day,
    sod: u32,
    ltp: &str,
    (bid, ask): (&str, &str),
    (ltq, oi): (u64, u64),
) -> String {
    format!(
        "{stem},{:02}/{:02}/{:04},{},{ltp},{bid},10,{ask},10,{ltq},{oi}",
        d.day(),
        d.month(),
        d.year(),
        crate::gdfl_fixtures::hms(sod)
    )
}

/// One invented option file of `(second, LTP text, LTQ, OI)` rows, every
/// row quoted `quotes`.
fn nfo_csv(ticker: &str, d: Day, quotes: (&str, &str), rows: &[(u32, &str, u64, u64)]) -> Vec<u8> {
    let stem = format!("{ticker}.NFO");
    let lines: Vec<String> = rows
        .iter()
        .map(|&(sod, ltp, ltq, oi)| quoted_row(&stem, d, sod, ltp, quotes, (ltq, oi)))
        .collect();
    csv(&lines)
}

/// The entry name of `stem`'s file of `tree` on `d`.
fn entry_of(tree: Tree, d: Day, stem: &str) -> String {
    match tree {
        Tree::Indices => format!(
            "{}/{stem}.csv",
            crate::gdfl_cm::day_folder_name(CmKind::Indices, d)
        ),
        Tree::Stocks => format!(
            "{}/{stem}.csv",
            crate::gdfl_cm::day_folder_name(CmKind::Stocks, d)
        ),
        Tree::Options => format!(
            "{}\\Options\\{stem}.NFO.csv",
            crate::gdfl_nfo::day_folder_name(d)
        ),
    }
}

/// Writes each `(day, tree, files)` into a tick store (`ts/`) and into the
/// vendor's zips (`zips/cm.zip`, `zips/options/<yyyy>.zip`) under `root`,
/// as `world` does for its fixed files, and answers each day zip's size.
fn world_of(root: &Path, plan: &[DayPlan]) -> BTreeMap<(Day, Tree), u64> {
    let mut sizes = BTreeMap::new();
    let mut cm_outer: Vec<(String, Vec<u8>)> = Vec::new();
    let mut nfo_outer: BTreeMap<u16, Vec<(String, Vec<u8>)>> = BTreeMap::new();
    for (d, tree, entries) in plan {
        let d = *d;
        let month = format!("{}_{:04}", mon(d), d.year());
        let methods: Vec<(&str, &[u8], Method)> = entries
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Deflated))
            .collect();
        let day_zip = zip(&methods);
        sizes.insert((d, *tree), u64::try_from(day_zip.len()).unwrap());
        let rel = match tree {
            Tree::Indices => format!(
                "INDICES/{:04}/{month}/GFDLCM_INDICES_TICK_{}",
                d.year(),
                ddmmyyyy(d)
            ),
            Tree::Stocks => format!(
                "STOCKS/{:04}/{month}/GFDLCM_STOCK_TICK_{}",
                d.year(),
                ddmmyyyy(d)
            ),
            Tree::Options => format!("{:04}/{month}/GFDLNFO_TICK_{}", d.year(), ddmmyyyy(d)),
        };
        if *tree == Tree::Options {
            put(
                root,
                &format!("ts/options/{rel}.bts"),
                &bts(&as_refs(entries)),
            );
            nfo_outer
                .entry(d.year())
                .or_default()
                .push((format!("{month}/GFDLNFO_TICK_{}.zip", ddmmyyyy(d)), day_zip));
        } else {
            put(root, &format!("ts/cm/{rel}.bts"), &bts(&as_refs(entries)));
            cm_outer.push((format!("{rel}.zip"), day_zip));
        }
    }
    let stored: Vec<(&str, &[u8], Method)> = cm_outer
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
        .collect();
    put(root, "zips/cm.zip", &zip(&stored));
    std::fs::create_dir_all(root.join("zips/options")).unwrap();
    for (year, inner) in nfo_outer {
        let stored: Vec<(&str, &[u8], Method)> = inner
            .iter()
            .map(|(n, b)| (n.as_str(), b.as_slice(), Method::Stored))
            .collect();
        put(root, &format!("zips/options/{year:04}.zip"), &zip(&stored));
    }
    sizes
}

/// Where a run of `kind` from `src` reads under `root`, and the source kind.
fn source_path(root: &Path, src: Src, kind: ImportKind) -> (SourceKind, PathBuf) {
    match (src, kind) {
        (Src::Store, _) => (SourceKind::TickStore, root.join("ts")),
        (Src::Zips, ImportKind::Options) => (SourceKind::Zip, root.join("zips/options")),
        (Src::Zips, _) => (SourceKind::Zip, root.join("zips/cm.zip")),
    }
}

/// The run of `kind` over `(from, to)` from `src` under `root` into `store`,
/// filtered to `only`, its origin measured (with the invented [`COMMIT`])
/// when `origin` is set.
fn run_from(
    root: &Path,
    (src, kind): (Src, ImportKind),
    (from, to): (Day, Day),
    (store, only): (&Path, &[String]),
    origin: bool,
) -> Report {
    let run = Run {
        kind,
        from,
        to,
        only,
        store_root: store,
    };
    let (source_kind, path) = source_path(root, src, kind);
    let measured = Origin::measure(source_kind, &path, Some(COMMIT));
    let options = RunOptions {
        origin: origin.then_some(&measured),
        retry_refused: false,
        ..RunOptions::default()
    };
    match (src, kind) {
        (Src::Store, ImportKind::Options) => run_nfo_from(&NfoTickStore::new(&path), &run, options),
        (Src::Zips, ImportKind::Options) => run_nfo_from(&NfoZips::new(&path), &run, options),
        (Src::Store, _) => run_cm_from(&TickStore::new(&path), &run, options),
        (Src::Zips, _) => run_cm_from(&Archive::open(&path).unwrap(), &run, options),
    }
    .unwrap()
}

/// The day ledger's lines.
fn flags_lines(store: &Path) -> Vec<String> {
    std::fs::read_to_string(quality::dayflags_path(store))
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A ledger's lines.
fn ledger_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// `line` with its `member_bytes` and `member_src` for a zip of `bytes`.
fn from_zip(line: &str, bytes: u64) -> String {
    line.replace(" member_bytes=0 ", &format!(" member_bytes={bytes} "))
        .replace(" member_src=none", " member_src=zip")
}

/// Every bar of one instrument-month, the month `on`'s.
fn bars_on(
    store: &Path,
    (segment, symbol, contract): (&str, &str, Option<Contract>),
    on: Day,
) -> Vec<Bar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment,
        symbol,
        contract,
        timeframe: Timeframe::SECOND_1,
        month: on.year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    let file = BarFile::open_existing(store, path, symbol_id_of(symbol)).unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

/// The index files of 30 May 2019's shape (DB-026): NIFTY one row a minute,
/// stamped `HH:MM:59`. BANKNIFTY with 2018-09-03's 34-second hole (DB-030),
/// twelve rows of 2020-10-15's UTC burst (DB-029), one second holding two
/// values 20 bps apart (DB-G96) and one zero-LTP row the reader refuses by
/// name (D-3318).
fn index_shapes(on: Day) -> Vec<(String, Vec<u8>)> {
    let mut nifty: Vec<(u32, String, u64)> = (0..375)
        .map(|m| {
            (
                NINE_FIFTEEN + m * 60 + 59,
                format!("1234.{:02}", m % 100),
                0,
            )
        })
        .collect();
    nifty.push((CLOSE + 59, "1234.00".to_owned(), 0));
    let mut bank: Vec<(u32, String, u64)> = Vec::new();
    for s in 0..22_500 {
        let sod = NINE_FIFTEEN + s;
        if (hms_at(12, 21, 40)..hms_at(12, 22, 14)).contains(&sod) {
            continue;
        }
        let stamp = if (hms_at(11, 31, 28)..hms_at(11, 31, 40)).contains(&sod) {
            sod - 19_800
        } else {
            sod
        };
        bank.push((stamp, format!("5000.{:02}", s % 100), 0));
        if sod == hms_at(13, 30, 29) {
            bank.push((sod, "5010.30".to_owned(), 0));
        }
        if sod == hms_at(10, 0, 0) {
            bank.push((sod, "0".to_owned(), 0));
        }
    }
    bank.push((CLOSE + 30, "5000.00".to_owned(), 0));
    vec![
        (
            entry_of(Tree::Indices, on, "NIFTY 50.NSE_IDX"),
            cm_csv("NIFTY 50.NSE_IDX", on, &nifty),
        ),
        (
            entry_of(Tree::Indices, on, "NIFTY BANK.NSE_IDX"),
            cm_csv("NIFTY BANK.NSE_IDX", on, &bank),
        ),
    ]
}

/// The two index lines [`index_shapes`] makes, from the tick store.
const INDEX_SHAPE_LINES: [&str; 2] = [
    "flags indices 2019-05-30 BANKNIFTY v=1 src=import definition=4 member_bytes=0 late=12 \
     max_back_s=19799 deferred_s=1 rows_refused=1 rows=22468 seconds=22454 session_s=22500 \
     ss_distinct=60 holes_5s=2 hole_max_s=34 hole_max_at=12:21:40 dual_spread_s=1 \
     dual_spread_max_bps=20 zero_ltp_rows=1 halt_s=0 halt_held_s=0 flat_max_s=1 \
     flat_max_at=09:15:00 max_defer_s=19812 member_src=none",
    "flags indices 2019-05-30 NIFTY v=1 src=import definition=4 member_bytes=0 late=0 \
     max_back_s=0 deferred_s=0 rows_refused=0 rows=376 seconds=375 session_s=22500 \
     ss_distinct=1 holes_5s=375 hole_max_s=59 hole_max_at=09:15:00 dual_spread_s=0 \
     dual_spread_max_bps=0 zero_ltp_rows=0 halt_s=0 halt_held_s=0 flat_max_s=1 \
     flat_max_at=09:15:59 max_defer_s=0 member_src=none",
];

/// D-3335's index line, every count exact, for 30 May 2019's minute-stamped
/// NIFTY and a BANKNIFTY carrying a hole, a UTC burst, a dual value and a
/// refused row. The bars are the ones the agreed definition builds; nothing
/// the ledger counts changes them.
#[test]
fn the_day_ledger_counts_a_minute_stamped_index_holes_late_rows_and_dual_values() {
    let on = day(2019, 5, 30);
    let root = scratch("wp11b-index");
    let sizes = world_of(&root, &[(on, Tree::Indices, index_shapes(on))]);
    let store = root.join("S");
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&store, &[]),
        false,
    );
    assert_eq!(got.flags_lines, 2, "{:?}", got.failures);
    assert_eq!(flags_lines(&store), INDEX_SHAPE_LINES);
    // The bars: NIFTY's 375 minute rows at their own seconds; BANKNIFTY's
    // twelve UTC rows deferred into 11:31:40, merged with its own row.
    let nifty = bars_on(&store, ("INDEX", "NIFTY", None), on);
    assert_eq!(nifty.len(), 375);
    assert!(
        nifty
            .iter()
            .all(|bar| second_of(bar.ts_micros, on) % 60 == 59)
    );
    let bank = bars_on(&store, ("INDEX", "BANKNIFTY", None), on);
    assert_eq!(bank.len(), 22_454);
    let at = |sod: u32| {
        bank.iter()
            .find(|bar| second_of(bar.ts_micros, on) == sod)
            .copied()
            .unwrap()
    };
    let merged = at(hms_at(11, 31, 40));
    assert_eq!(
        (merged.open, merged.high, merged.low, merged.close),
        (500_088, 500_099, 500_000, 500_000)
    );
    let dual = at(hms_at(13, 30, 29));
    assert_eq!(
        (dual.open, dual.high, dual.low, dual.close),
        (500_029, 501_030, 500_029, 501_030),
        "the fold is faithful; the ledger names the spread"
    );
    assert!(bank.iter().all(|bar| {
        !(hms_at(12, 21, 40)..hms_at(12, 22, 14)).contains(&second_of(bar.ts_micros, on))
    }));
    // From the zips, the same lines with the day zip's size measured.
    let zipped = root.join("Z");
    run_from(
        &root,
        (Src::Zips, ImportKind::Indices),
        (on, on),
        (&zipped, &[]),
        false,
    );
    let bytes = sizes[&(on, Tree::Indices)];
    assert_eq!(
        flags_lines(&zipped),
        INDEX_SHAPE_LINES.map(|line| from_zip(line, bytes))
    );
    // A re-run writes nothing: the day is done.
    let before = flags_lines(&store);
    let again = run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&store, &[]),
        false,
    );
    assert_eq!((again.days_skipped, again.flags_lines), (1, 0));
    assert_eq!(flags_lines(&store), before);
    // A filtered run writes the index lines it read in whole: an index line
    // is one index's.
    let filtered = root.join("F");
    let only = ["NIFTY".to_owned()];
    run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&filtered, &only),
        false,
    );
    assert_eq!(flags_lines(&filtered), [INDEX_SHAPE_LINES[1]]);
    std::fs::remove_dir_all(root).unwrap();
}

/// One invented share file of the 30 May 2019 shape: untraded pre-open
/// rows, the first trade at 11:57:48 carrying a cumulative volume, an LTQ =
/// 0 row after it with a new price and one with the same price, then a
/// trade a minute.
fn late_share(stem: &str, d: Day) -> Vec<u8> {
    let mut rows = vec![
        (hms_at(9, 0, 0), "100.00".to_owned(), 0),
        (hms_at(11, 57, 48), "100.50".to_owned(), 3_240_155),
        (hms_at(11, 57, 49), "101.00".to_owned(), 0),
        (hms_at(11, 57, 50), "100.50".to_owned(), 0),
    ];
    let mut sod = hms_at(11, 58, 48);
    while sod < CLOSE {
        rows.push((sod, "100.50".to_owned(), 10));
        sod += 60;
    }
    cm_csv(stem, d, &rows)
}

/// One invented share file of the 2022-03-07 shape: a trade a second from
/// the open to 09:16:43, none from 09:16:44 to 09:39:22 unless `through`
/// (then one every 30 s), then one every 30 s from 09:39:23.
fn holed_share(stem: &str, d: Day, through: bool) -> Vec<u8> {
    let mut rows: Vec<(u32, String, u64)> = (NINE_FIFTEEN..=hms_at(9, 16, 43))
        .map(|sod| (sod, "200.00".to_owned(), 5))
        .collect();
    if through {
        rows.extend(
            (hms_at(9, 16, 44)..hms_at(9, 39, 23))
                .step_by(30)
                .map(|sod| (sod, "200.05".to_owned(), 5)),
        );
    }
    rows.extend(
        (hms_at(9, 39, 23)..CLOSE)
            .step_by(30)
            .map(|sod| (sod, "200.10".to_owned(), 5)),
    );
    cm_csv(stem, d, &rows)
}

/// D-3335's stocks line: 30 May 2019's late first trades (the median first
/// trade 11:57:48, the morning one hole every file shares) and its LTQ = 0
/// rows carrying a new price (DB-056); 2022-03-07's 23-minute hole, shared by
/// ten of eleven files, at least 90%, so counted (DB-030).
#[test]
fn the_day_ledger_counts_late_first_trades_shared_holes_and_new_untraded_prices() {
    let degraded = day(2019, 5, 30);
    let hole = day(2022, 3, 7);
    let root = scratch("wp11b-stocks");
    let shares = [
        "RELIANCE",
        "SBIN",
        "INFY",
        "HDFCBANK",
        "ICICIBANK",
        "TCS",
        "ITC",
        "LT",
        "AXISBANK",
        "KOTAKBANK",
    ];
    let mut late = Vec::new();
    let mut holed = Vec::new();
    for share in shares {
        let stem = format!("{share}.NSE");
        late.push((
            entry_of(Tree::Stocks, degraded, &stem),
            late_share(&stem, degraded),
        ));
        holed.push((
            entry_of(Tree::Stocks, hole, &stem),
            holed_share(&stem, hole, false),
        ));
    }
    holed.push((
        entry_of(Tree::Stocks, hole, "IDEA.NSE"),
        holed_share("IDEA.NSE", hole, true),
    ));
    world_of(
        &root,
        &[(degraded, Tree::Stocks, late), (hole, Tree::Stocks, holed)],
    );
    let store = root.join("S");
    for on in [degraded, hole] {
        let got = run_from(
            &root,
            (Src::Store, ImportKind::Stocks),
            (on, on),
            (&store, &[]),
            false,
        );
        assert!(got.failures.is_empty(), "{:?}", got.failures);
    }
    assert_eq!(
        flags_lines(&store),
        [
            "flags stocks 2019-05-30 * v=1 src=import definition=4 member_bytes=0 late=0 \
             max_back_s=0 deferred_s=0 rows_refused=0 files=10 files_traded=10 \
             first_trade_p50=11:57:48 shared_holes=1 shared_hole_max_s=9768 \
             shared_hole_max_at=09:15:00 ltq0_new_ltp_rows=10 auction_prints=0 halt_s=0 \
             halt_held_s=0 max_defer_s=0 member_src=none",
            "flags stocks 2022-03-07 * v=1 src=import definition=4 member_bytes=0 late=0 \
             max_back_s=0 deferred_s=0 rows_refused=0 files=11 files_traded=11 \
             first_trade_p50=09:15:00 shared_holes=1 shared_hole_max_s=1359 \
             shared_hole_max_at=09:16:44 ltq0_new_ltp_rows=0 auction_prints=0 halt_s=0 \
             halt_held_s=0 max_defer_s=0 member_src=none",
        ]
    );
    // The cumulative first print is filed as the bar it is: flagged, never
    // altered (D-3335, the operator's answer).
    let reliance = bars_on(&store, ("CASH", "RELIANCE", None), degraded);
    assert_eq!(reliance[0].volume, 3_240_155);
    assert_eq!(
        second_of(reliance[0].ts_micros, degraded),
        hms_at(11, 57, 48)
    );
    // A filtered share run writes no line: `*` is the whole kind-day.
    let filtered = root.join("F");
    let only = ["SBIN".to_owned()];
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Stocks),
        (degraded, degraded),
        (&filtered, &only),
        false,
    );
    assert_eq!((got.files, got.flags_lines), (1, 0));
    assert!(flags_lines(&filtered).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

/// 30 May 2019's options shape (DB-026): files whose every row carries zero
/// bid and ask and that lead with LTQ = 1 marker rows, beside a quoted file
/// and an untraded one; and an LTQ = 0 row with a new price (DB-056).
#[test]
fn the_day_ledger_counts_zero_quote_files_and_leading_marker_rows() {
    let on = day(2019, 5, 30);
    let root = scratch("wp11b-options");
    let zero = ("0", "0.00");
    let quotes = ("10.00", "10.50");
    let files = vec![
        (
            entry_of(Tree::Options, on, "ACC30MAY191700PE"),
            nfo_csv(
                "ACC30MAY191700PE",
                on,
                zero,
                &[
                    (hms_at(9, 15, 11), "8.70", 1, 24_000),
                    (hms_at(9, 15, 12), "8.70", 1, 24_000),
                    (hms_at(9, 20, 0), "9.00", 400, 24_400),
                ],
            ),
        ),
        (
            entry_of(Tree::Options, on, "NIFTY30MAY1911900CE"),
            nfo_csv(
                "NIFTY30MAY1911900CE",
                on,
                zero,
                &[
                    (hms_at(9, 15, 30), "50.00", 1, 9_000),
                    (hms_at(9, 16, 0), "51.00", 75, 9_075),
                    (hms_at(9, 16, 5), "52.00", 0, 9_075),
                ],
            ),
        ),
        (
            entry_of(Tree::Options, on, "SBIN30MAY19340CE"),
            nfo_csv(
                "SBIN30MAY19340CE",
                on,
                zero,
                &[
                    (hms_at(9, 16, 0), "3.00", 1, 3_000),
                    (hms_at(9, 16, 1), "3.00", 1, 3_000),
                    (hms_at(9, 16, 2), "3.00", 1, 3_000),
                    (hms_at(9, 30, 0), "3.10", 3_000, 6_000),
                ],
            ),
        ),
        (
            entry_of(Tree::Options, on, "RELIANCE30MAY191300CE"),
            nfo_csv(
                "RELIANCE30MAY191300CE",
                on,
                quotes,
                &[
                    (hms_at(9, 17, 0), "10.20", 505, 1_010),
                    (hms_at(9, 17, 5), "10.20", 0, 1_010),
                ],
            ),
        ),
        (
            entry_of(Tree::Options, on, "INFY30MAY19700PE"),
            nfo_csv(
                "INFY30MAY19700PE",
                on,
                quotes,
                &[(hms_at(9, 15, 0), "0", 0, 0)],
            ),
        ),
    ];
    let sizes = world_of(&root, &[(on, Tree::Options, files)]);
    let store = root.join("S");
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Options),
        (on, on),
        (&store, &[]),
        false,
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let want = "flags options 2019-05-30 * v=1 src=import definition=4 member_bytes=0 late=0 \
                max_back_s=0 deferred_s=0 rows_refused=0 files=5 files_traded=4 \
                zero_quote_files=3 ltq1_lead_files=3 ltq1_lead_rows=6 ltq0_new_ltp_rows=1 \
                first_trade_p50=09:15:30 halt_s=0 halt_held_s=0 max_defer_s=0 \
                member_src=none";
    assert_eq!(flags_lines(&store), [want]);
    let zipped = root.join("Z");
    run_from(
        &root,
        (Src::Zips, ImportKind::Options),
        (on, on),
        (&zipped, &[]),
        false,
    );
    assert_eq!(
        flags_lines(&zipped),
        [from_zip(want, sizes[&(on, Tree::Options)])]
    );
    // The marker rows stay in the bars: flagged only (the operator's
    // answer). ACC's first bar is its two LTQ = 1 rows, volume 2.
    let acc = bars_on(
        &store,
        (
            "FNO",
            "ACC",
            Some(Contract::parse("2019-05-30-170000-PE").unwrap()),
        ),
        on,
    );
    assert_eq!((acc[0].volume, acc[1].volume), (1, 1));
    std::fs::remove_dir_all(root).unwrap();
}

/// `cli gdfl-dayflags` (D-3335 §1): a rescan of days already imported writes
/// the same counts as the import, `src=rescan`, writes nothing else, reads
/// no day the journal does not record as imported, and a second rescan
/// writes nothing; a line that says something new replaces the key's last.
#[test]
fn a_rescan_writes_the_import_s_counts_and_nothing_else() {
    let on = day(2019, 5, 30);
    let root = scratch("wp11b-rescan");
    world_of(&root, &[(on, Tree::Indices, index_shapes(on))]);
    let store = root.join("S");
    run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&store, &[]),
        false,
    );
    let flags = quality::dayflags_path(&store);
    std::fs::remove_file(&flags).unwrap();
    let held = tree(&store);
    let source = TickStore::new(&root.join("ts"));
    let range = (day(2019, 5, 29), day(2019, 5, 31));
    let done = rescan_cm(&source, ImportKind::Indices, range, &store).unwrap();
    assert_eq!(
        (done.days_read, done.days_not_imported, done.flags_lines),
        (1, 2, 2),
        "{done:?}"
    );
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert_eq!(
        flags_lines(&store),
        INDEX_SHAPE_LINES.map(|line| line.replace(" src=import ", " src=rescan "))
    );
    let mut after = tree(&store);
    assert!(
        after
            .remove(&PathBuf::from("imports/gdfl.dayflags"))
            .is_some()
    );
    assert_eq!(after, held, "nothing but the day ledger was written");
    let again = rescan_cm(&source, ImportKind::Indices, range, &store).unwrap();
    assert_eq!((again.flags_lines, again.flags_unchanged), (0, 2));
    // A key whose last line says otherwise is written again.
    let text = std::fs::read_to_string(&flags).unwrap();
    std::fs::write(&flags, text.replace("holes_5s=375", "holes_5s=374")).unwrap();
    let mended = rescan_cm(&source, ImportKind::Indices, range, &store).unwrap();
    assert_eq!((mended.flags_lines, mended.flags_unchanged), (1, 1));
    assert_eq!(
        flags_lines(&store).last().map(String::as_str),
        Some(
            INDEX_SHAPE_LINES[1]
                .replace(" src=import ", " src=rescan ")
                .as_str()
        )
    );
    // An options rescan of the kind is its own: no options day was imported.
    let none = rescan_nfo(&NfoTickStore::new(&root.join("ts")), range, &store, true).unwrap();
    assert_eq!((none.days_read, none.days_not_imported), (0, 3));
    assert!(!crate::gdfl_lineage::lineage_path(&store).exists());
    std::fs::remove_dir_all(root).unwrap();
}

/// A gzip of `text`, as an NSE security master arrives.
fn gzip(text: &str) -> Vec<u8> {
    use std::io::Write as _;
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    enc.write_all(text.as_bytes()).unwrap();
    enc.finish().unwrap()
}

/// D-3320 and D-3335: a closing-auction print is counted in the day's
/// `auction_prints`; the import files it in its sidecar, and a rescan,
/// which files nothing, never writes one.
#[test]
fn a_rescan_counts_the_closing_auction_and_never_writes_its_sidecar() {
    let on = day(2026, 9, 24);
    let root = scratch("wp11b-cas");
    let stem = "RELIANCE.NSE";
    let mut rows: Vec<String> = (0..10)
        .map(|i| {
            quoted_row(
                stem,
                on,
                hms_at(15, 5, 0) + i * 50,
                &format!("4444.{i}5"),
                ("4444.00", "4444.90"),
                (3, 0),
            )
        })
        .collect();
    for _ in 0..2 {
        rows.push(quoted_row(
            stem,
            on,
            hms_at(15, 28, 7),
            "4446.65",
            ("0", "0.00"),
            (98_765, 0),
        ));
    }
    world_of(
        &root,
        &[(
            on,
            Tree::Stocks,
            vec![(entry_of(Tree::Stocks, on, stem), csv(&rows))],
        )],
    );
    let masters = root.join("masters");
    let at = brutex_core::universe::FNO_INDEX
        .position("RELIANCE")
        .unwrap();
    let isin = brutex_core::universe::FNO_UNDERLYINGS_ISIN[at];
    let master = format!(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n2885,RELIANCE,EQ,{isin},1\n"
    );
    crate::cash_session_cache::install(&masters, on, &gzip(&master)).unwrap();
    let scope = std::sync::Arc::new(
        crate::gdfl_cm::CmScope::stored(std::iter::empty::<&str>())
            .unwrap()
            .with_masters(masters.clone()),
    );
    let source = TickStore::new(&root.join("ts")).with_scope(scope);
    let store = root.join("S");
    let run = Run {
        kind: ImportKind::Stocks,
        from: on,
        to: on,
        only: &[],
        store_root: &store,
    };
    let got = run_cm(&source, &run).unwrap();
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let key = brutex_core::instrument::InstrumentKey::cash(
        brutex_core::instrument::Exchange::Nse,
        "RELIANCE",
    )
    .unwrap();
    let sidecar = crate::gdfl_cm::auction::path(&store, &key, on).unwrap();
    assert!(sidecar.exists());
    let line = flags_lines(&store).pop().unwrap();
    assert!(line.contains(" auction_prints=1 "), "{line}");
    std::fs::remove_file(&sidecar).unwrap();
    std::fs::remove_file(quality::dayflags_path(&store)).unwrap();
    let done = rescan_cm(&source, ImportKind::Stocks, (on, on), &store).unwrap();
    assert!(done.failures.is_empty(), "{:?}", done.failures);
    assert!(
        !sidecar.exists(),
        "a rescan files nothing, not even a print"
    );
    assert_eq!(
        flags_lines(&store),
        [line.replace(" src=import ", " src=rescan ")]
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A day ledger that will not take a day's lines leaves the day
/// `incomplete`, named, so the next run writes them; one that cannot be
/// read refuses the run before any day.
#[test]
fn a_day_ledger_that_will_not_take_a_line_leaves_the_day_open() {
    let on = day(2019, 5, 30);
    let root = scratch("wp11b-ledger-fault");
    world_of(&root, &[(on, Tree::Indices, index_shapes(on))]);
    let store = root.join("S");
    let flags = put(&store, "imports/gdfl.dayflags", b"");
    let mut perms = std::fs::metadata(&flags).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o444);
    std::fs::set_permissions(&flags, perms.clone()).unwrap();
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&store, &[]),
        false,
    );
    assert_eq!(got.closed_incomplete, vec![on]);
    assert!(
        got.failures
            .iter()
            .any(|f| f.instrument.ends_with("day ledger")),
        "{:?}",
        got.failures
    );
    assert!(line_of(&store, "indices", "2019-05-30").contains(" stage=journal reason=day_ledger "));
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o644);
    std::fs::set_permissions(&flags, perms).unwrap();
    let again = run_from(
        &root,
        (Src::Store, ImportKind::Indices),
        (on, on),
        (&store, &[]),
        false,
    );
    assert!(again.failures.is_empty(), "{:?}", again.failures);
    assert_eq!(flags_lines(&store), INDEX_SHAPE_LINES);
    // A ledger that is not a file refuses the run, by name, before any day.
    let broken = root.join("B");
    std::fs::create_dir_all(quality::dayflags_path(&broken)).unwrap();
    let run = Run {
        kind: ImportKind::Indices,
        from: on,
        to: on,
        only: &[],
        store_root: &broken,
    };
    let refused = run_cm(&TickStore::new(&root.join("ts")), &run).unwrap_err();
    assert_eq!(refused.code(), "day_ledger");
    assert!(!journal_path(&broken).exists(), "no day began");
    std::fs::remove_dir_all(root).unwrap();
}

// ── D-3334: a provenance record per landing ────────────────────────────────

/// The sidecar of `NIFTY04APR2422000CE`'s April month.
fn nifty_ce_sidecar(store: &Path) -> Option<store::provenance::Sidecar> {
    let path = StorePath::new(PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "FNO",
        symbol: "NIFTY",
        contract: Some(nifty_ce()),
        timeframe: Timeframe::SECOND_1,
        month: d1().year_month().unwrap(),
        file: FileKind::Bars,
    })
    .unwrap();
    match store::provenance::read(store, path).unwrap() {
        store::provenance::Found::Present(sidecar) => Some(sidecar),
        _ => None,
    }
}

/// The line number, from 1, of `line` in the journal.
fn journal_line_of(store: &Path, line: &str) -> u64 {
    let text = std::fs::read_to_string(journal_path(store)).unwrap();
    let at = text.lines().position(|l| l == line).unwrap();
    u64::try_from(at + 1).unwrap()
}

/// DB-G56, D-3334: every landing of a run that names its origin is recorded
/// beside its month: the archive member (`<year zip>!<day zip>!<entry>`, its
/// size and CRC-32) or the tick-store day file, the day, the bars appended,
/// the bar definition, the journal's `begin` line and the commit. A re-run
/// appends no record; a run with no origin writes no sidecar; the bars are
/// the same bytes either way.
#[test]
fn every_landing_of_a_run_with_an_origin_records_its_provenance() {
    let root = scratch("wp11b-provenance");
    world(&root, &[d1(), d2()]);
    let (zipped, ticked, bare) = (root.join("Z"), root.join("T"), root.join("N"));
    let z = run_from(
        &root,
        (Src::Zips, ImportKind::Options),
        (d1(), d2()),
        (&zipped, &[]),
        true,
    );
    assert!(z.failures.is_empty(), "{:?}", z.failures);
    assert_eq!(z.provenance_records, 6, "three files a day, two days");
    let sidecar = nifty_ce_sidecar(&zipped).unwrap();
    let records = sidecar.records();
    assert_eq!(records.len(), 2);
    let first = &records[0];
    let entry = format!(
        "{}\\Options\\NIFTY04APR2422000CE.NFO.csv",
        crate::gdfl_nfo::day_folder_name(d1())
    );
    let name = format!(
        "{}!APR_2024/GFDLNFO_TICK_01042024.zip!{entry}",
        root.join("zips/options/2024.zip").display()
    );
    assert_eq!(
        first.source_kind,
        store::provenance::SourceKind::ArchiveMember
    );
    assert_eq!(
        first.source_name_blake3,
        brutex_core::blake3::hash(name.as_bytes())
    );
    assert!(
        name.starts_with(&first.source_name),
        "{}",
        first.source_name
    );
    let member = option_file("NIFTY04APR2422000CE", d1(), "101.5");
    assert_eq!(first.source_bytes, u64::try_from(member.len()).unwrap());
    assert_eq!(first.source_crc32, crate::gdfl_archive::crc32(&member));
    assert_eq!(first.day.to_string(), "2024-04-01");
    assert_eq!(first.bar_definition, BAR_DEFINITION);
    assert_eq!(
        first.journal_ordinal,
        journal_line_of(&zipped, "begin options 2024-04-01 *")
    );
    assert_eq!(first.commit_hex().as_deref(), Some(COMMIT));
    let held = bars(&zipped, "FNO", "NIFTY", Some(nifty_ce()));
    assert_eq!(first.first_index, 0);
    assert_eq!(records[1].first_index, first.n_bars);
    assert_eq!(
        records[1].first_index + records[1].n_bars,
        u64::try_from(held.len()).unwrap()
    );
    assert_eq!(
        records[1].journal_ordinal,
        journal_line_of(&zipped, "begin options 2024-04-02 *")
    );
    // The tick store: its day file, below its root.
    let t = run_from(
        &root,
        (Src::Store, ImportKind::Options),
        (d1(), d2()),
        (&ticked, &[]),
        true,
    );
    assert!(t.failures.is_empty(), "{:?}", t.failures);
    let sidecar = nifty_ce_sidecar(&ticked).unwrap();
    let ts = &sidecar.records()[0];
    assert_eq!(ts.source_kind, store::provenance::SourceKind::TickstoreDay);
    assert_eq!(
        ts.source_name,
        "options/2024/APR_2024/GFDLNFO_TICK_01042024.bts"
    );
    let bts_len =
        std::fs::metadata(root.join("ts/options/2024/APR_2024/GFDLNFO_TICK_01042024.bts"))
            .unwrap()
            .len();
    assert_eq!((ts.source_bytes, ts.source_crc32), (bts_len, 0));
    // A re-run appends nothing.
    let before = tree(&zipped);
    let again = run_from(
        &root,
        (Src::Zips, ImportKind::Options),
        (d1(), d2()),
        (&zipped, &[]),
        true,
    );
    assert_eq!((again.days_skipped, again.provenance_records), (2, 0));
    assert_eq!(tree(&zipped), before);
    // No origin, no sidecar; the bars are the same bytes.
    let n = run_from(
        &root,
        (Src::Zips, ImportKind::Options),
        (d1(), d2()),
        (&bare, &[]),
        false,
    );
    assert_eq!(n.provenance_records, 0);
    assert!(nifty_ce_sidecar(&bare).is_none());
    let bars_of = |store: &Path| -> BTreeMap<PathBuf, Vec<u8>> {
        tree(store)
            .into_iter()
            .filter(|(p, _)| p.starts_with("bars") && p.extension().is_none_or(|ext| ext != "prv"))
            .collect()
    };
    assert_eq!(bars_of(&zipped), bars_of(&bare));
    assert_eq!(bars_of(&ticked), bars_of(&bare));
    // An index landing names its capital-market member.
    let cm = run_from(
        &root,
        (Src::Zips, ImportKind::Indices),
        (d1(), d1()),
        (&zipped, &[]),
        true,
    );
    assert_eq!(cm.provenance_records, 2, "{:?}", cm.failures);
    std::fs::remove_dir_all(root).unwrap();
}

/// A provenance record the sidecar refuses (here a commit that is not forty
/// hexadecimal digits) is a named failure that leaves the day open; the bars
/// are filed, and unrecorded.
#[test]
fn a_refused_provenance_record_is_named_and_leaves_the_day_open() {
    let root = scratch("wp11b-provenance-refused");
    world(&root, &[d1()]);
    let store = root.join("S");
    let run = Run {
        kind: ImportKind::Indices,
        from: d1(),
        to: d1(),
        only: &[],
        store_root: &store,
    };
    let origin = Origin::measure(SourceKind::TickStore, &root.join("ts"), Some("abc"));
    let options = RunOptions {
        origin: Some(&origin),
        retry_refused: false,
        ..RunOptions::default()
    };
    let got = run_cm_from(&TickStore::new(&root.join("ts")), &run, options).unwrap();
    assert_eq!(got.closed_incomplete, vec![d1()]);
    assert!(
        got.failures
            .iter()
            .all(|f| f.why.contains("provenance record was refused")),
        "{:?}",
        got.failures
    );
    assert!(got.seconds_committed > 0, "the bars are filed");
    assert!(line_of(&store, "indices", "2024-04-01").contains(" stage=file reason=provenance "),);
    std::fs::remove_dir_all(root).unwrap();
}

/// The journal names the line a `begin` lands on even when another process
/// appended to it since this run last wrote, and when a load closed a torn
/// tail (D-3334's `journal_ordinal`).
#[test]
fn the_journal_counts_its_lines_across_other_writers() {
    let root = scratch("wp11b-journal-lines");
    let path = journal_path(&root);
    put(
        &root,
        "imports/gdfl.journal",
        b"begin indices 2024-04-01 *\nbegin indices 2024-04-02 *\nbegin indi",
    );
    let journal = Journal::load(path.clone()).unwrap();
    assert_eq!(journal.append("begin indices 2024-04-03 *").unwrap(), 4);
    let mut other = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(
        &mut other,
        b"begin stocks 2024-04-01 *\nbegin stocks 2024-04-02 *\n",
    )
    .unwrap();
    assert_eq!(journal.append("begin indices 2024-04-04 *").unwrap(), 7);
    let text = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        text.lines().nth(6),
        Some("begin indices 2024-04-04 *"),
        "{text}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ── D-3331, D-3343: the corporate-action lineage, wired ────────────────────

/// The pre-ex day, the ex-date and the day after, of an invented x1.2
/// adjustment in DB-G71's shape (NTPC's lot 4,000 to 4,800).
fn ex_days() -> (Day, Day, Day) {
    (day(2019, 3, 18), day(2019, 3, 19), day(2019, 3, 20))
}

/// The options files of [`ex_days`]: five pre-ex contracts at a lot of
/// 4,000; on the ex-date each renamed to its strike / 1.2 at a lot of 4,800,
/// `135PE` carrying the pre-ex OI unadjusted until 11:00:00 (DB-G77) and
/// `150CE` the text of a different pre-ex contract (DB-G71); the day after,
/// the retired text `150PE` traded again by another contract.
fn ex_world(root: &Path) {
    let (d1, d2, d3) = ex_days();
    let q = ("1.00", "1.10");
    let file = |d: Day, ticker: &str, rows: &[(u32, &str, u64, u64)]| {
        (
            entry_of(Tree::Options, d, ticker),
            nfo_csv(ticker, d, q, rows),
        )
    };
    world_of(
        root,
        &[
            (
                d1,
                Tree::Options,
                vec![
                    file(
                        d1,
                        "NTPC28MAR19150PE",
                        &[
                            (hms_at(9, 20, 0), "5.00", 4_000, 100_000),
                            (hms_at(10, 0, 0), "5.10", 8_000, 104_000),
                        ],
                    ),
                    file(
                        d1,
                        "NTPC28MAR19162PE",
                        &[
                            (hms_at(9, 21, 0), "9.00", 4_000, 50_000),
                            (hms_at(10, 1, 0), "9.20", 4_000, 52_000),
                        ],
                    ),
                    file(
                        d1,
                        "NTPC28MAR19168PE",
                        &[(hms_at(9, 22, 0), "12.00", 4_000, 30_000)],
                    ),
                    file(
                        d1,
                        "NTPC28MAR19150CE",
                        &[(hms_at(9, 23, 0), "20.00", 4_000, 200_000)],
                    ),
                    file(
                        d1,
                        "NTPC28MAR19180CE",
                        &[(hms_at(9, 24, 0), "4.00", 4_000, 96_000)],
                    ),
                ],
            ),
            (
                d2,
                Tree::Options,
                vec![
                    file(
                        d2,
                        "NTPC28MAR19125PE",
                        &[(hms_at(9, 15, 25), "6.00", 4_800, 124_800)],
                    ),
                    file(
                        d2,
                        "NTPC28MAR19135PE",
                        &[
                            (hms_at(9, 15, 30), "11.00", 4_800, 52_000),
                            (hms_at(11, 0, 0), "11.10", 4_800, 62_400),
                        ],
                    ),
                    file(
                        d2,
                        "NTPC28MAR19140PE",
                        &[(hms_at(9, 16, 0), "14.00", 4_800, 36_000)],
                    ),
                    file(
                        d2,
                        "NTPC28MAR19125CE",
                        &[(hms_at(9, 17, 0), "24.00", 4_800, 240_000)],
                    ),
                    file(
                        d2,
                        "NTPC28MAR19150CE",
                        &[(hms_at(9, 18, 0), "4.80", 4_800, 115_200)],
                    ),
                ],
            ),
            (
                d3,
                Tree::Options,
                vec![
                    file(
                        d3,
                        "NTPC28MAR19150PE",
                        &[(hms_at(9, 30, 0), "1.00", 4_800, 4_800)],
                    ),
                    file(
                        d3,
                        "NTPC28MAR19125PE",
                        &[(hms_at(9, 31, 0), "6.50", 4_800, 130_000)],
                    ),
                ],
            ),
        ],
    );
}

/// The lineage ledger [`ex_world`] must leave: every alias row once, then
/// the five renamed contracts, the reused `150CE` on the ex-date and the
/// reused `150PE` the day after.
fn ex_lineage() -> Vec<String> {
    let (_, d2, _) = ex_days();
    let stale_until = i64::from(d2.days_from_epoch()) * 86_400 + i64::from(hms_at(11, 0, 0))
        - crate::session::IST_OFFSET_SECS;
    let mut want: Vec<String> = crate::expiry_alias::alias_records()
        .unwrap()
        .iter()
        .map(crate::expiry_alias::AliasRecord::line)
        .collect();
    let k = |strike: &str, side: &str| format!("NSE-FNO-NTPC-2019-03-28-{strike}-{side}");
    for (old, new, until) in [
        (k("15000", "CE"), k("12500", "CE"), 0),
        (k("15000", "PE"), k("12500", "PE"), 0),
        (k("16200", "PE"), k("13500", "PE"), stale_until),
        (k("16800", "PE"), k("14000", "PE"), 0),
        (k("18000", "CE"), k("15000", "CE"), 0),
    ] {
        want.push(format!(
            "lineage {old} {new} ex=2019-03-19 factor=6/5 lot_old=4000 lot_new=4800 \
             oi_unadjusted_until={until} v=1"
        ));
    }
    want.push(format!(
        "stitched {} ex=2019-03-19 before=2019-03-28-12500-CE after=2019-03-28-18000-CE v=1",
        k("15000", "CE")
    ));
    // The event itself (D-3355), written on the ex-date: a lot step that
    // paired all five pre-ex keys.
    want.push(
        "event NSE-FNO-NTPC ex=2019-03-19 kind=lot_step shift=0 lot_old=4000 lot_new=4800 \
         paired=5 stopped=0 v=1"
            .to_owned(),
    );
    // The reused `150PE` text first trades again on the day after.
    want.push(format!(
        "stitched {} ex=2019-03-19 before=2019-03-28-12500-PE after=- v=1",
        k("15000", "PE")
    ));
    want
}

/// The lot ledger [`ex_world`] must leave: each contract when first seen,
/// and each reused text again when its new contract first trades.
fn ex_lots() -> Vec<String> {
    let k = |strike: &str, side: &str| format!("NSE-FNO-NTPC-2019-03-28-{strike}-{side}");
    let mut want = Vec::new();
    for (strike, side) in [
        ("15000", "PE"),
        ("16200", "PE"),
        ("16800", "PE"),
        ("15000", "CE"),
        ("18000", "CE"),
    ] {
        want.push(format!(
            "lot {} from=2019-03-18 lot=4000 basis=ltq_gcd v=1",
            k(strike, side)
        ));
    }
    for (strike, side) in [
        ("12500", "PE"),
        ("13500", "PE"),
        ("14000", "PE"),
        ("12500", "CE"),
        ("15000", "CE"),
    ] {
        want.push(format!(
            "lot {} from=2019-03-19 lot=4800 basis=ltq_gcd v=1",
            k(strike, side)
        ));
    }
    want.push(format!(
        "lot {} from=2019-03-20 lot=4800 basis=ltq_gcd v=1",
        k("15000", "PE")
    ));
    want
}

/// DB-G71, DB-G72, DB-G77 (D-3331, D-3343): the options import detects the
/// adjustment from the data and writes, never splits: a `lineage` record
/// per renamed contract (old key, new key, factor, both inferred lots, and
/// how long the vendor's OI stayed unadjusted), a `stitched` record per
/// reused text, every `alias` row once, and the lots. The bars are not
/// touched: the reused `150PE` holds both contracts, as the record says.
/// A re-run writes nothing; a run that starts on the ex-date reads the day
/// before to know what was renamed; a rescan writes the same records.
#[test]
fn the_options_import_records_a_renamed_contract_and_a_reused_key() {
    let root = scratch("wp11b-lineage");
    ex_world(&root);
    let (d1, d2, d3) = ex_days();
    let store = root.join("S");
    let got = run_from(
        &root,
        (Src::Zips, ImportKind::Options),
        (d1, d3),
        (&store, &[]),
        false,
    );
    assert!(got.failures.is_empty(), "{:?}", got.failures);
    let lineage = crate::gdfl_lineage::lineage_path(&store);
    let lots = crate::gdfl_lineage::lots_path(&store);
    assert_eq!(ledger_lines(&lineage), ex_lineage());
    assert_eq!(ledger_lines(&lots), ex_lots());
    assert_eq!(got.adjustments, 1);
    assert_eq!((got.lineage_lines, got.lot_lines), (ex_lineage().len(), 11));
    // Every record reads back as the API reads it.
    for line in ledger_lines(&lineage) {
        assert!(
            line.starts_with("alias ")
                || crate::gdfl_lineage::LineageRecord::parse(&line).is_some()
                || crate::gdfl_lineage::StitchedRecord::parse(&line).is_some()
                || crate::gdfl_lineage::EventRecord::parse(&line).is_some(),
            "{line}"
        );
    }
    // The bars are never altered or split: 150PE holds the pre-ex contract's
    // two bars and the post-ex one's.
    let reused = bars_on(
        &store,
        (
            "FNO",
            "NTPC",
            Some(Contract::parse("2019-03-28-15000-PE").unwrap()),
        ),
        d1,
    );
    assert_eq!(
        reused.iter().map(|bar| bar.close).collect::<Vec<_>>(),
        [500, 510, 100]
    );
    // A re-run writes nothing.
    let again = run_from(
        &root,
        (Src::Zips, ImportKind::Options),
        (d1, d3),
        (&store, &[]),
        false,
    );
    assert_eq!(
        (again.lineage_lines, again.lot_lines, again.days_skipped),
        (0, 0, 3)
    );
    assert_eq!(ledger_lines(&lineage), ex_lineage());
    // A run that starts on the ex-date folds the day before in, read-only.
    let late = root.join("L");
    let from_ex = run_from(
        &root,
        (Src::Store, ImportKind::Options),
        (d2, d3),
        (&late, &[]),
        false,
    );
    assert!(from_ex.failures.is_empty(), "{:?}", from_ex.failures);
    assert_eq!(
        ledger_lines(&crate::gdfl_lineage::lineage_path(&late)),
        ex_lineage()
    );
    assert_eq!(
        ledger_lines(&crate::gdfl_lineage::lots_path(&late)),
        ex_lots()
    );
    assert!(
        bars_tree_has_no_day(&late, d1),
        "the warm-up day is read, never filed"
    );
    // `cli gdfl-dayflags options ... --lineage` writes the same records for
    // days already imported, and the day ledger.
    std::fs::remove_file(&lineage).unwrap();
    std::fs::remove_file(&lots).unwrap();
    let rescan = rescan_nfo(
        &NfoZips::new(&root.join("zips/options")),
        (d1, d3),
        &store,
        true,
    )
    .unwrap();
    assert!(rescan.failures.is_empty(), "{:?}", rescan.failures);
    assert_eq!((rescan.days_read, rescan.adjustments), (3, 1));
    assert_eq!(ledger_lines(&lineage), ex_lineage());
    assert_eq!(ledger_lines(&lots), ex_lots());
    std::fs::remove_dir_all(root).unwrap();
}

/// Whether no bar of `store` lies on `on`.
fn bars_tree_has_no_day(store: &Path, on: Day) -> bool {
    let start = micros_at(on, 0);
    let end = micros_at(on, 86_399);
    tree(store)
        .into_keys()
        .filter(|path| path.extension().is_some_and(|ext| ext == "bin"))
        .all(|path| {
            let rel = path.to_string_lossy().into_owned();
            let parts: Vec<&str> = rel.split('/').collect();
            // bars/gdfl/NSE/FNO/<under>/<contract>/1s/<yyyy-mm>.bin
            let (Some(under), Some(contract)) = (parts.get(4), parts.get(5)) else {
                return true;
            };
            bars_on(store, ("FNO", under, Contract::parse(contract)), on)
                .iter()
                .all(|bar| bar.ts_micros < start || bar.ts_micros > end)
        })
}

/// A day none of whose files was read clean writes no kind-day line: there
/// is nothing to count, and the journal names why (D-3317's `refused`).
#[test]
fn a_day_with_no_file_read_writes_no_ledger_line() {
    let on = day(2019, 5, 30);
    let root = scratch("wp11b-nothing-read");
    let bad = nfo_csv(
        "NIFTY30MAY1911900CE",
        on,
        ("1.00", "1.10"),
        &[(hms_at(9, 20, 0), "50.125", 75, 9_000)],
    );
    world_of(
        &root,
        &[(
            on,
            Tree::Options,
            vec![(entry_of(Tree::Options, on, "NIFTY30MAY1911900CE"), bad)],
        )],
    );
    let store = root.join("S");
    let got = run_from(
        &root,
        (Src::Store, ImportKind::Options),
        (on, on),
        (&store, &[]),
        false,
    );
    assert_eq!((got.files, got.files_refused), (0, 1));
    assert_eq!(got.closed_refused, vec![on]);
    assert_eq!(got.flags_lines, 0);
    assert!(flags_lines(&store).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

// ═══ WP-16: W workers leave the bytes one worker leaves ═══════════════════

/// Every file of a store, the wall clock taken out of its provenance records
/// (`landed_at_micros`, the one field a run writes from the clock, re-encoded
/// as zero so the record's own check still covers every other byte) and out
/// of its generation records (`promoted_at_micros` and the seal over it, a
/// back-filled month's, D-3350); every other file byte for byte.
fn store_bytes(store: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    tree(store)
        .into_iter()
        .map(|(rel, bytes)| {
            if rel.extension().is_some_and(|ext| ext == "prv") {
                let mut out = bytes.get(..64).unwrap().to_vec();
                for (ordinal, chunk) in (0_u64..).zip(bytes.get(64..).unwrap().chunks(256)) {
                    let image = <&[u8; 256]>::try_from(chunk).unwrap();
                    let mut record = store::provenance::Record::decode(image, ordinal).unwrap();
                    assert!(record.landed_at_micros > 0, "{}", rel.display());
                    record.landed_at_micros = 0;
                    out.extend_from_slice(&record.image());
                }
                (rel, out)
            } else if rel
                .extension()
                .is_some_and(|ext| ext == store::generation::EXTENSION)
            {
                // A generation record's promotion time (bytes 48..56) and the
                // seal over it (60..64) are the clock's too; every other byte
                // of every record stands.
                let mut out = bytes;
                for record in out.chunks_mut(64) {
                    for at in (48..56).chain(60..64) {
                        if let Some(byte) = record.get_mut(at) {
                            *byte = 0;
                        }
                    }
                }
                (rel, out)
            } else {
                (rel, bytes)
            }
        })
        .collect()
}

/// The options files of `d` for the workers' test: `n` NIFTY strikes, both
/// sides, a BANKNIFTY pair, one file whose price the reader refuses, one
/// that never traded and one whose name does not decode; every price
/// invented.
fn many_options(d: Day, n: u32) -> Vec<(String, Vec<u8>)> {
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let entry = |ticker: &str| format!("{folder}\\Options\\{ticker}.NFO.csv");
    let mut out = Vec::new();
    for i in 0..n {
        let strike = 21_000 + i * 50;
        for side in ["CE", "PE"] {
            let ticker = format!("NIFTY04APR24{strike}{side}");
            let ltp = format!("{}.{:02}", 40 + i, (i % 20) * 5);
            out.push((entry(&ticker), option_file(&ticker, d, &ltp)));
        }
        if i == n / 3 {
            let bad = "NIFTY04APR2422975CE";
            out.push((entry(bad), option_file(bad, d, "101.505")));
            let idle = "NIFTY04APR2422985PE";
            let rows: Vec<String> = (0..5)
                .map(|s| {
                    row(
                        &format!("{idle}.NFO"),
                        d,
                        NINE_FIFTEEN + s * 60,
                        "7.05",
                        0,
                        900,
                    )
                })
                .collect();
            out.push((entry(idle), csv(&rows)));
        }
        if i == 2 * n / 3 {
            let odd = "NIFTYQQ04APR24";
            out.push((entry(odd), option_file(odd, d, "3.05")));
        }
    }
    for side in ["CE", "PE"] {
        let ticker = format!("BANKNIFTY10APR2447000{side}");
        out.push((entry(&ticker), option_file(&ticker, d, "212.35")));
    }
    out
}

/// A run of `kind` from `src` under `root` into `store` on `workers`, its
/// origin the one given (so every run line is the same line), each day made
/// durable as [`Durability::DEFAULT`] makes it.
fn run_on(
    root: &Path,
    (src, kind): (Src, ImportKind),
    (from, to): (Day, Day),
    store: &Path,
    (workers, origin): (usize, &Origin),
) -> Report {
    run_durable(
        root,
        (src, kind),
        (from, to),
        store,
        (workers, origin, Durability::DEFAULT),
    )
    .unwrap()
}

/// [`run_on`] with the durability given, answering what the run answers.
fn run_durable(
    root: &Path,
    (src, kind): (Src, ImportKind),
    (from, to): (Day, Day),
    store: &Path,
    (workers, origin, durability): (usize, &Origin, Durability),
) -> Result<Report, ImportRefusal> {
    let run = Run {
        kind,
        from,
        to,
        only: &[],
        store_root: store,
    };
    let (_, path) = source_path(root, src, kind);
    let options = RunOptions {
        origin: Some(origin),
        retry_refused: false,
        workers: Workers::new(workers).unwrap(),
        durability,
    };
    match (src, kind) {
        (Src::Store, ImportKind::Options) => run_nfo_from(&NfoTickStore::new(&path), &run, options),
        (Src::Zips, ImportKind::Options) => run_nfo_from(&NfoZips::new(&path), &run, options),
        (Src::Store, _) => run_cm_from(&TickStore::new(&path), &run, options),
        (Src::Zips, _) => run_cm_from(&Archive::open(&path).unwrap(), &run, options),
    }
}

/// WP-16 (DB-N25), IA-GR-08: THE FILES OF A DAY ON W WORKERS LEAVE THE BYTES
/// ONE WORKER LEAVES — every bar file, checksum and time index, the census,
/// the journal (its run lines included, the origin being one), the day ledger,
/// the lineage and lot ledgers, and every provenance record but the second it
/// was written — and the same report, failures in the same order. Three
/// options days of 66 files, from both sources, on 2, 5 and 12 workers.
#[test]
fn w_workers_build_the_same_store_journal_and_census_as_one() {
    let root = scratch("wp16-workers");
    let days = [d1(), d2(), d3()];
    let plan: Vec<DayPlan> = days
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 30)))
        .collect();
    world_of(&root, &plan);
    for src in [Src::Store, Src::Zips] {
        let (kind_of_source, path) = source_path(&root, src, ImportKind::Options);
        let origin = Origin {
            started: 1_790_000_000,
            ..Origin::measure(kind_of_source, &path, Some(COMMIT))
        };
        let one_store = root.join(format!("one-{}", src as u8));
        let one = run_on(
            &root,
            (src, ImportKind::Options),
            (d1(), d3()),
            &one_store,
            (1, &origin),
        );
        assert_eq!(one.days_imported, 3);
        assert_eq!(
            (one.files, one.files_refused),
            (3 * 63, 3 * 2),
            "{:?}",
            one.failures
        );
        assert!(one.provenance_records > 0 && one.seconds_committed > 0);
        let want = store_bytes(&one_store);
        assert!(want.keys().any(|rel| rel.starts_with("manifest")));
        assert!(want.contains_key(Path::new("imports/gdfl.journal")));
        for workers in [2, 5, Workers::MAX] {
            let store = root.join(format!("w{workers}-{}", src as u8));
            let got = run_on(
                &root,
                (src, ImportKind::Options),
                (d1(), d3()),
                &store,
                (workers, &origin),
            );
            assert_eq!(got, one, "{workers} workers: the same report");
            let have = store_bytes(&store);
            assert_eq!(
                have.keys().collect::<Vec<_>>(),
                want.keys().collect::<Vec<_>>(),
                "{workers} workers: the same files"
            );
            for (rel, bytes) in &want {
                assert!(
                    have.get(rel) == Some(bytes),
                    "{workers} workers: {} differs",
                    rel.display()
                );
            }
        }
        // A run that starts after d1 warms the lineage up on d1's files
        // first (D-3331), read on the workers too: the same ledgers.
        let late: Vec<_> = [1, 6]
            .iter()
            .map(|&workers| {
                let store = root.join(format!("late{workers}-{}", src as u8));
                let report = run_on(
                    &root,
                    (src, ImportKind::Options),
                    (d2(), d3()),
                    &store,
                    (workers, &origin),
                );
                (report, store_bytes(&store))
            })
            .collect();
        assert_eq!(late[0].0.days_imported, 2);
        assert_eq!(late[0], late[1], "the warm-up on 6 workers");
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// WP-16: a day's refusals keep the order one pass over its listing names
/// them in, whether the listing refused an entry (two names of one contract)
/// or the reader refused a file (a price with three places), whichever
/// worker read it.
#[test]
fn a_days_refusals_keep_listing_order_on_any_number_of_workers() {
    let root = scratch("wp16-order");
    let d = day(2018, 12, 3);
    let folder = crate::gdfl_nfo::day_folder_name(d);
    let entry = |ticker: &str| format!("{folder}\\Options\\{ticker}.NFO.csv");
    let mut entries = vec![(
        entry("NIFTY27DEC1810500CE"),
        option_file("NIFTY27DEC1810500CE", d, "5"),
    )];
    for strike in (10_600..11_400).step_by(100) {
        let good = format!("NIFTY18DEC{strike}CE");
        entries.push((entry(&good), option_file(&good, d, "5.05")));
        let bad = format!("NIFTY18DEC{}PE", strike + 50);
        entries.push((entry(&bad), option_file(&bad, d, "5.055")));
    }
    entries.push((
        entry("NIFTY18DEC10500CE"),
        option_file("NIFTY18DEC10500CE", d, "5"),
    ));
    world_of(&root, &[(d, Tree::Options, entries)]);
    let (kind_of_source, path) = source_path(&root, Src::Store, ImportKind::Options);
    let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
    let named = |report: &Report| -> Vec<String> {
        report
            .failures
            .iter()
            .map(|f| f.instrument.clone())
            .collect()
    };
    let one = run_on(
        &root,
        (Src::Store, ImportKind::Options),
        (d, d),
        &root.join("one"),
        (1, &origin),
    );
    let mut want = vec![format!("{d} NIFTY27DEC1810500CE")];
    for strike in (10_600..11_400).step_by(100) {
        want.push(format!("{d} NIFTY18DEC{}PE", strike + 50));
    }
    want.push(format!("{d} NIFTY18DEC10500CE"));
    assert_eq!(named(&one), want, "listing order: {:?}", one.failures);
    for workers in [3, 8] {
        let got = run_on(
            &root,
            (Src::Store, ImportKind::Options),
            (d, d),
            &root.join(format!("w{workers}")),
            (workers, &origin),
        );
        assert_eq!(named(&got), want, "{workers} workers");
        assert_eq!(got, one, "{workers} workers");
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// WP-16 (IA-GR-08's `.auc`): the closing-auction sidecars of a stocks day,
/// and its index day, are the bytes one worker writes.
#[test]
fn w_workers_write_the_auction_sidecars_and_index_days_one_worker_writes() {
    use std::fmt::Write as _;
    let on = day(2026, 9, 24);
    let root = scratch("wp16-auction");
    let shares = ["RELIANCE", "SBIN", "INFY", "TCS", "ITC"];
    let mut files = Vec::new();
    let mut master = String::from("FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn\n");
    for (n, share) in (0_u32..).zip(shares) {
        let stem = format!("{share}.NSE");
        let mut rows: Vec<String> = (0..10)
            .map(|i| {
                quoted_row(
                    &stem,
                    on,
                    hms_at(15, 5, 0) + i * 50,
                    &format!("{}.{i}5", 1_000 + n * 100),
                    ("1000.00", "1999.90"),
                    (3, 0),
                )
            })
            .collect();
        rows.push(quoted_row(
            &stem,
            on,
            hms_at(15, 28, 7),
            &format!("{}.65", 1_001 + n * 100),
            ("0", "0.00"),
            (98_765, 0),
        ));
        files.push((entry_of(Tree::Stocks, on, &stem), csv(&rows)));
        let at = brutex_core::universe::FNO_INDEX.position(share).unwrap();
        let isin = brutex_core::universe::FNO_UNDERLYINGS_ISIN[at];
        writeln!(master, "{},{share},EQ,{isin},1", 2_000 + n).unwrap();
    }
    world_of(&root, &[(on, Tree::Stocks, files)]);
    let masters = root.join("masters");
    crate::cash_session_cache::install(&masters, on, &gzip(&master)).unwrap();
    let scope = std::sync::Arc::new(
        crate::gdfl_cm::CmScope::stored(std::iter::empty::<&str>())
            .unwrap()
            .with_masters(masters),
    );
    let source = TickStore::new(&root.join("ts")).with_scope(scope);
    let (kind_of_source, path) = source_path(&root, Src::Store, ImportKind::Stocks);
    let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
    let stocks = |workers: usize, store: &Path| {
        let run = Run {
            kind: ImportKind::Stocks,
            from: on,
            to: on,
            only: &[],
            store_root: store,
        };
        let options = RunOptions {
            origin: Some(&origin),
            retry_refused: false,
            workers: Workers::new(workers).unwrap(),
            durability: Durability::DEFAULT,
        };
        run_cm_from(&source, &run, options).unwrap()
    };
    let one = stocks(1, &root.join("one"));
    assert!(one.failures.is_empty(), "{:?}", one.failures);
    assert_eq!(one.files, shares.len());
    let want = store_bytes(&root.join("one"));
    let sidecars = want
        .keys()
        .filter(|rel| rel.extension().is_some_and(|ext| ext == "auc"))
        .count();
    assert_eq!(sidecars, shares.len(), "one sidecar per share: {want:?}");
    let got = stocks(4, &root.join("four"));
    assert_eq!(got, one);
    assert_eq!(store_bytes(&root.join("four")), want);

    world(&root, &[d1()]);
    let (kind_of_source, path) = source_path(&root, Src::Zips, ImportKind::Indices);
    let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
    let indices: Vec<_> = [1, 3]
        .iter()
        .map(|&workers| {
            let store = root.join(format!("idx{workers}"));
            let report = run_on(
                &root,
                (Src::Zips, ImportKind::Indices),
                (d1(), d1()),
                &store,
                (workers, &origin),
            );
            (report, store_bytes(&store))
        })
        .collect();
    assert!(
        indices[0].0.files == 2 && indices[0].0.failures.is_empty(),
        "{:?}",
        indices[0].0
    );
    assert_eq!(indices[0], indices[1]);
    std::fs::remove_dir_all(root).unwrap();
}

/// Every month file of `store` opens and reads every committed record back,
/// each block against its sealed checksum: no month is half written. Answers
/// how many months hold a commit, and how many were cut off while they were
/// being created.
///
/// A MONTH CUT OFF IN ITS CREATION IS NOT A HALF MONTH, and is told apart by
/// its bytes rather than excused: it is no longer than the header region,
/// holds nothing past its first slot, and that slot does not decode, so it
/// never committed a bar (`store::file`'s `is_interrupted_genesis`, D-1521).
/// A reader refuses it by name and the writer's next open creates it again,
/// which the caller proves by comparing the resumed store byte for byte. Any
/// other month that does not open fails the test here.
fn every_month_reads_whole(store: &Path) -> (usize, usize) {
    let (mut months, mut unborn) = (0, 0);
    for rel in tree(store).into_keys() {
        let text = rel.to_string_lossy().into_owned();
        if !(text.starts_with("bars/") && text.ends_with(".bin")) {
            continue;
        }
        let parts: Vec<&str> = text.split('/').collect();
        let (symbol, contract) = (parts[4], Contract::parse(parts[5]).unwrap());
        let month = parts[7].trim_end_matches(".bin");
        let (y, m) = month.split_once('-').unwrap();
        let path = StorePath::new(PathParts {
            vendor: Vendor::Gdfl,
            exchange: "NSE",
            segment: "FNO",
            symbol,
            contract: Some(contract),
            timeframe: Timeframe::SECOND_1,
            month: store::path::YearMonth::new(y.parse().unwrap(), m.parse().unwrap()).unwrap(),
            file: FileKind::Bars,
        })
        .unwrap();
        let file = match BarFile::open_existing(store, path, symbol_id_of(symbol)) {
            Ok(file) => file,
            Err(why) => {
                let bytes = std::fs::read(store.join(&rel)).unwrap();
                let (slot, rest) = bytes.split_at(bytes.len().min(store::format::SLOT_LEN));
                assert!(
                    u64::try_from(bytes.len()).unwrap() <= store::format::HEADER_LEN
                        && rest.iter().all(|&byte| byte == 0)
                        && store::header::Header::decode(slot).is_err(),
                    "{text} is neither whole nor cut off in its creation ({} bytes): {why}",
                    bytes.len()
                );
                unborn += 1;
                continue;
            }
        };
        for index in 0..file.header().n_valid {
            file.read_record(index)
                .unwrap_or_else(|why| panic!("{text} record {index}: {why}"));
        }
        months += 1;
    }
    (months, unborn)
}

/// WP-16 (DB-N25): A KILL -9 AT ANY POINT LEAVES EVERY MONTH AT ITS OLD STATE
/// OR ITS NEW ONE, NEVER HALF, AND THE DAY IS DONE AGAIN ON RESUME. A child of
/// this test binary imports three options days on four workers and is killed
/// (SIGKILL) at points spread over its run; after each kill every month opens
/// and reads back whole (or was cut off in its creation, holds no commit and
/// belongs to a day still open), the journal and the census load, and a resumed run
/// on three workers finishes with no failure and leaves the bars, checksums,
/// time indexes and census of an import that was never killed, byte for
/// byte. (A provenance record for a landing the kill cut off before its
/// record is not written again by the resume, which finds the bars present;
/// `/provenance.json` labels such bars reconstructed, as before WP-16.)
#[test]
fn a_kill_at_any_point_leaves_no_half_month_and_the_resume_lands_the_same_store() {
    const CHILD: &str = "BRUTEX_WP16_KILL_CHILD";
    let child_of = |root: &Path, store: &Path| {
        let (kind_of_source, path) = source_path(root, Src::Zips, ImportKind::Options);
        let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
        run_on(
            root,
            (Src::Zips, ImportKind::Options),
            (d1(), d3()),
            store,
            (4, &origin),
        )
    };
    if let Some(dirs) = std::env::var_os(CHILD) {
        let dirs = PathBuf::from(dirs);
        let _report = child_of(&dirs.join("world"), &dirs.join("store"));
        return;
    }
    let root = scratch("wp16-kill");
    let world_root = root.join("world");
    let plan: Vec<DayPlan> = [d1(), d2(), d3()]
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 45)))
        .collect();
    world_of(&world_root, &plan);
    let clean = root.join("clean");
    let started = std::time::Instant::now();
    let reference = child_of(&world_root, &clean);
    let took = started.elapsed();
    assert!(reference.failures.len() == 6, "{:?}", reference.failures);
    let want: BTreeMap<PathBuf, Vec<u8>> = store_bytes(&clean)
        .into_iter()
        .filter(|(rel, _)| {
            !rel.starts_with("imports") && rel.extension().is_none_or(|e| e != "prv")
        })
        .collect();
    let mut interrupted = 0;
    for (n, eighths) in [1_u32, 2, 3, 4, 5, 6, 7].into_iter().enumerate() {
        let run = root.join(format!("kill-{n}"));
        std::fs::create_dir_all(&run).unwrap();
        std::os::unix::fs::symlink(&world_root, run.join("world")).unwrap();
        let store = run.join("store");
        std::fs::create_dir_all(&store).unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "gdfl_import::tests::a_kill_at_any_point_leaves_no_half_month_and_the_resume_lands_the_same_store",
                "--test-threads=1",
            ])
            .env(CHILD, &run)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(took * eighths / 8);
        child.kill().unwrap();
        let status = child.wait().unwrap();
        let journal = std::fs::read_to_string(journal_path(&store)).unwrap_or_default();
        let open_day = journal
            .lines()
            .last()
            .is_some_and(|l| l.starts_with("begin "));
        if !status.success() && open_day {
            interrupted += 1;
        }
        // Whatever the kill left: every month whole, the journal and the
        // census readable.
        let (months, unborn) = every_month_reads_whole(&store);
        // A month cut off in its creation committed nothing, so the day that
        // was creating it cannot have closed: the kill left it open.
        assert!(
            unborn == 0 || open_day,
            "kill {n}: {unborn} unborn, day closed"
        );
        Journal::load(journal_path(&store)).unwrap();
        let census = std::fs::read(manifest_path(&store, Vendor::Gdfl)).unwrap_or_default();
        Manifest::open_image(Vendor::Gdfl, &census).unwrap();
        // The resume finishes the run and lands what an unkilled run lands.
        let (kind_of_source, path) = source_path(&world_root, Src::Zips, ImportKind::Options);
        let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
        let resumed = run_on(
            &world_root,
            (Src::Zips, ImportKind::Options),
            (d1(), d3()),
            &store,
            (3, &origin),
        );
        let refused: Vec<_> = resumed
            .failures
            .iter()
            .filter(|f| {
                !f.instrument.ends_with("NIFTY04APR2422975CE")
                    && !f.instrument.ends_with("NIFTYQQ04APR24")
            })
            .collect();
        assert!(
            refused.is_empty(),
            "kill {n} after {months} months, {unborn} unborn: {refused:?}"
        );
        // The resume created again every month the kill cut off in creation.
        let (_whole, still_unborn) = every_month_reads_whole(&store);
        assert_eq!(still_unborn, 0, "kill {n}");
        let have: BTreeMap<PathBuf, Vec<u8>> = store_bytes(&store)
            .into_iter()
            .filter(|(rel, _)| {
                !rel.starts_with("imports") && rel.extension().is_none_or(|e| e != "prv")
            })
            .collect();
        assert_eq!(
            have.keys().collect::<Vec<_>>(),
            want.keys().collect::<Vec<_>>(),
            "kill {n}"
        );
        for (rel, bytes) in &want {
            assert!(
                have.get(rel) == Some(bytes),
                "kill {n}: {} differs",
                rel.display()
            );
        }
    }
    assert!(
        interrupted >= 2,
        "only {interrupted} kill(s) landed inside a day"
    );
    std::fs::remove_dir_all(root).unwrap();
}

// ═══ The summary events a run tells (D-3317, D-4878) ════════════════════

/// One summary event as [`note_day`] built it: its words, its level and the
/// text fields it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Summary {
    pub(super) words: String,
    pub(super) warn: bool,
    pub(super) text: BTreeMap<String, String>,
}

thread_local! {
    /// Every summary event the runs of THIS thread told, in order. A run
    /// closes its days on the thread that called it and each test runs on a
    /// thread of its own, so a test reads only its own runs' events.
    static SUMMARIES: std::cell::RefCell<Vec<Summary>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Keeps `event` for the test on this thread ([`summaries_taken`]).
pub(super) fn summary_emitted(event: &telemetry::Event<'_>) {
    let text = event
        .fields()
        .iter()
        .filter_map(|(name, value)| Some(((*name).to_owned(), value.as_str()?.to_owned())))
        .collect();
    SUMMARIES.with_borrow_mut(|kept| {
        kept.push(Summary {
            words: event.message().to_owned(),
            warn: event.level() == telemetry::Level::Warn,
            text,
        });
    });
}

/// The summary events told on this thread since the last call, in order.
pub(super) fn summaries_taken() -> Vec<Summary> {
    SUMMARIES.with_borrow_mut(std::mem::take)
}

// ═══ WP-16: the day made durable (D-3353) ═══════════════════════════════

/// What a test asks of each flush a day's batch issues under its store: run
/// before the flush, on whichever thread issues it; an `Err` is the flush's.
type DayFlushHook = Box<dyn Fn(&Path) -> std::io::Result<()> + Send + Sync>;

/// The hooks, by the store root they watch: a day's flushes run on threads
/// of their own and the tests of this binary run in parallel, so each test
/// hooks only its own root.
static DAY_FLUSH_HOOKS: std::sync::Mutex<Vec<(PathBuf, DayFlushHook)>> =
    std::sync::Mutex::new(Vec::new());

/// Runs the hook of the store `path` lies under, if a test set one.
pub(super) fn before_a_day_flush(path: &Path) -> std::io::Result<()> {
    let hooks = DAY_FLUSH_HOOKS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for (root, hook) in hooks.iter() {
        if path.starts_with(root) {
            return hook(path);
        }
    }
    Ok(())
}

/// Hooks every day flush under `root` until the answer drops.
struct HookedFlushes(PathBuf);

impl HookedFlushes {
    fn new(root: &Path, hook: DayFlushHook) -> Self {
        DAY_FLUSH_HOOKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((root.to_path_buf(), hook));
        Self(root.to_path_buf())
    }
}

impl Drop for HookedFlushes {
    fn drop(&mut self) {
        DAY_FLUSH_HOOKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .retain(|(root, _)| *root != self.0);
    }
}

/// WP-16 (DB-N25, D-3353): THE DURABILITY UNIT CHANGES NO BYTE A RUN LEAVES.
/// Three options days imported with every write flushed as it is written
/// (one worker, as before WP-16), with each day made durable through `O_SYNC`
/// writes and one flush per device (eight workers), and with each day made
/// durable by a flush of every file (four workers), leave the same bars,
/// checksums, time indexes, provenance records (but the second each was
/// written), census, journal and day ledger, and the same report.
#[test]
fn the_durability_unit_changes_no_byte_a_run_leaves() {
    let root = scratch("wp16-durability");
    let plan: Vec<DayPlan> = [d1(), d2(), d3()]
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 20)))
        .collect();
    world_of(&root, &plan);
    let (kind_of_source, path) = source_path(&root, Src::Zips, ImportKind::Options);
    let origin = Origin {
        started: 1_790_000_000,
        ..Origin::measure(kind_of_source, &path, Some(COMMIT))
    };
    let runs = [
        (1, Durability::EachWrite),
        (8, Durability::Day(store::file::BatchFlush::OncePerDevice)),
        (4, Durability::Day(store::file::BatchFlush::EveryFile)),
    ];
    let done: Vec<(Report, BTreeMap<PathBuf, Vec<u8>>)> = runs
        .iter()
        .map(|&(workers, durability)| {
            let store = root.join(durability.as_str());
            let report = run_durable(
                &root,
                (Src::Zips, ImportKind::Options),
                (d1(), d3()),
                &store,
                (workers, &origin, durability),
            )
            .unwrap();
            (report, store_bytes(&store))
        })
        .collect();
    let (want_report, want) = &done[0];
    assert_eq!(want_report.days_imported, 3);
    assert!(want_report.seconds_committed > 0 && want_report.provenance_records > 0);
    assert!(want.contains_key(Path::new("imports/gdfl.journal")));
    assert!(want.keys().any(|rel| rel.starts_with("manifest")));
    for ((report, have), (_, durability)) in done.iter().zip(runs).skip(1) {
        assert_eq!(
            report,
            want_report,
            "{}: the same report",
            durability.as_str()
        );
        assert_eq!(
            have.keys().collect::<Vec<_>>(),
            want.keys().collect::<Vec<_>>(),
            "{}: the same files",
            durability.as_str()
        );
        for (rel, bytes) in want {
            assert!(
                have.get(rel) == Some(bytes),
                "{}: {} differs",
                durability.as_str(),
                rel.display()
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// The journal's `begin` lines and closing lines, counted.
fn journal_counts(store: &Path) -> (usize, usize) {
    let text = std::fs::read_to_string(journal_path(store)).unwrap_or_default();
    let begins = text
        .lines()
        .filter(|line| line.starts_with("begin "))
        .count();
    let closes = text
        .lines()
        .filter(|line| {
            ["done ", "incomplete ", "refused "]
                .iter()
                .any(|verb| line.starts_with(verb))
        })
        .count();
    (begins, closes)
}

/// WP-16 (D-3353): EACH DAY IS MADE DURABLE AFTER ITS LAST FILE AND BEFORE
/// ITS CLOSING LINE. Every flush a day's batch issues sees the journal with
/// that day begun and not closed, and every day before it closed; each of
/// the three days issues some.
#[test]
fn each_day_is_made_durable_before_its_closing_line() {
    let root = scratch("wp16-order-durable");
    let plan: Vec<DayPlan> = [d1(), d2(), d3()]
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 6)))
        .collect();
    world_of(&root, &plan);
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let seen: std::sync::Arc<std::sync::Mutex<Vec<(usize, usize)>>> = std::sync::Arc::default();
    let hook = {
        let (store, seen) = (store.clone(), std::sync::Arc::clone(&seen));
        HookedFlushes::new(
            &store.clone(),
            Box::new(move |_path| {
                seen.lock().unwrap().push(journal_counts(&store));
                Ok(())
            }),
        )
    };
    let (kind_of_source, path) = source_path(&root, Src::Zips, ImportKind::Options);
    let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
    let report = run_on(
        &root,
        (Src::Zips, ImportKind::Options),
        (d1(), d3()),
        &store,
        (5, &origin),
    );
    drop(hook);
    assert_eq!(report.days_imported, 3);
    let seen = seen.lock().unwrap().clone();
    assert!(
        seen.iter().all(|&(begins, closes)| begins == closes + 1),
        "a flush saw its day closed, or another day open: {seen:?}"
    );
    let mut days: Vec<usize> = seen.iter().map(|&(begins, _)| begins).collect();
    days.dedup();
    assert_eq!(days, [1, 2, 3], "every day made durable, in order");
    assert_eq!(journal_counts(&store), (3, 3));
    std::fs::remove_dir_all(root).unwrap();
}

/// WP-16 (D-3353): A DAY THAT CANNOT BE MADE DURABLE STOPS THE RUN AND STAYS
/// OPEN. The second day's flush is refused: the run answers
/// [`ImportRefusal::Durability`] naming the day and the store's words, the
/// first day is done, the second begun and never closed (so the next run
/// reads it again), the third never begun, and no census row or ledger line
/// was written for the second. Every month the refused batch wrote takes no
/// further append in this process, by name.
#[test]
fn a_day_that_cannot_be_made_durable_stops_the_run_and_stays_open() {
    let root = scratch("wp16-durable-refused");
    let plan: Vec<DayPlan> = [d1(), d2(), d3()]
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 4)))
        .collect();
    world_of(&root, &plan);
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let (kind_of_source, path) = source_path(&root, Src::Zips, ImportKind::Options);
    let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
    let census_after_d1: std::sync::Arc<std::sync::Mutex<Option<Vec<u8>>>> =
        std::sync::Arc::default();
    let hook = {
        let (at, census) = (store.clone(), std::sync::Arc::clone(&census_after_d1));
        HookedFlushes::new(
            &store.clone(),
            Box::new(move |_path| {
                if journal_counts(&at).0 == 2 {
                    let mut census = census.lock().unwrap();
                    if census.is_none() {
                        *census = Some(std::fs::read(manifest_path(&at, Vendor::Gdfl)).unwrap());
                    }
                    return Err(std::io::Error::other("injected day flush fault"));
                }
                Ok(())
            }),
        )
    };
    let refused = run_durable(
        &root,
        (Src::Zips, ImportKind::Options),
        (d1(), d3()),
        &store,
        (3, &origin, Durability::DEFAULT),
    );
    drop(hook);
    let Err(ImportRefusal::Durability {
        kind,
        day,
        why,
        done,
    }) = refused
    else {
        panic!("the run stopped at the day's flush: {refused:?}");
    };
    assert_eq!((kind, day), (ImportKind::Options, d2()));
    assert!(
        why.contains("syncing") && why.contains(&store.display().to_string()),
        "the flush the host refused, by path: {why}"
    );
    assert_eq!(done.days_imported, 1);
    assert!(
        done.failures
            .iter()
            .any(|f| f.instrument == format!("{} durability", d2())),
        "{:?}",
        done.failures
    );
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    let lines: Vec<&str> = text.lines().filter(|l| !l.starts_with("run ")).collect();
    assert!(
        lines[0].starts_with(&format!("begin options {}", d1())),
        "{text}"
    );
    assert!(
        lines[1].starts_with(&format!("done options {}", d1())),
        "{text}"
    );
    assert!(
        lines[2].starts_with(&format!("begin options {}", d2())),
        "{text}"
    );
    assert_eq!(lines.len(), 3, "d2 open, d3 never begun: {text}");
    assert_eq!(
        std::fs::read(manifest_path(&store, Vendor::Gdfl)).unwrap(),
        census_after_d1.lock().unwrap().clone().unwrap(),
        "no census row of d2"
    );
    let ledger = std::fs::read_to_string(quality::dayflags_path(&store)).unwrap_or_default();
    assert!(
        !ledger.contains(&d2().to_string()),
        "no ledger line of d2: {ledger}"
    );
    // In this process, the months d2 wrote take no further append.
    let again = run_on(
        &root,
        (Src::Zips, ImportKind::Options),
        (d2(), d2()),
        &store,
        (2, &origin),
    );
    assert_eq!(again.resumed, vec![d2()], "d2 is read again");
    assert!(
        again
            .failures
            .iter()
            .any(|f| f.why.contains("durability barrier") && f.why.contains("already failed")),
        "{:?}",
        again.failures
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// WP-16 (D-3353): A DAY AN EARLIER RUN LEFT OPEN IS CONFIRMED DURABLE BEFORE
/// THIS RUN CLOSES IT. The store is the one a run killed just before its
/// second day's closing line leaves: every month of that day written and
/// readable, the journal with the day begun and never closed, and (as far as
/// this run can know) not one of those writes ever flushed. Read again, every
/// month answers "already present" and nothing is written, so no writer has
/// a flush to defer; the run must still flush every file of every month the
/// day files (bars, checksums, time index, provenance) for itself, while the
/// day is still open, whatever its own durability. Without the confirmation
/// the day closed over writes that only the kernel held.
#[test]
fn a_day_an_earlier_run_left_open_is_confirmed_durable_before_it_closes() {
    let root = scratch("wp16-left-open");
    let plan: Vec<DayPlan> = [d1(), d2()]
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 4)))
        .collect();
    world_of(&root, &plan);
    let (kind_of_source, path) = source_path(&root, Src::Zips, ImportKind::Options);
    let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
    let closed_d2 = format!("done options {}", d2());
    for (workers, durability) in [
        (3, Durability::DEFAULT),
        (2, Durability::Day(store::file::BatchFlush::EveryFile)),
        (1, Durability::EachWrite),
    ] {
        let store = root.join(durability.as_str());
        std::fs::create_dir_all(&store).unwrap();
        let first = run_on(
            &root,
            (Src::Zips, ImportKind::Options),
            (d1(), d2()),
            &store,
            (4, &origin),
        );
        assert_eq!(first.days_imported, 2);
        // The kill: the second day's closing line was never written.
        let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
        let mut kept = String::with_capacity(journal.len());
        for line in journal.lines().filter(|line| !line.starts_with(&closed_d2)) {
            kept.push_str(line);
            kept.push('\n');
        }
        assert!(kept.len() < journal.len(), "{journal}");
        std::fs::write(journal_path(&store), kept).unwrap();
        let before = store_bytes(&store);
        // (the path flushed, whether the day was already closed when it was)
        let seen: std::sync::Arc<std::sync::Mutex<Vec<(PathBuf, bool)>>> =
            std::sync::Arc::default();
        let hook = {
            let (at, seen, closed_d2) = (
                store.clone(),
                std::sync::Arc::clone(&seen),
                closed_d2.clone(),
            );
            HookedFlushes::new(
                &store.clone(),
                Box::new(move |path| {
                    let text = std::fs::read_to_string(journal_path(&at)).unwrap();
                    let closed = text.lines().any(|line| line.starts_with(&closed_d2));
                    seen.lock().unwrap().push((path.to_path_buf(), closed));
                    Ok(())
                }),
            )
        };
        let resumed = run_durable(
            &root,
            (Src::Zips, ImportKind::Options),
            (d1(), d2()),
            &store,
            (workers, &origin, durability),
        )
        .unwrap();
        drop(hook);
        let word = durability.as_str();
        assert_eq!(resumed.resumed, vec![d2()], "{word}: d2 is read again");
        assert_eq!(resumed.seconds_committed, 0, "{word}: nothing was written");
        let seen = seen.lock().unwrap().clone();
        assert!(
            seen.iter().all(|(_, closed)| !closed),
            "{word}: a flush came after the day was closed: {seen:?}"
        );
        let flushed: std::collections::BTreeSet<&Path> =
            seen.iter().map(|(path, _)| path.as_path()).collect();
        let mut months = 0;
        for rel in before.keys() {
            if !rel.starts_with("bars") || rel.extension().is_some_and(|e| e == "lock") {
                continue;
            }
            let at = store.join(rel);
            assert!(
                flushed.contains(at.as_path()),
                "{word}: {} was never flushed",
                rel.display()
            );
            months += usize::from(rel.extension().is_some_and(|e| e == "bin"));
            // And every directory down to it, whose names that run never
            // flushed either.
            let mut dir = at.parent();
            while let Some(one) = dir.filter(|one| one.starts_with(&store)) {
                assert!(
                    flushed.contains(one),
                    "{word}: {} was never flushed",
                    one.display()
                );
                dir = one.parent();
            }
        }
        assert!(months >= 8, "{word}: four strikes, both sides: {months}");
        // Not a byte of a bar, a checksum or an index moved, and the day is
        // closed now.
        let after = store_bytes(&store);
        for (rel, bytes) in &before {
            if rel.starts_with("bars") {
                assert!(after.get(rel) == Some(bytes), "{word}: {}", rel.display());
            }
        }
        let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
        assert!(
            journal.lines().any(|line| line.starts_with(&closed_d2)),
            "{word}: {journal}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// WP-16, repair 1 (the second agent's A6; the operator's rule 4: torn,
/// truncated): A MONTH TORN INSIDE A DAY LEFT OPEN IS NAMED ON RESUME, AND THE
/// DAY NEVER CLOSES `done` OVER IT. What a power cut inside an open day can
/// leave under the default durability (D-3353): the day begun and never
/// closed, and a month whose header slot counts the day's records while the
/// records themselves never reached the drive. The resume reads the day
/// again, is refused that month by name, writes not one byte into it, closes
/// the day `incomplete` with the failure counted, and leaves the closed day
/// before it and every other month as they were. A second resume answers the
/// same: nothing here heals a torn month (a named limit, for
/// `docs/06-limits.md`; the versioned month repair is `store::repair`).
#[test]
fn a_month_torn_inside_a_day_left_open_is_named_on_resume_and_the_day_is_never_done() {
    let root = scratch("wp16-torn-open");
    let plan: Vec<DayPlan> = [d1(), d2()]
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 4)))
        .collect();
    world_of(&root, &plan);
    let (kind_of_source, path) = source_path(&root, Src::Zips, ImportKind::Options);
    let origin = Origin::measure(kind_of_source, &path, Some(COMMIT));
    let store = root.join("store");
    std::fs::create_dir_all(&store).unwrap();
    let source = (Src::Zips, ImportKind::Options);
    let lengths = |store: &Path| -> BTreeMap<PathBuf, usize> {
        tree(store)
            .into_iter()
            .filter(|(rel, _)| {
                rel.starts_with("bars") && rel.extension().is_some_and(|e| e == "bin")
            })
            .map(|(rel, bytes)| (rel, bytes.len()))
            .collect()
    };
    let first = run_on(&root, source, (d1(), d1()), &store, (4, &origin));
    assert_eq!(first.days_imported, 1);
    let after_d1 = lengths(&store);
    let second = run_on(&root, source, (d2(), d2()), &store, (4, &origin));
    assert_eq!(second.days_imported, 1);
    // The fixture's own refused files, which a whole day counts too.
    let refused = second.failures.len();
    // THE POWER CUT. The day's closing line was never written, and one month
    // both days filed is back at its first day's length, its header slot and
    // its sidecars still the second day's.
    let closed_d2 = format!("done options {}", d2());
    let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
    let kept: String = journal
        .lines()
        .filter(|line| !line.starts_with(&closed_d2))
        .flat_map(|line| [line, "\n"])
        .collect();
    assert!(kept.len() < journal.len(), "{journal}");
    std::fs::write(journal_path(&store), kept).unwrap();
    let (torn, short) = lengths(&store)
        .into_iter()
        .find_map(|(rel, now)| {
            let before = *after_d1.get(&rel)?;
            (before < now).then_some((rel, before))
        })
        .expect("a month both days filed");
    let cut = std::fs::OpenOptions::new()
        .write(true)
        .open(store.join(&torn))
        .unwrap();
    cut.set_len(u64::try_from(short).unwrap()).unwrap();
    drop(cut);
    let before = tree(&store);
    let name = torn.file_stem().unwrap().to_string_lossy().into_owned();
    let contract = torn
        .components()
        .rev()
        .nth(2)
        .unwrap()
        .as_os_str()
        .to_string_lossy()
        .into_owned();
    for attempt in ["the resume", "a second resume"] {
        let resumed = run_durable(
            &root,
            source,
            (d1(), d2()),
            &store,
            (3, &origin, Durability::DEFAULT),
        )
        .unwrap();
        assert_eq!(resumed.resumed, vec![d2()], "{attempt}: d2 is read again");
        assert_eq!(
            resumed.seconds_committed, 0,
            "{attempt}: nothing is written"
        );
        assert_eq!(
            resumed.failures.len(),
            refused + 1,
            "{attempt}: one failure more than the whole day had: {:?}",
            resumed.failures
        );
        let named = resumed
            .failures
            .iter()
            .filter(|failure| failure.why.contains(&*torn.to_string_lossy()))
            .count();
        assert_eq!(
            named, 1,
            "{attempt}: the torn month ({contract} {name}) is named once: {:?}",
            resumed.failures
        );
        let journal = std::fs::read_to_string(journal_path(&store)).unwrap();
        assert!(
            !journal.lines().any(|line| line.starts_with(&closed_d2)),
            "{attempt}: the day closed done over a torn month: {journal}"
        );
        let last = journal.lines().last().unwrap();
        assert!(
            last.starts_with(&format!("incomplete options {}", d2()))
                && last.contains(&format!(" failures={} ", refused + 1)),
            "{attempt}: {last}"
        );
        assert!(
            journal
                .lines()
                .any(|line| line.starts_with(&format!("done options {}", d1()))),
            "{attempt}: the closed day stays closed: {journal}"
        );
        // Not a byte of any month moved, the torn one included.
        let after = tree(&store);
        for (rel, bytes) in &before {
            if rel.starts_with("bars") && rel.extension().is_none_or(|e| e != "lock") {
                assert!(
                    after.get(rel) == Some(bytes),
                    "{attempt}: {}",
                    rel.display()
                );
            }
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

/// WP-16, repair 2: A RUN SAYS WHAT IT WAS STARTED WITH. "run settings"
/// carries the run's id, its workers and its durability unit by the word the
/// operator sets it with, so the audit trail can tell which durability a day
/// left open ran under; an `each-write` run leaves no "day made durable"
/// event to infer it from. Three fields, none dropped, at `info`.
#[test]
fn a_run_says_its_workers_and_durability_beside_its_id() {
    for (durability, word) in [
        (Durability::EachWrite, "each-write"),
        (
            Durability::Day(store::file::BatchFlush::OncePerDevice),
            "day",
        ),
        (
            Durability::Day(store::file::BatchFlush::EveryFile),
            "day-each-file",
        ),
    ] {
        for workers in [1_usize, 3, Workers::MAX] {
            let options = RunOptions {
                workers: Workers::new(workers).unwrap(),
                durability,
                ..RunOptions::default()
            };
            let event = settings_event(41, &options);
            assert_eq!(event.message(), "run settings");
            assert_eq!(event.level(), telemetry::Level::Info);
            assert_eq!(
                event.fields(),
                [
                    ("run", telemetry::Value::from(41_u64)),
                    ("workers", telemetry::Value::from(workers)),
                    ("durability", telemetry::Value::from(word)),
                ]
            );
            assert_eq!(event.dropped_fields(), 0);
        }
    }
}

/// WP-16, repair 1: THE DEFAULT WORKERS ARE READ FROM THE HOST, not written
/// down for one machine: every core but two, never fewer than one, never past
/// the eight where more stopped paying. What a caller may ask for is bounded
/// on its own, by the end of what was measured.
#[test]
fn the_default_workers_follow_the_hosts_cores() {
    for (cores, workers) in [
        (0, 1),
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 2),
        (9, 7),
        (10, 8),
        (14, 8),
        (usize::MAX, 8),
    ] {
        assert_eq!(Workers::for_cores(cores).get(), workers, "{cores} cores");
    }
    let host = std::thread::available_parallelism().map_or(1, std::num::NonZeroUsize::get);
    assert_eq!(Workers::for_host(), Workers::for_cores(host));
    assert_eq!(Workers::default(), Workers::for_host());
    assert!(Workers::new(Workers::for_host().get()).is_ok());
    assert!(Workers::new(0).is_err() && Workers::new(Workers::MAX + 1).is_err());
    assert_eq!(
        Workers::new(Workers::MAX).map(Workers::get),
        Ok(Workers::MAX)
    );
}

/// WP-16 with WP-12d (D-3353 beside D-3350): A DAY ITS MONTH LACKS IS
/// BACK-FILLED OUTSIDE THE DAY'S BATCH, IN LISTING ORDER, AND LEAVES THE STORE
/// ONE WORKER LEAVES. Every month holds two later days when the first is
/// offered, so the append door refuses each file and each lands as a new
/// generation of its month. The versioned repair flushes a revision's bars
/// before the receipt that publishes them and before their promotion; run on
/// a worker inside the day's batch, those flushes were left to the batch,
/// past both. So the day's batch may be handed no file and no directory of a
/// revision, and the store (bars, revisions, receipts, the repair journal,
/// the census, the journal, every ledger) is byte for byte the one a single
/// worker flushing each write leaves, the repair journal's lines in listing
/// order.
#[test]
fn a_back_fill_is_published_outside_the_days_batch_and_leaves_one_workers_store() {
    let root = scratch("wp16-back-fill");
    let plan: Vec<DayPlan> = [d1(), d2(), d3()]
        .iter()
        .map(|&d| (d, Tree::Options, many_options(d, 6)))
        .collect();
    world_of(&root, &plan);
    let (kind_of_source, path) = source_path(&root, Src::Zips, ImportKind::Options);
    let origin = Origin {
        started: 1_790_000_000,
        ..Origin::measure(kind_of_source, &path, Some(COMMIT))
    };
    let of_a_revision = |path: &Path| {
        path.components()
            .any(|part| part.as_os_str() == "bar-revisions-v1")
    };
    let runs = [
        (1, Durability::EachWrite),
        (8, Durability::Day(store::file::BatchFlush::OncePerDevice)),
        (4, Durability::Day(store::file::BatchFlush::EveryFile)),
    ];
    let done: Vec<(Report, BTreeMap<PathBuf, Vec<u8>>)> = runs
        .iter()
        .map(|&(workers, durability)| {
            let word = durability.as_str();
            let store = root.join(word);
            std::fs::create_dir_all(&store).unwrap();
            let later = run_durable(
                &root,
                (Src::Zips, ImportKind::Options),
                (d2(), d3()),
                &store,
                (workers, &origin, durability),
            )
            .unwrap();
            assert_eq!(
                (later.days_imported, later.months_repaired),
                (2, 0),
                "{word}"
            );
            let flushed: std::sync::Arc<std::sync::Mutex<Vec<PathBuf>>> = std::sync::Arc::default();
            let hook = {
                let flushed = std::sync::Arc::clone(&flushed);
                HookedFlushes::new(
                    &store,
                    Box::new(move |path| {
                        flushed.lock().unwrap().push(path.to_path_buf());
                        Ok(())
                    }),
                )
            };
            let report = run_durable(
                &root,
                (Src::Zips, ImportKind::Options),
                (d1(), d3()),
                &store,
                (workers, &origin, durability),
            )
            .unwrap();
            drop(hook);
            let held_back: Vec<PathBuf> = flushed
                .lock()
                .unwrap()
                .iter()
                .filter(|path| of_a_revision(path))
                .cloned()
                .collect();
            assert!(
                held_back.is_empty(),
                "{word}: the day's batch was left a revision's flushes: {held_back:?}"
            );
            (report, store_bytes(&store))
        })
        .collect();
    let (want_report, want) = &done[0];
    assert_eq!(want_report.days_imported, 1);
    assert!(want_report.months_repaired > 1, "{want_report:?}");
    let journal = String::from_utf8_lossy(
        want.get(Path::new("audit/gdfl.repairs"))
            .expect("the repair journal"),
    )
    .into_owned();
    let repaired = |verb: &str| {
        journal
            .lines()
            .filter(|line| line.split(' ').next() == Some(verb))
            .count()
    };
    assert_eq!(
        (
            repaired("repairing"),
            repaired("repair"),
            repaired("refused")
        ),
        (want_report.months_repaired, want_report.months_repaired, 0),
        "{journal}"
    );
    assert!(
        want.keys().any(|rel| of_a_revision(rel)),
        "a revision is published"
    );
    for ((report, have), (_, durability)) in done.iter().zip(runs).skip(1) {
        let word = durability.as_str();
        assert_eq!(report, want_report, "{word}: the same report");
        assert_eq!(
            have.keys().collect::<Vec<_>>(),
            want.keys().collect::<Vec<_>>(),
            "{word}: the same files"
        );
        for (rel, bytes) in want {
            assert!(
                have.get(rel) == Some(bytes),
                "{word}: {} differs",
                rel.display()
            );
        }
    }
    std::fs::remove_dir_all(root).unwrap();
}

// ── DB-N26, WP-12d: a day inside what its month holds ──────────────────────

#[path = "gdfl_backfill_tests.rs"]
mod backfill;

/// The scope a close records and the days a wider scope reads again
/// (WP-12f, D-4878), over this module's invented worlds.
#[path = "gdfl_scope_tests.rs"]
mod scope_reopen;

// ── WP-11c (DB-A04, DB-A07): a halt's seconds, the flat run, the sampled day ─

#[path = "gdfl_wp11c_tests.rs"]
mod wp11c;
