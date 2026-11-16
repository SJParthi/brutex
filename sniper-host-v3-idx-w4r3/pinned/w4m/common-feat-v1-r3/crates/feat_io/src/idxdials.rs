//! S4 (r3, wave 4; SNIPER-IDXOPT-PLAN-v3.1 s3.3, v2 s4): the INDEX tracks' context dials, each a reading of data rows, through the
//! ONE dial code path [`crate::dials`]:
//!
//! | Dial | Index reading at S (the decision candle's END) |
//! |---|---|
//! | X-PEER | the OTHER index's last COMPLETED 1-minute close of the session (END <= S) against its previous session's OFFICIAL close, bp (NIFTY reads NIFTY BANK; BANKNIFTY reads NIFTY 50: v2 s4, MAIN Q5) |
//! | X-VIX | INDIA VIX's last completed 1-minute close of the session, x100 (as ADANIENT) |
//! | D-EXP | the session is the index's own TRUE expiry day AS KNOWN before it (DP-IDXCAL `known_eff` = the day; holiday rule and re-dates applied; asserted equal to S1's truth column on every row at load) |
//! | D-MEND | the last session of its month as known on the prior evening (as ADANIENT, the same table) |
//! | G-52H / G-52L | the 252 sessions of the dated table before the session, over the INDEX official rows (no corporate action: factor 1) |
//! | G-DRNG | the session's range so far on the decision stream against the daily ATR 14 of the index official rows before it |
//! | D-DTE (new) | regular sessions in (d, E_known] from the dated session table, 0..4 and 5 = 5+ (F2 / R6: the ex-post count labelled `dte_closure_unverified` where a closure or special session lies inside; on the stale windows the stale as-known date; at the data end, where no later session is in the table, the causal weekday count, labelled `data_end`) |
//!
//! V-RVOL is refused for an index row ([`crate::instr::rvol_check`], m-IX3). Muhurat candles never reach a decision stream
//! ([`crate::host::ZRead::muhurat_excluded`], Q39). K11 holds as in [`crate::dials`]: the mutant `m_k11` also makes D-DTE count to
//! the TRUE current expiry (a date first listed AFTER the session's file) and must turn the stale-window tests red.

use std::collections::BTreeMap;
use std::path::Path;

use common_feat::cal::{parse_ymd, ymd};
use common_feat::fold::Candle;

use crate::data::Session;
use crate::dials::{self, CtxRead};
use crate::host::{Ctx, Stream};
use crate::instr::{CalKind, InstrRow};

/// One DP-IDXCAL row (S1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CalRow {
    /// The session.
    pub day: i32,
    /// The bhavcopy file read (dated before the session, K11).
    pub file_read: i32,
    /// The current expiry as known (holiday rule and re-dates applied).
    pub known_eff: i32,
    /// The true current expiry (ex-post; `None` at the data end).
    pub true_current: Option<i32>,
    /// S1's stale flag (as known != true).
    pub stale: bool,
    /// S1's truth column: the session is a true expiry day.
    pub d_exp_true: bool,
    /// Regular sessions in (d, known_eff] (ex-post, the session table).
    pub dte_sessions: u32,
    /// Weekdays in (d, known_eff] (the causal count without closure announcements).
    pub dte_weekdays: u32,
    /// A closure or special session lies inside (d, known_eff].
    pub unverified: bool,
}

/// The D-DTE reading of one session.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Dte {
    /// 0..=5 (5 = 5+); `None`: unknown.
    pub value: Option<u8>,
    /// The as-known expiry it counts to.
    pub known: i32,
    /// The regular-session count (ex-post).
    pub sessions: u32,
    /// The weekday count (causal).
    pub weekdays: u32,
    /// `dte_closure_unverified` (F2): a closure or special session inside; both counts print.
    pub unverified: bool,
    /// The as-known date is a stale listing (MAIN Q6: used and printed).
    pub stale: bool,
    /// At the data end: no later session in the table, the causal weekday count is read (labelled).
    pub data_end: bool,
}

/// DP-IDXCAL of one index, checked at load.
#[derive(Clone, Debug, Default)]
pub struct IdxCal {
    rows: BTreeMap<i32, CalRow>,
    /// The regular sessions of the index session table, ascending.
    regular: Vec<i32>,
    last_session: i32,
}

fn day_or_dash(s: &str) -> Result<Option<i32>, String> {
    if s == "-" { Ok(None) } else { parse_ymd(s).map(Some).ok_or_else(|| format!("DP-IDXCAL date {s}")) }
}

