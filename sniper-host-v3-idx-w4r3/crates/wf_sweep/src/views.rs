//! The sweep's views as DATA rows (plan section 6 items 3 and 5): THE picks `WF-R73` / `TT-R73` (E1 + E2 + E6), the BESIDE
//! evidence floors `-F2 / -F4 / -F8` (at least 2 / 4 / 8 trades in EVERY learning year part, E4), the BESIDE `-N` order (REQ 55
//! way B-N: K4 K6 K5) and the closest misses `-MISS` (P17: E7, key K1 K16 K17 then REQ 73's). Every key part is `common_wf`'s
//! (`acc::KeyKind`, SWEEP-EQ). Format (TSV, `#` comments): `id  window  floor  role  [key]  [k]`; `key` is R73 | N | MISS
//! (default R73), `k` the list bound (default: the run's `stage2_top`), as COMMON-WF VIEWS-STD-v1's k column.

use crate::SweepErr;
use crate::acc::KeyKind;

/// One view row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct View {
    /// The view id (the same id as COMMON-WF VIEWS-STD-v1, whose row the report path reads).
    pub id: String,
    /// The window index in the layout.
    pub win: usize,
    /// The E4 trade floor per learning year part (1 = E2's "a trade").
    pub floor: u32,
    /// THE (a pick of the search) or BESIDE / MISS.
    pub the: bool,
    /// The key the view ranks by.
    pub kind: KeyKind,
    /// The list bound (`None`: the run's `stage2_top`).
    pub k: Option<u64>,
}

/// Parses the views rows against the layout's window names.
///
/// # Errors
/// `BadView` for an unknown window, a floor of 0, an unknown role or key, a repeated id, or no THE view per window.
pub fn parse(text: &str, windows: &[String]) -> Result<Vec<View>, SweepErr> {
    let mut v: Vec<View> = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let l = line.trim_end();
        if l.is_empty() || l.starts_with('#') || l.starts_with("id\t") {
            continue;
        }
        let f: Vec<&str> = l.split('\t').collect();
        let bad = || SweepErr::BadView(format!("row {}: {l}", i + 1));
        let (Some(id), Some(w), Some(fl), Some(role)) = (f.first(), f.get(1), f.get(2), f.get(3)) else { return Err(bad()) };
        let win = windows.iter().position(|x| x == w).ok_or_else(bad)?;
        let floor: u32 = fl.parse().map_err(|_| bad())?;
        if floor == 0 {
            return Err(bad());
        }
        let kind = match f.get(4).copied().unwrap_or("R73") {
            "R73" => KeyKind::R73,
            "N" => KeyKind::N,
            "MISS" => KeyKind::Miss,
            _ => return Err(bad()),
        };
        let the = match (*role, kind) {
            ("THE", KeyKind::R73) => true,
            ("BESIDE", KeyKind::R73 | KeyKind::N) | ("MISS", KeyKind::Miss) => false,
            _ => return Err(bad()),
        };
        let k = match f.get(5) {
            None => None,
            Some(x) => Some(x.parse::<u64>().ok().filter(|k| *k > 0).ok_or_else(bad)?),
        };
        if v.iter().any(|x| x.id == *id) {
            return Err(bad());
        }
        v.push(View { id: (*id).to_string(), win, floor, the, kind, k });
    }
    for (wi, w) in windows.iter().enumerate() {
        if !v.iter().any(|x| x.the && x.win == wi) {
            return Err(SweepErr::BadView(format!("no THE view for window {w}")));
        }
    }
    Ok(v)
}
