//! Measurements of the causal admission rule over a READ-ONLY store, for
//! `docs/06-limits.md` and `docs/05-decisions.md` (lookahead-oos-1). Ignored:
//! they need a store and print figures; they assert only that they measured.
//!
//! Run against a private shadow whose `bars` is the real store's link:
//! `AH01_STORE=<shadow root> cargo test -p cli --locked --lib
//! minute_gaps::measure_tests -- --ignored --nocapture --test-threads=1`.
//! Nothing is written: every read is `stored::load`.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]

use super::*;
use std::time::Instant;

fn store() -> Option<std::path::PathBuf> {
    std::env::var_os("AH01_STORE").map(std::path::PathBuf::from)
}

fn months() -> impl Iterator<Item = (u16, u8)> {
    (2018_u16..=2026)
        .flat_map(|year| (1_u8..=12).map(move |month| (year, month)))
        .filter(|(year, month)| (*year, *month) <= (2026, 9))
}

/// Policy 3, kept here only as the BEFORE of the timing, statement for
/// statement as `feat/db-fix` ran it: every day with an interior hole or an
/// absent demanded closing minute (one forward cursor, a last-day check, one
/// ascending merge), then withheld whole through a day set into a fresh
/// vector. Returns the kept bars.
fn whole_day_rule(signal: &[Candle], minutes: &[Candle], length: i64) -> Vec<Candle> {
    let set = whole_day_set(signal, minutes, length);
    if set.is_empty() {
        return signal.to_vec();
    }
    let mut kept: Vec<Candle> = Vec::with_capacity(signal.len());
    for bar in signal {
        if !set.contains(&indicators::ist_day(bar.ts_micros)) {
            kept.push(*bar);
        }
    }
    kept
}

/// Policy 3's interior walk, statement for statement: a day is holed when two
/// ADJACENT bars of the same IST day are more than a minute apart. It never
/// looked at the calendar, so a day missing its 09:15 minute was not holed and
/// a two-window session was holed between its windows; the BEFORE must be
/// that rule, not policy 4's [`days_with_interior_gaps`].
fn policy_3_interior_days(minutes: &[Candle]) -> Vec<i64> {
    let mut days: Vec<i64> = Vec::new();
    for pair in minutes.windows(2) {
        let (before, bar) = (pair[0], pair[1]);
        let day = indicators::ist_day(bar.ts_micros);
        if indicators::ist_day(before.ts_micros) == day
            && bar.ts_micros - before.ts_micros > MINUTE_MICROS
            && days.last() != Some(&day)
        {
            days.push(day);
        }
    }
    days
}

/// The days policy 3 withheld whole: an interior hole or an absent demanded
/// closing minute. A recorded halt was NOT one of them, so a halt day without
/// a hole kept every row under policy 3; the both-ways count below must not
/// charge policy 3 for rows policy 4 alone withholds.
fn whole_day_set(
    signal: &[Candle],
    minutes: &[Candle],
    length: i64,
) -> std::collections::HashSet<i64> {
    let interior = policy_3_interior_days(minutes);
    let mut edges: Vec<i64> = Vec::new();
    let mut cursor = 0_usize;
    for bar in signal {
        let day = indicators::ist_day(bar.ts_micros);
        let expected = indicators::anchored::exact_closing_minute(
            bar.ts_micros,
            length,
            crate::stored::nse_session_close_minute(day),
        );
        while minutes.get(cursor).is_some_and(|m| m.ts_micros < expected) {
            cursor += 1;
        }
        if minutes.get(cursor).is_none_or(|m| m.ts_micros != expected) && edges.last() != Some(&day)
        {
            edges.push(day);
        }
    }
    // The ascending merge `merge_ascending` did; a set needs no order, so the
    // union is the same days.
    interior.into_iter().chain(edges).collect()
}

fn quantiles(mut values: Vec<f64>) -> (usize, f64, f64, f64) {
    values.sort_by(f64::total_cmp);
    let at = |q: usize| values[(values.len() * q / 100).min(values.len() - 1)];
    (values.len(), at(50), at(99), values[values.len() - 1])
}

