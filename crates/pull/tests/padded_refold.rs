//! A DERIVED MONTH STORED UNDER THE FOLD BEFORE D-7270, MET BY EVERY DOOR
//! (R4-27 repair 1, verify-R4-27-r1 B1 and B2).
//!
//! D-7270 changed how a Zerodha cash share's derived rungs fold a
//! vendor-padded minute: a bucket opens on its first unpadded minute and a
//! bucket of only padded minutes has no bar. Every derived month stored
//! before it was folded with a padded minute read as a trade
//! (`pull::fold::PADDED_FOLD_BEFORE`). These drive `pull::ingest::from_window`
//! and `pull::ingest::repair_window` over a scratch store whose 5min month was
//! seeded with that old fold, and pin what each door says:
//!
//! - the append door refuses the rung by the rule change's own name, never
//!   as missing source (B2), on a re-ingest of the same day and on the next
//!   day of the month, while the minutes land;
//! - a month seeded under the rule in force takes the next day, and a
//!   disagreement that is not the rule change keeps its own name;
//! - the repair door refuses the same month by the same name instead of
//!   reporting it repaired (B1), writes no generation, and refuses a held
//!   bar its fold would drop by name, which it used to keep and report clean;
//! - a held bar the repair's fold changes, not by the padded rule, stays the
//!   append door's own refusal (IA-16), never reported repaired and never
//!   named the rule change.
//!
//! Prices are invented and cannot be mistaken for a real share's.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fs;
use std::path::{Path, PathBuf};

use pull::fetch::{RawRow, RawWindow};
use pull::fold::{Bucket, PaddedFold, derive_minutes_with};
use pull::session::{Day, IST_OFFSET_SECS, Window};
use store::format::Bar;
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("brutex-PADREFOLD-{tag}-{}", std::process::id()));
        let _best_effort = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

const SYMBOL: &str = "PADX";
const NAME: &str = "stored under the fold before D-7270";
const WRONG_REMEDY: &str = "restore complete minute source";

fn stamp(day: Day, minute: i64) -> i64 {
    i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60
}

/// An invented traded candle at `minute` past IST midnight on `day`.
fn traded(day: Day, minute: i64) -> RawRow {
    let base = 2_000_000 + (minute % 89) * 7 + i64::from(day.day()) * 3;
    RawRow {
        timestamp: stamp(day, minute),
        open: base,
        high: base + 30,
        low: base - 20,
        close: base + 5,
        volume: 50 + minute,
        open_interest: None,
    }
}

/// A vendor-padded minute: flat at the close of the candle before, volume
/// zero (D-7270's definition).
fn padded(day: Day, minute: i64, before: &RawRow) -> RawRow {
    RawRow {
        timestamp: stamp(day, minute),
        open: before.close,
        high: before.close,
        low: before.close,
        close: before.close,
        volume: 0,
        open_interest: None,
    }
}

/// A regular cash session on `day`, 09:15 to 15:29: the minutes in `pads`
/// padded, the minutes in `silent` absent (no trade and no candle).
fn session(day: Day, pads: &[i64], silent: &[i64]) -> Vec<RawRow> {
    let mut rows: Vec<RawRow> = Vec::new();
    for minute in 555..930 {
        if silent.contains(&minute) {
            continue;
        }
        let row = match rows.last() {
            Some(before) if pads.contains(&minute) => padded(day, minute, before),
            _ => traded(day, minute),
        };
        rows.push(row);
    }
    rows
}

fn request(from: Day, to: Day) -> pull::fetch::BarRequest {
    pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Equity,
        window: Window::new(from, to).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    }
}

fn plan(request: &pull::fetch::BarRequest) -> pull::ingest::Plan<'_> {
    pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        contract: None,
    }
}

fn land(root: &Path, from: Day, to: Day, rows: &[RawRow]) -> pull::ingest::Ingested {
    let req = request(from, to);
    pull::ingest::from_window(
        &RawWindow {
            rows: rows.to_vec(),
        },
        SYMBOL,
        "ZERODHA TEST",
        root,
        plan(&req),
    )
}

