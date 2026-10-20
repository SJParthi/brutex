//! `cli calendar-days YEAR` and `cli calendar-days check` (CAL-1,
//! D-5150..D-5158). The year page reads tables only; the check is driven
//! against scratch stores built from invented bars (prices of a few rupees,
//! which no index ever printed) and invented census rows. No vendor row.

#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

use std::path::{Path, PathBuf};

use brutex_core::instrument::{Contract, Exchange, InstrumentKey, Segment};
use brutex_core::symbol::Symbol;
use brutex_core::vendor::Vendor;
use pull::calendar::{self, DayKind, NotTradable};
use pull::manifest::{Closes, Entry, EntryKey, Held};
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

use super::*;

fn run(words: &[&str]) -> (u8, String) {
    let args: Vec<String> = words.iter().map(|w| (*w).to_owned()).collect();
    let mut out = String::new();
    let code = crate::run(&args, &mut out);
    (code, out)
}

fn at(y: u16, m: u8, d: u8) -> i64 {
    i64::from(
        pull::session::Day::new(y, m, d)
            .expect("a real day")
            .days_from_epoch(),
    )
}

// ── `calendar-days YEAR` ──

/// **2024 LISTS EVERY EXCLUDED SESSION WITH ITS REASON AND SOURCE, AND THE
/// TOTALS ADD UP.** The four sessions are the charter's and the NSE
/// circulars' (D-5152): 2024-01-20, the two drills and the Muhurat.
#[test]
fn calendar_days_2024_lists_the_excluded_sessions_and_totals() {
    let (code, out) = run(&["calendar-days", "2024"]);
    assert_eq!(code, crate::OK, "{out}");
    for (day, reason, source) in [
        ("2024-01-20", "WeekendFullSession", "NSE/MSD/60340"),
        ("2024-03-02", "DisasterRecoveryDrill", "NSE/MSD/60677"),
        ("2024-05-18", "DisasterRecoveryDrill", "NSE/MSD/61893"),
        ("2024-11-01", "Muhurat", "NSE/CMTR/64628"),
    ] {
        let row: Vec<&str> = out.lines().filter(|l| l.starts_with(day)).collect();
        assert_eq!(row.len(), 1, "{day}: {out}");
        assert!(
            row[0].contains(reason) && row[0].contains(source),
            "{}",
            row[0]
        );
    }
    // The Ram Mandir closure is a listed holiday; an ordinary Sunday is
    // counted, not listed; an ordinary Friday is neither.
    assert!(
        out.lines()
            .any(|l| l.starts_with("2024-01-22  Mon  Holiday")),
        "{out}"
    );
    assert!(!out.lines().any(|l| l.starts_with("2024-01-21")), "{out}");
    assert!(!out.lines().any(|l| l.starts_with("2024-01-19")), "{out}");
    assert!(out.contains("  excluded sessions          4"), "{out}");
    assert!(out.contains("  days in the year         366"), "{out}");
    assert!(out.contains("  unexplained                0"), "{out}");
    assert!(
        out.contains("Holidays of 2024: measured from the store"),
        "{out}"
    );
    assert!(
        out.contains("Special-session notices recorded: 4 (0 untimed)"),
        "{out}"
    );
    let counts = calendar::year_exclusions(2024).expect("2024").counts;
    let listed = out.lines().filter(|l| l.starts_with("2024-")).count();
    assert_eq!(
        listed,
        usize::try_from(counts.excluded_sessions() + counts.count(NotTradable::Holiday))
            .expect("small")
    );
    assert!(out.contains(&format!("  tradable sessions       {:>4}", counts.tradable)));
}

