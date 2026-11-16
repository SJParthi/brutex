//! CRATE v2.3 (spikefade v3.1, 15 Nov 2026; MAIN-RULINGS-ENGINE-v22 item 1 and addendum 2): THE ONE MARGIN MODULE of Brutex. Every strategy
//! (spike selling, index options, positional, plain selling, the s26 portfolio) reads the margin of a SHORT option position here; no host
//! keeps its own copy (addendum 2, "one common runtime").
//!
//! THE RULE (margin-truth `MARGIN-TRUTH-8DAY.md` section 3 with the page fix r2; the rule that rebuilds the operator's Dhan screen, GRASIM
//! 16 Nov 2026, to Rs 21.14), all in INTEGER PAISA for q units:
//! - SPAN = span_unit_short x q (exact: NSE prints the risk-array value to 2 decimals);
//! - premium term = file_price x q (exact; the option's value at NSE's file price, the term the Dhan calibration needs);
//! - ELM = rate x S x q rounded ONCE to the paisa, half away from zero, with S the live spot at the decision (paisa) and the rate in
//!   hundredths of a percent: INDEX 2% (+2% on the option's own expiry day from 2024-11-20, SEBI/HO/MRD/TPD-1/P/CIR/2024/132 para 5.6);
//!   STOCK the AEL "Total applicable ELM%" of the OTM row when the option is out of the money at S (CALL K > S, PUT K < S), else the OTH
//!   row;
//! - exact = SPAN + premium term + ELM; NSE-only reading = SPAN + ELM (printed beside).
//!
//! THE TABLE: margin-history `MARGIN-TABLE-FORMAT.md` (one session's begin-of-day SPAN file, or one intraday snapshot, as tab-separated text
//! with a header row). [`Snapshot::parse`] reads the DECOMPRESSED bytes (the host runs `zstd -dc`; this crate does no I/O). Every refused
//! row is counted by reason ([`ParseCounts`]), never silent.
//!
//! THE INDEX (O(1)): one FLAT open-addressing table per snapshot over a packed u64 key (underlying id, expiry day, right, strike paisa);
//! capacity a power of two at least twice the rows, linear probing, the longest probe chain measured at build ([`Snapshot::max_probe`]) and
//! every lookup's probes returned, so a host prints p50 / p99 per lookup kind. A host resolves each contract-day ONCE (key -> row) and then
//! reads row offsets by contract index (a flat array): O(1) per decision.
//!
//! Rules: std only, no I/O, no float, checked arithmetic (a refusal, never a wrap).

use std::collections::HashMap;

/// The index ELM rate, hundredths of a percent (2%).
pub const IDX_ELM_H: i64 = 200;
/// The extra index ELM on the option's own expiry day, hundredths of a percent (+2%).
pub const IDX_EXPIRY_ADD_H: i64 = 200;
/// The first trade date of the expiry-day extra ELM (SEBI/HO/MRD/TPD-1/P/CIR/2024/132, effective 20 Nov 2024), yyyymmdd.
pub const IDX_EXPIRY_ADD_FROM: u32 = 2024_11_20;
/// This module's change name (hosts print it beside the crate release).
pub const MARGIN_CHANGE: &str = "M1+M2";
/// MARGIN RULE v2 (MAIN-RULINGS-ENGINE-v22 addendum 9; margin-calibration RULE-CHANGE.md: 15 of 15 Dhan screens to the paisa): a flat Rs 20
/// per ORDER on top of the lots' margin (Dhan's F&O brokerage per order; outside the per-lot margin).
pub const ORDER_ADDON_PAISA: i64 = 2_000;

/// Why a margin could not be given (each counted by the host).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarginErr {
    /// The table text is not the documented format (missing header column, no rows).
    BadTable,
    /// The contract is not in the snapshot (a new listing, another series): refused and counted.
    NoRow,
    /// A stock row whose exposure rate is not known (no AEL file that day, or no AEL row): refused and counted.
    NoElmRate,
    /// A quantity of 0 units, a spot at or below 0, or a product that left its type: refused.
    BadInput,
}

impl std::fmt::Display for MarginErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BadTable => "margin: the table is not the documented format (refused)",
            Self::NoRow => "margin: the contract is not in the snapshot (no_span_row, refused)",
            Self::NoElmRate => "margin: no exposure rate for the stock that day (no_elm_rate, refused)",
            Self::BadInput => "margin: zero units, a spot at or below 0, or an overflow (refused)",
        })
    }
}

impl std::error::Error for MarginErr {}

/// The exposure class of a row as the table judged it at the file's own underlying price.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Cls {
    /// Stock, other than OTM at the file price.
    #[default]
    Oth,
    /// Stock, out of the money at the file price.
    Otm,
    /// Index.
    Idx,
    /// Index on the option's own expiry day (from 2024-11-20).
    IdxExpiry,
    /// No AEL file that day (stock rates unknown).
    NoAelFile,
    /// An AEL file without this symbol's row.
    NoAelRow,
}

