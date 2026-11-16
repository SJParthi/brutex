//! S8 of SNIPER-IDXOPT-PLAN-v3.1 (INDEX S8 builder, main 5b507203, 15 Nov 2026): the POSITIONAL legs of the shared option pricer
//! for IDXOPT (plan s4.2, A3 with R5, B1 with R2, B6; s4.5 free cash), on 1-s blocks of the option's own seconds:
//!
//! * the ENTRY: the A2 / s23a ITM-fit contract of the current expiry as known (DP-IDXCAL; at or after 15:00:00 on an expiry day the
//!   next expiry), sized C11 closest of the stream's slot in that contract against free cash (R5: the CONTRACT is the Rs 5,00,000
//!   pick; the QUANTITY is the slot), each second's HIGH, the share dial, volume alone, REQ 70a pay from free cash (Q17);
//! * the ROLL on the held contract's TRUE expiry day (as known: DP-IDXCAL `series` + `d_exp`) at 15:00:00: the CLOSE leg sells the
//!   held units by each second's LOW; a close above that day's one-order maximum is `close_split k` = `freeze_slices(units,
//!   max_one_order)` (R2), the orders sent at once and filled as ONE quantity (crate S18); the OPEN leg (headline (a) SEQUENTIAL)
//!   is decided at t2 = the END of the second in which the close's LAST unit filled, re-reads the spot and the next expiry's ITM
//!   fit at t2 (prints strictly before t2 only), and is sized against free cash that now holds the close proceeds; beside, (b)
//!   SAME DECISION at 15:00:00 against free cash with the close not yet released (`roll_refused_cash` counted). No open leg is
//!   decided at or after the index close 15:30:00 (`roll_close_incomplete`);
//! * the forced EXIT and carried exits (a non-expiry day carries the unfilled rest to the next session's first block; an expiry
//!   day settles it);
//! * LEFTOVERS at the dated F&O end of an expiry day: the WORSE of the settlement at intrinsic against the official index close
//!   (crate S15 bullet 3, `limits::settle_intrinsic`, the dated exercise law of `costs::option_clock`) and S15 bullet 1 = the worst
//!   traded price in [W0, F&O end), W0 = the first block of the first closing order sent THAT session (never an E0 of an earlier
//!   day, B6); the bullet-1 reading carries its own sale order (check-build-idx D1);
//! * charges per executed order at that order's own day (lane B R17): Dhan's estimator (the headline; buy side at the buy day's
//!   rates, sell side per sell day, the dearer of the dated total and today's total) and the per-leg reading beside (every slice an
//!   order); the never-past-expiry guard before every leg (a failure is an error, never a refusal);
//! * one account ledger for several streams (s4.5 base Q7: free(t) = account + min(0, realised) - premium held - live buy orders'
//!   reservations; money released when the trip's last fill is known); events in time order, ties by stream order (LONG before
//!   SHORT, then NIFTY before BANKNIFTY: the stream list's order).
//!
//! K11 / B-mutants (each turns a test red; plan s4.7 (17)): `m_idx_spot` (moneyness from a second not yet ended, = m-IX14),
//! `m_idx_roll_lead` (the roll-open pick reads prints at or after t2), `m_idx_settle` (expiry leftovers valued at the last print),
//! `m_idx_redate` (a held series rolled on its stale listed date), `m_idx_folder` (the expiry read from the store folder's name),
//! `m_idx_settle_window` (S15 bullet 1 from an E0 of an earlier day), `m_idx_roll_funds` (a sequential roll-open funded by close
//! units not yet filled), `m_idx_close_one_order` (a close forced into one order above that day's maximum), `m_idx_roll_late` (a
//! leg after the held expiry).
//!
//! s8b (S8 BUILDER, wave 4, main 5b507203, 15 Nov 2026), the fixes of check-build-idx-S7-S10.md applied word for word, and MAIN's
//! ruling 2 of MAIN-RULINGS-S7-S10-20261115.md:
//! * **D-S8-1** (plan s4.2 (4) / F4, s4.7 (7) bullets 2-3, s4.2 (5)): every buy order (entry, roll-open) walks to min(its stop, the
//!   stream's next exit decision); at `Ev::Exit` a buy order still live there is cut at the decision, so only the units known by
//!   then are held, and its cancelled rest is counted (`entry_cut_by_exit` / `open_cut_by_flip`, units beside); at `Ev::Exit` with a
//!   pending `Ev::Open` the open is cancelled and counted (`roll_cancelled_by_flip`); an entry still filling at 15:00:00 on its
//!   expiry day has its rest cancelled at the roll (`entry_rest_cancelled_at_roll`, units beside); its filled units roll.
//! * **D-S8-2** (plan s4.2 (3), B6): the bullet-1 window starts at W0 = min(E0, the closing order's decision) when the position was
//!   entered that same session (`ist_day(e0) == d`), else at the closing order's decision.
//! * **MIXED rolls** (ruling 2; plan s4.5 amended): the roll closes of ONE instant in the account are one group; every close of the
//!   instant is settled first, and only then is each re-open decided, in stream order, at the instant the LAST of them is known
//!   ((a): the latest t2; (b): 15:00:00 itself). Each re-open is C11 closest of its slot against free cash under base Q7 (Rs 20 L plus
//!   min(0, the net so far): lost money never reused, profits never added); a re-open its slot cannot fund is refused and counted
//!   (`roll_refused_cash`; `cash_short` for a cut count). Under (a), if any close of the instant does not finish before the index
//!   close, no re-open of that instant is decided (`roll_wait_close_incomplete` on the others).
//! * Named mutants: `m_idx_exit_cut` (the buy walk ignores the stream's exit decision: the D-S8-1 cut removed), `m_idx_flip_open`
//!   (a pending roll-open survives the flip), `m_idx_w0_same_session` (W0 always the closing order's decision: D-S8-2 removed),
//!   `m_idx_roll_group` (each re-open decided at its own close's t2: the order dependence of 2025-09-30 back).
//! * (s8b) `SlotRule::BaseOverN` was beside only then; the W4 REPAIR below makes it the headline.
//!
//! W4 REPAIR (check-wave4-S9.md D-W4-1 / D-W4-2 / D-W4-3, the checker's exact fixes; relaunch-13 copy for MAIN, 16 Nov 2026):
//! * **D-W4-1, the data end** (plan s4.3a B2 / s4.4 "Data end" / s4.7 (15)): a stream with NO exit before the data end carries the
//!   sentinel exit [`DATA_END`]; no `Ev::Exit` is queued for it. When every event is done, a stream with the sentinel still holding
//!   units is `open_at_data_end`: EVERY held unit is valued at the lowest LOW of the LAST session (5 paisa when none) in [W0, that
//!   session's F&O end), W0 = E0 when the trip was bought that session, else its first block 09:15:00, charged as one sale order (the
//!   dated estimator), `closed_at` = that F&O end, flagged `open_at_data_end` (+ units); never `open_after_run`, never an error. An exit
//!   or carry on the last session whose rest cannot sell keeps the S7 path. Mutant `m_idx_data_end_standin`: the final4 stand-in exit
//!   (the last session's index close, or 1 s after the entry when later) comes back.
//! * **D-W4-2, Muhurat carries** (plan s4.4 / Q39; S1 MUHURAT-LIST "positions are held through"): an exit or carry rest unsold on a
//!   non-expiry day is carried to [`Sessions::next_carry`], the next session that is NOT a Muhurat session (the special Saturday
//!   sessions stay carry targets: exits allowed where the option prints). Mutant `m_idx_carry_muhurat`: the Muhurat session again.
//! * **D-W4-3, one slot rule** (plan s5.2; MAIN-RULINGS-S7-S10 item 2 "each slot is base / N"): [`SlotRule::BaseOverN`] is the
//!   `#[default]` headline, read by this module AND by [`crate::acct`] through ONE formula, [`SlotRule::slot`]; the fixed Rs 20 L / N
//!   slot is the beside reading (`g-roll --slot-rule fixed`).
//!
//! W4 REPAIR r3 (check-wave4-S9-r2.md D-R2-1, the checker's exact fix; MAIN ruling of 16 Nov 2026 ~14:3x IST, main a3019c01):
//! * **D-R2-1, the data-end valuation on a hole**: a data-end valuation reads the prints of [W0, the last session's F&O end); when a
//!   DATA-law hole of the contract-day overlaps that window (the span test `walk_order` applies to orders), the trip is a hole trade:
//!   [`TripOut::hole`], counted `data_end_hole` (+ `data_end_hole_units`) in the stream's flags, on both data-end paths (the sentinel loop
//!   and the exit or carry on the last session). Its value is unchanged. The host's flag word carries it as HOLE (AFR-2: the second
//!   reading leaves a winning one out). Mutant `m_idx_data_end_nohole`: the span test skipped.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::PathBuf;

use brutex_core::vendor::Vendor;
use common_fill::legs::freeze_slices;
use common_fill::limits::{self, SizeWhy, settle_intrinsic, size_units_closest};
use common_fill::std70::{Kind, Order};
use common_fill::tape::{SecIndex, Seg, Tape};
use common_fill::{Side, US_S};
use costs::option_clock::{self as oc, Trip};
use idx_cal::day::{self, Day};
use idx_cal::storeio::{self, at_sod};
use store::path::Timeframe;

use crate::chain::{self, Folders, ItmCand, ItmPick};
use crate::newpx;
use crate::freeze::FreezeTable;
use crate::ladder::Ladder;
use crate::price::{self, Leg};
use crate::secs::Sec;
use crate::sess::Sessions;

/// One DP-IDXCAL row (S1): the file read, the listed alias and effective date of the current expiry as known, its series, the
/// next expiry's effective date, and whether the session is that expiry's day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CalRow {
    /// The bhavcopy file read (dated before the session).
    pub file_read: Day,
    /// The current expiry's listed alias in that file.
    pub known_listed: Day,
    /// Its effective date (holiday rule) as known.
    pub known_eff: Day,
    /// Its series id (IDXCAL-SERIES).
    pub series: u32,
    /// The next expiry's effective date as known.
    pub next_eff: Option<Day>,
    /// The session is the current expiry's day (known_eff == session).
    pub d_exp: bool,
}

/// One series (IDXCAL-SERIES): its aliases (listed date, first file, last file) and its true last trading day.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SeriesRow {
    /// Id.
    pub id: u32,
    /// (listed date, first file, last file), in listing order.
    pub aliases: Vec<(Day, Day, Day)>,
    /// The last file listing it under any alias.
    pub true_last: Day,
}

impl SeriesRow {
    /// The final listed date (the store's folders carry it).
    #[must_use]
    pub fn final_date(&self) -> Day {
        self.aliases.last().map_or(0, |a| a.0)
    }
}