/// **A HALT DAY CARRIES ITS LANE; A MEASURED SHORT DAY IS NOT EXCLUDED.**
/// 2022-03-07 lost minutes in the vendors' answers, and the exchange held
/// its whole session, so it stays tradable and is not listed (judge, 16 Oct
/// 2026).
#[test]
fn calendar_days_2020_to_2022_name_halts_and_keep_short_days_tradable() {
    let (code, out) = run(&["calendar-days", "2020"]);
    assert_eq!(code, crate::OK, "{out}");
    for day in ["2020-03-13", "2020-03-23"] {
        let row = out.lines().find(|l| l.starts_with(day)).unwrap_or_default();
        assert!(
            row.contains("MarketHalt") && row.contains("suspected"),
            "{row}"
        );
    }
    let budget = out
        .lines()
        .find(|l| l.starts_with("2020-02-01"))
        .unwrap_or_default();
    assert!(budget.contains("BudgetWeekend") && budget.contains("NSE/CMTR/43290"));
    let (code, out) = run(&["calendar-days", "2022"]);
    assert_eq!(code, crate::OK, "{out}");
    assert!(!out.lines().any(|l| l.starts_with("2022-03-07")), "{out}");
    assert_eq!(calendar::tradable_day(at(2022, 3, 7)), Ok(()));
    let (_, out) = run(&["calendar-days", "2021"]);
    let outage = out
        .lines()
        .find(|l| l.starts_with("2021-02-24"))
        .unwrap_or_default();
    assert!(
        outage.contains("SystemsOutage") && outage.contains("market-wide halt (sourced)"),
        "{outage}"
    );
}

/// **THE UNTIMED 2026 MUHURAT IS EXCLUDED AND SAYS ITS WINDOW IS
/// UNVERIFIED**, 2018's Muhurat cites its circular, and each page names its
/// year's holiday circular.
#[test]
fn calendar_days_cites_the_circulars_and_their_gaps() {
    let (code, out) = run(&["calendar-days", "2026"]);
    assert_eq!(code, crate::OK, "{out}");
    let muhurat = out
        .lines()
        .find(|l| l.starts_with("2026-11-08"))
        .unwrap_or_default();
    assert!(
        muhurat.contains("Muhurat") && muhurat.contains("UNVERIFIED"),
        "{muhurat}"
    );
    assert!(
        out.contains("Holidays of 2026: NSE circular NSE/CMTR/71775"),
        "{out}"
    );
    assert!(out.contains("(1 untimed)"), "{out}");
    let (_, out) = run(&["calendar-days", "2018"]);
    let muhurat = out
        .lines()
        .find(|l| l.starts_with("2018-11-07"))
        .unwrap_or_default();
    assert!(muhurat.contains("NSE/CMTR/39216"), "{muhurat}");
    let republic = out
        .lines()
        .find(|l| l.starts_with("2018-01-26"))
        .unwrap_or_default();
    assert!(
        republic.contains("Holiday") && republic.contains("Republic"),
        "{republic}"
    );
}

/// **A YEAR OUTSIDE THE RECORD IS REFUSED BY NAME**, a word that is not a
/// year is a misuse with the usage, and the wrong arity is named.
#[test]
fn calendar_days_refuses_what_it_cannot_count() {
    for year in ["2027", "2017"] {
        let (code, out) = run(&["calendar-days", year]);
        assert_eq!(code, crate::FAILED, "{out}");
        assert!(
            out.starts_with(&format!(
                "refused: {year} is not wholly inside the calendar of record"
            )),
            "{out}"
        );
        assert!(out.contains("no NSE holiday circular"), "{out}");
        assert!(!out.contains("usage:"), "{out}");
    }
    let (code, out) = run(&["calendar-days", "1969"]);
    assert_eq!(code, crate::FAILED, "{out}");
    assert!(
        out.contains("1969 is not a year this calendar spells"),
        "{out}"
    );
    let (code, out) = run(&["calendar-days", "twenty"]);
    assert_eq!(code, crate::MISUSED, "{out}");
    assert!(
        out.contains("`twenty` is neither") && out.contains("usage:"),
        "{out}"
    );
    let (code, out) = run(&["calendar-days", "2024", "2025"]);
    assert_eq!(code, crate::MISUSED, "{out}");
    assert!(
        out.contains("is a command, but not with 2 arguments"),
        "{out}"
    );
}

