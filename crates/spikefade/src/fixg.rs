//! v3.3 GATE FIXES (fix:v33, 16 Nov 2026; MAIN-RULINGS-ENGINE-v22 Addenda 24-28): the BLOCKING items of the two v3.3 gates and the
//! independent re-check on binary 0577f612 (`v32/audit-v33/STANDING-SCORECARD.md` F1, F2, F4, F5, F6; `v32/exits-audit-v33/EXITS-SCORECARD.md`
//! F-B2 (= F4); `v33/RECHECK-V33.md` F1) that code alone can close. F3 (dated stock freeze files) and F7 / F-B1 (price bands, NSE limit-price
//! protection, Dhan's option protection %) need data or a ruling and are NOT here; F2's rows print their real state.
//!
//! Like the `fix33` switches these CHANGE results. Each defaults to OFF, so with every one off the binary replays v3.3 (0577f612...) byte for
//! byte, with or without `--fix33` / `--v33` (proof P0). `run3 --fix33g` turns every one on; each has its own flag. Their words go to RUN.log,
//! to the RESULTS header and to the day-major checkpoint fingerprint (empty when none is on, so every v3.3 print stays byte for byte).
//! Process-wide in the binary (set once from the command line before any work); under `cfg(test)` each test thread has its own copy.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use crate::cube::{Cube, SecTape};

/// A fix:v33 switch (an index into the switch table).
pub type Fg = usize;
/// STANDING F1 (out-of-box G1): on corporate-action ex-dates the day cubes keep the old, too-small lot (48 events, 65 stock-days, lots 2 to
/// 667 times too small; TATASTEEL 2022-07-28 lot 425 against a traded quantum of 4,250). Every trade is a whole number of REAL lots, so every
/// traded second's quantity is a multiple of the real lot. Under this switch each loaded day groups its contracts by (underlying, cube lot)
/// and takes the GCD of every traded second's quantity in the group; a group whose GCD is a whole multiple k >= 2 of its cube lot holds a lot
/// below the traded quantum: every event on its contracts is REFUSED (`lot_below_traded_quantum`, counted per contract-day and per pick),
/// never sized with a lot no live order could carry.
pub const LOT: Fg = 0;
/// STANDING F2 (dynamic-common D6, out-of-box G2): the checklist rows follow their own evidence: row 12 (freeze slicing) COVERED only when
/// every contract-day read a DATED limit (PARTIAL with UNDATED reads, NOT COVERED with contract-days of no limit at all: unsliced orders);
/// row 14 PARTIAL (the cube contents are not sha-checked at load: F9); row 8 names GST on the SEBI fee and stays PARTIAL without it on a
/// run holding a session from 2022-07-18; row 13 cites the v3.3 proof; row 17 cites this round's tests and stays PARTIAL.
pub const ITEMS: Fg = 1;
/// STANDING F4 = EXITS F-B2 (dynamic-common D12): GST 18% on the SEBI fee from 2022-07-18 (NSE FATAX52993 "GST @ 18% on the fees payable
/// to SEBI from July 18, 2022"; Dhan's pricing: 18% on brokerage + transaction charges + SEBI turnover fee + IPFT), on both sides.
pub const GST: Fg = 2;
/// STANDING F5 (dynamic-common D17): index exposure from a DATED table (`--idx-elm FILE`: from, to, rate, the rate printed beside, class,
/// source), never the flat 2% constant: Addendum 4 rules 2% where NSE's dated rule says so and, before the first dated evidence of 2%,
/// NSE's circular value; until it is found 3% (conservative), labelled, with 2% beside. The 2024-11-20 expiry-day add-on stays on top.
pub const IDXELM: Fg = 3;
/// RECHECK-V33 F1 (BLOCKING): the 15 SLIDING window rows (R1 / R2 / R3, learning starting 2019-2024) flag 45.5% of the checked accounts each
/// on the run's own all-years lists (19.6 M rows, a 5.53 GB checkpoint and a 50.6 GB footprint after ONE 6 M batch; ~4.3e9 rows, ~1.2 TB of
/// checkpoint text extrapolated). Under this switch the sweep evaluates the 6 GROWING rows only (they never flag: their learning starts at
/// the first piece) and holds no flagged row; after the sweep each sliding row ranks a BOUNDED candidate set (every SEARCH account any group
/// and view keeps: the top K of each, which holds every pick), each candidate walked FRESH from the row's first learning piece (exact, the
/// D3 re-walk). A sliding row is then NOT ranked over all combos: MAIN and the operator rule how far Addendum 18's rolling splits must rank.
pub const WIN: Fg = 4;
const N: usize = 5;
/// The number of switches (tests reset every one).
pub const COUNT: usize = N;

