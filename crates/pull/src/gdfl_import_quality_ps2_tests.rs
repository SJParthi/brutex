#![cfg(test)]
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a \
              test that cannot panic cannot fail."
)]

//! BASE-COV-PULL-PS2 (D-7336): the day ledger's edges its other tests leave
//! unreached: the count of files taken, a traded file with no first trade, a
//! share gap that is no gap of the day, the shared holes of a kind that keeps
//! none, and the flags ledger's lines that are no flags lines. Every fixture
//! is invented.

use super::*;

fn day() -> Day {
    Day::new(2019, 11, 14).unwrap()
}

fn ledger(kind: ImportKind) -> DayLedger {
    let session = crate::gdfl_cm::session_of(day()).unwrap();
    DayLedger::new(kind, day(), session)
}

/// THE FILES TAKEN ARE COUNTED, traded or not.
#[test]
fn the_day_ledger_counts_every_file_it_takes() {
    let mut options = ledger(ImportKind::Options);
    assert_eq!(options.files(), 0);
    options.take("*", &FileQuality::default());
    options.take(
        "*",
        &FileQuality {
            traded: true,
            ..FileQuality::default()
        },
    );
    assert_eq!(options.files(), 2);
}

/// A TRADED FILE WITH NO FIRST TRADE adds nothing to the median, and a share
/// gap that runs past the day's last second is no gap of the day: neither
/// moves the shared holes.
#[test]
fn a_traded_file_without_a_first_trade_and_a_gap_past_the_day_count_nothing() {
    let mut shares = ledger(ImportKind::Stocks);
    let past = u32::try_from(DAY_SECONDS).unwrap();
    shares.take(
        "INVENTED",
        &FileQuality {
            traded: true,
            first_trade: None,
            share_gaps: vec![(past - 2, 10)],
            ..FileQuality::default()
        },
    );
    assert_eq!(shares.first_trade_p50(), None, "no first trade to count");
    assert_eq!(
        shares.shared_holes(),
        (0, None),
        "a gap past the day is not counted"
    );
    let mut inside = ledger(ImportKind::Stocks);
    inside.take(
        "INVENTED",
        &FileQuality {
            traded: true,
            first_trade: Some(40_000),
            share_gaps: vec![(40_000, 600)],
            ..FileQuality::default()
        },
    );
    assert_eq!(inside.first_trade_p50(), Some(40_000));
    assert_eq!(inside.shared_holes(), (1, Some((40_000, 600))));
}

/// A KIND THAT KEEPS NO SHARED HOLES answers none, however many files traded.
#[test]
fn a_kind_without_shared_holes_answers_none_after_a_trade() {
    let mut options = ledger(ImportKind::Options);
    options.take(
        "*",
        &FileQuality {
            traded: true,
            first_trade: Some(36_000),
            share_gaps: vec![(36_000, 60)],
            ..FileQuality::default()
        },
    );
    assert_eq!(options.shared_holes(), (0, None));
}

/// A FLAGS LEDGER KEEPS ONLY FLAGS LINES: a line of another kind on disk is
/// left for the reader, and a fresh head that is no flags line is never
/// written.
#[test]
fn the_flags_ledger_keys_only_flags_lines() {
    let dir = std::env::temp_dir().join(format!(
        "brutex-quality-ps2-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos())
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("dayflags");
    let old = render("flags options 2019-11-14 *", Src::Import, "rows=1");
    std::fs::write(&path, format!("run 1 invented\n{old}\nflags torn\n")).unwrap();
    let mut flags = Ledger::load(path.clone()).unwrap();
    assert!(
        flags
            .news(
                vec![("flags options 2019-11-14 *".to_owned(), "rows=1".to_owned())],
                Src::Rescan
            )
            .is_empty(),
        "the same line again, from another source, says nothing new"
    );
    assert!(
        flags
            .news(
                vec![("notflags a b c".to_owned(), "rows=2".to_owned())],
                Src::Import
            )
            .is_empty(),
        "a head that is no flags line is never written"
    );
    let news = flags.news(
        vec![("flags options 2019-11-14 *".to_owned(), "rows=2".to_owned())],
        Src::Import,
    );
    assert_eq!(news.len(), 1, "{news:?}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// A SHARE ROW WITH NO TRADE AND NO PRICE after a trade is no new price: only
/// a row that names another price than the last trade's is counted.
#[test]
fn a_no_trade_row_with_no_price_is_no_new_price() {
    let on = day();
    let session = crate::gdfl_cm::session_of(on).unwrap();
    let tick = |sod: u32, ltp: i64, ltq: u64| Tick {
        sod,
        ltp,
        ltq,
        oi: None,
    };
    let measured = |ticks: &[Tick]| {
        let mut seconds: Vec<u32> = ticks.iter().map(|t| t.sod).collect();
        seconds.dedup();
        measure_file(
            ImportKind::Stocks,
            (&session, halt_window(ImportKind::Stocks, on)),
            ticks,
            &seconds,
            (Placement::default(), Deferred::default()),
            ReaderCounts::default(),
        )
    };
    let unpriced = measured(&[tick(40_000, 3_100_000, 5), tick(40_001, 0, 0)]);
    assert_eq!(unpriced.ltq0_new_ltp_rows, 0);
    let repriced = measured(&[tick(40_000, 3_100_000, 5), tick(40_001, 3_100_005, 0)]);
    assert_eq!(repriced.ltq0_new_ltp_rows, 1);
    let same = measured(&[tick(40_000, 3_100_000, 5), tick(40_001, 3_100_000, 0)]);
    assert_eq!(same.ltq0_new_ltp_rows, 0);
}