/// GARBAGE YEARS ARE REFUSED BY NAME, BEFORE ANY COUNT (CAL-1 self-check,
/// D-5153). `u16::from_str` took `+2024` and printed 2024's page, and took
/// `02024` and `7` as years; only four ASCII digits are a YEAR now, and every
/// other word is a misuse that names itself and prints the usage.
#[test]
fn calendar_days_refuses_garbage_years_by_name() {
    for word in [
        "",
        "+2024",
        "-2024",
        "02024",
        "7",
        "65536",
        "2024.0",
        " 2024",
        "2024 ",
        "２０２４",
    ] {
        let (code, out) = run(&["calendar-days", word]);
        assert_eq!(code, crate::MISUSED, "`{word}`: {out}");
        assert!(
            out.contains(&format!("`{word}` is neither")) && out.contains("usage:"),
            "`{word}`: {out}"
        );
        assert!(!out.contains("TOTALS"), "`{word}`: {out}");
    }
    // Four digits that spell no year of the record are refused as such, never
    // counted: the smallest and the largest.
    for word in ["0000", "9999"] {
        let (code, out) = run(&["calendar-days", word]);
        assert_eq!(code, crate::FAILED, "`{word}`: {out}");
        assert!(out.starts_with("refused: "), "`{word}`: {out}");
        assert!(!out.contains("TOTALS"), "`{word}`: {out}");
    }
    // The boundary years of the record still count.
    for word in ["2018", "2026"] {
        let (code, out) = run(&["calendar-days", word]);
        assert_eq!(code, crate::OK, "`{word}`: {out}");
        assert!(out.contains(&format!("TOTALS {word}")), "`{word}`: {out}");
    }
}

/// Every year of the record prints, exits OK and has nothing unexplained.
#[test]
fn every_year_of_the_record_counts_cleanly() {
    for year in 2018..=2026 {
        let (code, out) = run(&["calendar-days", &year.to_string()]);
        assert_eq!(code, crate::OK, "{year}: {out}");
        assert!(out.contains(&format!("TOTALS {year}")), "{year}");
        assert!(!out.contains("UNEXPLAINED"), "{year}");
    }
}

/// A day outside the record still prints as a number rather than a guess.
#[test]
fn civil_falls_back_to_the_number() {
    assert_eq!(civil(-1), "-1");
    assert_eq!(civil(19_742), "2024-01-20");
}

/// An unexplained day is marked on its line, and a year holding one is
/// refused with the count named, never passed as complete.
#[test]
fn an_unexplained_day_is_marked_and_refuses_the_year() {
    let row = line(19_742, NotTradable::Unknown);
    assert!(
        row.contains("UNEXPLAINED") && row.starts_with("2024-01-20  Sat"),
        "{row}"
    );
    let mut counted = calendar::year_exclusions(2024).expect("2024");
    let mut out = String::new();
    assert_eq!(verdict(&counted, &mut out), crate::OK);
    assert!(out.is_empty(), "{out}");
    counted.counts.by_reason[NotTradable::Unknown.index()] = 2;
    assert_eq!(verdict(&counted, &mut out), crate::FAILED);
    assert!(
        out.starts_with("refused: 2 day(s) of 2024 have no row that explains them"),
        "{out}"
    );
    assert!(page(&counted).contains("  unexplained                2"));
}

// ── `calendar-days check`: the pure findings ──

/// A synthetic holding: `feeds` named, each day held by the given mask.
fn holding(days: &[(i64, u8)]) -> Holdings {
    Holdings {
        feeds: vec![
            Feed {
                vendor: Vendor::Zerodha,
                timeframe: Timeframe::DAY_1,
            },
            Feed {
                vendor: Vendor::Gdfl,
                timeframe: Timeframe::SECOND_1,
            },
        ],
        days: days.iter().copied().collect(),
        ..Holdings::default()
    }
}

