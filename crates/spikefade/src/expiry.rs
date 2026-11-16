//! v3.2 (MAIN-RULINGS-ENGINE-v22 addendum 16): the INDEX expiry captured.
//! - WHICH expiry the Top Volume list is built inside (a dial, index options only; stock options keep rule R3): nearest (v3.1), next, the
//!   current monthly (the last expiry of the nearest expiry's calendar month among the contracts staged that day; else the next month's
//!   last), or the BUSIEST expiry by volume at the decision second (the day-so-far volume of each candidate expiry's contracts up to T;
//!   liquidity first, addendum 14). `run3 --expiry-pick nearest|next|monthly|busiest`.
//! - DAYS TO EXPIRY (a dial AND a result split): sessions from the trade day (exclusive) to the expiry (inclusive) on the NSE holiday
//!   calendar (weekdays not in the holiday files, plus the run's special sessions); 0 = the option's own expiry day. Buckets 0, 1, 2, 3-4,
//!   5+. `run3 --dte all|0|1|2|3-4|5+`.
//! - Every trade's expiry facts: the date, weekly / monthly (monthly = the last expiry of its calendar month among the expiries NSE listed
//!   for that underlying in the previous session's F&O bhavcopy; the cube's staged expiries when no bhavcopy, labelled), the weekday, the
//!   sessions to expiry, and whether a holiday moved it (its weekday differs from the regular weekday of that underlying's listed expiries
//!   = the most common weekday among them).
//! - The expiry-day exposure add-on (index options on their own expiry day, +2% from 2024-11-20, SEBI/HO/MRD/TPD-1/P/CIR/2024/132 para
//!   5.6) is THE margin module's (crate v2.3+ `IDX_EXPIRY_ADD_H` / `IDX_EXPIRY_ADD_FROM`); every expiry-day trade prints its 4% rate.

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

use crate::cal::{civil, day_of_ymd};
use crate::cube::Cube;

/// The run's session calendar: NSE holidays (both files) and the special sessions of the loaded days.
#[derive(Clone, Debug, Default)]
pub struct Sessions {
    pub hol: BTreeSet<i64>,
    pub special: BTreeSet<i64>,
}

/// The calendar (set once from the run's Cal3).
pub static SESS: OnceLock<Sessions> = OnceLock::new();
/// The which-expiry dial: 0 nearest (v3.1), 1 next, 2 current monthly, 3 busiest by volume at T.
pub static PICK: AtomicU8 = AtomicU8::new(0);
/// The days-to-expiry dial: 0 all, 1 = 0 (expiry day), 2 = 1, 3 = 2, 4 = 3-4, 5 = 5+.
pub static DTE: AtomicU8 = AtomicU8::new(0);

/// The dial names.
pub const PICK_NAMES: [&str; 4] = ["nearest", "next", "monthly", "busiest"];
pub const DTE_NAMES: [&str; 6] = ["all", "0", "1", "2", "3-4", "5+"];
/// The bucket names of the result split.
pub const BUCKETS: [&str; 5] = ["0 (expiry day)", "1", "2", "3-4", "5+"];

/// Whether `d` is a session: a weekday not in the holiday files, or a special session.
#[must_use]
pub fn is_session(d: i64) -> bool {
    let Some(s) = SESS.get() else { return (d + 3).rem_euclid(7) < 5 };
    if s.special.contains(&d) {
        return true;
    }
    // 1970-01-01 (day 0) was a Thursday: (d + 3) mod 7 = 0 Monday .. 4 Friday
    (d + 3).rem_euclid(7) < 5 && !s.hol.contains(&d)
}

/// Sessions from `day` (exclusive) to `expiry_day` (inclusive) on the run's calendar: 0 on the expiry day itself.
#[must_use]
pub fn sessions_to(day: i64, expiry_day: i64) -> i64 {
    match SESS.get() {
        Some(s) => sessions_between(day, expiry_day, &s.hol, &s.special),
        None => sessions_between(day, expiry_day, &BTreeSet::new(), &BTreeSet::new()),
    }
}