#[cfg(not(test))]
static FLAGS: [std::sync::atomic::AtomicBool; N] = [const { std::sync::atomic::AtomicBool::new(false) }; N];
#[cfg(test)]
thread_local! {
    static FLAGS: [std::cell::Cell<bool>; N] = const { [const { std::cell::Cell::new(false) }; N] };
}

/// One switch's value.
#[inline]
#[must_use]
pub fn on(s: Fg) -> bool {
    #[cfg(not(test))]
    {
        FLAGS.get(s).is_some_and(|f| f.load(Ordering::Relaxed))
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| f.get(s).is_some_and(std::cell::Cell::get))
    }
}

/// Sets one switch.
pub fn set(s: Fg, v: bool) {
    #[cfg(not(test))]
    {
        if let Some(f) = FLAGS.get(s) {
            f.store(v, Ordering::Relaxed);
        }
    }
    #[cfg(test)]
    {
        FLAGS.with(|f| {
            if let Some(c) = f.get(s) {
                c.set(v);
            }
        });
    }
}

/// Whether any switch is on.
#[must_use]
pub fn any() -> bool {
    (0..N).any(on)
}

/// The words (RUN.log, the RESULTS header, the checkpoint fingerprint; empty when none is on).
#[must_use]
pub fn words() -> String {
    if !any() {
        return String::new();
    }
    let w = |s: Fg, a: String, b: &str| if on(s) { a } else { b.to_owned() };
    [
        w(LOT, "lots checked against each day's traded quantum: a contract whose (underlying, cube lot) group trades only in whole multiples k >= 2 of the cube lot REFUSED (lot_below_traded_quantum, counted) (standing F1)".to_owned(), "lots as v3.3"),
        w(ITEMS, "checklist rows follow their evidence: row 12 COVERED only when every contract-day read a dated freeze limit; row 14 PARTIAL (cube contents not sha-checked); rows 8 / 13 / 17 from this run's facts (standing F2)".to_owned(), "checklist words as v3.3"),
        w(GST, "GST 18% on the SEBI fee from 2022-07-18 (NSE FATAX52993) (standing F4, exits F-B2)".to_owned(), "SEBI fee outside the GST base (v3.3)"),
        w(IDXELM, format!("index exposure from the dated table {} (2% only where dated evidence holds; before it 3% CONSERVATIVE, 2% printed beside; Addendum 4) (standing F5)", IDX.get().map_or_else(|| "(none)".to_owned(), |t| t.src.clone())), "index exposure 2% flat (v3.3)"),
        w(WIN, "sliding window rows R1 / R2 / R3 ranked over a bounded candidate set (every SEARCH account a group and view keeps), each walked fresh from its row's start (exact); the sweep holds no flagged row (Recheck-v33 F1)".to_owned(), "sliding window rows over every account (v3.3)"),
    ]
    .join("; ")
}

/// The fingerprint part (empty when none is on: the v3.3 fingerprints unchanged).
#[must_use]
pub fn fp_part() -> String {
    if any() { format!("; fix33g {}", words()) } else { String::new() }
}

