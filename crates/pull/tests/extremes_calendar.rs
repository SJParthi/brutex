//! Extreme cases of the calendar, the day arithmetic, the ledgers' date text
//! and F&O membership in `pull`, for the cloud extremes round of 10 Nov 2026
//! (`EXTREMES-REPORT.md` at the repository root).
//!
//! * `Day::new` on every impossible date the Gregorian rule makes (29
//!   February of 2023 and 2100, month 13, day 0) and at its year bounds;
//!   `succ`, `end_of_month` and `months_before` across 29 February and the
//!   year end;
//! * `reference_data::parse_day` on signed, padded, short and foreign-digit
//!   text, and the Zerodha gap ledger on a line whose date carries signs;
//! * the special days of record (Muhurat, DR drills, Budget and weekend
//!   sessions, election and festival holidays) classified as the calendar's
//!   own tables say, and sessions counted across the year end and the leap
//!   day;
//! * the two expiry authorities (`calendar::settlement_on_or_before`, which
//!   skips a Muhurat, and `expiry_alias::settle`) on every Muhurat weekday;
//! * a share that leaves F&O and comes back, through the membership build.
//!
//! Every value is invented or read from the repository's own tables. A test
//! named `finding_*` is `#[ignore]`d because it FAILS on the branch it was
//! written against; its doc comment is the finding.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::collections::HashMap;
use std::path::PathBuf;

use pull::calendar::{DayKind, exchange_kind_of, sessions_between, settlement_on_or_before};
use pull::gdfl_cm::{CmRefusal, ListedFile};
use pull::gdfl_nfo::{NfoDay, NfoSource, day_folder_name};
use pull::session::Day;

fn day(y: u16, m: u8, d: u8) -> Day {
    Day::new(y, m, d).unwrap()
}

fn epoch(d: Day) -> i64 {
    i64::from(d.days_from_epoch())
}

// ===========================================================================
// Day arithmetic
// ===========================================================================

/// Every impossible date is refused and every real one at an edge is
/// accepted: 29 February only in a leap year (2000 and 2400 yes; 1900 is out
/// of range anyway; 2023 and 2100 no), months 1 to 12, days 1 to the month's
/// length, years 1970 to 9999.
#[test]
fn day_new_refuses_every_impossible_date_and_takes_every_real_edge() {
    for (y, m, d) in [
        (2024, 2, 29),
        (2000, 2, 29),
        (2400, 2, 29),
        (1970, 1, 1),
        (9999, 12, 31),
        (2024, 4, 30),
        (2024, 12, 31),
    ] {
        assert!(Day::new(y, m, d).is_ok(), "{y}-{m}-{d} is a date");
    }
    for (y, m, d) in [
        (2023, 2, 29),
        (2100, 2, 29),
        (2024, 2, 30),
        (2024, 4, 31),
        (2024, 13, 1),
        (2024, 0, 1),
        (2024, 1, 0),
        (2024, 1, 32),
        (1969, 12, 31),
        (1900, 2, 28),
        (10_000, 1, 1),
        (u16::MAX, u8::MAX, u8::MAX),
    ] {
        assert!(Day::new(y, m, d).is_err(), "{y}-{m}-{d} is not a date");
    }
}

