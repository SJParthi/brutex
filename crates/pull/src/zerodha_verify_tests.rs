#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail; and one test per test id, \
              each holding every arm of its row."
)]

//! IA-ZV-01..11, IA-VD-08 and IA-VD-15, the matrix rows of §8 Z the
//! verifier owns (Z1, Z4, Z5, Z15), and the extreme cases of the spool and
//! its merge (IA-9). Every bar and body here is invented: the instruments are
//! `ZZIDX` and `ZZSHARE`, and the prices sit near 11,111.00, which no real
//! print of either names (CLAUDE.md, no vendor row in the repository).

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use brutex_core::instrument::{Exchange, Segment};
use store::format::{Bar, OI_NULL};
use store::path::{StorePath, Timeframe, YearMonth};
use store::verdict::{self, Code, Found, Mode, flags};

use super::spool::{self, LogRecord, LogState, SpoolId, SpoolRecord, SpoolWriter};
use super::*;
use crate::fetch::{RawRow, RowCause};
use crate::session::Day;

// ------------------------------------------------------------- fixtures --

fn scratch(tag: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    let root = std::env::temp_dir().join(format!(
        "brutex-ia9-{tag}-{}-{}-{nanos}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

/// A regular full session of record (Tuesday 23 April 2024).
fn regular() -> Day {
    day(2024, 4, 23)
}

fn index() -> Series {
    Series {
        exchange: Exchange::Nse,
        segment: Segment::Index,
        symbol: "ZZIDX".to_owned(),
        contract: None,
    }
}

fn share() -> Series {
    Series {
        exchange: Exchange::Nse,
        segment: Segment::Cash,
        symbol: "ZZSHARE".to_owned(),
        contract: None,
    }
}

fn utc_s(on: Day, minute: u32) -> i64 {
    i64::from(on.days_from_epoch()) * 86_400 - 19_800 + i64::from(minute) * 60
}

fn bar_at(on: Day, minute: u32, price: i64) -> Bar {
    Bar {
        ts_micros: utc_s(on, minute) * 1_000_000,
        open: price,
        high: price + 50,
        low: price - 50,
        close: price + 10,
        volume: 100 + i64::from(minute),
        open_interest: OI_NULL,
    }
}

fn row_of(bar: &Bar) -> RawRow {
    RawRow {
        timestamp: bar.ts_micros / 1_000_000,
        open: bar.open,
        high: bar.high,
        low: bar.low,
        close: bar.close,
        volume: bar.volume,
        open_interest: (bar.open_interest != OI_NULL).then_some(bar.open_interest),
    }
}

/// Every minute 09:15..15:29 of `on`, less `skip`.
fn full_day(on: Day, skip: &[u32]) -> Vec<Bar> {
    (555..930)
        .filter(|m| !skip.contains(m))
        .map(|m| bar_at(on, m, 1_111_100 + i64::from(m)))
        .collect()
}

fn whole_day(on: Day) -> (i64, i64) {
    (utc_s(on, 0), utc_s(on, 0) + 86_399)
}

fn write_month(root: &Path, series: &Series, tf: Timeframe, bars: &[Bar]) {
    let first = bars.first().expect("bars");
    let (number, _) = ist_day_minute(first.ts_micros / 1_000_000);
    let on = Day::from_days(u32::try_from(number).unwrap()).unwrap();
    let month = on.year_month().unwrap();
    let path = StorePath::new(series.parts(tf, month)).unwrap();
    let mut file =
        BarFile::open_or_create(root, path, repair_plan::symbol_id(&series.symbol)).unwrap();
    file.append(bars).unwrap();
}

fn write_ledger(root: &Path, text: &str) {
    std::fs::create_dir_all(root.join("audit")).unwrap();
    std::fs::write(root.join("audit").join("zerodha.gaps"), text).unwrap();
}

/// The answer the oracle makes of `bars` asked over the whole of `on`.
fn answer_of(on: Day, bars: &[Bar], receipt: u32) -> DayAnswer {
    let rows: Vec<RawRow> = bars.iter().map(row_of).collect();
    let mut made = oracle(
        &rows,
        TimestampEncoding::IsoDateTimeOffset,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(on),
    );
    let mut answer = made.days.remove(&on.days_from_epoch()).unwrap_or_default();
    answer.receipt = receipt;
    answer.asked = whole_day(on);
    answer.kept = true;
    answer.last_stamp = rows.last().map_or(0, |r| r.timestamp);
    answer
}

fn minute_record(series: &Series, on: Day, inputs: &DayInputs<'_>) -> SpoolRecord {
    verify_day(series, on, inputs)
        .into_iter()
        .find(|r| r.timeframe == Timeframe::MINUTE_1)
        .expect("a 1min record")
        .record
}

fn reason_word(record: &SpoolRecord) -> &'static str {
    words::reason_word(record.facts.reason).unwrap_or("")
}

fn the_record(root: &Path, segment: Segment, tf: Timeframe, on: Day, key: &str) -> verdict::Record {
    let series = verdict::Series {
        root,
        feed: Vendor::Zerodha,
        exchange: Exchange::Nse,
        segment,
        timeframe: tf,
    };
    let looked = verdict::Reader::new()
        .lookup(
            &series,
            store_day(on).unwrap(),
            &Key::instrument(key).unwrap(),
        )
        .unwrap();
    match looked.answer.expect("a generation").found {
        Found::Record { record, .. } => record,
        Found::NoRecord(none) => panic!("no record: {none:?}"),
    }
}

fn spooled(on: Day, key: &str, code: Code, receipt: u32) -> SpoolRecord {
    SpoolRecord {
        exchange: Exchange::Nse,
        day: on,
        key: Key::instrument(key).unwrap(),
        facts: Facts::of(code),
        receipt,
        details: Vec::new(),
    }
}

fn year_generation(root: &Path, segment: Segment, tf: Timeframe, on: Day) -> u32 {
    let series = verdict::Series {
        root,
        feed: Vendor::Zerodha,
        exchange: Exchange::Nse,
        segment,
        timeframe: tf,
    };
    committed_generation(&series, store_day(on).unwrap()).unwrap()
}

#[test]
fn the_fixture_days_are_what_the_calendar_of_record_says() {
    assert!(matches!(
        calendar::exchange_kind_of(i64::from(regular().days_from_epoch())),
        DayKind::Open(session) if session == calendar::Session::full()
    ));
    assert!(matches!(
        calendar::exchange_kind_of(i64::from(day(2024, 4, 21).days_from_epoch())),
        DayKind::Closed
    ));
    let Owes::Windows(windows) = repair_plan::owed("INDEX", regular().days_from_epoch()) else {
        panic!("a regular day owes windows");
    };
    assert_eq!(windows.1, 1);
    assert_eq!(windows.0[0], (555, 930), "375 minutes, 09:15..15:30");
}

// ------------------------------------------------------------- IA-ZV-01 --

/// IA-ZV-01: a minute the store holds and the answer lacks is `extra`.
#[test]
fn a_minute_the_store_holds_and_the_answer_lacks_is_extra() {
    let on = regular();
    let stored = full_day(on, &[]);
    let answered = full_day(on, &[600]);
    let answers = [answer_of(on, &answered, 1)];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::ExtraInStore);
    assert_eq!(record.facts.extra, 1);
    assert_eq!(record.facts.first_diff, 600);
    assert_eq!(record.facts.source_units, 374);
    assert_eq!(record.facts.store_units, 375);
    assert_eq!(record.receipt, 1);

    // Equal on every minute: `match`, the one green.
    let answers = [answer_of(on, &stored, 1)];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::Match);
    assert_eq!(record.facts.first_diff, verdict::NONE);
    assert_eq!(
        (record.facts.first_unit, record.facts.last_unit),
        (555, 929)
    );

    // One price off: `value_mismatch` at that minute.
    let mut changed = stored.clone();
    changed[10].close += 1;
    let answers = [answer_of(on, &changed, 1)];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::ValueMismatch);
    assert_eq!((record.facts.value_diff, record.facts.first_diff), (1, 565));

    // The answer has a minute the store lacks, no ledger line in a covered
    // month: the one store defect, `missing_in_store (unexplained)`.
    let short = full_day(on, &[700]);
    let answers = [answer_of(on, &stored, 1)];
    let inputs = DayInputs {
        minutes: &short,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::MissingInStore);
    assert_eq!(reason_word(&record), "unexplained");
    assert_eq!(record.facts.missing, 1);
}

// ------------------------------------------------------------- IA-ZV-02 --

/// IA-ZV-02: a pull with no kept answer is `no_response_kept`, never
/// `match`; an answer whose body was not kept says so.
#[test]
fn a_pull_with_no_kept_answer_is_no_response_kept_never_match() {
    let on = regular();
    let stored = full_day(on, &[]);
    let inputs = DayInputs {
        minutes: &stored,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::NoResponseKept);
    assert_eq!(record.facts.flags & flags::BODY_NOT_KEPT, 0);
    assert_eq!(record.facts.missing, 0, "every owed minute is held");

    let unkept = [DayAnswer {
        receipt: 1,
        asked: whole_day(on),
        kept: false,
        not_kept: words::reason("pack_full").unwrap(),
        ..DayAnswer::default()
    }];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &unkept,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::NoResponseKept);
    assert_ne!(record.facts.flags & flags::BODY_NOT_KEPT, 0);
    assert_eq!(reason_word(&record), "pack_full");
}

// ------------------------------------------------------------- IA-ZV-03 --

/// IA-ZV-03: every stored rung is verified; an option has no derived rung.
#[test]
fn every_stored_rung_is_verified() {
    let on = regular();
    let stored = full_day(on, &[]);
    let empty: Vec<Bar> = Vec::new();
    let derived: Vec<DerivedDay<'_>> = derived_rungs()
        .into_iter()
        .map(|tf| (tf, Some(empty.as_slice()), None))
        .collect();
    let day_bar = Bar {
        ts_micros: utc_s(on, 0) * 1_000_000,
        open: stored[0].open,
        high: stored.iter().map(|b| b.high).max().unwrap(),
        low: stored.iter().map(|b| b.low).min().unwrap(),
        close: stored.last().unwrap().close,
        volume: stored.iter().map(|b| b.volume).sum(),
        open_interest: OI_NULL,
    };
    let inputs = DayInputs {
        minutes: &stored,
        derived: derived.clone(),
        day_bar: Some(day_bar),
        day_month_held: true,
        covered: true,
        ..DayInputs::default()
    };
    let records = verify_day(&index(), on, &inputs);
    let rungs: Vec<&str> = records.iter().map(|r| r.timeframe.as_str()).collect();
    assert_eq!(
        rungs,
        [
            "1min", "2min", "3min", "5min", "10min", "15min", "30min", "60min", "1day"
        ],
        "1min, every derived rung, and 1day"
    );
    assert_eq!(derived_rungs().len(), 7);
    // The day bar is the fold of its minutes: daycheck agrees, no flag.
    let daily = records.last().unwrap();
    assert_eq!(daily.record.facts.flags & flags::DAYCHECK_DIFFERS, 0);
    assert_eq!(daily.record.facts.code, Code::NoResponseKept);

    // A day bar that its minutes exceed: `daycheck_differs` with its counts.
    let mut low = day_bar;
    low.high -= 1;
    let inputs = DayInputs {
        minutes: &stored,
        day_bar: Some(low),
        day_month_held: true,
        covered: true,
        ..DayInputs::default()
    };
    let daily = verify_day(&index(), on, &inputs).pop().unwrap();
    assert_eq!(daily.timeframe, Timeframe::DAY_1);
    assert_ne!(daily.record.facts.flags & flags::DAYCHECK_DIFFERS, 0);
    let detail = &daily.record.details[0];
    assert_eq!(detail.kind, words::DETAIL_DAYCHECK);
    assert_eq!(detail.payload.len(), 28);
    assert_eq!(
        u32::from_le_bytes(detail.payload[8..12].try_into().unwrap()),
        1,
        "contradicted"
    );

    // An option: its pulled rungs only, never a derived one.
    let option = Series {
        exchange: Exchange::Nse,
        segment: Segment::Fno,
        symbol: "ZZIDX".to_owned(),
        contract: Contract::parse("2024-04-25-2222200-CE"),
    };
    assert!(option.is_option());
    let inputs = DayInputs {
        minutes: &stored,
        derived,
        covered: true,
        ..DayInputs::default()
    };
    let rungs: Vec<Timeframe> = verify_day(&option, on, &inputs)
        .iter()
        .map(|r| r.timeframe)
        .collect();
    assert_eq!(rungs, [Timeframe::MINUTE_1]);
}

// ------------------------------------------------------------- IA-ZV-04 --

/// IA-ZV-04 (and Z1, Z12, DB-G46, DB-A12): owed minutes the store lacks are
/// classified by the ledger's `why`; no ledger is `no_ledger`; a minute
/// neither vendor holds is named so.
#[test]
fn each_missing_minute_is_classified_by_why() {
    let on = regular();
    let stored = full_day(on, &[600, 601]);
    let line = |why: &str, origin_store: bool| GapLine {
        why: words::reason(why).unwrap(),
        origin_store,
    };
    let cases: [(Option<GapLine>, bool, Code, &str); 10] = [
        (
            Some(line("vendor_empty", false)),
            true,
            Code::VendorHole,
            "vendor_empty",
        ),
        (
            Some(line("withheld_unmeasured_session", false)),
            true,
            Code::WithheldByRule,
            "withheld_unmeasured_session",
        ),
        // `not_compared` ranks below `no_response_kept` (D-4844's order), so a
        // day with no kept answer keeps the worse code.
        (
            Some(line("calendar_unverified", false)),
            true,
            Code::NoResponseKept,
            "",
        ),
        (
            Some(line("unordered_stamps", false)),
            true,
            Code::NoResponseKept,
            "",
        ),
        (
            Some(line("refused_day", false)),
            true,
            Code::Owed,
            "refused_day",
        ),
        (
            Some(line("refused_front", false)),
            true,
            Code::Owed,
            "refused_front",
        ),
        (
            Some(line("stored_short", false)),
            true,
            Code::Owed,
            "stored_short",
        ),
        (
            Some(line("repaired", false)),
            true,
            Code::MissingInStore,
            "unexplained",
        ),
        (None, true, Code::MissingInStore, "unexplained"),
        (None, false, Code::NoLedger, ""),
    ];
    for (gap, covered, code, word) in cases {
        let inputs = DayInputs {
            minutes: &stored,
            gap,
            covered,
            ..DayInputs::default()
        };
        let record = minute_record(&index(), on, &inputs);
        assert_eq!(record.facts.code, code, "{gap:?} covered={covered}");
        assert_eq!(reason_word(&record), word);
        assert_eq!(record.facts.missing, 2);
        assert_eq!(record.facts.first_diff, 600);
        assert_ne!(
            record.facts.flags & flags::PEER_UNAVAILABLE,
            0,
            "no GDFL month here"
        );
    }

    // THE PEER (DB-A12): GDFL traded neither minute, so neither vendor has
    // it: `vendor_hole (neither_vendor_has_it)`, never a store defect.
    let none: HashSet<u32> = (555..930).filter(|m| *m != 600 && *m != 601).collect();
    let inputs = DayInputs {
        minutes: &stored,
        covered: false,
        peer: Some(&none),
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::VendorHole);
    assert_eq!(reason_word(&record), "neither_vendor_has_it");
    assert_eq!(record.facts.flags & flags::PEER_MINUTE_ABSENT, 0);
    let peer = &record.details[0];
    assert_eq!(peer.kind, words::DETAIL_PEER);
    assert_eq!(
        u32::from_le_bytes(peer.payload[8..12].try_into().unwrap()),
        2,
        "both absent"
    );

    // GDFL traded one of them: the ledger's class stands, and the peer says so.
    let one: HashSet<u32> = (555..930).filter(|m| *m != 601).collect();
    let inputs = DayInputs {
        minutes: &stored,
        covered: false,
        peer: Some(&one),
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::NoLedger);
    assert_ne!(record.facts.flags & flags::PEER_MINUTE_ABSENT, 0);
    assert_eq!(
        u32::from_le_bytes(record.details[0].payload[0..4].try_into().unwrap()),
        1
    );
    assert_eq!(
        u32::from_le_bytes(record.details[0].payload[4..8].try_into().unwrap()),
        600
    );

    // A vendor-empty line keeps its class even when GDFL lacks the minutes.
    let inputs = DayInputs {
        minutes: &stored,
        gap: Some(line("vendor_empty", false)),
        covered: true,
        peer: Some(&none),
        ..DayInputs::default()
    };
    assert_eq!(
        reason_word(&minute_record(&index(), on, &inputs)),
        "vendor_empty"
    );

    // A closed day owes nothing; a bar on it is extra.
    let sunday = day(2024, 4, 21);
    let inputs = DayInputs {
        minutes: &[],
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        minute_record(&index(), sunday, &inputs).facts.code,
        Code::NoResponseKept
    );
    let stray = [bar_at(sunday, 600, 1_111_100)];
    let inputs = DayInputs {
        minutes: &stray,
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        minute_record(&index(), sunday, &inputs).facts.code,
        Code::ExtraInStore
    );

    // Z1: an empty answer on an owed day is the vendor's hole, said by the
    // answer itself.
    let answers = [answer_of(on, &[], 1)];
    let inputs = DayInputs {
        minutes: &[],
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::VendorHole);
    assert_eq!(reason_word(&record), "vendor_empty");
}

/// Z1 by its own name: an empty answer on an owed day is named.
#[test]
fn an_empty_answer_on_an_owed_day_is_named() {
    let on = regular();
    let answers = [answer_of(on, &[], 3)];
    let inputs = DayInputs {
        answers: &answers,
        covered: false,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        (record.facts.code, reason_word(&record)),
        (Code::VendorHole, "vendor_empty")
    );
    assert_eq!(
        (record.facts.source_units, record.facts.store_units),
        (0, 0)
    );
}

/// RA-300: a whole, well-formed answer that stops before the end it was
/// asked for is `answer_short`, kept apart from a vendor hole.
#[test]
fn a_short_answer_is_named_apart_from_a_hole() {
    let on = regular();
    let held = full_day(on, &(900..930).collect::<Vec<_>>());
    let answers = [answer_of(on, &held, 1)];
    let inputs = DayInputs {
        minutes: &held,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::VendorHole);
    assert_eq!(reason_word(&record), "answer_short");

    // A hole in the middle of a whole answer is the vendor's hole.
    let held = full_day(on, &[700]);
    let answers = [answer_of(on, &held, 1)];
    let inputs = DayInputs {
        minutes: &held,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        reason_word(&minute_record(&index(), on, &inputs)),
        "vendor_empty"
    );
}

// ------------------------------------------------------------- IA-ZV-05 --

/// IA-ZV-05: identical duplicate minutes count once with `dup_minutes`;
/// both candles of a conflicting stamp are refused, and every other row
/// lands (D-3351); an off-grid candle is refused by its own word.
#[test]
fn duplicate_minutes_follow_d3351() {
    let on = regular();
    let stored = full_day(on, &[]);
    let mut rows: Vec<RawRow> = stored.iter().map(row_of).collect();
    rows.push(rows[5]); // identical: once
    let mut other = rows[7];
    other.close += 1;
    rows.push(other); // conflicting at minute 562: both refused
    let mut off = rows[9];
    off.timestamp += 30; // off the minute grid
    rows.push(off);
    let made = oracle(
        &rows,
        TimestampEncoding::IsoDateTimeOffset,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(on),
    );
    assert_eq!(
        made.judged, 3,
        "two conflicting candles and one off the grid"
    );
    let mut answer = made.days.get(&on.days_from_epoch()).cloned().unwrap();
    assert_eq!(answer.dup, 1);
    assert_eq!(answer.bars.len(), 374, "every other row lands");
    assert!(!answer.bars.contains_key(&562));
    let words_of: Vec<&str> = answer
        .refused
        .iter()
        .map(|(_, w)| words::reason_word(*w).unwrap())
        .collect();
    assert_eq!(
        words_of
            .iter()
            .filter(|w| **w == "conflicting_candles")
            .count(),
        2,
        "BOTH candles of the stamp"
    );
    assert_eq!(
        words_of.iter().filter(|w| **w == "off_grid_candle").count(),
        1
    );
    answer.receipt = 1;
    answer.asked = whole_day(on);
    answer.kept = true;
    // The landing refused the same two at minute 562: the store lacks it,
    // and that is `source_refused`, never `missing_in_store`.
    let landed = full_day(on, &[562]);
    let answers = [answer];
    let inputs = DayInputs {
        minutes: &landed,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::SourceRefused);
    assert_eq!(record.facts.missing, 0);
    assert_ne!(record.facts.flags & flags::DUP_MINUTES, 0);
    let kinds: Vec<u16> = record.details.iter().map(|d| d.kind).collect();
    assert!(kinds.contains(&words::DETAIL_LANDING_REFUSED));
    assert!(kinds.contains(&words::DETAIL_DUP_MINUTES));
}

/// The oracle's count of the words it judges is compared with the landing's
/// refused rows: a difference is `quality_differs`, never silent.
#[test]
fn the_oracles_refusals_are_counted_against_the_landings() {
    let on = regular();
    let stored = full_day(on, &[]);
    let mut rows: Vec<RawRow> = stored.iter().map(row_of).collect();
    let mut off = rows[9];
    off.timestamp += 30;
    rows.push(off);
    let refused_row = response_keep::RefusedRow {
        local_second: utc_s(on, 564) + 30 + 19_800,
        cause: RowCause::OffGrid,
    };
    let mut agree = oracle(
        &rows,
        TimestampEncoding::IsoDateTimeOffset,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(on),
    );
    agree.take_landing_refusals(&[refused_row], 1);
    assert_eq!(agree.days[&on.days_from_epoch()].quality, None);
    let mut differ = oracle(
        &rows,
        TimestampEncoding::IsoDateTimeOffset,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(on),
    );
    differ.take_landing_refusals(&[], 0);
    assert_eq!(differ.days[&on.days_from_epoch()].quality, Some((0, 1)));
}

