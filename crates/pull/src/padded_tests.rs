//! The proofs of the vendor-padded minute (INT-15, R4-27): the definition at
//! its edges, the count per day equal between a full read and the month
//! summary, and a derived bucket that never opens on a padded minute.
//!
//! Every bar is invented: the days are real NSE sessions of March 2024 (the
//! calendar must know them, and 2024-03-02 is the disaster-recovery Saturday
//! whose two windows are the halt case), the prices are near ₹7,777, which no
//! share in this store printed on those days. Nothing is removed: every store
//! root is a new directory of its own.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

use brutex_core::vendor::Vendor;
use store::file::BarFile;
use store::format::{Bar, OI_NULL};
use store::month_summary::{MonthSummary, NotCounted, read_beside};
use store::path::{FileKind, PathParts, StorePath, Timeframe, YearMonth};

use super::{PaddedDay, Uncounted, Walk, is_padded, per_day, per_day_from_summary, window_of};
use crate::calendar::{DayKind, Runtime};
use crate::fold::{Bucket, PaddedFold, derive_minutes_with};
use crate::vendor::Venue;

const MICROS: i64 = 1_000_000;
/// 2024-03-01, a Friday, as days since the epoch.
const FRI: i64 = 19_783;
/// 2024-03-02, the disaster-recovery Saturday: 09:15-09:59 and 11:30-12:29.
const DR_SAT: i64 = 19_784;
/// 2024-03-04, a Monday.
const MON: i64 = 19_786;
/// 2024-03-05, a Tuesday.
const TUE: i64 = 19_787;
/// 2024-03-06, a Wednesday.
const WED: i64 = 19_788;
/// 09:15, as a minute of the day.
const OPEN: i64 = 555;
/// Invented: no share printed this.
const BASE: i64 = 777_700;

/// The stamp of minute-of-day `minute` on IST epoch day `day`.
fn at(day: i64, minute: i64) -> i64 {
    (day * 86_400 - 19_800 + minute * 60) * MICROS
}

/// A traded minute: a real range and some volume.
fn traded(day: i64, minute: i64, price: i64) -> Bar {
    Bar {
        ts_micros: at(day, minute),
        open: price,
        high: price + 15,
        low: price - 10,
        close: price + 5,
        volume: 40 + minute,
        open_interest: OI_NULL,
    }
}

/// A flat zero-volume minute at `price`.
fn flat(day: i64, minute: i64, price: i64) -> Bar {
    Bar {
        ts_micros: at(day, minute),
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 0,
        open_interest: OI_NULL,
    }
}

/// A flat zero-volume minute repeating `before`'s close: a padded minute
/// when it is in `before`'s window.
fn pad(day: i64, minute: i64, before: &Bar) -> Bar {
    flat(day, minute, before.close)
}

fn counts(minutes: &[Bar]) -> Vec<PaddedDay> {
    per_day(minutes, Venue::NseCash, None, Runtime::default()).unwrap()
}

fn padded_on(minutes: &[Bar], day: i64) -> u32 {
    counts(minutes)
        .into_iter()
        .find(|d| d.day == day)
        .map_or(0, |d| d.padded)
}

fn fold_under(minutes: &[Bar], secs: u32, rule: PaddedFold) -> crate::fold::Derived {
    derive_minutes_with(
        minutes,
        Bucket::of_secs(secs).unwrap(),
        Venue::NseCash,
        None,
        Runtime::default(),
        rule,
    )
    .unwrap()
}

/// A whole regular session of traded minutes on `day`, every price distinct.
fn traded_session(day: i64) -> Vec<Bar> {
    (OPEN..OPEN + 375)
        .map(|m| traded(day, m, BASE + (m - OPEN) * 3))
        .collect()
}

#[test]
fn the_rule_in_force_is_absent_and_every_rule_has_one_name() {
    assert_eq!(crate::fold::PADDED_FOLD, PaddedFold::Absent, "D-7270");
    let names = [
        PaddedFold::AsTrade,
        PaddedFold::KeepFlat,
        PaddedFold::Absent,
    ]
    .map(PaddedFold::name);
    assert_eq!(names, ["as_trade", "keep_flat", "absent"]);
}