/// The admission cost per bar and per month, before (policy 3's whole-day
/// rule) and after (policy 4's cuts), over NIFTY and BANKNIFTY 1min and 15min
/// and twelve shares' 1min from 2018-01 to 2026-09: one load per month, then
/// the census and the withholding timed alone (the load is not timed).
#[test]
#[ignore = "a measurement over a read-only store named by AH01_STORE"]
fn admission_cost_per_bar_and_month() {
    let Some(root) = store() else {
        println!("AH01_STORE unset: nothing measured");
        return;
    };
    let mut selections: Vec<(&str, &str)> = vec![
        ("NIFTY", "1min"),
        ("NIFTY", "15min"),
        ("BANKNIFTY", "1min"),
        ("BANKNIFTY", "15min"),
    ];
    for share in [
        "IDEA",
        "RELIANCE",
        "SBIN",
        "TCS",
        "INFY",
        "HDFCBANK",
        "ICICIBANK",
        "TATASTEEL",
        "BANKBARODA",
        "SUZLON",
        "BANDHANBNK",
        "PNBHOUSING",
    ] {
        selections.push((share, "1min"));
    }
    let (mut before_bar, mut after_bar, mut before_month, mut after_month) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let (mut cut_months, mut rows_before, mut rows_after) = (0_u64, 0_u64, 0_u64);
    for (symbol, rung) in selections {
        let length = crate::stored::rung_length_micros(rung).expect("rung");
        for (year, month) in months() {
            let Ok(signal) = crate::stored::load(
                &root,
                brutex_core::vendor::Vendor::Zerodha,
                symbol,
                rung,
                year,
                month,
            ) else {
                continue;
            };
            let minutes = if rung == "1min" {
                signal.bars.clone()
            } else {
                match crate::stored::load(
                    &root,
                    brutex_core::vendor::Vendor::Zerodha,
                    symbol,
                    "1min",
                    year,
                    month,
                ) {
                    Ok(minutes) => minutes.bars,
                    Err(_) => continue,
                }
            };
            if signal.bars.is_empty() {
                continue;
            }
            let bars = signal.bars.len() as f64;
            let start = Instant::now();
            let kept_before = whole_day_rule(&signal.bars, &minutes, length);
            let spent_before = start.elapsed().as_nanos() as f64;
            let start = Instant::now();
            let cuts = days_with_minute_holes(&signal.bars, &minutes, length, |day| {
                crate::stored::nse_session_close_minute(day)
            });
            let (kept, _) = withhold(&signal.bars, &cuts);
            let spent_after = start.elapsed().as_nanos() as f64;
            before_bar.push(spent_before / bars);
            after_bar.push(spent_after / bars);
            before_month.push(spent_before / 1_000.0);
            after_month.push(spent_after / 1_000.0);
            if !cuts.is_empty() {
                cut_months += 1;
            }
            rows_before += kept_before.len() as u64;
            rows_after += kept.len() as u64;
        }
    }
    let (n, p50, p99, max) = quantiles(before_bar);
    println!("BEFORE per bar (ns): n={n} p50={p50:.1} p99={p99:.1} max={max:.1}");
    let (n, p50, p99, max) = quantiles(after_bar);
    println!("AFTER  per bar (ns): n={n} p50={p50:.1} p99={p99:.1} max={max:.1}");
    let (n, p50, p99, max) = quantiles(before_month);
    println!("BEFORE per month (us): n={n} p50={p50:.1} p99={p99:.1} max={max:.1}");
    let (n, p50, p99, max) = quantiles(after_month);
    println!("AFTER  per month (us): n={n} p50={p50:.1} p99={p99:.1} max={max:.1}");
    println!(
        "months with a cut: {cut_months}; rows admitted before {rows_before}, after {rows_after}"
    );
    assert!(n >= 200, "at least 200 months measured, got {n}");
}

