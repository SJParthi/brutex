//! `feat_io`: the host side of `common_feat` v1 for the ADANIENT sniper (SNIPER-REQUIREMENTS section 13; MAIN-SIGNED-
//! SNIPER-ADANIENT S1). Read-only on every input; it writes only into the run folder it is given.
//!
//! - MINUTE families read the store's ZERODHA candles ([`zstore`], [`host::zerodha_stream`]); the 4-minute family is
//!   folded from Zerodha's 1-minute candles ([`host::fold_stream`]). Kite's back-adjustment is undone first ([`camea`]
//!   measures it, `common_feat::ca` undoes it), so no reading sees a future event.
//! - The GDFL tick store ([`ticks`], LTQ = 0 new-price rows KEPT) feeds only the seconds families, the fills and the
//!   reconciliation ([`recon`]).
//! - [`run`] writes the RAW readings (3-MINUTE FIRST), the calibration table and the step-count gate.

#![allow(
    clippy::many_single_char_names,
    clippy::similar_names,
    reason = "host glue: o / h / l / c, the join cursors a / b and the CA symbols N, Z, K, D follow the plan's notation"
)]

pub mod camea;
pub mod crc;
pub mod data;
pub mod dials;
pub mod host;
pub mod idxdials;
pub mod idxofficial;
pub mod instr;
pub mod recon;
pub mod run;
pub mod ticks;
pub mod zstore;

use std::path::{Path, PathBuf};

/// The default inputs (every one overridable on the command line).
pub mod defaults {
    /// The frozen store.
    pub const STORE: &str = "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/shadow/fib-frozen-2";
    /// The GDFL tick store.
    pub const TICKS: &str = "/Volumes/WD_BLACK/brutex/tickstore-data";
    /// The 2,021 session dates (the tick store's NSE calendar).
    pub const DATES: &str = "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/sniper-req76-20261113/scout/nse-sessions-tickstore.txt";
    /// The bhavcopy extract.
    pub const BHAV: &str = "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/sniper-req76-20261113/scout/bhav-cm-adanient.tsv";
}

/// The crate's data folder (`common-feat-v1/data`).
#[must_use]
pub fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("data")
}

/// Loads the context from the given inputs (`cont_start` = the first session any candle reading reads).
///
/// # Errors
///
/// Any malformed input.
pub fn load_ctx(dates: &Path, bhav: &Path, cont_start: i32) -> Result<host::Ctx, String> {
    let d = data_dir();
    Ok(host::Ctx {
        sessions: data::sessions(dates, &d.join("SESSIONS-SPECIAL-v1.tsv"))?,
        ca: data::ca(&d.join("CA-ADANIENT-v1.tsv"))?,
        ticks: data::ticks(&d.join("TICK-ADANIENT-v1.tsv"))?,
        bhav: data::bhav(bhav)?,
        cont_start,
    })
}

/// The minute source of this build: Zerodha (standard) or, under `--features red`, the store's GDFL 1-s bars.
#[must_use]
pub fn minute_stream(store: &Path, symbol: &str, tfname: &str, ctx: &host::Ctx) -> (host::Stream, host::LoadStats) {
    let tf_s: i64 = match tfname {
        "1min" => 60,
        "2min" => 120,
        "3min" => 180,
        "4min" => 240,
        "5min" => 300,
        "10min" => 600,
        "15min" => 900,
        "30min" => 1800,
        "60min" => 3600,
        _ => 0,
    };
    if cfg!(feature = "red") {
        return host::gdfl_store_stream(store, symbol, tf_s, ctx);
    }
    if tfname == "4min" {
        let (one, st) = host::zerodha_stream(store, symbol, "1min", ctx, true);
        return (host::fold_stream(&one, 240, ctx, "4min"), st);
    }
    host::zerodha_stream(store, symbol, tfname, ctx, true)
}

/// S4 (r3, wave 4): [`minute_stream`] for any read row ([`host::ZRead`]: segment, symbol, span and the Muhurat rule as data; the
/// 4-minute family folded from the 1-minute candles of the same read). The CASH row of a symbol equals [`minute_stream`].
#[must_use]
pub fn minute_stream_read(store: &Path, r: &host::ZRead<'_>, tfname: &str, ctx: &host::Ctx) -> (host::Stream, host::LoadStats) {
    if *r == host::ZRead::cash(r.symbol) {
        return minute_stream(store, r.symbol, tfname, ctx);
    }
    if tfname == "4min" {
        let (one, st) = host::zerodha_stream_read(store, r, "1min", ctx, true);
        return (host::fold_stream(&one, 240, ctx, "4min"), st);
    }
    host::zerodha_stream_read(store, r, tfname, ctx, true)
}