#[test]
fn the_predicate_is_one_comparison_of_four_prices_and_a_volume() {
    let before = traded(MON, OPEN, BASE);
    let carry = pad(MON, OPEN + 1, &before);
    assert!(is_padded(&before, &carry, true));
    assert!(!is_padded(&before, &carry, false), "another session window");
    for broken in [
        Bar { volume: 1, ..carry },
        Bar {
            open: carry.open + 5,
            ..carry
        },
        Bar {
            high: carry.high + 5,
            ..carry
        },
        Bar {
            low: carry.low - 5,
            ..carry
        },
        Bar {
            close: carry.close + 5,
            ..carry
        },
    ] {
        assert!(!is_padded(&before, &broken, true), "{broken:?}");
    }
    // Flat and silent at ANOTHER price is not a carry of the bar before.
    assert!(!is_padded(&before, &flat(MON, OPEN + 1, before.open), true));
}

#[test]
fn a_padded_looking_open_is_never_padded() {
    // FRIDAY closes at its last traded close; MONDAY opens flat and silent at
    // exactly that price. It looks padded and is the first bar of its
    // session: never.
    let friday = traded_session(FRI);
    let last = *friday.last().unwrap();
    let mut month = friday;
    month.push(pad(MON, OPEN, &last));
    month.push(traded(MON, OPEN + 1, BASE));
    assert_eq!(padded_on(&month, MON), 0);
    // And the month's first bar of all, with nothing before it.
    assert_eq!(padded_on(&[flat(MON, OPEN, BASE)], MON), 0);
}

#[test]
fn a_padded_minute_is_counted_and_not_a_trade() {
    // 09:15 trades; 09:16-09:18 are the vendor's flat carries; 09:19 trades.
    let open = traded(MON, OPEN, BASE);
    let mut month = vec![open];
    for m in 1..=3 {
        month.push(pad(MON, OPEN + m, &open));
    }
    month.push(traded(MON, OPEN + 4, BASE + 20));
    assert_eq!(
        counts(&month),
        vec![PaddedDay {
            day: MON,
            bars: 5,
            padded: 3
        }],
        "counted"
    );
    // NOT A TRADE: the 09:15 five-minute bucket holds five minutes and two
    // trades, so it is partial; its volume is the two trades' and nothing
    // else; it is the same bar under every rule that reads padding.
    for rule in [PaddedFold::KeepFlat, PaddedFold::Absent] {
        let derived = fold_under(&month, 300, rule);
        assert_eq!(derived.bars.len(), 1, "{rule:?}");
        assert_eq!(
            derived.tally.partial, 1,
            "{rule:?}: two trades of five minutes"
        );
        assert_eq!(derived.bars[0].volume, month[0].volume + month[4].volume);
        assert_eq!(derived.bars[0].high, month[4].high);
        assert_eq!(derived.bars[0].low, open.low);
    }
    // The fold before INT-15 called the same bucket complete.
    let before = fold_under(&month, 300, PaddedFold::AsTrade);
    assert_eq!(before.tally.partial, 0);
    assert_eq!(
        before.bars,
        fold_under(&month, 300, PaddedFold::Absent).bars
    );
}

#[test]
fn a_bucket_opens_on_its_first_unpadded_minute() {
    // 09:15-09:19 trades up to a close of BASE + 80. 09:20-09:21 carry that
    // close flat; the first trade of the 09:20 bucket is at 09:22, far below.
    let mut month: Vec<Bar> = (0..5)
        .map(|m| traded(MON, OPEN + m, BASE + m * 15))
        .collect();
    let last = *month.last().unwrap();
    month.push(pad(MON, OPEN + 5, &last));
    month.push(pad(MON, OPEN + 6, &last));
    let first_trade = traded(MON, OPEN + 7, BASE - 300);
    month.push(first_trade);
    month.push(traded(MON, OPEN + 8, BASE - 290));
    month.push(traded(MON, OPEN + 9, BASE - 280));
    let derived = fold_under(&month, 300, PaddedFold::Absent);
    let second = derived.bars[1];
    assert_eq!(second.ts_micros, at(MON, OPEN + 5));
    assert_eq!(
        second.open, first_trade.open,
        "opens on its first unpadded minute"
    );
    assert_eq!(second.high, BASE - 280 + 15, "the carried price is no high");
    assert_eq!(second.low, first_trade.low);
    assert_eq!(second.close, month[9].close);
    assert_eq!(
        second.volume,
        month[7..].iter().map(|b| b.volume).sum::<i64>()
    );
    // Before INT-15 the bucket opened at the carried close, a price nobody
    // paid in it, and that price was its high.
    let before = fold_under(&month, 300, PaddedFold::AsTrade).bars[1];
    assert_eq!(before.open, last.close);
    assert_eq!(before.high, last.close);
    // The first bucket, with no padded minute, is the same under both.
    assert_eq!(
        derived.bars[0],
        fold_under(&month, 300, PaddedFold::AsTrade).bars[0]
    );
}