fn repair(root: &Path, from: Day, to: Day, rows: &[RawRow]) -> pull::ingest::Ingested {
    let req = request(from, to);
    pull::ingest::repair_window(
        &RawWindow {
            rows: rows.to_vec(),
        },
        SYMBOL,
        "ZERODHA TEST",
        root,
        plan(&req),
        "R4-27 repair 1 fixture: refold a month stored under the fold before D-7270",
    )
}

fn symbol_id() -> u32 {
    u32::try_from(brutex_core::universe::fnv1a(SYMBOL) & u64::from(u32::MAX)).unwrap()
}

fn path(tf: Timeframe) -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: SYMBOL,
        contract: None,
        timeframe: tf,
        month: YearMonth::new(2025, 7).unwrap(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Every bar of the July 2025 month at `tf`, as every reader opens it.
fn bars(root: &Path, tf: Timeframe) -> Vec<Bar> {
    let file = store::file::BarFile::open_existing(root, path(tf), symbol_id()).unwrap();
    (0..file.header().n_valid)
        .map(|i| file.read_record(i).unwrap())
        .collect()
}

fn bytes(root: &Path, tf: Timeframe) -> Vec<u8> {
    fs::read(path(tf).to_path_buf(root)).unwrap()
}

/// Seeds the 5min month with `held`, as a derive of an earlier build wrote it.
fn seed_five(root: &Path, held: &[Bar]) {
    let mut file =
        store::file::BarFile::open_or_create(root, path(Timeframe::MINUTE_5), symbol_id()).unwrap();
    file.append(held).unwrap();
}

/// The minutes as the landing commits them, read back from a store of their own.
fn committed(rows: &[RawRow], from: Day, to: Day, tag: &str) -> Vec<Bar> {
    let clean = Scratch::new(tag);
    let done = land(&clean.0, from, to, rows);
    assert!(done.bars_committed > 0, "{:?}", done.failures);
    bars(&clean.0, Timeframe::MINUTE_1)
}

fn five(minutes: &[Bar], rule: PaddedFold) -> Vec<Bar> {
    derive_minutes_with(
        minutes,
        Bucket::of_secs(300).unwrap(),
        pull::vendor::Venue::NseCash,
        None,
        pull::calendar::Runtime::default(),
        rule,
    )
    .unwrap()
    .bars
}

/// The receipt's failures of the 5min rung.
fn five_failures(done: &pull::ingest::Ingested) -> Vec<String> {
    done.failures
        .iter()
        .filter(|f| f.why.starts_with("5min: "))
        .map(|f| f.why.clone())
        .collect()
}

/// The repair journal's `refused` lines of the 5min month
/// (`audit/zerodha.repairs`); none when the journal was never written.
fn refused_in_journal(root: &Path) -> Vec<String> {
    fs::read_to_string(root.join("audit").join("zerodha.repairs"))
        .unwrap_or_default()
        .lines()
        .filter(|line| line.starts_with("refused NSE-CASH-PADX 5min 2025-07 v=1 why="))
        .map(str::to_owned)
        .collect()
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// 09:20 to 09:24 padded: the 09:20 bucket holds only padded minutes, which
/// the fold before D-7270 wrote as a flat bar and the rule in force drops.
const DROPPED: [i64; 5] = [560, 561, 562, 563, 564];
/// 09:25 padded: the 09:25 bucket opens on a padded minute, so the two rules
/// give it different values.
const MOVED: [i64; 1] = [565];

/// **B2: THE APPEND DOOR NAMES THE RULE CHANGE, NOT MISSING SOURCE.** A 5min
/// month stored under the fold before D-7270, ingested again (the same day,
/// and the next day of the month), refuses that rung as "stored under the
/// fold before D-7270" and names the store step that owes the refold; it
/// used to say the minute source was incomplete. The minutes land; the held
/// bytes are kept; the month takes no day of the other rule.
#[test]
fn a_month_stored_under_the_fold_before_is_refused_by_that_name_at_the_append_door() {
    let (one, two) = (day(2025, 7, 1), day(2025, 7, 2));
    for (tag, pads, what) in [
        ("DROPPED", &DROPPED[..], "has no bar at it"),
        ("MOVED", &MOVED[..], "folds it differently"),
    ] {
        let first = session(one, pads, &[]);
        let minutes = committed(&first, one, one, &format!("{tag}-CLEAN"));
        assert_eq!(minutes.len(), 375);
        let old = five(&minutes, PaddedFold::AsTrade);
        assert_ne!(
            old,
            five(&minutes, PaddedFold::Absent),
            "{tag}: the rule changes it"
        );

        // The same day again.
        let again = Scratch::new(&format!("{tag}-AGAIN"));
        seed_five(&again.0, &old);
        let held = bytes(&again.0, Timeframe::MINUTE_5);
        let done = land(&again.0, one, one, &first);
        assert_eq!(done.bars_committed, 375, "{tag}: the minutes land");
        assert_eq!(done.derived_files, 6, "{tag}: every rung but 5min");
        let refused = five_failures(&done);
        assert_eq!(refused.len(), 1, "{tag}: {refused:?}");
        assert!(refused[0].contains(NAME), "{tag}: {}", refused[0]);
        assert!(refused[0].contains(what), "{tag}: {}", refused[0]);
        assert!(refused[0].contains("S-47"), "{tag}: {}", refused[0]);
        assert!(
            refused[0].contains("the append door takes no new day"),
            "{tag}: {}",
            refused[0]
        );
        assert!(!refused[0].contains(WRONG_REMEDY), "{tag}: {}", refused[0]);
        assert_eq!(
            held,
            bytes(&again.0, Timeframe::MINUTE_5),
            "{tag}: bytes preserved"
        );

        // The next day of the month in flight.
        let next = Scratch::new(&format!("{tag}-NEXT"));
        seed_five(&next.0, &old);
        let mut both = first.clone();
        both.extend(session(two, &[600, 601], &[]));
        let done = land(&next.0, one, two, &both);
        assert_eq!(done.bars_committed, 750, "{tag}: both days of minutes land");
        let refused = five_failures(&done);
        assert_eq!(refused.len(), 1, "{tag}: {refused:?}");
        assert!(refused[0].contains(NAME), "{tag}: {}", refused[0]);
        assert!(!refused[0].contains(WRONG_REMEDY), "{tag}: {}", refused[0]);
        assert_eq!(
            bars(&next.0, Timeframe::MINUTE_5),
            old,
            "{tag}: no bar of the other rule"
        );
    }
}

/// **THE CONTROL, AND A DISAGREEMENT THAT IS NOT THE RULE CHANGE.** Seeded
/// under the rule in force, the next day appends at every rung. Seeded with a
/// bar neither rule folds, the refusal keeps its own words and never claims
/// the rule change.
#[test]
fn a_month_under_the_rule_in_force_appends_and_another_disagreement_keeps_its_name() {
    let (one, two) = (day(2025, 7, 1), day(2025, 7, 2));
    let first = session(one, &DROPPED, &[]);
    let minutes = committed(&first, one, one, "CONTROL-CLEAN");
    let new = five(&minutes, PaddedFold::Absent);

    let control = Scratch::new("CONTROL");
    seed_five(&control.0, &new);
    let mut both = first.clone();
    both.extend(session(two, &[600, 601], &[]));
    let done = land(&control.0, one, two, &both);
    assert!(five_failures(&done).is_empty(), "{:?}", done.failures);
    assert_eq!(done.derived_files, 7);
    let held = bars(&control.0, Timeframe::MINUTE_5);
    let day_two = stamp(two, 0) * 1_000_000;
    assert_eq!(held.iter().filter(|b| b.ts_micros >= day_two).count(), 75);

    let mut tampered = new.clone();
    tampered[3].close += 1;
    let conflict = Scratch::new("CONFLICT");
    seed_five(&conflict.0, &tampered);
    let done = land(&conflict.0, one, one, &first);
    let refused = five_failures(&done);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].contains("historical derived conflict at"),
        "{}",
        refused[0]
    );
    assert!(!refused[0].contains("D-7270"), "{}", refused[0]);
}

