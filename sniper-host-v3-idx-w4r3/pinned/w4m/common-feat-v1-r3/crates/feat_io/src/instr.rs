//! S4 (r3, wave 4; SNIPER-IDXOPT-PLAN-v3.1 N5): the instrument READ rows as data (`data/FEAT-INSTR-ROWS-v1.tsv`), so no reader
//! names ADANIENT, CASH or a span in code: the store segment and symbol, the read span, the dated session table and its special
//! rows, the corporate actions, the ticks, the official daily rows, the CONT start, the expiry calendar, the X-NIFTY / X-PEER and
//! X-VIX sources, the Muhurat rule (Q39), V-RVOL and the dated F&O-end rows. The ADANIENT row is r2's reads exactly
//! (`feat_io::defaults`, SESSIONS-SPECIAL-v1, CA-ADANIENT-v1, TICK-ADANIENT-v1, the CM extract, CONT 2018-09-07, the host's
//! census `EXPIRIES`), so a CASH load through the rows equals r2's [`crate::load_ctx`] (test `rows_cash_equals_r2_load`).
//!
//! Laws checked at load (each refused by name): an INDEX row must refuse V-RVOL (index candles carry volume 0: m-IX3), exclude the
//! Muhurat candles (Q39), read the DP-IDXCAL calendar and the index official rows, and carry no corporate action; X-PEER may not be
//! the row's own underlying (X-NIFTY on the NIFTY track duplicates P-CHG: refused); a CASH row reads the census calendar.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use common_feat::cal::parse_ymd;

use crate::host::{Ctx, ZRead};

/// The official daily rows' kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BhavKind {
    /// NSE's CM bhavcopy extract (`scout/bhav-cm-adanient.tsv`, [`crate::data::bhav`]).
    CmExtract,
    /// The index official rows (`data/idx/INDEX-OFFICIAL-<IX>-v1.tsv`, [`crate::data::bhav_index`]).
    IndexOfficial,
}

/// The expiry calendar's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalKind {
    /// The store's GDFL FNO chain census of a stock (the host's `ctxdials::EXPIRIES`: monthly expiry + first traded instant).
    Census,
    /// S1's DP-IDXCAL rows (the dated NSE expiry calendar: true expiry days, as known before each session).
    IdxCal,
}

/// The X-NIFTY / X-PEER previous-close source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeerClose {
    /// The peer's Zerodha 1day close (the host's X-NIFTY on ADANIENT).
    Zerodha1day,
    /// The peer's official close (its own row's index official file; v2 s4: Zerodha 1day is never the index close source).
    Official,
}

/// One instrument's read rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstrRow {
    /// The underlying (`ADANIENT`, `NIFTY`, `BANKNIFTY`).
    pub underlying: String,
    /// The store segment of its candles (`CASH`, `INDEX`).
    pub segment: String,
    /// The store symbol of its Zerodha candles.
    pub zsymbol: String,
    /// The first month read.
    pub from: (u16, u8),
    /// The last month read.
    pub to: (u16, u8),
    /// The session dates file.
    pub dates: PathBuf,
    /// The special session rows.
    pub special: PathBuf,
    /// The corporate actions.
    pub ca: PathBuf,
    /// The tick sizes.
    pub tick: PathBuf,
    /// The official daily rows' kind.
    pub bhav_kind: BhavKind,
    /// The official daily rows.
    pub bhav: PathBuf,
    /// The first session any candle reading reads.
    pub cont_start: i32,
    /// The expiry calendar's kind.
    pub cal_kind: CalKind,
    /// The expiry calendar.
    pub cal: PathBuf,
    /// The X-NIFTY (CASH) / X-PEER (INDEX) source symbol.
    pub peer: String,
    /// Its previous-close source.
    pub peer_close: PeerClose,
    /// The X-VIX source symbol.
    pub vix: String,
    /// Q39: Muhurat candles dropped from every signal stream.
    pub muhurat_excluded: bool,
    /// V-RVOL may be ON (a source with volume).
    pub rvol: bool,
    /// The dated F&O-end rows (`None`: not an option underlying of this lane).
    pub fno_end: Option<PathBuf>,
}

fn path_of(v: &str, data: &Path) -> Option<PathBuf> {
    if v == "-" || v.is_empty() {
        None
    } else if v.starts_with('/') {
        Some(PathBuf::from(v))
    } else {
        Some(data.join(v))
    }
}

fn ym(s: &str) -> Option<(u16, u8)> {
    let (y, m) = s.split_once('-')?;
    let (y, m): (u16, u8) = (y.parse().ok()?, m.parse().ok()?);
    (1..=12).contains(&m).then_some((y, m))
}