#[test]
fn a_bucket_of_only_padded_minutes_follows_its_rule() {
    let open = traded(MON, OPEN, BASE);
    let mut month = vec![open];
    for m in 1..10 {
        month.push(pad(MON, OPEN + m, &open));
    }
    month.push(traded(MON, OPEN + 10, BASE + 50));
    // 09:20-09:24 holds five padded minutes and nothing else.
    let absent = fold_under(&month, 300, PaddedFold::Absent);
    assert_eq!(
        absent.bars.iter().map(|b| b.ts_micros).collect::<Vec<_>>(),
        vec![at(MON, OPEN), at(MON, OPEN + 10)],
        "no trade, so no bar, as a bucket with no minute at all"
    );
    assert_eq!(absent.tally.withheld, 0, "not withheld either");
    assert_eq!(absent.tally.buckets, 2);
    let kept = fold_under(&month, 300, PaddedFold::KeepFlat);
    assert_eq!(kept.bars.len(), 3);
    assert_eq!(kept.bars[1].open, open.close);
    assert_eq!(kept.bars[1].volume, 0);
    assert_eq!(kept.tally.partial, 3, "every bucket short of trades");
    let as_trade = fold_under(&month, 300, PaddedFold::AsTrade);
    assert_eq!(
        as_trade.bars[1], kept.bars[1],
        "keep-flat keeps today's bar"
    );
}

#[test]
fn a_whole_padded_session_is_counted_and_folds_to_its_one_trade() {
    // One trade at 09:15, then the vendor's flat carry to 15:29.
    let open = traded(MON, OPEN, BASE);
    let mut month = vec![open];
    for m in 1..375 {
        month.push(pad(MON, OPEN + m, &open));
    }
    assert_eq!(padded_on(&month, MON), 374);
    let derived = fold_under(&month, 900, PaddedFold::Absent);
    assert_eq!(derived.bars.len(), 1, "only the 09:15 bucket traded");
    assert_eq!(derived.bars[0].open, open.open);
    assert_eq!(derived.bars[0].high, open.high);
    assert_eq!(derived.tally.partial, 1);
    assert_eq!(fold_under(&month, 900, PaddedFold::KeepFlat).bars.len(), 25);
}

#[test]
fn a_padded_minute_after_a_halt_is_not_padded() {
    // The DR Saturday trades 09:15-09:59, stops, and trades 11:30-12:29.
    let session = crate::fold::session_of(DR_SAT, Venue::NseCash, None, Runtime::default());
    let DayKind::Open(s) = session else {
        panic!("2024-03-02 is a recorded special session: {session:?}");
    };
    assert_eq!(s.count, 2, "two windows");
    assert_eq!(window_of(session, 599), Some(0));
    assert_eq!(window_of(session, 600), None, "the halt is in no window");
    assert_eq!(window_of(session, 690), Some(1));
    let last_before = traded(DR_SAT, 599, BASE);
    let month = vec![
        traded(DR_SAT, 598, BASE - 5),
        last_before,
        // The first minute after the halt: flat, silent, at the close before
        // the halt. A new window, so NOT padded.
        pad(DR_SAT, 690, &last_before),
        // The minute after it in the same window: padded.
        pad(DR_SAT, 691, &last_before),
    ];
    assert_eq!(padded_on(&month, DR_SAT), 1);
    let mut walk = Walk::new();
    assert!(!walk.step(&month[0], DR_SAT, Some(0)));
    assert!(!walk.step(&month[1], DR_SAT, Some(0)));
    assert!(!walk.step(&month[2], DR_SAT, Some(1)), "a new window");
    assert!(walk.step(&month[3], DR_SAT, Some(1)));
}

#[test]
fn a_minute_outside_every_window_is_never_padded() {
    let open = traded(MON, OPEN, BASE);
    // 15:30 and 15:31: after the cash close, flat and silent.
    let month = vec![
        open,
        pad(MON, OPEN + 1, &open),
        pad(MON, 930, &open),
        pad(MON, 931, &open),
    ];
    assert_eq!(padded_on(&month, MON), 1, "only 09:16");
}