/// MEASURE BOTH WAYS (operator rule): over the F&O shares' Zerodha 1min from
/// 2018-01, the rows admitted under the whole-day rule and under the causal
/// cut, and the >=5% / >=10% move rates of the recovered pre-hole windows
/// (first open to the last admitted close) against the kept days' open to
/// close.
#[test]
#[ignore = "a measurement over a read-only store named by AH01_STORE"]
fn rows_and_move_rates_both_ways() {
    let Some(root) = store() else {
        println!("AH01_STORE unset: nothing measured");
        return;
    };
    let (mut symbols, mut months_read) = (0_u64, 0_u64);
    let (mut rows_old, mut rows_new) = (0_u64, 0_u64);
    let (mut kept_days, mut kept5, mut kept10) = (0_u64, 0_u64, 0_u64);
    let (mut windows, mut win5, mut win10) = (0_u64, 0_u64, 0_u64);
    let (mut holed_days, mut empty_windows) = (0_u64, 0_u64);
    let (mut lead_days, mut lead_outside, mut lead_admitted) = (0_u64, 0_u64, 0_u64);
    // Real share-days to re-check on a door: missing their open minute, inside
    // the calendar's range, and not the month's first session (whose prior
    // session would be the previous month's).
    let mut examples: Vec<String> = Vec::new();
    for symbol in brutex_core::universe::FNO_UNDERLYINGS {
        let mut any = false;
        for (year, month) in months() {
            let Ok(loaded) = crate::stored::load(
                &root,
                brutex_core::vendor::Vendor::Zerodha,
                symbol,
                "1min",
                year,
                month,
            ) else {
                continue;
            };
            any = true;
            months_read += 1;
            let bars = &loaded.bars;
            let cuts = days_with_minute_holes(bars, bars, MINUTE_MICROS, |day| {
                crate::stored::nse_session_close_minute(day)
            });
            let admission = Admission::of(&cuts);
            let cut_days: std::collections::HashSet<i64> = cuts.iter().map(|cut| cut.day).collect();
            let old_days = whole_day_set(bars, bars, MINUTE_MICROS);
            // EVERY DAY POLICY 3 WITHHELD IS CUT UNDER POLICY 4, except a
            // two-window session that policy 3 holed between its windows: the
            // calendar expects no minute there, so policy 4 does not.
            assert!(
                old_days.iter().all(|day| cut_days.contains(day)
                    || matches!(
                        pull::calendar::kind_of(*day),
                        pull::calendar::DayKind::Open(session) if session.count == 2
                    )),
                "{symbol} {year}-{month:02}: every day policy 3 withheld is cut under policy 4"
            );
            let mut index = 0_usize;
            while index < bars.len() {
                let day = indicators::ist_day(bars[index].ts_micros);
                let end = bars[index..]
                    .iter()
                    .position(|bar| indicators::ist_day(bar.ts_micros) != day)
                    .map_or(bars.len(), |offset| index + offset);
                let session = &bars[index..end];
                let open = session[0].open;
                // THE LEADING HOLE the first re-check found: the day's first held
                // minute later than its session's first minute.
                // A day outside the calendar's range is held to 09:15 (555),
                // written here as a number rather than read from the code under test.
                let first_expected = match pull::calendar::kind_of(day) {
                    pull::calendar::DayKind::Open(calendar) => {
                        Some(i64::from(calendar.windows[0].from))
                    }
                    pull::calendar::DayKind::Unmeasured => Some(555),
                    _ => None,
                };
                let leading =
                    first_expected.is_some_and(|first| minute_of_day(session[0].ts_micros) > first);
                lead_days += u64::from(leading);
                lead_outside += u64::from(
                    leading && pull::calendar::kind_of(day) == pull::calendar::DayKind::Unmeasured,
                );
                if leading
                    && index > 0
                    && examples.len() < 12
                    && matches!(
                        pull::calendar::kind_of(day),
                        pull::calendar::DayKind::Open(_)
                    )
                {
                    examples.push(format!(
                        "{symbol} {year}-{month:02} IST day {day} first minute {}",
                        minute_of_day(session[0].ts_micros)
                    ));
                }
                if leading && !cut_days.contains(&day) {
                    lead_admitted += session.len() as u64;
                }
                if cut_days.contains(&day) {
                    holed_days += 1;
                    if !old_days.contains(&day) {
                        // A day policy 4 cuts and policy 3 kept whole: a
                        // recorded halt with no hole in the minutes, or a day
                        // missing its open minute (policy 3 never saw a
                        // leading hole; AH-01 repair round 1).
                        rows_old += session.len() as u64;
                    }
                    let admitted: Vec<&Candle> = session
                        .iter()
                        .filter(|bar| admission.admits(bar.ts_micros))
                        .collect();
                    rows_new += admitted.len() as u64;
                    if leading {
                        lead_admitted += admitted.len() as u64;
                    }
                    if let Some(last) = admitted.last() {
                        windows += 1;
                        if open > 0 {
                            let moved = (last.close - open).unsigned_abs() as f64 / open as f64;
                            win5 += u64::from(moved >= 0.05);
                            win10 += u64::from(moved >= 0.10);
                        }
                    } else {
                        empty_windows += 1;
                    }
                } else {
                    kept_days += 1;
                    if !old_days.contains(&day) {
                        rows_old += session.len() as u64;
                    }
                    rows_new += session.len() as u64;
                    if open > 0 {
                        let close = session[session.len() - 1].close;
                        let moved = (close - open).unsigned_abs() as f64 / open as f64;
                        kept5 += u64::from(moved >= 0.05);
                        kept10 += u64::from(moved >= 0.10);
                    }
                }
                index = end;
            }
        }
        symbols += u64::from(any);
    }
    let rate = |n: u64, d: u64| {
        if d == 0 {
            0.0
        } else {
            100.0 * n as f64 / d as f64
        }
    };
    println!("shares {symbols}, months {months_read}");
    println!(
        "rows admitted: whole-day rule {rows_old}, causal cut {rows_new} ({:+})",
        i128::from(rows_new) - i128::from(rows_old)
    );
    println!(
        "holed days {holed_days}: pre-hole windows {windows}, empty (hole right after the open or a halt before any row) {empty_windows}"
    );
    println!(
        "kept days {kept_days}: >=5% {:.3}% >=10% {:.3}%",
        rate(kept5, kept_days),
        rate(kept10, kept_days)
    );
    println!(
        "recovered pre-hole windows {windows}: >=5% {:.3}% >=10% {:.3}%",
        rate(win5, windows),
        rate(win10, windows)
    );
    println!(
        "share-days missing their open minute {lead_days} ({lead_outside} outside the calendar's range): rows admitted on them {lead_admitted}"
    );
    println!("examples: {}", examples.join("; "));
    assert_eq!(
        lead_admitted, 0,
        "a day missing its open minute admits no row"
    );
    assert!(months_read > 0, "measured nothing");
}
