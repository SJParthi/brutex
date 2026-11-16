//! The data rows the host reads: the dated session table (K14), the tick sizes, the corporate actions (K13) and the
//! NSE bhavcopy extract (official daily OHLC, D-S4). Every file is read whole and refused on any malformed row.

use std::collections::BTreeMap;
use std::path::Path;

use common_feat::ca::{CaBook, Event, Kind, Status};
use common_feat::cal::{at, parse_ymd, ymd};

use crate::ticks::parse_paisa;

fn read(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn rows(text: &str) -> impl Iterator<Item = Vec<&str>> {
    text.lines().filter(|l| !l.starts_with('#') && !l.trim().is_empty()).map(|l| l.split('\t').collect())
}

/// One session of the dated table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Session {
    /// IST day.
    pub day: i32,
    /// REGULAR or the special kind.
    pub kind: String,
    /// Segments `[open, end)` as epoch seconds.
    pub segs: Vec<(i64, i64)>,
    /// An entry session.
    pub entry: bool,
    /// A regular full session (relative-volume rings).
    pub regular_full: bool,
    /// Market-wide halt windows `[from, resume)` (epoch seconds) from the table's HALT rows (K11; plan 2.1 HALTS;
    /// `pull::calendar::HALTS`). A decision candle whose END lies inside one never enters (`decision_in_halt`).
    pub halts: Vec<(i64, i64)>,
}

impl Session {
    /// The session table's open (the first segment's start).
    #[must_use]
    pub fn open(&self) -> i64 {
        self.segs.first().map_or(0, |s| s.0)
    }
    /// The last segment's end.
    #[must_use]
    pub fn end(&self) -> i64 {
        self.segs.last().map_or(0, |s| s.1)
    }
    /// The end of the segment holding `t` (a candle's end is clamped to it), or `None` outside every segment.
    #[must_use]
    pub fn seg_end_of(&self, t: i64) -> Option<i64> {
        self.segs.iter().find(|s| t >= s.0 && t < s.1).map(|s| s.1)
    }

    /// The first segment that the interval `[start, end)` overlaps, as `(segment start, segment end)`, or `None` when
    /// it overlaps no segment. REPAIR 1 (check-build-feat-v1 B2): a vendor candle is placed on the session by OVERLAP,
    /// so a Zerodha candle that starts before a special session's open or in a DR gap but holds that session's trades
    /// is kept (SNIPER s13 (1), plan 2.1: "the grid is Zerodha's candles"; "a candle exists only if a trade falls in it").
    #[must_use]
    pub fn seg_overlapping(&self, start: i64, end: i64) -> Option<(i64, i64)> {
        self.segs.iter().find(|s| start < s.1 && end > s.0).copied()
    }

    /// True when the instant `t` lies inside one of the session's market-wide halts `[from, resume)`.
    #[must_use]
    pub fn in_halt(&self, t: i64) -> bool {
        common_feat::cal::in_halt(t, &self.halts)
    }

    /// Why a decision candle of this session ending at `end` may not enter: `None` when it may; otherwise the counted
    /// reason(s): `session_no_entry` (the session is never an entry session, K14 and plan 2.1), `decision_in_halt`
    /// (its END lies inside a market-wide halt, K11 / plan 2.1, REPAIR 1 B3), or both joined by `+`.
    #[must_use]
    pub fn refusal(&self, end: i64) -> Option<&'static str> {
        match (self.entry, self.in_halt(end)) {
            (true, false) => None,
            (true, true) => Some("decision_in_halt"),
            (false, false) => Some("session_no_entry"),
            (false, true) => Some("session_no_entry+decision_in_halt"),
        }
    }

    /// The decision candle ending at `end` may enter: an entry session and its END outside every halt. Every host
    /// reads this, never the bare session flag `entry`, for a decision.
    #[must_use]
    pub fn entry_at(&self, end: i64) -> bool {
        self.refusal(end).is_none()
    }
}

fn hms(s: &str) -> Option<i64> {
    let mut it = s.split(':');
    let h: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let x: i64 = it.next()?.parse().ok()?;
    Some(h * 3600 + m * 60 + x)
}