// ------------------------------------------------------------- IA-ZV-06 --

/// IA-ZV-06: prices are scaled, never re-snapped: a three-decimal rupee
/// price the decoder snapped reads as stored.
#[test]
fn the_landing_oracle_scales_and_never_snaps_again() {
    let crate::vendor::Transport::Http(spec) = crate::vendor::Feed::Zerodha.descriptor().transport
    else {
        panic!("Zerodha is an HTTP feed");
    };
    // Invented: 11111.125 is not a price any real index printed here.
    let body = r#"{"status":"success","data":{"candles":[
        ["2024-04-23T09:15:00+0530",11111.125,11111.5,11110.995,11111.004,7]]}}"#;
    let window =
        crate::http::decode_body(body, &spec, crate::vendor::Listing::Index).expect("decodes");
    assert_eq!(
        window.rows[0].open, 1_111_113,
        "decode_body snapped half-up, once"
    );
    let on = regular();
    let made = oracle(
        &window.rows,
        spec.timestamps,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(on),
    );
    let bar = made.days[&on.days_from_epoch()].bars[&555];
    assert_eq!(
        (bar.open, bar.high, bar.low, bar.close),
        (1_111_113, 1_111_150, 1_111_100, 1_111_100),
        "the scale only: never x100 again, never a second snap"
    );
    assert_eq!(bar.ts_micros, utc_s(on, 555) * 1_000_000);
}

// ------------------------------------------------------------- IA-ZV-07 --

/// IA-ZV-07: the refold withholds an incomplete INDEX bucket and folds a
/// CASH bucket as partial (D-3321); a month with no `derive` line is
/// `built_under_earlier_rule (pre_ledger)`.
#[test]
fn the_refold_follows_d3321() {
    let on = regular();
    let five = Timeframe::MINUTE_5;
    let minutes = full_day(on, &[600]);
    // The store's 5min bars as the derive files them: every complete bucket,
    // and for a share the partial one too.
    let fold = |series: &Series| -> Vec<Bar> {
        let (buckets, unjudged) = refold(series, on.days_from_epoch(), &minutes, five);
        assert_eq!(unjudged, None);
        buckets
            .iter()
            .filter(|b| b.complete)
            .map(|b| b.bar)
            .collect()
    };
    let index_bars = fold(&index());
    assert_eq!(
        index_bars.len(),
        74,
        "75 buckets, the one holding 10:00 withheld"
    );
    let first = index_bars[0];
    assert_eq!(
        first.ts_micros,
        utc_s(on, 555) * 1_000_000,
        "the grid starts at 09:15"
    );
    assert_eq!(first.open, minutes[0].open);
    assert_eq!(first.close, minutes[4].close);
    assert_eq!(first.volume, (0..5).map(|i| minutes[i].volume).sum::<i64>());
    let derived = vec![(five, Some(index_bars.as_slice()), Some(1))];
    let inputs = DayInputs {
        minutes: &minutes,
        derived,
        covered: true,
        ..DayInputs::default()
    };
    let records = verify_day(&index(), on, &inputs);
    let five_record = &records[1].record;
    assert_eq!(records[1].timeframe, five);
    assert_eq!(five_record.facts.code, Code::WithheldByRule);
    assert_eq!(reason_word(five_record), "index_bucket_incomplete");
    assert_eq!(five_record.facts.rule_set, RULES_D3321);
    assert_eq!(
        (
            five_record.facts.source_units,
            five_record.facts.store_units
        ),
        (74, 74)
    );

    // A share's bucket folds as partial: 75 bars, and they match.
    let share_bars = fold(&share());
    assert_eq!(share_bars.len(), 75);
    let derived = vec![(five, Some(share_bars.as_slice()), Some(1))];
    let inputs = DayInputs {
        minutes: &minutes,
        derived,
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        verify_day(&share(), on, &inputs)[1].record.facts.code,
        Code::Match
    );

    // One stored bar differs: `value_mismatch` at its bucket.
    let mut wrong = share_bars.clone();
    wrong[3].high += 5;
    let derived = vec![(five, Some(wrong.as_slice()), Some(1))];
    let inputs = DayInputs {
        minutes: &minutes,
        derived,
        covered: true,
        ..DayInputs::default()
    };
    let record = &verify_day(&share(), on, &inputs)[1].record;
    assert_eq!(
        (record.facts.code, record.facts.first_diff),
        (Code::ValueMismatch, 570)
    );

    // No derive line: counts recorded, never judged by value.
    let derived = vec![(five, Some(wrong.as_slice()), None)];
    let inputs = DayInputs {
        minutes: &minutes,
        derived,
        covered: true,
        ..DayInputs::default()
    };
    let record = &verify_day(&share(), on, &inputs)[1].record;
    assert_eq!(record.facts.code, Code::BuiltUnderEarlierRule);
    assert_eq!(reason_word(record), "pre_ledger");
    assert_eq!(record.details[0].kind, words::DETAIL_RULE);

    // A 60min rung: six whole buckets and the 15-minute stub at 15:15.
    let (hourly, _) = refold(
        &index(),
        on.days_from_epoch(),
        &full_day(on, &[]),
        Timeframe::MINUTE_60,
    );
    let starts: Vec<u32> = hourly.iter().map(|b| b.start).collect();
    assert_eq!(starts, [555, 615, 675, 735, 795, 855, 915]);
    assert!(hourly.iter().all(|b| b.complete));
    assert_eq!(hourly.last().unwrap().owed, 15);
}

/// RA-267: a derived rung holding fewer days than its 1min source is named,
/// whatever rule set derived its month: a day whose minutes fold to standing
/// buckets but whose rung holds no bar of it (its month held, or not held
/// at all) reads `missing_in_store (rung_day_absent)`, every standing bucket
/// counted missing and the first one named. A rung that holds even one bar
/// of the day is judged bucket by bucket as before, and a day with no
/// minutes at all is never a rung's absence.
#[test]
fn a_rung_holding_fewer_days_than_its_source_is_named() {
    let on = regular();
    let minutes = full_day(on, &[]);
    let (five, ten, sixty) = (
        Timeframe::MINUTE_5,
        Timeframe::MINUTE_10,
        Timeframe::MINUTE_60,
    );
    let (hourly, _) = refold(&index(), on.days_from_epoch(), &minutes, sixty);
    let one_hour = [hourly[0].bar];
    let inputs = DayInputs {
        minutes: &minutes,
        derived: vec![
            // Held month, ledger v=1, no bar of the day.
            (five, Some(&[][..]), Some(RULES_D3321)),
            // Month not held, derived before the ledger.
            (ten, None, None),
            // One bar of the day held: judged per bucket, not as absent.
            (sixty, Some(&one_hour[..]), Some(RULES_D3321)),
        ],
        covered: true,
        ..DayInputs::default()
    };
    let records = verify_day(&index(), on, &inputs);
    let by_rung = |tf: Timeframe| &records.iter().find(|r| r.timeframe == tf).unwrap().record;
    for (tf, standing, rule_set) in [(five, 75, RULES_D3321), (ten, 38, 0)] {
        let record = by_rung(tf);
        assert_eq!(record.facts.code, Code::MissingInStore, "{tf:?}");
        assert_eq!(reason_word(record), "rung_day_absent", "{tf:?}");
        assert_eq!(
            (
                record.facts.source_units,
                record.facts.store_units,
                record.facts.missing,
                record.facts.first_diff,
                record.facts.rule_set,
            ),
            (standing, 0, standing, 555, rule_set),
            "{tf:?}"
        );
    }
    let hour = by_rung(sixty);
    assert_eq!(
        (hour.facts.code, reason_word(hour), hour.facts.missing),
        (Code::MissingInStore, "unexplained", 6)
    );

    // No minutes: no standing bucket, so no rung is "absent".
    let inputs = DayInputs {
        minutes: &[],
        derived: vec![(five, Some(&[][..]), Some(RULES_D3321))],
        covered: true,
        ..DayInputs::default()
    };
    let records = verify_day(&index(), on, &inputs);
    let record = &records.iter().find(|r| r.timeframe == five).unwrap().record;
    assert_ne!(record.facts.reason, reason_of("rung_day_absent"));
    assert_eq!((record.facts.source_units, record.facts.missing), (0, 0));
}

// ------------------------------------------------------------- IA-ZV-08 --

/// IA-ZV-08 (and Z4): a re-pull that differs from the store is
/// `vendor_revised`, never `value_mismatch`.
#[test]
fn a_vendor_revision_is_not_a_store_defect() {
    let on = regular();
    let stored = full_day(on, &[]);
    let mut revised = stored.clone();
    revised[20].close += 7;
    let answers = [answer_of(on, &stored, 1), answer_of(on, &revised, 2)];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        record.facts.code,
        Code::Match,
        "the store kept the first answer"
    );
    assert_ne!(record.facts.flags & flags::VENDOR_REVISED, 0);
    let receipts = record
        .details
        .iter()
        .find(|d| d.kind == words::DETAIL_RECEIPTS)
        .unwrap();
    assert_eq!(receipts.payload, [1, 0, 0, 0, 2, 0, 0, 0]);
}

/// Z4 by its own name, through the merge: two records of one key and day
/// in one spool become one, flagged and naming both receipts.
#[test]
fn a_repull_that_differs_from_the_store_is_named() {
    let root = scratch("repull");
    let on = regular();
    let writer = SpoolWriter::new(&root, 41, Mode::ZerodhaMerge, 1);
    let mut later = spooled(on, "NSE-INDEX-ZZIDX", Code::ValueMismatch, 2);
    later.facts.value_diff = 1;
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[later, spooled(on, "NSE-INDEX-ZZIDX", Code::Match, 1)],
        )
        .unwrap();
    drop(writer);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!(outcome.generations, 1);
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(record.facts.code, Code::Match, "the first receipt's record");
    assert_ne!(record.facts.flags & flags::VENDOR_REVISED, 0);
    assert_eq!(record.run, 41);
}

// ------------------------------------------------------------- IA-ZV-09 --

/// IA-ZV-09: the calendar sweep writes a record for a day no window landed
/// (and for a day with no bar at all).
#[test]
fn a_day_without_a_window_is_still_verified() {
    let root = scratch("sweep");
    let on = regular();
    let next = day(2024, 4, 24);
    let mut bars = full_day(on, &[]);
    bars.extend(full_day(day(2024, 4, 25), &[]));
    write_month(&root, &index(), Timeframe::MINUTE_1, &bars);
    let writer = SpoolWriter::new(&root, 7, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: day(2024, 4, 25),
        only: vec!["ZZIDX".to_owned()],
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(report.series, 1);
    assert_eq!(report.days, 3, "23, 24 (no bar) and 25 April");
    drop(writer);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!(
        outcome.spools_merged, 8,
        "one spool per rung: 1min and seven derived"
    );
    assert_eq!(outcome.generations, 24, "one generation per (rung, day)");
    for on in [on, next, day(2024, 4, 25)] {
        assert_eq!(
            year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
            1
        );
    }
    let held = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(held.facts.code, Code::NoResponseKept);
    let empty = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        next,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        empty.facts.code,
        Code::NoLedger,
        "375 owed minutes, no ledger"
    );
    assert_eq!(empty.facts.missing, 375);
    assert_eq!(empty.run, 7);
    assert!(
        spool::waiting(&root).unwrap().spools.is_empty(),
        "the merged spool is gone"
    );
}

/// RA-145: the cold sweep checks series side by side on workers, and the
/// answer is the same whatever their count: seven series (each with its own
/// hole) on one worker and on four write the same records and the same
/// report, but for the worker count it names. Asked for more workers than
/// series, it starts no more than the series; asked for none, one.
#[test]
fn the_cold_sweep_runs_on_workers_and_answers_as_one() {
    let on = regular();
    let all: Vec<Series> = (0..7_u32)
        .map(|n| Series {
            exchange: Exchange::Nse,
            segment: if n == 6 {
                Segment::Cash
            } else {
                Segment::Index
            },
            symbol: format!("ZZW{n}"),
            contract: None,
        })
        .collect();
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let mut answers = Vec::new();
    for (tag, workers) in [("w1", 1), ("w4", 4), ("w0", 0), ("w99", 99)] {
        let root = scratch(tag);
        for (n, series) in all.iter().enumerate() {
            let hole = 600 + u32::try_from(n).unwrap() * 11;
            write_month(&root, series, Timeframe::MINUTE_1, &full_day(on, &[hole]));
        }
        let writer = SpoolWriter::new(&root, 50, Mode::ZerodhaCold, 1);
        let report = cold_on(&root, &request, &writer, None, workers).unwrap();
        drop(writer);
        merge_all(&root, None, None).unwrap();
        let records: Vec<verdict::Record> = all
            .iter()
            .flat_map(|series| {
                let key = series.key_text();
                let root = &root;
                [Timeframe::MINUTE_1, Timeframe::MINUTE_5]
                    .into_iter()
                    .map(move |tf| the_record(root, series.segment, tf, on, &key))
            })
            .collect();
        answers.push((tag, workers, report, records));
    }
    let (_, _, first_report, first_records) = &answers[0];
    assert_eq!(first_report.series, 7);
    assert_eq!(first_report.days, 7);
    assert_eq!(first_records.len(), 14);
    // EVERY UNIT LACKING IS COUNTED BY ITS CLASS (DB-A12): one hole in each
    // series' day is seven minutes at 1min, whatever their records' codes.
    let minutes_lacking: u64 = first_report
        .missing
        .iter()
        .filter(|(rung, ..)| rung == "1min")
        .map(|(.., units)| *units)
        .sum();
    assert_eq!(minutes_lacking, 7, "{:?}", first_report.missing);
    for (tag, workers, report, records) in &answers {
        let started = (*workers).clamp(1, MAX_COLD_WORKERS).min(7);
        assert_eq!(report.workers, started as u64, "{tag}");
        assert_eq!(
            ColdReport {
                workers: 0,
                ..report.clone()
            },
            ColdReport {
                workers: 0,
                ..first_report.clone()
            },
            "{tag}: the same report"
        );
        assert_eq!(records, first_records, "{tag}: the same records");
    }
    assert!((1..=MAX_COLD_WORKERS).contains(&cold_workers()));
}

/// A stop seen BETWEEN series, with series left, is a run that did not reach
/// the span's end, and says so (`stopped`), even though no series saw the
/// stop itself: on one worker the third check (before the second series)
/// trips, so one series of three is checked and spooled.
#[test]
fn a_stop_between_series_says_the_span_was_not_reached() {
    let root = scratch("stop-between");
    let on = regular();
    for n in 0..3 {
        let series = Series {
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: format!("ZZS{n}"),
            contract: None,
        };
        write_month(&root, &series, Timeframe::MINUTE_1, &full_day(on, &[]));
    }
    let writer = SpoolWriter::new(&root, 60, Mode::ZerodhaCold, 1);
    let stop = StopControl::tripping_at(root.join("verify/zerodha.stop"), 60, 3);
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let report = cold_on(&root, &request, &writer, Some(&stop), 1).unwrap();
    assert!(report.stopped, "{report:?}");
    assert_eq!((report.series, report.months, report.days), (1, 1, 1));
}

/// A month missing in the MIDDLE of a series' life: the cold sweep walks
/// every month from the series' first held 1min month to its last, in
/// order, held or not, so each calendar-open day of the absent month still
/// gets a record, classified by the ledger (23 April by its `vendor_empty`
/// line; 24 April, with no line in a month the ledger covers, as the
/// store's own hole). Before the walk was by month, an absent month was
/// silence: no record at all.
#[test]
fn a_month_missing_in_the_middle_is_still_verified() {
    let root = scratch("midhole");
    let (march, april, may) = (day(2024, 3, 1), regular(), day(2024, 5, 2));
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(march, &[]));
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(may, &[]));
    write_ledger(
        &root,
        "gap NSE-INDEX-ZZIDX 2024-04-23 v=1 rung=1min owed=375 held=0 missing=375 \
         first_missing=09:15 runs=1 longest=375 longest_at=09:15 why=vendor_empty run=1\n",
    );
    let writer = SpoolWriter::new(&root, 41, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: day(2024, 1, 1),
        to: day(2024, 12, 31),
        only: vec!["ZZIDX".to_owned()],
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(
        report.months, 3,
        "March, the absent April and May; nothing before the first held month or after the last"
    );
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let hole = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        april,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        (
            hole.facts.code,
            words::reason_word(hole.facts.reason),
            hole.facts.missing,
            hole.facts.store_units
        ),
        (Code::VendorHole, Some("vendor_empty"), 375, 0)
    );
    let unexplained = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        day(2024, 4, 24),
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        (unexplained.facts.code, unexplained.facts.missing),
        (Code::MissingInStore, 375)
    );
    assert_eq!(
        next_month(YearMonth::new(2024, 12).unwrap()),
        Some(YearMonth::new(2025, 1).unwrap())
    );
    assert_eq!(next_month(YearMonth::new(9999, 12).unwrap()), None);
}

// ------------------------------------------------------------- IA-ZV-10 --

/// IA-ZV-10: a crash between a landing and its keep reads
/// `no_response_kept`, and an older `match` reads stale.
#[test]
fn a_landing_without_its_keep_is_never_an_old_match() {
    let root = scratch("unkept");
    let on = regular();
    let whole = full_day(on, &[]);
    // The day landed and was kept: `match`, over its 375 rows.
    let answers = [answer_of(on, &whole, 1)];
    let inputs = DayInputs {
        minutes: &whole,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let first = minute_record(&index(), on, &inputs);
    assert_eq!(first.facts.code, Code::Match);
    assert_eq!(
        (
            first.facts.month_generation,
            first.facts.first_row,
            first.facts.rows
        ),
        (0, 0, 375)
    );
    let writer = SpoolWriter::new(&root, 11, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            std::slice::from_ref(&first),
        )
        .unwrap();
    drop(writer);
    merge_all(&root, None, None).unwrap();

    // Only the morning's answer was kept; the afternoon landed and the
    // process died before its keep. The minutes no kept answer asked for
    // are never vouched for: `no_response_kept`, never the old `match`.
    let morning: Vec<Bar> = whole
        .iter()
        .copied()
        .filter(|b| minute_of(b) < 720)
        .collect();
    let mut kept = answer_of(on, &morning, 2);
    kept.asked = (utc_s(on, 555), utc_s(on, 719));
    let answers = [kept];
    let inputs = DayInputs {
        minutes: &whole,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let after = minute_record(&index(), on, &inputs);
    assert_eq!(after.facts.code, Code::NoResponseKept);
    assert_eq!(after.facts.missing, 0);

    // And the older `match` reads stale once the day's month changed under
    // it: another served generation, or another row count.
    let series = verdict::Series {
        root: &root,
        feed: Vendor::Zerodha,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
    };
    let looked = verdict::Reader::new()
        .lookup(&series, store_day(on).unwrap(), &first.key)
        .unwrap();
    let answer = looked.answer.unwrap();
    assert_eq!(answer.code(), Some(Code::Match));
    let read = |generation, rows| verdict::Now {
        verifier: VERIFIER,
        month: verdict::MonthNow::Read {
            generation,
            first_row: 0,
            rows,
        },
    };
    assert!(
        answer.staleness(&read(0, 375)).is_fresh(),
        "unchanged: fresh"
    );
    assert!(
        !answer.staleness(&read(1, 375)).is_fresh(),
        "a repaired month reads stale"
    );
    assert!(
        !answer.staleness(&read(0, 376)).is_fresh(),
        "another row count reads stale"
    );
}

// ------------------------------------------------------------- IA-ZV-11 --

/// IA-ZV-11: a window landed from another store is verified cold only: its
/// minutes read `no_response_kept (landed_from_store)`, never `match`; its
/// derived rungs and 1day are still checked.
#[test]
fn a_window_landed_from_another_store_is_verified_cold_only() {
    let root = scratch("fromstore");
    let on = regular();
    let minutes = full_day(on, &[]);
    write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
    let (buckets, _) = refold(
        &index(),
        on.days_from_epoch(),
        &minutes,
        Timeframe::MINUTE_5,
    );
    let five: Vec<Bar> = buckets.iter().map(|b| b.bar).collect();
    write_month(&root, &index(), Timeframe::MINUTE_5, &five);
    let mut daily = minutes[0];
    daily.ts_micros = utc_s(on, 0) * 1_000_000;
    write_month(&root, &index(), Timeframe::DAY_1, &[daily]);
    write_ledger(
        &root,
        "gap NSE-INDEX-ZZIDX 2024-04-23 v=1 rung=1min owed=375 held=375 missing=0 first_missing=- runs=0 longest=0 longest_at=- why=repaired run=3 origin=store\n\
         derive NSE-INDEX-ZZIDX 5min 2024-04 v=1 days=1 buckets=75 withheld=0 partial=0 run=3\n",
    );
    let writer = SpoolWriter::new(&root, 12, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(report.records, 9, "1min, seven derived rungs and 1day");
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let minute = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(minute.facts.code, Code::NoResponseKept);
    assert_eq!(
        words::reason_word(minute.facts.reason),
        Some("landed_from_store")
    );
    let derived = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_5,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        derived.facts.code,
        Code::Match,
        "the 5min rung is refolded and matches"
    );
    let day_record = the_record(
        &root,
        Segment::Index,
        Timeframe::DAY_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(day_record.facts.code, Code::NoResponseKept);
    assert_ne!(
        day_record.facts.flags & flags::DAYCHECK_DIFFERS,
        0,
        "a day bar of one minute"
    );
    // A rung whose month is not held at all: the rung holds none of a day
    // its 1min holds, `missing_in_store (rung_day_absent)` under every rule
    // set (RA-267), never skipped and never "built under an earlier rule".
    let two = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_2,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        (two.facts.code, words::reason_word(two.facts.reason)),
        (Code::MissingInStore, Some("rung_day_absent")),
        "no 2min month held"
    );
    assert_eq!(two.facts.source_units, 188);
    assert_eq!(two.facts.store_units, 0);
    assert_eq!(two.facts.missing, 188);
}

// ----------------------------------------------------------------- Z5, Z15 --

/// Z5: an absent OI and a zero OI are different facts, compared apart.
#[test]
fn an_absent_oi_and_a_zero_oi_are_compared_apart() {
    let on = regular();
    let mut stored = full_day(on, &[]);
    stored[0].open_interest = 0;
    let answers = [answer_of(on, &full_day(on, &[]), 1)];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::ValueMismatch);
    assert_eq!((record.facts.oi_diff, record.facts.value_diff), (1, 0));
}

/// Z15 (CMP-18): a minute of another security is refused as the landing
/// refuses it; one already stored before the series' first day bar is
/// `store_without_source`.
#[test]
fn a_foreign_minute_is_refused_as_the_landing_refuses_it() {
    let on = regular();
    let stored = full_day(on, &[600]);
    let rows: Vec<RawRow> = full_day(on, &[]).iter().map(row_of).collect();
    let mut made = oracle(
        &rows,
        TimestampEncoding::IsoDateTimeOffset,
        Timeframe::MINUTE_1,
        Segment::Index,
        whole_day(on),
    );
    made.take_landing_refusals(
        &[response_keep::RefusedRow {
            local_second: utc_s(on, 600) + 19_800,
            cause: RowCause::ForeignLevel,
        }],
        1,
    );
    let mut answer = made.days.remove(&on.days_from_epoch()).unwrap();
    answer.receipt = 1;
    answer.asked = whole_day(on);
    answer.kept = true;
    assert_eq!(answer.quality, None, "the landing's count is the oracle's");
    let answers = [answer];
    let inputs = DayInputs {
        minutes: &stored,
        answers: &answers,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::SourceRefused);
    assert_eq!(reason_word(&record), "foreign_level");

    let whole = full_day(on, &[]);
    let inputs = DayInputs {
        minutes: &whole,
        covered: true,
        first_day_bar: Some(day(2024, 5, 2)),
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::StoreWithoutSource);
    assert_eq!(reason_word(&record), "before_first_day_bar");
}

/// A stored day whose stamps do not increase is `store_unreadable`, by its
/// own word.
#[test]
fn a_day_whose_stamps_do_not_increase_is_unreadable() {
    let on = regular();
    let mut stored = full_day(on, &[]);
    stored.swap(3, 4);
    let inputs = DayInputs {
        minutes: &stored,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        (record.facts.code, reason_word(&record)),
        (Code::StoreUnreadable, "out_of_order")
    );
    stored.swap(3, 4);
    stored[4] = stored[3];
    let inputs = DayInputs {
        minutes: &stored,
        covered: true,
        ..DayInputs::default()
    };
    assert_eq!(
        reason_word(&minute_record(&index(), on, &inputs)),
        "duplicate"
    );
}

/// A special session (the Muhurat of 1 November 2024) owes its recorded
/// windows only, and the refold places it on the same grid.
#[test]
fn a_special_session_owes_its_recorded_windows() {
    let muhurat = day(2024, 11, 1);
    let Owes::Windows((windows, count)) = repair_plan::owed("INDEX", muhurat.days_from_epoch())
    else {
        panic!("the Muhurat of record owes windows");
    };
    let owed: Vec<u32> = windows
        .iter()
        .take(count)
        .flat_map(|(from, to)| *from..*to)
        .collect();
    assert!(!owed.is_empty() && owed.len() < 375);
    let stored: Vec<Bar> = owed
        .iter()
        .map(|m| bar_at(muhurat, *m, 1_111_100))
        .collect();
    let inputs = DayInputs {
        minutes: &stored,
        gap: None,
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), muhurat, &inputs);
    assert_eq!(
        record.facts.missing, 0,
        "every owed minute of the session is held"
    );
    assert_eq!(record.facts.code, Code::NoResponseKept);
}

