//! DP-IDXCAL (plan s4.1): the dated NSE expiry calendar of one index from its F&O bhavcopy listings, read causally.
//!
//! * A TRUE expiry day is a date whose OWN bhavcopy lists it as an expiry (B3).
//! * For session `d`, the file dated before `d` is read: the current expiry is the listed series whose effective date is
//!   the first one at or after `d`. The HOLIDAY RULE: a listed date that is not a REGULAR session of the dated session table (a Muhurat day is a trading holiday with a special evening session)
//!   expires on the previous session (dates after the table's last session are kept as listed: data end).
//! * RE-DATED SERIES: a listed date that vanishes from the next file before it has expired is the same series as the
//!   date that replaces it, joined by CONTRACT IDENTITY: the UDiFF `FinInstrmId` where both files are UDiFF, else the
//!   strikes and sides within 3 days (old form; the window is asserted over EVERY listed series).
//! * THE TRUTH ASSERTION (G-CAL (1)): for every series that was ever current as known, the causal roll day must equal
//!   the series' true last trading day (the last file that lists it under any alias).

use std::collections::{BTreeMap, BTreeSet};

use crate::bhav::Row;
use crate::day::{self, Day};
use crate::listing::FileRow;

/// The effective expiry of a listed date under the holiday rule.
#[must_use]
pub fn eff(x: Day, sessions: &BTreeSet<Day>) -> Day {
    let last = sessions.iter().next_back().copied().unwrap_or(Day::MIN);
    if x > last || sessions.contains(&x) {
        return x;
    }
    sessions.range(..x).next_back().copied().unwrap_or(x)
}

/// One run of consecutive files listing one date.
#[derive(Clone, Debug)]
pub struct Alias {
    /// The listed date.
    pub date: Day,
    /// First and last file index (into the index's file list) of the run.
    pub first: usize,
    /// Last file index.
    pub last: usize,
    /// The series it belongs to.
    pub series: usize,
}

/// A listed date that left the listing before it expired, and its successor by contract identity.
#[derive(Clone, Debug)]
pub struct Vanish {
    /// The last file (date) that lists it.
    pub last_file: Day,
    /// The next file.
    pub next_file: Day,
    /// The vanished date.
    pub date: Day,
    /// The successor date (`None`: none found; a G-CAL disagreement).
    pub succ: Option<Day>,
    /// How: `fin_id` (UDiFF identity), `strike_side` (old form within 3 days), `none`.
    pub how: &'static str,
    /// Contracts compared and found.
    pub detail: String,
    /// |succ - date| in days.
    pub shift: i32,
}

/// One series (an equivalence class of aliases).
#[derive(Clone, Debug)]
pub struct Series {
    /// Its aliases (indexes into `Cal::aliases`), in time order.
    pub aliases: Vec<usize>,
    /// The true last trading day: the last file that lists it under any alias.
    pub true_last: Day,
    /// Still listed (unexpired) in the last file: open at the data end.
    pub open_at_end: bool,
    /// The causal roll day: the first session on which it is current as known with its effective date that day.
    pub causal_roll: Option<Day>,
    /// It was current as known on at least one session in range.
    pub ever_current: bool,
}

/// One session's calendar row (the DP-IDXCAL row).
#[derive(Clone, Debug)]
pub struct Row1 {
    /// The session.
    pub session: Day,
    /// The file read (dated before the session).
    pub file_read: Day,
    /// The current expiry as known: its listed alias and its effective date.
    pub known_listed: Day,
    /// Effective date (holiday rule).
    pub known_eff: Day,
    /// The series id.
    pub series: usize,
    /// The next expiry as known (effective), if listed.
    pub next_eff: Option<Day>,
    /// The true current expiry (the first true expiry day at or after the session).
    pub true_current: Option<Day>,
    /// D-EXP: the session is a true expiry day of the index.
    pub d_exp: bool,
    /// Regular sessions in (session, known_eff] (the ex-post count on the dated session table).
    pub dte_sessions: u32,
    /// Weekdays in (session, known_eff] (the causal count with no closure announcement on disk).
    pub dte_weekdays: u32,
    /// Weekday closures (weekdays that are not sessions) inside (session, known_eff].
    pub closures: Vec<Day>,
    /// Special sessions (not regular) inside (session, known_eff].
    pub specials: Vec<Day>,
}

