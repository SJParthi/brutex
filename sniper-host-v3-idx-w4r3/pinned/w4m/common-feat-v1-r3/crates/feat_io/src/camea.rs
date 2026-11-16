//! The corporate-action check of the Zerodha series (SNIPER-REQUIREMENTS 13 (4); REQ72-PLAN A.7 check (1) as signed
//! with amendment A1): does Kite re-base ADANIENT's history around each event, by how much, and is the undo exact?
//!
//! - **CA-BASIS per span** (between consecutive events): N = K x Z + D with N = NSE's official close (bhavcopy), Z =
//!   Zerodha's RAW 1day close. K = the MEDIAN of dN / dZ over the span's sessions (A1: never a fit across a step); D =
//!   the median of N - K Z. `g_Z` = the gcd of Zerodha's closes on the span.
//! - **The first-difference law (A1):** a step t is quiet when `|(N_t - N_{t-1}) - K (Z_t - Z_{t-1})| <= K g_Z +
//!   tol_ticks x tick_N`. Every non-quiet session is listed with its residual.
//! - **Kite's step at each event:** `K_after / K_before` against the event's f, and the EXACT test: every Zerodha close
//!   before the event equals NSE's close times Kite's factor rounded to Zerodha's tick (half up), counted.
//! - **Volumes:** Zerodha's 1day volume against the bhavcopy's traded quantity before and after each event.

use std::cmp::Ordering;
use std::fmt::Write as _;

use common_feat::cal::ymd;
use common_feat::fold::Candle;
use common_feat::num::mul_snap;

use crate::data::tick_on;
use crate::host::{Ctx, Stream};

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The median of a list of ratios `(num, den)` with `den > 0`, compared exactly.
fn median(mut v: Vec<(i128, i128)>) -> Option<(i128, i128)> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| (a.0 * b.1).cmp(&(b.0 * a.1)));
    v.get(v.len() / 2).copied()
}