// --------------------------------------------------------------- the ledger --

/// The gap ledger is read once: the last line of a key and day wins, a
/// month is covered by any line of it, and a line that does not read is a
/// fault by its number.
#[test]
fn the_gap_ledger_is_read_once_and_its_faults_are_named() {
    let ledger = GapLedger::parse(
        b"gap NSE-INDEX-ZZIDX 2024-04-23 v=1 rung=1min owed=375 held=300 missing=75 why=vendor_empty run=1\n\
          gap NSE-INDEX-ZZIDX 2024-04-23 v=1 rung=1min owed=375 held=375 missing=0 why=repaired run=2 origin=store\n\
          derive NSE-INDEX-ZZIDX 5min 2024-05 v=1 days=1 buckets=75 withheld=0 partial=0 run=2\n\
          gap NSE-INDEX-ZZIDX not-a-day v=1 why=vendor_empty\n\
          gap NSE-INDEX-ZZIDX 2024-04-24 v=1 why=a_word_no_table_names\n\
          some other record kind\n\
          gap NSE-INDEX-ZZIDX 2024-04-25 v=1 why=vendor_empty",
    );
    let line = ledger.gap("NSE-INDEX-ZZIDX", regular()).unwrap();
    assert_eq!(
        words::reason_word(line.why),
        Some("repaired"),
        "the last line wins"
    );
    assert!(line.origin_store);
    assert!(ledger.covers("NSE-INDEX-ZZIDX", YearMonth::new(2024, 4).unwrap()));
    assert!(ledger.covers("NSE-INDEX-ZZIDX", YearMonth::new(2024, 5).unwrap()));
    assert!(!ledger.covers("NSE-INDEX-ZZIDX", YearMonth::new(2024, 6).unwrap()));
    assert!(!ledger.covers("NSE-CASH-ZZSHARE", YearMonth::new(2024, 4).unwrap()));
    assert_eq!(
        ledger.derive_version(
            "NSE-INDEX-ZZIDX",
            Timeframe::MINUTE_5,
            YearMonth::new(2024, 5).unwrap()
        ),
        Some(1)
    );
    assert_eq!(
        ledger.derive_version(
            "NSE-INDEX-ZZIDX",
            Timeframe::MINUTE_2,
            YearMonth::new(2024, 5).unwrap()
        ),
        None
    );
    assert_eq!(
        ledger.faults,
        [4, 5],
        "a line that does not read, by its number"
    );
    assert!(
        ledger.gap("NSE-INDEX-ZZIDX", day(2024, 4, 25)).is_some(),
        "a last line with no newline"
    );
    assert_eq!(GapLedger::parse(b"").lines, 0);
    let root = scratch("noledger");
    let absent = GapLedger::read(&root).unwrap();
    assert!(absent.faults.is_empty() && absent.lines == 0);
}

// ------------------------------------------------------------- IA-VD-08 --

/// IA-VD-08: a run writes one generation per (feed, segment, timeframe,
/// day); an orphan spool is merged once; a second merger exits `run_busy`;
/// 100 single requests leave no spool once the merge that followed them
/// ends (D-4921).
#[test]
fn a_run_writes_one_generation_per_day_and_orphan_spools_merge_once() {
    let root = scratch("merge");
    let days = [regular(), day(2024, 4, 24), day(2024, 4, 25)];
    let writer = SpoolWriter::new(&root, 21, Mode::ZerodhaMerge, 1);
    for (at, on) in days.iter().enumerate() {
        // Two windows of one day append twice: still one generation.
        for window in 0..2 {
            let key = format!("NSE-INDEX-ZZIDX{window}");
            writer
                .append(
                    Segment::Index,
                    Timeframe::MINUTE_1,
                    &[spooled(*on, &key, Code::Match, u32::try_from(at).unwrap())],
                )
                .unwrap();
        }
    }
    // An orphan spool of another run, killed before its merge.
    let orphan = SpoolWriter::new(&root, 22, Mode::ZerodhaMerge, 1);
    orphan
        .append(
            Segment::Cash,
            Timeframe::MINUTE_1,
            &[spooled(days[0], "NSE-CASH-ZZSHARE", Code::NoLedger, 1)],
        )
        .unwrap();
    // A second merger while the first holds the lock: `run_busy`, nothing read.
    let held = crate::gdfl_run::locks::open_lock_file(&root, spool::MERGE_LOCK).unwrap();
    let lock = store::flock::Flock::try_lock(held, root.join(spool::MERGE_LOCK)).unwrap();
    assert!(matches!(
        merge_all(&root, None, None),
        Err(MergeError::RunBusy { .. })
    ));
    drop(lock);
    drop((writer, orphan));
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!(outcome.spools_merged, 2);
    assert_eq!(
        outcome.generations, 4,
        "three days of run 21, one of run 22"
    );
    assert_eq!(outcome.records, 7);
    for on in days {
        assert_eq!(
            year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
            1
        );
    }
    assert_eq!(
        year_generation(&root, Segment::Cash, Timeframe::MINUTE_1, days[0]),
        1
    );
    // Merged once: a second merge finds nothing and writes nothing.
    let again = merge_all(&root, None, None).unwrap();
    assert_eq!((again.spools_merged, again.generations), (0, 0));
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, days[0]),
        1
    );
    assert!(
        !root.join(spool::SPOOL_DIR).join("21").exists(),
        "the run's empty folders go"
    );

    // D-4921: 100 single requests, each its own run id and spool.
    for run in 100..200_u64 {
        let single = SpoolWriter::new(&root, run, Mode::ZerodhaMerge, 1);
        single
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(
                    days[1],
                    &format!("NSE-INDEX-Z{run}"),
                    Code::Match,
                    1,
                )],
            )
            .unwrap();
    }
    assert_eq!(spool::waiting(&root).unwrap().spools.len(), 100);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!(outcome.spools_merged, 100);
    assert_eq!(outcome.waiting, 0, "no spool is left once the merge ends");
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        days[1],
        "NSE-INDEX-Z150",
    );
    assert_eq!(record.run, 150);
    // Carry::Others: run 21's keys of that day are carried, not lost.
    let carried = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        days[1],
        "NSE-INDEX-ZZIDX0",
    );
    assert_eq!(carried.run, 21);
}

// ------------------------------------------------------------- IA-VD-15 --

/// IA-VD-15: a failed spool write is written over at the run's own offset;
/// the merge ignores a torn tail and an unsynced last record and names both;
/// a damaged earlier record refuses the spool once, by name; a crash between
/// a day's commit and its merge-log line writes no duplicate generation.
#[test]
fn a_spool_survives_a_crash_without_a_duplicate_or_a_silent_skip() {
    let on = regular();
    let id = |_root: &Path, run: u64| SpoolId {
        run,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };

    // (1) A torn tail is written over by the run's next append.
    let root = scratch("torn");
    let writer = SpoolWriter::new(&root, 31, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-A", Code::Match, 1)],
        )
        .unwrap();
    let path = id(&root, 31).path(&root);
    append_raw(&path, &[0xAB; 70]);
    let read = spool::read_spool(&path, &id(&root, 31)).unwrap();
    assert_eq!(read.torn, Some((224, 70)), "named by offset and bytes");
    assert_eq!(read.records.len(), 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-B", Code::Match, 2)],
        )
        .unwrap();
    let read = spool::read_spool(&path, &id(&root, 31)).unwrap();
    assert_eq!(
        read.records.len(),
        2,
        "the next append wrote over the torn bytes"
    );
    assert_eq!(read.torn, None);

    // (2) An unsynced last record is ignored and named; the merge goes on.
    let last = std::fs::metadata(&path).unwrap().len() - 1;
    flip(&path, last);
    let read = spool::read_spool(&path, &id(&root, 31)).unwrap();
    assert_eq!(read.unsynced, Some(224));
    assert_eq!(read.records.len(), 1);
    drop(writer);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!(outcome.generations, 1);
    assert_eq!(outcome.torn, [(path.clone(), 224, 160)]);

    // (2b) The same run resumed in a NEW process (a fresh writer, as after a
    // restart) writes over an unsynced last record rather than after it, so
    // it never becomes an earlier bad record that refuses the whole spool.
    let root = scratch("resumed");
    let before = SpoolWriter::new(&root, 34, Mode::ZerodhaMerge, 1);
    for key in ["NSE-INDEX-A", "NSE-INDEX-B"] {
        before
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(on, key, Code::Match, 1)],
            )
            .unwrap();
    }
    drop(before);
    let path = id(&root, 34).path(&root);
    let last = std::fs::metadata(&path).unwrap().len() - 1;
    flip(&path, last);
    let after = SpoolWriter::new(&root, 34, Mode::ZerodhaMerge, 1);
    after
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-C", Code::Match, 2)],
        )
        .unwrap();
    let read = spool::read_spool(&path, &id(&root, 34)).unwrap();
    let keys: Vec<String> = read.records.iter().map(|r| r.key.to_string()).collect();
    assert_eq!(keys, ["NSE-INDEX-A", "NSE-INDEX-C"], "B was written over");
    assert_eq!((read.unsynced, read.torn), (None, None));
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 64 + 2 * 160);

    // (3) A damaged EARLIER record refuses the spool once, by name, with an
    // alert; a later merge does not try again until `--retry`.
    let root = scratch("damaged");
    let writer = SpoolWriter::new(&root, 32, Mode::ZerodhaMerge, 1);
    for key in ["NSE-INDEX-A", "NSE-INDEX-B", "NSE-INDEX-C"] {
        writer
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(on, key, Code::Match, 1)],
            )
            .unwrap();
    }
    // A second spool file of the same run, damaged too: one alert for the
    // run, never one per path (D-4883), and both named in the outcome.
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_5,
            &[
                spooled(on, "NSE-INDEX-A", Code::Match, 1),
                spooled(on, "NSE-INDEX-B", Code::Match, 1),
            ],
        )
        .unwrap();
    let other = SpoolId {
        timeframe: Timeframe::MINUTE_5,
        ..id(&root, 32)
    }
    .path(&root);
    flip(&other, 64 + 20);
    let path = id(&root, 32).path(&root);
    flip(&path, 64 + 20);
    assert!(matches!(
        spool::read_spool(&path, &id(&root, 32)),
        Err(SpoolError::Damaged { record: 0, .. })
    ));
    drop(writer);
    let first = merge_all(&root, None, None).unwrap();
    assert_eq!(first.refused.len(), 2);
    for (_, why) in &first.refused {
        assert!(why.starts_with("spool_damaged"), "{why}");
    }
    assert_eq!(first.generations, 0, "nothing of it is merged");
    let alerts = crate::alerts::window(&root).unwrap();
    let damaged: Vec<_> = alerts
        .alerts
        .iter()
        .filter(|a| a.record.kind == 10)
        .collect();
    assert_eq!(damaged.len(), 1, "one alert for the run's two spools");
    assert_eq!(damaged[0].record.run, 32);
    let second = merge_all(&root, None, None).unwrap();
    assert!(second.refused.is_empty());
    assert_eq!(second.refused_earlier.len(), 2);
    assert!(second.refused_earlier.contains(&path));
    assert!(second.refused_earlier.contains(&other));
    let retried = merge_all(&root, Some(&path), None).unwrap();
    assert_eq!(
        retried.refused.len(),
        1,
        "a retry reads it again, and refuses it again"
    );

    // (4) A crash between a day's commit and its merge-log line: the year
    // index and the image's run say it was merged; no second generation.
    let root = scratch("crash");
    let writer = SpoolWriter::new(&root, 33, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-A", Code::Match, 1)],
        )
        .unwrap();
    let spool_id = id(&root, 33);
    let mut log = spool::read_log(&spool_id.log_path(&root)).unwrap();
    // As the merger writes it: what it read, then the day's intent.
    spool::append_log(
        &spool_id.log_path(&root),
        &mut log,
        LogRecord {
            day: 1,
            generation: 0,
            state: LogState::Seen,
            exchange: 0,
        },
    )
    .unwrap();
    spool::append_log(
        &spool_id.log_path(&root),
        &mut log,
        LogRecord {
            day: on.days_from_epoch(),
            generation: 0,
            state: LogState::Intent,
            exchange: 1,
        },
    )
    .unwrap();
    let series = verdict::Series {
        root: &root,
        feed: Vendor::Zerodha,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
    };
    let generation = Generation {
        day: store_day(on).unwrap(),
        run: 33,
        written_at: 1,
        verifier: VERIFIER,
        mode: Mode::ZerodhaMerge,
        rule_set: 0,
        origin: [0; 16],
        day_block: DayBlock::of(Code::Match),
        day_details: Vec::new(),
        entries: vec![Entry {
            key: Key::instrument("NSE-INDEX-A").unwrap(),
            facts: Facts::of(Code::Match),
            details: Vec::new(),
        }],
        carry: Carry::Others,
    };
    verdict::commit(&series, &generation).unwrap();
    drop(writer);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!((outcome.days_recovered, outcome.generations), (1, 0));
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
        1,
        "no duplicate"
    );

    // (5) A crash after the intent and before the commit: merged again, once.
    let root = scratch("intent");
    let writer = SpoolWriter::new(&root, 34, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-A", Code::Match, 1)],
        )
        .unwrap();
    let spool_id = id(&root, 34);
    let mut log = spool::read_log(&spool_id.log_path(&root)).unwrap();
    // As the merger writes it: what it read, then the day's intent.
    spool::append_log(
        &spool_id.log_path(&root),
        &mut log,
        LogRecord {
            day: 1,
            generation: 0,
            state: LogState::Seen,
            exchange: 0,
        },
    )
    .unwrap();
    spool::append_log(
        &spool_id.log_path(&root),
        &mut log,
        LogRecord {
            day: on.days_from_epoch(),
            generation: 0,
            state: LogState::Intent,
            exchange: 1,
        },
    )
    .unwrap();
    drop(writer);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!((outcome.days_recovered, outcome.generations), (0, 1));
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
        1
    );
}

/// RA-319: the merge log's own torn tail is written over, and a bad record
/// before its last refuses the spool by name.
#[test]
fn a_damaged_merge_log_is_refused_and_its_torn_tail_written_over() {
    let root = scratch("log");
    let path = root.join("2024-04.vsp.merged");
    let mut log = spool::read_log(&path).unwrap();
    assert_eq!(log, spool::LogRead::default(), "an absent log is empty");
    let record = |n: u32| LogRecord {
        day: n,
        generation: n,
        state: LogState::Committed,
        exchange: 1,
    };
    spool::append_log(&path, &mut log, record(1)).unwrap();
    append_raw(&path, &[7; 9]);
    let mut read = spool::read_log(&path).unwrap();
    assert_eq!(
        (read.records.len(), read.next),
        (1, 16),
        "a torn tail is ignored"
    );
    spool::append_log(&path, &mut read, record(2)).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().len(),
        32,
        "and written over"
    );
    spool::append_log(&path, &mut read, record(3)).unwrap();
    flip(&path, 47);
    let read = spool::read_log(&path).unwrap();
    assert_eq!(read.records.len(), 2, "an unsynced last record is ignored");
    assert_eq!(read.next, 32);
    flip(&path, 47);
    flip(&path, 3);
    assert!(matches!(
        spool::read_log(&path),
        Err(SpoolError::LogDamaged { record: 0, .. })
    ));
    // A refused spool's log line is a refusal, never a day.
    let refused = LogRecord {
        day: 0,
        generation: 0,
        state: LogState::Refused,
        exchange: 0,
    };
    assert_eq!(LogRecord::decode(&refused.encode()), Some(refused));
    let mut bad = refused.encode();
    bad[10] = 1;
    assert_eq!(LogRecord::decode(&bad), None, "reserved bytes are zero");
}