/// One contract's row (per ONE unit; prices in paisa; rates in hundredths of a percent, -1 = not known).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Row {
    /// The underlying's id in [`Snapshot::unds`].
    pub und: u16,
    /// Expiry, yyyymmdd.
    pub expiry: u32,
    /// 0 CALL, 1 PUT.
    pub right: u8,
    /// Strike, paisa.
    pub strike: i64,
    /// SPAN per unit for a short = max(scan, SOM), paisa.
    pub span_unit: i64,
    /// The scan loss per unit (beside), paisa.
    pub scan_unit: i64,
    /// The worst scenario 1..16 (0 = none loses).
    pub worst: u8,
    /// The option's price in the file, paisa.
    pub price: i64,
    /// The underlying's price in the file, paisa (the fallback spot; the stale-file test reads it).
    pub und_price: i64,
    /// The table's rate as judged at the file price (index: 200 or 400), hundredths of a percent; -1 = NA.
    pub rate_h: i32,
    /// The stock's OTH and OTM AEL rates, hundredths of a percent; -1 = NA.
    pub oth_h: i32,
    /// See `oth_h`.
    pub otm_h: i32,
    /// The series' price scan range per unit (paisa; NSE prints it in rupees, e.g. 2143.94 for NIFTY = 9.30%), 0 = not given.
    pub price_scan: i64,
    /// Index row.
    pub idx: bool,
    /// The table's exposure class.
    pub cls: Cls,
}

/// The parts of one short position's margin (paisa) and the rate used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Parts {
    /// SPAN = span_unit x q.
    pub span: i64,
    /// Premium term = file price x q.
    pub premium: i64,
    /// ELM = rate x S x q, rounded once half away from zero.
    pub elm: i64,
    /// SPAN + premium + ELM (THE margin).
    pub exact: i64,
    /// SPAN + ELM (beside).
    pub nse_only: i64,
    /// The rate used, hundredths of a percent.
    pub rate_h: i64,
    /// The stock option was judged out of the money at S (index: false).
    pub otm: bool,
}

/// Rows refused while parsing, by reason (never silent).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParseCounts {
    /// Rows read (header excluded).
    pub rows: u64,
    /// Rows with a field that is not a whole number of paisa (more than 2 decimals) or not a number.
    pub bad_number: u64,
    /// Rows whose key repeats an earlier row (the first one is kept).
    pub dup_key: u64,
    /// Rows with too few fields.
    pub short: u64,
    /// Rows of another date than the first row's.
    pub other_date: u64,
}

/// One snapshot (one session's begin-of-day table or one intraday file's table) with its flat O(1) index.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    /// The session the rows serve, yyyymmdd.
    pub date: u32,
    /// The SPAN file label (e.g. `nsccl.20261113.i1`).
    pub file: String,
    /// The file's XML created stamp (YYYYMMDDHHMM IST).
    pub created: String,
    /// The AEL file used, or NONE.
    pub ael: String,
    /// The underlying codes; a row's `und` indexes here.
    pub unds: Vec<String>,
    und_ix: HashMap<String, u16>,
    rows: Vec<Row>,
    slots: Vec<u32>,
    mask: usize,
    /// The longest probe chain of any key (measured at build).
    pub max_probe: u32,
    /// Rows refused while parsing.
    pub counts: ParseCounts,
}

/// "3020" / "112.5" / "1690.32" -> paisa; None for a sign, more than 2 decimals or no digit.
#[must_use]
pub fn parse_paisa(s: &[u8]) -> Option<i64> {
    parse_x100(s)
}

/// "3.5" / "5.25" / "18.5" / "2.00" -> hundredths of a percent (350 / 525 / 1850 / 200); None like [`parse_paisa`].
#[must_use]
pub fn parse_pct_h(s: &[u8]) -> Option<i64> {
    parse_x100(s)
}

/// A rupee value with any number of decimals -> paisa rounded half up (the underlying's file price is printed with 6 decimals; it is only
/// the fallback spot and the stale-file test's base, never a margin term).
#[must_use]
pub fn parse_paisa_round(s: &[u8]) -> Option<i64> {
    let s = trim(s);
    let p = s.iter().position(|&c| c == b'.').unwrap_or(s.len());
    let (a, b) = (s.get(..p)?, s.get(p + 1..).unwrap_or(&[]));
    let head: Vec<u8> = a.iter().copied().chain(b.iter().copied().take(2)).chain(std::iter::repeat_n(b'0', 2_usize.saturating_sub(b.len()))).collect();
    if head.is_empty() || !head.iter().all(u8::is_ascii_digit) || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut v: i64 = 0;
    for &c in &head {
        v = v.checked_mul(10)?.checked_add(i64::from(c - b'0'))?;
    }
    Some(if b.get(2).is_some_and(|&c| c >= b'5') { v.checked_add(1)? } else { v })
}

fn parse_x100(s: &[u8]) -> Option<i64> {
    let s = trim(s);
    if s.is_empty() {
        return None;
    }
    let (a, b) = match s.iter().position(|&c| c == b'.') {
        Some(p) => (s.get(..p)?, s.get(p + 1..)?),
        None => (s, &[][..]),
    };
    if a.is_empty() && b.is_empty() {
        return None;
    }
    if !a.iter().all(u8::is_ascii_digit) || !b.iter().all(u8::is_ascii_digit) {
        return None;
    }
    // more than 2 decimals: only trailing zeros are allowed (an exact paisa value printed long)
    if b.len() > 2 && !b.get(2..)?.iter().all(|&c| c == b'0') {
        return None;
    }
    let mut v: i64 = 0;
    for &c in a {
        v = v.checked_mul(10)?.checked_add(i64::from(c - b'0'))?;
    }
    let d0 = b.first().map_or(0, |&c| i64::from(c - b'0'));
    let d1 = b.get(1).map_or(0, |&c| i64::from(c - b'0'));
    v.checked_mul(100)?.checked_add(d0 * 10 + d1)
}