/// Reads the switches (`--fix33g` = every one; each flag may override) and the dated index exposure table.
///
/// # Errors
/// A switch value that is not one of its words; IDXELM on without a readable `--idx-elm` table.
pub fn from_args(a: &[String]) -> Result<(), String> {
    let all = a.iter().any(|x| x == "--fix33g");
    let flag = |k: &str, yes: &str, no: &str| -> Result<Option<bool>, String> {
        match a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).map(String::as_str) {
            None => Ok(None),
            Some(v) if v == yes => Ok(Some(true)),
            Some(v) if v == no => Ok(Some(false)),
            Some(v) => Err(format!("{k} {yes}|{no}, not {v}")),
        }
    };
    for (s, k, yes, no) in [(LOT, "--lot-guard", "on", "off"), (ITEMS, "--checklist33g", "on", "off"), (GST, "--gst-sebi", "on", "off"), (IDXELM, "--idx-elm-dated", "on", "off"), (WIN, "--win-sliding", "candidates", "all")] {
        set(s, flag(k, yes, no)?.unwrap_or(all));
    }
    if on(IDXELM) {
        let p = a.iter().position(|x| x == "--idx-elm").and_then(|i| a.get(i + 1)).ok_or("--idx-elm-dated on (or --fix33g) needs --idx-elm <the dated index exposure table>")?;
        let t = IdxElm::load(std::path::Path::new(p))?;
        let _ = IDX.set(t);
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------ LOT (standing F1)

/// LOT: the contract-days refused (distinct (session, security id, segment)), and the groups found ((segment, session, underlying, cube
/// lot) -> (k, traded seconds of the group)) in words.
static LOT_SEEN: Mutex<BTreeSet<(i64, u32, u8)>> = Mutex::new(BTreeSet::new());
static LOT_GROUPS: Mutex<BTreeMap<(u8, i64, String, u32), (u64, u32)>> = Mutex::new(BTreeMap::new());

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    a
}

/// The day's traded quantum per (underlying, cube lot) group: (the GCD of every traded second's quantity of the group's contracts, 0 = no
/// traded second; the group's traded seconds). O(seconds of the day) once per loaded day.
#[must_use]
pub fn quanta(c: &Cube, t: &SecTape) -> HashMap<(u16, u32), (u64, u32)> {
    let mut g: HashMap<(u16, u32), (u64, u32)> = HashMap::new();
    for (ci, k) in c.cons.iter().enumerate() {
        // v3.4 E8: a segment slice holds no rows of the other segment's contracts; their (underlying, lot) groups are the other segment's
        // own (an underlying is one segment's), so leaving them out is exact for every contract the slice holds (always true on a whole cube)
        if k.lot == 0 || !c.allowed(ci) {
            continue;
        }
        let e = g.entry((k.und, k.lot)).or_insert((0, 0));
        for r in t.of(ci) {
            if r.vol > 0 {
                e.0 = gcd(e.0, u64::from(r.vol));
                e.1 = e.1.saturating_add(1);
            }
        }
    }
    g
}

/// LOT: marks every contract of the day whose (underlying, cube lot) group trades only in whole multiples k >= 2 of its cube lot
/// (`c.lotbad[ci]` = k, capped at 255; 0 = the lot holds). Counted once per contract-day (a streamed run reloads a day per pass). A thin
/// group (few traded seconds) can show k >= 2 by chance: it is refused too (conservative: never a lot no order could carry), and its
/// traded seconds are printed beside so a thin refusal is visible.
pub fn lot_mark(c: &mut Cube, t: &SecTape) {
    if !on(LOT) {
        return;
    }
    let q = quanta(c, t);
    let mut bad = vec![0_u8; c.cons.len()];
    for (ci, k) in c.cons.iter().enumerate() {
        let Some(&(g, ns)) = q.get(&(k.und, k.lot)) else { continue };
        let lot = u64::from(k.lot);
        if lot > 0 && g > 0 && g % lot == 0 && g / lot >= 2 {
            let kk = u8::try_from(g / lot).unwrap_or(u8::MAX);
            if let Some(b) = bad.get_mut(ci) {
                *b = kk;
            }
            if let Ok(mut s) = LOT_SEEN.lock() {
                s.insert((c.day, k.sid, k.kind));
            }
            if let Ok(mut m) = LOT_GROUPS.lock() {
                let name = c.unds.get(usize::from(k.und)).map_or("?", |u| u.name.as_str()).to_owned();
                m.insert((k.kind, c.day, name, k.lot), (g / lot, ns));
            }
        }
    }
    c.lotbad = bad;
}

