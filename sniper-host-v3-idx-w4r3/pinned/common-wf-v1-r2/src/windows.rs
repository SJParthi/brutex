//! Windows as DATA (P3-P5, 4.1): `WINDOWS-*.tsv` rows, validated V1-V14 and fail closed; the run's window SETS (at most 32 pieces
//! each, at most 8 sets); per set the PIECES (cut at every boundary of every row in the set: the prefix snapshots live there) and
//! the YEAR UNITS.
//!
//! DV1 (builder, 13 Nov 2026, for MAIN's confirmation; CHANGES.md): a window's year parts are its OWN months grouped by calendar
//! year, cut only by its own boundaries. P5's shared "calendar year inside a piece" lets ANOTHER row's cut split a window's year
//! (vector V59: monthly rows turned WF's "every year positive" into "every month positive"). With the standard rows every cut is a
//! January, a period edge or the row's own boundary, so units equal the plan's parts there and nothing changes.

use std::sync::Arc;

use crate::cal::Calendar;
use crate::date::{mk_m, mk_y, mon, parse_mk};
use crate::rules::{RuleProvenance, parse_prov};
use crate::{MAX_PARTS, MAX_PIECES, MAX_SET_WINDOWS, MAX_SETS, Mk, SetId, WfErr, WinId};

/// The columns of a WINDOWS file, in order (4.1 with repair 3's `provenance`, `gap_words` and A3's `fixed_after`; the builder adds
/// `protocol` (the W04 / W05 pick-protocol word) and `follows` (V8: the split row a paper row follows)).
pub const WIN_COLS: &[&str] = &["id", "kind", "honest", "protocol", "test_name", "pick_label", "learn_from", "learn_to", "test_from", "test_to", "gap_months", "follows", "owner_words", "role", "label", "provenance", "gap_words", "fixed_after"];
const WORD_COLS: &[usize] = &[4, 5, 12, 14, 16, 17, 3, 13];

/// A row's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinKind {
    /// A split window (learning part, test part).
    Split,
    /// The hindsight window (all months).
    Hindsight,
    /// A way-A row (labelled comparison only).
    WayA,
    /// The PAPER row (appended months).
    Paper,
}

/// Why a row is not run in this run (P3, W08).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotRunWhy {
    /// A learning month lies outside the period.
    Learning,
    /// A test month lies outside the period.
    Test,
    /// The row alone needs more than 32 pieces.
    SetTooBig,
}

/// One WINDOWS row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinRow {
    /// Its index in the file.
    pub idx: u8,
    /// The id (WF, TT, HS, A2020, ...).
    pub id: String,
    /// The kind.
    pub kind: WinKind,
    /// An honest row (only a split row): it joins the method line (P19).
    pub honest: bool,
    /// The pick protocol word (W04, W05): "walk-forward", "train/test", ...
    pub protocol: String,
    /// `{T}`: "Walk-forward test", "Train/test", ...
    pub test_name: String,
    /// The pick label (W19, W12).
    pub pick_label: String,
    /// The learning part.
    pub learn: Option<(Mk, Mk)>,
    /// The first test month.
    pub test_from: Option<Mk>,
    /// The last test month (None for an open PAPER row).
    pub test_to: Option<Mk>,
    /// Months between learning and test, in neither part.
    pub gap: i32,
    /// The split row a PAPER row follows.
    pub follows: String,
    /// The owner template (W04, W08), e.g. "train months {learn}".
    pub owner_words: String,
    /// The row's role text.
    pub role: String,
    /// The row's label.
    pub label: String,
    /// The signed provenance of an honest split row (Z1, A3, D56), or of any row that carries one.
    pub prov: Option<RuleProvenance>,
    /// Why the gap months are in neither part (Z2).
    pub gap_words: String,
    /// A3: the words printed beside W15 on every pick of this row (and once in W12b), or "-".
    pub fixed_after: String,
}