#[test]
fn an_index_or_a_future_is_never_judged() {
    let open = traded(MON, OPEN, BASE);
    let month = vec![open, pad(MON, OPEN + 1, &open)];
    for venue in [Venue::NseIndex, Venue::NseDerivatives] {
        assert_eq!(
            per_day(&month, venue, None, Runtime::default()),
            Err(Uncounted::NotCash(venue))
        );
        // And every rule folds it exactly as before INT-15.
        let fold = |rule| {
            derive_minutes_with(
                &month,
                Bucket::of_secs(300).unwrap(),
                venue,
                None,
                Runtime::default(),
                rule,
            )
            .unwrap()
        };
        assert_eq!(
            fold(PaddedFold::Absent),
            fold(PaddedFold::AsTrade),
            "{venue}"
        );
        assert_eq!(
            fold(PaddedFold::KeepFlat),
            fold(PaddedFold::AsTrade),
            "{venue}"
        );
    }
    assert!(
        Uncounted::NotCash(Venue::NseIndex)
            .to_string()
            .starts_with("not_cash: ")
    );
}

#[test]
fn every_extreme_of_the_walk_is_counted_not_smoothed() {
    // Empty and single.
    assert_eq!(counts(&[]), Vec::new());
    let one = traded(MON, OPEN, BASE);
    assert_eq!(
        counts(&[one]),
        vec![PaddedDay {
            day: MON,
            bars: 1,
            padded: 0
        }]
    );
    // A day the calendar says is closed (2024-03-03, a Sunday): no window,
    // nothing padded, every bar still counted as a bar.
    let sunday = 19_785;
    let closed = vec![traded(sunday, OPEN, BASE), flat(sunday, OPEN + 1, BASE + 5)];
    assert_eq!(padded_on(&closed, sunday), 0);
    // A duplicate stamp: the second row repeats the first flat and silent in
    // the same window, so it is judged padded, as the fold's walk judges it;
    // the fold refuses the bucket as invalid either way.
    let duplicate = vec![one, pad(MON, OPEN, &one)];
    assert_eq!(padded_on(&duplicate, MON), 1);
    assert_eq!(
        fold_under(&duplicate, 300, PaddedFold::Absent)
            .tally
            .withheld,
        1
    );
    // A stamp with no IST day.
    let far = Bar {
        ts_micros: i64::MAX,
        ..one
    };
    assert_eq!(
        per_day(&[far], Venue::NseCash, None, Runtime::default()),
        Err(Uncounted::Overflow(i64::MAX))
    );
    assert!(Uncounted::Overflow(1).to_string().starts_with("overflow: "));
    // Out of order: the fold refuses by name, under every rule.
    let backwards = vec![traded(MON, OPEN + 1, BASE), traded(MON, OPEN, BASE)];
    for rule in [
        PaddedFold::AsTrade,
        PaddedFold::KeepFlat,
        PaddedFold::Absent,
    ] {
        assert!(
            derive_minutes_with(
                &backwards,
                Bucket::of_secs(300).unwrap(),
                Venue::NseCash,
                None,
                Runtime::default(),
                rule
            )
            .is_err(),
            "{rule:?}"
        );
    }
}

