//! The option-chain readings (plan s2 N4): the store folders of a strike, the busiest rule (REQ 49 corrected + note 7, as
//! the idx1s tool chose it: the comparison column, gate G-EQ1), the lot rule (C17 / D9) and the A2 / s23a ITM-fit rule (the
//! headline contract, gate G-EQ3).

use std::collections::BTreeMap;
use std::path::Path;

use common_fill::limits::{SizeWhy, size_units_closest};
use idx_cal::day::{self, Day};
use idx_cal::storeio::{self, at_sod};

use crate::ladder::LadderRow;
use crate::secs::{self, Sec};

/// The store's option folders of one index: (strike, CE?) -> the expiry dates of its folders.
#[derive(Clone, Debug, Default)]
pub struct Folders {
    by: BTreeMap<(i64, bool), Vec<Day>>,
}

impl Folders {
    /// A folder list from (strike, CE?) -> expiry dates (unit tests and checks).
    #[must_use]
    pub fn from_pairs(pairs: Vec<((i64, bool), Vec<Day>)>) -> Self {
        Self { by: pairs.into_iter().collect() }
    }

    /// Lists `bars/gdfl/NSE/FNO/<index>` (directory names only).
    ///
    /// # Errors
    /// The directory cannot be read.
    pub fn list(root: &Path, index: &str) -> Result<Self, String> {
        let (m, _) = storeio::list_contracts(root, index)?;
        let mut by: BTreeMap<(i64, bool), Vec<Day>> = BTreeMap::new();
        for (d, sides) in m {
            for (si, ce) in [(0usize, true), (1usize, false)] {
                for k in sides.get(si).map_or(&[][..], Vec::as_slice) {
                    by.entry((*k, ce)).or_default().push(d);
                }
            }
        }
        Ok(Self { by })
    }

    /// The idx1s reading (its `foldersOf`): the folders of (strike, side) within 4 days of `e`, nearest first, then earlier.
    #[must_use]
    pub fn near(&self, e: Day, strike: i64, ce: bool) -> Vec<Day> {
        let mut v: Vec<Day> = self.by.get(&(strike, ce)).map_or(&[][..], Vec::as_slice).iter().copied().filter(|x| (x - e).abs() <= 4).collect();
        v.sort_by_key(|x| ((x - e).abs(), *x));
        v
    }

    /// Whether the folder of (expiry, strike, side) exists.
    #[must_use]
    pub fn has(&self, e: Day, strike: i64, ce: bool) -> bool {
        self.by.get(&(strike, ce)).is_some_and(|v| v.contains(&e))
    }
}

/// One candidate contract of a side.
#[derive(Clone, Debug)]
pub struct Cand {
    /// Its folder name.
    pub folder: String,
    /// Strike, paisa.
    pub strike: i64,
    /// Units traded in [09:15:00, t) for each asked instant.
    pub units: Vec<u64>,
}

/// The idx1s busiest candidates of a (day, side, expiry): per listed strike, among its folders within 4 days of `e`, the
/// one with the most units before the last instant (ties: the first), with its units before each instant.
///
/// # Errors
/// A refused store month.
#[allow(clippy::too_many_arguments, reason = "one (day, side, expiry) read")]
pub fn busiest_cands(root: &Path, index: &str, folders: &Folders, row: &LadderRow, ce: bool, e: Day, d: Day, ts: &[i64]) -> Result<Vec<Cand>, String> {
    let strikes = if ce { &row.ce } else { &row.pe };
    let last = ts.iter().copied().max().unwrap_or(0);
    let mut out = Vec::new();
    for &k in strikes {
        let mut best: Option<Cand> = None;
        for fd in folders.near(e, k, ce) {
            let name = storeio::contract_name(fd, k, ce);
            let to_sod = (last - at_sod(d, 0)) / 1_000_000;
            let Some(rows) = secs::option_window(root, index, &name, d, 9 * 3600 + 15 * 60, to_sod)? else { continue };
            let pre: Vec<&Sec> = rows.iter().take_while(|r| r.ts < last).collect();
            let units: Vec<u64> = ts.iter().map(|t| pre.iter().filter(|r| r.ts < *t).map(|r| r.vol).sum()).collect();
            let mx = units.iter().copied().max().unwrap_or(0);
            if best.as_ref().is_none_or(|b| mx > b.units.iter().copied().max().unwrap_or(0)) {
                best = Some(Cand { folder: name, strike: k, units });
            }
        }
        if let Some(b) = best {
            out.push(b);
        }
    }
    Ok(out)
}