pub(crate) fn has_literal(s: &str) -> bool {
    let b = s.as_bytes();
    let dig = |i: usize| b.get(i).is_some_and(u8::is_ascii_digit);
    for i in 0..b.len() {
        if (0..4).all(|k| dig(i + k)) {
            if b.get(i + 4) == Some(&b'-') && dig(i + 5) && dig(i + 6) {
                return true;
            }
            if b.get(i + 4) == Some(&b'.') && b.get(i + 5) == Some(&b'.') && (6..10).all(|k| dig(i + k)) {
                return true;
            }
            let before = i == 0 || !dig(i - 1);
            let after = !dig(i + 4);
            let century = matches!((b.get(i), b.get(i + 1)), (Some(b'1'), Some(b'9')) | (Some(b'2'), Some(b'0')));
            if before && after && century {
                return true;
            }
        }
    }
    false
}

/// Parses and validates a WINDOWS file (V1-V3, V6-V8, V10, V11, V13, V14; V4, V5, V9, V12 at the set build).
///
/// # Errors
/// `BadRow(W, row, rule)`.
pub fn parse_windows(text: &str) -> Result<Vec<WinRow>, WfErr> {
    let bad = |row: usize, rule: u8| WfErr::BadRow { file: b'W', row: u32::try_from(row).unwrap_or(u32::MAX), rule };
    let mut lines = text.lines().filter(|l| !l.is_empty());
    if lines.next() != Some(WIN_COLS.join("\t").as_str()) {
        return Err(bad(0, 6));
    }
    let mut rows: Vec<WinRow> = Vec::new();
    for (i, l) in lines.enumerate() {
        let n = i + 1;
        let c: Vec<&str> = l.split('\t').collect();
        if c.len() != WIN_COLS.len() {
            return Err(bad(n, 6));
        }
        let col = |k: usize| c.get(k).copied().unwrap_or("");
        let kind = match col(1) {
            "split" => WinKind::Split,
            "hindsight" => WinKind::Hindsight,
            "way-a" => WinKind::WayA,
            "paper" => WinKind::Paper,
            _ => return Err(bad(n, 6)),
        };
        let honest = match col(2) {
            "yes" => true,
            "no" => false,
            _ => return Err(bad(n, 6)),
        };
        let gap: i32 = if !col(10).is_empty() && col(10).bytes().all(|b| b.is_ascii_digit()) { col(10).parse().map_err(|_| bad(n, 2))? } else { return Err(bad(n, 2)) };
        let m = |s: &str| -> Result<Option<Mk>, WfErr> { if s == "-" { Ok(None) } else { parse_mk(s).map(Some).ok_or_else(|| bad(n, 2)) } };
        let lf = m(col(6))?;
        let lt = m(col(7))?;
        let tf = m(col(8))?;
        let open = col(9) == "open";
        let tt = if open { None } else { m(col(9))? };
        if lf.is_some() != lt.is_some() || matches!((lf, lt), (Some(a), Some(b)) if a > b) || matches!((tf, tt), (Some(a), Some(b)) if a > b) {
            return Err(bad(n, 2));
        }
        if matches!(kind, WinKind::Split | WinKind::WayA) && (lf.is_none() || tf.is_none() || tt.is_none()) {
            return Err(bad(n, 2));
        }
        if kind == WinKind::Hindsight && (lf.is_none() || tf.is_some()) {
            return Err(bad(n, 2));
        }
        if kind == WinKind::Paper && (tf.is_none() || !open || lf.is_some()) {
            return Err(bad(n, 2));
        }
        if kind != WinKind::Paper && open {
            return Err(bad(n, 2));
        }
        if honest && kind != WinKind::Split {
            return Err(bad(n, 11));
        }
        if kind == WinKind::Split
            && let (Some(t), Some(l)) = (tf, lt)
            && t != l + 1 + gap
        {
            return Err(bad(n, 3));
        }
        if WORD_COLS.iter().any(|&k| has_literal(col(k))) {
            return Err(bad(n, 10));
        }
        let prov = if col(15) == "-" { None } else { Some(parse_prov(col(15)).ok_or_else(|| bad(n, 14))?) };
        if honest && matches!(prov, None | Some(RuleProvenance::Chosen { .. })) {
            // V14 (Z1) with F4's first fix: an honest row needs a signed provenance, and CHOSEN never sits on an honest row.
            return Err(bad(n, 14));
        }
        if kind == WinKind::Paper {
            let f = rows.iter().find(|x| x.id == col(11));
            let Some(f) = f.filter(|f| f.kind == WinKind::Split) else { return Err(bad(n, 8)) };
            if tf != f.test_to.map(|t| t + 1 + gap) {
                return Err(bad(n, 13));
            }
        }
        if rows.iter().any(|x| x.id == col(0)) {
            return Err(bad(n, 1));
        }
        rows.push(WinRow {
            idx: u8::try_from(i).map_err(|_| bad(n, 5))?,
            id: col(0).to_owned(),
            kind,
            honest,
            protocol: col(3).to_owned(),
            test_name: col(4).to_owned(),
            pick_label: col(5).to_owned(),
            learn: lf.zip(lt),
            test_from: tf,
            test_to: tt,
            gap,
            follows: col(11).to_owned(),
            owner_words: col(12).to_owned(),
            role: col(13).to_owned(),
            label: col(14).to_owned(),
            prov,
            gap_words: col(16).to_owned(),
            fixed_after: col(17).to_owned(),
        });
    }
    if !rows.iter().any(|r| r.kind == WinKind::Split && r.honest) {
        return Err(bad(0, 7));
    }
    Ok(rows)
}

