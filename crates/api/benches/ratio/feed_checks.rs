//! Gate 8 for the data checks on `/ingest` (DATA-1w, D-5190 to D-5194):
//! `/feed-checks.json`, `/tick-checks.json` and `/calendar-days.json` answer
//! a page in time that does not grow with how much the checks hold.
//!
//! Each served store is given 2, 20 and 200 feed-reconcile months (1x, 10x,
//! 100x) of 10, 100 and 1,000 invented shares each, and one tick-to-second
//! year of 10, 100 and 1,000 invented shares, written in the writers' own
//! formats (`cli::feed_reconcile::FORMAT` and `COLUMNS`,
//! `cli::tick_second_check::HEADER` and `index::encode`). Every target asks
//! one fixed page, so a route that scanned the folder, a month or a year per
//! request would grow 100x and breach. The first ask of each version is the
//! `cold` figure (the one-pass load, `docs/06-limits.md` L-D1W-02); what is
//! gated is every later ask. Every share is `ZZB<n>` and every price ends in
//! 7777: nothing here can be mistaken for a vendor's row.

use std::fmt::Write as _;

use super::refuse;
use super::served::{Served, flat, ok_with};

/// The routes this module times, read by `.github/gates_routes.rs`.
pub(crate) const ROUTES: &[&str] = &[
    "/feed-checks.json",
    "/tick-checks.json",
    "/calendar-days.json",
];

/// Months per 1,000 census series: 2, 20 and 200 at the three sizes.
const MONTHS_PER_THOUSAND: usize = 2;

/// Shares per 1,000 census series: 10, 100 and 1,000.
const SHARES_PER_THOUSAND: usize = 10;

/// The tick-to-second year every store holds.
const YEAR: u16 = 2024;

/// One row in `header`'s column order: every named cell, the rest empty.
fn cells(header: &str, named: &[(&str, &str)]) -> String {
    header
        .split(',')
        .map(|column| {
            named
                .iter()
                .find(|(name, _)| *name == column)
                .map_or("", |(_, value)| *value)
        })
        .collect::<Vec<_>>()
        .join(",")
}

/// The `at`-th month back from 2018-12: 2018-12, 2018-11, ...
fn month_name(at: usize) -> String {
    let from_end = i64::try_from(at).unwrap_or(0);
    let total = 2018 * 12 + 11 - from_end;
    format!("{}-{:02}", total / 12, total % 12 + 1)
}

/// One whole format-3 feed-reconcile month of `shares` shares, one day each.
fn feed_month(month: &str, shares: usize) -> String {
    let mut text = format!(
        "{}\n# month {month}; store bench; commit none\n{}\n",
        cli::feed_reconcile::FORMAT,
        cli::feed_reconcile::COLUMNS
    );
    let day = format!("{month}-03");
    for share in 0..shares {
        let symbol = format!("ZZB{share:05}");
        text.push_str(&cells(
            cli::feed_reconcile::COLUMNS,
            &[
                ("symbol", &symbol),
                ("day", &day),
                ("day_kind", "open_375"),
                ("status", "matched"),
                ("applied_lag", "0"),
                ("lag_source", "own"),
                ("median_z", "127777"),
                ("median_g", "137777"),
                ("short_volume", "7"),
                ("short_judgement", "unjudged:no_env1_envelope"),
                ("ex_date", "no:close"),
                ("flags", "adjusted"),
                ("source", "bench"),
            ],
        ));
        text.push('\n');
    }
    for share in 0..shares {
        let _ = writeln!(text, "# carry,ZZB{share:05},lags=-");
    }
    let _ = writeln!(
        text,
        "# summary: feed-reconcile {month}: {shares} shares (bench); {shares} stock-days"
    );
    text
}