/// The same on an explicit calendar (weekdays not in `hol`, plus `special` sessions).
#[must_use]
pub fn sessions_between(day: i64, expiry_day: i64, hol: &BTreeSet<i64>, special: &BTreeSet<i64>) -> i64 {
    if expiry_day <= day {
        return 0;
    }
    // MUTANT m_v32_dte_calendar_days: calendar days counted instead of sessions
    if cfg!(feature = "m_v32_dte_calendar_days") {
        return expiry_day - day;
    }
    let n = ((day + 1)..=expiry_day).filter(|&d| special.contains(&d) || ((d + 3).rem_euclid(7) < 5 && !hol.contains(&d))).count();
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// The bucket (0..5) of a sessions-to-expiry count.
#[must_use]
pub const fn bucket(n: i64) -> usize {
    match n {
        i64::MIN..=0 => 0,
        1 => 1,
        2 => 2,
        3 | 4 => 3,
        _ => 4,
    }
}

/// Whether bucket `b` passes the run's DTE dial.
#[must_use]
pub fn dte_pass(b: usize) -> bool {
    match DTE.load(Ordering::Relaxed) {
        0 => true,
        x => usize::from(x) - 1 == b,
    }
}

/// The weekday name of a day.
#[must_use]
pub fn weekday(d: i64) -> &'static str {
    ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].get(usize::try_from((d + 3).rem_euclid(7)).unwrap_or(0)).copied().unwrap_or("?")
}

/// The expiries staged in the cube per underlying index (sorted).
#[must_use]
pub fn staged(c: &Cube) -> HashMap<u16, Vec<u32>> {
    let mut m: HashMap<u16, Vec<u32>> = HashMap::new();
    for k in &c.cons {
        let v = m.entry(k.und).or_default();
        if !v.contains(&k.expiry) {
            v.push(k.expiry);
        }
    }
    for v in m.values_mut() {
        v.sort_unstable();
    }
    m
}

/// The current monthly expiry of a sorted expiry list given the nearest: the last expiry in the nearest's calendar month (None: none).
#[must_use]
pub fn monthly_of(exps: &[u32], nearest: u32) -> Option<u32> {
    if nearest == 0 {
        return None;
    }
    let ym = nearest / 100;
    exps.iter().copied().filter(|&e| e / 100 == ym && e >= nearest).max()
}

/// The list expiry of an INDEX underlying under the which-expiry dial (busiest: None, chosen per decision second by the caller).
#[must_use]
pub fn pick_of(nearest: u32, next: u32, exps: &[u32]) -> Option<u32> {
    // MUTANT m_v32_expiry_pick_nearest: the dial read but the nearest expiry always used
    if cfg!(feature = "m_v32_expiry_pick_nearest") {
        return Some(nearest);
    }
    match PICK.load(Ordering::Relaxed) {
        0 => Some(nearest),
        1 => (next != 0).then_some(next),
        2 => monthly_of(exps, nearest),
        _ => None,
    }
}

/// REPAIR A (check-engine-2 M2): the which-expiry dial's TARGET for one index underlying, from the PREVIOUS session's F&O bhavcopy list of
/// that underlying (`listed`, the expiries NSE listed; only those on or after today count), checked against the cube's STAGED expiries:
/// - nearest (dial 0): the cube's nearest (v3.1's reading, unchanged);
/// - next (1): the second listed expiry on or after today; monthly (2): the last listed expiry of the nearest listed expiry's calendar month;
/// - busiest (3): the candidates (nearest, next, monthly) that are staged; each unstaged candidate is counted.
/// A target the cube did not stage, or no list held, is REFUSED (Err with its label): never replaced by another expiry.
///
/// # Errors
/// The refusal words (counted per day and underlying by the caller).
pub fn dial_allowed(pick: u8, nearest: u32, listed: Option<&[u32]>, staged: &[u32], today: u32) -> Result<(Vec<u32>, Vec<String>), String> {
    if pick == 0 {
        return Ok((if nearest != 0 { vec![nearest] } else { Vec::new() }, Vec::new()));
    }
    let Some(listed) = listed else {
        return Err(format!("{}: no F&O bhavcopy of the previous session held: the target expiry is unknown (refused, never substituted)", PICK_NAMES.get(usize::from(pick)).copied().unwrap_or("?")));
    };
    let mut alive: Vec<u32> = listed.iter().copied().filter(|&e| e >= today).collect();
    alive.sort_unstable();
    alive.dedup();
    let near = alive.first().copied().unwrap_or(0);
    let next = alive.get(1).copied().unwrap_or(0);
    let monthly = monthly_of(&alive, near).unwrap_or(0);
    let staged_words = staged.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(",");
    let need = |e: u32, what: &str| -> Result<u32, String> {
        if e == 0 {
            return Err(format!("{what}: NSE listed no such expiry in the previous session (listed {})", alive.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(",")));
        }
        // MUTANT m_v32r_expiry_substitute: an unstaged target replaced by the nearest staged expiry (h6's silent substitution)
        if !staged.contains(&e) {
            if cfg!(feature = "m_v32r_expiry_substitute") && nearest != 0 {
                return Ok(nearest);
            }
            return Err(format!("{what} expiry {e} not staged in the cube (staged {staged_words}): refused, never substituted"));
        }
        Ok(e)
    };
    match pick {
        1 => need(next, "next").map(|e| (vec![e], Vec::new())),
        2 => need(monthly, "monthly").map(|e| (vec![e], Vec::new())),
        _ => {
            let mut ok: Vec<u32> = Vec::new();
            let mut miss: Vec<String> = Vec::new();
            let mut cands = vec![near, next, monthly];
            cands.sort_unstable();
            cands.dedup();
            for e in cands.into_iter().filter(|&e| e != 0) {
                match need(e, "busiest candidate") {
                    Ok(x) => ok.push(x),
                    Err(w) => miss.push(w),
                }
            }
            ok.sort_unstable();
            ok.dedup();
            if ok.is_empty() { Err(format!("busiest: no candidate staged ({})", miss.join("; "))) } else { Ok((ok, miss)) }
        }
    }
}