impl IdxCal {
    /// Parses DP-IDXCAL (`session weekday file_read known_listed known_eff series next_eff true_current stale d_exp class_known
    /// dte_sessions dte_weekdays dte_closure_unverified closures specials`) against the index session table, asserting:
    /// (1) every file read is dated before its session (K11); (2) D-EXP as known (`known_eff` = the day) equals S1's truth column
    /// on every row (the G-CAL truth assertion, re-proved on D-EXP); (3) the regular-session count recomputed here from the session
    /// table equals `dte_sessions` on every row whose as-known expiry is inside the table.
    ///
    /// # Errors
    /// A malformed row or a failed assertion, named.
    pub fn parse(text: &str, sessions: &[Session]) -> Result<Self, String> {
        // S1's regular sessions (idx-cal main.rs:140): the table less its MUHURAT, DR and WEEKEND rows (an OUTAGE day traded)
        let regular: Vec<i32> = sessions.iter().filter(|s| !matches!(s.kind.as_str(), "MUHURAT" | "DR" | "WEEKEND")).map(|s| s.day).collect();
        let last_session = sessions.last().map_or(0, |s| s.day);
        let mut rows = BTreeMap::new();
        let mut head: Vec<&str> = Vec::new();
        for l in text.lines() {
            if l.starts_with('#') || l.trim().is_empty() {
                continue;
            }
            let c: Vec<&str> = l.split('\t').collect();
            if c.first() == Some(&"session") {
                head = c;
                continue;
            }
            let g = |n: &str| -> Result<&str, String> {
                let k = head.iter().position(|h| *h == n).ok_or_else(|| format!("DP-IDXCAL: no column {n}"))?;
                c.get(k).copied().ok_or_else(|| format!("DP-IDXCAL row too short: {l}"))
            };
            let num = |n: &str| -> Result<u32, String> { g(n)?.parse().map_err(|_| format!("DP-IDXCAL {n}: {l}")) };
            let r = CalRow {
                day: day_or_dash(g("session")?)?.ok_or("DP-IDXCAL session")?,
                file_read: day_or_dash(g("file_read")?)?.ok_or("DP-IDXCAL file_read")?,
                known_eff: day_or_dash(g("known_eff")?)?.ok_or("DP-IDXCAL known_eff")?,
                true_current: day_or_dash(g("true_current")?)?,
                stale: g("stale")? == "1",
                d_exp_true: g("d_exp")? == "1",
                dte_sessions: num("dte_sessions")?,
                dte_weekdays: num("dte_weekdays")?,
                unverified: g("dte_closure_unverified")? == "1",
            };
            if r.file_read >= r.day {
                return Err(format!("DP-IDXCAL {}: file {} is not dated before the session (K11): refused", ymd(r.day), ymd(r.file_read)));
            }
            if (r.known_eff == r.day) != r.d_exp_true {
                return Err(format!("DP-IDXCAL {}: D-EXP as known ({}) differs from the truth column: refused", ymd(r.day), ymd(r.known_eff)));
            }
            rows.insert(r.day, r);
        }
        let me = Self { rows, regular, last_session };
        for r in me.rows.values() {
            if r.known_eff <= last_session && me.count(r.day, r.known_eff) != r.dte_sessions {
                return Err(format!("DP-IDXCAL {}: regular sessions to {} recomputed {} != dte_sessions {}: refused", ymd(r.day), ymd(r.known_eff), me.count(r.day, r.known_eff), r.dte_sessions));
            }
        }
        Ok(me)
    }

    /// Regular sessions in (d, e].
    #[must_use]
    pub fn count(&self, d: i32, e: i32) -> u32 {
        let a = self.regular.partition_point(|&x| x <= d);
        let b = self.regular.partition_point(|&x| x <= e);
        u32::try_from(b.saturating_sub(a)).unwrap_or(u32::MAX)
    }

    /// The row of session `d`.
    #[must_use]
    pub fn row(&self, d: i32) -> Option<&CalRow> {
        self.rows.get(&d)
    }

    /// D-EXP at session `d`: the index's true expiry day as known before the session (`None` outside the calendar).
    #[must_use]
    pub fn d_exp(&self, d: i32) -> Option<bool> {
        self.rows.get(&d).map(|r| r.known_eff == d)
    }

    /// D-DTE at session `d` (module doc).
    #[must_use]
    pub fn dte(&self, d: i32) -> Dte {
        let Some(r) = self.rows.get(&d) else { return Dte::default() };
        // MUTANT m_k11: counts to the TRUE current expiry (on a stale window a date first listed after the session's file).
        let target = if cfg!(feature = "m_k11") { r.true_current.unwrap_or(r.known_eff) } else { r.known_eff };
        let data_end = target > self.last_session;
        let n = if data_end { r.dte_weekdays } else { self.count(d, target) };
        Dte { value: Some(u8::try_from(n.min(5)).unwrap_or(5)), known: r.known_eff, sessions: r.dte_sessions, weekdays: r.dte_weekdays, unverified: r.unverified, stale: r.stale, data_end }
    }
}

/// One index decision's readings: the seven (with `nifty_bp` = X-PEER) and D-DTE.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct IdxRead {
    /// X-PEER (in `nifty_bp`), X-VIX, D-EXP, D-MEND, G-52H, G-52L, G-DRNG.
    pub ctx: CtxRead,
    /// D-DTE.
    pub dte: Dte,
}