/// The dated session table: the session dates file plus the special rows. A row of kind `HALT` is not a session: its
/// window(s) `from-resume` (IST, resume exclusive) are the market-wide halts of that date's session (REPAIR 1, B3);
/// its entry column must be 0 (no decision ending inside it enters).
///
/// # Errors
///
/// A malformed date or row, a HALT row with entry 1, an empty halt window, or a HALT row on a day that is not a session.
pub fn sessions(dates: &Path, special: &Path) -> Result<Vec<Session>, String> {
    let mut sp: BTreeMap<i32, Session> = BTreeMap::new();
    let mut halts: BTreeMap<i32, Vec<(i64, i64)>> = BTreeMap::new();
    for r in rows(&read(special)?) {
        if r.first() == Some(&"date") {
            continue;
        }
        let [d, kind, segs, entry, full, ..] = r.as_slice() else {
            return Err(format!("special row {r:?}"));
        };
        let day = parse_ymd(d).ok_or_else(|| format!("special date {d}"))?;
        let mut v = Vec::new();
        for s in segs.split(',') {
            let (a, b) = s.split_once('-').ok_or_else(|| format!("segment {s}"))?;
            let (a, b) = (hms(a).ok_or("hms")?, hms(b).ok_or("hms")?);
            if a >= b {
                return Err(format!("empty window {s} on {d}"));
            }
            v.push((at(day, 0, 0, a), at(day, 0, 0, b)));
        }
        if *kind == "HALT" {
            if *entry != "0" {
                return Err(format!("HALT row {d}: entry must be 0"));
            }
            halts.entry(day).or_default().extend(v);
            continue;
        }
        if sp.contains_key(&day) {
            return Err(format!("two session rows for {d}"));
        }
        sp.insert(day, Session { day, kind: (*kind).to_owned(), segs: v, entry: *entry == "1", regular_full: *full == "1", halts: Vec::new() });
    }
    let mut out = Vec::new();
    for l in read(dates)?.lines().filter(|l| !l.trim().is_empty()) {
        let day = parse_ymd(l).ok_or_else(|| format!("session date {l}"))?;
        out.push(sp.remove(&day).unwrap_or_else(|| Session {
            day,
            kind: "REGULAR".into(),
            segs: vec![(at(day, 9, 15, 0), at(day, 15, 30, 0))],
            entry: true,
            regular_full: true,
            halts: Vec::new(),
        }));
    }
    if let Some((d, _)) = sp.iter().next() {
        return Err(format!("special row {} is not a session of the dates file", ymd(*d)));
    }
    out.sort_by_key(|s| s.day);
    for (d, w) in halts {
        let s = out.iter_mut().find(|s| s.day == d).ok_or_else(|| format!("HALT row {} is not a session of the dates file", ymd(d)))?;
        s.halts = w;
    }
    Ok(out)
}

/// The tick size (paisa) by date.
///
/// # Errors
///
/// A malformed row.
pub fn ticks(path: &Path) -> Result<Vec<(i32, i64)>, String> {
    let mut v = Vec::new();
    for r in rows(&read(path)?) {
        if r.first() == Some(&"from") {
            continue;
        }
        let d = parse_ymd(r.first().copied().unwrap_or("")).ok_or("tick date")?;
        let t: i64 = r.get(1).copied().unwrap_or("").parse().map_err(|_| "tick")?;
        v.push((d, t));
    }
    v.sort_unstable();
    Ok(v)
}

/// The tick on `day`.
#[must_use]
pub fn tick_on(t: &[(i32, i64)], day: i32) -> i64 {
    t.iter().rev().find(|x| x.0 <= day).map_or(5, |x| x.1)
}

fn ratio(s: &str) -> Option<(i128, i128)> {
    let (a, b) = s.split_once('/')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// The corporate actions.
///
/// # Errors
///
/// A malformed row.
pub fn ca(path: &Path) -> Result<CaBook, String> {
    let mut v = Vec::new();
    for r in rows(&read(path)?) {
        if r.first() == Some(&"day") {
            continue;
        }
        let [d, kind, f, g, status, kite, kite_vol, ..] = r.as_slice() else {
            return Err(format!("ca row {r:?}"));
        };
        let kind = match *kind {
            "BONUS" => Kind::Bonus,
            "SPLIT" => Kind::Split,
            "CONSOLIDATION" => Kind::Consolidation,
            "RIGHTS" => Kind::Rights,
            "DEMERGER" => Kind::Demerger,
            "SCHEME" => Kind::Scheme,
            "CAPITAL_REDUCTION" => Kind::CapitalReduction,
            "DIVIDEND" => Kind::Dividend,
            k => return Err(format!("ca kind {k}")),
        };
        let status = match *status {
            "PROVEN" => Status::Proven,
            "EX_DATE_UNKNOWN" => Status::ExDateUnknown,
            s => return Err(format!("ca status {s}")),
        };
        v.push(Event {
            day: parse_ymd(d).ok_or("ca day")?,
            kind,
            f: ratio(f).ok_or("ca f")?,
            g: ratio(g).ok_or("ca g")?,
            status,
            kite: if *kite == "NOT_REBASED" { None } else { Some(ratio(kite).ok_or("ca kite")?) },
            kite_vol: if *kite_vol == "NOT_REBASED" { None } else { Some(ratio(kite_vol).ok_or("ca kite_vol")?) },
        });
    }
    Ok(CaBook::new(v))
}

/// One bhavcopy EQ row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Bhav {
    /// Official open, paisa.
    pub open: i64,
    /// Official high.
    pub high: i64,
    /// Official low.
    pub low: i64,
    /// Official close (NSE's 15:00-15:30 weighted average; the closing auction from Aug 2026).
    pub close: i64,
    /// The last trade.
    pub last: i64,
    /// NSE's PREVCLOSE.
    pub prevclose: i64,
    /// Traded quantity.
    pub volume: u64,
}