fn trim(s: &[u8]) -> &[u8] {
    let mut a = 0;
    let mut b = s.len();
    while a < b && s.get(a).is_some_and(u8::is_ascii_whitespace) {
        a += 1;
    }
    while b > a && s.get(b - 1).is_some_and(u8::is_ascii_whitespace) {
        b -= 1;
    }
    s.get(a..b).unwrap_or(&[])
}

fn parse_u32(s: &[u8]) -> Option<u32> {
    let s = trim(s);
    if s.is_empty() || !s.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut v: u32 = 0;
    for &c in s {
        v = v.checked_mul(10)?.checked_add(u32::from(c - b'0'))?;
    }
    Some(v)
}

/// Days since 1970-01-01 of a yyyymmdd date (the key's expiry field); None for an impossible date.
#[must_use]
pub fn days_of(ymd: u32) -> Option<i64> {
    let y = i64::from(ymd / 10_000);
    let m = i64::from((ymd / 100) % 100);
    let d = i64::from(ymd % 100);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) || y < 1970 {
        return None;
    }
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// The packed key of one contract: und 16 bits | expiry days 15 bits | right 1 bit | strike paisa 32 bits. None when a part does not fit.
#[must_use]
pub fn key_of(und: u16, expiry: u32, right: u8, strike: i64) -> Option<u64> {
    let d = u64::try_from(days_of(expiry)?).ok()?;
    let k = u64::try_from(strike).ok()?;
    if d > 0x7FFF || k > 0xFFFF_FFFF || right > 1 {
        return None;
    }
    Some((u64::from(und) << 48) | (d << 33) | (u64::from(right) << 32) | k)
}

/// splitmix64's finalizer (the slot hash).
#[must_use]
pub const fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

const HEAD: [&str; 22] = ["date", "und", "seg", "expiry", "right", "strike", "file_price", "und_file_price", "span_unit_short", "scan_unit", "worst_scenario", "som_unit", "elm_rate_pct", "elm_class", "elm_oth_pct", "elm_otm_pct", "lot", "delta", "series_price_scan", "file", "file_created", "ael_file"];