/// Parses `DP-IDXCAL-<IX>.tsv`.
///
/// # Errors
/// A malformed row.
pub fn parse_cal(text: &str) -> Result<BTreeMap<Day, CalRow>, String> {
    let mut m = BTreeMap::new();
    for l in text.lines() {
        if l.starts_with('#') || l.starts_with("session") || l.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let p = |k: usize| c.get(k).and_then(|x| day::parse(x));
        let (Some(s), Some(f), Some(kl), Some(ke)) = (p(0), p(2), p(3), p(4)) else { return Err(format!("DP-IDXCAL row: {l}")) };
        let series: u32 = c.get(5).and_then(|x| x.parse().ok()).ok_or_else(|| format!("DP-IDXCAL series: {l}"))?;
        m.insert(s, CalRow { file_read: f, known_listed: kl, known_eff: ke, series, next_eff: p(6), d_exp: c.get(9) == Some(&"1") });
    }
    Ok(m)
}

/// Parses `IDXCAL-SERIES-<IX>.tsv`.
///
/// # Errors
/// A malformed row.
pub fn parse_series(text: &str) -> Result<Vec<SeriesRow>, String> {
    let mut v = Vec::new();
    for l in text.lines() {
        if l.starts_with('#') || l.starts_with("series") || l.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        let id: u32 = c.first().and_then(|x| x.parse().ok()).ok_or_else(|| format!("series id: {l}"))?;
        let aliases: Vec<(Day, Day, Day)> = c
            .get(1)
            .unwrap_or(&"")
            .split(" -> ")
            .map(|a| Some((day::parse(a.get(0..10)?)?, day::parse(a.get(12..22)?)?, day::parse(a.get(24..34)?)?)))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| format!("series aliases: {l}"))?;
        let true_last = c.get(2).and_then(|x| day::parse(x)).ok_or_else(|| format!("series true_last: {l}"))?;
        v.push(SeriesRow { id, aliases, true_last });
    }
    Ok(v)
}

/// Parses `INDEX-CLOSE-<IX>.tsv` (the `official` column, paisa).
///
/// # Errors
/// A malformed row.
pub fn parse_close(text: &str) -> Result<BTreeMap<Day, i64>, String> {
    let mut m = BTreeMap::new();
    let mut col = None;
    for l in text.lines() {
        if l.starts_with('#') || l.trim().is_empty() {
            continue;
        }
        let c: Vec<&str> = l.split('\t').collect();
        if l.starts_with("day\t") {
            col = c.iter().position(|h| *h == "official");
            continue;
        }
        let k = col.ok_or("INDEX-CLOSE: no official column")?;
        let d = c.first().and_then(|x| day::parse(x)).ok_or_else(|| format!("INDEX-CLOSE row: {l}"))?;
        let v: i64 = c.get(k).and_then(|x| x.parse().ok()).ok_or_else(|| format!("INDEX-CLOSE official: {l}"))?;
        m.insert(d, v);
    }
    Ok(m)
}

/// One fill row: (block start, units, block volume, price, known end).
pub type FillRow = (i64, u64, u64, i64, i64);

/// The contract-day rows read so far: (folder, day) -> rows (None: no month file).
type OptCache = BTreeMap<(String, Day), Option<Vec<Sec>>>;

/// A trip.s buy fills (known, cost) and the instant its money is released.
type HeldFills = (Vec<(i64, i64)>, Option<i64>);

/// Everything one index.s positional legs read (read only).
pub struct Book {
    /// The index (`NIFTY`, `BANKNIFTY`).
    pub index: String,
    /// DP-IDXCAL.
    pub cal: BTreeMap<Day, CalRow>,
    /// IDXCAL-SERIES.
    pub series: Vec<SeriesRow>,
    /// LISTED-CONTRACTS-IDX.
    pub ladder: Ladder,
    /// The store's option folders.
    pub folders: Folders,
    /// The dated freeze table.
    pub freeze: FreezeTable,
    /// The official index close per session.
    pub close: BTreeMap<Day, i64>,
    /// The session facts (D5).
    pub sess: Sessions,
    /// The store root (None: the injected rows only, unit tests).
    pub store: Option<PathBuf>,
    opt: RefCell<OptCache>,
    spot: RefCell<BTreeMap<Day, Vec<Sec>>>,
    /// RESUMED WAVE 4: the PRODUCTION option source (DP-NEWPX-IDXOPT + DP-NEWPX-FAILS under the DATA law, [`crate::newpx`]);
    /// None = the store's 1-s bars (the G-ROLL / G-EQ2 comparison source, byte-identical to S8b).
    newpx: Option<NewpxSrc>,
    holes: RefCell<HoleCache>,
    /// RESUMED WAVE 4: the decision reads (ITM-fit candidates) from the store while the production source is set.
    opt_store: RefCell<OptCache>,
}

/// Holes per contract-day (the DATA law's).
type HoleCache = BTreeMap<(String, Day), Vec<(i64, i64)>>;

/// RESUMED WAVE 4: the production option source of one index: the pack's `newpx-<IX>` folder (month folders of day files) and
/// every month's `DP-NEWPX-FAILS` rows. Day files are decoded once each, on first touch.
pub struct NewpxSrc {
    /// The month folders' parent (`.../newpx2-<IX>`): a day's file is `<dir>/<YYYY-MM>/<YYYY>/<day>.newpx.zst`.
    pub dir: PathBuf,
    /// The sidecar rows of every month.
    pub fails: newpx::Fails,
    days: RefCell<BTreeMap<Day, Option<newpx::NewpxDay>>>,
    /// Contract-days by how their rows were made (the report's counts): Agree, Seconds, WholeHole, Empty.
    pub counts: RefCell<BTreeMap<&'static str, u64>>,
}

impl NewpxSrc {
    /// The source over `dir` with its sidecar rows.
    #[must_use]
    pub fn new(dir: PathBuf, fails: newpx::Fails) -> Self {
        Self { dir, fails, days: RefCell::new(BTreeMap::new()), counts: RefCell::new(BTreeMap::new()) }
    }

    /// One contract-day's production rows ([`newpx::day_rows`]).
    ///
    /// # Errors
    /// A malformed day file.
    pub fn day_rows(&self, folder: &str, d: Day, from: i64, to: i64, halt: Option<(i64, i64)>) -> Result<newpx::DayRows, String> {
        if !self.days.borrow().contains_key(&d) {
            let month = self.dir.join(day::fmt(d).get(0..7).unwrap_or("0000-00"));
            let f = newpx::NewpxDay::open(&month, d)?;
            self.days.borrow_mut().insert(d, f);
        }
        let days = self.days.borrow();
        let blk = days.get(&d).and_then(Option::as_ref).and_then(|f| f.get(folder));
        let r = newpx::day_rows(d, blk, self.fails.get(folder, d), from, to, halt);
        let k = match r.src {
            newpx::Src::Agree => "agree",
            newpx::Src::Seconds(_) => "seconds",
            newpx::Src::WholeHole(..) => "whole_hole",
            newpx::Src::Empty => "empty",
            newpx::Src::NotInPack => "not_in_pack",
        };
        *self.counts.borrow_mut().entry(k).or_insert(0) += 1;
        Ok(r)
    }
}

impl Book {
    /// A book.
    #[must_use]
    #[allow(clippy::too_many_arguments, reason = "one book of read-only inputs")]
    pub fn new(index: &str, cal: BTreeMap<Day, CalRow>, series: Vec<SeriesRow>, ladder: Ladder, folders: Folders, freeze: FreezeTable, close: BTreeMap<Day, i64>, sess: Sessions, store: Option<PathBuf>) -> Self {
        Self { index: index.to_string(), cal, series, ladder, folders, freeze, close, sess, store, opt: RefCell::new(BTreeMap::new()), spot: RefCell::new(BTreeMap::new()), newpx: None, holes: RefCell::new(BTreeMap::new()), opt_store: RefCell::new(BTreeMap::new()) }
    }

    /// RESUMED WAVE 4: the same book reading the PRODUCTION source (DP-NEWPX-IDXOPT under the DATA law) instead of the store.
    #[must_use]
    pub fn with_newpx(mut self, src: NewpxSrc) -> Self {
        self.newpx = Some(src);
        self
    }

    /// The production source, when set.
    #[must_use]
    pub const fn newpx(&self) -> Option<&NewpxSrc> {
        self.newpx.as_ref()
    }

    /// The DATA law's holes of a contract-day (empty on the store source or a clean day).
    #[must_use]
    pub fn holes_of(&self, folder: &str, d: Day) -> Vec<(i64, i64)> {
        self.holes.borrow().get(&(folder.to_string(), d)).cloned().unwrap_or_default()
    }

    /// Injects a contract-day's rows (unit tests; a real run reads the store).
    pub fn put_opt(&self, folder: &str, d: Day, rows: Option<Vec<Sec>>) {
        self.opt.borrow_mut().insert((folder.to_string(), d), rows);
    }

    /// Injects a day's index seconds (unit tests).
    pub fn put_spot(&self, d: Day, rows: Vec<Sec>) {
        self.spot.borrow_mut().insert(d, rows);
    }

    /// A contract-day's 1-s rows in [09:15:00, the dated F&O end), valid prints only, the derivatives halt cut.
    ///
    /// # Errors
    /// No F&O-end row (F13), a refused store month.
    pub fn opt_rows(&self, folder: &str, d: Day) -> Result<Option<Vec<Sec>>, String> {
        if let Some(r) = self.opt.borrow().get(&(folder.to_string(), d)) {
            return Ok(r.clone());
        }
        let end = self.sess.fno_end(d)?;
        if let Some(src) = &self.newpx {
            // the PRODUCTION source: the DATA law over the pack (a contract-day outside DP-IDXNEED is refused by name)
            let r = src.day_rows(folder, d, at_sod(d, 9 * 3600 + 15 * 60), end, self.sess.halt_der(d))?;
            if r.src == newpx::Src::NotInPack {
                return Err(format!("not_in_pack {folder} {} (DP-IDXNEED does not hold this contract-day)", day::fmt(d)));
            }
            self.holes.borrow_mut().insert((folder.to_string(), d), r.holes);
            self.opt.borrow_mut().insert((folder.to_string(), d), Some(r.rows.clone()));
            return Ok(Some(r.rows));
        }
        let got = self.store_read(folder, d, end)?;
        self.opt.borrow_mut().insert((folder.to_string(), d), got.clone());
        Ok(got)
    }