/// **EACH KIND OF DISAGREEMENT LANDS IN ITS OWN LIST.** A held Budget
/// Saturday is refused (a); a held weekend and a held holiday no row names
/// are candidates (b); a weekday the record calls open that no feed holds is
/// missing (c); a held weekend of an owed year is an owed candidate; a
/// normal session is nothing.
#[test]
fn the_findings_sort_every_disagreement_into_its_list() {
    let budget = at(2025, 2, 1);
    let saturday = at(2023, 7, 15);
    let holiday = at(2024, 1, 22);
    let owed_sunday = at(2016, 10, 30);
    let owed_weekday = at(2016, 10, 31);
    let past = at(2026, 9, 8);
    let mut days = vec![
        (budget, 1),
        (saturday, 2),
        (holiday, 3),
        (owed_sunday, 1),
        (owed_weekday, 1),
        (past, 2),
    ];
    // Every session of January 2025 but the 15th, held by both feeds.
    let mut day = at(2025, 1, 1);
    while day <= at(2025, 1, 31) {
        if calendar::tradable_day(day).is_ok() && day != at(2025, 1, 15) {
            days.push((day, 3));
        }
        day += 1;
    }
    let held = holding(&days);
    let found = findings(&held);
    assert_eq!(found.refused, [(budget, NotTradable::BudgetWeekend, 1)]);
    assert_eq!(
        found.candidates,
        [
            (saturday, "a weekend the record calls closed", 2),
            (holiday, "a trading holiday the record calls closed", 3),
        ]
    );
    assert_eq!(found.owed_candidates, [(owed_sunday, 1)]);
    assert_eq!(
        found.unmeasured.get(&2016),
        Some(&(2, owed_sunday, owed_weekday))
    );
    assert!(
        found.missing.contains(&at(2025, 1, 15)),
        "{:?}",
        found.missing
    );
    // Every weekday session between the held days is missing unless held,
    // and nothing outside the first and last held day is.
    assert!(
        found
            .missing
            .iter()
            .all(|d| (owed_sunday..=past).contains(d))
    );
    assert!(found.missing.iter().all(|d| matches!(
        calendar::sourced_kind_of(*d),
        DayKind::Open(_) | DayKind::OpenLengthUnmeasured
    )));
    assert_eq!(found.past_window, 1);
    assert_eq!(
        found.failing(),
        found.candidates.len() + found.missing.len(),
        "the Budget Saturday is a known exclusion, not a failure"
    );
    assert_eq!(holders(&held, 3), "zerodha 1day, gdfl 1s");
    assert_eq!(holders(&held, 2), "gdfl 1s");
    assert_eq!(findings(&Holdings::default()), Findings::default());
}

// ── `calendar-days check` against a scratch store ──

/// A scratch store of this test's own, removed when it drops.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-calendar-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_nanos())
        ));
        std::fs::create_dir_all(&root).expect("scratch root");
        Self(root)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn ym(year: u16, month: u8) -> YearMonth {
    YearMonth::new(year, month).expect("a month")
}

/// An invented bar at `ts`: a few rupees, which no index ever printed.
fn bar(ts: i64, n: i64) -> Bar {
    Bar {
        ts_micros: ts,
        open: 700 + n,
        high: 710 + n,
        low: 690 + n,
        close: 700 + n,
        volume: 0,
        open_interest: store::format::OI_NULL,
    }
}

/// Writes `bars` to one index month through the store's writer and records
/// its census row; returns nothing, refuses by panicking.
fn land(root: &Path, vendor: Vendor, symbol: &str, tf: Timeframe, month: YearMonth, bars: &[Bar]) {
    let key = InstrumentKey::index(Exchange::Nse, symbol).expect("an index");
    let path = StorePath::for_key(vendor, &key, tf, month, FileKind::Bars).expect("a path");
    #[expect(clippy::cast_possible_truncation, reason = "the store's own id fold")]
    let id = brutex_core::universe::fnv1a(symbol) as u32;
    let mut file = store::file::BarFile::open_or_create(root, path, id).expect("open");
    let _ = file.append(bars).expect("append");
    census_row(root, vendor, Segment::Index, symbol, None, tf, month, bars);
}

/// Records one census row; the bars it names need not exist.
#[allow(clippy::too_many_arguments, reason = "one row's every field")]
fn census_row(
    root: &Path,
    vendor: Vendor,
    segment: Segment,
    symbol: &str,
    contract: Option<Contract>,
    tf: Timeframe,
    month: YearMonth,
    bars: &[Bar],
) {
    let (first, last) = month.ist_bounds_micros();
    let row = Held::new(
        Entry {
            key: EntryKey {
                contract,
                exchange: Exchange::Nse,
                segment,
                symbol: Symbol::new(symbol).expect("a symbol"),
                timeframe: tf,
                month,
            },
            rows: u64::try_from(bars.len().max(1)).expect("small"),
            first_ts_micros: bars.first().map_or(first, |b| b.ts_micros),
            last_ts_micros: bars.last().map_or(last - 1, |b| b.ts_micros),
        },
        Closes::UNKNOWN,
    );
    assert_eq!(pull::ingest::record_held(root, vendor, &[row]), None);
}