/// LOT: whether contract `ci` of the day is refused (its lot below the traded quantum).
#[inline]
#[must_use]
pub fn lot_refused(c: &Cube, ci: usize) -> bool {
    // MUTANT m_oob_lot_guard_off: the guard computed but never applied (a lot below the traded quantum sized as if it held)
    on(LOT) && !cfg!(feature = "m_oob_lot_guard_off") && c.lotbad.get(ci).is_some_and(|&k| k >= 2)
}

/// LOT: the counts in words per segment (RESULTS, checklist item 12): the cube holds both segments, so a day's check covers both whatever
/// segment the run walks.
#[must_use]
pub fn lot_line() -> String {
    let seg = |kind: u8| {
        let n = LOT_SEEN.lock().map_or(0, |s| s.iter().filter(|x| x.2 == kind).count());
        let (ng, g) = LOT_GROUPS.lock().map(|m| {
            let v: Vec<String> = m.iter().filter(|x| x.0.0 == kind).map(|((_, d, u, l), (k, ns))| format!("{} {u} cube lot {l} x{k} ({ns} traded seconds)", crate::cal::fmt_day(*d))).collect();
            (v.len(), v.into_iter().take(40).collect::<Vec<_>>())
        }).unwrap_or_default();
        format!("{} options {n} contract-day(s) in {ng} (session, underlying, lot) group(s){}{}", if kind == 0 { "INDEX" } else { "STOCK" }, if g.is_empty() { String::new() } else { format!(": {}", g.join("; ")) }, if ng > 40 { " ..." } else { "" })
    };
    format!("lot guard (fix:v33 LOT, standing F1; the cube days loaded by this run, both segments): refused as lot_below_traded_quantum: {}; {}", seg(0), seg(1))
}

// ------------------------------------------------------------------------------------------------ GST (standing F4)

/// GST: whether the SEBI fee joins the GST base on session `d` (NSE FATAX52993: from 2022-07-18).
#[inline]
#[must_use]
pub fn gst_on_sebi(d: i64) -> bool {
    // MUTANT m_v33f_gst_no_sebi: the SEBI fee left out of the GST base under the switch
    on(GST) && !cfg!(feature = "m_v33f_gst_no_sebi") && d >= crate::cal::days_from_civil(2022, 7, 18)
}

/// GST: the sessions of the run from 2022-07-18 (item 8 is PARTIAL on such a run with the switch off).
pub static GST_DAYS: AtomicU64 = AtomicU64::new(0);

// ------------------------------------------------------------------------------------------------ IDXELM (standing F5)

/// One dated row of the index exposure table: [from, to] (yyyymmdd), the rate and the rate printed beside (hundredths of a percent), the
/// class (VERIFIED / CONSERVATIVE / ...), the source.
#[derive(Clone, Debug)]
pub struct IdxRow {
    pub from: u32,
    pub to: u32,
    pub rate_h: i64,
    pub beside_h: i64,
    pub class: String,
    pub source: String,
}

/// The dated index exposure table (`--idx-elm`).
#[derive(Clone, Debug, Default)]
pub struct IdxElm {
    pub rows: Vec<IdxRow>,
    /// The file, its sha256 and its rows (printed).
    pub src: String,
}

/// The run's table (set once from the command line).
pub static IDX: OnceLock<IdxElm> = OnceLock::new();