/// Runs the check and returns the report text (TSV sections).
#[allow(clippy::too_many_lines, reason = "one report")]
#[must_use]
pub fn check(ctx: &Ctx, zday_raw: &Stream, tol_ticks: i64) -> String {
    let mut out = String::new();
    let mut z: std::collections::BTreeMap<i32, Candle> = std::collections::BTreeMap::new();
    for &(i, c) in &zday_raw.candles {
        if let Some(s) = ctx.sessions.get(i) {
            z.insert(s.day, c);
        }
    }
    // the joined days
    let days: Vec<(i32, i64, i64, u64, u64)> = ctx
        .bhav
        .iter()
        .filter_map(|(d, b)| z.get(d).map(|c| (*d, b.close, c.close, b.volume, c.volume)))
        .collect();
    let _ = writeln!(out, "# feat-run ca: Zerodha RAW 1day close Z against NSE's official close N (bhavcopy extract), {} joined sessions", days.len());
    let no_z: Vec<String> = ctx.bhav.keys().filter(|d| !z.contains_key(d)).map(|d| ymd(*d)).collect();
    let _ = writeln!(out, "bhavcopy sessions with no Zerodha 1day bar\t{}\t{}", no_z.len(), no_z.join(","));
    let mut cuts: Vec<i32> = ctx.ca.events().iter().map(|e| e.day).collect();
    cuts.sort_unstable();
    let span_of = |d: i32| cuts.iter().filter(|&&e| e <= d).count();
    let nspans = cuts.len() + 1;
    let _ = writeln!(out, "\n## CA-BASIS per span (N = K Z + D; K = median dN/dZ, D = median N - K Z; g_Z = gcd of Z)");
    let _ = writeln!(out, "span\tfirst\tlast\tsessions\tK\tK_decimal\tD_paisa\tg_Z\tequal_N_eq_Z\tquiet\tnon_quiet\tmax_resid_ticks");
    let mut ks: Vec<Option<(i128, i128)>> = vec![None; nspans];
    let mut listing = String::new();
    for sp in 0..nspans {
        let v: Vec<&(i32, i64, i64, u64, u64)> = days.iter().filter(|x| span_of(x.0) == sp).collect();
        if v.is_empty() {
            continue;
        }
        let mut ratios = Vec::new();
        let mut g = 0i64;
        for w in v.windows(2) {
            let (Some(a), Some(b)) = (w.first(), w.get(1)) else { continue };
            let dz = i128::from(b.2) - i128::from(a.2);
            let dn = i128::from(b.1) - i128::from(a.1);
            if dz != 0 {
                ratios.push(if dz < 0 { (-dn, -dz) } else { (dn, dz) });
            }
        }
        for x in &v {
            g = gcd(g, x.2);
        }
        let k = median(ratios).unwrap_or((1, 1));
        if let Some(slot) = ks.get_mut(sp) {
            *slot = Some(k);
        }
        let mut dres: Vec<(i128, i128)> = v.iter().map(|x| (i128::from(x.1) * k.1 - k.0 * i128::from(x.2), k.1)).collect();
        dres.sort_by(|a, b| (a.0 * b.1).cmp(&(b.0 * a.1)));
        let dmed = dres.get(dres.len() / 2).map_or(0, |x| x.0.div_euclid(x.1));
        let eq = v.iter().filter(|x| x.1 == x.2).count();
        let (mut quiet, mut loud, mut maxr) = (0u64, 0u64, 0i128);
        for w in v.windows(2) {
            let (Some(a), Some(b)) = (w.first(), w.get(1)) else { continue };
            let tick = i128::from(tick_on(&ctx.ticks, b.0));
            // |dN - K dZ| x k.den <= (K g_Z + tol tick) x k.den
            let r = ((i128::from(b.1) - i128::from(a.1)) * k.1 - k.0 * (i128::from(b.2) - i128::from(a.2))).abs();
            let lim = k.0 * i128::from(g) + i128::from(tol_ticks) * tick * k.1;
            let r_ticks_x100 = (100 * r).div_euclid(tick * k.1);
            maxr = maxr.max(r_ticks_x100);
            if r <= lim {
                quiet += 1;
            } else {
                loud += 1;
                let _ = writeln!(listing, "{sp}\t{}\t{}\t{}\t{}\t{}\t{}.{:02}", ymd(b.0), a.1, b.1, a.2, b.2, r_ticks_x100 / 100, r_ticks_x100 % 100);
            }
        }
        let kd = (k.0 * 1_000_000).div_euclid(k.1);
        let _ = writeln!(
            out,
            "{sp}\t{}\t{}\t{}\t{}/{}\t{}.{:06}\t{dmed}\t{g}\t{eq}\t{quiet}\t{loud}\t{}.{:02}",
            ymd(v.first().map_or(0, |x| x.0)),
            ymd(v.last().map_or(0, |x| x.0)),
            v.len(),
            k.0,
            k.1,
            kd / 1_000_000,
            kd % 1_000_000,
            maxr / 100,
            maxr % 100
        );
    }
    let _ = writeln!(out, "\n## Non-quiet first differences (|dN - K dZ| > K g_Z + {tol_ticks} NSE ticks): span, date, N_prev, N, Z_prev, Z, residual in NSE ticks");
    out.push_str(&listing);
    let _ = writeln!(out, "\n## Kite's step at each event, and the exact test of Kite's factor on every earlier session");
    let _ = writeln!(out, "event\tkind\tf\tK_before\tK_after\tK_after/K_before\tkite_row\tsessions_before\tZ_eq_round_half_up(N x kite, 5 paise)\tZ_eq_round_half_up(N x kite, Zerodha tick g_Z)\tZ_eq_N");
    for (ei, e) in ctx.ca.events().iter().enumerate() {
        let kb = ks.get(ei).copied().flatten();
        let ka = ks.get(ei + 1).copied().flatten();
        let step = match (kb, ka) {
            (Some(b), Some(a)) => format!("{}/{} ({})", a.0 * b.1, a.1 * b.0, {
                let x = (a.0 * b.1 * 1_000_000).div_euclid(a.1 * b.0);
                format!("{}.{:06}", x / 1_000_000, x % 1_000_000)
            }),
            _ => "NA".into(),
        };
        let before: Vec<&(i32, i64, i64, u64, u64)> = days.iter().filter(|x| x.0 < e.day).collect();
        let (mut eq5, mut eqg, mut eqn) = (0u64, 0u64, 0u64);
        let mut misses = Vec::new();
        if let Some(k) = ctx.ca.kite_undo(e.day - 1) {
            let g = before.iter().fold(0i64, |g, x| gcd(g, x.2)).max(1);
            for x in &before {
                if mul_snap(x.1, k.0, k.1, 5) == x.2 {
                    eq5 += 1;
                }
                let want = mul_snap(x.1, k.0, k.1, g);
                if want == x.2 {
                    eqg += 1;
                } else {
                    misses.push(format!("{} N {} Z {} round(N x {}/{}, {g}) {want}", ymd(x.0), x.1, x.2, k.0, k.1));
                }
            }
        }
        for x in &before {
            if x.1 == x.2 {
                eqn += 1;
            }
        }
        let _ = writeln!(
            out,
            "{}\t{:?}\t{}/{}\t{}\t{}\t{step}\t{}\t{}\t{eq5}\t{eqg}\t{eqn}",
            ymd(e.day),
            e.kind,
            e.f.0,
            e.f.1,
            kb.map_or("NA".into(), |k| format!("{}/{}", k.0, k.1)),
            ka.map_or("NA".into(), |k| format!("{}/{}", k.0, k.1)),
            e.kite.map_or("NOT_REBASED".into(), |k| format!("{}/{}", k.0, k.1)),
            before.len()
        );
        for m in &misses {
            let _ = writeln!(out, "#   not equal: {m}");
        }
    }
    let _ = writeln!(out, "\n## Volumes: Zerodha 1day volume V_Z against the bhavcopy's traded quantity V_N, per span");
    let _ = writeln!(out, "span\tsessions\tV_Z_eq_V_N\tmedian V_Z/V_N x 10^6");
    for sp in 0..nspans {
        let v: Vec<&(i32, i64, i64, u64, u64)> = days.iter().filter(|x| span_of(x.0) == sp && x.3 > 0).collect();
        let eq = v.iter().filter(|x| x.3 == x.4).count();
        let mut r: Vec<(i128, i128)> = v.iter().map(|x| (i128::from(x.4), i128::from(x.3))).collect();
        r.sort_by(|a, b| match (a.0 * b.1).cmp(&(b.0 * a.1)) {
            Ordering::Less => Ordering::Less,
            o => o,
        });
        let m = r.get(r.len() / 2).map_or(0, |x| (x.0 * 1_000_000).div_euclid(x.1.max(1)));
        let _ = writeln!(out, "{sp}\t{}\t{eq}\t{m}", v.len());
    }
    out
}