/// critic IA-9-2: two threads of one run appending to one spool file never
/// interleave: every record reads back whole.
#[test]
fn two_tasks_of_one_run_append_to_one_spool_whole() {
    let root = scratch("threads");
    let on = regular();
    let writer = std::sync::Arc::new(SpoolWriter::new(&root, 51, Mode::ZerodhaMerge, 1));
    let handles: Vec<_> = (0..2)
        .map(|thread| {
            let writer = std::sync::Arc::clone(&writer);
            std::thread::spawn(move || {
                for n in 0..200 {
                    let mut record =
                        spooled(on, &format!("NSE-INDEX-T{thread}N{n}"), Code::Match, n);
                    record.details.push(Detail {
                        kind: 15,
                        payload: vec![1, 2, 3],
                    });
                    writer
                        .append(Segment::Index, Timeframe::MINUTE_1, &[record])
                        .unwrap();
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
    let id = SpoolId {
        run: 51,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    let read = spool::read_spool(&id.path(&root), &id).unwrap();
    assert_eq!(read.records.len(), 400);
    assert!(read.records.iter().all(|r| r.details.len() == 1));
    assert_eq!(read.torn, None);
}

/// The spool's extreme inputs: an unborn file, a header of another run, a
/// link at the name, a detail past the cap, a day outside the month, a
/// month and a year boundary, and stray Finder names in the tree.
#[test]
fn the_spools_extreme_inputs_are_named() {
    let root = scratch("extreme");
    let on = regular();
    let id = SpoolId {
        run: 61,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    let path = id.path(&root);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    // Unborn: zero bytes, then all-zero header bytes; both read as empty, and
    // the next append writes the header again.
    std::fs::write(&path, b"").unwrap();
    assert_eq!(spool::read_spool(&path, &id).unwrap().records.len(), 0);
    std::fs::write(&path, [0_u8; 64]).unwrap();
    assert!(spool::read_spool(&path, &id).unwrap().header.is_none());
    let writer = SpoolWriter::new(&root, 61, Mode::ZerodhaCold, 9);
    let wrote = writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-A", Code::Match, 0)],
        )
        .unwrap();
    assert_eq!((wrote.records, wrote.born), (1, 1));
    let read = spool::read_spool(&path, &id).unwrap();
    assert_eq!(read.header.unwrap().mode, Mode::ZerodhaCold);
    assert_eq!(read.header.unwrap().created, 9);
    // Another run's writer may not append to this spool.
    let other_root = scratch("foreign");
    let foreign_id = SpoolId { run: 62, ..id };
    let foreign_path = foreign_id.path(&other_root);
    std::fs::create_dir_all(foreign_path.parent().unwrap()).unwrap();
    std::fs::copy(&path, &foreign_path).unwrap();
    let thief = SpoolWriter::new(&other_root, 62, Mode::ZerodhaMerge, 1);
    assert!(matches!(
        thief.append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-A", Code::Match, 0)]
        ),
        Err(SpoolError::Header { .. })
    ));
    assert!(matches!(
        spool::read_spool(&foreign_path, &foreign_id),
        Err(SpoolError::Header { .. })
    ));
    // A link at a spool's name is never followed.
    let linked_root = scratch("link");
    let linked = SpoolId { run: 63, ..id }.path(&linked_root);
    std::fs::create_dir_all(linked.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&path, &linked).unwrap();
    let link_writer = SpoolWriter::new(&linked_root, 63, Mode::ZerodhaMerge, 1);
    assert!(matches!(
        link_writer.append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-A", Code::Match, 0)]
        ),
        Err(SpoolError::NotRegular { .. })
    ));
    // A detail past 128 bytes, and a run past u32::MAX, are refused whole.
    let mut big = spooled(on, "NSE-INDEX-A", Code::Match, 0);
    big.details.push(Detail {
        kind: 4,
        payload: vec![0; spool::DETAIL_PAYLOAD + 1],
    });
    assert!(matches!(
        writer.append(Segment::Index, Timeframe::MINUTE_1, &[big]),
        Err(SpoolError::Record { .. })
    ));
    let mut fits = spooled(on, "NSE-INDEX-A", Code::Match, 0);
    fits.details.push(Detail {
        kind: 4,
        payload: vec![9; spool::DETAIL_PAYLOAD],
    });
    writer
        .append(Segment::Index, Timeframe::MINUTE_1, &[fits])
        .unwrap();
    assert!(
        spool::encode_record(
            &spooled(on, "NSE-INDEX-A", Code::Match, 0),
            u64::from(u32::MAX) + 1
        )
        .is_err()
    );
    let mut flagged = spooled(on, "NSE-INDEX-A", Code::Match, 0);
    flagged.facts.flags = 1 << 40;
    assert!(
        spool::encode_record(&flagged, 1).is_err(),
        "a bit the format does not name"
    );
    // A window over a month and a year end: one spool per month.
    let eve = day(2024, 12, 31);
    let new_year = day(2025, 1, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[
                spooled(eve, "NSE-INDEX-Y", Code::Match, 1),
                spooled(new_year, "NSE-INDEX-Y", Code::Match, 1),
            ],
        )
        .unwrap();
    // Finder's names in the tree are stray: counted, never read, never removed.
    let tf_dir = path.parent().unwrap();
    for name in [".DS_Store", "._2024-04.vsp", "Icon\r"] {
        std::fs::write(tf_dir.join(name), b"x").unwrap();
    }
    std::fs::create_dir_all(root.join(spool::SPOOL_DIR).join(".Spotlight-V100")).unwrap();
    std::fs::create_dir_all(root.join(spool::SPOOL_DIR).join(".fseventsd")).unwrap();
    let waiting = spool::waiting(&root).unwrap();
    assert_eq!(waiting.spools.len(), 3, "April, December and January");
    assert_eq!(waiting.stray.len(), 5);
    drop(writer);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!(outcome.spools_merged, 3);
    assert_eq!(outcome.stray, 5);
    assert!(
        tf_dir.join(".DS_Store").exists(),
        "a stray name is never removed"
    );
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, eve),
        1
    );
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, new_year),
        1
    );
    // A record whose day is outside its spool's month is damage.
    let mut bytes = spool::encode_header(&id, Mode::ZerodhaMerge, 1).to_vec();
    bytes.extend(spool::encode_record(&spooled(eve, "NSE-INDEX-A", Code::Match, 0), 61).unwrap());
    bytes.extend(spool::encode_record(&spooled(on, "NSE-INDEX-B", Code::Match, 0), 61).unwrap());
    let odd_root = scratch("odd");
    let odd = id.path(&odd_root);
    std::fs::create_dir_all(odd.parent().unwrap()).unwrap();
    std::fs::write(&odd, &bytes).unwrap();
    assert!(matches!(
        spool::read_spool(&odd, &id),
        Err(SpoolError::Damaged { record: 0, .. })
    ));
}

/// A stop asked between days ends the merge with the spool still waiting
/// and its merged days on record; the next merge finishes it, once.
#[test]
fn a_stopped_merge_leaves_its_spool_for_the_next() {
    let root = scratch("stop");
    let writer = SpoolWriter::new(&root, 71, Mode::ZerodhaMerge, 1);
    let days = [regular(), day(2024, 4, 24), day(2024, 4, 25)];
    for on in days {
        writer
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(on, "NSE-INDEX-A", Code::Match, 1)],
            )
            .unwrap();
    }
    let stop = StopControl::tripping_at(root.join("verify/merge.stop"), 71, 3);
    drop(writer);
    let stopped = merge_all(&root, None, Some(&stop)).unwrap();
    assert!(stopped.stopped);
    assert!(stopped.generations < 3);
    assert_eq!(stopped.waiting, 1);
    let rest = merge_all(&root, None, None).unwrap();
    assert_eq!(rest.generations + stopped.generations, 3, "every day once");
    assert_eq!(rest.days_already, stopped.generations);
    for on in days {
        assert_eq!(
            year_generation(&root, Segment::Index, Timeframe::MINUTE_1, on),
            1
        );
    }
}

/// A month file that refuses to open is `store_unreadable` for its days,
/// never skipped.
#[test]
fn a_month_that_refuses_to_open_is_named() {
    let on = regular();
    let inputs = DayInputs {
        minutes_refused: Some("crc mismatch"),
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(
        (record.facts.code, reason_word(&record)),
        (Code::StoreUnreadable, "month_refused")
    );
}

/// The record of `rung` among `records`, as (code, reason word).
fn rung_said(records: &[RungRecord], rung: Timeframe) -> Option<(Code, &'static str)> {
    let mut found = None;
    for made in records {
        if made.timeframe == rung {
            found = Some((made.record.facts.code, reason_word(&made.record)));
        }
    }
    found
}

/// A RUNG WHOSE MONTH WOULD NOT OPEN IS SAID, NEVER READ AS A MONTH NOT HELD
/// (design §6.3, "A month a writer holds"): a derived month a writer held
/// past the wait is `not_compared (month_busy)` where it read `missing_in_store
/// (rung_day_absent)` before; a 1day month that refused is `store_unreadable
/// (month_refused)` though nothing else says the rung is held; the 1min
/// rung's refusal splits the same way. A month truly not held still owes
/// its days (RA-267).
#[test]
fn a_rung_whose_month_would_not_open_is_said_never_read_as_not_held() {
    let on = regular();
    let minutes = full_day(on, &[]);
    let mut refused = HashMap::new();
    refused.insert(Timeframe::MINUTE_5, true);
    refused.insert(Timeframe::DAY_1, false);
    let inputs = DayInputs {
        minutes: &minutes,
        derived: vec![
            (Timeframe::MINUTE_5, None, Some(RULES_D3321)),
            (Timeframe::MINUTE_15, None, Some(RULES_D3321)),
        ],
        covered: true,
        refused: Some(&refused),
        ..DayInputs::default()
    };
    let records = verify_day(&index(), on, &inputs);
    assert_eq!(
        rung_said(&records, Timeframe::MINUTE_5),
        Some((Code::NotCompared, "month_busy")),
        "a busy derived month is never missing"
    );
    assert_eq!(
        rung_said(&records, Timeframe::MINUTE_15),
        Some((Code::MissingInStore, "rung_day_absent")),
        "a derived month NOT held still owes the day (RA-267)"
    );
    assert_eq!(
        rung_said(&records, Timeframe::DAY_1),
        Some((Code::StoreUnreadable, "month_refused")),
        "a 1day month that refused gets its record"
    );
    // The same day with every month opened: no 1day record (none is held),
    // and the 5min rung owes its day like the 15min one.
    let opened = DayInputs {
        refused: None,
        ..inputs.clone()
    };
    let records = verify_day(&index(), on, &opened);
    assert_eq!(rung_said(&records, Timeframe::DAY_1), None);
    assert_eq!(
        rung_said(&records, Timeframe::MINUTE_5),
        Some((Code::MissingInStore, "rung_day_absent"))
    );
    // The 1min rung: a writer's hold and any other refusal, apart.
    for (busy, said) in [
        (true, (Code::NotCompared, "month_busy")),
        (false, (Code::StoreUnreadable, "month_refused")),
    ] {
        let mut one = HashMap::new();
        one.insert(Timeframe::MINUTE_1, busy);
        let inputs = DayInputs {
            minutes_refused: Some("the store's words"),
            covered: true,
            refused: Some(&one),
            ..DayInputs::default()
        };
        let record = minute_record(&index(), on, &inputs);
        assert_eq!((record.facts.code, reason_word(&record)), said, "{busy}");
    }
}

/// A MONTH A WRITER HOLDS (design §6.3; round 4 SF-4, F67; FM-167's
/// premise): a pull appending a month holds its writer's lock, so the
/// verifier's shared open meets `Locked`. With no wait it is busy at once;
/// a writer that lets go within the wait is waited for; past the wait the
/// cold sweep's records of that rung read `not_compared (month_busy)`, the
/// month is said apart (`busy`, never a fault in `refused`), and the window
/// path says the same instead of losing the whole window.
#[test]
fn a_month_a_writer_holds_is_not_compared_never_missing() {
    let on = regular();
    let series = index();
    let month = on.year_month().unwrap();
    let root = scratch("busy");
    let minutes = full_day(on, &[]);
    write_month(&root, &series, Timeframe::MINUTE_1, &minutes);
    let (buckets, _) = refold(&series, on.days_from_epoch(), &minutes, Timeframe::MINUTE_5);
    let folded: Vec<Bar> = buckets.iter().map(|b| b.bar).collect();
    write_month(&root, &series, Timeframe::MINUTE_5, &folded);
    let five = StorePath::new(series.parts(Timeframe::MINUTE_5, month)).unwrap();
    let hold = || BarFile::open_or_create(&root, five, repair_plan::symbol_id(&series.symbol));

    // (1) No wait: busy at once, never "not held", never another refusal.
    let writer = hold().unwrap();
    let refused = open_month_within(
        &root,
        series.parts(Timeframe::MINUTE_5, month),
        std::time::Duration::ZERO,
    )
    .err()
    .expect("a month a writer holds does not open");
    assert!(refused.busy, "{refused:?}");
    assert!(refused.why.starts_with("month_busy"), "{}", refused.why);
    // (1b) The series' first 1day month a pull is appending (a series listed
    // this month): its first day bar is busy, never a fault.
    let mut daily = minutes[0];
    daily.ts_micros = utc_s(on, 0) * 1_000_000;
    write_month(&root, &series, Timeframe::DAY_1, &[daily]);
    let day_month = StorePath::new(series.parts(Timeframe::DAY_1, month)).unwrap();
    let day_writer =
        BarFile::open_or_create(&root, day_month, repair_plan::symbol_id(&series.symbol)).unwrap();
    let first = first_served_day(&root, &series).expect_err("a busy first month");
    assert!(first.busy, "{first:?}");
    drop(day_writer);
    assert_eq!(first_served_day(&root, &series).unwrap(), Some(on));

    // (2) Within the wait: a writer that lets go after 150 ms is waited for.
    let opened = std::thread::scope(|scope| {
        let reader = scope.spawn(|| {
            open_month_within(
                &root,
                series.parts(Timeframe::MINUTE_5, month),
                std::time::Duration::from_secs(2),
            )
            .map(|held| held.map(|m| m.bars))
        });
        std::thread::sleep(std::time::Duration::from_millis(150));
        drop(writer);
        reader.join().unwrap()
    });
    assert_eq!(
        opened,
        Ok(Some(folded.len() as u64)),
        "waited for, then read whole"
    );

    // (3) The cold sweep beside a writer that never lets go.
    let writer = hold().unwrap();
    let spool_writer = SpoolWriter::new(&root, 91, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let report = cold_on(&root, &request, &spool_writer, None, 1).unwrap();
    assert_eq!(report.busy.len(), 1, "{:?}", report.busy);
    assert!(
        report.busy[0].contains("5min") && report.busy[0].contains("month_busy"),
        "{:?}",
        report.busy
    );
    assert!(
        report.refused.is_empty(),
        "a writer's hold is not a fault: {:?}",
        report.refused
    );
    // (4) The window path beside the same writer: records, not an error.
    let rows: Vec<RawRow> = minutes.iter().map(row_of).collect();
    let window = WindowInput {
        series: &series,
        rung: Timeframe::MINUTE_1,
        rows: &rows,
        asked: whole_day(on),
        receipt: 1,
        refused: &[],
        rows_refused: 0,
        first_day_bar: None,
        shape: &Shape::default(),
    };
    let window_writer = SpoolWriter::new(&root, 92, Mode::ZerodhaMerge, 1);
    let wrote = verify_window(&root, &window, &GapLedger::default(), &window_writer).unwrap();
    assert_eq!(
        wrote.records, 8,
        "1min and seven derived rungs, the busy one said"
    );
    drop(writer);
    drop((spool_writer, window_writer));
    merge_all(&root, None, None).unwrap();
    let said = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_5,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        (said.facts.code, words::reason_word(said.facts.reason)),
        (Code::NotCompared, Some("month_busy")),
        "never missing_in_store, never store_unreadable"
    );
    let minute = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        minute.facts.code,
        Code::Match,
        "the window's 1min is compared"
    );
}

/// THE MERGE LOG ANSWERS A DAY BY ONE PROBE (C-ZV-03): the last record of
/// each (exchange, day) and whether a refusal is on record are kept as each
/// record is learned, the same whether appended or read back; and a log past
/// `MAX_LOG_BYTES` is refused by name before a byte is read, never read in
/// part (a log of exactly the cap is read, and its first bad record named).
#[test]
fn the_merge_log_answers_a_day_by_one_probe_and_refuses_a_log_past_its_cap() {
    let root = scratch("log-index");
    let path = root.join("2026-09.vsp.merged");
    let at = |day: u32, generation: u32, state: LogState| LogRecord {
        day,
        generation,
        state,
        exchange: 1,
    };
    let mut log = spool::read_log(&path).unwrap();
    for record in [
        at(10, 3, LogState::Intent),
        at(10, 4, LogState::Committed),
        at(11, 4, LogState::Intent),
    ] {
        spool::append_log(&path, &mut log, record).unwrap();
    }
    assert_eq!(log.last_of(1, 10), Some(at(10, 4, LogState::Committed)));
    assert_eq!(log.last_of(1, 11), Some(at(11, 4, LogState::Intent)));
    assert_eq!(
        log.last_of(2, 10),
        None,
        "another exchange's day is its own"
    );
    assert_eq!(log.last_of(1, 12), None);
    assert!(!log.refused());
    spool::append_log(
        &path,
        &mut log,
        LogRecord {
            day: 0,
            generation: 0,
            state: LogState::Refused,
            exchange: 0,
        },
    )
    .unwrap();
    assert!(log.refused());
    assert_eq!(log.last_of(0, 0), None, "a refusal is no day's record");
    assert_eq!(
        spool::read_log(&path).unwrap(),
        log,
        "read back, the same records and the same answers"
    );

    // A LOG PAST THE CAP (a sparse file: nothing is written but its length).
    let oversized = root.join("oversized.vsp.merged");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(spool::MAX_LOG_BYTES + 16)
        .unwrap();
    match spool::read_log(&oversized) {
        Err(SpoolError::LogDamaged { record, why, .. }) => {
            assert_eq!(record, spool::MAX_LOG_BYTES / spool::LOG_RECORD_LEN);
            assert!(why.contains("64 MiB"), "{why}");
        }
        other => panic!("a log past the cap is refused by name: {other:?}"),
    }
    // A LOG OF EXACTLY THE CAP is read: its first record (zeros) fails its
    // CRC and, not being the last, refuses it as damaged.
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(spool::MAX_LOG_BYTES)
        .unwrap();
    match spool::read_log(&oversized) {
        Err(SpoolError::LogDamaged { record: 0, why, .. }) => {
            assert_eq!(why, "fails its CRC");
        }
        other => panic!("a log of the cap is read: {other:?}"),
    }
}