fn bhav_date(s: &str) -> Option<i32> {
    const MON: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
    if let Some(d) = parse_ymd(s) {
        return Some(d);
    }
    // 01-APR-2019
    let mut it = s.split('-');
    let d: u32 = it.next()?.parse().ok()?;
    let mon = it.next()?;
    let m = MON.iter().position(|x| x.eq_ignore_ascii_case(mon))?;
    let y: i32 = it.next()?.parse().ok()?;
    // the extract holds one row dated `13-Jul-20` (two-digit year): 20 -> 2020
    let y = if y < 100 { 2000 + y } else { y };
    Some(common_feat::cal::days_from_civil(y, u32::try_from(m).ok()? + 1, d))
}

/// The bhavcopy extract (`scout/bhav-cm-adanient.tsv`): EQ rows by day.
///
/// # Errors
///
/// A malformed row.
pub fn bhav(path: &Path) -> Result<BTreeMap<i32, Bhav>, String> {
    let mut m = BTreeMap::new();
    for r in rows(&read(path)?) {
        if r.first() == Some(&"src") || r.get(3) != Some(&"EQ") {
            continue;
        }
        let g = |i: usize| r.get(i).copied().and_then(parse_paisa).ok_or_else(|| format!("bhav field {i} of {r:?}"));
        let day = bhav_date(r.get(2).copied().unwrap_or("")).ok_or_else(|| format!("bhav date {r:?}"))?;
        m.insert(day, Bhav { open: g(4)?, high: g(5)?, low: g(6)?, close: g(7)?, last: g(8)?, prevclose: g(9)?, volume: r.get(10).and_then(|v| v.parse().ok()).unwrap_or(0) });
    }
    Ok(m)
}

/// S4 (r3, wave 4; plan v3.1 N5 / v2 s4 L-CPR and C-GAP rows): the dated official daily rows of an INDEX
/// (`data/idx/INDEX-OFFICIAL-<IX>-v1.tsv`, written by `feat-run idx-official`): open = Zerodha's first 1-minute open of the
/// session (v2 s4 C-GAP: the 1day open is 1 paisa off on 51 days), high / low = the store's GDFL index seconds outside every HALT
/// window of the session (`ix_data.rs:3`'s rule), close = the dated official close of S1 (`INDEX-CLOSE-<IX>.tsv` column
/// `official`), each row naming its sources. Columns: `day open high low close prevclose ...` (paisa). The index carries no
/// volume (0) and its last trade is the official close. A row whose open / high / low / close is `-` is left out (the session
/// then has no official row: G-52H / G-52L unknown over it, the daily ATR resets, as for ADANIENT's holes).
///
/// # Errors
///
/// A malformed row.
pub fn bhav_index(path: &Path) -> Result<BTreeMap<i32, Bhav>, String> {
    let mut m = BTreeMap::new();
    for r in rows(&read(path)?) {
        if r.first() == Some(&"day") {
            continue;
        }
        let day = parse_ymd(r.first().copied().unwrap_or("")).ok_or_else(|| format!("index official day {r:?}"))?;
        let num = |i: usize| -> Result<Option<i64>, String> {
            match r.get(i).copied() {
                Some("-") => Ok(None),
                Some(x) => x.parse::<i64>().map(Some).map_err(|_| format!("index official field {i} of {r:?}")),
                None => Err(format!("index official row too short {r:?}")),
            }
        };
        let (Some(open), Some(high), Some(low), Some(close)) = (num(1)?, num(2)?, num(3)?, num(4)?) else { continue };
        let prevclose = num(5)?.unwrap_or(0);
        m.insert(day, Bhav { open, high, low, close, last: close, prevclose, volume: 0 });
    }
    Ok(m)
}
