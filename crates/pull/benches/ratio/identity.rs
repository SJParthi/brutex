//! C-ZI-01 and C-ZI-02 (R4-17): the listing probe at 1x and 10x a master's
//! instruments, and the identity judge per minute at 1x, 10x and 100x an
//! answer's minutes.
//!
//! C-ZI-01 is `pull::listing::Listing::span`, the one probe a CASH landing
//! makes per answer: one hash probe into the master's index, which must cost
//! the same with ten times the master's rows. C-ZI-02 is
//! `pull::series_identity::MinuteJudge::judge`, what the landing door runs per
//! minute (a run step, the day arm's compares, one binary search over the day
//! bars of the months the answer touches, the jump compare): its cost per
//! minute must not grow with the answer. p50, p99 and max of single calls are
//! printed beside each ratio.
//!
//! Every input is invented here; the master is written into a new folder under
//! the system temp directory, printed, and left there (this harness removes no
//! file, R4-08's no-delete rule).

use std::fmt::Write as _;
use std::hint::black_box;
use std::io::Write as _;

use pull::listing::{Listing, MASTER_DIR};
use pull::series_identity::{DayBook, FirstDay, MinuteJudge, Rules};
use pull::session::Day;
use store::format::Bar;

use super::{cost_ps, ratio, refuse, spread};

/// Rows of the 1x master; the 10x master holds ten times as many.
const BASE_ROWS: u32 = 300;
/// Minutes of the 1x answer, one session; 10x and 100x are ten and a hundred
/// sessions.
const SESSION: i64 = 375;

/// A receipted master of `rows` invented EQ rows under a new folder.
fn master(rows: u32) -> Listing {
    let base = std::env::temp_dir().join(format!(
        "brutex-zi-bench-{}-{rows}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos())
    ));
    println!("  C-ZI scratch (left for clean-up): {}", base.display());
    let mut csv = String::from(
        "FinInstrmId,TckrSymb,SctySrs,ISIN,ElgbltyClsgAuctnSsn,ListgDt,RmvlDt,RadmssnDt\n",
    );
    for i in 0..rows {
        let _infallible = writeln!(
            csv,
            "{},ZB{i:05},EQ,INE{i:09},1,{},0,0",
            10_000 + i,
            1_296_345_600 + 86_400 * (i % 900)
        );
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    if encoder.write_all(csv.as_bytes()).is_err() {
        refuse("the invented master compresses");
    }
    let Ok(bytes) = encoder.finish() else {
        refuse("the invented master compresses")
    };
    let masters = base.join(MASTER_DIR);
    let Ok(on) = Day::new(2026, 8, 3) else {
        refuse("a real day")
    };
    if std::fs::create_dir_all(&masters).is_err()
        || pull::cash_session_cache::install(&masters, on, &bytes).is_err()
    {
        refuse("the invented master installs");
    }
    let listing = Listing::load(&masters);
    if listing.master().is_none() {
        refuse("the invented master loads");
    }
    listing
}

/// C-ZI-01: the listing probe at 1x and 10x rows.
fn listing_probe_is_flat() -> bool {
    let mut probes = Vec::new();
    for rows in [BASE_ROWS, BASE_ROWS * 10] {
        let listing = master(rows);
        let (symbol, isin) = (format!("ZB{:05}", rows / 2), format!("INE{:09}", rows / 2));
        if listing.span("NSE", &symbol, Some(&isin)).is_none() {
            refuse("the probed row is in the master");
        }
        probes.push(cost_ps(200_000, || {
            black_box(&listing)
                .span("NSE", black_box(&symbol), Some(black_box(&isin)))
                .is_some()
        }));
        spread(
            &format!("C-ZI-01 listing probe, {rows} rows"),
            20_000,
            || {
                black_box(listing.span("NSE", black_box(&symbol), Some(black_box(&isin))));
            },
        );
    }
    let [one, ten] = probes.as_slice() else {
        refuse("two sizes measured")
    };
    ratio("C-ZI-01 listing probe, 10x master", *one, *ten)
}

/// An answer of `sessions` sessions of minutes and the day bars it touches.
fn answer(sessions: i64) -> (Vec<Bar>, Vec<Bar>) {
    let first = 19_723_i64; // 2024-01-01, IST days since the epoch.
    let (mut days, mut minutes) = (Vec::new(), Vec::new());
    for s in 0..sessions {
        let midnight = (first + s) * 86_400 - 19_800;
        days.push(Bar {
            ts_micros: midnight * 1_000_000,
            open: 1_000_000,
            high: 1_002_000,
            low: 999_000,
            close: 1_000_500,
            volume: 0,
            open_interest: i64::MIN,
        });
        for m in 0..SESSION {
            let price = 1_000_000 + (m % 97) * 13;
            minutes.push(Bar {
                ts_micros: (midnight + 33_300 + m * 60) * 1_000_000,
                open: price,
                high: price + 50,
                low: price - 40,
                close: price + 10,
                volume: 0,
                open_interest: i64::MIN,
            });
        }
    }
    (days, minutes)
}

/// C-ZI-02: the identity judge per minute at 1x, 10x and 100x an answer.
fn minute_judge_is_flat() -> bool {
    let mut per_minute = Vec::new();
    for sessions in [1_i64, 10, 100] {
        let (days, minutes) = answer(sessions);
        let book = DayBook::new(&days, FirstDay::NoDayRung);
        let rules = Rules {
            listed_on: Some(19_000),
            jumps: true,
        };
        let count = u32::try_from(minutes.len()).unwrap_or_else(|_| refuse("a small answer"));
        let whole = cost_ps(20, || {
            let mut judge = MinuteJudge::new(&book, rules, 60);
            for bar in &minutes {
                let day = pull::series_identity::ist_day(bar.ts_micros);
                black_box(judge.judge(
                    bar.ts_micros / 1_000_000,
                    day,
                    pull::series_identity::prices(bar),
                ));
            }
        });
        per_minute.push(whole / u128::from(count));
        if sessions == 100 {
            let mut judge = MinuteJudge::new(&book, rules, 60);
            let mut at = 0usize;
            spread("C-ZI-02 judge one minute, 100x answer", 20_000, || {
                let bar = minutes.get(at % minutes.len()).copied().unwrap_or_default();
                at += 1;
                black_box(judge.judge(
                    bar.ts_micros / 1_000_000,
                    pull::series_identity::ist_day(bar.ts_micros),
                    pull::series_identity::prices(&bar),
                ));
            });
        }
    }
    let [one, ten, hundred] = per_minute.as_slice() else {
        refuse("three sizes measured")
    };
    ratio("C-ZI-02 judge per minute, 10x answer", *one, *ten)
        & ratio("C-ZI-02 judge per minute, 100x answer", *one, *hundred)
}

/// Gate 8's R4-17 rows.
pub(crate) fn identity_is_flat() -> bool {
    listing_probe_is_flat() & minute_judge_is_flat()
}