/// **B1: THE REPAIR DOOR DOES NOT REFOLD, AND SAYS SO.** `zerodha-repair
/// --from-store`, which an earlier draft of S-47 named as the refold's door,
/// lands through `repair_window`. Over a 5min month stored under the fold
/// before D-7270 it reported every rung repaired and changed nothing when the
/// rule drops a bar, and refused in a repair's generic words when the rule
/// moves one. Now both are refused by the rule change's name, and no
/// generation is written.
#[test]
fn the_repair_door_refuses_a_month_stored_under_the_fold_before_and_never_reports_it_repaired() {
    let one = day(2025, 7, 1);
    for (tag, pads) in [("DROPPED", &DROPPED[..]), ("MOVED", &MOVED[..])] {
        let rows = session(one, pads, &[]);
        let minutes = committed(&rows, one, one, &format!("R-{tag}-CLEAN"));
        let old = five(&minutes, PaddedFold::AsTrade);
        let scratch = Scratch::new(&format!("R-{tag}"));
        seed_five(&scratch.0, &old);
        let _landed = land(&scratch.0, one, one, &rows);
        let held = bytes(&scratch.0, Timeframe::MINUTE_5);

        let done = repair(&scratch.0, one, one, &rows);
        let refused = five_failures(&done);
        assert_eq!(refused.len(), 1, "{tag}: {:?}", done.failures);
        assert!(refused[0].contains(NAME), "{tag}: {}", refused[0]);
        assert!(
            refused[0].contains("the repair door never changes or drops a held bar"),
            "{tag}: {}",
            refused[0]
        );
        assert!(!refused[0].contains(WRONG_REMEDY), "{tag}: {}", refused[0]);
        assert_eq!(
            held,
            bytes(&scratch.0, Timeframe::MINUTE_5),
            "{tag}: bytes preserved"
        );
        let served = store::generation::served(&scratch.0, path(Timeframe::MINUTE_5)).unwrap();
        assert_eq!(served.ordinal(), 0, "{tag}: no generation written");
        assert_eq!(bars(&scratch.0, Timeframe::MINUTE_5), old, "{tag}");
        // JOURNALED AS THE DOOR JOURNALS ITS OWN REFUSALS: one `refused`
        // line with the run's reason, which a refusal asked before the door
        // used to leave out of the journal.
        let journal = refused_in_journal(&scratch.0);
        assert_eq!(journal.len(), 1, "{tag}: {journal:?}");
        assert!(
            journal[0].contains("stored_under_the_fold_before_D-7270"),
            "{tag}: {}",
            journal[0]
        );
        assert!(
            journal[0].contains("reason=R4-27_repair_1_fixture"),
            "{tag}: {}",
            journal[0]
        );
    }
}