impl IdxElm {
    /// Parses the table text (tab-separated; header `from\tto\trate_h\tbeside_h\tclass\tsource`; days YYYY-MM-DD; rows in order, never
    /// overlapping).
    ///
    /// # Errors
    /// A header or row that is not the format, or overlapping rows.
    pub fn parse(text: &str) -> Result<Vec<IdxRow>, String> {
        let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
        let head = lines.next().unwrap_or("");
        if !head.starts_with("from\tto\trate_h\tbeside_h\tclass\tsource") {
            return Err(format!("--idx-elm: header must be from<TAB>to<TAB>rate_h<TAB>beside_h<TAB>class<TAB>source, not {head}"));
        }
        let mut rows: Vec<IdxRow> = Vec::new();
        for l in lines {
            let f: Vec<&str> = l.split('\t').collect();
            let ymd = |s: Option<&&str>| s.and_then(|x| crate::cal::parse_day(x.trim())).map(crate::cal::ymd_of);
            let (Some(from), Some(to), Some(rate_h), Some(beside_h), Some(class), Some(source)) = (ymd(f.first()), ymd(f.get(1)), f.get(2).and_then(|x| x.trim().parse::<i64>().ok()), f.get(3).and_then(|x| x.trim().parse::<i64>().ok()), f.get(4), f.get(5)) else {
                return Err(format!("--idx-elm: bad row {l}"));
            };
            if from > to || !(0..=10_000).contains(&rate_h) || !(0..=10_000).contains(&beside_h) {
                return Err(format!("--idx-elm: bad row {l}"));
            }
            if rows.last().is_some_and(|r| r.to >= from) {
                return Err(format!("--idx-elm: rows overlap or are out of order at {l}"));
            }
            rows.push(IdxRow { from, to, rate_h, beside_h, class: (*class).trim().to_owned(), source: (*source).trim().to_owned() });
        }
        if rows.is_empty() {
            return Err("--idx-elm: no row".to_owned());
        }
        Ok(rows)
    }

    /// Loads the table file.
    ///
    /// # Errors
    /// An unreadable file or a refused row.
    pub fn load(p: &std::path::Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(p).map_err(|e| format!("--idx-elm {}: {e}", p.display()))?;
        let rows = Self::parse(&text)?;
        let src = format!("{} (sha256 {}, {} dated rows)", p.display(), crate::cube::sha256(p), rows.len());
        Ok(Self { rows, src })
    }

    /// The row in force on trade date `ymd` (None = the table does not cover it).
    #[must_use]
    pub fn at(&self, ymd: u32) -> Option<&IdxRow> {
        let i = self.rows.partition_point(|r| r.to < ymd);
        self.rows.get(i).filter(|r| r.from <= ymd)
    }
}

/// IDXELM: the index exposure rate on trade date `ymd` in place of the crate's flat 2% (the crate's expiry-day add-on `add` on top): (rate,
/// the rate printed beside, conservative = the row is not VERIFIED). None = the switch is off (the crate's rate stands).
///
/// # Errors
/// A trade date the table does not cover (the run refuses rather than guess a rate).
pub fn idx_rate(ymd: u32, add: i64) -> Result<Option<(i64, i64, bool)>, String> {
    if !on(IDXELM) {
        return Ok(None);
    }
    let t = IDX.get().ok_or("--idx-elm table not loaded")?;
    let r = t.at(ymd).ok_or_else(|| format!("--idx-elm: no dated row covers {ymd}"))?;
    // MUTANT m_v33f_idx_elm_flat_2pct: the flat 2% kept under the switch (the dated table read but never applied)
    let rate = if cfg!(feature = "m_v33f_idx_elm_flat_2pct") { common_fill::margin::IDX_ELM_H } else { r.rate_h };
    Ok(Some((rate + add, r.beside_h + add, r.class != "VERIFIED")))
}

/// IDXELM: checks that the table covers every run session; returns the counts in words (sessions per class).
///
/// # Errors
/// A run session the table does not cover.
pub fn idx_cover(days: &[i64]) -> Result<String, String> {
    if !on(IDXELM) {
        return Ok(String::new());
    }
    let t = IDX.get().ok_or("--idx-elm table not loaded")?;
    let mut by: BTreeMap<String, (usize, i64, i64)> = BTreeMap::new();
    for &d in days {
        let ymd = crate::cal::ymd_of(d);
        let r = t.at(ymd).ok_or_else(|| format!("--idx-elm: no dated row covers the run session {}", crate::cal::fmt_day(d)))?;
        let e = by.entry(format!("{} {}.{:02}% (beside {}.{:02}%; {})", r.class, r.rate_h / 100, r.rate_h % 100, r.beside_h / 100, r.beside_h % 100, r.source)).or_insert((0, d, d));
        e.0 += 1;
        e.1 = e.1.min(d);
        e.2 = e.2.max(d);
    }
    Ok(format!("index exposure (fix:v33 IDXELM, standing F5; table {}): {}", t.src, by.iter().map(|(k, (n, a, b))| format!("{n} session(s) {}..{} at {k}", crate::cal::fmt_day(*a), crate::cal::fmt_day(*b))).collect::<Vec<_>>().join("; ")))
}