/// The whole calendar of one index.
#[derive(Clone, Debug, Default)]
pub struct Cal {
    /// The index.
    pub index: String,
    /// File dates.
    pub files: Vec<Day>,
    /// Every alias run.
    pub aliases: Vec<Alias>,
    /// Every vanish event with its resolution.
    pub vanishes: Vec<Vanish>,
    /// Every series.
    pub series: Vec<Series>,
    /// True expiry days (all files).
    pub true_days: BTreeSet<Day>,
    /// The session rows in range.
    pub rows: Vec<Row1>,
    /// Truth-assertion differences (empty = pass).
    pub truth_diffs: Vec<String>,
    /// Series open at the data end that were current as known (listed, never a difference).
    pub open_at_end: Vec<String>,
    /// Old-form re-dates outside the 3-day window, or unresolved (empty = pass).
    pub window_violations: Vec<String>,
}

/// Where the raw contract rows of one file come from (the bhavcopy zips; a test double in the tests).
pub trait RowSource {
    /// The OPTIDX rows of the index in the file dated `file` (form `udiff`).
    ///
    /// # Errors
    /// The zip cannot be found or read.
    fn rows(&mut self, file: Day, udiff: bool) -> Result<Vec<Row>, String>;
}

/// Union-find with path halving.
fn find(p: &mut [usize], mut i: usize) -> usize {
    loop {
        let Some(&pi) = p.get(i) else { return i };
        if pi == i {
            return i;
        }
        let Some(&gp) = p.get(pi) else { return pi };
        if let Some(slot) = p.get_mut(i) {
            *slot = gp;
        }
        i = gp;
    }
}

fn resolve(src: &mut dyn RowSource, a: &FileRow, b: &FileRow, x: Day) -> (Option<Day>, &'static str, String) {
    let (ra, rb) = match (src.rows(a.file, a.udiff), src.rows(b.file, b.udiff)) {
        (Ok(ra), Ok(rb)) => (ra, rb),
        (Err(e), _) | (_, Err(e)) => return (None, "none", format!("rows unreadable: {e}")),
    };
    let new_dates: BTreeSet<Day> = {
        let la: BTreeSet<Day> = a.listed.iter().map(|l| l.expiry).collect();
        b.listed.iter().map(|l| l.expiry).filter(|e| !la.contains(e)).collect()
    };
    if a.udiff && b.udiff {
        let ids: BTreeSet<u64> = ra.iter().filter(|r| r.expiry == x).filter_map(|r| r.fin_id).collect();
        let by_id: BTreeMap<u64, Day> = rb.iter().filter_map(|r| r.fin_id.map(|i| (i, r.expiry))).collect();
        let mut to: BTreeMap<Day, usize> = BTreeMap::new();
        let mut missing = 0usize;
        for i in &ids {
            match by_id.get(i) {
                Some(d) => *to.entry(*d).or_default() += 1,
                None => missing += 1,
            }
        }
        let detail = format!("{} ids; found {}; missing {missing}; dates {:?}", ids.len(), ids.len() - missing, to.iter().map(|(d, n)| format!("{}x{n}", day::fmt(*d))).collect::<Vec<_>>());
        if to.len() == 1 && missing == 0 && !ids.is_empty() {
            return (to.keys().next().copied(), "fin_id", detail);
        }
        return (None, "none", detail);
    }
    let sx: BTreeSet<(i64, bool)> = ra.iter().filter(|r| r.expiry == x).map(|r| (r.strike, r.ce)).collect();
    let mut fits = Vec::new();
    let mut far = Vec::new();
    for y in &new_dates {
        let sy: BTreeSet<(i64, bool)> = rb.iter().filter(|r| r.expiry == *y).map(|r| (r.strike, r.ce)).collect();
        if !sx.is_empty() && sx.is_subset(&sy) {
            if (y - x).abs() <= 3 {
                fits.push(*y);
            } else {
                far.push(*y);
            }
        }
    }
    let detail = format!(
        "{} contracts; new dates {:?}; within 3 days {:?}; beyond 3 days {:?}",
        sx.len(),
        new_dates.iter().map(|d| day::fmt(*d)).collect::<Vec<_>>(),
        fits.iter().map(|d| day::fmt(*d)).collect::<Vec<_>>(),
        far.iter().map(|d| day::fmt(*d)).collect::<Vec<_>>()
    );
    if fits.len() == 1 {
        return (fits.first().copied(), "strike_side", detail);
    }
    (None, "none", detail)
}