    /// The store's 1-s rows of a contract-day (valid prints, the derivatives halt cut), uncached.
    fn store_read(&self, folder: &str, d: Day, end: i64) -> Result<Option<Vec<Sec>>, String> {
        Ok(match &self.store {
            None => None,
            Some(root) => {
                let to = (end - at_sod(d, 0)) / US_S;
                let h = self.sess.halt_der(d);
                storeio::day_bars(root, Vendor::Gdfl, "FNO", &self.index, Some(folder), Timeframe::SECOND_1, d, 9 * 3600 + 15 * 60, to)?.map(|bars| {
                    bars.iter()
                        .filter(|b| b.volume > 0 && b.high > 0 && b.low > 0 && b.high >= b.low)
                        .filter(|b| !h.is_some_and(|(a, z)| b.ts_micros >= a && b.ts_micros < z))
                        .map(|b| Sec { ts: b.ts_micros, high: b.high, low: b.low, close: b.close, vol: u64::try_from(b.volume).unwrap_or(0) })
                        .collect()
                })
            }
        })
    }

    /// RESUMED WAVE 4: the rows a DECISION reads (the ITM-fit candidates' prints before t, as DP-IDXITM reads them): the STORE's
    /// 1-s bars, cached apart, when the production source is set (the pack holds only the contract-days the rules can FILL, not
    /// every candidate); [`Book::opt_rows`] otherwise (and in unit tests with no store).
    ///
    /// # Errors
    /// No F&O-end row (F13), a refused store month.
    pub fn decide_rows(&self, folder: &str, d: Day) -> Result<Option<Vec<Sec>>, String> {
        if self.newpx.is_none() || self.store.is_none() {
            return self.opt_rows(folder, d);
        }
        if let Some(r) = self.opt_store.borrow().get(&(folder.to_string(), d)) {
            return Ok(r.clone());
        }
        let end = self.sess.fno_end(d)?;
        let got = self.store_read(folder, d, end)?;
        self.opt_store.borrow_mut().insert((folder.to_string(), d), got.clone());
        Ok(got)
    }

    /// The index's GDFL seconds of `d` in [09:15:00, the F&O end), the index halt cut.
    ///
    /// # Errors
    /// No F&O-end row, a refused store month.
    pub fn spot_rows(&self, d: Day) -> Result<Vec<Sec>, String> {
        if let Some(r) = self.spot.borrow().get(&d) {
            return Ok(r.clone());
        }
        let end = self.sess.fno_end(d)?;
        let got: Vec<Sec> = match &self.store {
            None => Vec::new(),
            Some(root) => {
                let to = (end - at_sod(d, 0)) / US_S;
                let h = self.sess.halt_idx(d);
                storeio::day_bars(root, Vendor::Gdfl, "INDEX", &self.index, None, Timeframe::SECOND_1, d, 9 * 3600 + 15 * 60, to)?
                    .unwrap_or_default()
                    .iter()
                    .filter(|b| !h.is_some_and(|(a, z)| b.ts_micros >= a && b.ts_micros < z))
                    .map(|b| Sec { ts: b.ts_micros, high: b.high, low: b.low, close: b.close, vol: u64::try_from(b.volume.max(0)).unwrap_or(0) })
                    .collect()
            }
        };
        self.spot.borrow_mut().insert(d, got.clone());
        Ok(got)
    }

    /// The series listing `alias` in file `file`.
    #[must_use]
    pub fn series_of(&self, alias: Day, file: Day) -> Option<&SeriesRow> {
        self.series.iter().find(|s| s.aliases.iter().any(|&(a, f0, f1)| a == alias && f0 <= file && file <= f1))
    }

    /// The listed alias in `file` whose effective date is `eff` (the alias itself when listed; else within 3 days after: the
    /// holiday rule's listed date).
    #[must_use]
    pub fn alias_in(&self, file: Day, eff: Day) -> Option<Day> {
        if self.ladder.get(&self.index, file, eff).is_some() {
            return Some(eff);
        }
        (1..=3).map(|k| eff + k).find(|x| self.ladder.get(&self.index, file, *x).is_some())
    }
}

/// The open-leg rule of a roll (A3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OpenRule {
    /// (a) the headline: decided at t2 with the close proceeds released.
    Sequential,
    /// (b) beside: decided at 15:00:00 with the close not yet released.
    SameDecision,
}

/// W4 REPAIR (D-W4-1): the sentinel exit of a stream with NO exit decision before the data end (plan s4.3a B2): no `Ev::Exit` is
/// queued; units still held when every event is done are `open_at_data_end`, valued at the worst print of the last session.
pub const DATA_END: i64 = i64::MAX;

/// One stream (a forced position of one index and side).
#[derive(Clone, Debug)]
pub struct Stream {
    /// Which book (index).
    pub book: usize,
    /// CALL (long) or PUT (short).
    pub ce: bool,
    /// The entry decision instant (UTC micros; a candle END).
    pub entry: i64,
    /// The forced exit decision instant; [`DATA_END`] = no exit before the data end (W4 REPAIR, D-W4-1).
    pub exit: i64,
    /// The stream's slot (Rs 20 L / N), paisa: it carries the stream's N (N = account / slot); [`SlotRule::slot`] sizes the buys.
    pub slot: i64,
    /// Label.
    pub label: String,
}

/// THE slot rule of the one account (s8b; W4 REPAIR D-W4-3: ONE rule, read by [`run`] and by [`crate::acct`] alike).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlotRule {
    /// Beside (`g-roll --slot-rule fixed`): the stream's fixed slot Rs 20 L / N (plan s4.2 (2) as first written; REQ 66a: every trade
    /// the same amount); free cash under base Q7 caps it.
    Fixed,
    /// THE HEADLINE (W4 REPAIR D-W4-3; plan s5.2, MAIN-RULINGS-S7-S10 item 2: "the base is Rs 20 L minus net losses so far ... each
    /// slot is base / N"): base = account + min(0, the net known at the decision), slot = base / N; profits never added.
    #[default]
    BaseOverN,
}

impl SlotRule {
    /// The slot of one of `n` streams of an account of `account` paisa when the net of the trades whose money is released by the
    /// decision is `realised` (the ONE formula: `opt_pos::pos` and `opt_pos::acct` both call it): base / N under the headline
    /// (base = account + min(0, realised), never below 0), account / N under the fixed reading.
    #[must_use]
    pub fn slot(self, account: i64, n: i64, realised: i64) -> i64 {
        let n = n.max(1);
        match self {
            Self::Fixed => account / n,
            Self::BaseOverN => (account + realised.min(0)).max(0) / n,
        }
    }

    /// The rule's printed name (SN1, the pick lines, g-roll).
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fixed => "fixed Rs 20 L / N (beside)",
            Self::BaseOverN => "base / N, base = Rs 20 L + min(0, the net known) (MAIN-RULINGS-S7-S10 item 2; the headline)",
        }
    }

    /// The rule's short tag (a table column, as the cash path's `qty_rule`).
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Fixed => "fixed",
            Self::BaseOverN => "base_over_n",
        }
    }
}

/// One run's rows.
#[derive(Clone, Debug)]
pub struct Spec {
    /// The streams, in tie order.
    pub streams: Vec<Stream>,
    /// The account (Rs 20 L), paisa.
    pub account: i64,
    /// The share dial, bp.
    pub bp: u16,
    /// Protection kept (true) or off.
    pub kept: bool,
    /// The roll's open rule.
    pub open_rule: OpenRule,
    /// The A2 fit amount (Rs 5,00,000), paisa.
    pub target: i64,
    /// The slot rule (W4 REPAIR D-W4-3: the headline is `SlotRule::default()` = `SlotRule::BaseOverN`).
    pub slot_rule: SlotRule,
}

/// The A2 pick facts of a buy leg.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PickInfo {
    /// The spot read (the index's last second strictly before the decision).
    pub spot: i64,
    /// ITM depth.
    pub depth: u32,
    /// The size reference (HIGH of the last traded second before t).
    pub reference: i64,
    /// Its age, s.
    pub ref_age_s: i64,
    /// Strikes skipped for no trade before t.
    pub skipped_no_trade: u32,
    /// Strikes skipped as too big for one order.
    pub skipped_big: u32,
    /// The A2 (Rs 5 L) closest quantity.
    pub qty_a2: u64,
    /// The listed alias of the expiry picked from.
    pub alias: Day,
    /// The file read.
    pub file: Day,
}

/// One leg (an order) with its fill rows.
#[derive(Clone, Debug, Default)]
pub struct LegOut {
    /// entry | roll_close | roll_open | exit | carry.
    pub kind: &'static str,
    /// Its session.
    pub day: Day,
    /// The decision instant (= the first block's start).
    pub decided: i64,
    /// The order's stop.
    pub stop: i64,
    /// The contract folder.
    pub folder: String,
    /// Strike, paisa.
    pub strike: i64,
    /// Series id.
    pub series: u32,
    /// The held contract's true expiry day.
    pub expiry: Day,
    /// Lot.
    pub lot: u64,
    /// That day's freeze.
    pub freeze: i64,
    /// That day's one-order maximum for the lot.
    pub max_order: i64,
    /// Units asked.
    pub want: u64,
    /// Units filled.
    pub units: u64,
    /// Their value, paisa.
    pub value: i64,
    /// REQ 70a units cancelled.
    pub unpaid: u64,
    /// The orders it was sent as (freeze_slices(want, max_order), R2): `close_split k` when k > 1 on a sell.
    pub slices: u64,
    /// The protection limit (kept path).
    pub limit: Option<i64>,
    /// The payable budget (buys), paisa.
    pub budget: Option<i64>,
    /// Free cash at the decision, paisa.
    pub free: i64,
    /// (block start, units, block volume, price, known end).
    pub fills: Vec<(i64, u64, u64, i64, i64)>,
    /// E0 (first filling block's start).
    pub e0: i64,
    /// The last fill's known end (t2 for a roll close).
    pub known_at: i64,
    /// A buy's pick.
    pub pick: Option<PickInfo>,
    /// RESUMED WAVE 4: the order spans a DATA-law hole (production source only; always false on the store source).
    pub hole: bool,
}