/// **A HELD BAR THE REPAIR'S FOLD DROPS IS REFUSED BY NAME (D-3350).** Not the
/// padded rule: a 5min bar held where the source month has no minute at all
/// (10:00 to 10:04 silent). The append door keeps its own words; the repair
/// door, which keeps every held bar, used to answer "nothing new" and report
/// the rung repaired. It refuses it now.
#[test]
fn the_repair_door_refuses_a_held_bar_its_fold_drops() {
    let one = day(2025, 7, 1);
    let silent = [600, 601, 602, 603, 604];
    let rows = session(one, &[], &silent);
    let minutes = committed(&rows, one, one, "DROP-CLEAN");
    assert_eq!(minutes.len(), 370);
    let fold = five(&minutes, PaddedFold::Absent);
    assert_eq!(
        fold,
        five(&minutes, PaddedFold::AsTrade),
        "no padded minute"
    );
    let ten = stamp(one, 600) * 1_000_000;
    let at = fold.iter().position(|b| b.ts_micros > ten).unwrap();
    let mut held = fold.clone();
    held.insert(
        at,
        Bar {
            ts_micros: ten,
            ..fold[at - 1]
        },
    );
    let scratch = Scratch::new("DROP");
    seed_five(&scratch.0, &held);
    let landed = land(&scratch.0, one, one, &rows);
    let refused = five_failures(&landed);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].contains(&format!("historical derived evidence incomplete at {ten}")),
        "{}",
        refused[0]
    );
    assert!(!refused[0].contains("D-7270"), "{}", refused[0]);

    let before = bytes(&scratch.0, Timeframe::MINUTE_5);
    let done = repair(&scratch.0, one, one, &rows);
    let refused = five_failures(&done);
    assert_eq!(refused.len(), 1, "{:?}", done.failures);
    assert!(
        refused[0].contains(&format!(
            "historical derived bar at {ten} has no bar in the fold of the source month as now served"
        )),
        "{}",
        refused[0]
    );
    assert!(
        refused[0].contains("never drops a held bar"),
        "{}",
        refused[0]
    );
    assert_eq!(before, bytes(&scratch.0, Timeframe::MINUTE_5));
    let journal = refused_in_journal(&scratch.0);
    assert_eq!(journal.len(), 1, "{journal:?}");
    assert!(
        journal[0].contains(&format!(
            "why=historical_derived_bar_at_{ten}_has_no_bar_in_the_fold"
        )),
        "{}",
        journal[0]
    );
    let served = store::generation::served(&scratch.0, path(Timeframe::MINUTE_5)).unwrap();
    assert_eq!(served.ordinal(), 0);
}