impl Snapshot {
    /// Parses one table (decompressed bytes, header row first) and builds its flat index. Rows that cannot be read are counted, never
    /// guessed.
    ///
    /// # Errors
    /// [`MarginErr::BadTable`] for a header that is not the documented one, or a table without a single usable row.
    pub fn parse(text: &[u8]) -> Result<Self, MarginErr> {
        let mut lines = text.split(|&c| c == b'\n');
        let head = lines.next().ok_or(MarginErr::BadTable)?;
        let head: Vec<&[u8]> = trim(head).split(|&c| c == b'\t').collect();
        let col = |name: &str| head.iter().position(|h| *h == name.as_bytes());
        let mut ix = [0_usize; 22];
        for (slot, name) in ix.iter_mut().zip(HEAD) {
            *slot = col(name).ok_or(MarginErr::BadTable)?;
        }
        fn pick<'t>(ix: &[usize; 22], f: &[&'t [u8]], k: usize) -> &'t [u8] {
            ix.get(k).and_then(|&i| f.get(i)).copied().unwrap_or(&[])
        }
        let mut s = Self::default();
        let mut rows: Vec<Row> = Vec::new();
        for raw in lines {
            let line = trim(raw);
            if line.is_empty() {
                continue;
            }
            s.counts.rows += 1;
            let f: Vec<&[u8]> = line.split(|&c| c == b'\t').collect();
            if f.len() < head.len() {
                s.counts.short += 1;
                continue;
            }
            let date = parse_u32(&pick(&ix, &f, 0).iter().copied().filter(|&c| c != b'-').collect::<Vec<u8>>());
            let Some(date) = date else {
                s.counts.bad_number += 1;
                continue;
            };
            if s.date == 0 {
                s.date = date;
                s.file = String::from_utf8_lossy(pick(&ix, &f, 19)).into_owned();
                s.created = String::from_utf8_lossy(pick(&ix, &f, 20)).into_owned();
                s.ael = String::from_utf8_lossy(pick(&ix, &f, 21)).into_owned();
            } else if date != s.date {
                s.counts.other_date += 1;
                continue;
            }
            let und_name = String::from_utf8_lossy(trim(pick(&ix, &f, 1))).into_owned();
            let idx = trim(pick(&ix, &f, 2)) == b"idx";
            let right = match trim(pick(&ix, &f, 4)) {
                b"C" | b"CE" => 0_u8,
                b"P" | b"PE" => 1_u8,
                _ => {
                    s.counts.bad_number += 1;
                    continue;
                }
            };
            let (Some(expiry), Some(strike), Some(price), Some(span_unit), Some(scan_unit)) = (parse_u32(pick(&ix, &f, 3)), parse_paisa(pick(&ix, &f, 5)), parse_paisa(pick(&ix, &f, 6)), parse_paisa(pick(&ix, &f, 8)), parse_paisa(pick(&ix, &f, 9))) else {
                s.counts.bad_number += 1;
                continue;
            };
            let und_price = parse_paisa_round(pick(&ix, &f, 7)).unwrap_or(0);
            let worst = parse_u32(pick(&ix, &f, 10)).and_then(|x| u8::try_from(x).ok()).unwrap_or(0);
            let na = |b: &[u8]| -> i32 { parse_pct_h(b).and_then(|x| i32::try_from(x).ok()).unwrap_or(-1) };
            let (rate_h, oth_h, otm_h) = (na(pick(&ix, &f, 12)), na(pick(&ix, &f, 14)), na(pick(&ix, &f, 15)));
            let cls = match trim(pick(&ix, &f, 13)) {
                b"OTM" => Cls::Otm,
                b"IDX" => Cls::Idx,
                b"IDX-EXPIRY-DAY" => Cls::IdxExpiry,
                b"NO-AEL-FILE" => Cls::NoAelFile,
                b"NO-AEL-ROW" => Cls::NoAelRow,
                _ => Cls::Oth,
            };
            let price_scan = parse_paisa_round(pick(&ix, &f, 18)).unwrap_or(0);
            let und = if let Some(&u) = s.und_ix.get(&und_name) {
                u
            } else {
                let u = u16::try_from(s.unds.len()).map_err(|_| MarginErr::BadTable)?;
                s.unds.push(und_name.clone());
                s.und_ix.insert(und_name, u);
                u
            };
            rows.push(Row { und, expiry, right, strike, span_unit, scan_unit, worst, price, und_price, rate_h, oth_h, otm_h, price_scan, idx, cls });
        }
        if rows.is_empty() {
            return Err(MarginErr::BadTable);
        }
        s.build(rows)?;
        Ok(s)
    }

    fn build(&mut self, rows: Vec<Row>) -> Result<(), MarginErr> {
        let cap = rows.len().checked_mul(2).ok_or(MarginErr::BadTable)?.next_power_of_two().max(16);
        let mut slots = vec![u32::MAX; cap];
        let mask = cap - 1;
        let mut kept: Vec<Row> = Vec::with_capacity(rows.len());
        let mut max_probe = 0_u32;
        for r in rows {
            let Some(k) = key_of(r.und, r.expiry, r.right, r.strike) else {
                self.counts.bad_number += 1;
                continue;
            };
            let mut i = usize::try_from(mix(k)).unwrap_or(0) & mask;
            let mut probes = 1_u32;
            let mut dup = false;
            loop {
                let Some(&cur) = slots.get(i) else { return Err(MarginErr::BadTable) };
                if cur == u32::MAX {
                    break;
                }
                let same = kept.get(cur as usize).and_then(|x| key_of(x.und, x.expiry, x.right, x.strike)) == Some(k);
                if same {
                    dup = true;
                    break;
                }
                i = (i + 1) & mask;
                probes += 1;
            }
            if dup {
                self.counts.dup_key += 1;
                continue;
            }
            let id = u32::try_from(kept.len()).map_err(|_| MarginErr::BadTable)?;
            if let Some(slot) = slots.get_mut(i) {
                *slot = id;
            }
            kept.push(r);
            max_probe = max_probe.max(probes);
        }
        self.rows = kept;
        self.slots = slots;
        self.mask = mask;
        self.max_probe = max_probe;
        Ok(())
    }

    /// The rows (in table order, duplicates dropped).
    #[must_use]
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The row with id `i`.
    #[must_use]
    pub fn row(&self, i: u32) -> Option<&Row> {
        self.rows.get(i as usize)
    }

    /// The id of an underlying code (O(1) expected; one hash lookup).
    #[must_use]
    pub fn und_id(&self, name: &str) -> Option<u16> {
        self.und_ix.get(name).copied()
    }

    /// The row id of a contract by underlying id (O(1): the flat table; `probes` receives the probe count).
    #[must_use]
    pub fn find_id(&self, und: u16, expiry: u32, right: u8, strike: i64, probes: &mut u32) -> Option<u32> {
        let k = key_of(und, expiry, right, strike)?;
        let mut i = usize::try_from(mix(k)).unwrap_or(0) & self.mask;
        *probes = 0;
        loop {
            *probes += 1;
            let cur = *self.slots.get(i)?;
            if cur == u32::MAX {
                return None;
            }
            let r = self.rows.get(cur as usize)?;
            if r.und == und && r.expiry == expiry && r.right == right && r.strike == strike {
                return Some(cur);
            }
            if *probes > self.max_probe {
                return None;
            }
            i = (i + 1) & self.mask;
        }
    }

    /// The row id of a contract by underlying code.
    #[must_use]
    pub fn find(&self, und: &str, expiry: u32, right: u8, strike: i64, probes: &mut u32) -> Option<u32> {
        let u = self.und_id(und)?;
        self.find_id(u, expiry, right, strike, probes)
    }
}