/// A year unit of a window set (DV1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unit {
    /// Its first month.
    pub from: Mk,
    /// Its last month.
    pub to: Mk,
    /// `part_label`: "YYYY" when whole or partial only at the period edge, else "YYYY Mon-Mon" (4.3).
    pub label: String,
}

/// One window's masks inside its set.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WinSpec {
    /// Its index among the set's windows.
    pub local: u8,
    /// Its kind.
    pub kind: WinKind,
    /// The learning months.
    pub learn: Option<(Mk, Mk)>,
    /// The test months (PAPER: to the period end).
    pub test: Option<(Mk, Mk)>,
    /// The learning pieces.
    pub learn_pieces: u32,
    /// The test pieces.
    pub test_pieces: u32,
    /// The piece that ends the learning part (the prefix point).
    pub learn_end_piece: Option<u8>,
    /// The learning part starts at the period's first month: one prefix cell (B5 (b)).
    pub anchored: bool,
    /// The learning units.
    pub learn_units: u64,
    /// The test units.
    pub test_units: u64,
    /// The learning units in month order.
    pub learn_list: Vec<u8>,
    /// The test units in month order.
    pub test_list: Vec<u8>,
}

/// Everything static of one window set, shared by every `Acc` of the set.
#[derive(Debug, PartialEq, Eq)]
pub struct SetTab {
    /// The set.
    pub set: SetId,
    /// The run's period.
    pub period: (Mk, Mk),
    /// The calendar.
    pub cal: Arc<Calendar>,
    /// Every row of the WINDOWS file.
    pub rows: Arc<Vec<WinRow>>,
    /// The pieces (first, last month).
    pub pieces: Vec<(Mk, Mk)>,
    /// The year units.
    pub units: Vec<Unit>,
    /// Per period month: its piece.
    pub month_piece: Vec<u8>,
    /// Per period month: its part (piece x calendar year), the accumulation granularity.
    pub month_part: Vec<u16>,
    /// Per part: the units holding it.
    pub part_units: Vec<u64>,
    /// Per window row (global index): its spec, if it is in this set.
    pub spec: Vec<Option<WinSpec>>,
    /// The set's windows.
    pub members: Vec<WinId>,
    /// Per unit: the local windows whose LEARNING units hold it (the MISS money gap, F9).
    pub unit_learn_windows: Vec<u64>,
    /// Per unit: no walked session in the calendar ("no session in the data", P8).
    pub unit_missing: u64,
    /// The identity hash of the set (windows, calendar, period).
    pub ident: u64,
}

/// One window set (P5): what `Acc`, `Lists` and `verdict` read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Windows {
    /// The shared tables.
    pub tab: Arc<SetTab>,
}

/// The run's window sets and the rows not run.
#[derive(Clone, Debug)]
pub struct WindowSets {
    /// The sets, in file order.
    pub sets: Vec<Windows>,
    /// The rows not run, with their reason (W08).
    pub not_run: Vec<(WinId, NotRunWhy)>,
    /// Every row.
    pub rows: Arc<Vec<WinRow>>,
}

