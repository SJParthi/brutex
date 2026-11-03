//! BASE-COV-PULL: the stored-month gap ledger's refusals and skips no test had
//! reached (Route B, recipe v4): the day word's shape, a ledger line that is
//! not a well-formed `gap`, a torn fault read twice, a held month outside the
//! window, a month the plan refuses, a session no record sizes, a record that
//! will not read, and a ledger that cannot be appended. Every series, price
//! and stamp here is invented (`ZZIDX`, `ZZSEG`, prices near 11,111 rupees);
//! only the calendar of record is the repository's own.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "a test that cannot panic cannot fail"
)]

use std::fs;
use std::io::{Seek as _, SeekFrom, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use store::format::{Bar, HEADER_LEN, OI_NULL, RECORD_STRIDE};

use super::*;
use crate::session::IST_OFFSET_SECS;

/// A scratch store root of this test's own, removed when it ends.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir().join(format!(
            "brutex-gbf-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _stale = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("a scratch root");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _best_effort = fs::remove_dir_all(&self.0);
    }
}

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).expect("a real day")
}

/// `ZZIDX`, an invented index, in `segment`, at the minute rung.
fn parts(segment: &'static str, month: YearMonth) -> PathParts<'static> {
    PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment,
        symbol: "ZZIDX",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: store::path::FileKind::Bars,
    }
}

fn month(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).expect("a real month")
}