/// One held contract from its open to its close (a round trip for the charges).
#[derive(Clone, Debug, Default)]
pub struct TripOut {
    /// Folder.
    pub folder: String,
    /// Strike.
    pub strike: i64,
    /// CALL?
    pub ce: bool,
    /// The true expiry day.
    pub expiry: Day,
    /// Units bought, value, day, slices (per-leg reading).
    pub units_in: u64,
    /// Premium paid.
    pub value_in: i64,
    /// The buy day.
    pub buy_day: Day,
    /// Entry slices.
    pub buy_slices: u64,
    /// Sell orders: (day, value, slices).
    pub sells: Vec<(Day, i64, u64)>,
    /// Units left at the expiry day's F&O end.
    pub leftover: u64,
    /// W0 of the bullet-1 window.
    pub w0: i64,
    /// Bullet 1's price (the worst traded price in [W0, F&O end), 5 paisa when none).
    pub bullet1_px: i64,
    /// The official close used.
    pub settle_close: i64,
    /// The intrinsic value a unit.
    pub intrinsic: i64,
    /// Settled (the settlement was the worse reading).
    pub settled: bool,
    /// Net under the settlement reading.
    pub net_settle: i64,
    /// Net under bullet 1.
    pub net_bullet1: i64,
    /// The booked net (estimator, the dearer of the dated and today's totals).
    pub net: i64,
    /// The dated net.
    pub net_dated: i64,
    /// The booked charges.
    pub charges: i64,
    /// The per-leg net (every slice an order).
    pub net_per_leg: i64,
    /// The trip's last known instant (money released).
    pub closed_at: i64,
    /// S7 BUILDER wave 4 (plan v3.1 s4.3a B2 / s4.4 "Data end" / s4.7 (15)): the leftover is `open_at_data_end`, units still held
    /// after the LAST session of the data, valued at the worst print of that last session (`value_at_data_end`); never settled,
    /// never an expiry leftover. W4 REPAIR (D-W4-1): the whole position of a stream with the [`DATA_END`] sentinel (no exit before
    /// the data end), or the unsold rest of an exit or carry on the last session.
    pub data_end: bool,
    /// W4 REPAIR r3 (check-wave4-S9-r2.md D-R2-1): the data-end valuation (`value_at_data_end`) read a DATA-law hole: its window
    /// [W0, the last session's F&O end) overlaps a hole of the contract-day (the span test of `walk_order`; e.g. a contract-day listed
    /// `unchecked` in DP-NEWPX-FAILS, a whole-session hole), so the trip is a hole trade (AFR-2) like an order spanning one. Always false
    /// on the store source and for a trip closed by orders.
    pub hole: bool,
}

/// One stream's outcome.
#[derive(Clone, Debug, Default)]
pub struct StreamOut {
    /// Label.
    pub label: String,
    /// Legs in time order.
    pub legs: Vec<LegOut>,
    /// Trips.
    pub trips: Vec<TripOut>,
    /// Counted flags.
    pub flags: BTreeMap<&'static str, u32>,
    /// Sum of the booked trip nets.
    pub net: i64,
    /// Sum of the per-leg nets.
    pub net_per_leg: i64,
}

/// A run's outcome.
#[derive(Clone, Debug, Default)]
pub struct RunOut {
    /// Streams.
    pub streams: Vec<StreamOut>,
    /// The account's net.
    pub net: i64,
}

#[derive(Clone, Debug)]
struct Held {
    folder: String,
    strike: i64,
    series: u32,
    alias_open: Day,
    expiry: Day,
    lot: u64,
    units: u64,
    trip: usize,
    e0: i64,
}

#[derive(Clone, Debug, Default)]
struct Res {
    from: i64,
    until: i64,
    want: u64,
    refpx: i64,
    fills: Vec<(i64, u64)>,
}

#[derive(Default)]
struct Ledger {
    /// (stream, trip): (closed_at, net) once closed.
    closed: Vec<(i64, i64)>,
    /// Trips. buy fills: (known, cost) per (stream, trip key), and the instant the trip.s money is released (its last fill known).
    held: BTreeMap<(usize, usize), HeldFills>,
    res: Vec<Res>,
}

impl Ledger {
    /// The net of the trips whose last fill is known at or before `t` (s8b: the base of `SlotRule::BaseOverN`).
    fn realised(&self, t: i64) -> i64 {
        self.closed.iter().filter(|c| c.0 <= t).map(|c| c.1).sum()
    }

    fn free(&self, t: i64, account: i64) -> i64 {
        let realised: i64 = self.closed.iter().filter(|c| c.0 <= t).map(|c| c.1).sum();
        let held: i64 = self.held.values().filter(|(_, rel)| rel.is_none_or(|r| t < r)).flat_map(|(v, _)| v.iter()).filter(|f| f.0 <= t).map(|f| f.1).sum();
        let res: i64 = self
            .res
            .iter()
            .filter(|r| r.from <= t && t < r.until)
            .map(|r| {
                let got: u64 = r.fills.iter().filter(|f| f.0 <= t).map(|f| f.1).sum();
                i64::try_from(r.want.saturating_sub(got)).unwrap_or(0).saturating_mul(r.refpx)
            })
            .sum();
        account + realised.min(0) - held - res
    }
}

fn seg_of(b: &Book, d: Day) -> Result<Vec<Seg>, String> {
    let open = at_sod(d, 9 * 3600 + 15 * 60);
    let end = b.sess.fno_end(d)?;
    Ok(match b.sess.halt_der(d) {
        Some((a, z)) => vec![Seg { open, last_sec: a - US_S, auction_open: false }, Seg { open: z, last_sec: end - US_S, auction_open: false }],
        None => vec![Seg { open, last_sec: end - US_S, auction_open: false }],
    })
}

/// One order on one contract-day: the crate v2.1 walk on 1-s blocks.
#[allow(clippy::too_many_arguments, reason = "one order")]
fn walk_order(b: &Book, folder: &str, d: Day, side: Side, anchor: i64, stop: i64, lot: u64, limit: Option<i64>, want: u64, money: Option<i64>, bp: u16) -> Result<(common_fill::std70::LevelFill, Vec<FillRow>), String> {
    let rows = b.opt_rows(folder, d)?.ok_or_else(|| format!("no_option_rows {folder} {} (LATER refusal: a missing contract-day)", day::fmt(d)))?;
    // RESUMED WAVE 4: the DATA law's holes (none on the store source): an order spanning one carries the HOLE flag
    let holes = b.holes_of(folder, d);
    if rows.is_empty() || want == 0 {
        let hole_span = holes.iter().any(|&(a, z)| a < stop && z > anchor);
        return Ok((common_fill::std70::LevelFill { rest_at_stop: want, hole_span, ..Default::default() }, Vec::new()));
    }
    let open = at_sod(d, 9 * 3600 + 15 * 60);
    let end = b.sess.fno_end(d)?;
    let segs = seg_of(b, d)?;
    let ix = SecIndex::new(&rows, open, end).map_err(|e| format!("index {e:?}"))?;
    let tape = Tape::new(&rows, &ix, None).and_then(|t| t.with_segments(&segs)).and_then(|t| t.with_block(1)).map_err(|e| format!("tape {e:?}"))?.with_holes(&holes);
    let kind = if side == Side::Buy { Kind::Entry } else { Kind::SignalExit };
    let o = Order { side, anchor, stop: stop.min(end), unit: lot, limit, kind };
    let (fills, lf) = price::walk1(&tape, &o, want, money, bp, Leg::Entry)?;
    Ok((lf, fills.into_iter().map(|f| (f.1, f.2, f.3, f.4, f.5)).collect()))
}

/// The index's last second STRICTLY before `t` (m-IX14 / `m_idx_spot`: a second not yet ended is read).
fn spot_before(rows: &[Sec], t: i64) -> Option<i64> {
    let i = if cfg!(feature = "m_idx_spot") { rows.partition_point(|r| r.ts <= t) } else { rows.partition_point(|r| r.ts < t) };
    i.checked_sub(1).and_then(|j| rows.get(j)).map(|r| r.close)
}

/// The option's last print strictly before `t` (the roll-open pick's reading; `m_idx_roll_lead`: prints at or after t2 are read;
/// S7 BUILDER wave 4, lane B's P3 `m_pos_leader` mapped to the index ENTRY pick: prints at or after the entry decision are read).
fn rows_before(rows: &[Sec], t: i64, roll_open: bool) -> Vec<Sec> {
    if (roll_open && cfg!(feature = "m_idx_roll_lead")) || (!roll_open && cfg!(feature = "m_pos_leader")) {
        rows.iter().filter(|r| r.ts <= t).copied().collect()
    } else {
        rows.iter().filter(|r| r.ts < t).copied().collect()
    }
}

/// The A2 / s23a pick of (file, alias) at `t` on day `d` (chain::itm_fit on the nearest ITM strikes, widened until one fits);
/// a listed strike with no store folder is a candidate with no trade (counted, check-build-idx F-e).
#[allow(clippy::too_many_arguments, reason = "one pick")]
pub(crate) fn pick(b: &Book, d: Day, file: Day, alias: Day, folder_date: Day, ce: bool, t: i64, target: i64, roll_open: bool) -> Result<(ItmPick, PickInfo), String> {
    let row = b.ladder.get(&b.index, file, alias).ok_or_else(|| format!("no ladder row {} file {} alias {}", b.index, day::fmt(file), day::fmt(alias)))?;
    let spot_rows = b.spot_rows(d)?;
    let spot = spot_before(&spot_rows, t).ok_or_else(|| format!("no spot before {}", storeio::hms(storeio::ist_of(t).1)))?;
    let strikes = if ce { &row.ce } else { &row.pe };
    let mut itm: Vec<i64> = strikes.iter().copied().filter(|&k| if ce { k < spot } else { k > spot }).collect();
    if ce {
        itm.sort_by_key(|&k| std::cmp::Reverse(k));
    } else {
        itm.sort_unstable();
    }
    let freeze = b.freeze.units(&b.index, d).ok_or_else(|| format!("no_freeze_row {} {} (F13)", b.index, day::fmt(d)))?;
    let mut loaded: Vec<(i64, String, Vec<Sec>, u64)> = Vec::new();
    let mut n = 4usize;
    loop {
        while loaded.len() < n.min(itm.len()) {
            let Some(&k) = itm.get(loaded.len()) else { break };
            let name = storeio::contract_name(folder_date, k, ce);
            let rows = if b.folders.has(folder_date, k, ce) || b.store.is_none() { b.decide_rows(&name, d)?.unwrap_or_default() } else { Vec::new() };
            let pre = rows_before(&rows, t, roll_open);
            let lot = u64::try_from(chain::lot_at(row, &pre.iter().map(|x| x.vol).collect::<Vec<_>>()).0.max(0)).unwrap_or(0);
            loaded.push((k, name, pre, lot));
        }
        let cands: Vec<ItmCand<'_>> = loaded.iter().map(|(k, nm, r, l)| (*k, nm.as_str(), r.as_slice(), *l)).collect();
        // chain::itm_fit reads "the last traded second before t" from the rows given: they are already cut before t (or at t
        // under m_idx_roll_lead), so it is called with t + 1 s to keep its own strict cut from hiding the mutant.
        match chain::itm_fit(&cands, ce, spot, t + US_S, target, None, freeze) {
            Ok(p) => {
                let info = PickInfo { spot, depth: p.depth, reference: p.reference, ref_age_s: p.ref_age_s.saturating_sub(1), skipped_no_trade: p.skipped_no_trade, skipped_big: p.skipped_big, qty_a2: p.qty, alias, file };
                return Ok((p, info));
            }
            Err(e) if loaded.len() >= itm.len() => return Err(e),
            Err(_) => n *= 2,
        }
    }
}