/// Builds the calendar of one index.
///
/// `sessions` is the dated session table; `regular` its regular sessions (the table less the special rows that are not
/// regular full sessions); `lo..=hi` the session range of the rows; `identity = false` is the naive reading (mutant
/// `m_idx_redate`, used by the red test).
///
/// # Errors
/// An empty listing.
#[allow(clippy::too_many_lines, clippy::too_many_arguments, clippy::fn_params_excessive_bools, reason = "one pass per stage, in plan order; the two mutant switches are explicit")]
pub fn build(
    index: &str,
    files: &[FileRow],
    sessions: &BTreeSet<Day>,
    regular: &BTreeSet<Day>,
    lo: Day,
    hi: Day,
    src: &mut dyn RowSource,
    identity: bool,
    look_ahead: bool,
) -> Result<Cal, String> {
    if files.is_empty() {
        return Err(format!("{index}: no files"));
    }
    let n = files.len();
    let fdays: Vec<Day> = files.iter().map(|f| f.file).collect();
    // 1. alias runs
    let mut aliases: Vec<Alias> = Vec::new();
    let mut open: BTreeMap<Day, usize> = BTreeMap::new();
    for (k, f) in files.iter().enumerate() {
        let mut next_open = BTreeMap::new();
        for l in &f.listed {
            let ai = match open.get(&l.expiry) {
                Some(&ai) => {
                    if let Some(a) = aliases.get_mut(ai) {
                        a.last = k;
                    }
                    ai
                }
                None => {
                    aliases.push(Alias { date: l.expiry, first: k, last: k, series: 0 });
                    aliases.len() - 1
                }
            };
            next_open.insert(l.expiry, ai);
        }
        open = next_open;
    }
    let true_days: BTreeSet<Day> = files.iter().filter(|f| f.listed.iter().any(|l| l.expiry == f.file)).map(|f| f.file).collect();
    // 2. vanish events and identity
    let mut parent: Vec<usize> = (0..aliases.len()).collect();
    let mut vanishes = Vec::new();
    let mut window_violations = Vec::new();
    let start_of: BTreeMap<(Day, usize), usize> = aliases.iter().enumerate().map(|(i, a)| ((a.date, a.first), i)).collect();
    for (ai, a) in aliases.iter().enumerate() {
        if a.last + 1 >= n {
            continue;
        }
        let (Some(fa), Some(fb)) = (files.get(a.last), files.get(a.last + 1)) else { continue };
        if eff(a.date, regular) <= fa.file {
            continue; // expired normally on its last file
        }
        let (succ, how, detail) = if identity { resolve(src, fa, fb, a.date) } else { (None, "none", "identity off (m_idx_redate)".to_string()) };
        let shift = succ.map_or(0, |y| (y - a.date).abs());
        if !fa.udiff || !fb.udiff {
            match succ {
                Some(_) if shift <= 3 => {}
                _ => window_violations.push(format!("{index} {} vanished after file {}: {how}; {detail}", day::fmt(a.date), day::fmt(fa.file))),
            }
        } else if succ.is_none() {
            window_violations.push(format!("{index} {} vanished after file {} (UDiFF): {detail}", day::fmt(a.date), day::fmt(fa.file)));
        }
        if let Some(y) = succ {
            match start_of.get(&(y, a.last + 1)) {
                Some(&bi) => {
                    let (ra, rb) = (find(&mut parent, ai), find(&mut parent, bi));
                    if let Some(s) = parent.get_mut(ra) {
                        *s = rb;
                    }
                }
                None => window_violations.push(format!("{index} {} -> {}: the successor was already listed before file {}", day::fmt(a.date), day::fmt(y), day::fmt(fb.file))),
            }
        }
        vanishes.push(Vanish { last_file: fa.file, next_file: fb.file, date: a.date, succ, how, detail, shift });
    }
    // 3. series
    let mut root_to_series: BTreeMap<usize, usize> = BTreeMap::new();
    let mut series: Vec<Series> = Vec::new();
    for ai in 0..aliases.len() {
        let r = find(&mut parent, ai);
        let si = *root_to_series.entry(r).or_insert_with(|| {
            series.push(Series { aliases: Vec::new(), true_last: Day::MIN, open_at_end: false, causal_roll: None, ever_current: false });
            series.len() - 1
        });
        if let (Some(s), Some(a)) = (series.get_mut(si), aliases.get_mut(ai)) {
            a.series = si;
            s.aliases.push(ai);
            let lf = fdays.get(a.last).copied().unwrap_or(Day::MIN);
            s.true_last = s.true_last.max(lf);
            if a.last + 1 == n && eff(a.date, regular) > lf {
                s.open_at_end = true;
            }
        }
    }
    // 4. session rows (causal)
    let mut rows = Vec::new();
    for &d in sessions.range(lo..=hi) {
        let p = if look_ahead { fdays.partition_point(|&f| f <= d) } else { fdays.partition_point(|&f| f < d) };
        let Some(pk) = p.checked_sub(1) else { continue };
        let Some(fp) = files.get(pk) else { continue };
        let mut cands: Vec<(Day, Day)> = fp.listed.iter().map(|l| (eff(l.expiry, regular), l.expiry)).filter(|(e, _)| *e >= d).collect();
        cands.sort_unstable();
        let Some(&(ke, kl)) = cands.first() else { continue };
        let next_eff = cands.get(1).map(|c| c.0);
        let Some(&ai) = start_of.range(..=(kl, pk)).rev().find(|((dt, _), _)| *dt == kl).map(|(_, i)| i) else { continue };
        let si = aliases.get(ai).map_or(0, |a| a.series);
        if let Some(s) = series.get_mut(si) {
            s.ever_current = true;
            if ke == d && s.causal_roll.is_none() {
                s.causal_roll = Some(d);
            }
        }
        let true_current = true_days.range(d..).next().copied();
        let mut closures = Vec::new();
        let mut specials = Vec::new();
        let mut dte_sessions = 0u32;
        let mut dte_weekdays = 0u32;
        let mut z = d + 1;
        while z <= ke && z <= hi.max(ke) {
            let wd = day::weekday(z);
            let weekday = (1..=5).contains(&wd);
            if weekday {
                dte_weekdays += 1;
            }
            if regular.contains(&z) {
                dte_sessions += 1;
            } else if sessions.contains(&z) {
                specials.push(z);
            } else if weekday && z <= sessions.iter().next_back().copied().unwrap_or(Day::MIN) {
                closures.push(z);
            }
            z += 1;
        }
        rows.push(Row1 {
            session: d,
            file_read: fp.file,
            known_listed: kl,
            known_eff: ke,
            series: si,
            next_eff,
            true_current,
            d_exp: true_days.contains(&d),
            dte_sessions,
            dte_weekdays,
            closures,
            specials,
        });
    }
    // 5. the truth assertion
    let mut truth_diffs = Vec::new();
    let mut open_at_end = Vec::new();
    for (si, s) in series.iter().enumerate() {
        if !s.ever_current {
            continue;
        }
        let names: Vec<String> = s.aliases.iter().filter_map(|&ai| aliases.get(ai)).map(|a| day::fmt(a.date)).collect();
        if s.open_at_end {
            open_at_end.push(format!("{index} series {si} [{}] open at the data end (last file {})", names.join(" -> "), day::fmt(s.true_last)));
            continue;
        }
        if s.causal_roll != Some(s.true_last) {
            truth_diffs.push(format!(
                "{index} series {si} [{}]: causal roll {} vs true last trading day {}",
                names.join(" -> "),
                s.causal_roll.map_or_else(|| "none".to_string(), day::fmt),
                day::fmt(s.true_last)
            ));
        }
    }
    Ok(Cal { index: index.to_string(), files: fdays, aliases, vanishes, series, true_days, rows, truth_diffs, open_at_end, window_violations })
}