/// The window path (IA-10c's hook): one landed window's days verified and
/// spooled, its derived rungs with it.
#[test]
fn a_landed_window_is_verified_and_spooled() {
    let root = scratch("window");
    let on = regular();
    let minutes = full_day(on, &[]);
    write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
    let rows: Vec<RawRow> = minutes.iter().map(row_of).collect();
    let writer = SpoolWriter::new(&root, 81, Mode::ZerodhaMerge, 1);
    let series = index();
    let window = WindowInput {
        series: &series,
        rung: Timeframe::MINUTE_1,
        rows: &rows,
        asked: whole_day(on),
        receipt: 1,
        refused: &[],
        rows_refused: 0,
        first_day_bar: None,
        shape: &Shape::default(),
    };
    let ledger = GapLedger::default();
    let wrote = verify_window(&root, &window, &ledger, &writer).unwrap();
    assert_eq!(wrote.records, 8, "1min and seven derived rungs");
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(record.facts.code, Code::Match);
    assert_eq!(record.facts.rows, 375);
    assert!(
        crate::alerts::window(&root).map_or(true, |w| w
            .alerts
            .iter()
            .all(|a| a.record.kind != VENDOR_SHAPE_CHANGED)),
        "an unaltered answer puts no shape alert"
    );

    // THE SAME WINDOW WHOSE BODY CARRIED A NEW FIELD (RA-318, the hook's
    // half): its 1min record names the shape and one alert is put.
    let root = scratch("window-shape");
    write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
    let writer = SpoolWriter::new(&root, 82, Mode::ZerodhaMerge, 1);
    let changed = answer_shape("{\"status\":\"x\",\"data\":{\"candles\":[]},\"meta\":1}");
    let altered = WindowInput {
        shape: &changed,
        ..window
    };
    verify_window(&root, &altered, &ledger, &writer).unwrap();
    let id = SpoolId {
        run: 82,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    let read = spool::read_spool(&id.path(&root), &id).unwrap();
    let payloads: Vec<&[u8]> = read
        .records
        .iter()
        .flat_map(|r| r.details.iter())
        .filter(|d| d.kind == words::DETAIL_VENDOR_SHAPE)
        .map(|d| d.payload.as_slice())
        .collect();
    assert_eq!(payloads, vec![b"\x01\0\0\0meta".as_slice()]);
    let alerts = crate::alerts::window(&root).unwrap();
    assert_eq!(
        alerts
            .alerts
            .iter()
            .filter(|a| a.record.kind == VENDOR_SHAPE_CHANGED)
            .count(),
        1
    );
}

/// Every reason a record can carry is a number of the one table, and
/// round-trips through the verdict store (D-4809).
#[test]
fn every_reason_this_verifier_writes_is_in_the_table() {
    for word in [
        "unexplained",
        "index_bucket_incomplete",
        "pre_ledger",
        "landed_from_store",
        "neither_vendor_has_it",
        "answer_short",
        "out_of_order",
        "duplicate",
        "month_refused",
        "pack_full",
        "storage_full",
        "refused_answer",
        "duplicate_body",
        "withheld_unmeasured_session",
        "calendar_unverified",
        "calendar_past_record",
    ] {
        assert_ne!(reason_of(word), 0, "{word}");
    }
}

// -------------------------------------------------------------- helpers --

fn append_raw(path: &Path, bytes: &[u8]) {
    use std::io::Write as _;
    let mut file = std::fs::OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(bytes).unwrap();
}

fn flip(path: &Path, at: u64) {
    use std::os::unix::fs::FileExt as _;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    let mut byte = [0_u8];
    file.read_exact_at(&mut byte, at).unwrap();
    byte[0] ^= 0xFF;
    file.write_all_at(&byte, at).unwrap();
}

// ------------------------------------------- kept answers, end to end --

/// An invented Kite-shaped body of `bars` (rupees with two decimals, the
/// stamp with its zone), as `http::decode_body` reads one.
fn kite_body(bars: &[Bar]) -> String {
    let mut rows = Vec::with_capacity(bars.len());
    for bar in bars {
        let local = bar.ts_micros / 1_000_000 + 19_800;
        let (number, rest) = (local.div_euclid(86_400), local.rem_euclid(86_400));
        let on = Day::from_days(u32::try_from(number).unwrap()).unwrap();
        let rupees = |paisa: i64| format!("{}.{:02}", paisa / 100, paisa % 100);
        rows.push(format!(
            "[\"{on}T{:02}:{:02}:00+0530\",{},{},{},{},{}]",
            rest / 3_600,
            rest % 3_600 / 60,
            rupees(bar.open),
            rupees(bar.high),
            rupees(bar.low),
            rupees(bar.close),
            bar.volume
        ));
    }
    format!(
        "{{\"status\":\"success\",\"data\":{{\"candles\":[{}]}}}}",
        rows.join(",")
    )
}

/// Keeps `bars` as one answer of `on` for `ZZIDX` at 1min, as IA-8's keep
/// does after a landing; the pack's month.
fn keep(root: &Path, on: Day, bars: &[Bar]) -> YearMonth {
    keep_answer(
        root,
        on,
        kite_body(bars),
        bars,
        u32::try_from(bars.len()).unwrap(),
        response_keep::Door::FromWindow,
    )
}

/// Keeps `body` as one answer of `on` for `ZZIDX` at 1min whose landing
/// decoded `rows_decoded` rows and landed `landed`, through `door`.
fn keep_answer(
    root: &Path,
    on: Day,
    body: String,
    landed: &[Bar],
    rows_decoded: u32,
    door: response_keep::Door,
) -> YearMonth {
    let key = brutex_core::instrument::InstrumentKey::index(Exchange::Nse, "ZZIDX").unwrap();
    let pack = response_keep::Pack {
        vendor: Vendor::Zerodha,
        key: &key,
        timeframe: Timeframe::MINUTE_1,
    };
    let bars = landed;
    let answered = response_keep::Answered::of(crate::fetch::RawWindow::default(), body, 200);
    let landing = response_keep::Landing {
        window: crate::session::Window::new(on, on).unwrap(),
        first_stamp: bars.first().map_or(0, |b| b.ts_micros / 1_000_000),
        last_stamp: bars.last().map_or(0, |b| b.ts_micros / 1_000_000),
        rows_decoded,
        rows_landed: u32::try_from(bars.len()).unwrap(),
        rows_refused: 0,
        refused: &[],
        run: 5,
        rung: crate::vendor::Granularity::Minute1 as u8,
        rules_version: 1,
        door,
    };
    response_keep::append(
        root,
        pack,
        response_keep::Offered::Answer(&answered),
        &landing,
    )
    .unwrap();
    on.year_month().unwrap()
}

/// The whole road a pull will take once IA-10c hooks it: a landed day, its
/// answer kept by IA-8's keep, then the cold sweep decodes the kept body and
/// the store reads `match`; one price changed in the store reads
/// `value_mismatch` at that minute.
#[test]
fn a_kept_answer_is_decoded_and_matched_minute_by_minute() {
    let root = scratch("kept");
    let on = regular();
    let minutes = full_day(on, &[]);
    write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
    keep(&root, on, &minutes);
    let writer = SpoolWriter::new(&root, 91, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(record.facts.code, Code::Match, "{record:?}");
    assert_eq!(
        (record.facts.source_units, record.facts.store_units),
        (375, 375)
    );

    let root = scratch("kept-diff");
    let mut held = minutes.clone();
    held[100].low -= 3;
    write_month(&root, &index(), Timeframe::MINUTE_1, &held);
    keep(&root, on, &minutes);
    let writer = SpoolWriter::new(&root, 92, Mode::ZerodhaCold, 1);
    cold(&root, &request, &writer, None).unwrap();
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(
        (
            record.facts.code,
            record.facts.value_diff,
            record.facts.first_diff
        ),
        (Code::ValueMismatch, 1, 655)
    );
}

/// §8 Z16, this package's half: a window landed through the repair door
/// (`ingest::repair_window`, receipt `door=2`) is kept and verified exactly
/// as a `from_window` window is: its kept answer is read minute by minute,
/// so the repaired day reads `match`, and a store that still differs from
/// the repaired answer reads `value_mismatch` at that minute, never an
/// unverified pass. (IA-10c wires the keep on both doors.)
#[test]
fn a_repaired_window_is_kept_and_verified() {
    let on = regular();
    let minutes = full_day(on, &[]);
    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    let mut stale = minutes.clone();
    stale[7].close += 1;
    for (tag, held, code, first) in [
        ("repaired-match", &minutes, Code::Match, verdict::NONE),
        ("repaired-differs", &stale, Code::ValueMismatch, 562),
    ] {
        let root = scratch(tag);
        write_month(&root, &index(), Timeframe::MINUTE_1, held);
        keep_answer(
            &root,
            on,
            kite_body(&minutes),
            &minutes,
            375,
            response_keep::Door::RepairWindow,
        );
        let writer = SpoolWriter::new(&root, 96, Mode::ZerodhaCold, 1);
        let report = cold(&root, &request, &writer, None).unwrap();
        assert!(report.refused.is_empty(), "{:?}", report.refused);
        drop(writer);
        merge_all(&root, None, None).unwrap();
        let record = the_record(
            &root,
            Segment::Index,
            Timeframe::MINUTE_1,
            on,
            "NSE-INDEX-ZZIDX",
        );
        assert_eq!(
            (record.facts.code, record.facts.first_diff),
            (code, first),
            "{tag}: {record:?}"
        );
        assert_eq!(record.facts.source_units, 375, "{tag}: the answer was read");
    }
}

/// §8 Z6 (DB-G46): a candle whose four prices cannot be a bar is skipped by
/// the oracle exactly as by the landing, because both decode the answer with
/// the one `http::decode_body` step, and the skip is COUNTED: the receipt's
/// decoded rows are compared with the oracle's.
///
/// The impossible candle shares 10:55's stamp with a good one. Dropped
/// before any row rule runs, it never makes the stamp a conflict (which
/// would refuse BOTH candles), so the good candle lands and the day matches.
/// A receipt whose landing decoded 376 rows (a decoder that kept the
/// impossible candle) reads `quality_differs` with sub-kind 2, (376, 375).
#[test]
fn an_impossible_candle_is_skipped_by_the_oracle_as_by_the_decoder() {
    let on = regular();
    let minutes = full_day(on, &[]);
    let good = minutes[100];
    let mut answered = minutes.clone();
    answered.insert(
        101,
        Bar {
            high: good.low - 7,
            ..good
        },
    );
    let body = kite_body(&answered);
    let crate::vendor::Transport::Http(spec) = &crate::vendor::Feed::Zerodha.descriptor().transport
    else {
        panic!("Zerodha is an HTTP feed");
    };
    let window = crate::http::decode_body(&body, spec, crate::vendor::Listing::Index).unwrap();
    assert_eq!(window.rows.len(), 375, "the decoder dropped the one candle");
    let made = oracle(
        &window.rows,
        spec.timestamps,
        Timeframe::MINUTE_1,
        Segment::Index,
        (utc_s(on, 0), utc_s(on, 1_439)),
    );
    let answer = &made.days[&on.days_from_epoch()];
    assert_eq!(
        (made.judged, answer.dup),
        (0, 0),
        "no conflict, no duplicate"
    );
    assert_eq!(answer.bars.get(&655), Some(&good), "the good candle lands");

    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    for (tag, rows_decoded, code, quality) in [
        ("impossible-same", 375, Code::Match, None),
        (
            "impossible-kept",
            376,
            Code::Match,
            Some((376_u64, 375_u64)),
        ),
    ] {
        let root = scratch(tag);
        write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
        keep_answer(
            &root,
            on,
            body.clone(),
            &minutes,
            rows_decoded,
            response_keep::Door::FromWindow,
        );
        let writer = SpoolWriter::new(&root, 94, Mode::ZerodhaCold, 1);
        let report = cold(&root, &request, &writer, None).unwrap();
        assert!(report.refused.is_empty(), "{:?}", report.refused);
        // The record as spooled, with its details (the merge carries them
        // into the day image as they are).
        let id = SpoolId {
            run: 94,
            segment: Segment::Index,
            timeframe: Timeframe::MINUTE_1,
            month: on.year_month().unwrap(),
        };
        let read = spool::read_spool(&id.path(&root), &id).unwrap();
        let record = read
            .records
            .iter()
            .find(|r| r.day == on && r.key.text() == "NSE-INDEX-ZZIDX")
            .unwrap();
        assert_eq!(record.facts.code, code, "{tag}: {record:?}");
        assert_eq!(
            (record.facts.source_units, record.facts.store_units),
            (375, 375)
        );
        let found: Vec<(u16, u64, u64)> = record
            .details
            .iter()
            .filter(|d| d.kind == words::DETAIL_QUALITY_DIFFERS)
            .map(|d| {
                (
                    u16::from_le_bytes(d.payload[0..2].try_into().unwrap()),
                    u64::from_le_bytes(d.payload[2..10].try_into().unwrap()),
                    u64::from_le_bytes(d.payload[10..18].try_into().unwrap()),
                )
            })
            .collect();
        match quality {
            None => {
                assert!(found.is_empty(), "{tag}: {found:?}");
                assert_eq!(record.facts.flags & flags::QUALITY_DIFFERS, 0);
            }
            Some((landing, oracle)) => {
                assert_eq!(found, [(2, landing, oracle)], "{tag}");
                assert_ne!(record.facts.flags & flags::QUALITY_DIFFERS, 0);
            }
        }
        // And the day image says the same once merged.
        drop(writer);
        merge_all(&root, None, None).unwrap();
        let merged = the_record(
            &root,
            Segment::Index,
            Timeframe::MINUTE_1,
            on,
            "NSE-INDEX-ZZIDX",
        );
        assert_eq!(merged.facts.code, code, "{tag}");
        assert_eq!(
            merged.facts.flags & flags::QUALITY_DIFFERS != 0,
            quality.is_some(),
            "{tag}"
        );
    }
}

/// A kept body that no longer matches its receipt (a flipped byte) is never
/// read as an answer: the day reads `no_response_kept (body_unreadable)`
/// with `body_not_kept`, and the run names the file.
#[test]
fn a_damaged_kept_body_is_named_and_never_read_as_an_answer() {
    let root = scratch("badbody");
    let on = regular();
    let minutes = full_day(on, &[]);
    write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
    let month = keep(&root, on, &minutes);
    let key = brutex_core::instrument::InstrumentKey::index(Exchange::Nse, "ZZIDX").unwrap();
    let dir = response_keep::pack_dir(
        &root,
        response_keep::Pack {
            vendor: Vendor::Zerodha,
            key: &key,
            timeframe: Timeframe::MINUTE_1,
        },
    )
    .unwrap();
    flip(&response_keep::names(&dir, month, 0).bodies, 10);
    let writer = SpoolWriter::new(&root, 93, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(report.refused.len(), 1, "{:?}", report.refused);
    assert!(
        report.refused[0].contains("receipt 0"),
        "{:?}",
        report.refused
    );
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_eq!(record.facts.code, Code::NoResponseKept);
    assert_eq!(
        words::reason_word(record.facts.reason),
        Some("body_unreadable")
    );
    assert_ne!(record.facts.flags & flags::BODY_NOT_KEPT, 0);
}

/// A commit the verdict store refuses (here a lost year index: a day image
/// with no index beside it) is named with an alert, and the spool waits for
/// the next merge with no refusal on its log: it is not damage.
#[test]
fn a_refused_commit_leaves_the_spool_waiting_not_refused() {
    let root = scratch("lost");
    let on = regular();
    let writer = SpoolWriter::new(&root, 95, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-A", Code::Match, 1)],
        )
        .unwrap();
    let series = verdict::Series {
        root: &root,
        feed: Vendor::Zerodha,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
    };
    let image = series.image_path(store_day(on).unwrap(), 1);
    std::fs::create_dir_all(image.parent().unwrap()).unwrap();
    std::fs::write(&image, b"an image whose index was lost").unwrap();
    drop(writer);
    let outcome = merge_all(&root, None, None).unwrap();
    assert_eq!(outcome.refused.len(), 1, "{outcome:?}");
    assert_eq!(outcome.generations, 0);
    assert_eq!(outcome.waiting, 1, "the spool waits for the next merge");
    let id = SpoolId {
        run: 95,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    assert!(
        !spool::read_log(&id.log_path(&root)).unwrap().refused(),
        "no refusal on its log"
    );
    let alerts = crate::alerts::window(&root).unwrap();
    assert_eq!(
        alerts.alerts.iter().filter(|a| a.record.kind == 11).count(),
        1
    );
    // The next merge tries again (and is refused again by the same lost index).
    let again = merge_all(&root, None, None).unwrap();
    assert_eq!((again.refused.len(), again.refused_earlier.len()), (1, 0));
}

/// The ledger a run keeps reads only what was appended since: a landing's
/// own lines are seen at O(new bytes); a torn last line is read again whole;
/// a ledger that shrank is read again from its start.
#[test]
fn the_gap_ledger_refreshes_with_what_was_appended() {
    let root = scratch("refresh");
    write_ledger(
        &root,
        "gap NSE-INDEX-ZZIDX 2024-04-23 v=1 why=vendor_empty run=1\n",
    );
    let mut ledger = GapLedger::read(&root).unwrap();
    assert_eq!(ledger.lines, 1);
    assert!(ledger.gap("NSE-INDEX-ZZIDX", day(2024, 4, 24)).is_none());
    append_raw(
        &root.join("audit").join("zerodha.gaps"),
        b"gap NSE-INDEX-ZZIDX 2024-04-24 v=1 why=refused_front run=2\ngap NSE-INDEX-ZZIDX 2024-04-25 v=1 why=refu",
    );
    ledger.refresh(&root).unwrap();
    assert_eq!(ledger.lines, 2, "the torn line is not counted yet");
    assert_eq!(
        words::reason_word(ledger.gap("NSE-INDEX-ZZIDX", day(2024, 4, 24)).unwrap().why),
        Some("refused_front")
    );
    assert!(
        ledger.faults.is_empty(),
        "a torn last line is not a fault yet"
    );
    append_raw(&root.join("audit").join("zerodha.gaps"), b"sed_day run=3\n");
    ledger.refresh(&root).unwrap();
    assert_eq!(ledger.lines, 3);
    assert_eq!(
        words::reason_word(ledger.gap("NSE-INDEX-ZZIDX", day(2024, 4, 25)).unwrap().why),
        Some("refused_day")
    );
    write_ledger(
        &root,
        "gap NSE-CASH-ZZSHARE 2024-04-23 v=1 why=stored_short run=4\n",
    );
    ledger.refresh(&root).unwrap();
    assert_eq!(ledger.lines, 1, "a ledger that shrank is read again whole");
    assert!(ledger.gap("NSE-INDEX-ZZIDX", regular()).is_none());
    assert!(ledger.gap("NSE-CASH-ZZSHARE", regular()).is_some());
}

/// CMP-18 through the cold sweep: a series' minutes stored before its first
/// served day bar read `store_without_source (before_first_day_bar)`; a day
/// before it that holds nothing is not owed by the series and gets no
/// record, never a hole.
#[test]
fn minutes_before_the_first_day_bar_are_another_securitys() {
    let root = scratch("listing");
    let early = day(2024, 4, 22);
    let listed = day(2024, 4, 25);
    let mut bars = full_day(early, &[]);
    bars.extend(full_day(listed, &[]));
    write_month(&root, &share(), Timeframe::MINUTE_1, &bars);
    let mut daily = bars[400];
    daily.ts_micros = utc_s(listed, 0) * 1_000_000;
    write_month(&root, &share(), Timeframe::DAY_1, &[daily]);
    assert_eq!(first_served_day(&root, &share()).unwrap(), Some(listed));
    let writer = SpoolWriter::new(&root, 97, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: early,
        to: listed,
        only: vec!["ZZSHARE".to_owned()],
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(
        report.days, 2,
        "22 April (minutes) and 25 April; 23 and 24 are before the listing and empty"
    );
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let foreign = the_record(
        &root,
        Segment::Cash,
        Timeframe::MINUTE_1,
        early,
        "NSE-CASH-ZZSHARE",
    );
    assert_eq!(foreign.facts.code, Code::StoreWithoutSource);
    assert_eq!(
        words::reason_word(foreign.facts.reason),
        Some("before_first_day_bar")
    );
    let own = the_record(
        &root,
        Segment::Cash,
        Timeframe::MINUTE_1,
        listed,
        "NSE-CASH-ZZSHARE",
    );
    assert_eq!(own.facts.code, Code::NoResponseKept);
}

/// CMP-18 on every rung (found on the real store: KFINTECH's minutes from
/// May 2018, before its first day bar of 29 December 2022, read
/// `missing_in_store (rung_day_absent)` on every derived rung). On a day
/// before the series' first served day bar the series owes nothing: a
/// derived rung holding no bar gets no record, one holding bars and a 1min
/// day the answer lacks a minute of read `store_without_source
/// (before_first_day_bar)` with their counts kept; its held 1day month owes
/// no day bar on it (no record, never `missing_in_store`); a derived bar
/// that disagrees with its own minutes keeps `value_mismatch`; and the first
/// served day is judged as any other day.
#[test]
fn a_derived_rung_of_another_securitys_day_is_never_missing() {
    let early = day(2024, 4, 22);
    let listed = day(2024, 4, 25);
    let minutes = full_day(early, &[]);
    let answers = [answer_of(early, &full_day(early, &[600]), 1)];
    // One 5min bar off any bucket start; one 3min bar at 09:15 whose price is
    // not its minutes' (invented prices, far from any real one).
    let five = [bar_at(early, 556, 7)];
    let three = [bar_at(early, 555, 7)];
    let inputs = DayInputs {
        minutes: &minutes,
        answers: &answers,
        covered: true,
        derived: vec![
            (Timeframe::MINUTE_2, Some(&[][..]), Some(RULES_D3321)),
            (Timeframe::MINUTE_3, Some(&three[..]), Some(RULES_D3321)),
            (Timeframe::MINUTE_5, Some(&five[..]), None),
            (Timeframe::MINUTE_10, None, None),
        ],
        first_day_bar: Some(listed),
        day_month_held: true,
        ..DayInputs::default()
    };
    let records = verify_day(&share(), early, &inputs);
    let rungs: Vec<Timeframe> = records.iter().map(|r| r.timeframe).collect();
    assert_eq!(
        rungs,
        [
            Timeframe::MINUTE_1,
            Timeframe::MINUTE_3,
            Timeframe::MINUTE_5
        ],
        "a rung holding no bar on another security's day owes none, 1day too"
    );
    let minute = &records[0].record.facts;
    assert_eq!(
        minute.code,
        Code::StoreWithoutSource,
        "never extra_in_store"
    );
    assert_eq!(
        words::reason_word(minute.reason),
        Some("before_first_day_bar")
    );
    assert_eq!(minute.extra, 1, "the count is kept as it was measured");
    let three_min = &records[1].record.facts;
    assert_eq!(
        three_min.code,
        Code::ValueMismatch,
        "a bar unlike its minutes"
    );
    let five_min = &records[2].record.facts;
    assert_eq!(five_min.code, Code::StoreWithoutSource);
    assert_eq!(
        words::reason_word(five_min.reason),
        Some("before_first_day_bar")
    );
    assert_eq!((five_min.extra, five_min.store_units), (1, 1));

    // The first served day is the series' own: a rung lacking it is missing.
    let minutes = full_day(listed, &[]);
    let inputs = DayInputs {
        minutes: &minutes,
        covered: true,
        derived: vec![(Timeframe::MINUTE_2, Some(&[][..]), Some(RULES_D3321))],
        first_day_bar: Some(listed),
        ..DayInputs::default()
    };
    let records = verify_day(&share(), listed, &inputs);
    let two_min = &records[1].record.facts;
    assert_eq!(records[1].timeframe, Timeframe::MINUTE_2);
    assert_eq!(two_min.code, Code::MissingInStore);
    assert_eq!(words::reason_word(two_min.reason), Some("rung_day_absent"));
}

// ------------------------------------------------------------- IA-ZV-12 --

/// IA-ZV-12 (RA-301): a pull that halted part-way (a dead token at 06:00, a
/// stop) never asked for the rest of the day; the gap ledger's `pull_halted`
/// line makes those minutes `not_compared (pull_halted)`, never a vendor hole
/// and never the store's own defect. Without the line, in a month the ledger
/// covers, the same minutes are `missing_in_store (unexplained)`. Before
/// RA-301 the word had no number, so the line was unreadable.
#[test]
fn a_halted_pull_is_not_compared_never_a_hole() {
    let on = regular();
    // Landed until 12:00 IST (minute 720); the pull halted before it asked
    // for the afternoon, so the kept answer's window ends at 11:59:59.
    let held: Vec<Bar> = full_day(on, &(720..930).collect::<Vec<_>>());
    let mut answer = answer_of(on, &held, 1);
    answer.asked = (utc_s(on, 0), utc_s(on, 720) - 1);
    let answers = [answer];
    let halted = GapLine {
        why: words::reason("pull_halted").expect("pull_halted has a number"),
        origin_store: false,
    };
    let inputs = DayInputs {
        minutes: &held,
        answers: &answers,
        gap: Some(halted),
        covered: true,
        ..DayInputs::default()
    };
    let record = minute_record(&index(), on, &inputs);
    assert_eq!(record.facts.code, Code::NotCompared);
    assert_eq!(reason_word(&record), "pull_halted");
    assert_eq!(
        record.facts.missing, 210,
        "the afternoon's minutes, counted"
    );
    let unlined = DayInputs {
        gap: None,
        ..inputs
    };
    let record = minute_record(&index(), on, &unlined);
    assert_eq!(record.facts.code, Code::MissingInStore);
    assert_eq!(reason_word(&record), "unexplained");
}

// ------------------------------------------------------- progress (§5.7) --

fn progress_at(path: &Path) -> crate::gdfl_run::Progress {
    let bytes = std::fs::read(path).expect("a progress file");
    crate::gdfl_run::Progress::decode(&bytes).expect("a whole v1 record")
}

fn position_of(table: &[&str], word: &str) -> u8 {
    u8::try_from(table.iter().position(|w| *w == word).unwrap()).unwrap()
}

/// Design §5.7: a cold run keeps `verify/zerodha.progress` in the GDFL
/// run's record (§37 v1), so one reader reads every run: its mode is the
/// runs ledger's `zerodha-verify`, its kind `zerodha`, its span the asked
/// days, and after its end it says `finished` with every series counted.
/// Before it, no verification run wrote a progress file at all.
#[test]
fn a_cold_run_keeps_its_progress_file() {
    let root = scratch("progress-cold");
    let on = regular();
    for n in 0..3 {
        let series = Series {
            exchange: Exchange::Nse,
            segment: Segment::Index,
            symbol: format!("ZZP{n}"),
            contract: None,
        };
        write_month(&root, &series, Timeframe::MINUTE_1, &full_day(on, &[700]));
    }
    let writer = SpoolWriter::new(&root, 81, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: on,
        only: Vec::new(),
    };
    let path = root.join(PROGRESS_COLD);
    let progress = crate::gdfl_run::ProgressFile::of_record(
        path.clone(),
        progress_record("zerodha-verify", 1, 81, 2, Some((on, on))),
    );
    let report = cold_observed(&root, &request, &writer, None, 2, Some(&progress)).unwrap();
    let running = progress_at(&path);
    assert_eq!(running.state, crate::gdfl_run::RunState::Running.code());
    assert_eq!(
        running.day_total, 3,
        "the series to check, forced at the begin"
    );
    progress_end(&progress, report.stopped, None);
    let ended = progress_at(&path);
    assert_eq!(progress.fault(), None);
    assert_eq!(
        ended.mode,
        position_of(&crate::gdfl_run::ledger::MODES, "zerodha-verify")
    );
    assert_eq!(
        ended.kind,
        position_of(&crate::gdfl_run::ledger::KINDS, "zerodha") + 1
    );
    assert_eq!(ended.state, crate::gdfl_run::RunState::Finished.code());
    assert_eq!((ended.run, ended.by, ended.workers), (81, 1, 2));
    assert_eq!((ended.from, ended.to), (20_240_423, 20_240_423));
    assert_eq!(ended.pid, std::process::id());
    assert_eq!((ended.day_index, ended.day_total), (3, 3));
    assert_eq!(
        (ended.files, ended.days_imported, ended.rows),
        (report.months, report.days, report.records)
    );
    assert!(report.records > 0);
    // A refused run says so, by its words, in a slot.
    progress_end(&progress, false, Some("the gap ledger would not read"));
    let refused = progress_at(&path);
    assert_eq!(refused.state, crate::gdfl_run::RunState::Refused.code());
    assert_eq!(
        refused.refusals.last().map(|r| r.why.as_str()),
        Some("the gap ledger would not read")
    );
}

/// Design §5.7: a merge keeps `verify/merge.progress` while it holds
/// `verify/merge.lock` and ends it before it lets go: `finished` with the
/// spool, its generations and records counted; `stopped` when a stop was
/// asked; and a merger that found the lock held writes no record at all, so
/// it can never write over the holder's.
#[test]
fn a_merge_keeps_its_progress_file_only_while_it_holds_the_lock() {
    let root = scratch("progress-merge");
    let writer = SpoolWriter::new(&root, 82, Mode::ZerodhaMerge, 1);
    let days = [regular(), day(2024, 4, 24), day(2024, 4, 25)];
    for on in days {
        writer
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(on, "NSE-INDEX-P", Code::Match, 1)],
            )
            .unwrap();
    }
    let path = root.join(PROGRESS_MERGE);
    let fresh = || {
        crate::gdfl_run::ProgressFile::of_record(
            path.clone(),
            progress_record("verify-merge", 1, 83, 1, None),
        )
    };
    // A merger that finds the lock held: `run_busy`, and no record.
    let held = crate::gdfl_run::locks::open_lock_file(&root, spool::MERGE_LOCK).unwrap();
    let lock = store::flock::Flock::try_lock(held, root.join(spool::MERGE_LOCK)).unwrap();
    let busy = fresh();
    assert!(matches!(
        merge_observed(&root, None, None, Some(&busy)),
        Err(MergeError::RunBusy { .. })
    ));
    assert!(
        !path.exists(),
        "a merger without the lock writes no progress"
    );
    drop(lock);
    // A stop after the first day: `stopped`.
    let stopping = fresh();
    let stop = StopControl::tripping_at(root.join("verify/merge.stop"), 83, 3);
    drop(writer);
    let stopped = merge_observed(&root, None, Some(&stop), Some(&stopping)).unwrap();
    assert!(stopped.stopped);
    let record = progress_at(&path);
    assert_eq!(record.state, crate::gdfl_run::RunState::Stopped.code());
    assert_eq!(
        record.mode,
        position_of(&crate::gdfl_run::ledger::MODES, "verify-merge")
    );
    assert_eq!(record.day_total, 1, "one spool waiting at the begin");
    assert_eq!(record.days_imported, stopped.generations);
    // The rest: `finished`, every day once.
    let finishing = fresh();
    let rest = merge_observed(&root, None, None, Some(&finishing)).unwrap();
    let record = progress_at(&path);
    assert_eq!(record.state, crate::gdfl_run::RunState::Finished.code());
    assert_eq!((record.day_index, record.day_total), (1, 1));
    assert_eq!(record.files, 1, "the spool merged whole");
    assert_eq!(record.days_imported, rest.generations);
    assert_eq!(record.rows, rest.records);
    assert_eq!(record.days_skipped, rest.days_already + rest.days_recovered);
    assert_eq!(rest.generations + stopped.generations, 3);
    assert_eq!(finishing.fault(), None);
}

/// RA-287, RA-317: a link planted at a progress file's name is never
/// written through; the run still does its work and names the fault, and
/// the link's target keeps its bytes.
#[test]
fn a_link_at_the_progress_name_is_never_written_through() {
    let root = scratch("progress-link");
    let writer = SpoolWriter::new(&root, 84, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(regular(), "NSE-INDEX-L", Code::Match, 1)],
        )
        .unwrap();
    let target = root.join("elsewhere.bytes");
    std::fs::write(&target, b"not a progress record").unwrap();
    std::fs::create_dir_all(root.join("verify")).unwrap();
    std::os::unix::fs::symlink(&target, root.join(PROGRESS_MERGE)).unwrap();
    let progress = crate::gdfl_run::ProgressFile::of_record(
        root.join(PROGRESS_MERGE),
        progress_record("verify-merge", 1, 85, 1, None),
    );
    drop(writer);
    let merged = merge_observed(&root, None, None, Some(&progress)).unwrap();
    assert_eq!(merged.generations, 1, "the merge still ran");
    assert!(progress.fault().is_some(), "the fault is named");
    assert_eq!(std::fs::read(&target).unwrap(), b"not a progress record");
}

// ------------------------------------------------------------- RA-302 --

/// RA-302: a renamed share is owed, under each of its names, only the days
/// the exchange used that name (the alias table's `filed_under`, D-3333).
/// With the two real renames, ZOMATO to ETERNAL on 9 April 2025 and
/// TATAMOTORS to TMPV on 24 October 2025, each name holding its own part of
/// the month: the old name's empty days after the change and the new name's
/// before it get no record, so neither reads years (here, weeks) of false
/// gaps; every day a name was live is verified. Before RA-302 each name owed
/// the whole month and the other name's days read as holes (`no_ledger`).
#[test]
fn a_renamed_share_owes_only_the_days_of_its_own_name() {
    for (old, new, change) in [
        ("ZOMATO", "ETERNAL", day(2025, 4, 9)),
        ("TATAMOTORS", "TMPV", day(2025, 10, 24)),
    ] {
        let root = scratch(&format!("rename-{new}"));
        let share = |symbol: &str| Series {
            exchange: Exchange::Nse,
            segment: Segment::Cash,
            symbol: symbol.to_owned(),
            contract: None,
        };
        let month = change.year_month().unwrap();
        let first = Day::new(change.year(), change.month(), 1).unwrap();
        let (mut on, mut last) = (first, first);
        let (mut before, mut after) = (Vec::new(), Vec::new());
        let (mut open_before, mut open_after) = (0_u64, 0_u64);
        while on.year_month().ok() == Some(month) {
            let open = !matches!(
                repair_plan::owed("CASH", on.days_from_epoch()),
                Owes::Closed
            );
            if open && on < change {
                open_before += 1;
                before.extend(full_day(on, &[]));
            } else if open {
                open_after += 1;
                after.extend(full_day(on, &[]));
            }
            last = on;
            on = on.succ().unwrap();
        }
        write_month(&root, &share(old), Timeframe::MINUTE_1, &before);
        write_month(&root, &share(new), Timeframe::MINUTE_1, &after);
        let span = |only: &str| ColdRequest {
            from: first,
            to: last,
            only: vec![only.to_owned()],
        };
        let writer = SpoolWriter::new(&root, 90, Mode::ZerodhaCold, 1);
        let old_report = cold_on(&root, &span(old), &writer, None, 1).unwrap();
        let new_report = cold_on(&root, &span(new), &writer, None, 1).unwrap();
        assert!(open_before > 0 && open_after > 0, "{old}: both parts trade");
        assert_eq!(old_report.days, open_before, "{old}: only its own days");
        assert_eq!(new_report.days, open_after, "{new}: only its own days");
        // The 1min rung's holes (the derived rungs of this fixture are not
        // written, and are not what a rename changes).
        let minute_holes = |r: &ColdReport, codes: &[Code]| -> u64 {
            r.by_rung
                .iter()
                .filter(|(rung, code, _)| rung == "1min" && codes.contains(code))
                .map(|(_, _, n)| *n)
                .sum()
        };
        let hole_codes = [Code::MissingInStore, Code::NoLedger, Code::VendorHole];
        let holes = minute_holes(&old_report, &hole_codes) + minute_holes(&new_report, &hole_codes);
        assert_eq!(holes, 0, "{old} to {new}: no false gap on either name");

        // ONLY THE NEW NAME HELD, the share's whole history filed under it
        // (as Zerodha files ETERNAL and TMPV on the operator's store): it owes
        // every day, so an open day before the change that it lacks is still
        // verified, and reads as the hole it is, never hidden by the rename.
        let root = scratch(&format!("rename-only-{new}"));
        let first_open = before
            .first()
            .map(|bar| {
                let local = bar.ts_micros.div_euclid(1_000_000) + 19_800;
                u32::try_from(local.div_euclid(86_400)).unwrap()
            })
            .unwrap();
        let whole: Vec<Bar> = before
            .iter()
            .chain(&after)
            .copied()
            .filter(|bar| {
                let local = bar.ts_micros.div_euclid(1_000_000) + 19_800;
                u32::try_from(local.div_euclid(86_400)).unwrap() != first_open
            })
            .collect();
        write_month(&root, &share(new), Timeframe::MINUTE_1, &whole);
        let writer = SpoolWriter::new(&root, 91, Mode::ZerodhaCold, 1);
        let only_new = cold_on(&root, &span(new), &writer, None, 1).unwrap();
        assert_eq!(
            only_new.days,
            open_before + open_after,
            "{new}: every open day, its old name not held"
        );
        assert_eq!(
            minute_holes(&only_new, &[Code::NoLedger]),
            1,
            "{new}: the one day it lacks is a hole, not hidden: {only_new:?}"
        );
    }
}

// ------------------------------------------- writes beside a merge, named --

/// A refused spool whose alert cannot be written (here a directory planted
/// at the alert queue's name) and whose merge log cannot take its `refused`
/// line (a directory at the log's name) is still refused by name, and both
/// failed writes are named in `faults`, never dropped: before this, both
/// results were discarded and the merge said nothing.
#[test]
fn a_write_beside_a_merge_that_fails_is_named() {
    let root = scratch("faults");
    let on = regular();
    let writer = SpoolWriter::new(&root, 33, Mode::ZerodhaMerge, 1);
    for key in ["NSE-INDEX-A", "NSE-INDEX-B"] {
        writer
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(on, key, Code::Match, 1)],
            )
            .unwrap();
    }
    let id = SpoolId {
        run: 33,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    flip(&id.path(&root), 64 + 20);
    std::fs::create_dir_all(root.join("audit/alerts")).unwrap();
    // The spool's directory made read-only: the spool still reads, its merge
    // log cannot be created.
    let folder = id.path(&root).parent().unwrap().to_path_buf();
    std::fs::set_permissions(&folder, std::os::unix::fs::PermissionsExt::from_mode(0o555)).unwrap();
    drop(writer);
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!(merged.refused.len(), 1, "{merged:?}");
    assert_eq!(merged.generations, 0);
    assert!(
        merged
            .faults
            .iter()
            .any(|f| f.contains("the alert was not written")),
        "{:?}",
        merged.faults
    );
    assert!(
        merged.faults.iter().any(|f| f.contains("merge log")),
        "{:?}",
        merged.faults
    );
    std::fs::set_permissions(&folder, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
}

/// RA-318: a vendor that changes its answer in a way that still decodes (a
/// new field at the root, a new field beside `candles`, a `status` that is
/// not text) is invisible to the decoder, so the checker counts the fields
/// the documented shape does not name with its own eyes: each altered
/// answer's record carries `vendor_shape` with the count and the first
/// field, its data still compared as before (`match`), and one alert of kind
/// 21 is put per change, not per instrument or per run. An unaltered answer
/// carries nothing and puts nothing.
#[test]
fn a_changed_answer_shape_is_named_and_alerted_once() {
    let on = regular();
    let minutes = full_day(on, &[]);
    let rows = kite_body(&minutes);
    let candles = rows
        .strip_prefix("{\"status\":\"success\",\"data\":{\"candles\":")
        .and_then(|rest| rest.strip_suffix("}}"))
        .unwrap()
        .to_owned();
    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    for (tag, body, want) in [
        ("plain", rows.clone(), None),
        (
            "root",
            format!(
                "{{\"status\":\"success\",\"data\":{{\"candles\":{candles}}},\"meta\":{{\"v\":2}}}}"
            ),
            Some((1_u32, "meta")),
        ),
        (
            "data",
            format!("{{\"status\":\"success\",\"data\":{{\"candles\":{candles},\"next\":null}}}}"),
            Some((1, "data.next")),
        ),
        (
            "status",
            format!("{{\"status\":7,\"data\":{{\"candles\":{candles}}}}}"),
            Some((1, "status")),
        ),
    ] {
        let root = scratch(&format!("shape-{tag}"));
        write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
        keep_answer(
            &root,
            on,
            body,
            &minutes,
            375,
            response_keep::Door::FromWindow,
        );
        for run in [98_u64, 99] {
            let writer = SpoolWriter::new(&root, run, Mode::ZerodhaCold, 1);
            let report = cold(&root, &request, &writer, None).unwrap();
            assert!(report.refused.is_empty(), "{tag}: {:?}", report.refused);
            let id = SpoolId {
                run,
                segment: Segment::Index,
                timeframe: Timeframe::MINUTE_1,
                month: on.year_month().unwrap(),
            };
            let read = spool::read_spool(&id.path(&root), &id).unwrap();
            let record = read
                .records
                .iter()
                .find(|r| r.day == on && r.key.text() == "NSE-INDEX-ZZIDX")
                .unwrap();
            assert_eq!(record.facts.code, Code::Match, "{tag}: {record:?}");
            let found: Vec<(u32, String)> = record
                .details
                .iter()
                .filter(|d| d.kind == words::DETAIL_VENDOR_SHAPE)
                .map(|d| {
                    (
                        u32::from_le_bytes(d.payload[0..4].try_into().unwrap()),
                        String::from_utf8(d.payload[4..].to_vec()).unwrap(),
                    )
                })
                .collect();
            assert_eq!(
                found,
                want.map(|(n, first)| (n, first.to_owned()))
                    .into_iter()
                    .collect::<Vec<_>>(),
                "{tag}"
            );
        }
        let alerts = match crate::alerts::window(&root) {
            Ok(window) => window
                .alerts
                .into_iter()
                .filter(|a| a.record.kind == VENDOR_SHAPE_CHANGED)
                .count(),
            Err(_) => 0,
        };
        assert_eq!(
            alerts,
            usize::from(want.is_some()),
            "{tag}: one alert across two runs"
        );
    }
}

/// RA-318's counter at its edges: an empty body, a body that is not an
/// object, an object with no fields, a body with three unnamed fields, and a
/// field name past the cut that ends inside a multi-byte character.
#[test]
fn the_shape_counter_holds_at_its_edges() {
    for body in [
        "",
        "[]",
        "null",
        "{\"status\":\"x\",\"data\":{\"candles\":[]}}",
    ] {
        assert_eq!(answer_shape(body), Shape::default(), "{body:?}");
    }
    let none = answer_shape("{}");
    assert_eq!((none.unknown, none.first.as_str()), (1, "status"));
    let three =
        answer_shape("{\"status\":\"x\",\"a\":1,\"b\":2,\"data\":{\"candles\":[],\"c\":3}}");
    assert_eq!(three.unknown, 3);
    let long = format!("{}é", "x".repeat(SHAPE_NAME_BYTES - 1));
    let cut = answer_shape(&format!("{{\"status\":\"x\",\"{long}\":1}}"));
    assert_eq!(cut.unknown, 1);
    assert_eq!(cut.first, "x".repeat(SHAPE_NAME_BYTES - 1));
    assert!(cut.first.len() <= SHAPE_NAME_BYTES);
}

/// RA-315 over the cold sweep's own walk: the five names macOS drops into a
/// folder (`.DS_Store`, `._<name>`, `.Spotlight-V100`, `.fseventsd`,
/// `Icon\r`), as files or folders, at the segment and the contract level, are
/// each one stray, never a series and never removed; the held series beside
/// them is still found, and a folder at the contract level that names no
/// contract is stray too.
#[test]
fn the_series_walk_counts_every_finder_name_as_stray() {
    let root = scratch("walk-stray");
    let on = regular();
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[]));
    let segment = root.join("bars/zerodha/NSE/INDEX");
    for name in [".DS_Store", "._ZZIDX", "Icon\r"] {
        std::fs::write(segment.join(name), b"x").unwrap();
    }
    for name in [".Spotlight-V100", ".fseventsd"] {
        std::fs::create_dir_all(segment.join(name)).unwrap();
    }
    let fno = root.join("bars/zerodha/NSE/FNO/ZZOPT");
    std::fs::create_dir_all(fno.join("not-a-contract")).unwrap();
    std::fs::write(fno.join(".DS_Store"), b"x").unwrap();
    let mut stray = 0;
    let held = series_held(&root, &mut stray).unwrap();
    let keys: Vec<String> = held.iter().map(Series::key_text).collect();
    assert_eq!(keys, vec!["NSE-INDEX-ZZIDX".to_owned()]);
    assert_eq!(stray, 7, "five at the segment, two at the contract level");
    assert!(segment.join(".DS_Store").exists(), "never removed");
    let mut again = 0;
    assert_eq!(series_held(&root, &mut again).unwrap().len(), 1);
    assert_eq!(again, 7, "the same count every walk");
}