/// The protection reference: the contract's last print STRICTLY before the order's anchor (lane B's P3 (d); mutant `m_pos_ref`: the FIRST
/// print at or after the anchor is read).
fn last_close_before(rows: &[Sec], t: i64) -> Option<i64> {
    let i = rows.partition_point(|r| r.ts < t);
    if cfg!(feature = "m_pos_ref") {
        return rows.get(i).or_else(|| i.checked_sub(1).and_then(|j| rows.get(j))).map(|r| r.close);
    }
    i.checked_sub(1).and_then(|j| rows.get(j)).map(|r| r.close)
}

/// The roll day of the held contract from `from` on (DP-IDXCAL as known: the session where its series is current and is that
/// expiry's day); the mutants read a stale listed date, the folder's name, or roll one session late.
fn roll_day(b: &Book, h: &Held, from: Day) -> Option<Day> {
    let folder_day = h.folder.get(0..10).and_then(day::parse);
    let mut late = false;
    for &d in b.sess.days.iter().filter(|&&d| d >= from) {
        let hit = if cfg!(feature = "m_idx_redate") {
            d == h.alias_open
        } else if cfg!(feature = "m_idx_folder") {
            Some(d) == folder_day
        } else {
            b.cal.get(&d).is_some_and(|r| r.series == h.series && r.d_exp)
        };
        if cfg!(feature = "m_idx_roll_late") {
            if late {
                return Some(d);
            }
            late = hit;
            continue;
        }
        if hit {
            return Some(d);
        }
    }
    None
}

fn guard(h: &Held, d: Day) -> Result<(), String> {
    if d > h.expiry {
        return Err(format!("NEVER-PAST-EXPIRY guard: a leg on {} after the held contract's true expiry {} ({}) (build error, never a refusal)", day::fmt(d), day::fmt(h.expiry), h.folder));
    }
    Ok(())
}

/// The charges of one trip at the dearer of the dated total (each side at its own day's rates) and today's total: (booked
/// charges, dated charges). `extra` is a leftover reading: Some((day, value, settled, intrinsic value, notional)).
fn est_charges(t: &TripOut, extra: Option<(Day, i64, bool, i64, i64)>) -> Result<(i64, i64), String> {
    let qty = i64::try_from(t.units_in).unwrap_or(0);
    let rt = oc::rates_on(oc::today())?;
    let buy = Trip { qty, buy_value: t.value_in, entry_orders: 1, ..Trip::default() };
    let mut by_day: BTreeMap<Day, Trip> = BTreeMap::new();
    for &(d, v, _) in &t.sells {
        let e = by_day.entry(d).or_insert(Trip { qty, ..Trip::default() });
        e.sell_value += v;
        e.exit_orders += 1;
    }
    if let Some((d, v, settled, sv, sn)) = extra {
        let e = by_day.entry(d).or_insert(Trip { qty, ..Trip::default() });
        e.sell_value += v;
        if settled {
            e.rem_kind = oc::REM_SETTLED;
            e.settle_value = sv;
            e.settle_notional = sn;
        } else {
            e.exit_orders += 1;
        }
    }
    let rb = oc::rates_on(i64::from(t.buy_day))?;
    let mut dated = oc::charges_est(&rb, &buy).0;
    let mut today = oc::charges_est(&rt, &buy).0;
    for (d, s) in &by_day {
        let rd = oc::rates_on(i64::from(*d))?;
        dated += oc::charges_est(&rd, s).1;
        today += oc::charges_est(&rt, s).1;
    }
    Ok((dated.max(today), dated))
}

/// The per-leg charges (every slice an order; GST also on the SEBI fee from 2022-07-18; STT / stamp up to the rupee per side),
/// at the dearer of the dated and today's totals.
fn leg_charges(t: &TripOut, extra: Option<(Day, i64, bool, i64, i64)>) -> Result<i64, String> {
    let qty = i64::try_from(t.units_in).unwrap_or(0);
    let rt = oc::rates_on(oc::today())?;
    let buy = Trip { qty, buy_value: t.value_in, entry_orders: u32::try_from(t.buy_slices.max(1)).unwrap_or(1), ..Trip::default() };
    let mut by_day: BTreeMap<Day, Trip> = BTreeMap::new();
    for &(d, v, k) in &t.sells {
        let e = by_day.entry(d).or_insert(Trip { qty, ..Trip::default() });
        e.sell_value += v;
        e.exit_orders += u32::try_from(k.max(1)).unwrap_or(1);
    }
    if let Some((d, v, settled, sv, sn)) = extra {
        let e = by_day.entry(d).or_insert(Trip { qty, ..Trip::default() });
        e.sell_value += v;
        if settled {
            e.rem_kind = oc::REM_SETTLED;
            e.settle_value = sv;
            e.settle_notional = sn;
        } else {
            e.exit_orders += 1;
        }
    }
    let sum = |r: &oc::Rates, tr: &Trip| -> i64 { oc::charges(r, tr).iter().sum() };
    let rb = oc::rates_on(i64::from(t.buy_day))?;
    let mut dated = sum(&rb, &buy);
    let mut today = sum(&rt, &buy);
    for (d, s) in &by_day {
        let rd = oc::rates_on(i64::from(*d))?;
        dated += sum(&rd, s);
        today += sum(&rt, s);
    }
    Ok(dated.max(today))
}

/// Closes a trip: the leftover reading (the worse of settlement and bullet 1 on an expiry day), the nets.
fn close_trip(b: &Book, t: &mut TripOut, leftover_day: Option<(Day, i64, i64)>, h_e0: i64) -> Result<(), String> {
    let sold: i64 = t.sells.iter().map(|s| s.1).sum();
    let base = sold - t.value_in;
    match leftover_day {
        None => {
            let (c, cd) = est_charges(t, None)?;
            t.charges = c;
            t.net = base - c;
            t.net_dated = base - cd;
            t.net_per_leg = base - leg_charges(t, None)?;
        }
        Some((d, w0_order, end)) => {
            let un = i64::try_from(t.leftover).unwrap_or(0);
            let folder = t.folder.clone();
            let rows = b.opt_rows(&folder, d)?.unwrap_or_default();
            // D-S8-2 (check-build-idx-S7-S10, plan s4.2 (3)): W0 = E0 when the position was entered that same session, else the first
            // block of the closing order sent that session; never an E0 of an earlier day (m_idx_settle_window: that E0;
            // m_idx_w0_same_session: the closing order's decision every time, the S8 reading the checker found).
            let w0 = if cfg!(feature = "m_idx_settle_window") {
                h_e0
            } else if !cfg!(feature = "m_idx_w0_same_session") && storeio::ist_of(h_e0).0 == d {
                h_e0.min(w0_order)
            } else {
                w0_order
            };
            t.w0 = w0;
            t.bullet1_px = rows.iter().filter(|r| r.ts >= w0 && r.ts < end).map(|r| r.low).min().unwrap_or(5);
            let close = *b.close.get(&d).ok_or_else(|| format!("no official close {} {}", b.index, day::fmt(d)))?;
            t.settle_close = close;
            t.intrinsic = settle_intrinsic(t.ce, t.strike, close).map_err(|e| format!("settle {e:?}"))?;
            let sv = t.intrinsic.saturating_mul(un);
            let sn = close.saturating_mul(un);
            let bv = t.bullet1_px.saturating_mul(un);
            let (c1, d1) = est_charges(t, Some((d, bv, false, 0, 0)))?;
            let (c2, d2) = est_charges(t, Some((d, sv, true, sv, sn)))?;
            t.net_bullet1 = base + bv - c1;
            t.net_settle = base + sv - c2;
            if cfg!(feature = "m_idx_settle") {
                let lp = rows.last().map_or(5, |r| r.close);
                let lv = lp.saturating_mul(un);
                let (c3, d3) = est_charges(t, Some((d, lv, false, 0, 0)))?;
                (t.net, t.net_dated, t.charges, t.settled) = (base + lv - c3, base + lv - d3, c3, false);
                t.net_per_leg = t.net;
                return Ok(());
            }
            if t.net_settle <= t.net_bullet1 {
                (t.net, t.net_dated, t.charges, t.settled) = (t.net_settle, base + sv - d2, c2, true);
                t.net_per_leg = base + sv - leg_charges(t, Some((d, sv, true, sv, sn)))?;
            } else {
                (t.net, t.net_dated, t.charges, t.settled) = (t.net_bullet1, base + bv - d1, c1, false);
                t.net_per_leg = base + bv - leg_charges(t, Some((d, bv, false, 0, 0)))?;
            }
        }
    }
    Ok(())
}

/// S7 BUILDER wave 4 (plan v3.1 s4.3a B2: "`open_at_data_end` is kept, never refused. It is valued at the worst print of the last
/// session (the S15 pattern), flagged"; s4.4 "Data end"; s4.7 (15)): the `leftover` units of a trip still held after the LAST session
/// `d` of the data (W4 REPAIR D-W4-1: every held unit of a stream with the [`DATA_END`] sentinel; or an exit or carry on `d` whose
/// unfilled rest finds no next session) are valued at the lowest LOW of the contract's prints in
/// the window from `w0` to the F&O end of `d` (5 paisa when none, as bullet 1), charged as one more sale order at that value (the dated estimator, as an
/// S15 bullet 1 reading), never settled. The caller passes W0 = the trip's E0 when it was bought that session, else the session's first
/// block 09:15:00 ("the worst print of the last session").
///
/// W4 REPAIR r3 (check-wave4-S9-r2.md D-R2-1, the checker's exact fix): the valuation reads the prints of [W0, end), so a DATA-law hole
/// of the contract-day overlapping that window (the same span test `walk_order` applies to an order: `a < end && z > w0`) makes the trip
/// a hole trade: `t.hole` (the callers count `data_end_hole` + units). Both data-end paths (the sentinel loop and the exit or carry on
/// the last session) go through here. Mutant `m_idx_data_end_nohole`: the span test skipped (the w4r reading: never a hole).
fn value_at_data_end(b: &Book, t: &mut TripOut, d: Day, w0: i64, end: i64) -> Result<(), String> {
    let sold: i64 = t.sells.iter().map(|s| s.1).sum();
    let base = sold - t.value_in;
    let un = i64::try_from(t.leftover).unwrap_or(0);
    let rows = b.opt_rows(&t.folder, d)?.unwrap_or_default();
    // the holes are known once the contract-day's rows are read (opt_rows fills the cache on the production source; none on the store)
    t.hole = !cfg!(feature = "m_idx_data_end_nohole") && b.holes_of(&t.folder, d).iter().any(|&(a, z)| a < end && z > w0);
    t.w0 = w0;
    t.bullet1_px = rows.iter().filter(|r| r.ts >= w0 && r.ts < end).map(|r| r.low).min().unwrap_or(5);
    let bv = t.bullet1_px.saturating_mul(un);
    let (c1, d1) = est_charges(t, Some((d, bv, false, 0, 0)))?;
    t.net_bullet1 = base + bv - c1;
    (t.net, t.net_dated, t.charges, t.settled, t.data_end) = (t.net_bullet1, base + bv - d1, c1, false, true);
    t.net_per_leg = base + bv - leg_charges(t, Some((d, bv, false, 0, 0)))?;
    Ok(())
}

