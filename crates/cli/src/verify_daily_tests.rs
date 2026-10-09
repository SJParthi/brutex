//! V-04 on every daily stamp the store admits, folded as `cli verify` folds
//! it. F-CEC7A0, D-4756.
//!
//! D-1790 proved V-04 for daily bars stamped at IST midnight, in `indicators`.
//! The store admits a daily bar at any whole second (`store::file`'s
//! `OffGrid` doc, D-0915: vendors stamp a daily bar at midnight, the open or
//! the close), and `cli verify` folds the stored daily series through the
//! production evaluator. These tests write daily bars at the three stamps
//! through `store`, load them with `stored::load_span`, and fold them through
//! [`super::verify_series`], [`super::evaluator_stored`] and `Column::build`,
//! the path `measured_series_checks` takes.
#![allow(clippy::expect_used, reason = "a failed fixture must fail its test")]

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use indicators::column::Column;
use store::file::BarFile;
use store::format::Bar;
use store::path::{FileKind, StorePath, Timeframe, YearMonth};

/// The months each fixture store holds: fifteen, so a daily series warms.
const MONTHS: [(u16, u8); 15] = [
    (2024, 1),
    (2024, 2),
    (2024, 3),
    (2024, 4),
    (2024, 5),
    (2024, 6),
    (2024, 7),
    (2024, 8),
    (2024, 9),
    (2024, 10),
    (2024, 11),
    (2024, 12),
    (2025, 1),
    (2025, 2),
    (2025, 3),
];

const DAY_MICROS: i64 = 86_400_000_000;

fn temp_root(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "brutex-verify-daily-{tag}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ignored = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("a fixture store root");
    root
}

/// One daily bar per measured open IST day of `MONTHS`, every bar stamped at
/// IST minute-of-day `minute` (and `extra`, when given, a second bar on the
/// first open day at that minute), written through `store` for `underlying`.
fn daily_store(tag: &str, underlying: &str, minute: i64, extra: Option<i64>) -> PathBuf {
    let root = temp_root(tag);
    let key = crate::stored::swept_index(underlying).expect("a swept instrument");
    let symbol = u32::from_le_bytes(
        brutex_core::universe::fnv1a(underlying)
            .to_le_bytes()
            .get(..4)
            .and_then(|prefix| prefix.try_into().ok())
            .expect("four bytes"),
    );
    let mut ordinal = 0_i64;
    for (year, month) in MONTHS {
        let mut bars = Vec::new();
        for day in 1..=31 {
            let Ok(date) = pull::session::Day::new(year, month, day) else {
                continue;
            };
            let civil = i64::from(date.days_from_epoch());
            if !matches!(
                pull::calendar::kind_of(civil),
                pull::calendar::DayKind::Open(_)
            ) || indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS.contains(&civil)
            {
                continue;
            }
            let at = |minute: i64| {
                civil * DAY_MICROS + minute * 60_000_000 - indicators::IST_OFFSET_MICROS
            };
            let mid = 2_500_000 + ((ordinal * 137) % 811 - 405) * 70;
            let bar = |ts_micros: i64| Bar {
                ts_micros,
                open: mid,
                high: mid + 9_000 + (ordinal % 11) * 200,
                low: mid - 9_000 - (ordinal % 7) * 200,
                close: mid + ((ordinal % 5) - 2) * 400,
                volume: 1_000 + ordinal,
                open_interest: i64::MIN,
            };
            bars.push(bar(at(minute)));
            if let Some(second) = extra.filter(|_| ordinal == 0) {
                bars.push(bar(at(second)));
            }
            ordinal += 1;
        }
        let path = StorePath::for_key(
            brutex_core::vendor::Vendor::Zerodha,
            &key,
            Timeframe::DAY_1,
            YearMonth::new(year, month).expect("a month"),
            FileKind::Bars,
        )
        .expect("a store path");
        let mut writer = BarFile::open_or_create(&root, path, symbol).expect("a daily file");
        writer
            .append(&bars)
            .expect("the store admits every stamp here");
    }
    root
}

fn load(root: &Path, underlying: &str) -> crate::stored::Span {
    crate::stored::load_span(
        root,
        brutex_core::vendor::Vendor::Zerodha,
        underlying,
        "1day",
        *MONTHS.first().expect("a first month"),
        *MONTHS.last().expect("a last month"),
    )
    .expect("the daily span loads")
}

/// Every vocabulary position set in any row of `rows`.
fn set_positions(rows: &[vocab::ConditionMask]) -> BTreeSet<u16> {
    (0..u16::try_from(vocab::table::TABLE.len()).expect("the table fits u16"))
        .filter(|&p| rows.iter().any(|row| row.get(u32::from(p))))
        .collect()
}