/// **A HELD BAR THE REPAIR'S FOLD CHANGES, NOT BY THE PADDED RULE, STAYS THE
/// APPEND DOOR'S OWN REFUSAL (D-3350, IA-16).** A 5min bar held with a close
/// no fold of the source gives, in a month with no padded minute: the check
/// asked before the door hands it on, and the door's append refuses the
/// restated record at its stamp. The rung is never reported repaired (the
/// verifier's follow-up on the builder's head: the repair door "reports
/// clean"), never named the rule change, keeps its bytes and writes no
/// generation. As IA-16 left it, that refusal is filed and logged
/// `pull.derive` and is not journaled: only a refusal of the versioned
/// repair, and R4-27's refusals asked before the door, are.
#[test]
fn the_repair_door_refuses_a_held_bar_its_fold_changes_in_the_append_doors_words() {
    let one = day(2025, 7, 1);
    let rows = session(one, &[], &[]);
    let minutes = committed(&rows, one, one, "CHANGE-CLEAN");
    assert_eq!(minutes.len(), 375);
    let fold = five(&minutes, PaddedFold::Absent);
    assert_eq!(
        fold,
        five(&minutes, PaddedFold::AsTrade),
        "no padded minute"
    );
    let mut held = fold.clone();
    held[3].close += 1;
    let restated = held[3].ts_micros;
    let scratch = Scratch::new("CHANGE");
    seed_five(&scratch.0, &held);
    let _landed = land(&scratch.0, one, one, &rows);
    let before = bytes(&scratch.0, Timeframe::MINUTE_5);

    let done = repair(&scratch.0, one, one, &rows);
    let refused = five_failures(&done);
    assert_eq!(refused.len(), 1, "{:?}", done.failures);
    assert!(
        refused[0].contains(&format!("restates the record held at {restated}")),
        "{}",
        refused[0]
    );
    assert!(!refused[0].contains("D-7270"), "{}", refused[0]);
    assert!(
        !refused[0].contains("has no bar in the fold"),
        "{}",
        refused[0]
    );
    assert_eq!(before, bytes(&scratch.0, Timeframe::MINUTE_5));
    assert_eq!(bars(&scratch.0, Timeframe::MINUTE_5), held);
    let served = store::generation::served(&scratch.0, path(Timeframe::MINUTE_5)).unwrap();
    assert_eq!(served.ordinal(), 0, "no generation written");
    let journal = refused_in_journal(&scratch.0);
    assert!(journal.is_empty(), "{journal:?}");
}

