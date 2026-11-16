//! v3.2 (MAIN-RULINGS-ENGINE-v22 addenda 8 / 8b): IV and the five Greeks of every trade at its entry decision and at its exit decision,
//! through the app's OWN Greeks code (`wt/APP/crates/greeks`, the tree the :8080 app is built from: bsm.rs + solver.rs, linked UNMODIFIED as a path dependency: no copy, no new
//! formula), with exactly the conversions of the app's `pull::pricing::bar_greeks` (the code behind /greeks.json), as the v3.1 runner's
//! proven `greeks-tsv` tool reads them (its PROOF-GREEKS-8B: 164 / 164 rows equal to the live /greeks.json, f64 bit for bit):
//! - Contract { spot, strike (paisa as f64), years = tenor seconds / (365 x 86,400), rate = the continuously compounded 91-day T-bill
//!   (pull::riskfree::continuous: ln_1p(y x 91/365) / (91/365)), carry = 0 }; iv = implied_volatility(premium, kind); g = greeks(iv, kind);
//! - gamma per rupee = gamma x 100; theta Rs a day = theta / 100 / 365; vega Rs per vol point = vega / 100 / 100; rho Rs per rate point =
//!   rho / 100 / 100; a position figure = the per-unit figure x signed units (a SOLD option = negative units);
//! - the inputs "as /db shows them" (8b): the 1-minute bar holding the last second before the decision: premium = its close, spot = the
//!   underlying's close of the same bar, tenor = seconds from the bar's STAMP (its start) to the expiry day's derivatives close (15:30;
//!   15:40 from 2026-08-03); the T-bill = the latest RBI release strictly before the trade day (the app's 14-day staleness flag kept).
//! Dividends are not modelled (labelled). O(1) per trade: the solver's evaluation bound is the app's (75 model evaluations).

use std::path::Path;
use std::sync::OnceLock;

use greeks::{Contract, OptionKind};

use crate::cal::{days_from_civil, parse_day};

/// The RBI 91-day T-bill releases: (release day, implicit yield x 10,000 (e4 of a percent... the app's `yield_e4`: 5.5199% -> 55199)).
#[derive(Clone, Debug, Default)]
pub struct TBill {
    pub rows: Vec<(i64, u32)>,
    pub source: String,
}

/// The run's table.
pub static TBILL: OnceLock<TBill> = OnceLock::new();

impl TBill {
    /// Reads `state/rates/rbi-91d-tbill.tsv` (auction_date, settlement_date, released, cut_off_price, implicit_yield_pct, ...).
    ///
    /// # Errors
    /// An unreadable file or no release row.
    pub fn load(p: &Path) -> Result<Self, String> {
        let t = std::fs::read_to_string(p).map_err(|e| format!("--tbill {}: {e}", p.display()))?;
        let mut rows = Vec::new();
        for l in t.lines().skip(1) {
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(d), Some(y)) = (f.get(2).and_then(|x| parse_day(x)), f.get(4).and_then(|x| crate::bhav::paisa(x))) else { continue };
            // implicit_yield_pct has 4 decimals: 5.5199 -> 55199 (paisa parse gives 2 decimals; read the 4 directly)
            let ye4 = f.get(4).and_then(|x| ye4_of(x)).unwrap_or_else(|| u32::try_from(y * 100).unwrap_or(0));
            rows.push((d, ye4));
        }
        if rows.is_empty() {
            return Err(format!("--tbill {}: no release row", p.display()));
        }
        rows.sort_unstable();
        Ok(Self { rows, source: format!("{} (sha256 {}, {} releases)", p.display(), crate::cube::sha256(p), t.lines().count().saturating_sub(1)) })
    }

    /// The release in force on `day` (the latest strictly before it): (yield e4, release day, age in days, stale = older than 14 days).
    #[must_use]
    pub fn rate_for(&self, day: i64) -> Option<(u32, i64, i64, bool)> {
        let i = self.rows.partition_point(|x| x.0 < day);
        let (d, y) = *self.rows.get(i.checked_sub(1)?)?;
        Some((y, d, day - d, day - d > 14))
    }
}

/// "5.5199" -> 55199 (exactly 4 decimals or fewer).
#[must_use]
pub fn ye4_of(s: &str) -> Option<u32> {
    let s = s.trim();
    let (a, b) = s.split_once('.').unwrap_or((s, ""));
    if b.len() > 4 || a.is_empty() {
        return None;
    }
    let mut frac = b.to_owned();
    while frac.len() < 4 {
        frac.push('0');
    }
    format!("{a}{frac}").parse().ok()
}

/// pull::riskfree::continuous, verbatim (the runner's greeks-tsv).
#[must_use]
pub fn continuous(yield_e4: u32) -> f64 {
    let tau = 91.0 / 365.0;
    let simple = f64::from(yield_e4) / 1_000_000.0;
    // fix:determinism DET-2: ln_1p through det (the platform's unless --portable-math)
    // MUTANT m_det_rate_ln1p_platform: the rate keeps the platform's ln_1p under --portable-math
    if cfg!(feature = "m_det_rate_ln1p_platform") { (simple * tau).ln_1p() / tau } else { crate::det::ln_1p(simple * tau) / tau }
}

/// One instant's figures (the runner's greeks-tsv columns).
#[derive(Clone, Debug, Default)]
pub struct G {
    pub ok: bool,
    pub iv: f64,
    pub method: String,
    pub iterations: u32,
    pub uncertainty: f64,
    pub delta: f64,
    pub gamma_r: f64,
    pub theta_d: f64,
    pub vega_p: f64,
    pub rho_p: f64,
    pub pos_delta: f64,
    pub pos_gamma: f64,
    pub pos_theta: f64,
    pub pos_vega: f64,
    pub pos_rho: f64,
    pub below_band: bool,
    pub rate: f64,
    pub years: f64,
    pub refusal: String,
    pub ns: u64,
}