/// The prepared sources of an index row's dials.
#[derive(Clone, Debug, Default)]
pub struct IdxPrep {
    /// The peer's 1-minute candles (X-PEER).
    pub peer1: Vec<Candle>,
    /// INDIA VIX 1-minute candles.
    pub vix1: Vec<Candle>,
    /// Per session of the table: the peer's previous session's official close.
    pub peer_prev: Vec<Option<i64>>,
    /// D-MEND per session.
    pub mend: Vec<Option<bool>>,
    /// G-52H / G-52L per session.
    pub hl: Vec<Option<(i64, i64)>>,
    /// The daily ATR 14 known at each session.
    pub atr: Vec<Option<i128>>,
    /// Sessions without an official row (the ATR's holes).
    pub atr_holes: Vec<i32>,
    /// DP-IDXCAL.
    pub cal: IdxCal,
    /// Store notes of the reads.
    pub notes: Vec<String>,
}

fn read_1min(store: &Path, symbol: &str, from: (u16, u8), to: (u16, u8)) -> (Vec<Candle>, Vec<String>) {
    let tf = store::path::Timeframe::MINUTE_1;
    let (mut b, n) = crate::zstore::read_range(store, brutex_core::vendor::Vendor::Zerodha, "INDEX", symbol, tf, from, to);
    b.sort_by_key(|x| x.start);
    (b, n)
}

impl IdxPrep {
    /// Prepares the sources of `row` (an INDEX row) with its `peer` row and its context `ctx`.
    ///
    /// # Errors
    /// A non-index row, an unreadable calendar or official file, or an empty peer / VIX stream.
    pub fn prepare(row: &InstrRow, peer: &InstrRow, ctx: &Ctx, store: &Path) -> Result<Self, String> {
        if row.cal_kind != CalKind::IdxCal || row.peer != peer.underlying {
            return Err(format!("{}: index dials need an INDEX row and its X-PEER row ({}), got {}", row.underlying, row.peer, peer.underlying));
        }
        let (peer1, n1) = read_1min(store, &peer.zsymbol, row.from, row.to);
        let (vix1, n2) = read_1min(store, &row.vix, row.from, row.to);
        if peer1.is_empty() || vix1.is_empty() {
            return Err(format!("index dials: an empty stream (peer {} 1min {}, {} 1min {})", peer.zsymbol, peer1.len(), row.vix, vix1.len()));
        }
        let pb = crate::data::bhav_index(&peer.bhav)?;
        let peer_prev: Vec<Option<i64>> = (0..ctx.sessions.len()).map(|i| i.checked_sub(1).and_then(|j| ctx.sessions.get(j)).and_then(|s| pb.get(&s.day)).map(|b| b.close)).collect();
        let days: Vec<i32> = ctx.sessions.iter().map(|s| s.day).collect();
        let late: Vec<(i32, i32)> = dials::LATE_CLOSURES.iter().map(|(c, a)| parse_ymd(c).zip(parse_ymd(a)).ok_or_else(|| format!("late closure row {c} {a}"))).collect::<Result<_, _>>()?;
        let mend = dials::month_end_flags(&days, &late);
        let hl = (0..ctx.sessions.len()).map(|i| dials::hl52(ctx, i)).collect();
        let (atr, atr_holes) = dials::daily_atr(ctx);
        let cal = IdxCal::parse(&std::fs::read_to_string(&row.cal).map_err(|e| format!("{}: {e}", row.cal.display()))?, &ctx.sessions)?;
        Ok(Self { peer1, vix1, peer_prev, mend, hl, atr, atr_holes, cal, notes: n1.into_iter().chain(n2).collect() })
    }

    /// The readings of the decision candle ending at `end` on session `i` (day `day`), with the session's range so far `hi` / `lo`
    /// on the decision stream (the decision candle included).
    #[must_use]
    pub fn read_one(&self, i: usize, day: i32, end: i64, hi: i64, lo: i64) -> IdxRead {
        let hlv = self.hl.get(i).copied().flatten();
        IdxRead {
            ctx: CtxRead {
                nifty_bp: dials::change_bp(dials::last_close_at(&self.peer1, day, end), self.peer_prev.get(i).copied().flatten()),
                vix: dials::last_close_at(&self.vix1, day, end),
                exp: self.cal.d_exp(day),
                mend: self.mend.get(i).copied().flatten(),
                h52: hlv.map(|x| x.0),
                l52: hlv.map(|x| x.1),
                drng: dials::drng(hi, lo, self.atr.get(i).copied().flatten()),
            },
            dte: self.cal.dte(day),
        }
    }

    /// Every candle of the decision stream `stream` read at its END (the minute driver's form; the seconds driver calls
    /// [`IdxPrep::read_one`] per candidate with its own range so far): (session index, candle END, readings).
    #[must_use]
    pub fn read_stream(&self, ctx: &Ctx, stream: &Stream) -> Vec<(usize, i64, IdxRead)> {
        dials::day_range_so_far(stream)
            .into_iter()
            .filter_map(|(i, end, hi, lo)| ctx.sessions.get(i).map(|s| (i, end, self.read_one(i, s.day, end, hi, lo))))
            .collect()
    }
}

/// The D-DTE dial's value word (`0` .. `4`, `5+`, `unknown`).
#[must_use]
pub fn dte_word(d: &Dte) -> String {
    match d.value {
        Some(5) => "5+".to_owned(),
        Some(v) => v.to_string(),
        None => "unknown".to_owned(),
    }
}
