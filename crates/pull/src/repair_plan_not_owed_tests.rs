//! The repair plan asks `pull::gaps::not_owed` of every session (R4-19;
//! findings G4, Z3, COR-07). FORCEMOT, INE451A01017 and the interval
//! 2023-10-26..=2024-02-13 are the documented facts (`docs/00-charter.md`,
//! "FORCEMOT NSE interruption"); every price, every master row and every
//! listing day here is invented, and no bar is a vendor's.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::too_many_lines,
    reason = "test-only assertions"
)]

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use brutex_core::vendor::Vendor;
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{
    Missing, Month, NotOwedSession, key_of, listing_for, plan, plan_month, plan_month_for,
};
use crate::calendar::{DayKind, exchange_kind_of};
use crate::gaps::{Key, NotOwed};
use crate::session::{Day, IST_OFFSET_SECS, Window};

/// A scratch store root of this test's own, removed when it ends.
struct Root(PathBuf);

impl Root {
    fn new(tag: &str) -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "brutex-plan-owed-{tag}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }
}

impl Drop for Root {
    fn drop(&mut self) {
        let _best_effort = std::fs::remove_dir_all(&self.0);
    }
}

fn day(year: u16, month: u8, date: u8) -> Day {
    Day::new(year, month, date).unwrap()
}

fn cash(symbol: &'static str, month: YearMonth) -> PathParts<'static> {
    PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol,
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month,
        file: FileKind::Bars,
    }
}