/// ELM in paisa = rate_h / 10,000 x s x q, rounded half away from zero (every term >= 0): (2n + 10,000) / 20,000.
///
/// # Errors
/// [`MarginErr::BadInput`] for a negative term or an overflow.
pub fn elm_paisa(rate_h: i64, s: i64, q: u64) -> Result<i64, MarginErr> {
    if rate_h < 0 || s < 0 {
        return Err(MarginErr::BadInput);
    }
    let n = i128::from(rate_h).checked_mul(i128::from(s)).and_then(|x| x.checked_mul(i128::from(q))).ok_or(MarginErr::BadInput)?;
    // MUTANT m_v31_elm_round_down: the exposure product rounded down (the bug page fix r2 removed)
    let r = if cfg!(feature = "m_v31_elm_round_down") { n / 10_000 } else { (2 * n + 10_000) / 20_000 };
    i64::try_from(r).map_err(|_| MarginErr::BadInput)
}

/// Whether a stock option is out of the money at spot `s` (CALL K > S, PUT K < S).
#[must_use]
pub const fn is_otm(right: u8, strike: i64, s: i64) -> bool {
    if right == 0 { strike > s } else { strike < s }
}

/// The exposure rate of a row at live spot `s` on trade date `day` (yyyymmdd), hundredths of a percent.
///
/// # Errors
/// [`MarginErr::NoElmRate`] for a stock row without the needed AEL rate.
pub fn rate_of(r: &Row, s: i64, day: u32) -> Result<(i64, bool), MarginErr> {
    if r.idx {
        let add = if r.expiry == day && day >= IDX_EXPIRY_ADD_FROM { IDX_EXPIRY_ADD_H } else { 0 };
        return Ok((IDX_ELM_H + add, false));
    }
    let otm = is_otm(r.right, r.strike, s);
    let h = if otm { r.otm_h } else { r.oth_h };
    if h < 0 {
        return Err(MarginErr::NoElmRate);
    }
    Ok((i64::from(h), otm))
}

/// THE margin of a SHORT position of `q` units of the row's contract at live spot `s` (paisa) on trade date `day` (yyyymmdd): rule v1
/// (margin-truth: the exposure base is the live spot for an index and a stock alike).
///
/// # Errors
/// [`MarginErr::BadInput`] for q = 0, s <= 0 or an overflow; [`MarginErr::NoElmRate`] for a stock without its AEL rate.
pub fn short_margin(r: &Row, q: u64, s: i64, day: u32) -> Result<Parts, MarginErr> {
    short_margin_base(r, q, s, s, day)
}

/// MARGIN RULE v2 (addendum 9): the exposure BASE is given apart from the spot that judges a stock's OTM row: an index option's base is the
/// near-month index FUTURE's price in the same SPAN file ([`near_future`]); a stock option's base stays its live price at T. The per-lot
/// margin is otherwise rule v1's; the order's Rs 20 ([`ORDER_ADDON_PAISA`]) is added once per order by the sizing ([`lots_fit_v2`]).
///
/// # Errors
/// As [`short_margin`], and [`MarginErr::BadInput`] for a base at or below 0.
pub fn short_margin_base(r: &Row, q: u64, s: i64, base: i64, day: u32) -> Result<Parts, MarginErr> {
    if q == 0 || s <= 0 || base <= 0 {
        return Err(MarginErr::BadInput);
    }
    let qi = i64::try_from(q).map_err(|_| MarginErr::BadInput)?;
    let (rate_h, otm) = rate_of(r, s, day)?;
    let span = r.span_unit.checked_mul(qi).ok_or(MarginErr::BadInput)?;
    // MUTANT m_v31_margin_no_premium: the premium term (the option's value, the Dhan calibration's term) left out
    let premium = if cfg!(feature = "m_v31_margin_no_premium") { 0 } else { r.price.checked_mul(qi).ok_or(MarginErr::BadInput)? };
    let elm = elm_paisa(rate_h, base, q)?;
    let nse_only = span.checked_add(elm).ok_or(MarginErr::BadInput)?;
    let exact = nse_only.checked_add(premium).ok_or(MarginErr::BadInput)?;
    Ok(Parts { span, premium, elm, exact, nse_only, rate_h, otm })
}

/// CRATE v2.4 change M3 (MAIN-RULINGS-ENGINE-v22 addendum 11; margin-calibration RULE-CHANGE-v3.md, 46 of 46 Dhan screens to the paisa):
/// MARGIN RULE v3's exposure rate. Index rows: rule v2's (2%, +2% on the option's own expiry day from 2024-11-20). Stock rows: the AEL
/// **OTH** row's "Total applicable ELM%" for EVERY short stock option, whatever its moneyness (the OTM row is never read; the moneyness at
/// `s` is returned as a reported field only).
///
/// # Errors
/// [`MarginErr::NoElmRate`] for a stock row without its OTH rate.
pub fn rate_of_v3(r: &Row, s: i64, day: u32) -> Result<(i64, bool), MarginErr> {
    if r.idx {
        return rate_of(r, s, day);
    }
    let otm = is_otm(r.right, r.strike, s);
    // MUTANT m_v32_margin_v3_otm_row: rule v2's OTM row read again for an out-of-the-money stock option
    let h = if cfg!(feature = "m_v32_margin_v3_otm_row") && otm { r.otm_h } else { r.oth_h };
    if h < 0 {
        return Err(MarginErr::NoElmRate);
    }
    Ok((i64::from(h), otm))
}