/// W0 of a data-end valuation on the last session `d` (plan s4.3a B2, the S15 pattern): the trip's E0 when it was bought on `d`, else
/// the session's first block 09:15:00 ("the worst print of the last session"). Mutant `m_idx_data_end_e0`: the first block even for a
/// trip bought on `d` (prints before the position existed are read).
fn data_end_w0(e0: i64, d: Day) -> i64 {
    let sod0 = at_sod(d, 9 * 3600 + 15 * 60);
    if !cfg!(feature = "m_idx_data_end_e0") && storeio::ist_of(e0).0 == d { e0.max(sod0) } else { sod0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Ev {
    Carry,
    Entry,
    Exit,
    RollClose,
    Open,
}

fn bump(f: &mut BTreeMap<&'static str, u32>, k: &'static str) {
    *f.entry(k).or_insert(0) += 1;
}

/// Adds `n` units to a counted quantity (s8b: the units a cut or the roll cancelled).
fn add(f: &mut BTreeMap<&'static str, u32>, k: &'static str, n: u64) {
    let e = f.entry(k).or_insert(0);
    *e = e.saturating_add(u32::try_from(n).unwrap_or(u32::MAX));
}

fn ts_of(t: i64) -> String {
    let (d, sod) = storeio::ist_of(t);
    format!("{} {}", day::fmt(d), storeio::hms(sod))
}

/// A buy order (an entry or a roll-open) that the stream's exit decision cut (D-S8-1): its walk stopped at the exit decision `at`, so
/// only the units known by then are held; its `rest` is cancelled there (counted at `Ev::Exit`).
#[derive(Clone, Copy, Debug)]
struct LiveBuy {
    open: bool,
    at: i64,
    rest: u64,
}

/// One roll close's outcome for its instant's group (MAIN ruling 2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ready {
    /// Its re-open may be decided from this instant: (a) the END of the second of its last close unit (t2); (b) 15:00:00 itself.
    At(i64),
    /// (a) No re-open: the close did not finish before the index close (`roll_close_incomplete` / `roll_t2_at_index_close`).
    NoOpen,
    /// Nothing was held (never in a valid run); it blocks no other close.
    Nothing,
}

/// The roll closes of ONE instant in the account (MAIN ruling 2): every close of the instant is settled first; only then is each
/// re-open decided, in stream order, at the instant the LAST of them is known.
#[derive(Clone, Debug, Default)]
struct RollGroup {
    /// Closes of the instant not processed yet.
    left: usize,
    /// (stream, day, outcome) of the closes processed.
    done: Vec<(usize, Day, Ready)>,
}

/// The slot a buy is sized to at `t` (s8b; W4 REPAIR D-W4-3): THE one formula [`SlotRule::slot`] with the stream's N (account / its
/// slot) and the net known at `t`: base / N under the headline (base = the account + min(0, that net)), the fixed slot beside.
fn slot_at(spec: &Spec, s: &Stream, led: &Ledger, t: i64) -> i64 {
    let n = spec.account.checked_div(s.slot).unwrap_or(1).max(1);
    spec.slot_rule.slot(spec.account, n, led.realised(t))
}

/// Prices one run (every stream in one account) on one protection path.
///
/// # Errors
/// A store read error, a missing row (F13), the never-past-expiry guard, the live-buy guard (s8b), a stream whose exit is decided
/// before its entry, or a refused rate day.
#[allow(clippy::too_many_lines, reason = "the positional legs in one place")]
pub fn run(books: &[Book], spec: &Spec) -> Result<RunOut, String> {
    if cfg!(feature = "m_idx_data_end_standin") && spec.streams.iter().any(|s| s.exit == DATA_END) {
        // MUTANT m_idx_data_end_standin (the final4 reading, check-wave4-S9 D-W4-1): a stream with no exit before the data end is forced
        // out at a STAND-IN exit, the last session's index close (or 1 s after its entry when later), and sells on the prints after it
        let mut sp = spec.clone();
        for st in &mut sp.streams {
            if st.exit == DATA_END {
                let bk = books.get(st.book).ok_or("book")?;
                let last = *bk.sess.days.last().ok_or("no session in the data")?;
                st.exit = bk.sess.index_close(last)?.max(st.entry + US_S);
            }
        }
        return run(books, &sp);
    }
    let ns = spec.streams.len();
    let mut out: Vec<StreamOut> = spec.streams.iter().map(|s| StreamOut { label: s.label.clone(), ..StreamOut::default() }).collect();
    let mut held: Vec<Option<Held>> = vec![None; ns];
    // s8b: the buy order each stream's exit decision cut (D-S8-1), and the roll closes of each instant (MAIN ruling 2)
    let mut live: Vec<Option<LiveBuy>> = vec![None; ns];
    let mut groups: BTreeMap<i64, RollGroup> = BTreeMap::new();
    // the instant of each stream's queued Ev::Open (D-S8-1: a flip cancels it by key, O(log n), never a queue scan)
    let mut pend: Vec<Option<i64>> = vec![None; ns];
    let mut led = Ledger::default();
    let mut q: BTreeMap<(i64, usize, Ev), Day> = BTreeMap::new();
    for (i, s) in spec.streams.iter().enumerate() {
        if s.exit < s.entry {
            return Err(format!("stream {}: exit decided before its entry (spec error)", s.label));
        }
        q.insert((s.entry, i, Ev::Entry), storeio::ist_of(s.entry).0);
        // W4 REPAIR (D-W4-1): no Ev::Exit for the data-end sentinel (the position is valued after every event, below)
        if s.exit != DATA_END {
            q.insert((s.exit, i, Ev::Exit), storeio::ist_of(s.exit).0);
        }
    }
    let roll_ist = 15 * 3600;
    while let Some(((t, i, ev), d)) = q.pop_first() {
        let s = spec.streams.get(i).ok_or("stream")?;
        let b = books.get(s.book).ok_or("book")?;
        let o = out.get_mut(i).ok_or("out")?;
        match ev {
            Ev::Entry => {
                let row = *b.cal.get(&d).ok_or_else(|| format!("no DP-IDXCAL row {}", day::fmt(d)))?;
                let next = row.d_exp && t >= at_sod(d, roll_ist);
                let (alias, file) = if next {
                    let ne = row.next_eff.ok_or("no next expiry")?;
                    (b.alias_in(row.file_read, ne).ok_or("next alias not listed")?, row.file_read)
                } else {
                    (row.known_listed, row.file_read)
                };
                let sr = b.series_of(alias, file).ok_or_else(|| format!("no series for alias {} file {}", day::fmt(alias), day::fmt(file)))?.clone();
                let (p, info) = match pick(b, d, file, alias, sr.final_date(), s.ce, t, spec.target, false) {
                    Ok(x) => x,
                    Err(e) => {
                        bump(&mut o.flags, "entry_refused_no_itm_fit");
                        o.legs.push(LegOut { kind: "entry_refused", day: d, decided: t, folder: e, ..LegOut::default() });
                        continue;
                    }
                };
                let free = led.free(t, spec.account);
                let slot = slot_at(spec, s, &led, t);
                let (qty, why) = size_units_closest(slot, Some(p.reference), p.lot, Some(free.max(0))).map_err(|e| format!("C11 {e:?}"))?;
                let uncapped = size_units_closest(slot, Some(p.reference), p.lot, None).map_or(0, |x| x.0);
                if matches!(why, SizeWhy::CashShortZero { .. }) || why.refused() || qty == 0 {
                    bump(&mut o.flags, "entry_cash_short_zero");
                    continue;
                }
                if qty < uncapped {
                    bump(&mut o.flags, "cash_short");
                }
                let h0 = Held { folder: p.folder.clone(), strike: p.strike, series: sr.id, alias_open: alias, expiry: if cfg!(feature = "m_idx_folder") { sr.final_date() } else { sr.true_last }, lot: p.lot, units: 0, trip: 0, e0: 0 };
                guard(&h0, d)?;
                let rd = roll_day(b, &h0, d);
                let roll_at = at_sod(d, roll_ist);
                let at_roll = rd == Some(d) && t < roll_at;
                let stop0 = if at_roll { roll_at } else { b.sess.fno_end(d)? };
                // D-S8-1: the buy walks to min(its stop, the stream's next exit decision) (m_idx_exit_cut: to its stop, the S8 walk)
                let cut = !cfg!(feature = "m_idx_exit_cut") && s.exit <= stop0;
                let stop = if cut { s.exit } else { stop0 };
                let rows = b.opt_rows(&p.folder, d)?.unwrap_or_default();
                let limit = if spec.kept { last_close_before(&rows, t).map(|c| limits::protection(c, Side::Buy, 1_000, 5)) } else { None };
                let (lf, fills) = walk_order(b, &p.folder, d, Side::Buy, t, stop, p.lot, limit, qty, Some(free.max(0)), spec.bp)?;
                let mo = b.freeze.max_one_order(&b.index, d, i64::try_from(p.lot).unwrap_or(0)).unwrap_or(0);
                if i64::try_from(p.qty).unwrap_or(i64::MAX) > mo {
                    bump(&mut o.flags, "one_order_gate_fail");
                }
                let sl = freeze_slices(qty, u64::try_from(mo).unwrap_or(0));
                if sl > 1 {
                    bump(&mut o.flags, "entry_slices_beside");
                }
                led.res.push(Res { from: t, until: if lf.units == qty { lf.known_at } else { stop }, want: qty, refpx: p.reference, fills: fills.iter().map(|f| (f.4, f.1)).collect() });
                o.legs.push(LegOut { kind: "entry", day: d, decided: t, stop, folder: p.folder.clone(), strike: p.strike, series: sr.id, expiry: h0.expiry, lot: p.lot, freeze: b.freeze.units(&b.index, d).unwrap_or(0), max_order: mo, want: qty, units: lf.units, value: lf.value, unpaid: lf.unpaid, slices: sl, limit, budget: Some(free.max(0)), free, fills: fills.clone(), e0: lf.e0, known_at: lf.known_at, pick: Some(info), hole: lf.hole_span });
                if cut && lf.rest_at_stop > 0 {
                    // still live at the exit decision: cut there, the rest cancelled and counted at Ev::Exit
                    if let Some(x) = live.get_mut(i) {
                        *x = Some(LiveBuy { open: false, at: stop, rest: lf.rest_at_stop });
                    }
                } else if at_roll && lf.rest_at_stop > 0 {
                    // s4.2 (5) bullet 3: an entry still filling at 15:00:00 on its expiry day: the rest is cancelled at the roll; the
                    // filled units roll
                    bump(&mut o.flags, "entry_rest_cancelled_at_roll");
                    add(&mut o.flags, "entry_rest_cancelled_at_roll_units", lf.rest_at_stop);
                }
                if lf.units == 0 {
                    bump(&mut o.flags, "entry_zero_fill");
                    continue;
                }
                let trip = TripOut { folder: p.folder.clone(), strike: p.strike, ce: s.ce, expiry: h0.expiry, units_in: lf.units, value_in: lf.value, buy_day: d, buy_slices: sl, ..TripOut::default() };
                o.trips.push(trip);
                let ti = o.trips.len() - 1;
                led.held.insert((i, ti), (fills.iter().map(|f| (f.4, i64::try_from(f.1).unwrap_or(0).saturating_mul(f.3))).collect(), None));
                let h = Held { units: lf.units, trip: ti, e0: lf.e0, ..h0 };
                if let Some(rd) = roll_day(b, &h, d) {
                    let at = at_sod(rd, roll_ist);
                    if at < s.exit {
                        q.insert((at, i, Ev::RollClose), rd);
                    }
                }
                if let Some(x) = held.get_mut(i) {
                    *x = Some(h);
                }
            }
            Ev::RollClose => {
                // MAIN ruling 2: the roll closes of this instant in the account are ONE group (sized when its first close comes up)
                groups.entry(t).or_insert_with(|| {
                    let others = q.range((t, 0, Ev::Carry)..=(t, usize::MAX, Ev::Open)).filter(|(k, _)| k.2 == Ev::RollClose).count();
                    RollGroup { left: others + 1, done: Vec::new() }
                });
                let ready = 'close: {
                    let Some(h) = held.get(i).cloned().flatten() else { break 'close Ready::Nothing };
                    guard(&h, d)?;
                    let rows = b.opt_rows(&h.folder, d)?.unwrap_or_default();
                    let limit = if spec.kept { last_close_before(&rows, t).map(|c| limits::protection(c, Side::Sell, 1_000, 5)) } else { None };
                    let end = b.sess.fno_end(d)?;
                    let (lf, fills) = walk_order(b, &h.folder, d, Side::Sell, t, end, h.lot, limit, h.units, None, spec.bp)?;
                    let mo = b.freeze.max_one_order(&b.index, d, i64::try_from(h.lot).unwrap_or(0)).unwrap_or(0);
                    let k = if cfg!(feature = "m_idx_close_one_order") { 1 } else { freeze_slices(h.units, u64::try_from(mo).unwrap_or(0)) };
                    if k > 1 {
                        bump(&mut o.flags, "close_split");
                    }
                    let rest = h.units - lf.units;
                    let t2 = lf.known_at;
                    o.legs.push(LegOut { kind: "roll_close", day: d, decided: t, stop: end, folder: h.folder.clone(), strike: h.strike, series: h.series, expiry: h.expiry, lot: h.lot, freeze: b.freeze.units(&b.index, d).unwrap_or(0), max_order: mo, want: h.units, units: lf.units, value: lf.value, unpaid: 0, slices: k, limit, budget: None, free: led.free(t, spec.account), fills, e0: lf.e0, known_at: t2, pick: None, hole: lf.hole_span });
                    let trip = o.trips.get_mut(h.trip).ok_or("trip")?;
                    if lf.units > 0 {
                        trip.sells.push((d, lf.value, k));
                    }
                    trip.leftover = rest;
                    close_trip(b, trip, (rest > 0).then_some((d, t, end)), h.e0)?;
                    trip.closed_at = if rest > 0 { end } else { t2 };
                    if rest > 0 {
                        bump(&mut o.flags, "leftover_at_expiry");
                        if trip.settled {
                            bump(&mut o.flags, "settled");
                        }
                    }
                    led.closed.push((trip.closed_at, trip.net));
                    if let Some(e) = led.held.get_mut(&(i, h.trip)) {
                        e.1 = Some(trip.closed_at);
                    }
                    if let Some(x) = held.get_mut(i) {
                        *x = None;
                    }
                    // the instant from which this stream's open leg may be decided
                    let idx_close = b.sess.index_close(d)?;
                    match spec.open_rule {
                        OpenRule::Sequential => {
                            if rest > 0 {
                                bump(&mut o.flags, "roll_close_incomplete");
                                break 'close Ready::NoOpen;
                            }
                            if t2 >= idx_close {
                                bump(&mut o.flags, "roll_t2_at_index_close");
                                break 'close Ready::NoOpen;
                            }
                            let t_open = if cfg!(feature = "m_idx_roll_funds") { lf.e1 } else { t2 };
                            if cfg!(feature = "m_idx_roll_funds") && t_open < t2 {
                                // MUTANT: the close is counted as released before its last unit filled
                                if let Some(c) = led.closed.last_mut() {
                                    c.0 = t_open;
                                }
                                if let Some(e) = led.held.get_mut(&(i, h.trip)) {
                                    e.1 = Some(t_open);
                                }
                            }
                            Ready::At(t_open)
                        }
                        OpenRule::SameDecision => Ready::At(t),
                    }
                };
                let g = groups.get_mut(&t).ok_or("roll group")?;
                g.left = g.left.saturating_sub(1);
                g.done.push((i, d, ready));
                if cfg!(feature = "m_idx_roll_group") {
                    // MUTANT: each re-open decided at its own close's instant, before the other closes of the instant are settled
                    if let Ready::At(x) = ready {
                        q.insert((x, i, Ev::Open), d);
                        if let Some(p) = pend.get_mut(i) {
                            *p = Some(x);
                        }
                    }
                    if g.left == 0 {
                        groups.remove(&t);
                    }
                    continue;
                }
                if g.left == 0 {
                    let g = groups.remove(&t).unwrap_or_default();
                    if g.done.iter().any(|m| m.2 == Ready::NoOpen) {
                        // (a): a close of this instant did not finish before the index close, so no re-open of it can be decided
                        for &(j, _, r) in &g.done {
                            if matches!(r, Ready::At(_))
                                && let Some(oj) = out.get_mut(j)
                            {
                                bump(&mut oj.flags, "roll_wait_close_incomplete");
                            }
                        }
                    } else if let Some(t_open) = g.done.iter().filter_map(|m| if let Ready::At(x) = m.2 { Some(x) } else { None }).max() {
                        // every close of the instant is settled first: the open legs are their own events at the LAST close's instant
                        // ((a) the latest t2; (b) 15:00:00), decided in stream order (ties: stream order)
                        for &(j, dj, r) in &g.done {
                            if matches!(r, Ready::At(_)) {
                                q.insert((t_open, j, Ev::Open), dj);
                                if let Some(p) = pend.get_mut(j) {
                                    *p = Some(t_open);
                                }
                            }
                        }
                    }
                }
            }
            Ev::Open => {
                if let Some(p) = pend.get_mut(i) {
                    *p = None;
                }
                let t_open = t;
                let end = b.sess.fno_end(d)?;
                let row = *b.cal.get(&d).ok_or("cal")?;
                let ne = row.next_eff.ok_or("no next expiry at the roll")?;
                let alias = b.alias_in(row.file_read, ne).ok_or("next alias not listed in the file read")?;
                let sr = b.series_of(alias, row.file_read).ok_or("no series of the next expiry")?.clone();
                let (p, info) = match pick(b, d, row.file_read, alias, sr.final_date(), s.ce, t_open, spec.target, true) {
                    Ok(x) => x,
                    Err(_) => {
                        bump(&mut o.flags, "roll_no_itm");
                        continue;
                    }
                };
                let free = led.free(t_open, spec.account);
                let slot = slot_at(spec, s, &led, t_open);
                let (qty, why) = size_units_closest(slot, Some(p.reference), p.lot, Some(free.max(0))).map_err(|e| format!("C11 {e:?}"))?;
                let uncapped = size_units_closest(slot, Some(p.reference), p.lot, None).map_or(0, |x| x.0);
                if matches!(why, SizeWhy::CashShortZero { .. }) || why.refused() || qty == 0 {
                    bump(&mut o.flags, "roll_refused_cash");
                    o.legs.push(LegOut { kind: "roll_open_refused_cash", day: d, decided: t_open, folder: p.folder.clone(), strike: p.strike, free, pick: Some(info), ..LegOut::default() });
                    continue;
                }
                if qty < uncapped {
                    bump(&mut o.flags, "cash_short");
                }
                let mo2 = b.freeze.max_one_order(&b.index, d, i64::try_from(p.lot).unwrap_or(0)).unwrap_or(0);
                if i64::try_from(p.qty).unwrap_or(i64::MAX) > mo2 {
                    bump(&mut o.flags, "one_order_gate_fail");
                }
                let sl = freeze_slices(qty, u64::try_from(mo2).unwrap_or(0));
                if sl > 1 {
                    bump(&mut o.flags, "open_slices_beside");
                }
                let orows = b.opt_rows(&p.folder, d)?.unwrap_or_default();
                let olimit = if spec.kept { last_close_before(&orows, t_open).map(|c| limits::protection(c, Side::Buy, 1_000, 5)) } else { None };
                // D-S8-1: the roll-open walks to min(the F&O end, the stream's next exit decision) (m_idx_exit_cut: to the F&O end)
                let cut = !cfg!(feature = "m_idx_exit_cut") && s.exit <= end;
                let ostop = if cut { s.exit } else { end };
                let (olf, ofills) = walk_order(b, &p.folder, d, Side::Buy, t_open, ostop, p.lot, olimit, qty, Some(free.max(0)), spec.bp)?;
                led.res.push(Res { from: t_open, until: if olf.units == qty { olf.known_at } else { ostop }, want: qty, refpx: p.reference, fills: ofills.iter().map(|f| (f.4, f.1)).collect() });
                let nh = Held { folder: p.folder.clone(), strike: p.strike, series: sr.id, alias_open: alias, expiry: if cfg!(feature = "m_idx_folder") { sr.final_date() } else { sr.true_last }, lot: p.lot, units: olf.units, trip: 0, e0: olf.e0 };
                o.legs.push(LegOut { kind: "roll_open", day: d, decided: t_open, stop: ostop, folder: p.folder.clone(), strike: p.strike, series: sr.id, expiry: nh.expiry, lot: p.lot, freeze: b.freeze.units(&b.index, d).unwrap_or(0), max_order: mo2, want: qty, units: olf.units, value: olf.value, unpaid: olf.unpaid, slices: sl, limit: olimit, budget: Some(free.max(0)), free, fills: ofills.clone(), e0: olf.e0, known_at: olf.known_at, pick: Some(info), hole: olf.hole_span });
                let cut_rest = if cut { olf.rest_at_stop } else { 0 };
                if cut_rest > 0 {
                    // plan s4.2 (4) bullet 3: still filling at the flip: cut there, the rest cancelled and counted at Ev::Exit
                    if let Some(x) = live.get_mut(i) {
                        *x = Some(LiveBuy { open: true, at: ostop, rest: cut_rest });
                    }
                } else if olf.units < qty {
                    // "roll short: n of m lots" (the rest unfilled at the session end, or REQ 70a)
                    bump(&mut o.flags, "roll_short");
                }
                if olf.units == 0 {
                    bump(&mut o.flags, "roll_open_zero");
                    continue;
                }
                o.trips.push(TripOut { folder: p.folder.clone(), strike: p.strike, ce: s.ce, expiry: nh.expiry, units_in: olf.units, value_in: olf.value, buy_day: d, buy_slices: sl, ..TripOut::default() });
                let ti = o.trips.len() - 1;
                led.held.insert((i, ti), (ofills.iter().map(|f| (f.4, i64::try_from(f.1).unwrap_or(0).saturating_mul(f.3))).collect(), None));
                let nh = Held { trip: ti, ..nh };
                bump(&mut o.flags, "rolls");
                if let Some(nd) = b.sess.next(d).and_then(|n| roll_day(b, &nh, n)) {
                    let at = at_sod(nd, roll_ist);
                    if at < s.exit {
                        q.insert((at, i, Ev::RollClose), nd);
                    }
                }
                if let Some(x) = held.get_mut(i) {
                    *x = Some(nh);
                }
            }
            Ev::Exit | Ev::Carry => {
                if ev == Ev::Exit {
                    // D-S8-1 (plan s4.2 (4) bullet 2): a flip while the roll's open leg is still pending (the position already closing)
                    // sends no open leg: cancelled and counted (m_idx_flip_open: the open survives)
                    if !cfg!(feature = "m_idx_flip_open")
                        && let Some(to) = pend.get_mut(i).and_then(Option::take)
                        && q.remove(&(to, i, Ev::Open)).is_some()
                    {
                        bump(&mut o.flags, "roll_cancelled_by_flip");
                    }
                    // D-S8-1: a buy order still live at the exit decision is cut there (its walk stopped at it, so only the units known
                    // by then are held); its cancelled rest is counted
                    if let Some(lb) = live.get_mut(i).and_then(Option::take) {
                        if lb.at != t {
                            return Err(format!("LIVE-BUY guard: a buy order cut at {} but the exit decided at {} (build error, never a refusal)", ts_of(lb.at), ts_of(t)));
                        }
                        if lb.open {
                            bump(&mut o.flags, "open_cut_by_flip");
                            add(&mut o.flags, "open_cut_units", lb.rest);
                        } else {
                            bump(&mut o.flags, "entry_cut_by_exit");
                            add(&mut o.flags, "entry_cut_units", lb.rest);
                        }
                    }
                }
                let Some(h) = held.get(i).cloned().flatten() else { continue };
                guard(&h, d)?;
                let rows = b.opt_rows(&h.folder, d)?.unwrap_or_default();
                let limit = if spec.kept { last_close_before(&rows, t).map(|c| limits::protection(c, Side::Sell, 1_000, 5)) } else { None };
                let end = b.sess.fno_end(d)?;
                let (lf, fills) = walk_order(b, &h.folder, d, Side::Sell, t, end, h.lot, limit, h.units, None, spec.bp)?;
                let mo = b.freeze.max_one_order(&b.index, d, i64::try_from(h.lot).unwrap_or(0)).unwrap_or(0);
                let k = if cfg!(feature = "m_idx_close_one_order") { 1 } else { freeze_slices(h.units, u64::try_from(mo).unwrap_or(0)) };
                if k > 1 {
                    bump(&mut o.flags, "close_split");
                }
                let rest = h.units - lf.units;
                o.legs.push(LegOut { kind: if ev == Ev::Exit { "exit" } else { "carry" }, day: d, decided: t, stop: end, folder: h.folder.clone(), strike: h.strike, series: h.series, expiry: h.expiry, lot: h.lot, freeze: b.freeze.units(&b.index, d).unwrap_or(0), max_order: mo, want: h.units, units: lf.units, value: lf.value, unpaid: 0, slices: k, limit, budget: None, free: led.free(t, spec.account), fills, e0: lf.e0, known_at: lf.known_at, pick: None, hole: lf.hole_span });
                let trip = o.trips.get_mut(h.trip).ok_or("trip")?;
                if lf.units > 0 {
                    trip.sells.push((d, lf.value, k));
                }
                let expiry_day = roll_day(b, &h, d) == Some(d);
                if rest > 0 && !expiry_day {
                    // W4 REPAIR (D-W4-2; plan s4.4 / Q39, S1 MUHURAT-LIST "positions are held through"): the rest is carried to the
                    // next session that is NOT a Muhurat session (m_idx_carry_muhurat: the next session, Muhurat or not, as final4)
                    let next_sess = if cfg!(feature = "m_idx_carry_muhurat") { b.sess.next(d) } else { b.sess.next_carry(d) };
                    if cfg!(feature = "m_idx_data_end_error") && next_sess.is_none() {
                        // MUTANT m_idx_data_end_error: the S8b reading, a carry past the last session stops the run
                        return Err("no next session for the carry".to_string());
                    }
                    if let Some(nd) = next_sess {
                        bump(&mut o.flags, "carry");
                        q.insert((at_sod(nd, 9 * 3600 + 15 * 60), i, Ev::Carry), nd);
                        if let Some(x) = held.get_mut(i) {
                            *x = Some(Held { units: rest, ..h });
                        }
                        continue;
                    }
                    // S7 BUILDER wave 4 (plan s4.3a B2 / s4.4 / s4.7 (15)): no session follows d, the last session of the data, so the
                    // unsold rest is `open_at_data_end`: kept (never refused, never an error), valued at the worst print of that last
                    // session with W0 = E0 when the trip was bought that session, else the session's first block (mutants
                    // m_idx_data_end_kind: W0 = this closing order's decision; m_idx_data_end_e0: the session's first block always)
                    bump(&mut o.flags, "open_at_data_end");
                    add(&mut o.flags, "open_at_data_end_units", rest);
                    let w0 = if cfg!(feature = "m_idx_data_end_kind") { t } else { data_end_w0(h.e0, d) };
                    trip.leftover = rest;
                    value_at_data_end(b, trip, d, w0, end)?;
                    if trip.hole {
                        // W4 REPAIR r3 (D-R2-1): the valuation read a DATA-law hole: a hole trade (AFR-2), counted
                        bump(&mut o.flags, "data_end_hole");
                        add(&mut o.flags, "data_end_hole_units", rest);
                    }
                    trip.closed_at = end;
                    led.closed.push((trip.closed_at, trip.net));
                    if let Some(e) = led.held.get_mut(&(i, h.trip)) {
                        e.1 = Some(trip.closed_at);
                    }
                    if let Some(x) = held.get_mut(i) {
                        *x = None;
                    }
                    continue;
                }
                trip.leftover = rest;
                close_trip(b, trip, (rest > 0).then_some((d, t, end)), h.e0)?;
                trip.closed_at = if rest > 0 { end } else { lf.known_at };
                if rest > 0 {
                    bump(&mut o.flags, "leftover_at_expiry");
                    if trip.settled {
                        bump(&mut o.flags, "settled");
                    }
                }
                led.closed.push((trip.closed_at, trip.net));
                if let Some(e) = led.held.get_mut(&(i, h.trip)) {
                    e.1 = Some(trip.closed_at);
                }
                if let Some(x) = held.get_mut(i) {
                    *x = None;
                }
            }
        }
    }
    for (si, slot_held) in held.iter().enumerate() {
        let Some(hp) = slot_held else { continue };
        let st = spec.streams.get(si).ok_or("stream")?;
        let so = out.get_mut(si).ok_or("out")?;
        if st.exit != DATA_END {
            bump(&mut so.flags, "open_after_run");
            continue;
        }
        // W4 REPAIR (D-W4-1; plan s4.3a B2 "open_at_data_end is kept, never refused. It is valued at the worst print of the last session
        // (the S15 pattern), flagged"): no exit before the data end and every event done: EVERY held unit is open_at_data_end, valued at
        // the lowest LOW of the LAST session in [W0, its F&O end) (5 paisa when none; W0 = E0 when bought that session, else 09:15:00),
        // charged as one sale order (the dated estimator), known at that F&O end; never open_after_run, never an error
        let bk = books.get(st.book).ok_or("book")?;
        let last = *bk.sess.days.last().ok_or("no session in the data (the data end needs one)")?;
        guard(hp, last)?;
        let end = bk.sess.fno_end(last)?;
        let trip = so.trips.get_mut(hp.trip).ok_or("trip")?;
        trip.leftover = hp.units;
        value_at_data_end(bk, trip, last, data_end_w0(hp.e0, last), end)?;
        trip.closed_at = end;
        bump(&mut so.flags, "open_at_data_end");
        add(&mut so.flags, "open_at_data_end_units", hp.units);
        if trip.hole {
            // W4 REPAIR r3 (D-R2-1): the valuation read a DATA-law hole: a hole trade (AFR-2), counted
            bump(&mut so.flags, "data_end_hole");
            add(&mut so.flags, "data_end_hole_units", hp.units);
        }
        led.closed.push((trip.closed_at, trip.net));
        if let Some(rel) = led.held.get_mut(&(si, hp.trip)) {
            rel.1 = Some(trip.closed_at);
        }
    }
    let mut total = 0i64;
    for o in &mut out {
        o.net = o.trips.iter().map(|t| t.net).sum();
        o.net_per_leg = o.trips.iter().map(|t| t.net_per_leg).sum();
        total += o.net;
    }
    Ok(RunOut { streams: out, net: total })
}

#[cfg(test)]
#[path = "pos_tests.rs"]
mod tests;