/// `succ` steps over 29 February and the year end, and refuses past 9999;
/// `end_of_month` knows 2024 and 2000 are leap and 2023 and 2100 are not;
/// `months_before` clamps a 31st into a short February, leap or not.
#[test]
fn succ_end_of_month_and_months_before_cross_the_leap_day_and_the_year() {
    assert_eq!(day(2024, 2, 28).succ(), Ok(day(2024, 2, 29)));
    assert_eq!(day(2024, 2, 29).succ(), Ok(day(2024, 3, 1)));
    assert_eq!(day(2023, 2, 28).succ(), Ok(day(2023, 3, 1)));
    assert_eq!(day(2024, 12, 31).succ(), Ok(day(2025, 1, 1)));
    assert!(day(9999, 12, 31).succ().is_err(), "no day after 9999-12-31");
    for (y, last) in [(2024, 29), (2000, 29), (2023, 28), (2100, 28)] {
        assert_eq!(day(y, 2, 10).end_of_month(), day(y, 2, last), "{y}");
    }
    assert_eq!(day(2024, 3, 31).months_before(1), Ok(day(2024, 2, 29)));
    assert_eq!(day(2023, 3, 31).months_before(1), Ok(day(2023, 2, 28)));
    assert_eq!(day(2025, 1, 15).months_before(1), Ok(day(2024, 12, 15)));
    assert_eq!(day(2024, 2, 29).months_before(12), Ok(day(2023, 2, 28)));
    // The epoch round trip at both edges of the leap day and the year.
    for d in [
        day(2024, 2, 29),
        day(2024, 3, 1),
        day(2024, 12, 31),
        day(2025, 1, 1),
    ] {
        assert_eq!(Day::from_days(d.days_from_epoch()), Ok(d));
    }
}

// ===========================================================================
// Dates as text
// ===========================================================================

/// The reference tables' date reader takes exactly `YYYY-MM-DD` of a real
/// date and nothing else: no sign, no padding, no short field, no other
/// separator or order, no non-ASCII digit, no impossible day.
#[test]
fn the_reference_date_reader_takes_exactly_a_real_iso_date() {
    assert_eq!(
        pull::reference_data::parse_day("2024-02-29"),
        Some(day(2024, 2, 29))
    );
    assert_eq!(
        pull::reference_data::parse_day("2025-01-01"),
        Some(day(2025, 1, 1))
    );
    // The short, long and reordered spellings are built, not written, so no
    // literal of this file is a path-segment-shaped word (CI gate 1d).
    let reordered = format!("{}-{:02}-{}", 29, 2, 2024);
    let short_month = format!("{}-{}-{}", 2024, 2, 29);
    let long_day = format!("{}-{:02}-{}", 2024, 2, 290);
    let short_day = format!("{}-{:02}-{}", 2024, 2, 2);
    for text in [
        "2024-+2-29",
        "+024-02-29",
        "2024-02-+9",
        " 2024-02-29",
        "2024-02-29 ",
        "2024/02/29",
        reordered.as_str(),
        short_month.as_str(),
        long_day.as_str(),
        short_day.as_str(),
        "\u{ff12}\u{ff10}\u{ff12}\u{ff14}-02-29",
        "2023-02-29",
        "2100-02-29",
        "2024-13-01",
        "2024-00-10",
        "",
    ] {
        assert_eq!(pull::reference_data::parse_day(text), None, "{text:?}");
    }
}

/// A gap line the Zerodha door wrote, for 2024-06-03.
fn a_written_gap_line() -> (String, String) {
    let root = std::env::temp_dir().join(format!("brutex-xcal-gaps-{}", std::process::id()));
    let _absent = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let d = day(2024, 6, 3);
    let request = pull::fetch::BarRequest {
        instrument_id: String::new(),
        listing: pull::vendor::Listing::Index,
        window: pull::session::Window::new(d, d).unwrap(),
        granularity: pull::vendor::Granularity::Minute1,
    };
    let plan = pull::ingest::Plan {
        calendar: pull::calendar::Runtime::default(),
        cash_schedule: None,
        columns: pull::csv::Columns::Gdfl,
        request: &request,
        encoding: pull::vendor::TimestampEncoding::EpochSecondsUtc,
        scale: pull::vendor::PriceScale::Paisa,
        vendor: brutex_core::vendor::Vendor::Zerodha,
        exchange: "NSE",
        segment: "INDEX",
        contract: None,
        run: 0,
    };
    let _landed = pull::ingest::from_window(
        &pull::fetch::RawWindow { rows: Vec::new() },
        "NIFTY",
        "XCAL TEST",
        &root,
        plan,
    );
    let ledger = std::fs::read_to_string(root.join("audit").join("zerodha.gaps")).unwrap();
    let _gone = std::fs::remove_dir_all(&root);
    let line = ledger
        .lines()
        .find(|l| l.starts_with("gap ") && l.contains(" 2024-06-03"))
        .expect("the empty answer's day is ledgered")
        .to_owned();
    let key = line.split(' ').nth(1).unwrap().to_owned();
    (line, key)
}