/// Parses the rows; relative paths resolve against `data` (the crate's data folder).
///
/// # Errors
/// A malformed row, a duplicate underlying, or a law refused by name (module doc).
pub fn parse_rows(text: &str, data: &Path) -> Result<BTreeMap<String, InstrRow>, String> {
    let mut head: Vec<&str> = Vec::new();
    let mut out = BTreeMap::new();
    for l in text.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        if c.first() == Some(&"underlying") {
            head = c;
            continue;
        }
        let g = |n: &str| -> Result<&str, String> {
            let k = head.iter().position(|h| *h == n).ok_or_else(|| format!("FEAT-INSTR-ROWS: no column {n}"))?;
            c.get(k).copied().ok_or_else(|| format!("FEAT-INSTR-ROWS: row too short: {l}"))
        };
        let p = |n: &str| -> Result<PathBuf, String> { path_of(g(n)?, data).ok_or_else(|| format!("FEAT-INSTR-ROWS {}: {n} missing", c.first().copied().unwrap_or(""))) };
        let row = InstrRow {
            underlying: g("underlying")?.to_owned(),
            segment: g("segment")?.to_owned(),
            zsymbol: g("zerodha_symbol")?.to_owned(),
            from: ym(g("span_from")?).ok_or("span_from")?,
            to: ym(g("span_to")?).ok_or("span_to")?,
            dates: p("dates")?,
            special: p("special")?,
            ca: p("ca")?,
            tick: p("tick")?,
            bhav_kind: match g("bhav_kind")? {
                "cm_extract" => BhavKind::CmExtract,
                "index_official" => BhavKind::IndexOfficial,
                k => return Err(format!("FEAT-INSTR-ROWS: bhav_kind {k}")),
            },
            bhav: p("bhav")?,
            cont_start: parse_ymd(g("cont_start")?).ok_or("cont_start")?,
            cal_kind: match g("cal_kind")? {
                "census" => CalKind::Census,
                "idxcal" => CalKind::IdxCal,
                k => return Err(format!("FEAT-INSTR-ROWS: cal_kind {k}")),
            },
            cal: p("cal")?,
            peer: g("peer")?.to_owned(),
            peer_close: match g("peer_close")? {
                "zerodha_1day" => PeerClose::Zerodha1day,
                "official" => PeerClose::Official,
                k => return Err(format!("FEAT-INSTR-ROWS: peer_close {k}")),
            },
            vix: g("vix")?.to_owned(),
            muhurat_excluded: match g("muhurat_signals")? {
                "excluded" => true,
                "kept" => false,
                k => return Err(format!("FEAT-INSTR-ROWS: muhurat_signals {k}")),
            },
            rvol: match g("rvol")? {
                "on" => true,
                "refused" => false,
                k => return Err(format!("FEAT-INSTR-ROWS: rvol {k}")),
            },
            fno_end: path_of(g("fno_end")?, data),
        };
        law(&row)?;
        if out.insert(row.underlying.clone(), row).is_some() {
            return Err(format!("FEAT-INSTR-ROWS: two rows for {}", c.first().copied().unwrap_or("")));
        }
    }
    Ok(out)
}

/// The load laws (module doc), each refused by name.
///
/// # Errors
/// The first law the row breaks.
pub fn law(r: &InstrRow) -> Result<(), String> {
    let u = &r.underlying;
    if r.peer == r.underlying || r.peer == r.zsymbol {
        return Err(format!("{u}: X-PEER reads the row's own underlying (X-NIFTY on the NIFTY track duplicates P-CHG): refused"));
    }
    match r.segment.as_str() {
        "INDEX" => {
            // m-IX3: V-RVOL ON for a volume-0 source must be refused (the named mutant accepts it)
            if r.rvol && !cfg!(feature = "m_ix3") {
                return Err(format!("{u}: V-RVOL on an INDEX row (index candles carry volume 0): refused (m-IX3)"));
            }
            if !r.muhurat_excluded {
                return Err(format!("{u}: an INDEX row must exclude the Muhurat candles (Q39): refused"));
            }
            if r.cal_kind != CalKind::IdxCal || r.bhav_kind != BhavKind::IndexOfficial {
                return Err(format!("{u}: an INDEX row reads DP-IDXCAL and the index official rows: refused"));
            }
            if r.fno_end.is_none() {
                return Err(format!("{u}: an INDEX option row needs the dated F&O-end rows (F13): refused"));
            }
        }
        "CASH" => {
            if r.cal_kind != CalKind::Census || r.bhav_kind != BhavKind::CmExtract {
                return Err(format!("{u}: a CASH row reads the census calendar and the CM extract: refused"));
            }
        }
        s => return Err(format!("{u}: segment {s} has no read rows: refused")),
    }
    Ok(())
}

/// The V-RVOL dial with value `on` on this row: refused by name when the row's source carries no volume (m-IX3).
///
/// # Errors
/// V-RVOL ON on a row whose `rvol` is `refused`.
pub fn rvol_check(r: &InstrRow, on: bool) -> Result<(), String> {
    if on && !r.rvol && !cfg!(feature = "m_ix3") {
        return Err(format!("V-RVOL ON for {} ({} candles carry volume 0): refused (m-IX3)", r.underlying, r.segment));
    }
    Ok(())
}

impl InstrRow {
    /// The row's Zerodha read.
    #[must_use]
    pub fn zread(&self) -> ZRead<'_> {
        ZRead { segment: &self.segment, symbol: &self.zsymbol, from: self.from, to: self.to, muhurat_excluded: self.muhurat_excluded }
    }

    /// The row's context ([`Ctx`]): its session table, corporate actions, ticks, official rows and CONT start.
    ///
    /// # Errors
    /// Any malformed input.
    pub fn load_ctx(&self) -> Result<Ctx, String> {
        Ok(Ctx {
            sessions: crate::data::sessions(&self.dates, &self.special)?,
            ca: crate::data::ca(&self.ca)?,
            ticks: crate::data::ticks(&self.tick)?,
            bhav: match self.bhav_kind {
                BhavKind::CmExtract => crate::data::bhav(&self.bhav)?,
                BhavKind::IndexOfficial => crate::data::bhav_index(&self.bhav)?,
            },
            cont_start: self.cont_start,
        })
    }
}

/// The rows file of this crate (`data/FEAT-INSTR-ROWS-v1.tsv`).
///
/// # Errors
/// An unreadable or refused rows file.
pub fn load_default() -> Result<BTreeMap<String, InstrRow>, String> {
    let d = crate::data_dir();
    let p = d.join("FEAT-INSTR-ROWS-v1.tsv");
    parse_rows(&std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?, &d)
}