fn row_test(r: &WinRow, period: (Mk, Mk)) -> Option<(Mk, Mk)> {
    let tf = r.test_from?;
    Some((tf, if r.kind == WinKind::Paper { period.1 } else { r.test_to? }))
}
fn row_cuts(r: &WinRow, period: (Mk, Mk)) -> Vec<Mk> {
    let mut c = Vec::with_capacity(4);
    if let Some((a, b)) = r.learn {
        c.push(a);
        c.push(b + 1);
    }
    if let Some((a, b)) = row_test(r, period) {
        c.push(a);
        c.push(b + 1);
    }
    c
}
fn not_run(r: &WinRow, (p0, p1): (Mk, Mk)) -> Option<NotRunWhy> {
    if r.kind == WinKind::Paper {
        return if r.test_from.is_some_and(|t| t <= p1) { None } else { Some(NotRunWhy::Test) };
    }
    if r.learn.is_some_and(|(a, b)| a < p0 || b > p1) {
        return Some(NotRunWhy::Learning);
    }
    if row_test(r, (p0, p1)).is_some_and(|(a, b)| a < p0 || b > p1) {
        return Some(NotRunWhy::Test);
    }
    None
}
fn cuts_of(rows: &[&WinRow], period: (Mk, Mk)) -> Vec<Mk> {
    let mut s = vec![period.0, period.1 + 1];
    for r in rows {
        for c in row_cuts(r, period) {
            if c >= period.0 && c <= period.1 + 1 {
                s.push(c);
            }
        }
    }
    s.sort_unstable();
    s.dedup();
    s
}
fn years_of(a: Mk, b: Mk) -> Vec<(Mk, Mk)> {
    let mut out = Vec::new();
    let mut x = a;
    while x <= b {
        let e = b.min(mk_y(x) * 12 + 11);
        out.push((x, e));
        x = e + 1;
    }
    out
}

impl Windows {
    /// Builds the run's window sets (P5, F7): rows in file order; a row that would make the current set exceed 32 pieces (or 64
    /// windows) starts a new set; a row that alone needs more than 32 pieces is NOT RUN; at most 8 sets.
    ///
    /// # Errors
    /// `TooManySets`, `TooManyParts`.
    pub fn sets_from_rows(rows: &[WinRow], cal: &Calendar, period: (Mk, Mk)) -> Result<WindowSets, WfErr> {
        let mut not = Vec::new();
        let mut groups: Vec<Vec<&WinRow>> = Vec::new();
        let mut cur: Vec<&WinRow> = Vec::new();
        for r in rows {
            if let Some(w) = not_run(r, period) {
                not.push((WinId(r.idx), w));
                continue;
            }
            if cuts_of(&[r], period).len() - 1 > MAX_PIECES {
                not.push((WinId(r.idx), NotRunWhy::SetTooBig));
                continue;
            }
            let mut probe = cur.clone();
            probe.push(r);
            if !cur.is_empty() && (cuts_of(&probe, period).len() - 1 > MAX_PIECES || probe.len() > MAX_SET_WINDOWS) {
                groups.push(std::mem::take(&mut cur));
            }
            cur.push(r);
        }
        if !cur.is_empty() {
            groups.push(cur);
        }
        if groups.len() > MAX_SETS {
            return Err(WfErr::TooManySets);
        }
        let all = Arc::new(rows.to_vec());
        let cal = Arc::new(cal.clone());
        let mut sets = Vec::with_capacity(groups.len());
        for (si, g) in groups.iter().enumerate() {
            sets.push(Self { tab: Arc::new(make_tab(SetId(u8::try_from(si).map_err(|_| WfErr::TooManySets)?), g, period, &cal, &all)?) });
        }
        Ok(WindowSets { sets, not_run: not, rows: all })
    }
    /// The set's id.
    #[must_use]
    pub fn set_id(&self) -> SetId {
        self.tab.set
    }
    /// Is window `w` in this set?
    #[must_use]
    pub fn has(&self, w: WinId) -> bool {
        self.spec(w).is_some()
    }
    /// Window `w`'s spec in this set.
    #[must_use]
    pub fn spec(&self, w: WinId) -> Option<&WinSpec> {
        self.tab.spec.get(usize::from(w.0)).and_then(Option::as_ref)
    }
    /// The learning months of `w` (P21's read-set test).
    #[must_use]
    pub fn learn_range(&self, w: WinId) -> Option<(Mk, Mk)> {
        self.spec(w).and_then(|s| s.learn)
    }
    /// The test months of `w`.
    #[must_use]
    pub fn test_range(&self, w: WinId) -> Option<(Mk, Mk)> {
        self.spec(w).and_then(|s| s.test)
    }
    /// The learning units of `w`.
    #[must_use]
    pub fn learn_parts(&self, w: WinId) -> crate::PartMask {
        crate::PartMask(self.spec(w).map_or(0, |s| s.learn_units))
    }
    /// The test units of `w`.
    #[must_use]
    pub fn test_parts(&self, w: WinId) -> crate::PartMask {
        crate::PartMask(self.spec(w).map_or(0, |s| s.test_units))
    }
    /// Units with no walked session in the calendar.
    #[must_use]
    pub fn missing_parts(&self) -> crate::PartMask {
        crate::PartMask(self.tab.unit_missing)
    }
    /// Is `w` anchored (its learning part starts at the period's first month)?
    #[must_use]
    pub fn anchored(&self, w: WinId) -> bool {
        self.spec(w).is_some_and(|s| s.anchored)
    }
    /// The row of `w`.
    #[must_use]
    pub fn row(&self, w: WinId) -> Option<&WinRow> {
        self.tab.rows.get(usize::from(w.0))
    }
    /// The honest split rows of this set (P19).
    #[must_use]
    pub fn honest(&self) -> Vec<WinId> {
        self.tab.members.iter().copied().filter(|w| self.row(*w).is_some_and(|r| r.kind == WinKind::Split && r.honest)).collect()
    }
}