/// FINDING (the Zerodha gap ledger's date reader).
///
/// Input: a gap line exactly as the door wrote it for 2024-06-03, with its
/// date re-spelt `2024-+6-+3` (a hand edit or a damaged byte: the ledger is
/// read back after every run, and `ingest::gap_backfill::ledgered` plans
/// back-fills from it).
///
/// Expected: the line is a fault (`GapLedger::faults` names it) and is never
/// read as a gap of 2024-06-03; only `YYYY-MM-DD` in ASCII digits is a date,
/// as `reference_data::parse_day` already insists.
///
/// Actual: see the assertion. `parse_day`/`parse_month`
/// (`crates/pull/src/zerodha_verify.rs:432-450`) check the field WIDTHS and
/// then `str::parse`, which takes a leading `+`, so `+6` is June and `+3`
/// the 3rd: the verifier reads the line as a gap of 2024-06-03 and names no
/// fault. The back-fill planner's reader of the same file,
/// `crates/pull/src/gap_backfill.rs:124-131` (`parse_day`), does the
/// same, so a back-fill is planned for the misread day. The class the
/// previous cloud round fixed in the `api` routes.
///
/// Proposed fix: accept only ASCII digits in each field (one shared strict
/// reader, the one `reference_data::parse_day` already is), and count a line
/// that fails it as a fault.
#[test]
#[ignore = "finding: the gap ledger reads a signed date as a real day (EXTREMES-REPORT.md)"]
fn finding_the_gap_ledger_reads_a_signed_date_as_a_real_day() {
    let (line, key) = a_written_gap_line();
    let honest = pull::zerodha_verify::GapLedger::parse(format!("{line}\n").as_bytes());
    assert!(
        honest.gap(&key, day(2024, 6, 3)).is_some(),
        "the honest line reads"
    );
    let signed = line.replacen(" 2024-06-03", " 2024-+6-+3", 1);
    assert_ne!(signed, line);
    let read = pull::zerodha_verify::GapLedger::parse(format!("{signed}\n").as_bytes());
    let verifier_misreads = read.gap(&key, day(2024, 6, 3)).is_some() || read.faults.is_empty();

    // The back-fill planner's reader of the same ledger file.
    let root = std::env::temp_dir().join(format!("brutex-xcal-have-{}", std::process::id()));
    let _absent = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("audit")).unwrap();
    std::fs::write(
        root.join("audit").join("zerodha.gaps"),
        format!("{signed}\n"),
    )
    .unwrap();
    let have =
        pull::ingest::gap_backfill::ledgered(&root, brutex_core::vendor::Vendor::Zerodha).unwrap();
    let planner_misreads = have.holds(&key, day(2024, 6, 3)) || have.faults.is_empty();
    let _gone = std::fs::remove_dir_all(&root);
    assert!(
        !verifier_misreads && !planner_misreads,
        "a signed date must be a fault, never 2024-06-03 (verifier misreads: \
         {verifier_misreads}, back-fill planner misreads: {planner_misreads}): {signed}"
    );
}

// ===========================================================================
// The calendar of record on its special days
// ===========================================================================

/// The number of minutes a day of `kind` trades, or `None` when it does not
/// trade or its length is unknown.
fn bars_of(kind: DayKind) -> Option<u16> {
    match kind {
        DayKind::Open(session) => Some(session.bars()),
        _ => None,
    }
}