/// REPAIR A (check-engine-2 M2): the refused underlying-days of the strict dial: (day, underlying) -> the refusal words (each counted once).
pub static NOT_STAGED: std::sync::Mutex<std::collections::BTreeMap<(i64, String), String>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// REPAIR A: the busiest dial's candidates that were not staged (busiest chose among the staged ones; counted apart from the refusals).
pub static PARTIAL: std::sync::Mutex<std::collections::BTreeMap<(i64, String), String>> = std::sync::Mutex::new(std::collections::BTreeMap::new());

/// The words of the strict dial's refusals (count and the first refusals).
#[must_use]
pub fn not_staged_words() -> String {
    let g = NOT_STAGED.lock().map(|g| g.clone()).unwrap_or_default();
    if g.is_empty() {
        return "which-expiry dial: 0 underlying-days refused (every target expiry staged)".to_owned();
    }
    let p = PARTIAL.lock().map(|g| g.len()).unwrap_or(0);
    format!("which-expiry dial: {} underlying-days REFUSED (target expiry not staged or not listed; counted, never substituted){}, e.g. {}", g.len(), if p > 0 { format!("; busiest chose among the staged candidates on {p} underlying-days with a candidate not staged") } else { String::new() }, g.iter().take(8).map(|((d, u), w)| format!("{} {u}: {w}", crate::cal::fmt_day(*d))).collect::<Vec<_>>().join("; "))
}

