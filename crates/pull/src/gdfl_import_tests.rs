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

#[test]
fn an_option_second_carries_the_last_open_interest_of_its_traded_rows() {
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
        ],
    )
    .unwrap();
    assert_eq!(
        got.seconds[0].open_interest, 0,
        "zero is zero, the last traded row's"
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
            lines.iter().any(|l| l
                .starts_with(&format!("done options {d} * definition=2 files=3 "))
                && !l.contains(" stage=")
                && !l.contains(" vendor_refused=")),
            "{journal}"
        );
    }
    let last = lines.last().unwrap();
    assert!(
        last.starts_with("incomplete options 2024-04-04 * definition=2 files=3 ")
            && last.contains(" failures=1 ")
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
        "refused indices 2024-04-01 * definition=2 stage=vendor reason=index_ends_in_session \
         failures=2",
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

/// DB-025: a special session and a day the source does not hold are holes
/// with a reason in the journal, each closed `refused`; a re-run that meets
/// them again writes nothing.
#[test]
fn a_calendar_refused_day_and_a_missing_day_each_leave_a_refused_line() {
    let root = scratch("import-holes");
    world(&root, &[d1()]);
    let store = root.join("S");
    let special = import(
        &root,
        Src::Store,
        ImportKind::Indices,
        day(2024, 3, 2),
        day(2024, 3, 2),
        &store,
    );
    assert_eq!(special.days_refused, 1);
    assert_eq!(
        without_run(&line_of(&store, "indices", "2024-03-02")),
        "refused indices 2024-03-02 * definition=2 stage=calendar reason=calendar_not_regular \
         failures=1"
    );
    let missing = import(&root, Src::Store, ImportKind::Stocks, d1(), d2(), &store);
    assert_eq!(missing.days_missing, vec![d2()]);
    assert!(
        missing.failures.is_empty(),
        "a missing day names no failure"
    );
    assert_eq!(
        without_run(&line_of(&store, "stocks", "2024-04-02")),
        "refused stocks 2024-04-02 * definition=2 stage=vendor reason=day_not_held failures=0"
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
        day(2024, 3, 2),
        day(2024, 3, 2),
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
        "refused stocks 2011-01-03 * definition=2 stage=calendar reason=calendar_unmeasured \
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
        "done options 2024-04-01 * definition=2 files=3 seconds={} failures=1 late={} \
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
    let stats = "definition=2 files=2 seconds=9 failures=1 late=3 late_unresolved=0 max_back_s=4";
    assert_eq!(
        closing_line("k 1 *", Close::Done, &counts),
        format!("done k 1 * {stats}")
    );
    assert_eq!(
        closing_line("k 1 *", Close::WithRefusals("price_refused"), &counts),
        format!("done k 1 * {stats} vendor_refused=1 reason=price_refused")
    );
    assert_eq!(
        closing_line("k 1 *", Close::Incomplete(copy.cause), &counts),
        format!("incomplete k 1 * {stats} stage=vendor reason=source_crc_mismatch")
    );
    assert_eq!(
        closing_line("k 1 *", Close::Refused(calendar.cause), &counts),
        "refused k 1 * definition=2 stage=calendar reason=expiry_passed failures=1"
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
    let mut origin = Origin::measure(SourceKind::TickStore, &ts, Some("0123abcd"));
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
    };
    run_cm_from(&TickStore::new(&ts), &run, options).unwrap();
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines[0],
        format!(
            "run 1750000000 source=tickstore archive={} archive_bytes={} commit=0123abcd \
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
#[test]
fn a_file_or_copy_refusal_journals_its_stage_and_leaves_the_day_incomplete() {
    let root = scratch("import-stages");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &store);
    let late = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
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
    // The disaster-recovery Saturday is a special session, never a regular day.
    let special = import(
        &root,
        Src::Zips,
        ImportKind::Indices,
        day(2024, 3, 2),
        day(2024, 3, 2),
        &store,
    );
    assert_eq!(special.days_refused, 1);
    assert!(special.failures[0].instrument.ends_with("calendar"));
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

#[test]
fn a_day_older_than_what_the_month_holds_is_refused_by_the_store() {
    let root = scratch("import-order");
    world(&root, &[d1(), d2()]);
    let store = root.join("S");
    import(&root, Src::Store, ImportKind::Indices, d2(), d2(), &store);
    let late = import(&root, Src::Store, ImportKind::Indices, d1(), d1(), &store);
    assert!(!late.failures.is_empty(), "never inserted behind the tail");
    assert_eq!(late.seconds_committed, 0);
    let text = std::fs::read_to_string(journal_path(&store)).unwrap();
    assert!(text.contains("incomplete indices 2024-04-01"), "{text}");
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
        last.starts_with("incomplete indices 2024-04-01 * definition=2 files=2 ")
            && last.contains(" stage=census reason=census_refused run="),
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