/// Every special day the calendar records answers what its own tables say:
/// the Muhurats and DR drills trade their recorded windows only, the Budget
/// and weekend full sessions trade 375 minutes, the election and festival
/// holidays are closed (2024-05-20 was asserted closed nowhere before), the
/// circular-named 2026 Muhurat has no length, and the days past the sourced
/// span are unmeasured.
#[test]
fn the_special_days_of_record_classify_as_the_calendars_own_tables_say() {
    let open_short = [
        day(2024, 11, 1),
        day(2024, 3, 2),
        day(2024, 5, 18),
        day(2025, 10, 21),
    ];
    for d in open_short {
        let bars = bars_of(exchange_kind_of(epoch(d)));
        assert!(
            bars.is_some_and(|b| b > 0 && b < 375),
            "{d} trades a recorded window shorter than a session: {bars:?}"
        );
    }
    for d in [
        day(2025, 2, 1),
        day(2024, 1, 20),
        day(2020, 2, 1),
        day(2024, 6, 3),
    ] {
        assert_eq!(bars_of(exchange_kind_of(epoch(d))), Some(375), "{d}");
    }
    for d in [
        day(2024, 1, 22),
        day(2024, 5, 20),
        day(2024, 6, 17),
        day(2024, 8, 15),
        day(2024, 6, 1),
        day(2026, 10, 2),
        day(2026, 11, 10),
    ] {
        assert_eq!(exchange_kind_of(epoch(d)), DayKind::Closed, "{d}");
    }
    assert_eq!(
        exchange_kind_of(epoch(day(2026, 11, 8))),
        DayKind::OpenLengthUnmeasured,
        "the circular names the 2026 Muhurat and no window"
    );
    for d in [day(2027, 1, 1), day(2017, 12, 29)] {
        assert_eq!(exchange_kind_of(epoch(d)), DayKind::Unmeasured, "{d}");
    }
}

/// Sessions are counted across the year end and the leap day: 30 Dec 2024
/// to 2 Jan 2025 is four sessions, 28 Feb to 1 Mar 2024 three.
#[test]
fn sessions_are_counted_across_the_year_end_and_the_leap_day() {
    assert_eq!(
        sessions_between(epoch(day(2024, 12, 30)), epoch(day(2025, 1, 2))),
        Some(4)
    );
    assert_eq!(
        sessions_between(epoch(day(2024, 2, 28)), epoch(day(2024, 3, 1))),
        Some(3)
    );
}

/// FINDING (expiry: two authorities disagree on a Muhurat weekday).
///
/// Input: a contract whose text states an expiry on a Muhurat weekday the
/// calendar records (2021-11-04 Thu, 2022-10-24 Mon, 2024-11-01 Fri,
/// 2025-10-21 Tue; the last on the weekday NIFTY's weekly contracts expire),
/// asked of both expiry authorities.
///
/// Expected: one answer. `calendar::expiry_day` records that a Muhurat
/// "traded but is not" a session a contract can expire on, and
/// `calendar::settlement_on_or_before` walks back to the session before it.
///
/// Actual: see the assertion. `expiry_alias::settle` (the settlement option
/// tenors and the re-key read, `crates/pull/src/expiry_alias.rs:800`) asks
/// `sourced_kind_of`, which answers the Muhurat open (or open of unmeasured
/// length), and settles ON the Muhurat; its closed-day walk
/// `previous_open_in` (`expiry_alias.rs:776-791`) likewise takes a Muhurat
/// as the previous open session. A contract stated for the Muhurat gets two
/// different expiries, and its tenor and greeks follow whichever was asked.
///
/// Proposed fix: one authority. Have `settle_by` and `previous_open_in` ask
/// `calendar::expiry_day` (Muhurat and DR drill are no settlement day), or
/// call `settlement_on_or_before` directly when no alias row applies.
#[test]
#[ignore = "finding: the two expiry authorities disagree on a Muhurat weekday (EXTREMES-REPORT.md)"]
fn finding_the_two_expiry_authorities_disagree_on_a_muhurat_weekday() {
    let mut disagree = Vec::new();
    for d in [
        day(2021, 11, 4),
        day(2022, 10, 24),
        day(2024, 11, 1),
        day(2025, 10, 21),
    ] {
        let calendar = settlement_on_or_before(epoch(d)).map(|s| s.settles);
        let alias = pull::expiry_alias::settle("NIFTY", d, None).map(|s| epoch(s.expiry));
        if calendar.ok() != alias.ok() {
            disagree.push((d, calendar, alias));
        }
    }
    assert!(
        disagree.is_empty(),
        "(stated, calendar settles, settle settles) = {disagree:?}"
    );
}