/// A GDFL peer month that is held but will not open is a fault in words,
/// never only a quieter `peer_unavailable`: the cold run names it among the
/// months it could not read, and fails, while a peer that is simply not held
/// is no fault at all.
#[test]
fn a_peer_month_that_will_not_open_is_named() {
    let on = regular();
    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    let root = scratch("peer-absent");
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[600]));
    let writer = SpoolWriter::new(&root, 101, Mode::ZerodhaCold, 1);
    let report = cold(&root, &request, &writer, None).unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);

    let root = scratch("peer-torn");
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[600]));
    let peer = StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "ZZIDX",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: on.year_month().unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap()
    .to_path_buf(&root);
    std::fs::create_dir_all(peer.parent().unwrap()).unwrap();
    std::fs::write(&peer, b"not a bar file").unwrap();
    // Its siblings present, so the month is held and only unreadable.
    std::fs::write(peer.with_extension("lock"), b"").unwrap();
    std::fs::write(peer.with_extension("crc"), b"not a checksum").unwrap();
    let writer = SpoolWriter::new(&root, 102, Mode::ZerodhaCold, 1);
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(report.refused.len(), 1, "{:?}", report.refused);
    assert!(
        report.refused[0].contains("gdfl 1s peer"),
        "{:?}",
        report.refused
    );
}

/// GDFL's one-second bars of `ZZIDX` at `seconds` past IST midnight of `on`:
/// the peer month the cold sweep reads.
fn write_peer(root: &Path, on: Day, seconds: &[u32]) {
    let path = StorePath::new(store::path::PathParts {
        vendor: Vendor::Gdfl,
        exchange: "NSE",
        segment: "INDEX",
        symbol: "ZZIDX",
        contract: None,
        timeframe: Timeframe::SECOND_1,
        month: on.year_month().unwrap(),
        file: store::path::FileKind::Bars,
    })
    .unwrap();
    let bars: Vec<Bar> = seconds
        .iter()
        .map(|second| Bar {
            ts_micros: (utc_s(on, 0) + i64::from(*second)) * 1_000_000,
            open: 1_111_100,
            high: 1_111_150,
            low: 1_111_050,
            close: 1_111_110,
            volume: 0,
            open_interest: OI_NULL,
        })
        .collect();
    let mut file = BarFile::open_or_create(root, path, repair_plan::symbol_id("ZZIDX")).unwrap();
    file.append(&bars).unwrap();
}

/// A GDFL DAY THE PEER DOES NOT HOLD IS NO EVIDENCE (DB-A12; measured on the
/// operator's store 22 Oct 2026: GDFL's NIFTY 1s month is held but holds no
/// second of 2021-02-24, 2022-03-07 or the Muhurat sessions, and the verifier
/// read 590 minutes as `neither_vendor_has_it` on that): a peer month that
/// holds no second of the whole day did not import that day, so the minute
/// the store lacks is `peer_unavailable` and keeps its ledger class; a peer
/// that holds the day but no second of the minute still says neither vendor
/// has it.
#[test]
fn a_peer_day_not_held_is_no_evidence_that_neither_vendor_has_it() {
    let on = regular();
    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    let lacking = |report: &ColdReport| -> Vec<(Code, Option<&'static str>, u64)> {
        report
            .missing
            .iter()
            .filter(|(rung, ..)| rung == "1min")
            .map(|(_, code, reason, units)| (*code, words::reason_word(*reason), *units))
            .collect()
    };
    // (1) The peer month holds 22 April only: 23 April is not its to speak of.
    let root = scratch("peer-day-absent");
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[600]));
    write_peer(&root, day(2024, 4, 22), &[33_300, 33_301]);
    let writer = SpoolWriter::new(&root, 111, Mode::ZerodhaCold, 1);
    let report = cold(&root, &request, &writer, None).unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert_eq!(lacking(&report), vec![(Code::NoLedger, None, 1)]);
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_ne!(record.facts.flags & flags::PEER_UNAVAILABLE, 0);
    assert_eq!(record.facts.flags & flags::PEER_MINUTE_ABSENT, 0);

    // (2) The peer holds 23 April (09:15:00 and a second of 10:01) but no
    // second of 10:00: neither vendor has it.
    let root = scratch("peer-day-held");
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[600]));
    write_peer(&root, on, &[33_300, 36_060]);
    let writer = SpoolWriter::new(&root, 112, Mode::ZerodhaCold, 1);
    let report = cold(&root, &request, &writer, None).unwrap();
    assert!(report.refused.is_empty(), "{:?}", report.refused);
    assert_eq!(
        lacking(&report),
        vec![(Code::VendorHole, Some("neither_vendor_has_it"), 1)]
    );

    // (3) The peer holds a second of 10:00 itself: GDFL traded the minute the
    // store lacks, so it is the ledger's (no ledger here), with the peer named.
    let root = scratch("peer-minute-traded");
    write_month(&root, &index(), Timeframe::MINUTE_1, &full_day(on, &[600]));
    write_peer(&root, on, &[33_300, 36_030]);
    let writer = SpoolWriter::new(&root, 113, Mode::ZerodhaCold, 1);
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(lacking(&report), vec![(Code::NoLedger, None, 1)]);
    drop(writer);
    merge_all(&root, None, None).unwrap();
    let record = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZIDX",
    );
    assert_ne!(record.facts.flags & flags::PEER_MINUTE_ABSENT, 0);
    assert_eq!(record.facts.flags & flags::PEER_UNAVAILABLE, 0);
}