/// CRATE v2.4 change M3: THE margin of a SHORT position by MARGIN RULE v3 = rule v2 ([`short_margin_base`]: SPAN worst-of-16 x q, the option
/// value at NSE's file price x q, exposure = rate x base x q rounded ONCE half away from zero; the order's Rs 20 added once per order by the
/// sizing) with the stock exposure rate of [`rate_of_v3`].
///
/// # Errors
/// As [`short_margin_base`].
pub fn short_margin_base_v3(r: &Row, q: u64, s: i64, base: i64, day: u32) -> Result<Parts, MarginErr> {
    if q == 0 || s <= 0 || base <= 0 {
        return Err(MarginErr::BadInput);
    }
    let qi = i64::try_from(q).map_err(|_| MarginErr::BadInput)?;
    let (rate_h, otm) = rate_of_v3(r, s, day)?;
    let span = r.span_unit.checked_mul(qi).ok_or(MarginErr::BadInput)?;
    let premium = r.price.checked_mul(qi).ok_or(MarginErr::BadInput)?;
    let elm = elm_paisa(rate_h, base, q)?;
    let nse_only = span.checked_add(elm).ok_or(MarginErr::BadInput)?;
    let exact = nse_only.checked_add(premium).ok_or(MarginErr::BadInput)?;
    Ok(Parts { span, premium, elm, exact, nse_only, rate_h, otm })
}

/// MARGIN RULE v2: the price of the index future whose expiry is the FIRST on or after the option's expiry (`futs` = (expiry yyyymmdd, price
/// paisa) of the underlying's futures in the same SPAN file, any order).
#[must_use]
pub fn near_future(futs: &[(u32, i64)], option_expiry: u32) -> Option<i64> {
    futs.iter().filter(|f| f.0 >= option_expiry && f.1 > 0).min_by_key(|f| f.0).map(|f| f.1)
}

/// MARGIN RULE v2 sizing: the whole lots of per-lot margin `m_lot` that fit `slot` after the order's Rs 20: floor((slot - Rs 20) / m_lot)
/// (0 = not taken).
#[must_use]
pub fn lots_fit_v2(slot: i64, m_lot: i64) -> i64 {
    if m_lot <= 0 {
        return 0;
    }
    // MUTANT m_v31_addon_per_lot: the Rs 20 charged on every lot instead of once per order
    if cfg!(feature = "m_v31_addon_per_lot") {
        return (slot / (m_lot + ORDER_ADDON_PAISA)).max(0);
    }
    ((slot - ORDER_ADDON_PAISA) / m_lot).max(0)
}