/// CAL-2 RE-DERIVED on Zerodha 3-minute candles (MAIN-SIGNED-SNIPER S1): the independent node reference's values for the
/// 2026-04-01 09:15 candle (`check/cal2-zerodha-ref.mjs`, sha256 16 `177416ad9c35c2a7`: the scout's `ind.mjs` floats on the
/// store's Zerodha candles, its own reader; output `check/CAL2-ZERODHA-REF.txt`). Order of [`run::CHART`].
pub const CAL2_ZERODHA_REF: [&str; 12] = [
    "1825.00", "1831.70", "1803.80", "1831.50", "87.325920", "654.442620", "-0.254130", "1787.136117", "1763.045000",
    "1810.564136", "1715.525864", "1803.603812",
];

/// One calibration row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalRow {
    /// The value's name.
    pub name: String,
    /// The operator's chart (a TEST of the formulas, never data).
    pub chart: String,
    /// The module's value (two decimals).
    pub ours: String,
    /// The independent node reference on the same Zerodha candles, rounded to two decimals.
    pub zref: String,
    /// CAL-2 (re-derived): the module equals the reference at two decimals.
    pub ref_pass: bool,
    /// The range the computed task wrote (from the GDFL-tick figures of DATA-SCOUT 10.2): NOT Zerodha, NOT a gate
    /// (SNIPER s13 (5): only Zerodha's candles are data; check-build-feat-v1 F7). Printed beside, labelled so.
    pub task_range: String,
    /// The module's value inside the task's range (NOT a gate).
    pub task_pass: bool,
}

fn round2(s: &str) -> Option<i64> {
    // a decimal string to hundredths, half away from zero
    let s = s.trim();
    let neg = s.starts_with('-');
    let t = s.trim_start_matches('-');
    let (i, f) = t.split_once('.').unwrap_or((t, ""));
    let mut digits: Vec<u8> = f.bytes().collect();
    while digits.len() < 3 {
        digits.push(b'0');
    }
    let int: i64 = i.parse().ok()?;
    let d = |k: usize| digits.get(k).map_or(0, |b| i64::from(b.saturating_sub(b'0')));
    let mut v = int * 100 + d(0) * 10 + d(1);
    if d(2) >= 5 {
        v += 1;
    }
    Some(if neg { -v } else { v })
}

/// The calibration rows of the module's values `ours` (order of [`run::CHART`]).
#[must_use]
pub fn calib_check(ours: &[String; 12]) -> Vec<CalRow> {
    let req: [(&str, &str, &str); 12] = [
        ("open", "1825.00", "1825.00"),
        ("high", "1831.70", "1831.70"),
        ("low", "1803.80", "1803.80"),
        ("close", "1831.50", "1831.50"),
        ("RSI 14", "87.34", "87.36"),
        ("CCI 20", "654.00", "656.00"),
        ("%R 14", "-0.25", "-0.25"),
        ("Supertrend 10 x 3", "1786.80", "1787.30"),
        ("BB 20 x 3 basis", "1762.95", "1763.05"),
        ("BB 20 x 3 upper", "1810.45", "1810.55"),
        ("BB 20 x 3 lower", "1715.35", "1715.45"),
        ("EMA 200 (CONT, converged)", "1803.62", "1803.62"),
    ];
    let mut v = Vec::new();
    for (k, (name, lo, hi)) in req.iter().enumerate() {
        let o = ours.get(k).cloned().unwrap_or_default();
        let chart = run::CHART.get(k).map_or("", |x| x.1).to_owned();
        let zr = CAL2_ZERODHA_REF.get(k).copied().unwrap_or("NA");
        let zr2 = round2(zr);
        let task_pass = match (round2(&o), round2(lo), round2(hi)) {
            (Some(x), Some(a), Some(b)) => x >= a && x <= b,
            _ => false,
        };
        let range = if lo == hi { format!("= {lo}") } else { format!("{lo} .. {hi}") };
        v.push(CalRow {
            name: (*name).to_owned(),
            chart,
            zref: zr2.map_or_else(|| "NA".into(), common_feat::num::rupees),
            ref_pass: zr2.is_some() && round2(&o) == zr2,
            ours: o,
            task_range: range,
            task_pass,
        });
    }
    v
}
