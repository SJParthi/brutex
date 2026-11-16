//! CAL-1 / CAL-2 red first (SNIPER-ADANIENT-PLAN 11 as MAIN signed it, S1: re-derived on Zerodha 3-minute candles):
//! the ZERODHA ADANIENT 3-minute candle of 2026-04-01 09:15 must give O 1,825.00 / H 1,831.70 / L 1,803.80 /
//! C 1,831.50 exactly, RSI 87.34-87.36, CCI 654-656, %R -0.25, Supertrend 10 x 3 1,786.8-1,787.3, BB 20 x 3
//! 1,763.0 / 1,810.5 / 1,715.4 and EMA 200 (CONT, converged) 1,803.62. RED under `--features red` (the store's GDFL
//! 1-s bars: LTQ0-HL-1 present), GREEN only on the standard build. Reads the frozen store read-only.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing, clippy::panic, clippy::many_single_char_names, missing_docs)]

use std::path::Path;

use common_feat::cal::parse_ymd;
use feat_io::{calib_check, defaults, host, load_ctx, minute_stream, run};

#[test]
fn cal1_cal2_zerodha_3min_2026_04_01_0915() {
    let ctx = load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap();
    let store = Path::new(defaults::STORE);
    let (s3, _) = minute_stream(store, "ADANIENT", "3min", &ctx);
    let (s1, _) = minute_stream(store, "ADANIENT", "1min", &ctx);
    let lv = host::levels(&ctx, &host::day_facts(&s1, &ctx));
    let day = parse_ymd("2026-04-01").unwrap();
    let res = run::run(&ctx, &s3, &lv, &mut [], None, Some((day, 9 * 3600 + 15 * 60))).unwrap();
    let (r, _) = res.calib.expect("the calibration candle is in the stream");
    let rows = calib_check(&run::calib_values(&r));
    // CAL-1: the OHLC to the paisa (rows 0..4 are exact in every column).
    for x in rows.iter().take(4) {
        assert_eq!(x.ours, x.chart, "CAL-1 {}", x.name);
    }
    // CAL-2 re-derived on Zerodha (S1): every reading equal at 2 decimals to the independent node reference.
    let bad: Vec<_> = rows.iter().filter(|x| !x.ref_pass).collect();
    assert!(bad.is_empty(), "CAL-2 (re-derived) rows failing: {bad:?}");
}

/// NOT ZERODHA, NOT A GATE (check-build-feat-v1 F7; SNIPER s13 (5): only Zerodha's candles are data, the Dhan chart is a
/// test of the formulas). The computed task's ranges, written from DATA-SCOUT 10.2's GDFL-TICK figures. On Zerodha's candles four rows fall outside
/// them (RSI 87.33, BB upper 1,810.56, BB lower 1,715.53, EMA 200 1,803.60): a NAMED DATA DIFFERENCE (Zerodha's 3-minute
/// closes differ from GDFL's by a few paisa on most candles before the calibration candle; volumes are identical), the
/// independent node reference on the same candles gives the same four values. Kept RED, never tuned away; run with
/// `--ignored` (MAIN decides whether the task's ranges are re-derived on Zerodha too).
#[test]
#[ignore = "NOT ZERODHA, NOT A GATE: the computed task's GDFL-tick ranges on Zerodha candles (4 rows outside, data cause); a named difference only"]
fn cal2_task_ranges_from_gdfl_ticks_on_zerodha_candles() {
    let ctx = load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-07").unwrap()).unwrap();
    let store = Path::new(defaults::STORE);
    let (s3, _) = minute_stream(store, "ADANIENT", "3min", &ctx);
    let (s1, _) = minute_stream(store, "ADANIENT", "1min", &ctx);
    let lv = host::levels(&ctx, &host::day_facts(&s1, &ctx));
    let day = parse_ymd("2026-04-01").unwrap();
    let res = run::run(&ctx, &s3, &lv, &mut [], None, Some((day, 9 * 3600 + 15 * 60))).unwrap();
    let (r, _) = res.calib.unwrap();
    let bad: Vec<_> = calib_check(&run::calib_values(&r)).into_iter().filter(|x| !x.task_pass).collect();
    assert!(bad.is_empty(), "outside the task's ranges: {bad:?}");
}

#[test]
fn kite_rights_back_adjustment_is_undone_before_any_reading() {
    // 2025-11-14's Zerodha 1day close is Kite's re-based 2,440.00; undone it is NSE's raw 2,516.80 (no reading sees
    // the 2025-11-17 rights before 2025-11-17).
    let ctx = load_ctx(Path::new(defaults::DATES), Path::new(defaults::BHAV), parse_ymd("2018-09-03").unwrap()).unwrap();
    let store = Path::new(defaults::STORE);
    let (raw, _) = host::zerodha_stream(store, "ADANIENT", "1day", &ctx, false);
    let (und, _) = host::zerodha_stream(store, "ADANIENT", "1day", &ctx, true);
    let d = parse_ymd("2025-11-14").unwrap();
    let i = ctx.index_of(d).unwrap();
    let pick = |s: &host::Stream| s.candles.iter().find(|x| x.0 == i).map(|x| x.1.close);
    assert_eq!(pick(&raw), Some(244_000));
    assert_eq!(pick(&und), Some(251_680));
    let e = ctx.index_of(parse_ymd("2025-11-17").unwrap()).unwrap();
    let pe = |s: &host::Stream| s.candles.iter().find(|x| x.0 == e).map(|x| x.1.close);
    assert_eq!(pick(&raw).is_some(), pe(&raw) == pe(&und), "a candle on or after the ex-date is never changed");
}