/// The leader and the runner-up at instant number `j`: units desc, then |strike - spot| asc, then strike asc (C26).
#[must_use]
pub fn leader(cands: &[Cand], j: usize, spot: i64) -> (Option<&Cand>, Option<&Cand>) {
    let mut v: Vec<&Cand> = cands.iter().filter(|c| c.units.get(j).copied().unwrap_or(0) > 0).collect();
    v.sort_by(|a, b| {
        let (ua, ub) = (a.units.get(j).copied().unwrap_or(0), b.units.get(j).copied().unwrap_or(0));
        ub.cmp(&ua).then(((a.strike - spot).abs()).cmp(&(b.strike - spot).abs())).then(a.strike.cmp(&b.strike))
    });
    (v.first().copied(), v.get(1).copied())
}

fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// C17 / D9: the stated UDiFF lot, else the largest old-form candidate dividing every print before t (the traded-seconds
/// guard), the LCM when the dividing candidates' LCM exceeds the largest (F7), else refused (0, "lot unknown at t").
#[must_use]
pub fn lot_at(row: &LadderRow, prints: &[u64]) -> (i64, &'static str) {
    if let Some(l) = row.lot {
        return (l, "stated (UDiFF)");
    }
    let c = &row.cands;
    if c.len() == 1 && prints.is_empty() {
        return (c.first().copied().unwrap_or(0), "largest candidate dividing every print before t");
    }
    let div: Vec<i64> = c.iter().copied().filter(|&x| x > 0 && prints.iter().all(|p| i64::try_from(*p).unwrap_or(1) % x == 0)).collect();
    let Some(&largest) = div.iter().max() else { return (0, "lot unknown at t") };
    let mut l = 1i64;
    for x in &div {
        l = l / gcd(l, *x) * x;
    }
    if l == largest { (largest, "largest candidate dividing every print before t") } else { (l, "LCM of the dividing candidates (F7)") }
}

/// The A2 / s23a pick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItmPick {
    /// Folder name.
    pub folder: String,
    /// Strike.
    pub strike: i64,
    /// ITM depth: strikes skipped beyond the nearest ITM strike (no trade before t, or too big for one order).
    pub depth: u32,
    /// Units (lots x lot), C11 closest of the target.
    pub qty: u64,
    /// The size reference: HIGH of the last traded second before t (crate C4).
    pub reference: i64,
    /// Its age, seconds (F5).
    pub ref_age_s: i64,
    /// Strikes skipped for no trade in [09:15:00, t).
    pub skipped_no_trade: u32,
    /// Strikes skipped because their closest count exceeds one order (or is 0, or their lot is unknown).
    pub skipped_big: u32,
    /// The pick's lot (C17 / D9 at t).
    pub lot: u64,
    /// The one-order maximum for that lot on the day (12:06 note).
    pub max_order: i64,
}

/// One ITM candidate: (strike, folder, the contract-day's rows, its lot by C17 / D9 at t; 0 = lot unknown, never picked).
pub type ItmCand<'a> = (i64, &'a str, &'a [Sec], u64);

/// A2 / s23a: at instant `t`, the in-the-money strike nearest the money (CE: strike < spot; PE: strike > spot; a strike
/// exactly at the spot is NOT in the money) with a trade in [09:15:00, t) whose closest whole lots of `target` fit ONE order
/// (lots >= 1 and qty <= `max_order`, the dated one-order maximum); walked outward from the money; the first that fits wins.
///
/// # Errors
/// `no_itm_fit` when none fits (refused by name); a C11 host error.
pub fn itm_fit(cands: &[ItmCand<'_>], ce: bool, spot: i64, t: i64, target: i64, free: Option<i64>, freeze_units: i64) -> Result<ItmPick, String> {
    let mut itm: Vec<&ItmCand<'_>> = cands.iter().filter(|c| if ce { c.0 < spot } else { c.0 > spot }).collect();
    if ce {
        itm.sort_by_key(|c| std::cmp::Reverse(c.0));
    } else {
        itm.sort_by_key(|c| c.0);
    }
    let (mut depth, mut no_trade, mut big) = (0u32, 0u32, 0u32);
    for c in itm {
        let Some(last) = secs::last_before(c.2, t) else {
            no_trade += 1;
            depth += 1;
            continue;
        };
        let lot = c.3;
        if lot == 0 {
            big += 1;
            depth += 1;
            continue;
        }
        let max_order = (freeze_units / i64::try_from(lot).unwrap_or(i64::MAX)) * i64::try_from(lot).unwrap_or(0);
        let (qty, why) = size_units_closest(target, Some(last.high), lot, free).map_err(|e| format!("C11 {e:?}"))?;
        if !why.refused() && !matches!(why, SizeWhy::CashShortZero { .. }) && qty >= 1 && i64::try_from(qty).unwrap_or(i64::MAX) <= max_order {
            return Ok(ItmPick { folder: c.1.to_string(), strike: c.0, depth, qty, reference: last.high, ref_age_s: (t - last.ts) / 1_000_000, skipped_no_trade: no_trade, skipped_big: big, lot, max_order });
        }
        big += 1;
        depth += 1;
    }
    Err("no_itm_fit".to_string())
}

/// The instant `HH:MM:SS` on day `d`.
#[must_use]
pub fn at_hms(d: Day, h: i64, m: i64, s: i64) -> i64 {
    at_sod(d, h * 3600 + m * 60 + s)
}

/// `YYYY-MM-DD` of a day (re-export for the binaries).
#[must_use]
pub fn fmt(d: Day) -> String {
    day::fmt(d)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, reason = "tests")]