/// Daily bars on every day of `month` that `keep` admits.
fn daily(month: YearMonth, keep: impl Fn(i64) -> bool) -> Vec<Bar> {
    let (start, end) = month.ist_bounds_micros();
    let mut out = Vec::new();
    let mut ts = start;
    let mut n = 0;
    while ts < end {
        let day = (ts + IST_OFFSET_MICROS) / DAY_MICROS;
        if keep(day) {
            out.push(bar(ts, n));
            n += 1;
        }
        ts += DAY_MICROS;
    }
    out
}

/// The record calls `day` a session.
fn open(day: i64) -> bool {
    matches!(
        calendar::sourced_kind_of(day),
        DayKind::Open(_) | DayKind::OpenLengthUnmeasured
    )
}

/// The scratch store the check tests read: Zerodha NIFTY daily bars for
/// January and February 2024 (every session but 10 and 11 January, plus an
/// INVENTED Saturday, 10 February), GDFL NIFTY seconds on 10 January only,
/// and census rows that do and do not count toward a year.
fn scratch_store(tag: &str) -> Scratch {
    let scratch = Scratch::new(tag);
    let root = &scratch.0;
    let (jan, feb) = (ym(2024, 1), ym(2024, 2));
    let (tenth, eleventh) = (at(2024, 1, 10), at(2024, 1, 11));
    let invented_saturday = at(2024, 2, 10);
    land(
        root,
        Vendor::Zerodha,
        "NIFTY",
        Timeframe::DAY_1,
        jan,
        &daily(jan, |d| open(d) && d != tenth && d != eleventh),
    );
    land(
        root,
        Vendor::Zerodha,
        "NIFTY",
        Timeframe::DAY_1,
        feb,
        &daily(feb, |d| open(d) || d == invented_saturday),
    );
    let ten_am = ist_day_start(tenth) + 10 * 3_600_000_000;
    let seconds: Vec<Bar> = (0..5).map(|n| bar(ten_am + n * 1_000_000, n)).collect();
    land(
        root,
        Vendor::Gdfl,
        "NIFTY",
        Timeframe::SECOND_1,
        jan,
        &seconds,
    );
    // Census rows alone: an F&O cash equity in 2016 and INDIA VIX in 2015
    // count toward a year; a stock outside the F&O list (2014) and an option
    // contract (2013) do not.
    census_row(
        root,
        Vendor::Zerodha,
        Segment::Cash,
        "RELIANCE",
        None,
        Timeframe::DAY_1,
        ym(2016, 3),
        &[],
    );
    census_row(
        root,
        Vendor::Zerodha,
        Segment::Index,
        "INDIAVIX",
        None,
        Timeframe::DAY_1,
        ym(2015, 7),
        &[],
    );
    census_row(
        root,
        Vendor::Zerodha,
        Segment::Cash,
        "NOTANFNOSTOCKXYZ",
        None,
        Timeframe::DAY_1,
        ym(2014, 5),
        &[],
    );
    census_row(
        root,
        Vendor::Gdfl,
        Segment::Fno,
        "NIFTY",
        Some(Contract::parse("2013-01-31-600000-CE").expect("a contract")),
        Timeframe::SECOND_1,
        ym(2013, 1),
        &[],
    );
    scratch
}