/// An invented minute bar at `minute` past IST midnight on `on`.
fn bar(on: Day, minute: i64) -> Bar {
    let secs = i64::from(on.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + minute * 60;
    Bar {
        ts_micros: secs * 1_000_000,
        open: 77_700,
        high: 77_710,
        low: 77_690,
        close: 77_705,
        volume: 1,
        open_interest: OI_NULL,
    }
}

/// Every minute each session of `days` owes a share ([`super::owed`], a
/// special session's own windows included), as `symbol`'s 1min months.
fn land(root: &Path, symbol: &'static str, days: impl Iterator<Item = Day>) {
    let mut by_month: Vec<(YearMonth, Vec<Bar>)> = Vec::new();
    for on in days {
        let month = on.year_month().unwrap();
        let super::Owes::Windows((windows, count)) = super::owed("CASH", on.days_from_epoch())
        else {
            panic!("{on}: a landed session states its windows");
        };
        let bars: Vec<Bar> = windows[..count]
            .iter()
            .flat_map(|&(from, to)| (from..to).map(move |minute| bar(on, i64::from(minute))))
            .collect();
        match by_month.last_mut() {
            Some((held, rows)) if *held == month => rows.extend(bars),
            _ => by_month.push((month, bars)),
        }
    }
    for (month, rows) in by_month {
        let path = StorePath::new(cash(symbol, month)).unwrap();
        let mut file =
            store::file::BarFile::open_or_create(root, path, super::symbol_id(symbol)).unwrap();
        file.append(&rows).unwrap();
    }
}

/// The sized sessions of record of `from..=to`, oldest first.
fn sessions(from: Day, to: Day) -> impl Iterator<Item = Day> {
    (from.days_from_epoch()..=to.days_from_epoch())
        .filter(|number| matches!(exchange_kind_of(i64::from(*number)), DayKind::Open(_)))
        .map(|number| Day::from_days(number).unwrap())
}

/// **G4 and Z3 in the plan: an inactive session is not a repair month.** The
/// operator's FORCEMOT store, invented: every session it owed from
/// 2023-09-01 to 2024-03-31 held whole, and nothing in NSE's withdrawal
/// 2023-10-26..=2024-02-13, three months of it without a file at all. Before
/// R4-19 the nine sessions of 2024-02 before its first held bar were
/// `front`, a versioned repair (asked of the same month with `Key::NONE`,
/// below), and a repair asks the feed for bars NSE never traded. Now none of
/// the 76 sessions is owed, each is named with both NSE sources, and no month
/// needs a repair.
#[test]
fn an_inactive_session_is_not_a_repair_month() {
    let root = Root::new("forcemot");
    let (from, through) = (day(2023, 10, 26), day(2024, 2, 13));
    let window = Window::new(day(2023, 9, 1), day(2024, 3, 31)).unwrap();
    land(
        &root.0,
        "FORCEMOT",
        sessions(window.from(), window.to()).filter(|on| *on < from || *on > through),
    );
    let months = plan(
        &root.0,
        cash("FORCEMOT", YearMonth::new(2023, 9).unwrap()),
        window,
    );
    let names: Vec<String> = months.iter().map(|m| m.month.to_string()).collect();
    assert_eq!(
        names,
        [
            "2023-09", "2023-10", "2023-11", "2023-12", "2024-01", "2024-02", "2024-03"
        ]
    );
    for month in &months {
        assert_eq!(month.refused, None, "{}", month.month);
        assert!(!month.needs_repair(), "{} needs no repair", month.month);
        assert!(
            month.missing.is_empty(),
            "{}: {:?}",
            month.month,
            month.missing
        );
        assert!(month.short.is_empty(), "{}: {:?}", month.month, month.short);
        assert_eq!(
            month.held, month.sessions,
            "{}: every owed session held",
            month.month
        );
    }
    let excused: Vec<&NotOwedSession> = months.iter().flat_map(|m| &m.not_owed).collect();
    assert_eq!(excused.len(), 76, "the withdrawal's sessions of record");
    assert_eq!(excused.first().map(|s| s.day), Some(from));
    assert_eq!(excused.last().map(|s| s.day), Some(through));
    for session in &excused {
        let NotOwed::InstrumentInactive(interval) = session.why else {
            panic!("{}: {:?}", session.day, session.why);
        };
        assert!(interval.withdrawal_source.ends_with("/CML58560.pdf"));
        assert_eq!(session.bars, 0);
    }
    let evidence = excused[0].why.evidence();
    assert!(evidence.contains("CML58560"), "{evidence}");
    assert!(evidence.contains("2023-10-26..=2024-02-13"), "{evidence}");
    // NOVEMBER, DECEMBER AND JANUARY HOLD NO FILE AND OWE NOTHING.
    for month in &months[2..=4] {
        assert_eq!((month.bars, month.sessions), (0, 0), "{}", month.month);
        assert!(!month.not_owed.is_empty());
    }
    // OCTOBER AND FEBRUARY ARE SPLIT, AND SAID AS ONE RUN EACH.
    let october = months[1].not_owed_runs();
    assert_eq!(october.len(), 1);
    assert_eq!((october[0].from, october[0].to), (from, day(2023, 10, 31)));
    assert_eq!(october[0].sessions, 4, "the 26th, 27th, 30th and 31st");
    let february = months[5].not_owed_runs();
    assert_eq!(
        (february[0].from, february[0].to),
        (day(2024, 2, 1), through)
    );
    assert_eq!(february[0].sessions, 9);
    // BEFORE R4-19: the same February, asked with a key naming no share.
    let feb = cash("FORCEMOT", YearMonth::new(2024, 2).unwrap());
    let id = super::symbol_id("FORCEMOT");
    let unkeyed = plan_month_for(
        &root.0,
        feb,
        id,
        (day(2024, 2, 1), day(2024, 2, 29), false),
        &Key::NONE,
    );
    assert!(unkeyed.needs_repair(), "the old answer: a versioned repair");
    assert_eq!(
        unkeyed
            .missing
            .iter()
            .filter(|(_, why)| *why == Missing::Front)
            .count(),
        9
    );
    // THE STORED-MONTH GAP LEDGER'S DOOR builds the same key itself.
    let ledger = plan_month(&root.0, feb, id, (day(2024, 2, 1), day(2024, 2, 29), false));
    assert!(ledger.missing.is_empty());
    assert_eq!(ledger.not_owed.len(), 9);
}

/// Seconds from 1980-01-01 to `on`, as a master states a lifecycle date.
fn nse_seconds(on: Day) -> u64 {
    u64::from(on.days_from_epoch() - day(1980, 1, 1).days_from_epoch()) * 86_400
}

/// Install a receipted master dated 2026-08-03 under `root` stating `rows`
/// (symbol, ISIN, listing day, removal day).
fn install_master(root: &Path, rows: &[(&str, &str, Day, Option<Day>)]) {
    let mut csv = String::from(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n",
    );
    for (id, (symbol, isin, listed, removed)) in rows.iter().enumerate() {
        writeln!(
            csv,
            "{},{symbol},EQ,{isin},1,{},{},0",
            9_100 + id,
            nse_seconds(*listed),
            removed.map_or(0, nse_seconds)
        )
        .unwrap();
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(csv.as_bytes()).unwrap();
    let bytes = encoder.finish().unwrap();
    crate::cash_session_cache::install(
        &root.join(crate::listing::MASTER_DIR),
        day(2026, 8, 3),
        &bytes,
    )
    .unwrap();
}

/// **COR-07 in the plan: a session before the listing day is not a repair
/// month, and a bar on it is named, never planned** (a combination: foreign
/// bars in a month before the listing, the IRFC case). The share's store
/// holds one invented session of May 2023 and June from the 14th; the master
/// (invented row, the universe's ISIN) lists it on the 14th. Without the
/// listing June's first nine sessions were `front`, a repair; with it they
/// are not owed, May's held session is not owed with its 375 bars named, and
/// nothing needs a repair.
#[test]
fn a_session_before_listing_is_not_a_repair_month() {
    let root = Root::new("listing");
    let isin = brutex_core::universe::nse_isin("IRFC").unwrap();
    let listed = day(2023, 6, 14);
    install_master(&root.0, &[("IRFC", isin.as_str(), listed, None)]);
    let june = sessions(day(2023, 6, 1), day(2023, 6, 30));
    land(
        &root.0,
        "IRFC",
        std::iter::once(day(2023, 5, 2)).chain(june.filter(|on| *on >= listed)),
    );
    let window = Window::new(day(2023, 5, 1), day(2023, 6, 30)).unwrap();
    let months = plan(
        &root.0,
        cash("IRFC", YearMonth::new(2023, 5).unwrap()),
        window,
    );
    assert!(
        months
            .iter()
            .all(|m| !m.needs_repair() && m.refused.is_none())
    );
    let may = &months[0];
    assert_eq!(may.sessions, 0, "no session of May was owed");
    assert!(may.missing.is_empty());
    let foreign: Vec<&NotOwedSession> = may.not_owed.iter().filter(|s| s.bars > 0).collect();
    assert_eq!(foreign.len(), 1);
    assert_eq!((foreign[0].day, foreign[0].bars), (day(2023, 5, 2), 375));
    assert_eq!(foreign[0].why, NotOwed::BeforeListing { listed_on: listed });
    let june = &months[1];
    assert_eq!(june.not_owed.len(), 9, "1st to 13th June");
    assert_eq!(june.held, june.sessions);
    assert!(june.missing.is_empty());
    // WITHOUT THE MASTER'S WORD the same June is a repair.
    let unlisted = plan_month_for(
        &root.0,
        cash("IRFC", YearMonth::new(2023, 6).unwrap()),
        super::symbol_id("IRFC"),
        (day(2023, 6, 1), day(2023, 6, 30), false),
        &Key::series("NSE", "CASH", "IRFC", None),
    );
    assert!(unlisted.needs_repair());
    // A REMOVAL DAY THE MASTER STATES: owed through it, not after.
    let gone = Root::new("removal");
    install_master(
        &gone.0,
        &[(
            "IRFC",
            isin.as_str(),
            day(2010, 1, 4),
            Some(day(2023, 6, 16)),
        )],
    );
    land(&gone.0, "IRFC", sessions(day(2023, 6, 1), day(2023, 6, 16)));
    let after = &plan(
        &gone.0,
        cash("IRFC", YearMonth::new(2023, 6).unwrap()),
        Window::new(day(2023, 6, 1), day(2023, 6, 30)).unwrap(),
    )[0];
    assert!(after.missing.is_empty(), "{:?}", after.missing);
    assert_eq!(
        after.not_owed.len(),
        9,
        "19th to 30th June, Bakri Id excepted"
    );
    assert!(after.not_owed.iter().all(|s| s.why
        == NotOwed::AfterDelisting {
            delisted_on: day(2023, 6, 16)
        }));
}

/// **The listing is asked only for an NSE cash share's own series**, and a
/// contract names no share.
#[test]
fn only_an_nse_cash_share_asks_the_listing_and_a_contract_is_excused_nothing() {
    let root = Root::new("doors");
    let month = YearMonth::new(2023, 12).unwrap();
    let index = PathParts {
        segment: "INDEX",
        symbol: "NIFTY",
        ..cash("FORCEMOT", month)
    };
    assert!(listing_for(&root.0, &index).is_none());
    assert!(listing_for(&root.0, &cash("FORCEMOT", month)).is_some());
    let contract = PathParts {
        contract: Some(brutex_core::instrument::Contract::parse("2025-09-30-FUT").unwrap()),
        ..cash("FORCEMOT", month)
    };
    assert!(listing_for(&root.0, &contract).is_none());
    assert_eq!(key_of(&contract, None), Key::NONE);
    assert_eq!(
        key_of(&cash("FORCEMOT", month), None),
        Key::series("NSE", "CASH", "FORCEMOT", None)
    );
    // AN INDEX OF THE SAME WINDOW IS OWED EVERY SESSION, as before.
    let window = Window::new(day(2023, 12, 1), day(2023, 12, 31)).unwrap();
    let nifty = &plan(&root.0, index, window)[0];
    assert!(nifty.not_owed.is_empty());
    assert!(nifty.sessions > 0);
    assert!(nifty.missing.iter().all(|(_, why)| *why == Missing::Absent));
}

/// **Consecutive sessions excused alike are one run, whatever lies between;
/// a different reason starts another** (`Month::not_owed_runs`).
#[test]
fn sessions_not_owed_alike_are_one_run_and_a_new_reason_starts_another() {
    let listed = NotOwed::BeforeListing {
        listed_on: day(2023, 6, 14),
    };
    let session = |on: Day, why: NotOwed, bars: u64| NotOwedSession { day: on, why, bars };
    let month = Month {
        month: YearMonth::new(2023, 6).unwrap(),
        generation: 0,
        bars: 0,
        span: None,
        sessions: 0,
        held: 0,
        missing: Vec::new(),
        short: Vec::new(),
        length_unjudged: Vec::new(),
        unattested: Vec::new(),
        not_owed: vec![
            session(day(2023, 6, 9), listed, 0),
            session(day(2023, 6, 12), listed, 5),
            session(day(2023, 6, 13), NotOwed::Unread, 0),
            session(day(2023, 6, 14), listed, 0),
        ],
        refused: None,
    };
    let runs = month.not_owed_runs();
    assert_eq!(runs.len(), 3);
    assert_eq!(
        (runs[0].from, runs[0].to, runs[0].sessions, runs[0].bars),
        (day(2023, 6, 9), day(2023, 6, 12), 2, 5)
    );
    assert_eq!(runs[1].why, NotOwed::Unread);
    assert_eq!((runs[2].from, runs[2].sessions), (day(2023, 6, 14), 1));
    assert!(
        Month {
            not_owed: Vec::new(),
            ..month
        }
        .not_owed_runs()
        .is_empty()
    );
}