/// One invented minute bar at `minute` past IST midnight on day `number`.
fn bar_at(number: u32, minute: u32) -> Bar {
    let secs = i64::from(number) * 86_400 - IST_OFFSET_SECS + i64::from(minute) * 60;
    Bar {
        ts_micros: secs * 1_000_000,
        open: 1_111_100,
        high: 1_111_200,
        low: 1_111_000,
        close: 1_111_150,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// Every minute `segment` owes in `from..=to` by the calendar of record, but
/// the ones `skip` names.
fn owed_bars(segment: &str, from: Day, to: Day, skip: &dyn Fn(Day, u32) -> bool) -> Vec<Bar> {
    let mut bars = Vec::new();
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let Owes::Windows((windows, count)) = repair_plan::owed(segment, number) else {
            continue;
        };
        let on = Day::from_days(number).expect("a real day");
        for &(open, close) in &windows[..count] {
            for minute in open..close {
                if !skip(on, minute) {
                    bars.push(bar_at(number, minute));
                }
            }
        }
    }
    bars
}

/// The sessions of record of `from..=to`, as the minute rung owes them.
fn sessions(from: Day, to: Day) -> Vec<Day> {
    (from.days_from_epoch()..=to.days_from_epoch())
        .filter(|&n| matches!(repair_plan::owed("INDEX", n), Owes::Windows(_)))
        .map(|n| Day::from_days(n).expect("a real day"))
        .collect()
}

/// Lands `bars` as `parts`' month through the store's own writer.
fn land(root: &Path, parts: PathParts<'_>, bars: &[Bar]) {
    let mut file = store::file::BarFile::open_or_create(
        root,
        StorePath::new(parts).expect("a store path"),
        repair_plan::symbol_id(parts.symbol),
    )
    .expect("a bar file");
    file.append(bars).expect("the bars land");
}

/// 2019-09 of `ZZIDX` whole, less 2019-09-23 12:00-12:04: one interior hole.
fn september_with_a_hole(root: &Path) -> Vec<Bar> {
    let hole = day(2019, 9, 23);
    let bars = owed_bars("INDEX", day(2019, 9, 1), day(2019, 9, 30), &|on, minute| {
        on == hole && (720..725).contains(&minute)
    });
    land(root, parts("INDEX", month(2019, 9)), &bars);
    bars
}

fn whole_window(from: Day, to: Day) -> Window {
    Window::new(from, to).expect("a window")
}

/// **A DAY WORD IS FOUR, TWO AND TWO DIGITS, AND NOTHING ELSE IS A DAY.**
/// Each wrong shape is built at run time from integers, so this file adds no
/// segment-shaped literal (gate 1d).
#[test]
fn a_day_word_is_four_two_two_digits_and_nothing_else() {
    // The shapes that read.
    assert_eq!(
        parse_day(&format!("{:04}-{:02}-{:02}", 2019, 9, 23)),
        Some(day(2019, 9, 23))
    );
    assert_eq!(
        parse_day(&format!("{:04}-{:02}-{:02}", 1970, 1, 1)),
        Some(day(1970, 1, 1)),
        "the first day a Day holds"
    );
    assert_eq!(
        parse_day(&format!("{:04}-{:02}-{:02}", 9999, 12, 31)),
        Some(day(9999, 12, 31)),
        "and the last"
    );
    // A FOURTH PART: the day is not the last word of it.
    assert_eq!(
        parse_day(&format!("{:04}-{:02}-{:02}-{:02}", 2019, 9, 23, 1)),
        None
    );
    // A ONE-DIGIT MONTH, and a one-digit day: no padding is guessed.
    assert_eq!(parse_day(&format!("{:04}-{}-{:02}", 2019, 9, 23)), None);
    assert_eq!(parse_day(&format!("{:04}-{:02}-{}", 2019, 9, 3)), None);
    // A three-digit year, and a five-digit one.
    assert_eq!(parse_day(&format!("{}-{:02}-{:02}", 201, 9, 23)), None);
    assert_eq!(parse_day(&format!("{}-{:02}-{:02}", 20_190, 9, 23)), None);
    // Two parts, one part, nothing.
    assert_eq!(parse_day(&format!("{:04}-{:02}", 2019, 9)), None);
    assert_eq!(parse_day(&format!("{:04}", 2019)), None);
    assert_eq!(parse_day(""), None);
    // The right shape and no such day: 30 February, month 13, day 0.
    assert_eq!(parse_day(&format!("{:04}-{:02}-{:02}", 2019, 2, 30)), None);
    assert_eq!(parse_day(&format!("{:04}-{:02}-{:02}", 2019, 13, 1)), None);
    assert_eq!(parse_day(&format!("{:04}-{:02}-{:02}", 2019, 9, 0)), None);
    // OUT OF THE BOX: four characters a `u16` parses that are not four
    // digits, and a year before the first a Day can name.
    assert_eq!(parse_day(&format!("+{}-{:02}-{:02}", 201, 9, 23)), None);
    assert_eq!(parse_day(&format!("{:04}-{:02}-{:02}", 1969, 12, 31)), None);
    // A letter where a digit belongs.
    assert_eq!(
        parse_day(&format!(
            "{:04}-{}-{:02}",
            2019,
            "0A".to_ascii_lowercase(),
            23
        )),
        None
    );
}

/// **EVERY LINE THAT IS NOT A WELL-FORMED `gap` IS A FAULT, BY ITS NUMBER.**
/// An empty key, a key holding `=`, a day that is no day and a line that is
/// not UTF-8 are each named; another record kind is not a fault and not a
/// pair; only the well-formed line is a pair.
#[test]
fn every_line_that_is_not_a_well_formed_gap_is_a_fault_by_number() {
    let good = format!("gap NSE-INDEX-ZZIDX {:04}-{:02}-{:02} v=1", 2019, 9, 23);
    let empty_key = format!("gap  {:04}-{:02}-{:02} v=1", 2019, 9, 24);
    let keyed_pair = format!("gap K=V {:04}-{:02}-{:02}", 2019, 9, 25);
    let no_such_day = format!("gap NSE-INDEX-ZZIDX {:04}-{:02}-{:02}", 2019, 2, 30);
    let lines: [&[u8]; 7] = [
        good.as_bytes(),             // 1: a pair
        empty_key.as_bytes(),        // 2: an empty key
        keyed_pair.as_bytes(),       // 3: `=` in the key
        b"\xff\xfe NOT UTF-8",       // 4: not text
        no_such_day.as_bytes(),      // 5: no such day
        b"REPAIRED NSE-INDEX-ZZIDX", // 6: another record kind
        b"GAPS ARE NOT A GAP LINE",  // 7: not `gap `, so not a fault
    ];
    let mut ledger: Vec<u8> = Vec::new();
    for line in lines {
        ledger.extend_from_slice(line);
        ledger.push(b'\n');
    }
    let mut have = Have::default();
    scan(&mut have, std::io::Cursor::new(&ledger)).expect("an in-memory read");
    assert_eq!(have.faults, [2, 3, 4, 5], "named by line, each once");
    assert_eq!(have.len(), 1, "the one well-formed line is the one pair");
    assert!(have.holds("NSE-INDEX-ZZIDX", day(2019, 9, 23)));
    assert!(!have.holds("NSE-INDEX-ZZIDX", day(2019, 9, 24)));
    assert_eq!(have.lines, 7, "every whole line is counted read");
    assert_eq!(
        have.read_to,
        u64::try_from(ledger.len()).unwrap(),
        "and every byte of them"
    );
}

/// **A TORN FAULT READ TWICE IS NAMED ONCE.** A last line with no line break
/// is read again by the next scan, under the same number: it is one fault,
/// however often it is read, and it is counted read only once closed.
#[test]
fn a_torn_fault_read_twice_is_named_once() {
    let first = format!("gap NSE-INDEX-ZZIDX {:04}-{:02}-{:02} v=1\n", 2019, 9, 23);
    let torn = format!("gap ZZ=TORN {:04}-{:02}-{:02}", 2019, 9, 24);
    let mut have = Have::default();
    let body = format!("{first}{torn}");
    scan(&mut have, std::io::Cursor::new(body.as_bytes())).expect("an in-memory read");
    assert_eq!(have.faults, [2]);
    assert_eq!(have.lines, 1, "a torn line is not counted read");
    let stopped = usize::try_from(have.read_to).unwrap();
    assert_eq!(stopped, first.len());
    // THE SAME TORN LINE AGAIN, from where the read stopped: still fault 2.
    scan(&mut have, std::io::Cursor::new(&body.as_bytes()[stopped..])).expect("an in-memory read");
    assert_eq!(have.faults, [2], "named once however often it is read");
    assert_eq!(have.lines, 1);
    // CLOSED NOW, AND A SECOND FAULT AFTER IT: both named, in order.
    let closed = format!("{torn}\ngap  {:04}-{:02}-{:02}\n", 2019, 9, 25);
    scan(&mut have, std::io::Cursor::new(closed.as_bytes())).expect("an in-memory read");
    assert_eq!(have.faults, [2, 3]);
    assert_eq!(have.lines, 3);
    assert_eq!(
        have.read_to,
        u64::try_from(first.len() + closed.len()).unwrap()
    );
}

/// **A HELD MONTH OUTSIDE THE WINDOW IS NOT READ, AND ONE THE PLAN REFUSES
/// IS NAMED WITH WHY** — while the month beside them still yields its line.
/// 2019-07 holds a bar and lies outside the window; 2019-08's bar file is
/// not a bar file; 2019-09 holds every owed minute but 2019-09-23 12:00-12:04.
#[test]
fn a_month_outside_the_window_is_skipped_and_an_unreadable_one_is_refused() {
    let scratch = Scratch::new("WINDOW");
    let root = &scratch.0;
    let july = day(2019, 7, 1).days_from_epoch();
    land(root, parts("INDEX", month(2019, 7)), &[bar_at(july, 600)]);
    let garbage = StorePath::new(parts("INDEX", month(2019, 8)))
        .unwrap()
        .to_path_buf(root);
    fs::write(&garbage, b"NOT A BAR FILE, AN INVENTED ONE").expect("a garbage month");
    september_with_a_hole(root);

    let window = whole_window(day(2019, 8, 1), day(2019, 9, 30));
    let found = series(
        root,
        parts("INDEX", month(2019, 9)),
        window,
        &Have::default(),
    );

    assert_eq!(found.key, "NSE-INDEX-ZZIDX");
    assert_eq!(
        found.tally.months, 2,
        "2019-08 and 2019-09 are read; 2019-07 is outside the window"
    );
    assert_eq!(found.tally.refused.len(), 1, "{:?}", found.tally.refused);
    assert!(
        found.tally.refused[0].starts_with("2019-08: "),
        "the refused month is named: {:?}",
        found.tally.refused
    );
    assert_eq!(
        found.tally.sessions,
        u64::try_from(sessions(day(2019, 9, 1), day(2019, 9, 30)).len()).unwrap(),
        "only September's sessions are counted"
    );
    assert_eq!(found.days, [day(2019, 9, 23)]);
    assert_eq!(found.tally.short, 1);
    assert_eq!(found.tally.minutes, 5);
    let line = &found.lines[0];
    assert!(
        line.starts_with("gap NSE-INDEX-ZZIDX 2019-09-23 "),
        "{line}"
    );
    for word in [
        " owed=375 held=370 missing=5 first_missing=12:00 ",
        " fill=interior source=store",
    ] {
        assert!(line.contains(word), "{word} in {line}");
    }
    assert!(
        !line.contains("absence="),
        "an index minute is computed: {line}"
    );

    // THE SAME STORE, A WINDOW THAT HOLDS NO HELD MONTH: nothing is read.
    let none = series(
        root,
        parts("INDEX", month(2019, 9)),
        whole_window(day(2019, 10, 1), day(2019, 10, 31)),
        &Have::default(),
    );
    assert_eq!(none.tally, Tally::default());
    assert!(none.lines.is_empty());
}

/// **A SESSION NO RECORD SIZES IS COUNTED UNJUDGED AND NEVER WRITTEN.** An
/// invented segment no venue table names owes no stated minutes on any
/// session; a month of it holding a bar on its first and its last session
/// owes the ledger no line, and every one of its sessions is counted as
/// unjudged — the two held ones by the plan, the ones between by the ledger.
#[test]
fn a_session_no_record_sizes_is_counted_unjudged_and_never_written() {
    let scratch = Scratch::new("UNSIZED");
    let root = &scratch.0;
    let held = sessions(day(2019, 9, 1), day(2019, 9, 30));
    let (first, last) = (held[0], held[held.len() - 1]);
    land(
        root,
        parts("ZZSEG", month(2019, 9)),
        &[
            bar_at(first.days_from_epoch(), 600),
            bar_at(last.days_from_epoch(), 600),
        ],
    );
    let found = series(
        root,
        parts("ZZSEG", month(2019, 9)),
        whole_window(day(2019, 9, 1), day(2019, 9, 30)),
        &Have::default(),
    );
    let count = u64::try_from(held.len()).unwrap();
    assert!(found.lines.is_empty(), "{:?}", found.lines);
    assert_eq!(found.tally.sessions, count);
    assert_eq!(
        found.tally.short,
        count - 2,
        "every session between the two"
    );
    assert_eq!(found.tally.length_unjudged, count);
    assert_eq!(
        (
            found.tally.before_first,
            found.tally.frontier,
            found.tally.minutes
        ),
        (0, 0, 0)
    );
    assert!(found.tally.refused.is_empty());
}

/// **A RECORD THAT WILL NOT READ REFUSES ITS MONTH, AND NONE OF IT IS
/// WRITTEN.** The plan reads only the first and last held minutes of the
/// short session; the ledger reads every one of its stamps, and one of them,
/// 13:00, is damaged on disk after it landed. The same store read before the
/// damage yields the line, so the refusal is the damage alone.
#[test]
fn a_stamp_that_will_not_read_refuses_its_month_and_writes_none_of_it() {
    let scratch = Scratch::new("DAMAGED");
    let root = &scratch.0;
    let bars = september_with_a_hole(root);
    let window = whole_window(day(2019, 9, 1), day(2019, 9, 30));
    let before = series(
        root,
        parts("INDEX", month(2019, 9)),
        window,
        &Have::default(),
    );
    assert_eq!(before.days, [day(2019, 9, 23)], "the control: one line");

    let target = bar_at(day(2019, 9, 23).days_from_epoch(), 780).ts_micros;
    let index = bars
        .iter()
        .position(|bar| bar.ts_micros == target)
        .expect("13:00 is held");
    let path = StorePath::new(parts("INDEX", month(2019, 9)))
        .unwrap()
        .to_path_buf(root);
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .expect("the bar file");
    let at = HEADER_LEN + u64::try_from(index).unwrap() * RECORD_STRIDE + 8;
    file.seek(SeekFrom::Start(at))
        .expect("the damaged record's place");
    file.write_all(&[0x5a]).expect("one damaged byte");
    drop(file);

    let after = series(
        root,
        parts("INDEX", month(2019, 9)),
        window,
        &Have::default(),
    );
    assert!(after.lines.is_empty(), "{:?}", after.lines);
    assert!(after.days.is_empty());
    assert_eq!(after.tally.minutes, 0);
    assert_eq!(after.tally.refused.len(), 1, "{:?}", after.tally);
    assert!(
        after.tally.refused[0].starts_with("2019-09: "),
        "{:?}",
        after.tally.refused
    );
}

/// **A LEDGER THAT CANNOT BE APPENDED WRITES NOTHING AND SAYS WHY**, and the
/// read state is kept as it was: `audit` is a file, so the ledger's folder
/// cannot be made.
#[test]
fn a_ledger_that_cannot_be_appended_writes_nothing_and_keeps_its_read() {
    let scratch = Scratch::new("UNWRITABLE");
    let root = &scratch.0;
    let audit = root.join("audit");
    fs::write(&audit, "NOT A FOLDER").expect("a file where the folder goes");
    let mut found = Series {
        key: "NSE-INDEX-ZZIDX".to_owned(),
        lines: vec![format!(
            "gap NSE-INDEX-ZZIDX {} v=1 rung=1min owed=375 held=370 missing=5",
            day(2019, 9, 23)
        )],
        days: vec![day(2019, 9, 23)],
        tally: Tally::default(),
    };
    let mut have = Have::default();
    let why = write(root, Vendor::Zerodha, &mut found, &mut have).expect_err("no folder");
    assert!(why.contains(&audit.display().to_string()), "{why}");
    assert!(have.is_empty(), "nothing is recorded as held");
    assert_eq!((have.read_to, have.lines), (0, 0));
    assert_eq!(found.tally.raced, 0);
    assert_eq!(
        fs::read_to_string(&audit).expect("still the file"),
        "NOT A FOLDER"
    );
}

/// **A LINE WRITTEN IS FOUND AGAIN AND NEVER WRITTEN TWICE**, by the same
/// `Have` and by a fresh read of the ledger: the whole path, for the quick
/// check of this module, end to end.
#[test]
fn a_written_line_is_held_and_a_second_run_writes_nothing() {
    let scratch = Scratch::new("ROUNDTRIP");
    let root = &scratch.0;
    september_with_a_hole(root);
    let window = whole_window(day(2019, 9, 1), day(2019, 9, 30));
    let mut have = ledgered(root, Vendor::Zerodha).expect("no ledger is an empty one");
    assert!(have.is_empty());
    let mut found = series(root, parts("INDEX", month(2019, 9)), window, &have);
    assert_eq!(found.lines.len(), 1);
    assert_eq!(write(root, Vendor::Zerodha, &mut found, &mut have), Ok(1));
    assert!(have.holds(&found.key, day(2019, 9, 23)));
    let ledger = ledger_path(root, Vendor::Zerodha);
    let text = fs::read_to_string(&ledger).expect("the ledger");
    assert_eq!(text, format!("{}\n", found.lines[0]));
    assert_eq!(have.read_to, u64::try_from(text.len()).unwrap());
    assert_eq!(have.lines, 1);

    // THE SAME HAVE: the line is already held and nothing is found.
    let again = series(root, parts("INDEX", month(2019, 9)), window, &have);
    assert!(again.lines.is_empty());
    assert_eq!(again.tally.already, 1);
    // A FRESH READ: the same answer from the ledger itself.
    let read = ledgered(root, Vendor::Zerodha).expect("the ledger reads");
    assert_eq!(read.len(), 1);
    assert!(read.faults.is_empty());
    // AND A WRITE OF A LINE ANOTHER RUN ALREADY WROTE IS DROPPED AS RACED.
    let mut stale = series(
        root,
        parts("INDEX", month(2019, 9)),
        window,
        &Have::default(),
    );
    let mut fresh = Have::default();
    assert_eq!(write(root, Vendor::Zerodha, &mut stale, &mut fresh), Ok(0));
    assert_eq!(stale.tally.raced, 1);
    assert_eq!(
        fs::read_to_string(&ledger).expect("the ledger"),
        text,
        "not one byte more"
    );
}

/// **A WANTED SESSION THAT HOLDS EVERY OWED MINUTE WRITES NO LINE**, asked of
/// [`month_lines`] directly: the plan never hands it one, and a caller that
/// did would get no line rather than a line of nothing. A closed day handed
/// to it is counted unjudged; the month's file is opened once, for the first
/// session that needs its stamps, and read again for the next.
#[test]
fn a_wanted_session_that_holds_every_owed_minute_writes_no_line() {
    let scratch = Scratch::new("WHOLE");
    let root = &scratch.0;
    september_with_a_hole(root);
    let whole = sessions(day(2019, 9, 1), day(2019, 9, 30))
        .into_iter()
        .find(|on| *on > day(2019, 9, 23))
        .expect("a session after the hole");
    let sunday = day(2019, 9, 22);
    let mut out = Series {
        key: "NSE-INDEX-ZZIDX".to_owned(),
        ..Series::default()
    };
    month_lines(
        root,
        parts("INDEX", month(2019, 9)),
        repair_plan::symbol_id("ZZIDX"),
        (
            &[
                (sunday, Missing::Interior),
                (whole, Missing::Interior),
                (day(2019, 9, 23), Missing::Interior),
            ],
            false,
        ),
        &Have::default(),
        &mut out,
    )
    .expect("the month reads");
    assert_eq!(out.tally.short, 3, "every wanted session is counted");
    assert_eq!(
        out.tally.length_unjudged, 1,
        "the Sunday owes no stated minute"
    );
    assert_eq!(
        out.days,
        [day(2019, 9, 23)],
        "the whole session wrote nothing"
    );
    assert_eq!(out.tally.minutes, 5);
}

/// **THE SERIES' FRONT AND ITS FRONTIER ARE COUNTED AND NEVER WRITTEN, AND A
/// SHARE'S LINE SAYS ITS ABSENCE MAY BE NO TRADE.** `ZZIDX` lacks its first
/// session whole (before its first bar: a listing or a hole the bars cannot
/// tell apart), August's last fifteen minutes (a tail before the newest
/// month, written) and its newest month's last session whole (the frontier
/// a pull extends); `ZZSHR`, an invented share, lacks 12:00-12:04 on
/// 2019-09-23. A store with no month of a series owes nothing, and a ledger
/// name holding a folder is refused by name.
#[test]
fn the_front_and_the_frontier_are_counted_and_a_share_says_no_trade() {
    let scratch = Scratch::new("EDGES");
    let root = &scratch.0;
    let august = sessions(day(2019, 8, 1), day(2019, 8, 31));
    let september = sessions(day(2019, 9, 1), day(2019, 9, 30));
    let (first, last) = (august[0], september[september.len() - 1]);
    let august_last = august[august.len() - 1];
    let hole = day(2019, 9, 23);
    let skip = move |on: Day, minute: u32| {
        on == first
            || on == last
            || (on == august_last && minute >= 900)
            || (on == hole && (720..725).contains(&minute))
    };
    land(
        root,
        parts("INDEX", month(2019, 8)),
        &owed_bars("INDEX", day(2019, 8, 1), day(2019, 8, 31), &skip),
    );
    land(
        root,
        parts("INDEX", month(2019, 9)),
        &owed_bars("INDEX", day(2019, 9, 1), day(2019, 9, 30), &skip),
    );
    let window = whole_window(day(2019, 8, 1), day(2019, 9, 30));
    let found = series(
        root,
        parts("INDEX", month(2019, 9)),
        window,
        &Have::default(),
    );
    assert_eq!(found.tally.months, 2);
    assert_eq!(found.tally.before_first, 1, "{:?}", found.tally);
    assert_eq!(found.tally.frontier, 1, "{:?}", found.tally);
    // A TAIL IN A MONTH BEFORE THE NEWEST is a stored hole, and is written.
    assert_eq!(found.days, [august_last, hole]);
    assert!(found.lines[0].contains("fill=tail"), "{}", found.lines[0]);
    assert!(
        found.lines[0].contains(" missing=30 first_missing=15:00 "),
        "{}",
        found.lines[0]
    );

    // A SHARE: the same hole, and the line says what the store cannot.
    let share = |month| PathParts {
        symbol: "ZZSHR",
        ..parts("CASH", month)
    };
    land(
        root,
        share(month(2019, 9)),
        &owed_bars("CASH", day(2019, 9, 1), day(2019, 9, 30), &|on, minute| {
            on == hole && (720..725).contains(&minute)
        }),
    );
    let traded = series(root, share(month(2019, 9)), window, &Have::default());
    assert_eq!(traded.key, "NSE-CASH-ZZSHR");
    assert_eq!(traded.days, [hole]);
    assert!(
        traded.lines[0].ends_with(" absence=may_be_no_trade"),
        "{}",
        traded.lines[0]
    );

    // NO MONTH OF THE SERIES: nothing owed, the key still named.
    let none = series(
        root,
        PathParts {
            symbol: "ZZNONE",
            ..parts("INDEX", month(2019, 9))
        },
        window,
        &Have::default(),
    );
    assert_eq!(none.key, "NSE-INDEX-ZZNONE");
    assert_eq!(none.tally, Tally::default());

    // A FOLDER AT THE LEDGER'S NAME is refused by name, never read as empty.
    let ledger = ledger_path(root, Vendor::Zerodha);
    fs::create_dir_all(&ledger).expect("a folder at the ledger's name");
    let why = ledgered(root, Vendor::Zerodha).expect_err("a folder is no ledger");
    assert!(why.starts_with(&ledger.display().to_string()), "{why}");
}