/// **THE CHECK READS EVERY FEED THROUGH THE TIME INDEX AND PRINTS THE THREE
/// LISTS**: the held 2024-01-20 is refused as the special session it is,
/// the invented Saturday is a candidate, 11 January is missing (10 January
/// is held by GDFL alone); the years are 2015 and 2016 (owed) and 2024
/// (measured), never 2013 or 2014; and the run fails, naming two findings.
#[test]
fn the_check_reads_every_feed_and_prints_the_three_lists() {
    let scratch = scratch_store("lists");
    let held = read_holdings(&scratch.0);
    assert!(held.refusals.is_empty(), "{:?}", held.refusals);
    assert_eq!(
        held.feeds,
        [
            Feed {
                vendor: Vendor::Gdfl,
                timeframe: Timeframe::SECOND_1
            },
            Feed {
                vendor: Vendor::Zerodha,
                timeframe: Timeframe::DAY_1
            },
        ]
    );
    assert_eq!(held.months_read, [1, 2]);
    assert_eq!(
        held.years.keys().copied().collect::<Vec<_>>(),
        [2015, 2016, 2024]
    );
    assert_eq!(held.days.get(at(2024, 1, 10)), Some(1), "GDFL alone");
    assert_eq!(held.days.get(at(2024, 1, 11)), None);
    assert_eq!(held.days.get(at(2024, 1, 12)), Some(2), "Zerodha alone");
    let sessions_held = (at(2024, 1, 1)..=at(2024, 2, 29))
        .filter(|d| open(*d))
        .count()
        - 1;
    assert_eq!(held.days.len(), sessions_held + 1, "plus the Saturday");

    // January's 22 sessions (21 weekdays and the 20th) less the 10th and the
    // 11th, and February's 21 weekday sessions and the invented Saturday.
    let zerodha_days = held.days.values().filter(|mask| **mask & 2 == 2).count();
    assert_eq!(zerodha_days, 20 + 22);

    let mut out = String::new();
    let code = check_at(&scratch.0, &mut out);
    assert_eq!(code, crate::FAILED, "{out}");
    for needle in [
        "2015  OWED: no NSE holiday circular recorded",
        "2016  OWED: no NSE holiday circular recorded",
        // The columns stay apart: the owed text ran into its notices before.
        "are Unmeasured   0 (0 untimed)    ",
        "2024  measured from the store",
        "OWED: the holiday and special-session circulars of 2015, 2016 are not recorded",
        "  gdfl 1s              1 months read,     1 days held",
        "  zerodha 1day         2 months read,    42 days held",
        "2024-01-20  Sat  WeekendFullSession",
        "2024-02-10  Sat  a weekend the record calls closed; held by zerodha 1day; UNVERIFIED",
        "2024-01-11  Thu  called open; no feed holds a NIFTY or BANKNIFTY bar",
        "  (b) candidates in the record      1",
        "  (c) missing sessions              1",
        "refused: 0 source(s) could not be read and 2 finding(s) inside the record",
    ] {
        assert!(out.contains(needle), "{needle:?} missing from:\n{out}");
    }
    assert!(!out.contains("2013  "), "{out}");
    assert!(!out.contains("2014  "), "{out}");
    assert!(!out.contains("2024-01-10  Wed  called open"), "{out}");
}

/// **A MONTH THE CENSUS NAMES AND THE STORE CANNOT OPEN IS NAMED, NEVER A
/// SILENT GAP**, and the run fails; a store whose days and calendar agree
/// passes.
#[test]
fn a_month_that_cannot_be_read_is_named_and_a_clean_store_passes() {
    let scratch = Scratch::new("clean");
    let root = &scratch.0;
    let march = ym(2024, 3);
    land(
        root,
        Vendor::Zerodha,
        "NIFTY",
        Timeframe::DAY_1,
        march,
        &daily(march, |d| calendar::tradable_day(d).is_ok() || open(d)),
    );
    let mut out = String::new();
    assert_eq!(check_at(root, &mut out), crate::OK, "{out}");
    assert!(
        out.contains("  (c) missing sessions              0"),
        "{out}"
    );
    // 2024-03-02, the drill, is held and refused by name.
    assert!(
        out.contains("2024-03-02  Sat  DisasterRecoveryDrill"),
        "{out}"
    );

    census_row(
        root,
        Vendor::Zerodha,
        Segment::Index,
        "BANKNIFTY",
        None,
        Timeframe::DAY_1,
        ym(2024, 4),
        &[],
    );
    let mut out = String::new();
    assert_eq!(check_at(root, &mut out), crate::FAILED, "{out}");
    assert!(
        out.contains("  NOT READ: zerodha BANKNIFTY 1day 2024-04:"),
        "{out}"
    );
    assert!(
        out.contains("refused: 1 source(s) could not be read"),
        "{out}"
    );
}