/// The app's bar_greeks conversions over the app's crates/greeks (see the module text). `units` signed (a sale negative).
#[must_use]
pub fn bar_greeks(spot: i64, strike: i64, premium: i64, tenor_s: i64, call: bool, yield_e4: u32, units: i64) -> G {
    let year_s: i64 = 365 * 86_400;
    let year_days = 365.0_f64;
    let rate = continuous(yield_e4);
    let years = tenor_s as f64 / year_s as f64;
    let below = tenor_s < year_s / 50;
    let mut g = G { rate, years, below_band: below, ..G::default() };
    if let Some((field, v)) = [("spot", spot), ("strike", strike), ("premium", premium)].into_iter().find(|x| x.1 <= 0) {
        g.refusal = format!("not positive: {field} = {v} paisa");
        return g;
    }
    let kind = if call { OptionKind::Call } else { OptionKind::Put };
    let c = Contract { spot: spot as f64, strike: strike as f64, years_to_expiry: years, rate, carry: 0.0 };
    let t0 = std::time::Instant::now();
    let r = c.implied_volatility(premium as f64, kind).and_then(|iv| c.greeks(iv.volatility, kind).map(|x| (iv, x)));
    g.ns = u64::try_from(t0.elapsed().as_nanos()).unwrap_or(u64::MAX);
    match r {
        Ok((iv, x)) => {
            let u = units as f64;
            g.ok = true;
            g.iv = iv.volatility;
            g.method = format!("{:?}", iv.method);
            g.iterations = iv.iterations;
            g.uncertainty = iv.uncertainty;
            g.delta = x.delta;
            g.gamma_r = x.gamma * 100.0;
            g.theta_d = x.theta / 100.0 / year_days;
            g.vega_p = x.vega / 100.0 / 100.0;
            g.rho_p = x.rho / 100.0 / 100.0;
            g.pos_delta = g.delta * u;
            g.pos_gamma = g.gamma_r * u;
            g.pos_theta = g.theta_d * u;
            g.pos_vega = g.vega_p * u;
            g.pos_rho = g.rho_p * u;
        }
        Err(e) => g.refusal = e.to_string().replace(['\t', '\n'], " "),
    }
    g
}

/// The expiry day's derivatives close (seconds of the day): 15:30, 15:40 from 2026-08-03 (the app's tenor rule).
#[must_use]
pub fn close_of_expiry(expiry_day: i64) -> i64 {
    if expiry_day >= days_from_civil(2026, 8, 3) { 15 * 3_600 + 40 * 60 } else { 15 * 3_600 + 30 * 60 }
}

/// The inputs "as /db shows them" at decision instant `inst` (seconds of the day) of contract `ci`: (bar slot, bar stamp, premium, spot,
/// tenor seconds); None before the first bar.
#[must_use]
pub fn inputs_at(c: &crate::cube::Cube, ci: usize, inst: i64) -> Option<(usize, i64, i64, i64, i64)> {
    let k = c.cons.get(ci)?;
    let m = usize::try_from((inst - 1 - crate::cal::OPEN_S).div_euclid(60)).ok()?;
    if m >= crate::cal::NM {
        return None;
    }
    let stamp = crate::cal::OPEN_S + 60 * i64::try_from(m).ok()?;
    let prem = i64::from(c.close_at(ci, m));
    let spot = i64::from(c.spot_at(usize::from(k.und), m));
    let ed = crate::cal::day_of_ymd(k.expiry);
    // MUTANT m_v32_greeks_tenor_bar_end: the tenor counted from the bar's END (not its stamp, the app's rule)
    let from = if cfg!(feature = "m_v32_greeks_tenor_bar_end") { stamp + 60 } else { stamp };
    let tenor = (ed - c.day) * 86_400 + close_of_expiry(ed) - from;
    Some((m, stamp, prem, spot, tenor))
}

/// `spikefade greeks-tsv`: the runner's greeks-tsv contract on stdin/stdout, through THIS engine's module (the 8b proof feeds it the live
/// app's inputs and compares every field bit for bit).
///
/// # Errors
/// Never (bad lines are printed REFUSED).
pub fn cmd_greeks_tsv() -> Result<String, String> {
    use std::io::{BufRead, Write};
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = std::io::BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let key = f.first().copied().unwrap_or("");
        if f.len() < 8 {
            let _ = writeln!(out, "{key}\tREFUSED\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t\tbad input line ({} fields)", f.len());
            continue;
        }
        let p = |i: usize| f.get(i).and_then(|x| x.trim().parse::<i64>().ok()).unwrap_or(0);
        let call = f.get(5).is_some_and(|x| x.trim().starts_with('C'));
        let ye4: u32 = f.get(6).and_then(|x| x.trim().parse().ok()).unwrap_or(0);
        let g = bar_greeks(p(1), p(2), p(3), p(4), call, ye4, p(7));
        if g.ok {
            let _ = writeln!(out, "{key}\tOK\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t", g.iv, g.method, g.iterations, g.uncertainty, g.delta, g.gamma_r, g.theta_d, g.vega_p, g.rho_p, g.pos_delta, g.pos_gamma, g.pos_theta, g.pos_vega, g.pos_rho, g.below_band, g.ns, g.rate, g.years);
        } else {
            let _ = writeln!(out, "{key}\tREFUSED\t\t\t\t\t\t\t\t\t\t\t\t\t\t\t{}\t{}\t{}\t{}\t{}", g.below_band, g.ns, g.rate, g.years, g.refusal);
        }
    }
    Ok(String::new())
}
