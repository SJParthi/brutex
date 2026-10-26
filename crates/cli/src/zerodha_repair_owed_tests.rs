//! `cli zerodha-repair` names the sessions a series did not owe and plans no
//! repair for them (R4-19; findings G4, Z3). FORCEMOT, INE451A01017 and the
//! interval 2023-10-26..=2024-02-13 are the documented facts
//! (`docs/00-charter.md`, "FORCEMOT NSE interruption"); every price is
//! invented and no bar is a vendor's.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::too_many_lines,
    reason = "test-only assertions"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use pull::ingest::repair_plan::{self, Owes};
use pull::session::{Day, IST_OFFSET_SECS};
use store::format::{Bar, OI_NULL};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::command_with;
use crate::OK;

/// A store root of this test's own, removed when it ends.
struct Root(PathBuf);

impl Root {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "brutex-cli-zrepair-owed-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
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

/// Every minute each sized session of `from..=to` outside the withdrawal owes
/// a share, as FORCEMOT's invented 1min months.
fn land_owed(root: &std::path::Path, from: Day, to: Day, skip: (Day, Day)) {
    let mut months: Vec<(YearMonth, Vec<Bar>)> = Vec::new();
    for number in from.days_from_epoch()..=to.days_from_epoch() {
        let on = Day::from_days(number).unwrap();
        if on >= skip.0 && on <= skip.1 {
            continue;
        }
        let Owes::Windows((windows, count)) = repair_plan::owed("CASH", number) else {
            continue;
        };
        let base = i64::from(number) * 86_400 - IST_OFFSET_SECS;
        let bars = windows[..count].iter().flat_map(|&(open, close)| {
            (open..close).map(move |minute| Bar {
                ts_micros: (base + i64::from(minute) * 60) * 1_000_000,
                open: 66_600,
                high: 66_610,
                low: 66_590,
                close: 66_605,
                volume: 1,
                open_interest: OI_NULL,
            })
        });
        let month = on.year_month().unwrap();
        match months.last_mut() {
            Some((held, rows)) if *held == month => rows.extend(bars),
            _ => months.push((month, bars.collect())),
        }
    }
    for (month, rows) in months {
        let path = StorePath::new(PathParts {
            vendor: brutex_core::vendor::Vendor::Zerodha,
            exchange: "NSE",
            segment: "CASH",
            symbol: "FORCEMOT",
            contract: None,
            timeframe: Timeframe::MINUTE_1,
            month,
            file: FileKind::Bars,
        })
        .unwrap();
        let mut file =
            store::file::BarFile::open_or_create(root, path, repair_plan::symbol_id("FORCEMOT"))
                .unwrap();
        file.append(&rows).unwrap();
    }
}

/// **The read-only FORCEMOT plan of the operator's window names no repair
/// month, and says which sessions it did not owe and on whose word.** Before
/// R4-19 the same plan named 2024-02 a versioned repair (its nine sessions
/// before the 14th were `front`) and printed November to January as
/// sessions an ordinary pull would fill; NSE had withdrawn the share for all
/// of them.
#[test]
fn the_forcemot_plan_names_no_repair_month_and_says_what_was_not_owed() {
    let root = Root::new();
    let withdrawn = (day(2023, 10, 26), day(2024, 2, 13));
    land_owed(&root.0, day(2023, 9, 1), day(2024, 3, 31), withdrawn);
    let mut out = String::new();
    let store = root.0.clone();
    let code = command_with(
        &["FORCEMOT", "2023-09-01", "2024-03-31", "--segment", "CASH"],
        &move || Ok(store.clone()),
        &mut out,
    );
    assert_eq!(code, OK, "{out}");
    assert!(
        out.contains("  0 month(s) need a versioned repair"),
        "{out}"
    );
    assert!(!out.contains("POST /pull/repair"), "{out}");
    assert!(!out.contains("front"), "{out}");
    let lines: Vec<&str> = out
        .lines()
        .filter(|line| line.contains("not owed"))
        .collect();
    assert_eq!(
        lines.len(),
        5,
        "one run a month, October to February: {out}"
    );
    assert!(
        lines[0].contains("2023-10-26..2023-10-31  not owed: 4 session(s), 0 bar(s) held"),
        "{out}"
    );
    assert!(
        lines[4].contains("2024-02-01..2024-02-13  not owed: 9 session(s)"),
        "{out}"
    );
    for line in &lines {
        assert!(line.contains("instrument_inactive"), "{line}");
        assert!(line.contains("CML58560"), "{line}");
        assert!(line.contains("readmission circular CML60618"), "{line}");
        assert!(line.contains("forcemotors.com"), "{line}");
    }
    let sessions: u32 = lines
        .iter()
        .map(|line| {
            let after = line.split("not owed: ").nth(1).unwrap();
            after.split(' ').next().unwrap().parse::<u32>().unwrap()
        })
        .sum();
    assert_eq!(sessions, 76, "the withdrawal's sessions of record");
    assert!(
        out.contains("2023-12  generation 0       0 bars   0 of  0 sessions held"),
        "December owes nothing: {out}"
    );
}