/// The candidate expiries of the busiest pick (nearest, next, monthly; deduplicated, non-zero).
#[must_use]
pub fn candidates(nearest: u32, next: u32, exps: &[u32]) -> Vec<u32> {
    let mut v: Vec<u32> = [Some(nearest), Some(next), monthly_of(exps, nearest)].into_iter().flatten().filter(|&e| e != 0).collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// The busiest candidate expiry of an underlying at slot end `b` (day-so-far volume of its contracts in slots [0, b)); ties: the nearer.
#[must_use]
pub fn busiest(c: &Cube, und: u16, cands: &[u32], b: usize) -> Option<u32> {
    let mut best: Option<(u64, u32)> = None;
    for &e in cands {
        let v: u64 = c.cons.iter().enumerate().filter(|(_, k)| k.und == und && k.expiry == e).map(|(ci, _)| c.wvol(ci, 0, b)).sum();
        if best.is_none_or(|x| v > x.0) {
            best = Some((v, e));
        }
    }
    best.map(|x| x.1)
}

/// The regular expiry weekday around expiry `e` (the most common weekday of the REFERENCE expiries `exps` within 35 days of it, NIFTY's
/// listed weeklies in the engine and the calendar: the local regime,
/// so a weekday change of the exchange is not read as a holiday move; a tie keeps `e`'s own weekday) and whether a HOLIDAY moved `e`:
/// its weekday is not the regular one AND every day after it up to the regular weekday's date is not a session, that date included (NSE
/// moves an expiry that falls on a holiday to the previous trading day). Returns (regular weekday 0 Mon .. 6 Sun, moved).
#[must_use]
pub fn regular_and_moved(exps: &[u32], e: u32, is_sess: &dyn Fn(i64) -> bool) -> (usize, bool) {
    let ed = day_of_ymd(e);
    let wd_of = |d: i64| usize::try_from((d + 3).rem_euclid(7)).unwrap_or(0);
    let own = wd_of(ed);
    let mut wd = [0_u32; 7];
    for &x in exps {
        let xd = day_of_ymd(x);
        if (xd - ed).abs() <= 35
            && let Some(c) = wd.get_mut(wd_of(xd))
        {
            *c += 1;
        }
    }
    let best = wd.iter().copied().max().unwrap_or(0);
    let reg = if wd.get(own).copied().unwrap_or(0) == best { own } else { wd.iter().position(|&c| c == best).unwrap_or(own) };
    // moved: some weekday at least as common as e's own (a tie included: a holiday-moved weekly series can tie its regular weekday)
    // whose next date after e is a non-session with only non-sessions between
    let own_n = wd.get(own).copied().unwrap_or(0);
    let gap_to = |w: usize| -> bool {
        let mut d = ed + 1;
        for _ in 0..7 {
            if wd_of(d) == w {
                return !is_sess(d);
            }
            if is_sess(d) {
                return false;
            }
            d += 1;
        }
        false
    };
    let moved = (0..7).any(|w| w != own && wd.get(w).copied().unwrap_or(0) >= own_n.max(1) && gap_to(w));
    (if moved && reg == own { (0..7).find(|&w| w != own && wd.get(w).copied().unwrap_or(0) >= own_n.max(1) && gap_to(w)).unwrap_or(reg) } else { reg }, moved)
}

/// One trade's expiry facts: (expiry day, monthly?, the source of the monthly test, weekday, sessions to expiry, moved by a holiday?).
#[must_use]
pub fn facts(c: &Cube, ci: usize) -> Option<(i64, bool, &'static str, &'static str, i64, bool)> {
    let k = c.cons.get(ci)?;
    let name = c.unds.get(usize::from(k.und)).map_or("", |u| u.name.as_str());
    let ed = day_of_ymd(k.expiry);
    let (exps, src) = match crate::bhav::expiries(c.day, name) {
        Some((_, v)) => (v, "NSE F&O bhavcopy of the previous session"),
        None => (staged(c).remove(&k.und).unwrap_or_default(), "the cube's staged contracts (no bhavcopy held)"),
    };
    let monthly = monthly_of(&exps, k.expiry) == Some(k.expiry);
    // the regime weekday from NIFTY's listed WEEKLY expiries (a monthly-only list holds one expiry within 35 days and cannot show its regular
    // weekday); the gap condition (every day up to the regular weekday a non-session) keeps another underlying's own regime safe
    let reference = crate::bhav::expiries(c.day, "NIFTY").map_or_else(|| exps.clone(), |x| x.1);
    let (_, moved) = regular_and_moved(&reference, k.expiry, &is_session);
    Some((ed, monthly, src, weekday(ed), sessions_to(c.day, ed), moved))
}

/// The calendar month of a day as "YYYY-MM".
#[must_use]
pub fn month_of(d: i64) -> String {
    let (y, m, _) = civil(d);
    format!("{y}-{m:02}")
}

/// Reads the dials from the command line.
///
/// # Errors
/// An unknown dial value.
pub fn from_args(a: &[String]) -> Result<String, String> {
    let get = |k: &str| a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str);
    let p = match get("--expiry-pick") {
        None | Some("nearest") => 0,
        Some("next") => 1,
        Some("monthly") => 2,
        Some("busiest") => 3,
        Some(o) => return Err(format!("--expiry-pick nearest|next|monthly|busiest, not {o}")),
    };
    let d = match get("--dte") {
        None | Some("all") => 0,
        Some("0") => 1,
        Some("1") => 2,
        Some("2") => 3,
        Some("3-4") => 4,
        Some("5+") => 5,
        Some(o) => return Err(format!("--dte all|0|1|2|3-4|5+, not {o}")),
    };
    if (p != 0 || d != 0) && !crate::sw::on(crate::sw::EXPIRY16) {
        return Err("--expiry-pick / --dte need the EXPIRY16 switch (--expiry16 on or --v32)".to_owned());
    }
    PICK.store(p, Ordering::Relaxed);
    DTE.store(d, Ordering::Relaxed);
    Ok(format!("which expiry {} (index lists); days to expiry {}", PICK_NAMES.get(usize::from(p)).copied().unwrap_or("?"), DTE_NAMES.get(usize::from(d)).copied().unwrap_or("?")))
}