/// A copy of `bars` with every open interest written `0`, as an earlier build
/// wrote a share's derived bars.
fn zero_interest(bars: &[Bar]) -> Vec<Bar> {
    bars.iter()
        .map(|bar| Bar {
            open_interest: 0,
            ..*bar
        })
        .collect()
}

/// **AN OPEN-INTEREST ABSENCE WRITTEN `0` NEVER HIDES THE RULE CHANGE**
/// (R4-27 repair 1, measured first on a private copy of a real share-month,
/// SIEMENS 2019-03). An earlier build wrote a share's derived bars with open
/// interest `0`; this build holds the minutes' open interest null, so the
/// refold of the same minutes differs from every held bar in that field
/// alone, which the repair door reads as one absence (D-3322). Compared byte
/// for byte, the rule change went unnamed at both doors: the append door
/// called it a conflict and the repair door's append refused it as a
/// restated record. Both name it now, bytes kept, no generation written; and
/// a month that differs from the fold in force in that absence alone is never
/// named the rule change.
#[test]
fn an_open_interest_absence_written_zero_never_hides_the_rule_change() {
    let one = day(2025, 7, 1);
    let rows = session(one, &MOVED, &[]);
    let minutes = committed(&rows, one, one, "OI0-CLEAN");
    assert!(
        minutes
            .iter()
            .all(|m| m.open_interest == store::format::OI_NULL),
        "this build holds a share's open interest null"
    );
    let old = zero_interest(&five(&minutes, PaddedFold::AsTrade));

    // The append door, the same day again.
    let again = Scratch::new("OI0-AGAIN");
    seed_five(&again.0, &old);
    let held = bytes(&again.0, Timeframe::MINUTE_5);
    let done = land(&again.0, one, one, &rows);
    let refused = five_failures(&done);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(refused[0].contains(NAME), "{}", refused[0]);
    assert!(
        !refused[0].contains("historical derived conflict"),
        "{}",
        refused[0]
    );
    assert_eq!(
        held,
        bytes(&again.0, Timeframe::MINUTE_5),
        "bytes preserved"
    );

    // The repair door.
    let repaired = Scratch::new("OI0-REPAIR");
    seed_five(&repaired.0, &old);
    let _landed = land(&repaired.0, one, one, &rows);
    let held = bytes(&repaired.0, Timeframe::MINUTE_5);
    let done = repair(&repaired.0, one, one, &rows);
    let refused = five_failures(&done);
    assert_eq!(refused.len(), 1, "{:?}", done.failures);
    assert!(refused[0].contains(NAME), "{}", refused[0]);
    assert!(
        refused[0].contains("the repair door never changes or drops a held bar"),
        "{}",
        refused[0]
    );
    assert_eq!(
        held,
        bytes(&repaired.0, Timeframe::MINUTE_5),
        "bytes preserved"
    );
    let served = store::generation::served(&repaired.0, path(Timeframe::MINUTE_5)).unwrap();
    assert_eq!(served.ordinal(), 0, "no generation written");

    // The absence alone is not the rule change: the append door keeps its
    // byte-for-byte conflict, and neither door names D-7270.
    let new = zero_interest(&five(&minutes, PaddedFold::Absent));
    let only = Scratch::new("OI0-ONLY");
    seed_five(&only.0, &new);
    let held = bytes(&only.0, Timeframe::MINUTE_5);
    let landed = land(&only.0, one, one, &rows);
    let refused = five_failures(&landed);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert!(
        refused[0].contains("historical derived conflict at"),
        "{}",
        refused[0]
    );
    assert!(!refused[0].contains("D-7270"), "{}", refused[0]);
    let done = repair(&only.0, one, one, &rows);
    for why in five_failures(&done) {
        assert!(!why.contains("D-7270"), "{why}");
    }
    assert_eq!(held, bytes(&only.0, Timeframe::MINUTE_5), "bytes preserved");
    let served = store::generation::served(&only.0, path(Timeframe::MINUTE_5)).unwrap();
    assert_eq!(served.ordinal(), 0, "no generation written");
}