/// **ONE DAY IS TWO TIME-INDEX LOOKUPS, MEASURED.** `month_days` over the
/// scratch store's daily month, n = 240 calls of 31 days each; and the pure
/// findings over a whole record's worth of held days. p50, p99 and max are
/// printed per day (`--nocapture`); the bound is generous for a loaded host.
#[test]
fn the_check_costs_constant_time_per_day_measured() {
    use std::hint::black_box;
    let scratch = scratch_store("bench");
    let mut per_day = Vec::new();
    for _ in 0..240 {
        let started = Instant::now();
        let days = black_box(
            month_days(
                &scratch.0,
                Vendor::Zerodha,
                "NIFTY",
                Timeframe::DAY_1,
                ym(2024, 1),
            )
            .expect("read"),
        );
        assert_eq!(days.len(), 20);
        per_day.push(started.elapsed().as_nanos() / 31);
    }
    per_day.sort_unstable();
    let pick = |permille: usize| per_day[per_day.len() * permille / 1_000];
    let (p50, p99, max) = (pick(500), pick(990), per_day[per_day.len() - 1]);
    eprintln!(
        "month_days (open + two lookups a day), per day: p50 {p50} ns, p99 {p99} ns, \
         max {max} ns, n = {} calls of 31 days",
        per_day.len()
    );
    assert!(p99 < 2_000_000, "p99 {p99} ns a day");

    let mut days = Vec::new();
    let mut day = calendar::SOURCED_FIRST_DAY;
    while day <= calendar::SOURCED_LAST_DAY {
        if open(day) {
            days.push((day, 1_u8));
        }
        day += 1;
    }
    let held = holding(&days);
    let mut per_held = Vec::new();
    for _ in 0..200 {
        let started = Instant::now();
        black_box(findings(black_box(&held)));
        per_held.push(started.elapsed().as_nanos() / u128::try_from(days.len()).expect("small"));
    }
    per_held.sort_unstable();
    let pick = |permille: usize| per_held[per_held.len() * permille / 1_000];
    let (p50, p99, max) = (pick(500), pick(990), per_held[per_held.len() - 1]);
    eprintln!(
        "findings, per held day: p50 {p50} ns, p99 {p99} ns, max {max} ns, n = {} runs of {} \
         days",
        per_held.len(),
        days.len()
    );
    assert!(p99 < 100_000, "p99 {p99} ns a day");
}

// ── The dense day map and the empty check (CAL-1 self-check, 19 Oct 2026) ──

/// **THE DAY MAP IS DENSE AND REFUSES A DAY OUTSIDE ITS SPAN.** Two feeds
/// marking one day OR their bits and count the day once; an unheld day and a
/// day past either edge read as nothing, and marking one past an edge is
/// refused (`false`) without changing the map; the days come out in order;
/// the first and the last held day are tracked as days land, so an unheld
/// edge of the span is never reported as held; an empty or inverted span
/// holds nothing.
#[test]
fn the_day_map_is_dense_ordered_and_refuses_a_day_outside_its_span() {
    let (first, last) = (at(2024, 1, 1), at(2024, 1, 31));
    let mut map = DayMasks::over(first, last);
    assert!(map.is_empty());
    assert_eq!((map.first_day(), map.last_day()), (None, None));
    assert!(map.mark(at(2024, 1, 12), 2));
    assert!(map.mark(at(2024, 1, 3), 1));
    assert!(map.mark(at(2024, 1, 12), 1), "a second feed ORs its bit");
    assert!(map.mark(at(2024, 1, 20), 0), "a zero mask holds nothing");
    assert_eq!(map.len(), 2, "one day held by two feeds is one day");
    assert_eq!(map.get(at(2024, 1, 12)), Some(3));
    assert_eq!(map.get(at(2024, 1, 3)), Some(1));
    assert_eq!(map.get(at(2024, 1, 20)), None);
    assert_eq!(map.get(first), None, "the span's own edge is not held");
    assert_eq!(map.get(first - 1), None);
    assert_eq!(map.get(last + 1), None);
    assert!(!map.mark(first - 1, 1), "before the span: refused");
    assert!(!map.mark(last + 1, 1), "after the span: refused");
    assert!(!map.mark(i64::MIN, 1) && !map.mark(i64::MAX, 1));
    assert_eq!(map.len(), 2, "a refused day changes nothing");
    assert_eq!(
        map.iter().collect::<Vec<_>>(),
        [(at(2024, 1, 3), 1), (at(2024, 1, 12), 3)]
    );
    assert_eq!(map.values().copied().collect::<Vec<_>>(), [1, 3]);
    assert_eq!(
        (map.first_day(), map.last_day()),
        (Some(at(2024, 1, 3)), Some(at(2024, 1, 12)))
    );
    assert!(map.contains(at(2024, 1, 3)) && !map.contains(at(2024, 1, 4)));

    let inverted = DayMasks::over(last, first);
    assert!(inverted.is_empty());
    assert_eq!(inverted.iter().count(), 0);
    let mut inverted = inverted;
    assert!(!inverted.mark(first, 1), "an inverted span holds no slot");

    let collected: DayMasks = [
        (at(2024, 2, 9), 1),
        (at(2024, 2, 5), 2),
        (at(2024, 2, 9), 2),
    ]
    .into_iter()
    .collect();
    assert_eq!(
        collected.iter().collect::<Vec<_>>(),
        [(at(2024, 2, 5), 2), (at(2024, 2, 9), 3)],
        "out-of-order and duplicate days land once, in order"
    );
    let none: DayMasks = std::iter::empty().collect();
    assert!(none.is_empty() && none.iter().next().is_none());
}