/// The monthly weekday of an index in month `(y, m)` (0 = Sunday): NIFTY Thursday until 2025-08 and Tuesday after
/// (57264, 65336 item 1, 68747); BANKNIFTY Thursday until 2024-02, Wednesday 2024-03..2024-12 (60011 via 61814's list),
/// Thursday 2025-01..2025-08 (65336), Tuesday after (68747).
#[must_use]
pub fn monthly_weekday(index: &str, y: i32, m: u32) -> u32 {
    let k = y * 100 + i32::try_from(m).unwrap_or(0);
    if index == "BANKNIFTY" {
        if k <= 202_402 {
            4
        } else if k <= 202_412 {
            3
        } else if k <= 202_508 {
            4
        } else {
            2
        }
    } else if k <= 202_508 {
        4
    } else {
        2
    }
}

/// The weekly weekday (0 = Sunday): NIFTY Thursday until 2025-08-28 then Tuesday (68747); BANKNIFTY Thursday until
/// 2023-08-31, then Wednesday (57540 via 61814's list) to the last weekly 2024-11-13 (64506).
#[must_use]
pub fn weekly_weekday(index: &str, d: Day) -> u32 {
    if index == "BANKNIFTY" {
        if d <= day::from_civil(2023, 8, 31) { 4 } else { 3 }
    } else if d <= day::from_civil(2025, 8, 28) {
        4
    } else {
        2
    }
}

