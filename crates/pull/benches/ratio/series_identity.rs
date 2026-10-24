//! C-CHK-08 (CC-20, FM-615, D-6804): `pull::series_identity::audit`, the
//! store scan's identity check of a month's stored minutes against the
//! served day bars. It is O(minutes) per month by design, and this bench
//! asserts its cost PER BAR flat at 1x, 10x and 100x the minutes (one, ten
//! and a hundred sessions of 375). Each bar is one `ist_day`, one
//! `MinuteJudge::judge` step and one `Tally::count`; the judge alone is
//! C-ZI-02 (`ratio/identity.rs`), so what is new here is the whole step and
//! `Tally::count`, whose own doc calls it O(1) for minutes in time order and
//! is on `docs/06-limits.md`'s list of claims that name no proof.
//!
//! Two answers are timed. One is held minutes only. The other cycles a held,
//! a suspect, a foreign-level and a non-positive minute, so three of every
//! four bars go through `flag`, which looks among the last groups of the same
//! day; its sessions cross a month and a year boundary. That answer is timed
//! again with its sessions in REVERSE order (each session's minutes still in
//! time order), the out-of-order case: `flag` stops at the first group of
//! another day, so the cost per bar must not move. The harness,
//! `crates/pull/benches/ratio.rs`, fails on a ratio above its ceiling and
//! prints p50, p99 and max beside each.
//!
//! Every price is invented here, in paisa; nothing is a vendor's.

use std::hint::black_box;

use pull::series_identity::{DayBook, FirstDay, audit};
use store::format::Bar;

use super::{cost_ps, ratio, refuse, spread};

/// Minutes in one session.
const SESSION: i64 = 375;
/// 2023-12-20 as IST days since the epoch: a hundred sessions from it cross
/// the month and the year.
const FIRST_DAY: i64 = 19_711;
/// The day bar's range: every minute is judged against it.
const DAY_LOW: i64 = 999_000;
const DAY_HIGH: i64 = 1_002_000;

/// One minute of session `s`, minute `m`, priced so its verdict is `held`
/// when `flagged` is false, and otherwise cycles held, suspect (about 3%
/// above the day's high), foreign (about 10% above) and non-positive.
fn minute(s: i64, m: i64, flagged: bool) -> Bar {
    let midnight = (FIRST_DAY + s) * 86_400 - 19_800;
    let base = 1_000_000 + (m % 97) * 13;
    let (open, high, low, close) = match (flagged, m % 4) {
        (true, 1) => (1_032_000, 1_032_500, 1_031_900, 1_032_100),
        (true, 2) => (1_105_000, 1_105_500, 1_104_900, 1_105_100),
        (true, 3) => (0, base, 0, base),
        _ => (base, base + 50, base - 40, base + 10),
    };
    Bar {
        ts_micros: (midnight + 33_300 + m * 60) * 1_000_000,
        open,
        high,
        low,
        close,
        volume: 0,
        open_interest: i64::MIN,
    }
}

/// The day bars of `sessions` sessions and their minutes, sessions in order
/// or reversed.
fn answer(sessions: i64, flagged: bool, reversed: bool) -> (Vec<Bar>, Vec<Bar>) {
    let mut days = Vec::new();
    let mut minutes = Vec::new();
    let order: Vec<i64> = if reversed {
        (0..sessions).rev().collect()
    } else {
        (0..sessions).collect()
    };
    for s in order {
        let midnight = (FIRST_DAY + s) * 86_400 - 19_800;
        days.push(Bar {
            ts_micros: midnight * 1_000_000,
            open: 1_000_000,
            high: DAY_HIGH,
            low: DAY_LOW,
            close: 1_000_500,
            volume: 0,
            open_interest: i64::MIN,
        });
        minutes.extend((0..SESSION).map(|m| minute(s, m, flagged)));
    }
    (days, minutes)
}

/// The verdicts an answer must reach, or the timing below measures a
/// different path than its label says.
fn check(sessions: i64, flagged: bool, minutes: &[Bar], book: &DayBook) {
    let tally = audit(minutes, book);
    let n = u64::try_from(minutes.len()).unwrap_or(0);
    // Each code holds at least 93 of a session's 375 minutes.
    let quarter = u64::try_from(sessions * (SESSION / 4)).unwrap_or(0);
    let right = if flagged {
        tally.minutes() == n
            && tally.suspect_level >= quarter
            && tally.foreign_level >= quarter
            && tally.non_positive_price >= quarter
            && tally.flagged.len() == usize::try_from(sessions * 3).unwrap_or(0)
    } else {
        tally.held == n && tally.flagged.is_empty()
    };
    if !right {
        refuse(&format!(
            "the audit's verdicts on an invented answer ({sessions} sessions, flagged \
             {flagged}): {tally:?}"
        ));
    }
}

/// Picoseconds per bar of one whole `audit` of `sessions` sessions.
fn per_bar(sessions: i64, flagged: bool, reversed: bool) -> u128 {
    let (days, minutes) = answer(sessions, flagged, reversed);
    let book = DayBook::new(&days, FirstDay::NoDayRung);
    check(sessions, flagged, &minutes, &book);
    let reps = u32::try_from(100 / sessions).unwrap_or(1).max(1);
    let whole = cost_ps(reps, || audit(black_box(&minutes), black_box(&book)));
    whole / u128::try_from(minutes.len()).unwrap_or(u128::MAX).max(1)
}

/// C-CHK-08: the audit per bar at 1x, 10x and 100x the minutes, held and
/// flagged, and the flagged answer out of order.
pub(crate) fn audit_is_flat_per_bar() -> bool {
    let mut ok = true;
    for (label, flagged) in [("held minutes", false), ("flagged minutes", true)] {
        let [one, ten, hundred] = [1, 10, 100].map(|s| per_bar(s, flagged, false));
        ok &= ratio(
            &format!("C-CHK-08 audit per bar, {label}, 10x minutes"),
            one,
            ten,
        );
        ok &= ratio(
            &format!("C-CHK-08 audit per bar, {label}, 100x minutes"),
            one,
            hundred,
        );
        if flagged {
            ok &= ratio(
                "C-CHK-08 audit per bar, flagged, 100x reversed",
                one,
                per_bar(100, true, true),
            );
        }
    }
    // One whole month's audit, sampled: the per-run figure the row states.
    let (days, minutes) = answer(22, true, false);
    let book = DayBook::new(&days, FirstDay::NoDayRung);
    check(22, true, &minutes, &book);
    spread(
        "C-CHK-08 audit of a 22-session month (8,250 bars)",
        200,
        || {
            black_box(audit(black_box(&minutes), black_box(&book)));
        },
    );
    ok
}