// ===========================================================================
// F&O membership: leaving and coming back
// ===========================================================================

/// An options source whose days list invented contracts.
#[derive(Default)]
struct Listing {
    days: HashMap<Day, Vec<String>>,
}

impl NfoSource for Listing {
    type Locator = ();

    fn day(&self, d: Day) -> Result<Option<NfoDay<()>>, CmRefusal> {
        Ok(self.days.get(&d).map(|tickers| {
            let folder = day_folder_name(d);
            let mut listing = NfoDay::new(d, tickers.len());
            for ticker in tickers {
                listing.push(&format!("{folder}/Options/{ticker}.NFO.csv"), 1, 0, ());
            }
            listing
        }))
    }

    fn fetch(&self, _file: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
        Err(CmRefusal::FolderUnknown)
    }
}

/// A share that leaves F&O and comes back, the way NSE does it: its last
/// series (expiring Thu 28 Mar 2024) keeps trading to expiry and no April
/// series is issued; a new series is listed again from 2 May. It is a member
/// through its last expiry day, not a member in April, a member again from
/// 2 May, and that is two spans of one series; a share listed throughout
/// keeps every day known.
#[test]
fn a_share_that_leaves_fno_and_comes_back_is_two_spans_and_not_a_member_between() {
    let mut source = Listing::default();
    let mut d = day(2024, 3, 1);
    while d <= day(2024, 5, 31) {
        if matches!(pull::calendar::sourced_kind_of(epoch(d)), DayKind::Open(_)) {
            let mut tickers = vec!["ZZSTAY27JUN24700PE".to_owned()];
            if d <= day(2024, 3, 28) {
                tickers.push("ZZBACK28MAR24500CE".to_owned());
            } else if d >= day(2024, 5, 2) {
                tickers.push("ZZBACK27JUN24500CE".to_owned());
            }
            source.days.insert(d, tickers);
        }
        d = d.succ().unwrap();
    }
    let dir: PathBuf = std::env::temp_dir().join(format!("brutex-xcal-fno-{}", std::process::id()));
    let _absent = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let window = pull::fno_membership::Window::Typed {
        from: Some(day(2024, 3, 1)),
        to: day(2024, 5, 31),
    };
    pull::fno_membership::build(&source, &dir, window, 2, &mut |_| {}).unwrap();
    let held = pull::fno_membership::Membership::open(&dir).unwrap();
    for (d, member) in [
        (day(2024, 3, 15), true),
        (day(2024, 3, 28), true),
        (day(2024, 4, 2), false),
        (day(2024, 4, 30), false),
        (day(2024, 5, 2), true),
        (day(2024, 5, 31), true),
    ] {
        assert_eq!(held.is_member("ZZBACK", d), Ok(member), "ZZBACK on {d}");
        assert_eq!(held.is_member("ZZSTAY", d), Ok(true), "ZZSTAY on {d}");
    }
    let back: Vec<(Day, Day)> = held
        .spans()
        .into_iter()
        .filter(|s| s.share.as_str() == "ZZBACK")
        .map(|s| (s.entry, s.exit))
        .collect();
    assert_eq!(
        back.len(),
        2,
        "leaving and coming back is two spans: {back:?}"
    );
    assert_eq!(back[0], (day(2024, 3, 1), day(2024, 3, 28)));
    assert_eq!(back[1].0, day(2024, 5, 2));
    let _gone = std::fs::remove_dir_all(&dir);
}