/// **A CHECK OF NOTHING IS NOT A PASS.** A store whose census names only a
/// cash equity's month (no NIFTY or BANKNIFTY bar month) holds no day the
/// check can read, and the run fails saying so rather than printing zero
/// findings as agreement; its year is still listed. A store with no census
/// at all fails too.
#[test]
fn a_store_holding_no_index_day_fails_the_check_by_name() {
    let scratch = Scratch::new("no-index");
    census_row(
        &scratch.0,
        Vendor::Zerodha,
        Segment::Cash,
        "RELIANCE",
        None,
        Timeframe::DAY_1,
        ym(2024, 1),
        &[],
    );
    let held = read_holdings(&scratch.0);
    assert!(held.days.is_empty(), "{:?}", held.days);
    assert_eq!(held.years.keys().copied().collect::<Vec<_>>(), [2024]);
    let mut out = String::new();
    assert_eq!(check_at(&scratch.0, &mut out), crate::FAILED, "{out}");
    assert!(out.contains("a check of nothing is not a pass"), "{out}");
    assert!(
        out.contains("  days held by any feed             0"),
        "{out}"
    );

    let void = Scratch::new("void");
    let mut out = String::new();
    assert_eq!(check_at(&void.0, &mut out), crate::FAILED, "{out}");
}

/// **A DAY COSTS THE SAME WHATEVER THE SPAN, MEASURED.** Marking and reading
/// back every day of a 31-day span and of a 36,525-day span (a century): the
/// per-day p50, p99 and max over n = 200 runs each are printed
/// (`--nocapture`). An ordered tree grows a level per doubling; the dense map
/// is one indexed access a day at both sizes. The bound is generous for a
/// loaded host.
#[test]
fn a_day_costs_the_same_at_any_span_measured() {
    use std::hint::black_box;
    let mut summary = Vec::new();
    for width in [31_i64, 36_525] {
        let first = at(2000, 1, 1);
        let last = first + width - 1;
        let mut per_day = Vec::new();
        for _ in 0..200 {
            let started = Instant::now();
            let mut map = DayMasks::over(first, last);
            let mut day = first;
            while day <= last {
                assert!(map.mark(black_box(day), 1));
                day += 1;
            }
            let held = black_box(&map).iter().count();
            assert_eq!(held, usize::try_from(width).expect("small"));
            per_day.push(started.elapsed().as_nanos() / u128::try_from(width).expect("small"));
        }
        per_day.sort_unstable();
        let pick = |permille: usize| per_day[per_day.len() * permille / 1_000];
        let (p50, p99, max) = (pick(500), pick(990), per_day[per_day.len() - 1]);
        eprintln!(
            "day map, mark + walk, span {width} days, per day: p50 {p50} ns, p99 {p99} ns, \
             max {max} ns, n = {} runs",
            per_day.len()
        );
        assert!(p99 < 100_000, "span {width}: p99 {p99} ns a day");
        summary.push(p50);
    }
    assert_eq!(summary.len(), 2);
}