/// A new, empty store root of this test's own. Nothing is ever removed.
fn root(tag: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let at = std::env::temp_dir().join(format!(
        "brutex-padded-{tag}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&at).unwrap();
    at
}

fn march() -> YearMonth {
    YearMonth::new(2024, 3).unwrap()
}

fn cash_path() -> StorePath<'static> {
    StorePath::new(PathParts {
        vendor: Vendor::Zerodha,
        exchange: "NSE",
        segment: "CASH",
        symbol: "PADTEST",
        contract: None,
        timeframe: Timeframe::MINUTE_1,
        month: march(),
        file: FileKind::Bars,
    })
    .unwrap()
}

/// Every edge at once, over five sessions: a padded-looking open, runs of
/// padding mid-session, a whole padded afternoon, the DR Saturday's halt.
fn injected_month() -> Vec<Bar> {
    let mut month = traded_session(FRI);
    // The DR Saturday: its first window ends traded; the second opens flat on
    // the close before the halt (not padded), then carries (padded).
    let first_window: Vec<Bar> = (555..=599).map(|m| traded(DR_SAT, m, BASE + m)).collect();
    let last = *first_window.last().unwrap();
    month.extend(first_window);
    month.extend((690..=749).map(|m| pad(DR_SAT, m, &last)));
    // Monday opens flat on Saturday's close (not padded), then every
    // seventh minute is a run of three carries.
    let saturday_close = month.last().unwrap().close;
    month.push(flat(MON, OPEN, saturday_close));
    let mut m = OPEN + 1;
    while m < OPEN + 375 {
        if (m - OPEN) % 7 == 0 && m + 3 < OPEN + 375 {
            let before = *month.last().unwrap();
            for k in 0..3 {
                month.push(pad(MON, m + k, &before));
            }
            m += 3;
        } else {
            month.push(traded(MON, m, BASE + 2 * (m - OPEN)));
            m += 1;
        }
    }
    // Tuesday trades until 12:00 and the vendor carries the rest.
    let morning: Vec<Bar> = (OPEN..720).map(|m| traded(TUE, m, BASE + m)).collect();
    let noon = *morning.last().unwrap();
    month.extend(morning);
    month.extend((720..930).map(|m| pad(TUE, m, &noon)));
    // Wednesday trades every minute: nothing padded.
    month.extend(traded_session(WED));
    month
}

#[test]
fn padded_minutes_per_day_match_a_full_read() {
    let month = injected_month();
    let full = counts(&month);
    assert_eq!(
        full.iter().map(|d| (d.day, d.padded)).collect::<Vec<_>>(),
        vec![(FRI, 0), (DR_SAT, 59), (MON, 3 * 53), (TUE, 210), (WED, 0)],
        "the injected counts, by construction"
    );
    // Written through the store's own writer: the `.sum` it keeps is
    // version 2, and counted off it, with no bar read, every day agrees.
    let root = root("match");
    let mut writer = BarFile::open_or_create(&root, cash_path(), 7).unwrap();
    for day in [FRI, DR_SAT, MON, TUE, WED] {
        let rows: Vec<Bar> = month
            .iter()
            .copied()
            .filter(|b| (b.ts_micros / MICROS + 19_800).div_euclid(86_400) == day)
            .collect();
        writer.append(&rows).unwrap();
    }
    let kept = read_beside(writer.path(), &writer.header(), march()).unwrap();
    assert!(kept.tracks_carries(), "written at version 2");
    let read = per_day_from_summary(&kept, Venue::NseCash, None, Runtime::default()).unwrap();
    assert_eq!(read, full);
    // The same from a summary built on demand, as a small month's reader
    // builds one.
    let built = MonthSummary::build(&writer.header(), march(), &month);
    assert_eq!(
        per_day_from_summary(&built, Venue::NseCash, None, Runtime::default()).unwrap(),
        full
    );
    // A non-cash reader of the same summary is refused by name.
    assert_eq!(
        per_day_from_summary(&kept, Venue::NseIndex, None, Runtime::default()),
        Err(Uncounted::NotCash(Venue::NseIndex))
    );
}

#[test]
fn a_v1_summary_is_read_as_not_counted_never_as_zero() {
    let month = injected_month();
    let root = root("v1");
    let mut writer = BarFile::open_or_create(&root, cash_path(), 7).unwrap();
    writer.append(&month).unwrap();
    let kept = read_beside(writer.path(), &writer.header(), march()).unwrap();
    // The same summary as version 1 wrote it: the bytes before the carries,
    // with version 1's flags.
    let old = MonthSummary::decode(store::month_summary::VERSION_1, &v1_payload_of(&kept)).unwrap();
    assert!(!old.tracks_carries());
    assert!(old.wants_carries());
    assert_eq!(old.carry_minutes_on(MON), None);
    let read = per_day_from_summary(&old, Venue::NseCash, None, Runtime::default());
    assert_eq!(read, Err(Uncounted::Summary(NotCounted::Version1)));
    assert!(
        read.unwrap_err()
            .to_string()
            .starts_with("summary_v1: padded minutes not counted")
    );
}

/// The version 1 payload of a version 2 one-minute summary: the same bytes
/// without the flat-carry flag and without the carry bitmaps, which follow
/// everything version 1 wrote.
fn v1_payload_of(summary: &MonthSummary) -> Vec<u8> {
    let mut payload = summary.payload();
    let carries: usize = held_minute_spans(summary)
        .iter()
        .map(|(first, last)| usize::try_from(last - first + 1).unwrap().div_ceil(8))
        .sum();
    payload.truncate(payload.len() - carries);
    // The flags word is bytes 12..16; the flat-carry flag is bit 1.
    payload[12] &= !0b10;
    payload
}

/// Each held day's first and last minute of the day, from the minute stamps.
fn held_minute_spans(summary: &MonthSummary) -> Vec<(i64, i64)> {
    let mut spans: Vec<(i64, i64, i64)> = Vec::new();
    for stamp in summary.minute_stamps() {
        let minutes = (stamp / MICROS + 19_800).div_euclid(60);
        let (day, minute) = (minutes.div_euclid(1_440), minutes.rem_euclid(1_440));
        match spans.last_mut() {
            Some(span) if span.0 == day => span.2 = minute,
            _ => spans.push((day, minute, minute)),
        }
    }
    spans
        .into_iter()
        .map(|(_, first, last)| (first, last))
        .collect()
}