/// One tick-to-second year of `shares` shares, one day each, and its index.
fn tick_year(shares: usize) -> (String, Vec<u8>) {
    let mut csv = format!("{}\n", cli::tick_second_check::HEADER);
    let Ok(day) = pull::session::Day::new(YEAR, 2, 29) else {
        refuse("2024-02-29 is a day")
    };
    let names: Vec<String> = (0..shares).map(|share| format!("ZZB{share:05}")).collect();
    for name in &names {
        csv.push_str(&cells(
            cli::tick_second_check::HEADER,
            &[
                ("symbol", name),
                ("day", "2024-02-29"),
                ("role", "landing"),
                ("source", "archive_member"),
                ("status", "exact_T"),
                ("compare", "exact_T"),
                ("session", "regular"),
                ("zerodha_basis", "raw_unadjusted"),
                ("a_compared", "375"),
                ("a_high_match", "300"),
                ("a_low_match", "300"),
            ],
        ));
        csv.push('\n');
    }
    let cells: Vec<_> = names
        .iter()
        .enumerate()
        .map(|(at, name)| {
            (
                name.as_str(),
                day,
                cli::tick_second_check::index::SecondStatus {
                    status: cli::tick_second_check::index::Status::ExactT,
                    source: cli::tick_second_check::index::Source::ArchiveMember,
                    store_seconds: 100,
                    fill_compared: 100,
                    fill_worse: 7,
                    row: u32::try_from(at + 1).unwrap_or(u32::MAX),
                },
            )
        })
        .collect();
    let digest = brutex_core::blake3::hash(csv.as_bytes());
    match cli::tick_second_check::index::encode(YEAR, &cells, 1, digest) {
        Ok(image) => (csv, image),
        Err(why) => refuse(&format!("the bench index would not encode: {why}")),
    }
}

/// Writes the checks into `served`'s audit area.
fn checks_into(served: &Served) -> (String, String) {
    let thousands = (served.census / 1_000).max(1);
    let (months, shares) = (
        thousands * MONTHS_PER_THOUSAND,
        thousands * SHARES_PER_THOUSAND,
    );
    let feed = served.root.join("audit").join("feed-reconcile");
    let tick = served.root.join("audit").join("tick-to-second");
    for dir in [&feed, &tick] {
        if let Err(why) = std::fs::create_dir_all(dir) {
            refuse(&format!("the bench check folder would not create: {why}"));
        }
    }
    let newest = month_name(0);
    for at in 0..months {
        let month = month_name(at);
        let path = feed.join(format!("feed-reconcile-{month}.v1.csv"));
        if let Err(why) = std::fs::write(&path, feed_month(&month, shares)) {
            refuse(&format!("the bench month would not write: {why}"));
        }
    }
    let (csv, index) = tick_year(shares);
    let written = std::fs::write(tick.join(format!("tick-to-second-{YEAR}.v001.csv")), csv)
        .and_then(|()| std::fs::write(tick.join(format!("tick-to-second-{YEAR}.v001.idx")), index));
    if let Err(why) = written {
        refuse(&format!("the bench year would not write: {why}"));
    }
    (newest, format!("{months}"))
}

/// Every target at 1x, 10x and 100x, each p50 ratio gated. Run by
/// `crates/api/benches/ratio.rs`'s `served_routes_are_flat`.
pub(crate) fn checks_are_flat(stores: &mut [Served]) -> bool {
    flat("feed-checks", stores, |served| {
        let (newest, months) = checks_into(served);
        vec![
            ok_with(
                "a page of the months held",
                "/feed-checks.json?limit=1&offset=0".to_owned(),
                &format!("\"total\":{months}"),
            ),
            ok_with(
                "a page of one month's shares",
                format!("/feed-checks.json?month={newest}&limit=5&offset=0"),
                "\"returned\":5",
            ),
            ok_with(
                "one share's days",
                format!("/feed-checks.json?month={newest}&symbol=ZZB00003&limit=1&offset=0"),
                "\"returned\":1",
            ),
            ok_with(
                "a page of one year's months",
                format!("/tick-checks.json?year={YEAR}&limit=1&offset=0"),
                "\"returned\":1",
            ),
            ok_with(
                "a page of one month's shares, tick",
                format!("/tick-checks.json?month={YEAR}-02&limit=5&offset=0"),
                "\"returned\":5",
            ),
            ok_with(
                "a year's excluded days",
                "/calendar-days.json?year=2024&limit=1&offset=0".to_owned(),
                "\"returned\":1",
            ),
        ]
    })
}