// ---------------------------------------------------------- IA-ZV-X23 --

/// Writes the same spools under `root`: runs 41 and 42, each one day of each
/// month of 2024's first half at 1min and 5min (24 spools), and both runs
/// verifying one key of one day, run 41 `match` and run 42 `value_mismatch`.
fn spools_for_width(root: &Path) {
    for (run, shared) in [(41_u64, Code::Match), (42, Code::ValueMismatch)] {
        let writer = SpoolWriter::new(root, run, Mode::ZerodhaMerge, 1);
        for month in 1..=6_u8 {
            let on = day(2024, month, 10);
            for tf in [Timeframe::MINUTE_1, Timeframe::MINUTE_5] {
                let mut records: Vec<SpoolRecord> = (0..3_u32)
                    .map(|n| spooled(on, &format!("NSE-INDEX-ZZW{run}N{n}"), Code::Match, n))
                    .collect();
                records.push(spooled(on, "NSE-INDEX-ZZIDX", shared, 9));
                writer.append(Segment::Index, tf, &records).unwrap();
            }
        }
    }
}

/// IA-ZV-X23 (RA-050): spools merge on workers, one (segment, timeframe,
/// month) group per worker at a time, and the answer is the same at any
/// width: the same generations, records and outcome at 1 and at 4 workers,
/// and for a day two runs verified, the later run's record is the one
/// committed last, whatever the width or the drive's listing order.
#[test]
fn spools_merge_on_workers_and_answer_as_one() {
    let (narrow, wide) = (scratch("width-1"), scratch("width-4"));
    spools_for_width(&narrow);
    spools_for_width(&wide);
    let one = merge_observed_on(&narrow, None, None, None, 1).unwrap();
    let four = merge_observed_on(&wide, None, None, None, 4).unwrap();
    assert_eq!(one.spools_merged, 24, "{one:?}");
    assert_eq!(
        (one.generations, one.records),
        (24, 24 * 4),
        "each run's spool commits one generation per day"
    );
    let shape = |o: &MergeOutcome| {
        (
            o.spools_merged,
            o.generations,
            o.records,
            o.days_already,
            o.days_recovered,
            o.refused.len(),
            o.torn.len(),
            o.faults.len(),
            o.waiting,
            o.stopped,
        )
    };
    assert_eq!(shape(&one), shape(&four));
    assert!(one.faults.is_empty() && four.faults.is_empty(), "{four:?}");
    for month in 1..=6_u8 {
        let on = day(2024, month, 10);
        for tf in [Timeframe::MINUTE_1, Timeframe::MINUTE_5] {
            // Two runs, two generations of the day, at either width.
            assert_eq!(year_generation(&narrow, Segment::Index, tf, on), 2);
            assert_eq!(year_generation(&wide, Segment::Index, tf, on), 2);
            for root in [&narrow, &wide] {
                let shared = the_record(root, Segment::Index, tf, on, "NSE-INDEX-ZZIDX");
                assert_eq!(
                    shared.facts.code,
                    Code::ValueMismatch,
                    "run 42 commits last"
                );
                assert_eq!(shared.run, 42);
                // Run 41's own keys are carried into run 42's generation.
                let carried = the_record(root, Segment::Index, tf, on, "NSE-INDEX-ZZW41N0");
                assert_eq!((carried.run, carried.facts.code), (41, Code::Match));
            }
        }
    }
    // Nothing waits, and a second merge at any width finds nothing.
    assert_eq!((one.waiting, four.waiting), (0, 0));
    let again = merge_observed_on(&wide, None, None, None, 8).unwrap();
    assert_eq!((again.spools_merged, again.generations), (0, 0));
    // The width is clamped: 0 reads as 1, past the most as the most.
    assert!(merge_workers() >= 1 && merge_workers() <= MAX_MERGE_WORKERS);
    let none = scratch("width-0");
    assert_eq!(
        merge_observed_on(&none, None, None, None, 0).unwrap(),
        MergeOutcome::default()
    );
    let many = scratch("width-99");
    spools_for_width(&many);
    let widest = merge_observed_on(&many, None, None, None, 99).unwrap();
    assert_eq!(shape(&widest), shape(&one));
}

/// IA-ZV-X23: a stop asked before a merge on workers leaves every spool
/// waiting, merged by none, named stopped, and the next merge takes them all.
#[test]
fn a_stop_before_a_merge_on_workers_leaves_every_spool_waiting() {
    let root = scratch("width-stop");
    spools_for_width(&root);
    // Asked at its first check: before any spool is read.
    let stop = StopControl::tripping_at(root.join("verify/merge.stop"), 91, 1);
    let stopped = merge_observed_on(&root, None, Some(&stop), None, 4).unwrap();
    assert!(stopped.stopped);
    assert_eq!((stopped.spools_merged, stopped.generations), (0, 0));
    assert_eq!(stopped.waiting, 24);
    assert!(stopped.faults.is_empty(), "{stopped:?}");
    let after = merge_observed_on(&root, None, None, None, 4).unwrap();
    assert_eq!((after.spools_merged, after.waiting), (24, 0));
}

/// A cold report keeps one example of each (rung, code), at most
/// `MAX_EXAMPLES`, so a rare verdict is shown however many common ones came
/// first. Measured on a clone of the operator's NIFTY 2022: 188 records of
/// `no_response_kept` and `built_under_earlier_rule` came first, the first
/// 24 filled the old list, and the month's one `vendor_hole` was not shown.
#[test]
fn a_rare_verdict_is_an_example_however_many_common_ones_came_first() {
    let mut report = ColdReport::default();
    let common = spooled(regular(), "NSE-INDEX-ZZIDX", Code::NoResponseKept, 1);
    for _ in 0..500 {
        report.count(Timeframe::MINUTE_1, &common, "NSE-INDEX-ZZIDX");
    }
    let rare = SpoolRecord {
        facts: Facts::of(Code::VendorHole),
        ..common.clone()
    };
    report.count(Timeframe::MINUTE_1, &rare, "NSE-INDEX-ZZIDX");
    let kinds: Vec<(&str, Code)> = report
        .examples
        .iter()
        .map(|(rung, code, _)| (rung.as_str(), *code))
        .collect();
    assert_eq!(
        kinds,
        [("1min", Code::NoResponseKept), ("1min", Code::VendorHole)]
    );
    assert!(
        report.examples[1].2.contains("vendor_hole"),
        "{:?}",
        report.examples
    );
    assert_eq!(report.records, 501);
    // Folding two series keeps one of each kind, in the listing's order.
    let mut whole = ColdReport::default();
    whole.fold(report.clone());
    whole.fold(report);
    assert_eq!(whole.examples.len(), 2);
    // At most MAX_EXAMPLES, whatever the kinds: 9 rungs x 17 codes > 48.
    let mut many = ColdReport::default();
    let rungs: Vec<Timeframe> = std::iter::once(Timeframe::MINUTE_1)
        .chain(derived_rungs())
        .chain(std::iter::once(Timeframe::DAY_1))
        .collect();
    for rung in &rungs {
        for code in Code::ALL.into_iter().filter(|c| *c != Code::Match) {
            let one = SpoolRecord {
                facts: Facts::of(code),
                ..common.clone()
            };
            many.count(*rung, &one, "NSE-INDEX-ZZIDX");
        }
    }
    assert_eq!(many.examples.len(), MAX_EXAMPLES);
    // A match is never an example.
    let mut matched = ColdReport::default();
    matched.count(
        Timeframe::MINUTE_1,
        &spooled(regular(), "NSE-INDEX-ZZIDX", Code::Match, 1),
        "NSE-INDEX-ZZIDX",
    );
    assert!(matched.examples.is_empty());
}

// ------------------------------------------------- IA-9 repair round 1 --

/// The verdict record of `key` on `on` at `tf`, or `None` when the verdict
/// store holds no record of it.
fn looked_up(root: &Path, tf: Timeframe, on: Day, key: &str) -> Option<Code> {
    let series = verdict::Series {
        root,
        feed: Vendor::Zerodha,
        exchange: Exchange::Nse,
        segment: Segment::Index,
        timeframe: tf,
    };
    let looked = verdict::Reader::new()
        .lookup(
            &series,
            store_day(on).unwrap(),
            &Key::instrument(key).unwrap(),
        )
        .unwrap();
    match looked.answer.map(|a| a.found) {
        Some(Found::Record { record, .. }) => Some(record.facts.code),
        _ => None,
    }
}

/// Blocking 1 of the repair round (IA-VD-08): a merge that runs while a run
/// is still spooling never reads, commits or removes that run's spools. They
/// are named live and wait; records the run appends after it are merged
/// once the run ends, never written into a removed file and never skipped as
/// merged. Before the live lock the first merge committed A and removed the
/// spool, and B and C, appended next, were never merged (0 generations,
/// nothing waiting, every call Ok).
#[test]
fn a_merge_never_reads_a_run_still_spooling() {
    let root = scratch("live");
    let (d1, d2) = (regular(), day(2024, 4, 24));
    let writer = SpoolWriter::new(&root, 7, Mode::ZerodhaCold, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(d1, "NSE-INDEX-ZZLIVEA", Code::Match, 0)],
        )
        .unwrap();
    assert!(spool::live_lock_path(&root, 7).is_file());
    let first = merge_all(&root, None, None).unwrap();
    assert_eq!(
        (first.spools_merged, first.generations),
        (0, 0),
        "{first:?}"
    );
    assert_eq!(first.waiting, 1);
    assert_eq!(first.waiting_not_live(), 0);
    assert_eq!(first.live.len(), 1);
    assert_eq!((first.live[0].0, first.live[0].1), (7, 1));
    assert!(
        first.live[0].2.contains("held by this process"),
        "{}",
        first.live[0].2
    );
    assert!(first.faults.is_empty() && first.refused.is_empty());
    assert_eq!(
        looked_up(&root, Timeframe::MINUTE_1, d1, "NSE-INDEX-ZZLIVEA"),
        None
    );
    // The run goes on: a second key of the same day, and a later day.
    let more = writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[
                spooled(d1, "NSE-INDEX-ZZLIVEB", Code::Match, 0),
                spooled(d2, "NSE-INDEX-ZZLIVEC", Code::VendorHole, 0),
            ],
        )
        .unwrap();
    assert_eq!(
        (more.records, more.born),
        (2, 0),
        "the same spool, still there"
    );
    let second = merge_all(&root, None, None).unwrap();
    assert_eq!((second.generations, second.live.len()), (0, 1));
    // The run ends: its writer goes, and with it the live lock.
    drop(writer);
    let last = merge_all(&root, None, None).unwrap();
    assert_eq!(
        (last.spools_merged, last.generations, last.records),
        (1, 2, 3)
    );
    assert!(last.live.is_empty() && last.waiting == 0, "{last:?}");
    for (on, key, code) in [
        (d1, "NSE-INDEX-ZZLIVEA", Code::Match),
        (d1, "NSE-INDEX-ZZLIVEB", Code::Match),
        (d2, "NSE-INDEX-ZZLIVEC", Code::VendorHole),
    ] {
        assert_eq!(
            looked_up(&root, Timeframe::MINUTE_1, on, key),
            Some(code),
            "{key}"
        );
    }
    // One generation per (rung, day): the live run was merged once.
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, d1),
        1
    );
    // The lock file stays (never removed), free.
    assert!(spool::live_lock_path(&root, 7).is_file());
    assert!(matches!(
        spool::live_probe(&root, 7).unwrap(),
        spool::Live::Taken(_)
    ));
}

/// The live lock's edges: a run whose lock a merger holds (it is merging that
/// run's spools of an earlier process) makes a resumed writer of the run wait,
/// then append once the merger lets go; a cold run's spools are live while the
/// cold run's writer lives; many runs at once, one live, the rest merged.
#[test]
fn a_live_run_and_a_finished_run_are_told_apart() {
    let root = scratch("live-many");
    let on = regular();
    // Three finished runs and one live one.
    for run in [11, 12, 13] {
        let finished = SpoolWriter::new(&root, run, Mode::ZerodhaMerge, 1);
        finished
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(
                    on,
                    &format!("NSE-INDEX-ZZRUN{run}"),
                    Code::Match,
                    0,
                )],
            )
            .unwrap();
    }
    let live = SpoolWriter::new(&root, 14, Mode::ZerodhaMerge, 1);
    live.append(
        Segment::Index,
        Timeframe::DAY_1,
        &[spooled(on, "NSE-INDEX-ZZRUN14", Code::Match, 0)],
    )
    .unwrap();
    let outcome = merge_observed_on(&root, None, None, None, 4).unwrap();
    assert_eq!(outcome.spools_merged, 3, "{outcome:?}");
    assert_eq!(
        outcome.generations, 3,
        "three runs' spools of one day: one generation each, in run order"
    );
    assert_eq!(
        outcome.live.iter().map(|l| (l.0, l.1)).collect::<Vec<_>>(),
        [(14, 1)]
    );
    for run in [11, 12, 13] {
        assert_eq!(
            looked_up(
                &root,
                Timeframe::MINUTE_1,
                on,
                &format!("NSE-INDEX-ZZRUN{run}")
            ),
            Some(Code::Match)
        );
    }
    assert_eq!(
        looked_up(&root, Timeframe::DAY_1, on, "NSE-INDEX-ZZRUN14"),
        None
    );
    // A merger holds run 15's lock (as while it merges run 15's orphan
    // spools); a resumed writer of run 15 waits for it, then appends.
    let held = match spool::live_probe(&root, 15).unwrap() {
        spool::Live::Taken(lock) => lock,
        spool::Live::Spooling(holder) => panic!("free run 15 read as live: {holder}"),
    };
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        drop(held);
    });
    let started = std::time::Instant::now();
    let resumed = SpoolWriter::new(&root, 15, Mode::ZerodhaMerge, 1);
    resumed
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZRUN15", Code::Match, 0)],
        )
        .unwrap();
    assert!(started.elapsed() >= std::time::Duration::from_millis(250));
    release.join().unwrap();
    drop((live, resumed));
    let rest = merge_all(&root, None, None).unwrap();
    assert_eq!(
        (rest.spools_merged, rest.live.len(), rest.waiting),
        (2, 0, 0)
    );
    assert_eq!(
        looked_up(&root, Timeframe::DAY_1, on, "NSE-INDEX-ZZRUN14"),
        Some(Code::Match)
    );
    // A cold run's writer is live while it lives: its spools wait.
    let mut bars = full_day(on, &[]);
    bars.truncate(30);
    write_month(&root, &index(), Timeframe::MINUTE_1, &bars);
    let cold_writer = SpoolWriter::new(&root, 16, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: on,
        to: on,
        only: vec!["ZZIDX".to_owned()],
    };
    cold(&root, &request, &cold_writer, None).unwrap();
    let during = merge_all(&root, None, None).unwrap();
    assert_eq!((during.generations, during.live.len()), (0, 1));
    assert!(during.waiting >= 8, "1min and seven derived rungs wait");
    drop(cold_writer);
    let after = merge_all(&root, None, None).unwrap();
    assert_eq!((after.waiting, after.live.len()), (0, 0));
    assert_eq!(after.spools_merged, during.waiting);
}

/// The merge-log coverage (IA-9 repair 1): a day a merge committed covers
/// only the records that merge read. A resumed run that appends to a spool
/// a stopped merge left waiting gets its new records merged by the next
/// merge, never skipped as `Already` (before the `seen` record, B was lost
/// while every call returned Ok).
#[test]
fn records_a_resumed_run_adds_to_a_partly_merged_spool_are_merged() {
    let root = scratch("cover");
    let (d1, d2, d3) = (regular(), day(2024, 4, 24), day(2024, 4, 25));
    let first = SpoolWriter::new(&root, 8, Mode::ZerodhaMerge, 1);
    first
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[
                spooled(d1, "NSE-INDEX-ZZCOVA", Code::Match, 0),
                spooled(d2, "NSE-INDEX-ZZCOVX", Code::Match, 0),
                spooled(d3, "NSE-INDEX-ZZCOVY", Code::Match, 0),
            ],
        )
        .unwrap();
    drop(first);
    // A stop asked mid-spool: day 1 commits, the spool waits with its log.
    let stop = StopControl::tripping_at(root.join("verify/merge.stop"), 8, 3);
    let partly = merge_all(&root, None, Some(&stop)).unwrap();
    assert!(partly.stopped, "{partly:?}");
    assert!((1..3).contains(&partly.generations), "{partly:?}");
    assert_eq!((partly.waiting, partly.refused.len()), (1, 0));
    let id = SpoolId {
        run: 8,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: d1.year_month().unwrap(),
    };
    let log = spool::read_log(&id.log_path(&root)).unwrap();
    assert_eq!(log.covered(), 3, "the merge said what it read");
    let nse = verdict::exchange_code(Exchange::Nse);
    assert_eq!(log.covered_of(nse, d1.days_from_epoch()), Some(3));
    // The run resumes under its id in a new process and adds to day 1.
    let resumed = SpoolWriter::new(&root, 8, Mode::ZerodhaMerge, 1);
    let more = resumed
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(d1, "NSE-INDEX-ZZCOVB", Code::VendorHole, 0)],
        )
        .unwrap();
    assert_eq!(more.born, 0, "the spool the merge left");
    drop(resumed);
    let rest = merge_all(&root, None, None).unwrap();
    assert!(rest.refused.is_empty() && rest.waiting == 0, "{rest:?}");
    assert_eq!(
        rest.days_already,
        partly.generations - 1,
        "day 1 gained a record: merged again; a day wholly committed is not"
    );
    assert_eq!(rest.generations, 3 - partly.generations + 1);
    assert_eq!(
        year_generation(&root, Segment::Index, Timeframe::MINUTE_1, d1),
        2,
        "day 1's first commit and its re-merge"
    );
    for (on, key, code) in [
        (d1, "NSE-INDEX-ZZCOVB", Code::VendorHole),
        (d1, "NSE-INDEX-ZZCOVA", Code::Match),
        (d2, "NSE-INDEX-ZZCOVX", Code::Match),
        (d3, "NSE-INDEX-ZZCOVY", Code::Match),
    ] {
        assert_eq!(
            looked_up(&root, Timeframe::MINUTE_1, on, key),
            Some(code),
            "{key}"
        );
    }
    // A day whose commit covered every record is still `Already`: one log
    // line per state, the records of a crash re-run never twice.
    let again = SpoolWriter::new(&root, 9, Mode::ZerodhaMerge, 1);
    again
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[
                spooled(d1, "NSE-INDEX-ZZCOVD", Code::Match, 0),
                spooled(d2, "NSE-INDEX-ZZCOVE", Code::Match, 0),
            ],
        )
        .unwrap();
    drop(again);
    let crashed = SpoolId { run: 9, ..id };
    let mut log = spool::read_log(&crashed.log_path(&root)).unwrap();
    for record in [
        LogRecord {
            day: 2,
            generation: 0,
            state: LogState::Seen,
            exchange: 0,
        },
        LogRecord {
            day: d1.days_from_epoch(),
            generation: year_generation(&root, Segment::Index, Timeframe::MINUTE_1, d1),
            state: LogState::Committed,
            exchange: nse,
        },
    ] {
        spool::append_log(&crashed.log_path(&root), &mut log, record).unwrap();
    }
    let last = merge_all(&root, None, None).unwrap();
    assert_eq!((last.days_already, last.generations), (1, 1), "{last:?}");
    // The `seen` state survives its byte.
    assert_eq!(
        LogState::from_code(LogState::Seen.code()),
        Some(LogState::Seen)
    );
    assert_eq!(LogState::from_code(5), None);
}