/// REPAIR A (check-engine-2 M2): the expiries NSE listed for an underlying in the previous session's F&O bhavcopy (None: no file held).
#[must_use]
pub fn listed_for(day: i64, name: &str) -> Option<Vec<u32>> {
    crate::bhav::expiries(day, name).map(|x| x.1)
}

/// The busiest dial's candidate expiries of an index underlying: under the strict dial (EXP_NOSUB) the staged ones among the bhavcopy's
/// nearest / next / monthly; otherwise v3.2 h6's (the cube's nearest, next and staged monthly).
#[must_use]
pub fn busy_cands(c: &Cube, und: u16, st: &HashMap<u16, Vec<u32>>) -> Vec<u32> {
    let Some(u) = c.unds.get(usize::from(und)) else { return Vec::new() };
    let staged = st.get(&und).map_or(&[][..], Vec::as_slice);
    if crate::sw::on(crate::sw::EXP_NOSUB) {
        let listed = listed_for(c.day, &u.name);
        return dial_allowed(3, u.nearest, listed.as_deref(), staged, c.ymd).map(|x| x.0).unwrap_or_default();
    }
    candidates(u.nearest, u.next, staged)
}

/// REPAIR A (check-engine-2 M3): the DATED NSE special-session calendar (`--special-sessions <tsv>`: date, kind, first slot, last slot,
/// minutes, source): every special session NSE held (Muhurat, Saturday / Sunday live sessions), with its option-bar slots where the store
/// holds the day and its minutes; days to expiry and the theta target read it, never the days a run loads. day -> (first slot, last slot,
/// minutes, source).
#[derive(Clone, Debug, Default)]
pub struct SpecialCal {
    pub days: std::collections::BTreeMap<i64, (usize, usize, i64, String)>,
    pub path: String,
    pub sha: String,
}

/// The dated calendar (set once from `--special-sessions`).
pub static SPECIAL: OnceLock<SpecialCal> = OnceLock::new();

impl SpecialCal {
    /// Reads the table (header first; `#` notes). A row with slots must have minutes = last - first + 1; a row without ("-") gets the
    /// synthetic slots 0 .. minutes - 1 (only its length is ever read: no decision is taken on a day the store does not hold).
    ///
    /// # Errors
    /// An unreadable file or a malformed row.
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
        let mut days = std::collections::BTreeMap::new();
        for l in text.lines().skip(1) {
            if l.starts_with('#') || l.trim().is_empty() {
                continue;
            }
            let f: Vec<&str> = l.split('\t').collect();
            let (Some(d), Some(a), Some(b), Some(m), Some(src)) = (f.first().and_then(|x| crate::cal::parse_day(x)), f.get(2), f.get(3), f.get(4).and_then(|x| x.parse::<i64>().ok()), f.get(5)) else {
                return Err(format!("{}: bad row {l}", p.display()));
            };
            if m <= 0 {
                return Err(format!("{}: minutes must be positive: {l}", p.display()));
            }
            let (a, b) = match (a.parse::<usize>(), b.parse::<usize>()) {
                (Ok(a), Ok(b)) if b >= a => {
                    if m > i64::try_from(b + 1 - a).unwrap_or(0) {
                        return Err(format!("{}: trading minutes {m} above the span last - first + 1 in {l}", p.display()));
                    }
                    (a, b)
                }
                _ => (0, usize::try_from(m - 1).unwrap_or(0)),
            };
            days.insert(d, (a, b, m, (*src).to_owned()));
        }
        if days.is_empty() {
            return Err(format!("{}: no special session", p.display()));
        }
        Ok(Self { days, path: p.display().to_string(), sha: crate::cube::sha256(p) })
    }
}

/// The session calendar's words (REPAIR A M3).
#[must_use]
pub fn sess_words() -> String {
    match SPECIAL.get() {
        Some(s) if crate::sw::on(crate::sw::SESS_DATED) => format!("the DATED NSE special-session calendar {} (sha256 {}, {} sessions: {}); the loaded days never change it", s.path, s.sha, s.days.len(), s.days.iter().map(|(d, x)| format!("{} {} min", crate::cal::fmt_day(*d), x.2)).collect::<Vec<_>>().join(", ")),
        _ => "the special sessions of the loaded days (v3.2 h6)".to_owned(),
    }
}
