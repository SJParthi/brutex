//! C-PD-01 and C-PD-02 (R4-27, INT-15): the derived fold of a cash share's
//! minutes, which now judges every minute for vendor padding, and the full
//! read that counts padded minutes per day, each per minute at 1x, 10x and
//! 100x a month's minutes.
//!
//! C-PD-01 is `pull::fold::derive_minutes` to five minutes under the rule in
//! force: one padded judgement a minute (`pull::padded::Walk::step`, a window
//! lookup over at most two windows and one comparison of four prices) on top
//! of the completeness fold, with one calendar lookup a day. C-PD-02 is
//! `pull::padded::per_day`, the same walk alone. Both must cost the same per
//! minute at 1,000, 10,000 and 100,000 minutes; p50, p99 and max of whole
//! folds are printed beside each ratio.
//!
//! Every minute is invented: real NSE sessions from 2022-01-03 (the calendar
//! must attest a day for the fold to read it), prices near ₹7,777 that no
//! share printed, every seventh minute a flat carry of the minute before.

use std::hint::black_box;

use pull::calendar::{DayKind, Runtime};
use pull::fold::{Bucket, derive_minutes};
use pull::vendor::Venue;
use store::format::{Bar, OI_NULL};

use super::{cost_ps, ratio, refuse, spread};

/// Minutes folded: 1x / 10x / 100x.
const SIZES: [usize; 3] = [1_000, 10_000, 100_000];

/// 2022-01-03 as days since the epoch.
const FIRST_DAY: i64 = 18_995;

/// `n` minutes of a cash share over consecutive sessions, 09:15 onward.
fn minutes(n: usize) -> Vec<Bar> {
    let runtime = Runtime::default();
    let mut out: Vec<Bar> = Vec::with_capacity(n);
    let mut day = FIRST_DAY;
    while out.len() < n {
        if matches!(runtime.kind_of(day), DayKind::Open(_)) {
            for m in 555..930_i64 {
                if out.len() == n {
                    break;
                }
                let ts = (day * 86_400 - 19_800 + m * 60) * 1_000_000;
                let price = 777_700 + (m * 13 + day) % 211;
                let bar = match out.last() {
                    Some(before) if m % 7 == 0 && m > 555 => Bar {
                        ts_micros: ts,
                        open: before.close,
                        high: before.close,
                        low: before.close,
                        close: before.close,
                        volume: 0,
                        open_interest: OI_NULL,
                    },
                    _ => Bar {
                        ts_micros: ts,
                        open: price,
                        high: price + 9,
                        low: price - 7,
                        close: price + 3,
                        volume: 50 + m % 17,
                        open_interest: OI_NULL,
                    },
                };
                out.push(bar);
            }
        }
        day += 1;
    }
    out
}

/// C-PD-01 and C-PD-02.
pub(crate) fn padded_fold_is_flat_per_minute() -> bool {
    let five = Bucket::of_secs(300).unwrap_or_else(|| refuse("five minutes is a bucket"));
    let mut fold_per: Vec<u128> = Vec::with_capacity(SIZES.len());
    let mut walk_per: Vec<u128> = Vec::with_capacity(SIZES.len());
    for n in SIZES {
        let bars = minutes(n);
        let derived = derive_minutes(&bars, five, Venue::NseCash, None, Runtime::default())
            .unwrap_or_else(|_| refuse("the fold refused its fixture"));
        if derived.bars.is_empty() {
            refuse("the fold produced no bucket");
        }
        let counted = pull::padded::per_day(&bars, Venue::NseCash, None, Runtime::default())
            .unwrap_or_else(|_| refuse("the walk refused its fixture"));
        if counted.iter().all(|day| day.padded == 0) {
            refuse("the fixture holds no padded minute");
        }
        spread(
            &format!("C-PD-01 cash fold to 5min, {n} minutes"),
            200,
            || {
                black_box(
                    derive_minutes(
                        black_box(&bars),
                        five,
                        Venue::NseCash,
                        None,
                        Runtime::default(),
                    )
                    .map_or(0, |out| out.bars.len()),
                );
            },
        );
        spread(&format!("C-PD-02 padded count, {n} minutes"), 200, || {
            black_box(
                pull::padded::per_day(black_box(&bars), Venue::NseCash, None, Runtime::default())
                    .map_or(0, |out| out.len()),
            );
        });
        let n_u128 = u128::try_from(n).unwrap_or(1);
        fold_per.push(
            cost_ps(3, || {
                derive_minutes(
                    black_box(&bars),
                    five,
                    Venue::NseCash,
                    None,
                    Runtime::default(),
                )
                .map_or(0, |out| out.bars.len())
            }) / n_u128,
        );
        walk_per.push(
            cost_ps(3, || {
                pull::padded::per_day(black_box(&bars), Venue::NseCash, None, Runtime::default())
                    .map_or(0, |out| out.len())
            }) / n_u128,
        );
    }
    flat("C-PD-01 cash fold per minute", &fold_per)
        & flat("C-PD-02 padded count per minute", &walk_per)
}

/// The two ratios of one row's three per-minute costs, 1x to 10x and 1x to
/// 100x, each held to the ceiling.
fn flat(label: &str, per_minute: &[u128]) -> bool {
    let (Some(&base), Some(&mid), Some(&big)) =
        (per_minute.first(), per_minute.get(1), per_minute.get(2))
    else {
        refuse("three sizes measured");
    };
    ratio(&format!("{label}, 1k -> 10k"), base, mid)
        & ratio(&format!("{label}, 1k -> 100k"), base, big)
}