impl WindowSets {
    /// The honest split rows of EVERY set, in file order (P19, Y5).
    #[must_use]
    pub fn honest(&self) -> Vec<WinId> {
        let mut h: Vec<WinId> = self.sets.iter().flat_map(Windows::honest).collect();
        h.sort_unstable();
        h
    }
    /// The row of `w` (the caller passes an id of this file; a foreign id gives the first row).
    #[must_use]
    pub fn row(&self, w: WinId) -> &WinRow {
        static EMPTY: std::sync::OnceLock<WinRow> = std::sync::OnceLock::new();
        self.rows.get(usize::from(w.0)).unwrap_or_else(|| EMPTY.get_or_init(empty_row))
    }
}
fn empty_row() -> WinRow {
    WinRow { idx: u8::MAX, id: String::new(), kind: WinKind::Hindsight, honest: false, protocol: String::new(), test_name: String::new(), pick_label: String::new(), learn: None, test_from: None, test_to: None, gap: 0, follows: String::new(), owner_words: String::new(), role: String::new(), label: String::new(), prov: None, gap_words: String::new(), fixed_after: String::new() }
}

fn make_tab(set: SetId, rs: &[&WinRow], period: (Mk, Mk), cal: &Arc<Calendar>, all: &Arc<Vec<WinRow>>) -> Result<SetTab, WfErr> {
    let cuts = cuts_of(rs, period);
    let pieces: Vec<(Mk, Mk)> = cuts.windows(2).filter_map(|w| Some((*w.first()?, *w.get(1)? - 1))).collect();
    if pieces.len() > MAX_PIECES {
        return Err(WfErr::TooManyPieces);
    }
    let nm = usize::try_from(period.1 - period.0 + 1).map_err(|_| WfErr::BadDay)?;
    let mut month_piece = vec![0u8; nm];
    let mut month_part = vec![0u16; nm];
    let mut parts: Vec<(Mk, Mk)> = Vec::new();
    for (pi, &(a, b)) in pieces.iter().enumerate() {
        for (ya, yb) in years_of(a, b) {
            parts.push((ya, yb));
            for mk in ya..=yb {
                let i = usize::try_from(mk - period.0).map_err(|_| WfErr::BadDay)?;
                if let Some(x) = month_piece.get_mut(i) {
                    *x = u8::try_from(pi).map_err(|_| WfErr::TooManyPieces)?;
                }
                if let Some(x) = month_part.get_mut(i) {
                    *x = u16::try_from(parts.len() - 1).map_err(|_| WfErr::TooManyParts)?;
                }
            }
        }
    }
    let mut units: Vec<Unit> = Vec::new();
    let mut unit_of = |a: Mk, b: Mk| -> Result<u8, WfErr> {
        if let Some(i) = units.iter().position(|u| u.from == a && u.to == b) {
            return u8::try_from(i).map_err(|_| WfErr::TooManyParts);
        }
        let whole = (mk_m(a) == 1 || a == period.0) && (mk_m(b) == 12 || b == period.1);
        let label = if whole { format!("{}", mk_y(a)) } else { format!("{} {}-{}", mk_y(a), mon(a), mon(b)) };
        units.push(Unit { from: a, to: b, label });
        u8::try_from(units.len() - 1).map_err(|_| WfErr::TooManyParts)
    };
    let piece_mask = |r: (Mk, Mk)| -> u32 { pieces.iter().enumerate().filter(|(_, p)| p.0 >= r.0 && p.1 <= r.1).fold(0u32, |m, (i, _)| m | (1u32 << i)) };
    let mut spec: Vec<Option<WinSpec>> = vec![None; all.len()];
    let mut members = Vec::with_capacity(rs.len());
    for (local, r) in rs.iter().enumerate() {
        let learn = r.learn;
        let test = row_test(r, period);
        let mut learn_list = Vec::new();
        let mut test_list = Vec::new();
        if let Some((a, b)) = learn {
            for (x, y) in years_of(a, b) {
                learn_list.push(unit_of(x, y)?);
            }
        }
        if let Some((a, b)) = test {
            for (x, y) in years_of(a, b) {
                test_list.push(unit_of(x, y)?);
            }
        }
        let lp = learn.map_or(0, piece_mask);
        let s = WinSpec {
            local: u8::try_from(local).map_err(|_| WfErr::TooManySets)?,
            kind: r.kind,
            learn,
            test,
            learn_pieces: lp,
            test_pieces: test.map_or(0, piece_mask),
            learn_end_piece: if lp == 0 { None } else { u8::try_from(31 - lp.leading_zeros()).ok() },
            anchored: learn.is_some_and(|(a, _)| a == period.0),
            learn_units: learn_list.iter().fold(0u64, |m, &u| m | (1u64 << u)),
            test_units: test_list.iter().fold(0u64, |m, &u| m | (1u64 << u)),
            learn_list,
            test_list,
        };
        if let Some(x) = spec.get_mut(usize::from(r.idx)) {
            *x = Some(s);
        }
        members.push(WinId(r.idx));
    }
    if units.len() > MAX_PARTS {
        return Err(WfErr::TooManyParts);
    }
    let part_units = parts.iter().map(|&(a, b)| units.iter().enumerate().filter(|(_, u)| u.from <= a && b <= u.to).fold(0u64, |m, (i, _)| m | (1u64 << i))).collect();
    let unit_learn_windows = (0..units.len())
        .map(|u| spec.iter().flatten().filter(|s| s.learn_units & (1u64 << u) != 0).fold(0u64, |m, s| m | (1u64 << s.local)))
        .collect();
    let unit_missing = units.iter().enumerate().filter(|(_, u)| cal.walked_in(crate::date::first_day(u.from), crate::date::last_day(u.to)) == 0).fold(0u64, |m, (i, _)| m | (1u64 << i));
    let mut ident = crate::mix(cal.hash() ^ (u64::from(period.0.unsigned_abs()) << 20) ^ u64::from(period.1.unsigned_abs()));
    for r in rs {
        ident = crate::mix(ident ^ crate::text_hash(&format!("{r:?}")));
    }
    Ok(SetTab { set, period, cal: Arc::clone(cal), rows: Arc::clone(all), pieces, units, month_piece, month_part, part_units, spec, members, unit_learn_windows, unit_missing, ident })
}