/// Weekly / monthly classes of the true expiry days in `lo..=hi` (plan s4.1: monthly = the latest true expiry within
/// [L - 3 days, L], L = the month's last date on the era's monthly weekday). Returns true expiry day -> monthly.
#[must_use]
pub fn classes(index: &str, true_days: &BTreeSet<Day>, lo: Day, hi: Day) -> BTreeMap<Day, bool> {
    let in_range: Vec<Day> = true_days.range(lo..=hi).copied().collect();
    let mut monthly: BTreeSet<Day> = BTreeSet::new();
    let months: BTreeSet<(i32, u32)> = in_range.iter().map(|d| day::ym(*d)).collect();
    for (y, m) in months {
        let w = monthly_weekday(index, y, m);
        let mut l = day::month_end(day::from_civil(y, m, 1));
        while day::weekday(l) != w {
            l -= 1;
        }
        if let Some(c) = in_range.iter().rfind(|d| day::ym(**d) == (y, m) && **d <= l && **d >= l - 3) {
            monthly.insert(*c);
        }
    }
    in_range.into_iter().map(|d| (d, monthly.contains(&d))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::listing::Listed;

    struct NoRows;
    impl RowSource for NoRows {
        fn rows(&mut self, _: Day, _: bool) -> Result<Vec<Row>, String> {
            Err("no rows in this test".into())
        }
    }
    struct Fake(BTreeMap<Day, Vec<Row>>);
    impl RowSource for Fake {
        fn rows(&mut self, f: Day, _: bool) -> Result<Vec<Row>, String> {
            self.0.get(&f).cloned().ok_or_else(|| "missing".into())
        }
    }
    fn d(s: &str) -> Day {
        day::parse(s).unwrap_or(0)
    }
    fn fr(f: &str, udiff: bool, ex: &[&str]) -> FileRow {
        FileRow { file: d(f), udiff, listed: ex.iter().map(|e| Listed { expiry: d(e), actual: None, ce: 1, pe: 1 }).collect() }
    }
    fn row(e: &str, k: i64, ce: bool, id: Option<u64>) -> Row {
        Row { expiry: d(e), strike: k, ce, fin_id: id, underlying: None, lot: None }
    }

    /// The June 2023 holiday shift: 06-29 listed until the file of 06-27; 06-28 first listed by its own file; 06-29 is
    /// not a session. The series rolls on 06-28 and its true last trading day is 06-28 (identity: strikes, 1 day).
    #[test]
    fn holiday_shift_with_a_late_listing_rolls_on_the_true_day() {
        let files = vec![
            fr("2023-06-26", false, &["2023-06-29", "2023-07-06"]),
            fr("2023-06-27", false, &["2023-06-29", "2023-07-06"]),
            fr("2023-06-28", false, &["2023-06-28", "2023-07-06"]),
            fr("2023-06-30", false, &["2023-07-06"]),
        ];
        let s: BTreeSet<Day> = ["2023-06-26", "2023-06-27", "2023-06-28", "2023-06-30"].iter().map(|x| d(x)).collect();
        let mut src = Fake(BTreeMap::from([
            (d("2023-06-27"), vec![row("2023-06-29", 1, true, None), row("2023-06-29", 1, false, None), row("2023-07-06", 1, true, None)]),
            (d("2023-06-28"), vec![row("2023-06-28", 1, true, None), row("2023-06-28", 1, false, None), row("2023-07-06", 1, true, None)]),
        ]));
        let c = build("NIFTY", &files, &s, &s, d("2023-06-27"), d("2023-06-30"), &mut src, true, false);
        let c = c.unwrap_or_default();
        assert!(c.truth_diffs.is_empty(), "{:?}", c.truth_diffs);
        assert!(c.true_days.contains(&d("2023-06-28")));
        assert_eq!(c.vanishes.first().and_then(|v| v.succ), Some(d("2023-06-28")));
        assert!(c.rows.iter().any(|r| r.session == d("2023-06-28") && r.known_eff == d("2023-06-28") && r.d_exp));
    }

    /// A re-dated UDiFF series (BANKNIFTY 2025-01-29 -> 01-30 by FinInstrmId) rolls on 01-30; with identity OFF (the
    /// naive reading, mutant m_idx_redate) the truth assertion turns red.
    #[test]
    fn redated_series_join_by_identity_and_the_naive_reading_is_red() {
        let files = vec![
            fr("2024-12-31", true, &["2025-01-29", "2025-02-26"]),
            fr("2025-01-01", true, &["2025-01-29", "2025-02-26"]),
            fr("2025-01-02", true, &["2025-01-30", "2025-02-27"]),
            fr("2025-01-29", true, &["2025-01-30", "2025-02-27"]),
            fr("2025-01-30", true, &["2025-01-30", "2025-02-27"]),
            fr("2025-01-31", true, &["2025-02-27"]),
        ];
        let s: BTreeSet<Day> = files.iter().map(|f| f.file).collect();
        let rows = |e1: &str, e2: &str| vec![row(e1, 1, true, Some(10)), row(e1, 1, false, Some(11)), row(e2, 1, true, Some(20))];
        let mut src = Fake(BTreeMap::from([(d("2025-01-01"), rows("2025-01-29", "2025-02-26")), (d("2025-01-02"), rows("2025-01-30", "2025-02-27"))]));
        let good = build("BANKNIFTY", &files, &s, &s, d("2025-01-01"), d("2025-01-31"), &mut src, true, false).unwrap_or_default();
        assert!(good.truth_diffs.is_empty(), "{:?}", good.truth_diffs);
        assert_eq!(good.vanishes.len(), 2);
        assert!(good.window_violations.is_empty(), "{:?}", good.window_violations);
        // 2025-01-29 is a session but never an expiry day: its file lists 01-30.
        assert!(!good.true_days.contains(&d("2025-01-29")));
        let stale: Vec<Day> = good.rows.iter().filter(|r| r.true_current.is_some() && Some(r.known_eff) != r.true_current).map(|r| r.session).collect();
        assert_eq!(stale, vec![d("2025-01-01"), d("2025-01-02")]);
        let naive = build("BANKNIFTY", &files, &s, &s, d("2025-01-01"), d("2025-01-31"), &mut NoRows, false, false).unwrap_or_default();
        assert!(!naive.truth_diffs.is_empty(), "the naive reading must fail the truth assertion");
    }

    #[test]
    fn eff_follows_the_session_table_and_keeps_dates_after_it() {
        let s: BTreeSet<Day> = ["2023-06-27", "2023-06-28", "2023-06-30"].iter().map(|x| d(x)).collect();
        assert_eq!(eff(d("2023-06-29"), &s), d("2023-06-28"));
        assert_eq!(eff(d("2023-06-28"), &s), d("2023-06-28"));
        assert_eq!(eff(d("2023-07-06"), &s), d("2023-07-06"));
    }
}