/// **V-04 holds for a daily bar stamped at midnight, at the 09:15 open and
/// at the 15:30 close, for an index and for a share, folded as `cli verify`
/// folds it.** F-CEC7A0, D-4756.
///
/// Before D-4756 the 09:15 stamp set `early_morning` (bit 44) on every daily
/// row: `cli verify` folded the stored stamps as they were, and V-04 held only
/// because a midnight bar is before the open. The three stamps now fold to the
/// same column, bit for bit, known mask included.
#[test]
fn every_daily_stamp_the_store_admits_folds_with_no_time_of_day_or_vwap_bit() {
    let mut cleared: BTreeSet<u16> = [44, 45, 46, 47].into_iter().collect();
    cleared.extend(indicators::vwap::positions());
    for underlying in ["NIFTY", "RELIANCE"] {
        let mut columns = Vec::new();
        for (stamp, minute) in [("0000", 0_i64), ("0915", 555), ("1530", 930)] {
            let root = daily_store(&format!("{underlying}-{stamp}"), underlying, minute, None);
            let span = load(&root, underlying);
            assert!(span.complete(), "{underlying} {stamp}: every month is held");
            let series = super::verify_series(&span, usize::MAX).expect("the series folds");
            assert_eq!(series.len(), span.bars.len(), "no bar is dropped");
            for (folded, stored) in series.iter().zip(&span.bars) {
                assert_eq!(
                    indicators::ist_day(folded.ts_micros),
                    indicators::ist_day(stored.ts_micros),
                    "a daily bar keeps its IST day"
                );
                assert_eq!(
                    (folded.open, folded.high, folded.low, folded.close),
                    (stored.open, stored.high, stored.low, stored.close),
                );
                assert_eq!(folded.volume, stored.volume);
            }
            let mut evaluator =
                super::evaluator_stored(crate::stored::vwap_availability(&span.key))
                    .expect("the pinned evaluator");
            let column = Column::build(&series, &mut evaluator);
            assert!(
                !column.is_empty(),
                "{underlying} {stamp}: the daily series must warm and emit rows"
            );
            let set = set_positions(column.bits());
            let breached: BTreeSet<u16> = set.intersection(&cleared).copied().collect();
            assert!(
                breached.is_empty(),
                "{underlying} {stamp}: V-04 positions set on the daily rung: {breached:?}"
            );
            columns.push((stamp, column.bits().to_vec(), column.known().to_vec()));
            let _ignored = std::fs::remove_dir_all(&root);
        }
        let (_, bits, known) = columns.first().expect("three columns").clone();
        for (stamp, other_bits, other_known) in &columns {
            assert!(
                *other_bits == bits && *other_known == known,
                "{underlying} {stamp}: a daily stamp moved the folded column"
            );
        }
    }
}

/// **Two daily bars on one IST day are refused by name, never folded as two
/// sessions or collapsed into one.** F-CEC7A0, D-4756.
///
/// The store admits them, a midnight bar and a 15:30 bar being strictly
/// increasing. Mapping both to one stamp would hand the evaluator a duplicate
/// timestamp, and keeping both would fold one session's OHLC twice.
#[test]
fn two_daily_bars_on_one_ist_day_are_refused_by_name() {
    let root = daily_store("twice", "NIFTY", 0, Some(930));
    let span = load(&root, "NIFTY");
    let first_day = indicators::ist_day(span.bars.first().expect("bars").ts_micros);
    let why = super::verify_series(&span, usize::MAX)
        .expect_err("one IST day cannot hold two daily bars");
    assert!(why.contains(&first_day.to_string()), "{why}");
    assert!(why.contains("two daily bars"), "{why}");
    let _ignored = std::fs::remove_dir_all(&root);
}

/// **Only the daily rung is restamped, and `limit` still bounds the fold.**
/// F-CEC7A0, D-4756.
///
/// The same bars labelled with an intraday rung pass through with every
/// stamp as stored: restamping an intraday bar to its midnight would merge a
/// whole session into one instant. On the daily rung the fold keeps the first
/// `limit` bars, each at its IST midnight.
#[test]
fn only_the_daily_rung_is_restamped_and_the_limit_bounds_the_fold() {
    let root = daily_store("rungs", "NIFTY", 555, None);
    let span = load(&root, "NIFTY");
    let intraday = crate::stored::Span {
        timeframe: "1min",
        ..load(&root, "NIFTY")
    };
    let raw = super::verify_series(&intraday, usize::MAX).expect("an intraday span folds");
    assert_eq!(raw, intraday.bars, "an intraday stamp is never moved");
    let daily = super::verify_series(&span, 7).expect("the daily span folds");
    assert_eq!(daily.len(), 7, "the fold takes exactly `limit` bars");
    for (folded, stored) in daily.iter().zip(&span.bars) {
        assert_eq!(
            folded.ts_micros,
            stored.ts_micros - 555 * 60_000_000,
            "a 09:15 daily bar folds at its IST midnight"
        );
    }
    let _ignored = std::fs::remove_dir_all(&root);
}