/// IDXELM: the run's coverage words (set once per run after the day list is known; printed in RESULTS).
pub static IDX_WORDS: OnceLock<String> = OnceLock::new();

// ------------------------------------------------------------------------------------------------ the MTM mark (standing F6, matrix X22)

/// THE running-MTM mark of contract `ci`'s day, minute by minute (matrix X22): the minute's HIGH when its bar is present (the worst price
/// for a short), else the forward-filled close. The kernel reads minute b - 1 at a decision in minute b (bars ended by T only).
#[must_use]
pub fn minute_marks(c: &Cube, ci: usize) -> Vec<i32> {
    // v3.4 E8: a direct array read, counted outside a loaded segment slice
    c.guard(ci);
    (0..crate::cal::NM)
        .map(|m| {
            let k = Cube::ix(ci, m);
            // MUTANT m_v33f_mark_close: the minute's close read instead of its HIGH (a re-spike inside a minute missed)
            if c.flags.get(k).is_some_and(|f| f & 1 == 1) && !cfg!(feature = "m_v33f_mark_close") { c.high.get(k).copied().unwrap_or(0) } else { c.close_at(ci, m) }
        })
        .collect()
}

// ------------------------------------------------------------------------------------------------ WIN (Recheck-v33 F1)

/// WIN: whether the sweep skips window row `growing == false` (a sliding row: ranked after the sweep over the candidate set).
#[inline]
#[must_use]
pub fn skip_sliding(growing: bool) -> bool {
    // MUTANT m_v33g_win_sliding_swept: the sliding rows still swept (and flagged) under the switch
    on(WIN) && !growing && !cfg!(feature = "m_v33g_win_sliding_swept")
}

/// WIN: the candidate accounts walked fresh for the sliding rows, per segment (counted for RESULTS).
pub static WIN_CANDS: AtomicU64 = AtomicU64::new(0);

/// WIN: the candidate set of one segment: every distinct SEARCH account with a trade that any group and view keeps (`tops`), as (list,
/// setting slot, share level, N index, summary), sorted by (list, setting, share level, N).
#[must_use]
pub fn candidates(tops: &[crate::acct::Tops], search: &[bool], nl: usize) -> Vec<(u32, u32, u8, u8, crate::acct::Sum)> {
    let mut m: BTreeMap<(u32, u32, u8, u8), crate::acct::Sum> = BTreeMap::new();
    for t in tops {
        for v in 0..=crate::acct::NVIEW {
            for s in t.best(v) {
                let x = usize::from(s.x);
                let (si, l) = (x / nl.max(1), x % nl.max(1));
                if s.trades == 0 || !search.get(si).copied().unwrap_or(false) {
                    continue;
                }
                m.entry((s.list, u32::try_from(si).unwrap_or(0), u8::try_from(l).unwrap_or(0), s.n)).or_insert(s);
            }
        }
    }
    m.into_iter().map(|(k, s)| (k.0, k.1, k.2, k.3, s)).collect()
}

/// WIN: the words of the candidate pass (Table W, RESULTS).
#[must_use]
pub fn win_line() -> String {
    format!("sliding window rows (fix:v33 WIN, Recheck-v33 F1): ranked over the candidate set only ({} candidate account(s) over the segments: every SEARCH account with a trade that a group and view keeps), each walked FRESH from its row's first learning piece (exact); accounts outside the set are NOT ranked on the sliding rows; the 6 growing rows rank every account (MAIN / operator ruling owed: how far Addendum 18's rolling splits must rank)", WIN_CANDS.load(Ordering::Relaxed))
}