mod tests {
    use super::*;

    fn row(lot: Option<i64>, cands: &[i64]) -> LadderRow {
        LadderRow { lot, cands: cands.to_vec(), ..LadderRow::default() }
    }

    #[test]
    fn lot_rule_c17_d9() {
        assert_eq!(lot_at(&row(Some(65), &[]), &[130]), (65, "stated (UDiFF)"));
        assert_eq!(lot_at(&row(None, &[25, 75]), &[150, 225]), (75, "largest candidate dividing every print before t"));
        assert_eq!(lot_at(&row(None, &[75, 50]), &[150, 300]), (150, "LCM of the dividing candidates (F7)"));
        assert_eq!(lot_at(&row(None, &[40]), &[]), (40, "largest candidate dividing every print before t"));
        assert_eq!(lot_at(&row(None, &[40]), &[30]), (0, "lot unknown at t"));
    }

    fn s(ts: i64, high: i64) -> Sec {
        Sec { ts, high, low: high, close: high, vol: 75 }
    }

    /// s3.5 (1)-(4), (11): a strike exactly at the spot is not ITM; a strike with no trade before t is skipped and counted;
    /// a count one lot above the order maximum goes deeper; exactly at the maximum is ONE order; none fits: refused.
    #[test]
    fn itm_fit_rules() {
        let t = 100 * 1_000_000;
        let a: Vec<Sec> = vec![s(1_000_000, 20_000)];
        let b: Vec<Sec> = vec![];
        let c: Vec<Sec> = vec![s(2_000_000, 27_778)];
        let e: Vec<Sec> = vec![s(3_000_000, 50_000)];
        // spot 2,000,000 (20,000.00); CE strikes 2,000,000 (at the spot: not ITM), 1,995,000 (no trade), 1,990,000, 1,985,000
        let cands: Vec<ItmCand<'_>> = vec![(2_000_000, "atm", &e, 75), (1_995_000, "no", &b, 75), (1_990_000, "big", &a, 75), (1_985_000, "fit", &c, 75)];
        // Rs 5 L at Rs 200 with lot 75: 33 lots = 2,475 units > a 1,800 maximum (deeper); at Rs 277.78: 24 lots = 1,800 (fits)
        let p = itm_fit(&cands, true, 2_000_000, t, 50_000_000, None, 1_800);
        assert_eq!(p.as_ref().map(|p| (p.folder.as_str(), p.depth, p.skipped_no_trade, p.skipped_big)), Ok(("fit", 2, 1, 1)));
        // 50,000,000 / (27,778 x 75) = 23.99 -> 24 lots = 1,800 units: exactly at a 1,800 maximum is ONE order (F8)
        let p2 = itm_fit(&cands[3..], true, 2_000_000, t, 50_000_000, None, 1_800).map(|p| p.qty).unwrap_or(0);
        assert_eq!(p2, 1_800);
        assert!(itm_fit(&cands[3..], true, 2_000_000, t, 50_000_000, None, 1_725).is_err(), "one lot above the maximum: no strike left");
        // a PUT: strikes above the spot only
        let p3 = itm_fit(&cands, false, 1_992_000, t, 50_000_000, None, 3_000);
        assert_eq!(p3.map(|p| p.folder), Ok("atm".to_string()));
    }
}