/// Blocking 2 of the repair round (IA-ZV-03): every held rung month in the
/// span is verified. A series held at 1day only, and a 1day month before the
/// series' first 1min month, get 1day records (against their kept answer,
/// `no_response_kept` with none) naming `daycheck` not run, and no 1min
/// record; a name that matches no held series is said. Before the repair the
/// cold run listed only series holding a 1min directory and walked only the
/// 1min months: here 1 series, the March month never walked, exit OK.
#[test]
fn a_series_held_at_1day_only_and_a_day_month_outside_the_minutes_are_verified() {
    let root = scratch("dayonly");
    let march = day(2024, 3, 27);
    let april = regular();
    let minutes = full_day(april, &[]);
    write_month(&root, &index(), Timeframe::MINUTE_1, &minutes);
    let bar_of = |on: Day| {
        let mut bar = bar_at(on, 0, 1_111_100);
        bar.ts_micros = utc_s(on, 0) * 1_000_000;
        bar
    };
    write_month(&root, &index(), Timeframe::DAY_1, &[bar_of(march)]);
    let mut april_bar = minutes[0];
    april_bar.ts_micros = utc_s(april, 0) * 1_000_000;
    write_month(&root, &index(), Timeframe::DAY_1, &[april_bar]);
    let only_daily = Series {
        symbol: "ZZDAYONLY".to_owned(),
        ..index()
    };
    write_month(&root, &only_daily, Timeframe::DAY_1, &[bar_of(march)]);
    let writer = SpoolWriter::new(&root, 31, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: day(2024, 3, 1),
        to: day(2024, 4, 30),
        only: vec![
            "ZZIDX".to_owned(),
            "NSE-INDEX-ZZDAYONLY".to_owned(),
            "ZZNONE".to_owned(),
        ],
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(report.series, 2, "{report:?}");
    assert_eq!(report.unmatched, ["ZZNONE"]);
    assert_eq!(report.months, 3, "ZZIDX March and April, ZZDAYONLY March");
    assert_eq!(report.beyond_months, 2);
    let month_of = |tf: Timeframe, on: Day| SpoolId {
        run: 31,
        segment: Segment::Index,
        timeframe: tf,
        month: on.year_month().unwrap(),
    };
    // No 1min record outside the 1min months: the store owes no minute there.
    assert!(!month_of(Timeframe::MINUTE_1, march).path(&root).exists());
    assert!(!month_of(Timeframe::MINUTE_5, march).path(&root).exists());
    let daily = spool::read_spool(
        &month_of(Timeframe::DAY_1, march).path(&root),
        &month_of(Timeframe::DAY_1, march),
    )
    .unwrap();
    for key in ["NSE-INDEX-ZZIDX", "NSE-INDEX-ZZDAYONLY"] {
        let record = daily
            .records
            .iter()
            .find(|r| r.key.text() == key && r.day == march)
            .unwrap_or_else(|| panic!("{key} has its 27 March 1day record"));
        assert_eq!(record.facts.code, Code::NoResponseKept);
        assert_eq!(record.facts.store_units, 1);
        let named = record
            .details
            .iter()
            .find(|d| d.kind == words::DETAIL_DAYCHECK_NOT_RUN)
            .expect("daycheck named not run");
        assert_eq!(named.payload, [words::DAYCHECK_BEYOND_MINUTES]);
    }
    // Inside the 1min months the minutes are verified and `daycheck` runs.
    let april_daily = spool::read_spool(
        &month_of(Timeframe::DAY_1, april).path(&root),
        &month_of(Timeframe::DAY_1, april),
    )
    .unwrap();
    let held = april_daily
        .records
        .iter()
        .find(|r| r.key.text() == "NSE-INDEX-ZZIDX" && r.day == april)
        .unwrap();
    assert!(
        held.details
            .iter()
            .all(|d| d.kind != words::DETAIL_DAYCHECK_NOT_RUN)
    );
    assert!(month_of(Timeframe::MINUTE_1, april).path(&root).exists());
    // A day bar whose own minutes are absent inside the 1min months names it.
    let inputs = DayInputs {
        day_bar: Some(april_bar),
        day_month_held: true,
        ..DayInputs::default()
    };
    let made = verify_day(&index(), april, &inputs);
    let one_day = made
        .iter()
        .find(|r| r.timeframe == Timeframe::DAY_1)
        .unwrap();
    assert_eq!(
        one_day
            .record
            .details
            .last()
            .map(|d| (d.kind, d.payload.clone())),
        Some((
            words::DETAIL_DAYCHECK_NOT_RUN,
            vec![words::DAYCHECK_NO_MINUTE]
        ))
    );
    assert!(made.iter().any(|r| r.timeframe == Timeframe::MINUTE_1));
    let unread = DayInputs {
        minutes_refused: Some("refused for the test"),
        ..inputs.clone()
    };
    let made = verify_day(&index(), april, &unread);
    assert_eq!(
        made.iter()
            .find(|r| r.timeframe == Timeframe::DAY_1)
            .and_then(|r| r.record.details.last())
            .map(|d| d.payload.clone()),
        Some(vec![words::DAYCHECK_MINUTES_UNREAD])
    );
    // Beyond the 1min months: no 1min record, no derived record not held.
    let beyond = DayInputs {
        beyond_minutes: true,
        derived: vec![(Timeframe::MINUTE_5, None, None)],
        ..inputs
    };
    let made = verify_day(&index(), march, &beyond);
    assert_eq!(
        made.iter().map(|r| r.timeframe).collect::<Vec<_>>(),
        [Timeframe::DAY_1]
    );
    // Every series listed holds a rung: the 1day-only one is listed.
    let mut stray = 0;
    let listed = series_held(&root, &mut stray).unwrap();
    assert_eq!(listed.len(), 2);
    // Outside the calendar of record (March 2016, which no calendar attests):
    // the day holding a bar is verified; a day holding nothing is not judged,
    // only counted (a weekend there reads the same as a holiday). Days before
    // the series' first bar owe nothing, as anywhere.
    let old = day(2016, 3, 10);
    write_month(&root, &only_daily, Timeframe::DAY_1, &[bar_of(old)]);
    let writer = SpoolWriter::new(&root, 32, Mode::ZerodhaCold, 1);
    let request = ColdRequest {
        from: day(2016, 3, 1),
        to: day(2016, 3, 31),
        only: vec!["ZZDAYONLY".to_owned()],
    };
    let report = cold(&root, &request, &writer, None).unwrap();
    assert_eq!(
        (
            report.months,
            report.beyond_months,
            report.days,
            report.records
        ),
        (1, 1, 1, 1),
        "{report:?}"
    );
    assert_eq!(report.unattested_days, 21, "11 to 31 March");
    // The counter folds like every other.
    let mut whole = ColdReport::default();
    whole.fold(report.clone());
    whole.fold(report);
    assert_eq!((whole.unattested_days, whole.beyond_months), (42, 2));
}

/// The spool's own edges the repair closed: a merge log whose spool is gone
/// is named stray (never read, never removed); an append that would take a
/// spool past the merge's read cap is refused by name with nothing written.
#[test]
fn an_orphan_log_is_stray_and_a_full_spool_refuses_the_append() {
    let root = scratch("orphan-log");
    let on = regular();
    let id = SpoolId {
        run: 41,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    std::fs::create_dir_all(id.path(&root).parent().unwrap()).unwrap();
    std::fs::write(id.log_path(&root), b"").unwrap();
    let listed = spool::waiting(&root).unwrap();
    assert!(listed.spools.is_empty());
    assert_eq!(listed.stray, [id.log_path(&root)]);
    // With its spool beside it, the log is the spool's.
    let writer = SpoolWriter::new(&root, 41, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZFULL", Code::Match, 0)],
        )
        .unwrap();
    let listed = spool::waiting(&root).unwrap();
    assert_eq!((listed.spools.len(), listed.stray.len()), (1, 0));
    drop(writer);
    // A spool at the cap (sparse): one more two-slot record would pass it.
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(id.path(&root))
        .unwrap();
    file.set_len(spool::MAX_SPOOL_BYTES).unwrap();
    drop(file);
    let resumed = SpoolWriter::new(&root, 41, Mode::ZerodhaMerge, 1);
    let mut big = spooled(on, "NSE-INDEX-ZZFULL", Code::VendorHole, 0);
    big.details.push(Detail {
        kind: words::DETAIL_PEER,
        payload: vec![1; 8],
    });
    match resumed.append(Segment::Index, Timeframe::MINUTE_1, &[big]) {
        Err(SpoolError::Full { len, adding, .. }) => {
            assert_eq!(adding, 2 * spool::RECORD_LEN);
            assert!(len + adding > spool::MAX_SPOOL_BYTES);
            assert_eq!(
                std::fs::metadata(id.path(&root)).unwrap().len(),
                spool::MAX_SPOOL_BYTES,
                "nothing written"
            );
        }
        other => panic!("a full spool took the append: {other:?}"),
    }
    let words = SpoolError::Full {
        path: id.path(&root),
        len: 1,
        adding: 2,
    };
    assert_eq!(words.code(), "spool_full");
    assert!(words.to_string().contains("nothing was appended"));
    let busy = SpoolError::Busy {
        run: 41,
        holder: "a test holder".to_owned(),
    };
    assert_eq!(busy.code(), "run_live_busy");
    assert!(busy.to_string().contains("verify/live/41.lock"));
}

/// A verdict record counts the details that follow it (IA-9 repair 1, the
/// second check's follow-up): a last record whose detail an append never
/// finished is ignored whole and named, never merged without its detail; a
/// writer resuming writes over it from its start; a detail its record does
/// not declare is damage; a record with more details than a spool carries is
/// refused before a byte is written.
#[test]
fn a_torn_detail_is_never_merged_away() {
    let root = scratch("torn-detail");
    let on = regular();
    let id = SpoolId {
        run: 42,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    let with_detail = |key: &str, code: Code| {
        let mut record = spooled(on, key, code, 0);
        record.details.push(Detail {
            kind: words::DETAIL_PEER,
            payload: vec![1, 2, 3, 4, 5, 6, 7, 8],
        });
        record
    };
    let writer = SpoolWriter::new(&root, 42, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[with_detail("NSE-INDEX-ZZTORNA", Code::VendorHole)],
        )
        .unwrap();
    drop(writer);
    let path = id.path(&root);
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 64 + 320);
    // The detail torn: the verdict record is whole, its detail is not.
    let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(64 + 160 + 100).unwrap();
    drop(file);
    let read = spool::read_spool(&path, &id).unwrap();
    assert!(read.records.is_empty(), "never merged without its detail");
    assert_eq!((read.unsynced, read.unsynced_bytes), (Some(64), 160));
    assert_eq!(read.torn, Some((224, 100)));
    // A writer resuming writes over it from the record's start.
    let resumed = SpoolWriter::new(&root, 42, Mode::ZerodhaMerge, 1);
    resumed
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[with_detail("NSE-INDEX-ZZTORNB", Code::VendorHole)],
        )
        .unwrap();
    drop(resumed);
    assert_eq!(std::fs::metadata(&path).unwrap().len(), 64 + 320);
    let read = spool::read_spool(&path, &id).unwrap();
    assert_eq!(read.records.len(), 1);
    assert_eq!(read.records[0].key.text(), "NSE-INDEX-ZZTORNB");
    assert_eq!(read.records[0].details.len(), 1);
    assert_eq!((read.unsynced, read.torn), (None, None));
    let merged = merge_all(&root, None, None).unwrap();
    assert_eq!((merged.generations, merged.records), (1, 1));
    assert_eq!(
        looked_up(&root, Timeframe::MINUTE_1, on, "NSE-INDEX-ZZTORNB"),
        Some(Code::VendorHole)
    );
    assert_eq!(
        looked_up(&root, Timeframe::MINUTE_1, on, "NSE-INDEX-ZZTORNA"),
        None
    );
    // A detail its record does not declare, before a later record: damage.
    let root = scratch("torn-detail-extra");
    let bare = spool::encode_record(&spooled(on, "NSE-INDEX-ZZTORNC", Code::Match, 0), 43).unwrap();
    let full = spool::encode_record(&with_detail("NSE-INDEX-ZZTORNC", Code::Match), 43).unwrap();
    let writer = SpoolWriter::new(&root, 43, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZTORND", Code::Match, 0)],
        )
        .unwrap();
    drop(writer);
    let extra_path = SpoolId { run: 43, ..id }.path(&root);
    append_raw(&extra_path, &bare);
    append_raw(&extra_path, &full[160..]);
    append_raw(&extra_path, &bare);
    assert!(matches!(
        spool::read_spool(&extra_path, &SpoolId { run: 43, ..id }),
        Err(SpoolError::Damaged { record: 2, .. })
    ));
    // More details than a spooled record carries: refused, nothing written.
    let mut many = spooled(on, "NSE-INDEX-ZZTORNE", Code::Match, 0);
    for _ in 0..=spool::MAX_DETAILS {
        many.details.push(Detail {
            kind: words::DETAIL_PEER,
            payload: vec![9],
        });
    }
    assert!(matches!(
        spool::encode_record(&many, 43),
        Err(SpoolError::Record { .. })
    ));
}

/// IA-ZV-X32 (IA-9 repair 2, the second check's blocking issue): a writer
/// resuming over an ignored last group leaves no byte of it after its
/// append. The group can be a verdict record and 32 details and the append
/// one bare record; the spool is cut to the resume point before the append,
/// so the merge reads every whole record and refuses nothing. The three
/// shapes: a 32-detail group whose last detail fails its CRC, a clean cut
/// after 1 of 2 details, and that cut with 100 bytes of the next detail.
#[test]
fn a_resumed_append_shorter_than_its_torn_group_leaves_none_of_it() {
    let on = regular();
    let with_details = |key: &str, n: usize| {
        let mut record = spooled(on, key, Code::VendorHole, 0);
        for at in 0..n {
            record.details.push(Detail {
                kind: words::DETAIL_PEER,
                payload: vec![u8::try_from(at).unwrap(); 8],
            });
        }
        record
    };
    // (tag, run, details per group, whole details of the second group kept
    // and extra bytes after them; None = the last detail fails its CRC)
    for (tag, run, details, cut) in [
        ("crc", 50_u64, spool::MAX_DETAILS, None),
        ("cut", 51, 2, Some((1_u64, 0_u64))),
        ("cut-partial", 52, 2, Some((1, 100))),
    ] {
        let root = scratch(&format!("resume-cut-{tag}"));
        let id = SpoolId {
            run,
            segment: Segment::Index,
            timeframe: Timeframe::MINUTE_1,
            month: on.year_month().unwrap(),
        };
        let writer = SpoolWriter::new(&root, run, Mode::ZerodhaMerge, 1);
        for key in ["NSE-INDEX-ZZRESA", "NSE-INDEX-ZZRESB"] {
            writer
                .append(
                    Segment::Index,
                    Timeframe::MINUTE_1,
                    &[with_details(key, details)],
                )
                .unwrap();
        }
        drop(writer);
        let path = id.path(&root);
        let group = 160 * (1 + details as u64);
        let len = std::fs::metadata(&path).unwrap().len();
        assert_eq!(len, 64 + 2 * group, "{tag}");
        match cut {
            None => flip(&path, len - 100),
            Some((kept, extra)) => {
                let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
                file.set_len(64 + group + 160 * (1 + kept) + extra).unwrap();
            }
        }
        // The torn group is ignored whole: only the first is read.
        let read = spool::read_spool(&path, &id).unwrap();
        assert_eq!(read.records.len(), 1, "{tag}");
        assert_eq!(read.unsynced, Some(64 + group), "{tag}");
        // A writer resuming appends one bare record, shorter than the group.
        let resumed = SpoolWriter::new(&root, run, Mode::ZerodhaMerge, 1);
        resumed
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[spooled(on, "NSE-INDEX-ZZRESC", Code::Match, 0)],
            )
            .unwrap();
        drop(resumed);
        assert_eq!(
            std::fs::metadata(&path).unwrap().len(),
            64 + group + 160,
            "{tag}: no byte of the torn group is left after the append"
        );
        let read = spool::read_spool(&path, &id).unwrap();
        let keys: Vec<&str> = read.records.iter().map(|r| r.key.text()).collect();
        assert_eq!(keys, ["NSE-INDEX-ZZRESA", "NSE-INDEX-ZZRESC"], "{tag}");
        assert_eq!(read.records[0].details.len(), details, "{tag}");
        assert!(read.records[1].details.is_empty(), "{tag}");
        assert_eq!((read.unsynced, read.torn), (None, None), "{tag}");
        let merged = merge_all(&root, None, None).unwrap();
        assert!(merged.refused.is_empty(), "{tag}: {merged:?}");
        assert_eq!((merged.records, merged.waiting), (2, 0), "{tag}");
        assert_eq!(
            looked_up(&root, Timeframe::MINUTE_1, on, "NSE-INDEX-ZZRESA"),
            Some(Code::VendorHole),
            "{tag}"
        );
        assert_eq!(
            looked_up(&root, Timeframe::MINUTE_1, on, "NSE-INDEX-ZZRESC"),
            Some(Code::Match),
            "{tag}"
        );
        assert_eq!(
            looked_up(&root, Timeframe::MINUTE_1, on, "NSE-INDEX-ZZRESB"),
            None,
            "{tag}: the torn record is never merged"
        );
    }
}

/// IA-ZV-X33 (IA-9 repair 2, the second check's follow-up): a merge holds at
/// most `LIVE_LOCKS_AT_ONCE` runs' live locks open at once, whatever the
/// number of runs waiting: it merges them a bounded slice of runs at a time,
/// in run order, so a later run's record still commits last across slices,
/// and every run's keys are carried.
#[test]
fn a_merge_holds_a_bounded_number_of_live_locks() {
    let root = scratch("live-locks");
    let on = regular();
    let runs = LIVE_LOCKS_AT_ONCE as u64 + 3;
    let name = |run: u64| {
        let letter = |n: u64| char::from(b'A' + u8::try_from(n % 26).unwrap());
        format!("NSE-INDEX-ZZL{}{}", letter(run / 26), letter(run))
    };
    for run in 1..=runs {
        let code = if run == runs {
            Code::Match
        } else {
            Code::VendorHole
        };
        let writer = SpoolWriter::new(&root, 2000 + run, Mode::ZerodhaMerge, 1);
        writer
            .append(
                Segment::Index,
                Timeframe::MINUTE_1,
                &[
                    spooled(on, &name(run), Code::Match, 0),
                    spooled(on, "NSE-INDEX-ZZLSHARED", code, 0),
                ],
            )
            .unwrap();
        drop(writer);
    }
    let merged = merge_observed_on(&root, None, None, None, 4).unwrap();
    assert_eq!(
        merged.live_locks_peak, LIVE_LOCKS_AT_ONCE as u64,
        "{merged:?}"
    );
    assert_eq!((merged.spools_merged, merged.records), (runs, 2 * runs));
    assert!(merged.refused.is_empty() && merged.faults.is_empty() && merged.live.is_empty());
    assert_eq!(merged.waiting, 0);
    let shared = the_record(
        &root,
        Segment::Index,
        Timeframe::MINUTE_1,
        on,
        "NSE-INDEX-ZZLSHARED",
    );
    assert_eq!(
        (shared.run, shared.facts.code),
        (u32::try_from(2000 + runs).unwrap(), Code::Match)
    );
    for run in 1..=runs {
        assert_eq!(
            looked_up(&root, Timeframe::MINUTE_1, on, &name(run)),
            Some(Code::Match),
            "run {run}"
        );
    }
    // The slice ends after its runs' last spool; 0 reads as 1.
    let ids: Vec<SpoolId> = [7_u64, 7, 8, 9, 9, 9]
        .iter()
        .map(|&run| SpoolId {
            run,
            segment: Segment::Index,
            timeframe: Timeframe::MINUTE_1,
            month: on.year_month().unwrap(),
        })
        .collect();
    assert_eq!(runs_end(&ids, 0), 2);
    assert_eq!(runs_end(&ids, 1), 2);
    assert_eq!(runs_end(&ids, 2), 3);
    assert_eq!(runs_end(&ids, 3), 6);
    assert_eq!(runs_end(&ids, 9), 6);
    assert_eq!(runs_end(&[], 9), 0);
}

/// IA-ZV-X34 (IA-9 repair 2, the second check's follow-up): a spool or a
/// merge log the host will not open keeps the host's words. Only a name that
/// is not a regular file is `not_regular`; a regular file refused (here
/// `EACCES`; `EMFILE` takes the same path) is `io` with the host's error.
#[test]
fn a_refused_open_keeps_the_hosts_words() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = scratch("refused-open");
    let on = regular();
    let id = SpoolId {
        run: 53,
        segment: Segment::Index,
        timeframe: Timeframe::MINUTE_1,
        month: on.year_month().unwrap(),
    };
    let writer = SpoolWriter::new(&root, 53, Mode::ZerodhaMerge, 1);
    writer
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZDENY", Code::Match, 0)],
        )
        .unwrap();
    drop(writer);
    let path = id.path(&root);
    let log = root.join("verify/denied.log");
    std::fs::write(&log, []).unwrap();
    for name in [&path, &log] {
        std::fs::set_permissions(name, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
    let host = |why: SpoolError| match why {
        SpoolError::Io { source, .. } => source.kind(),
        other => panic!("not the host's words: {other}"),
    };
    assert_eq!(
        host(spool::read_spool(&path, &id).unwrap_err()),
        std::io::ErrorKind::PermissionDenied
    );
    let resumed = SpoolWriter::new(&root, 53, Mode::ZerodhaMerge, 1);
    let refused = resumed
        .append(
            Segment::Index,
            Timeframe::MINUTE_1,
            &[spooled(on, "NSE-INDEX-ZZDENZ", Code::Match, 0)],
        )
        .unwrap_err();
    assert_eq!(refused.code(), "io", "{refused}");
    assert_eq!(host(refused), std::io::ErrorKind::PermissionDenied);
    drop(resumed);
    assert_eq!(
        host(spool::read_log(&log).unwrap_err()),
        std::io::ErrorKind::PermissionDenied
    );
    let mut read = spool::LogRead::default();
    let record = LogRecord {
        day: 1,
        generation: 1,
        state: LogState::Committed,
        exchange: 1,
    };
    assert_eq!(
        host(spool::append_log(&log, &mut read, record).unwrap_err()),
        std::io::ErrorKind::PermissionDenied
    );
    // A link at the name is still not_regular.
    let link = root.join("verify/linked.log");
    std::os::unix::fs::symlink(&log, &link).unwrap();
    assert!(matches!(
        spool::read_log(&link),
        Err(SpoolError::NotRegular { .. })
    ));
    for name in [&path, &log] {
        std::fs::set_permissions(name, std::fs::Permissions::from_mode(0o644)).unwrap();
    }
}

/// A busy merge names its lock and its holder as two phrases (IA-9 repair 2,
/// the second check's follow-up: "verify/merge.lockheld by ...").
#[test]
fn a_busy_merge_names_its_lock_and_holder_apart() {
    let busy = MergeError::RunBusy {
        holder: "held by a test holder".to_owned(),
    };
    assert!(
        busy.to_string()
            .contains("holds verify/merge.lock (held by a test holder);"),
        "{busy}"
    );
}