/// The stale-file test of addendum 2: the live spot moved more than 1/3 of the series' price scan range since the file (|S - file spot| x 3
/// > the price scan range, both per unit in paisa).
#[must_use]
pub fn stale(r: &Row, s: i64) -> bool {
    if r.und_price <= 0 || r.price_scan <= 0 {
        return false;
    }
    i128::from((s - r.und_price).abs()) * 3 > i128::from(r.price_scan)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(rows: &[&str]) -> Vec<u8> {
        let mut t = HEAD.join("\t");
        t.push('\n');
        for r in rows {
            t.push_str(r);
            t.push('\n');
        }
        t.into_bytes()
    }

    // GRASIM 23 NOV 2026 3020 CE in the 16 Nov begin-of-day file (margin-truth section 4): scan 388.94, file price 66.70, spot 3,033.87,
    // OTH 3.5% / OTM 5.25%.
    const GRASIM: &str = "2026-11-16\tGRASIM\tstk\t20261123\tC\t3020\t66.7\t3033.87\t388.94\t388.94\t11\t0\t3.5\tOTH\t3.5\t5.25\t\t0.6\t14.2\tnsccl.20261116.i1\t202611140017\tael_16112026.csv";

    #[test]
    fn v31_margin_grasim_dhan_screen() {
        let s = Snapshot::parse(&table(&[GRASIM])).unwrap_or_else(|e| panic!("{e}"));
        let mut p = 0;
        let id = s.find("GRASIM", 2026_11_23, 0, 302_000, &mut p).unwrap_or_else(|| panic!("row"));
        let r = *s.row(id).unwrap_or_else(|| panic!("row"));
        let m = short_margin(&r, 250, 303_387, 2026_11_16).unwrap_or_else(|e| panic!("{e}"));
        // 388.94 x 250 = 97,235.00; 66.70 x 250 = 16,675.00; 3.5% x 3,033.87 x 250 = 26,546.3625 -> 26,546.36; total 1,40,456.36
        assert_eq!((m.span, m.premium, m.elm, m.exact, m.nse_only), (9_723_500, 1_667_500, 2_654_636, 14_045_636, 12_378_136));
        assert!(!m.otm);
        // the Dhan screen: Rs 1,40,477.50; the gap Rs 21.14 (margin-truth's open item)
        assert_eq!(14_047_750 - m.exact, 2_114);
    }

    #[test]
    fn v31_margin_elm_half_away_from_zero() {
        // 3.5% x 3,033.87 x 250 = 26,546.3625 paisa-exact? 350 x 303,387 x 250 / 10,000 = 2,654,636.25 -> 2,654,636
        assert_eq!(elm_paisa(350, 303_387, 250), Ok(2_654_636));
        // an exact half paisa rounds UP (away from zero): 2% x 0.25 paisa... 200 x 25 x 1 / 10,000 = 0.5 -> 1
        assert_eq!(elm_paisa(200, 25, 1), Ok(1));
        assert_eq!(elm_paisa(200, 24, 1), Ok(0));
        assert_eq!(elm_paisa(-1, 25, 1), Err(MarginErr::BadInput));
    }

    #[test]
    fn v31_margin_index_rates_and_expiry_day() {
        let r = Row { idx: true, expiry: 2026_11_03, right: 0, strike: 2_350_000, span_unit: 169_032, price: 665, rate_h: 200, ..Row::default() };
        // NIFTY 03 Nov 23500 CE, 11-02 i1 (margin-truth idx-A1 #1): S 23,266.95, lot 65 -> exact 1,40,550.09
        let m = short_margin(&r, 65, 2_326_695, 2026_11_02).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((m.span, m.premium, m.elm, m.exact), (10_987_080, 43_225, 3_024_704, 14_055_009));
        // on its own expiry day: 4%
        let e = short_margin(&r, 65, 2_326_695, 2026_11_03).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(e.rate_h, 400);
        // before 2024-11-20 no extra
        let old = Row { expiry: 2024_10_31, ..r };
        assert_eq!(short_margin(&old, 65, 2_326_695, 2024_10_31).map(|x| x.rate_h), Ok(200));
    }

    #[test]
    fn v31_margin_stock_otm_judged_at_live_spot() {
        let r = Row { idx: false, right: 0, strike: 190_000, span_unit: 10_000, price: 500, oth_h: 350, otm_h: 525, ..Row::default() };
        // a CALL at 1,900 is OTM at a spot of 1,876 and in the money at 1,950
        assert_eq!(short_margin(&r, 1, 187_600, 2026_11_05).map(|x| (x.rate_h, x.otm)), Ok((525, true)));
        assert_eq!(short_margin(&r, 1, 195_000, 2026_11_05).map(|x| (x.rate_h, x.otm)), Ok((350, false)));
        // a stock without AEL rates is refused, never guessed
        let na = Row { oth_h: -1, otm_h: -1, ..r };
        assert_eq!(short_margin(&na, 1, 187_600, 2026_11_05), Err(MarginErr::NoElmRate));
        assert_eq!(short_margin(&r, 0, 187_600, 2026_11_05), Err(MarginErr::BadInput));
        assert_eq!(short_margin(&r, 1, 0, 2026_11_05), Err(MarginErr::BadInput));
    }

    #[test]
    fn v31_margin_numbers_exact_or_refused() {
        assert_eq!(parse_paisa(b"3020"), Some(302_000));
        assert_eq!(parse_paisa(b"112.5"), Some(11_250));
        assert_eq!(parse_paisa(b"1690.32"), Some(169_032));
        assert_eq!(parse_paisa(b"0.05"), Some(5));
        assert_eq!(parse_paisa(b"1.2300"), Some(123));
        assert_eq!(parse_paisa(b"1.234"), None, "a third decimal is never rounded silently");
        assert_eq!(parse_paisa(b"-1"), None);
        assert_eq!(parse_paisa(b"NA"), None);
        assert_eq!(parse_pct_h(b"18.5"), Some(1_850));
        assert_eq!(parse_pct_h(b"5.25"), Some(525));
        assert_eq!(parse_paisa_round(b"984.980000"), Some(98_498));
        assert_eq!(parse_paisa_round(b"23053.105000"), Some(2_305_311));
        assert_eq!(parse_paisa_round(b"2143.94"), Some(214_394));
        assert_eq!(parse_paisa_round(b"7"), Some(700));
    }

    #[test]
    fn v31_margin_index_o1_and_refusals_counted() {
        let mut rows: Vec<String> = Vec::new();
        for u in 0..40 {
            for k in 0..300 {
                for r in ["C", "P"] {
                    rows.push(format!("2026-11-02\tS{u}\tstk\t20261124\t{r}\t{}\t1.5\t100\t10\t10\t11\t0\t3.5\tOTH\t3.5\t5.25\t\t0.5\t14.2\tnsccl.20261102.i1\t202610310013\tael_02112026.csv", 100 + k * 5));
                }
            }
        }
        // a duplicate, a three-decimal price, a short row and another date: each counted, none kept
        rows.push("2026-11-02\tS0\tstk\t20261124\tC\t100\t1.5\t100\t10\t10\t11\t0\t3.5\tOTH\t3.5\t5.25\t\t0.5\t14.2\tx\ty\tz".to_owned());
        rows.push("2026-11-02\tS0\tstk\t20261124\tC\t99\t1.555\t100\t10\t10\t11\t0\t3.5\tOTH\t3.5\t5.25\t\t0.5\t14.2\tx\ty\tz".to_owned());
        rows.push("2026-11-02\tS0".to_owned());
        rows.push("2026-11-03\tS0\tstk\t20261124\tC\t98\t1.5\t100\t10\t10\t11\t0\t3.5\tOTH\t3.5\t5.25\t\t0.5\t14.2\tx\ty\tz".to_owned());
        let refs: Vec<&str> = rows.iter().map(String::as_str).collect();
        let s = Snapshot::parse(&table(&refs)).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(s.rows().len(), 24_000);
        assert_eq!((s.counts.dup_key, s.counts.bad_number, s.counts.short, s.counts.other_date), (1, 1, 1, 1));
        // every key found, probes bounded by the measured chain; absent keys refused
        let mut worst = 0;
        for u in 0..40_u16 {
            for k in 0..300_i64 {
                for r in 0..2_u8 {
                    let mut p = 0;
                    let id = s.find_id(u, 2026_11_24, r, (100 + k * 5) * 100, &mut p);
                    assert!(id.is_some());
                    worst = worst.max(p);
                }
            }
        }
        assert!(worst <= s.max_probe && s.max_probe <= 64, "probe chain {worst} / {}", s.max_probe);
        let mut p = 0;
        assert_eq!(s.find("S0", 2026_11_24, 0, 10_100 + 1, &mut p), None, "a strike not in the file");
        assert_eq!(s.find("NEWLISTING", 2026_11_24, 0, 10_000, &mut p), None, "an underlying not in the file");
        assert_eq!(s.find("S0", 2026_12_29, 0, 10_000, &mut p), None, "another series");
    }

    #[test]
    fn v31_margin_stale_file_flag() {
        // file spot 2,187, price scan 14.20% = Rs 310.55: a third is Rs 103.52; 1,876 moved Rs 311 (stale), 2,100 moved Rs 87 (not stale)
        let r = Row { und_price: 218_700, price_scan: 31_055, ..Row::default() };
        assert!(stale(&r, 187_600));
        assert!(!stale(&r, 210_000));
        assert!(!stale(&Row::default(), 100));
    }

    /// MARGIN RULE v2 against the operator's Dhan screens (margin-calibration calibration.tsv, all on `nsccl.20261116.i1`): screen 2 NIFTY 17
    /// NOV 25000 CE x65, screen 8 BANKNIFTY 23 NOV 61000 PE x30 (index base = the 23 Nov future in the file), screen 15 GRASIM 3020 CE x250
    /// (stock base = NSE's close 3,034.00); each + Rs 20 = Dhan's figure to the paisa.
    #[test]
    fn v31_margin_rule_v2_dhan_screens() {
        let nifty = Row { idx: true, expiry: 2026_11_17, right: 0, strike: 2_500_000, span_unit: 226_817, price: 22_460, ..Row::default() };
        let base = near_future(&[(2026_12_29, 2_550_000), (2026_11_23, 2_517_100), (2027_01_25, 2_560_000)], 2026_11_17).unwrap_or(0);
        assert_eq!(base, 2_517_100, "the first future expiring on or after the option's expiry");
        let m = short_margin_base(&nifty, 65, 2_514_430, base, 2026_11_16).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!((m.span, m.premium, m.elm), (14_743_105, 1_459_900, 3_272_230));
        assert_eq!(m.exact + ORDER_ADDON_PAISA, 19_477_235, "Rs 1,94,772.35 = Dhan screen 2");
        let bank = Row { idx: true, expiry: 2026_11_23, right: 1, strike: 6_100_000, span_unit: 491_780, price: 64_545, ..Row::default() };
        let m8 = short_margin_base(&bank, 30, 6_103_410, 6_093_340, 2026_11_16).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(m8.exact + ORDER_ADDON_PAISA, 20_347_754, "Rs 2,03,477.54 = Dhan screen 8");
        let grasim = Row { idx: false, expiry: 2026_11_23, right: 0, strike: 302_000, span_unit: 38_894, price: 6_670, oth_h: 350, otm_h: 525, ..Row::default() };
        let m15 = short_margin_base(&grasim, 250, 303_400, 303_400, 2026_11_16).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(m15.exact + ORDER_ADDON_PAISA, 14_047_750, "Rs 1,40,477.50 = Dhan screen 15 (the old Rs 21.14 gap closed)");
        // the sizing: floor((slot - Rs 20) / per-lot margin); exactly one lot's room plus the Rs 20 fits one lot
        assert_eq!(lots_fit_v2(m15.exact + ORDER_ADDON_PAISA, m15.exact), 1);
        assert_eq!(lots_fit_v2(m15.exact + ORDER_ADDON_PAISA - 1, m15.exact), 0, "one paisa short of the Rs 20: not taken");
        assert_eq!(lots_fit_v2(40_000_000, m15.exact), 2);
        // the Rs 20 is once per ORDER, not per lot: Rs 30,050 of room fits 3 lots of Rs 10,000 (per lot it would fit 2)
        assert_eq!(lots_fit_v2(3_005_000, 1_000_000), 3);
        assert_eq!(near_future(&[], 2026_11_17), None);
    }

    #[test]
    fn v31_margin_bad_table_refused() {
        assert_eq!(Snapshot::parse(b"date\tund\n2026-11-02\tX\n").map(|_| ()), Err(MarginErr::BadTable));
        assert_eq!(Snapshot::parse(&table(&[])).map(|_| ()), Err(MarginErr::BadTable));
    }
}