// ------------------------------------------------------------------------------------------------ ITEMS (standing F2)

/// ITEMS: checklist row 12 (freeze slicing) from its own counters: COVERED only when every contract-day read a DATED limit; PARTIAL with
/// UNDATED reads only; NOT COVERED with contract-days of no limit at all (orders unsliced, their Rs 20 slices uncharged). `state` = the
/// switch-only reading (round B's); returns the state and the words added to the evidence.
#[must_use]
pub fn item12(state: &'static str, undated: u64, none: u64) -> (&'static str, String) {
    // MUTANT m_oob_item12_static: the row kept at its switch-only reading whatever its counters say
    if cfg!(feature = "m_oob_item12_static") || state != "COVERED" || (undated == 0 && none == 0) {
        return (state, String::new());
    }
    (
        if none > 0 { "NOT COVERED" } else { "PARTIAL" },
        format!("; fix:v33 ITEMS (standing F2 / F3): COVERED only when every contract-day read a DATED freeze limit: {undated} contract-day(s) read the UNDATED stock file (sessions before the first dated stock file) and {none} contract-day(s) had NO limit known (orders unsliced, their Rs 20 slices uncharged); NSE's dated stock freeze files are not held (data, or a MAIN ruling)"),
    )
}

/// ITEMS: checklist row 8 (charges) on GST on the SEBI fee: with the GST switch the evidence names it; without it a run holding a session
/// from 2022-07-18 reads PARTIAL.
#[must_use]
pub fn item8(state: &'static str, gst: bool, gst_days: u64) -> (&'static str, String) {
    if gst {
        (state, "; fix:v33 GST: GST 18% on the SEBI fee from 2022-07-18 (NSE FATAX52993; test v33f_gst_on_sebi_from_20220718)".to_owned())
    } else if gst_days > 0 && state == "COVERED" {
        ("PARTIAL", format!("; GST 18% on the SEBI fee from 2022-07-18 (NSE FATAX52993) NOT applied on {gst_days} session(s) of this run (standing F4: --gst-sebi on)"))
    } else {
        (state, String::new())
    }
}

/// ITEMS: checklist row 14 (data precision): PARTIAL until the cube contents are sha-checked at load and every seconds tape is checked
/// against its cube (standing F9, HIGH, open).
pub const ITEM14: (&str, &str) = ("PARTIAL", "cube proofs; every NSE file named with its sha256 in RUN.log; NOT yet: the cube CONTENTS sha-checked at load, a seconds tape checked against its cube (version, day, rows per contract), cube files hashed in the launcher's manifest (standing F9, HIGH, open)");

/// ITEMS: checklist row 13 (Greeks): the v3.3 proof files.
pub const ITEM13: &str = "crates/greeks linked unmodified; 8b proof on v3.3: v33/integrate/greeks8b/PROOF-GREEKS-8B-INT.txt (164 / 164 rows equal to the live /greeks.json, f64 bit equality), re-run on the fix:v33 build: v33/fix/greeks8b/PROOF-GREEKS-8B-FIX.txt";

/// ITEMS: checklist row 17 (the extreme matrix): this round's named tests, and the rows still open.
pub const ITEM17: &str = "named tests of v3.3: BUILD-V33 sections 1-3 (tests33.rs, tests33f.rs, tests33g.rs); matrix BLOCKING rows X06 / X22 / X61 named-tested with killed mutants; still NOT COVERED or PARTIAL: X05 / X10 (band lock, NSE limit-price protection: data), X07, X08, X09 (halts), X11, X12, X14, X15 (session clock), X17, X18, X19, X20 (CAP_MIN covers the pick views only), X21, X23, X25, X26 (STANDING-SCORECARD F32)";

/// ITEMS: checklist row 5 (protection and bands): what closes it.
pub const ITEM5: &str = "; standing F7 = exits F-B1 (BLOCKING): needs NSE's dated per-contract band and limit-price-protection files and Dhan's option market-protection % (data), or the operator's named acceptance of the gap";
