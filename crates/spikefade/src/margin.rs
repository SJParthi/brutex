//! The naked-margin ESTIMATE of one sold option unit (SNIPER s25 item 3 / s25b item 4).
//!
//! STOCK options: the s25 selling diagnostic's PROXY, reused (sell-diag `margin.rs` `per_unit`, `hundredths`, `pct_up`, copied):
//! per unit = max(ceil(S x VaR%) - OTM, ceil(S x 7.5%)) + ceil(S x ELM%), VaR% / ELM% = columns 7 / 8 of the stock's EQ row in NSE's
//! C_VAR1 cash margin file. NO C_VAR1 file for November 2026 is on this disk (the latest is 2026-10-30): the 2026-10-30 file is used for
//! every staged day (it is dated BEFORE every staged day, so no look-ahead; it is 1 to 8 sessions stale) and every result says so.
//! INDEX options: no estimate source exists on disk (no SPAN file; C_VAR1 has no index rows). ASSUMED formula, labelled on every row:
//! per unit = max(ceil(S x 6.00%) - OTM, ceil(S x 3.00%)) + ceil(S x 2.00%), plus ceil(S x 2.00%) on the option's own expiry day (the
//! SEBI 2024 extra ELM on short options on expiry day). S = the underlying's last 1-minute close at or before the decision.

use std::collections::BTreeMap;
use std::path::Path;

/// Stock options' short option minimum charge, % x 100 (sell-diag: 7.5%, UNVERIFIED).
pub const SOMC_STK_X100: i64 = 750;
/// ASSUMED index price scan range, % x 100.
pub const PSR_IDX_X100: i64 = 600;
/// ASSUMED index short option minimum charge, % x 100.
pub const SOMC_IDX_X100: i64 = 300;
/// ASSUMED index exposure (ELM), % x 100.
pub const ELM_IDX_X100: i64 = 200;
/// ASSUMED extra ELM on the option's own expiry day, % x 100.
pub const ELM_IDX_EXPIRY_X100: i64 = 200;

/// The EQ row of one stock in the C_VAR1 file (percentages x 100).
#[derive(Clone, Copy, Debug, Default)]
pub struct VarRow {
    /// Column 7: VaR margin (the PSR proxy).
    pub var_x100: i64,
    /// Column 8: extreme loss margin (the exposure proxy).
    pub elm_x100: i64,
}

/// "12.66" -> 1266 (sell-diag `hundredths`, copied).
#[must_use]
pub fn hundredths(s: &str) -> Option<i64> {
    let s = s.trim();
    let (a, b) = s.split_once('.').unwrap_or((s, ""));
    if a.is_empty() || !a.bytes().all(|c| c.is_ascii_digit()) || !b.bytes().all(|c| c.is_ascii_digit()) || b.len() > 2 {
        return None;
    }
    let whole: i64 = a.parse().ok()?;
    let frac: i64 = if b.is_empty() { 0 } else { b.parse::<i64>().ok()? * if b.len() == 1 { 10 } else { 1 } };
    Some(whole * 100 + frac)
}

/// Every EQ row of one C_VAR1 file, by symbol. The header date must equal `ddmmyyyy`.
///
/// # Errors
/// An unreadable file or a header date that is not the file's day.
pub fn load_var(path: &Path, ddmmyyyy: &str) -> Result<BTreeMap<String, VarRow>, String> {
    let text = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let text = String::from_utf8_lossy(&text);
    let mut lines = text.lines();
    let head = lines.next().unwrap_or("");
    let hd = head.split(',').nth(1).unwrap_or("").trim();
    if hd != ddmmyyyy {
        return Err(format!("{}: header date {hd:?} is not {ddmmyyyy}", path.display()));
    }
    let mut out = BTreeMap::new();
    for l in lines {
        let f: Vec<&str> = l.trim_end().split(',').collect();
        if f.len() != 10 || f.first() != Some(&"20") || f.get(2) != Some(&"EQ") {
            continue;
        }
        let (Some(v), Some(e)) = (f.get(6).and_then(|x| hundredths(x)), f.get(7).and_then(|x| hundredths(x))) else { continue };
        if let Some(sym) = f.get(1) {
            out.insert((*sym).to_owned(), VarRow { var_x100: v, elm_x100: e });
        }
    }
    Ok(out)
}

fn pct_up(s: i64, x100: i64) -> i64 {
    let n = i128::from(s.max(0)) * i128::from(x100.max(0));
    i64::try_from((n + 9_999) / 10_000).unwrap_or(i64::MAX)
}

/// One unit's estimate (paisa).
#[derive(Clone, Copy, Debug, Default)]
pub struct Unit {
    /// How far the sold strike is out of the money at S.
    pub otm: i64,
    /// The scan part.
    pub span: i64,
    /// The exposure part.
    pub exposure: i64,
    /// span + exposure.
    pub total: i64,
}

/// Stock option: the sell-diag proxy (copied): max(ceil(S x VaR) - OTM, ceil(S x 7.5%)) + ceil(S x ELM).
#[must_use]
pub fn per_unit_stock(s: i64, strike: i64, call: bool, v: &VarRow) -> Unit {
    let otm = if call { (strike - s).max(0) } else { (s - strike).max(0) };
    let span = (pct_up(s, v.var_x100) - otm).max(pct_up(s, SOMC_STK_X100));
    let exposure = pct_up(s, v.elm_x100);
    Unit { otm, span, exposure, total: span + exposure }
}

/// Index option: the ASSUMED formula (module docs).
#[must_use]
pub fn per_unit_index(s: i64, strike: i64, call: bool, expiry_day: bool) -> Unit {
    let otm = if call { (strike - s).max(0) } else { (s - strike).max(0) };
    let span = (pct_up(s, PSR_IDX_X100) - otm).max(pct_up(s, SOMC_IDX_X100));
    let exposure = pct_up(s, ELM_IDX_X100) + if expiry_day { pct_up(s, ELM_IDX_EXPIRY_X100) } else { 0 };
    Unit { otm, span, exposure, total: span + exposure }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_vectors() {
        // The sell-diag's own hand vector (S Rs 1,800, a PUT at Rs 1,760, VaR 16.05%, ELM 3.5%).
        let v = VarRow { var_x100: 1_605, elm_x100: 350 };
        let u = per_unit_stock(180_000, 176_000, false, &v);
        assert_eq!((u.otm, u.span, u.exposure, u.total), (4_000, 24_890, 6_300, 31_190));
        // Index, S Rs 25,000.00, a CALL at 25,500 (Rs 500 OTM): span = max(150,000 - 50,000, 75,000) = 100,000; ELM 50,000.
        let u = per_unit_index(2_500_000, 2_550_000, true, false);
        assert_eq!((u.otm, u.span, u.exposure, u.total), (50_000, 100_000, 50_000, 150_000));
        // Expiry day adds 2%.
        assert_eq!(per_unit_index(2_500_000, 2_550_000, true, true).exposure, 100_000);
        // Deep OTM falls to the 3% minimum.
        assert_eq!(per_unit_index(2_500_000, 2_800_000, true, false).span, 75_000);
    }
}
