//! S7 BUILDER (wave 4, main 5b507203, 15 Nov 2026): the INDEX OPTIONS modes of host v3 (SNIPER-IDXOPT-PLAN-v3.1 s3-s5, s8 S7), NIFTY
//! and BANKNIFTY, INTRADAY and POSITIONAL, as thin wiring of the checked parts:
//!
//! * the signals: the S4 index feature rows (`feat_io` r3: `instr` rows, the Zerodha NSE/INDEX read with the Muhurat candles excluded,
//!   Q39; the session table SESSIONS-SPECIAL-IDX-v1; the index official rows), folded by the SAME `cands::build` as the cash path;
//! * the context dials (`--context-dials on`): `feat_io::idxdials` (X-PEER in the X-NIFTY dial, X-VIX, D-EXP from DP-IDXCAL, D-MEND,
//!   G-52H / G-52L from the index official rows, G-DRNG), mapped field for field onto the host's seven readings;
//! * the money: the S8b pricer `opt_pos` on the S5 pack's production option seconds (DP-NEWPX-IDXOPT under the DATA law: `Book::with_newpx`;
//!   decisions read the store's 1-s bars as DP-IDXITM does). INTRADAY: `opt_pos::intraday` (the A2 / s23a pick once at the decision, C11 per
//!   N, `price_intraday_holes` per level with XC per protection path, D2). POSITIONAL: `opt_pos::pos::run` per candidate (entry, the 15:00
//!   roll with the SEQUENTIAL open, close_split, carry, settlement with the B6 window) on lane B's groups (`crate::posgroups`);
//! * the outcome rows: the cash encoding with every N of the 15-value grid (`price::record_width(15, 5)` = 472 B intraday; the plan's
//!   "442 B" is 352 + 3 columns x 30 B and predates the every-N row, named) and lane B's OUT-POS layout (`record_width_pos(15, 5)` = 792 B);
//! * everything downstream is the cash path's own code (`search::context`): the sweep at every N, the merged lists, the re-walk, common_wf's
//!   picks / verdicts / words, the twin, the HARD gate at 1x / 10x / 100x (Row.k u32), the DB rows with the audit chain.
//!
//! Money: options are BOUGHT only, so the money in is the premium (lev 1 / 1; no margin: MAIN-RULINGS-S7-S10 item 4's pluggable margin
//! applies to SELLING legs, out of this scope). The 15:10 square-off is the strategy's own order (D2, law row `sq_charge own_order`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use common_fill::{MAXL, US_S};
use idx_cal::day::Day;
use opt_pos::chain::Folders;
use opt_pos::freeze::FreezeTable;
use opt_pos::ladder::Ladder;
use opt_pos::newpx::Fails;
use opt_pos::pos::{self as opos, Book, CalRow, NewpxSrc, SeriesRow};
use opt_pos::sess::Sessions;
use wf_sweep::acc::Outcome;

use crate::cands::{self, Family};
use crate::instr::Instrument;
use crate::price::{self, LevelOut, Priced};
use crate::rows::ExitSet;
use crate::search::{RowOut, Table, compact};
use crate::size::Sized;

/// The read-only inputs of one index's option legs (DP-IDXCAL, IDXCAL-SERIES, the listed ladder, the store's option folders, the dated
/// freeze table, the official close, the session facts, the store root, the S5 pack and its DATA-law sidecar), each file sha-printed.
pub struct IdxEnv {
    /// `NIFTY` | `BANKNIFTY`.
    pub index: String,
    cal: BTreeMap<Day, CalRow>,
    series: Vec<SeriesRow>,
    ladder: Ladder,
    folders: Folders,
    freeze: FreezeTable,
    close: BTreeMap<Day, i64>,
    sess: Sessions,
    store: PathBuf,
    pack: PathBuf,
    fails: Fails,
    /// The A2 fit amount (Rs 5,00,000), paisa (row `contract_target_paisa`).
    pub target: i64,
    /// (file, sha256) of every input read.
    pub shas: Vec<(String, String)>,
    /// The pack's month folders read and the DATA-law sidecar rows.
    pub pack_note: String,
}

fn row<'a>(instr: &'a Instrument, k: &str) -> Result<&'a str, String> {
    instr.rows.get(k).map(String::as_str).ok_or_else(|| format!("IDXOPT row {k} missing (plan s3.4)"))
}

impl IdxEnv {
    /// Reads every input named by the instrument rows (fail closed).
    ///
    /// # Errors
    /// A missing row or file, a malformed table.
    pub fn load(instr: &Instrument) -> Result<Self, String> {
        let ix = instr.underlying().to_string();
        let mut shas = Vec::new();
        let mut rd = |k: &str| -> Result<String, String> {
            let p = row(instr, k)?;
            let (t, s) = crate::rows::read_with_sha(p)?;
            shas.push((format!("{k} {p}"), s));
            Ok(t)
        };
        let cal = opos::parse_cal(&rd("expiry_cal")?)?;
        let series = opos::parse_series(&rd("idx_series_file")?)?;
        let close = opos::parse_close(&rd("idx_close_file")?)?;
        // W4 REPAIR (check-wave4-S9 D-W4-2): S1's MUHURAT-LIST rows are data of the session facts (a carry is held through them)
        let sess = Sessions::parse(&rd("fno_end_file")?, &rd("halt_file")?)?.with_muhurat(&rd("muhurat_file")?)?;
        let ladder = Ladder::parse(&rd("lot_source")?)?;
        let freeze = FreezeTable::parse(&rd("freeze_file")?)?;
        let store = PathBuf::from(row(instr, "option_store")?);
        let pack = PathBuf::from(row(instr, "option_pack")?);
        let target: i64 = row(instr, "contract_target_paisa")?.parse().map_err(|_| "contract_target_paisa is not a whole paisa count".to_string())?;
        let folders = Folders::list(&store, &ix)?;
        let dir = pack.join(format!("newpxF-{ix}"));
        let mut months: Vec<PathBuf> = std::fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?.filter_map(Result::ok).map(|e| e.path()).collect();
        months.sort();
        let mut fails = Fails::default();
        let mut nf = 0usize;
        for m in &months {
            let f = m.join(format!("DP-NEWPX-FAILS-{ix}.tsv"));
            if f.exists() {
                let (t, s) = crate::rows::read_with_sha(&f.to_string_lossy())?;
                fails.add(&t)?;
                nf += 1;
                shas.push((format!("DP-NEWPX-FAILS {}", f.display()), s));
            }
        }
        if let Ok((_, s)) = crate::rows::read_with_sha(&pack.join("SHA256SUMS-PACK-FINAL").to_string_lossy()) {
            shas.push((format!("option_pack SHA256SUMS-PACK-FINAL {}", pack.display()), s));
        }
        let pack_note = format!("{} month folders of {} ({} DP-NEWPX-FAILS sidecars)", months.len(), dir.display(), nf);
        Ok(Self { index: ix, cal, series, ladder, folders, freeze, close, sess, store, pack, fails, target, shas, pack_note })
    }

    /// The Muhurat sessions of the session facts (S1 MUHURAT-LIST; W4 REPAIR D-W4-2).
    pub(crate) fn muhurat_days(&self) -> Vec<Day> {
        self.sess.muhurat_days()
    }

    /// The dated F&O end of session `d`, seconds of the IST day (S1 FNO-END-ROWS); None when the row is missing (F13).
    pub(crate) fn book_sess_end(&self, d: i32) -> Option<i64> {
        self.sess.fno_end(d).ok().map(|e| (e - price::at_sod(d, 0)) / US_S)
    }

    /// A positional entry at `t` that `opt_pos::pos::run` cannot even look up (a session without a DP-IDXCAL row, e.g. 2018-09-03
    /// before the calendar's first session, or no listed series for the expiry it would buy): a DECIDE refusal by name (known at T),
    /// never a pricing error.
    pub(crate) fn entry_refusal(&self, t: i64) -> Option<&'static str> {
        let (d, sod) = idx_cal::storeio::ist_of(t);
        let Some(row) = self.cal.get(&d) else { return Some("no_cal_row") };
        let alias = if row.d_exp && sod >= 15 * 3600 {
            let Some(ne) = row.next_eff else { return Some("no_next_expiry") };
            // Book::alias_in's rule on the ladder: the effective date itself when listed, else within 3 days after
            match (0..=3).map(|k| ne + k).find(|&x| self.ladder.get(&self.index, row.file_read, x).is_some()) {
                Some(a) => a,
                None => return Some("no_next_alias"),
            }
        } else {
            row.known_listed
        };
        let listed = self.series.iter().any(|s| s.aliases.iter().any(|&(a, f0, f1)| a == alias && f0 <= row.file_read && row.file_read <= f1));
        if listed { None } else { Some("no_series") }
    }

    /// A fresh book on the PRODUCTION source (one per worker and chunk: its caches stay bounded, check-build-idx-S7-S10 F-S8-a).
    #[must_use]
    pub fn book(&self) -> Book {
        Book::new(&self.index, self.cal.clone(), self.series.clone(), self.ladder.clone(), self.folders.clone(), self.freeze.clone(), self.close.clone(), self.sess.clone(), Some(self.store.clone()))
            .with_newpx(NewpxSrc::new(self.pack.join(format!("newpxF-{}", self.index)), self.fails.clone()))
    }
}

/// The contract facts of one priced intraday row (the IDX-ROWS table: plan s5.1 "the contract on every trade row"; the F8 gate).
#[derive(Clone, Debug, Default)]
pub(crate) struct RowFacts {
    pub(crate) cand: usize,
    pub(crate) folder: String,
    pub(crate) expiry: Day,
    pub(crate) lot: u64,
    pub(crate) freeze: i64,
    pub(crate) max_order: i64,
    pub(crate) qty_a2: u64,
    pub(crate) depth: u32,
    pub(crate) skipped_no_trade: u32,
    pub(crate) skipped_big: u32,
    pub(crate) reference: i64,
    pub(crate) ref_age_s: i64,
    pub(crate) spot: i64,
    pub(crate) expiring_today: bool,
    pub(crate) qty: Vec<u64>,
    pub(crate) flags_or: u16,
    pub(crate) calls: u32,
    /// The XC flip anchor (the END of the first index candle closing across Supertrend 10 x 3 before the square-off; 0 = none) and F17.
    pub(crate) flip: i64,
    pub(crate) immediate: bool,
    /// The pricing wall of this decision (the pick + every N and level), microseconds (p50 / p99 printed: O(1) measured).
    pub(crate) price_us: u64,
}

/// The cash `Priced` of an intraday index decision (the same encoding, cells and trade columns as the cash rows).
fn as_priced(p: &opt_pos::intraday::IPriced) -> Priced {
    let mut out = Priced { entry_first: p.entry_first, refpx: p.pick.info.reference, ..Priced::default() };
    for (ni, lvs) in p.lv.iter().enumerate() {
        let mut arr = [LevelOut::default(); MAXL];
        for (l, x) in lvs.iter().enumerate().take(MAXL) {
            if let Some(a) = arr.get_mut(l) {
                *a = LevelOut {
                    net: x.net,
                    rd_bp: x.rd_bp,
                    exit_anchor: x.exit_anchor,
                    exit_known: x.exit_known,
                    units: x.units,
                    unclosed: if x.flags & opt_pos::intraday::flag::UNCLOSABLE != 0 { x.leftover } else { 0 },
                    flags: x.flags,
                    entry_value: x.entry_value,
                    exit_value: x.exit_value,
                };
            }
            if x.flags & opt_pos::intraday::flag::REWALKED != 0 {
                out.rewalks += 1;
            }
        }
        out.lv.push(arr);
        let (q, u) = (p.qty.get(ni).copied().unwrap_or(0), p.uncapped.get(ni).copied().unwrap_or(0));
        out.sized.push(Sized { qty: q, rule_qty: u, capped: q < u });
    }
    out
}

/// The flip anchor and F17 of candidate `i` under the XC exit: the END of the first index candle closing across Supertrend 10 x 3
/// (cands' XS rule of the same setting), None when it comes at or after the square-off (the XC signal part is then the square-off).
pub(crate) fn xc_flip(fam: &Family, i: usize, long: bool, exit: &ExitSet, sq: i64) -> (Option<i64>, bool) {
    match cands::signal_exit(fam, i, long, exit) {
        Some((end, immediate)) if end * US_S < sq => (Some(end * US_S), immediate),
        _ => (None, false),
    }
}

/// INTRADAY: every candidate of the context priced at every N and level by `opt_pos::intraday` (the seam that replaces the cash
/// `price_all`; the table is the cash table's shape). Work in MONTH chunks, a fresh book per chunk and worker.
///
/// # Errors
/// A pricing error (never a refusal: refusals are counted by name) or a learning row after a test row.
#[allow(clippy::too_many_arguments, clippy::too_many_lines, reason = "one context's pricing")]
pub(crate) fn price_all_idx(env: &IdxEnv, fam: &Family, ctx: &feat_io::host::Ctx, exit: &ExitSet, long: bool, slots: &[i64], levels: &[u16], account: i64, hull_to: i32, threads: usize) -> Result<(Table, Vec<RowFacts>), String> {
    let mut by_month: BTreeMap<i32, Vec<usize>> = BTreeMap::new();
    for (i, c) in fam.cands.iter().enumerate() {
        by_month.entry(c.mk).or_default().push(i);
    }
    let chunks: Vec<Vec<usize>> = by_month.into_values().collect();
    let next = AtomicUsize::new(0);
    let nn = slots.len();
    let origin = price::at_sod(common_feat::cal::parse_ymd("2018-01-01").ok_or("origin")?, 0);
    type Res = (usize, Result<(RowOut, RowFacts), String>);
    let priced: Mutex<Vec<Res>> = Mutex::new(Vec::new());
    let errors: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let tp = Instant::now();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1) {
            s.spawn(|| {
                let mut local: Vec<Res> = Vec::new();
                loop {
                    let k = next.fetch_add(1, Ordering::Relaxed);
                    let Some(ch) = chunks.get(k) else { break };
                    let book = env.book();
                    for &i in ch {
                        let tr0 = Instant::now();
                        let Some(c) = fam.cands.get(i) else { continue };
                        let Some(se) = ctx.sessions.get(c.sess as usize) else { continue };
                        let t = c.end * US_S;
                        let sq = price::at_sod(se.day, 15 * 3600 + 10 * 60);
                        let (flip, immediate) = xc_flip(fam, i, long, exit, sq);
                        let r = match opt_pos::intraday::pick_intraday(&book, long, t, env.target) {
                            Err(e) => {
                                if let Ok(mut g) = errors.lock() {
                                    g.push(format!("candidate {i} ({} {}): {e}", se.day, c.end));
                                }
                                continue;
                            }
                            Ok(Err(code)) => Err(code.to_string()),
                            Ok(Ok(pk)) => match opt_pos::intraday::price_intraday_itm(&book, &pk, t, flip, immediate, slots, levels, account) {
                                Err(e) => {
                                    if let Ok(mut g) = errors.lock() {
                                        g.push(format!("candidate {i} ({} {}): {e}", se.day, c.end));
                                    }
                                    continue;
                                }
                                Ok(p) => {
                                    let cp = as_priced(&p);
                                    let facts = RowFacts {
                                        cand: i,
                                        folder: pk.folder.clone(),
                                        expiry: pk.expiry,
                                        lot: pk.lot,
                                        freeze: pk.freeze,
                                        max_order: pk.max_order,
                                        qty_a2: pk.info.qty_a2,
                                        depth: pk.info.depth,
                                        skipped_no_trade: pk.info.skipped_no_trade,
                                        skipped_big: pk.info.skipped_big,
                                        reference: pk.info.reference,
                                        ref_age_s: pk.info.ref_age_s,
                                        spot: pk.info.spot,
                                        expiring_today: pk.expiring_today,
                                        qty: p.qty.clone(),
                                        flags_or: p.lv.iter().flatten().fold(0u16, |a, x| a | x.flags),
                                        calls: p.calls,
                                        flip: flip.unwrap_or(0),
                                        immediate,
                                        price_us: u64::try_from(tr0.elapsed().as_micros()).unwrap_or(u64::MAX),
                                    };
                                    compact(&cp, c.day, nn, origin).map(|ro| (ro, facts))
                                }
                            },
                        };
                        local.push((i, r));
                    }
                }
                if let Ok(mut g) = priced.lock() {
                    g.extend(local);
                }
            });
        }
    });
    let errors = errors.into_inner().map_err(|_| "a pricing worker panicked")?;
    if let Some(e) = errors.first() {
        return Err(format!("index pricing error (never a refusal; {} in all): {e}", errors.len()));
    }
    let mut priced = priced.into_inner().map_err(|_| "a pricing worker panicked")?;
    priced.sort_by_key(|x| x.0);
    let price_s = tp.elapsed().as_secs_f64();
    let mut t = Table::empty(nn, price_s);
    let mut facts_out = Vec::new();
    let mut seen_test = false;
    for (i, r) in priced {
        let (p, f) = match r {
            Err(e) => {
                *t.refusals.entry(e.split(':').next().unwrap_or("?").to_string()).or_insert(0) += 1;
                continue;
            }
            Ok(x) => x,
        };
        let Some(c) = fam.cands.get(i) else { continue };
        if c.mk > hull_to {
            seen_test = true;
        } else if seen_test {
            return Err("a learning row after a test row: the learning rows are not a prefix".to_string());
        } else {
            t.nl += 1;
        }
        t.push_row(c.sess, c.mk, i, &p, nn);
        facts_out.push(f);
    }
    Ok((t, facts_out))
}

/// The IDX-ROWS table rows of a context (one per priced decision: the contract, why it was chosen, the size reference's age, the one-order
/// gate F8 at the A2 quantity, the quantity at every stored N).
pub(crate) fn facts_rows(run_id: &str, cname: &str, fam: &Family, facts: &[RowFacts], n_grid: &[i64], stored: &[i64], labels: &str) -> (Vec<Vec<String>>, u64, u64) {
    let mut rows = Vec::with_capacity(facts.len());
    let (mut f8_ok, mut f8_fail) = (0u64, 0u64);
    for f in facts {
        let c = fam.cands.get(f.cand);
        let ok = i64::try_from(f.qty_a2).unwrap_or(i64::MAX) <= f.max_order && f.qty_a2 > 0 && f.lot > 0 && f.qty_a2 % f.lot == 0;
        if ok {
            f8_ok += 1;
        } else {
            f8_fail += 1;
        }
        let q: Vec<String> = stored.iter().map(|n| n_grid.iter().position(|x| x == n).and_then(|ni| f.qty.get(ni)).map_or("-".to_string(), u64::to_string)).collect();
        rows.push(vec![
            run_id.to_string(),
            cname.to_string(),
            c.map_or(0, |c| c.day).to_string(),
            c.map_or(0, |c| c.k).to_string(),
            c.map_or(0, |c| c.end).to_string(),
            f.folder.clone(),
            idx_cal::day::fmt(f.expiry),
            u8::from(f.expiring_today).to_string(),
            f.spot.to_string(),
            f.depth.to_string(),
            f.skipped_no_trade.to_string(),
            f.skipped_big.to_string(),
            f.reference.to_string(),
            f.ref_age_s.to_string(),
            f.lot.to_string(),
            f.freeze.to_string(),
            f.max_order.to_string(),
            f.qty_a2.to_string(),
            if ok { "fits one order".to_string() } else { "F8 FAIL".to_string() },
            q.join(","),
            format!("{:#06x}", f.flags_or),
            f.calls.to_string(),
            if f.flip == 0 {
                "-".to_string()
            } else {
                let (d, sod) = idx_cal::storeio::ist_of(f.flip);
                format!("{} {}", idx_cal::day::fmt(d), idx_cal::storeio::hms(sod))
            },
            if f.immediate { "yes" } else { "no" }.to_string(),
            labels.to_string(),
        ]);
    }
    (rows, f8_ok, f8_fail)
}

/// The header of the IDX-ROWS table.
pub(crate) const FACTS_HEADER: [&str; 25] = [
    "run", "context", "day", "candle", "decision_end_s", "contract", "expiry_true", "expiring_today", "spot_paisa", "itm_depth", "skipped_no_trade", "skipped_big", "size_ref_paisa", "size_ref_age_s", "lot", "freeze", "max_one_order", "qty_a2", "f8_one_order_gate", "qty_at_stored_n", "flags_or", "pricer_calls", "xc_flip_decision_ist", "f17", "labels",
];

/// The kept rows of a context's outcome table indexed by candidate (the stored per-row width check).
#[must_use]
pub fn outcome_width(nn: usize) -> usize {
    price::record_width(nn, 5)
}

#[allow(dead_code, reason = "used by the positional mode")]
pub(crate) fn outcome_zero() -> Outcome {
    Outcome::default()
}

/// The readings of an index run: the context, the decision stream, the candidates (cut = the last decision instant), the lines and the
/// SN1 rows of the context dials.
pub(crate) struct IdxReadings {
    pub(crate) ctx: feat_io::host::Ctx,
    pub(crate) stream: feat_io::host::Stream,
    pub(crate) fam: Family,
    pub(crate) lines: Vec<String>,
    pub(crate) sn1: Vec<(String, String)>,
}

/// The S4 index readings of one family (the cash path's `cands::build` on the index stream; the H-* dials on the other timeframes of
/// the same underlying; the seven context dials from `feat_io::idxdials` when walked and `--context-dials on`).
///
/// # Errors
/// A refused instrument row, an empty stream, a dial source refused.
pub(crate) fn readings(inp: &crate::esn1::Inputs, spec: &crate::search::RunSpec, slice: Option<&crate::search::Slice>, cut: i64) -> Result<IdxReadings, String> {
    let irows = feat_io::instr::load_default()?;
    let ir = irows.get(inp.instr.underlying()).ok_or_else(|| format!("FEAT-INSTR-ROWS-v1.tsv (S4) has no row {}", inp.instr.underlying()))?;
    feat_io::instr::law(ir)?;
    if ir.segment != inp.instr.segment {
        return Err(format!("{}: instrument segment {} but the S4 read row says {}", inp.instr.underlying(), inp.instr.segment, ir.segment));
    }
    let ctx = ir.load_ctx()?;
    let store = Path::new(feat_io::defaults::STORE);
    let zr = ir.zread();
    let (stream, st) = feat_io::minute_stream_read(store, &zr, &spec.tf, &ctx);
    if stream.tf_s < 60 {
        return Err(format!("{}: the index seconds families are plan S12 (after the minute results; named)", spec.tf));
    }
    if stream.candles.is_empty() {
        return Err(format!("{} {}: the index stream is empty", inp.instr.underlying(), spec.tf));
    }
    let (s1, _) = feat_io::minute_stream_read(store, &zr, "1min", &ctx);
    let facts = feat_io::host::day_facts(&s1, &ctx);
    let lv = feat_io::host::levels(&ctx, &facts);
    let mut fam = cands::build(&ctx, &stream, &lv, (&inp.core.0, &inp.core.1), cut)?;
    let mut lines = vec![format!(
        "readings (S4 index rows, feat_io r3): {} candles ({}), {} decision candidates, {} not entry {:?} (Session::entry_at at the candle END), Muhurat candles excluded {} (Q39), K_tf {}, bars read {}, straddling candles kept {}",
        stream.candles.len(),
        stream.source,
        fam.cands.len(),
        fam.not_entry,
        fam.refused,
        st.muhurat_excluded,
        fam.kmax,
        st.bars,
        st.straddle_kept
    )];
    let wants_htf = inp.dials.iter().any(|d| d.id.starts_with("H-") && d.status.starts_with("LIVE") && slice.is_none_or(|s| s.has_dial(&d.id)));
    if wants_htf {
        let th = Instant::now();
        let lad = cands::htf_ladders(&inp.dials)?;
        let fam_tfs: Vec<String> = inp.run.list::<String>("minutes_family")?;
        let names: Vec<String> = fam_tfs.into_iter().filter(|x| *x != spec.tf).collect();
        let mut reads = Vec::with_capacity(names.len());
        for n in &names {
            let (s, _) = feat_io::minute_stream_read(store, &zr, n, &ctx);
            if s.candles.is_empty() {
                return Err(format!("H-* dials: the {n} index stream is empty"));
            }
            reads.push(cands::htf_reads(&ctx, &s, &lad)?);
        }
        cands::attach_htf(&mut fam, names.clone(), &reads);
        lines.push(format!("higher timeframes for the H-* dials: {} ({:.1} s); reads refused across the K13 reset per timeframe {:?}", names.join(", "), th.elapsed().as_secs_f64(), fam.htf_reset_refused));
    }
    let wants_ctx = spec.context_dials && inp.dials.iter().any(|d| crate::ctxdials::IDS.contains(&d.id.as_str()) && d.status.starts_with("LIVE") && slice.is_none_or(|s| s.has_dial(&d.id)));
    let mut sn1 = Vec::new();
    if wants_ctx {
        let tx = Instant::now();
        let peer = irows.get(&ir.peer).ok_or_else(|| format!("X-PEER row {} missing (S4)", ir.peer))?;
        let prep = feat_io::idxdials::IdxPrep::prepare(ir, peer, &ctx, store)?;
        let reads = prep.read_stream(&ctx, &rd_stream_ref(&stream));
        let mut out = Vec::with_capacity(fam.cands.len());
        let mut q = 0usize;
        for c in &fam.cands {
            let i = c.sess as usize;
            while reads.get(q).is_some_and(|r| (r.0, r.1) != (i, c.end)) {
                q += 1;
            }
            let r = reads.get(q).ok_or("index context dials: a candidate without its stream candle")?.2.ctx;
            out.push(crate::ctxdials::CtxRead { nifty_bp: r.nifty_bp, vix: r.vix, exp: r.exp, mend: r.mend, h52: r.h52, l52: r.l52, drng: r.drng });
        }
        fam.ctxr = out;
        lines.push(format!(
            "index context dials attached (S4 feat_io::idxdials; the X-NIFTY dial reads X-PEER = {} % change, D-EXP the TRUE expiry day from DP-IDXCAL, G-52H / G-52L the index official rows) in {:.1} s; store notes {}",
            ir.peer,
            tx.elapsed().as_secs_f64(),
            prep.notes.len()
        ));
        sn1.push(("context_dials".to_string(), format!("ON (S4 index rows: X-PEER {} in the X-NIFTY dial, X-VIX {}, D-EXP DP-IDXCAL {}, D-MEND, G-52H / G-52L / G-DRNG on the index official rows {})", ir.peer, ir.vix, ir.cal.display(), ir.bhav.display())));
    }
    Ok(IdxReadings { ctx, stream, fam, lines, sn1 })
}

fn rd_stream_ref(s: &feat_io::host::Stream) -> feat_io::host::Stream {
    s.clone()
}

pub(crate) fn rows_list(inp: &crate::esn1::Inputs, k: &str) -> Result<Vec<i64>, String> {
    let v = inp.instr.rows.get(k).ok_or_else(|| format!("IDXOPT row {k} missing (plan s3.4)"))?;
    v.split(',').map(|x| x.trim().parse::<i64>().map_err(|_| format!("IDXOPT row {k}: {x} is not a number"))).collect()
}

/// Runs an INDEX OPTIONS family (the instrument rows say IDXOPT): intraday here, positional in [`crate::idxpos::run_family_idx_pos`].
///
/// # Errors
/// A held gate, a refused input, an I/O error, a pricing error.
#[allow(clippy::too_many_lines, reason = "the index family's pipeline in one place, as search::run_family")]
pub fn run_family_idx(inp: &crate::esn1::Inputs, spec: &crate::search::RunSpec) -> Result<Vec<String>, String> {
    crate::refuse_if_red()?;
    if crate::sec::tf_secs(&spec.tf).is_some() {
        return Err(format!("{}: the index seconds families are plan S12 (after the minute results; named)", spec.tf));
    }
    if spec.period.is_some() {
        return Err("--from / --to restrict the seconds families only (named)".to_string());
    }
    if inp.instr.hold == crate::instr::Hold::Positional {
        return crate::idxpos::run_family_idx_pos(inp, spec);
    }
    let run = &inp.run;
    let mut lines = Vec::new();
    let gate = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, 8)?;
    lines.push(format!("{} gate ok={} hold={:?} pressure={:?} swap_used={:?} swap_free={:?} disk_free={:?}", crate::ist_now(), gate.ok, gate.hold, gate.pressure, gate.swap_used_mb, gate.swap_free_mb, gate.disk_free_gib));
    if !gate.ok {
        return Err(format!("launch gate HOLDS: {:?}", gate.hold));
    }
    let slice = spec.slice.as_deref().map(crate::search::Slice::load).transpose()?;
    let db = crate::db::Db::open(&spec.out.join("db"), crate::VERSION)?;
    let hb = spec.out.join("HEARTBEAT.tsv");
    let beat = |stage: &str, extra: String| {
        let _ = crate::monitor::heartbeat(&hb, &[("stage", stage.to_string()), ("rss_mb", format!("{:?}", crate::monitor::rss_mb())), ("detail", extra)]);
    };
    let t0 = Instant::now();
    let te = Instant::now();
    let env = IdxEnv::load(&inp.instr)?;
    lines.push(format!("index option inputs ({}): DP-IDXCAL {} sessions, {} series, the production source {}, target Rs {} a trade (A2), store {}; {:.1} s", env.index, env.cal.len(), env.series.len(), env.pack_note, env.target / 100, env.store.display(), te.elapsed().as_secs_f64()));
    let cut = run.hms("last_decision_before_ist")?;
    let rd = readings(inp, spec, slice.as_ref(), cut)?;
    lines.extend(rd.lines.iter().cloned());
    let (ctx, fam) = (&rd.ctx, &rd.fam);
    beat("readings", format!("{} candidates", fam.cands.len()));
    let cal = crate::wfio::calendar(&ctx.sessions)?;
    let (hull_from, hull_to) = inp.windows.hull();
    let levels: Vec<u16> = run.list("levels_bp")?;
    // options are bought: the money in is the premium (lev 1 / 1); the protection 10% on the 5-paisa tick (the pricer's own rows)
    let cfg = crate::price::Cfg {
        dial: common_fill::Dial::new(&levels).map_err(|e| e.to_string())?,
        lev: (1, 1),
        protection_bp: 1_000,
        limit_tick: 5,
        square_off_sod: run.hms("square_off_ist")?,
        cut_sod: cut,
        qty_rule: inp.qty_rule,
        account: run.num("account_paisa")?,
    };
    let account: i64 = run.num("account_paisa")?;
    let n_grid = rows_list(inp, "n_grid")?;
    let proxy_n = rows_list(inp, "proxy_n")?.first().copied().ok_or("IDXOPT row proxy_n empty")?;
    let slots: Vec<i64> = n_grid.iter().map(|n| account / n.max(&1)).collect();
    let sess_mk: Vec<i32> = ctx
        .sessions
        .iter()
        .map(|s| {
            let (y, m, _) = common_feat::cal::civil(s.day);
            y * 12 + i32::try_from(m).unwrap_or(1) - 1
        })
        .collect();
    let wins: Vec<(String, i32, i32)> = inp.windows.rows.iter().map(|r| (r.0.clone(), r.1, r.2)).collect();
    let lay = wf_sweep::acc::Layout::new(&sess_mk, (hull_from, hull_to), &wins).map_err(|e| e.to_string())?;
    let twin_draws: u32 = run.num("twin_draws")?;
    let twin_seed: u64 = run.num("twin_seed")?;
    let wf_data = PathBuf::from(run.get("windows_file")?).parent().map(Path::to_path_buf).ok_or("windows_file has no folder")?;
    let part_tpl = crate::rows::read_with_sha(&spec.data.join("PART-SNIPER-v1.tsv").to_string_lossy())?;
    let labels = inp.words.standing(1, "printed by the tool").join(" | ");
    let mut total = crate::search::CtxOut { lines: Vec::new(), searches: 0, wf_passed: 0, tt_passed: 0, both: 0, mismatches: 0 };
    let n_ctx = spec.sides.len() * spec.exits.len();
    let mut done_ctx = 0_usize;
    let tc = Instant::now();
    if let Some(l) = &spec.n_sweep {
        lines.push(format!("N swept: {l:?} of the index grid {n_grid:?} (every N priced and stored; a pick's N is searched over the swept N only; headline N {}; named)", inp.instr.rows.get("headline_n").map_or("?", String::as_str)));
    }
    for &long in &spec.sides {
        for exit_id in &spec.exits {
            let exit = inp.exits.iter().find(|e| &e.id == exit_id).ok_or_else(|| format!("exit set {exit_id} not in the exits file"))?;
            if inp.instr.rows.get("exit").is_some_and(|x| x != exit_id) {
                return Err(format!("exit {exit_id}: the IDXOPT intraday row set names exit {} (s20: one exit set)", inp.instr.rows.get("exit").map_or("?", String::as_str)));
            }
            let g = crate::monitor::launch_gate(run, spec.swap_base_mb, &spec.out, 1)?;
            if !g.ok {
                total.lines.push(format!("{} HOLD before {} {}: {:?} (named; the rest of the run waits for MAIN)", crate::ist_now(), if long { "LONG" } else { "SHORT" }, exit.id, g.hold));
                break;
            }
            let c = crate::search::context(inp, spec, &db, fam, ctx, &cfg, exit, long, &slots, &n_grid, proxy_n, &lay, &cal, &wf_data, &part_tpl.0, slice.as_ref(), (twin_seed, twin_draws), &labels, (hull_from, hull_to), &crate::search::Pricer::Idx(&env))?;
            total.searches += c.searches;
            total.wf_passed += c.wf_passed;
            total.tt_passed += c.tt_passed;
            total.both += c.both;
            total.mismatches += c.mismatches;
            for l in &c.lines {
                println!("{l}");
            }
            done_ctx += 1;
            let per = tc.elapsed().as_secs() / (done_ctx as u64).max(1);
            let left = per * (n_ctx.saturating_sub(done_ctx) as u64);
            println!("{} context {done_ctx} of {n_ctx} done; {per} s a context so far; about {} min left at this pace (ESTIMATE)", crate::ist_now(), left / 60);
            beat("context", format!("{} {} done ({done_ctx} of {n_ctx})", if long { "LONG" } else { "SHORT" }, exit.id));
        }
    }
    lines.extend(total.lines);
    lines.push(format!(
        "SUMMARY {} {} INDEX OPTIONS intraday: {} searches; walk-forward picks PASSED {} of {}; train/test picks PASSED {} of {}; both in the same search (REQ 73d) {} of {}; re-walk key mismatches {}",
        inp.instr.underlying(),
        spec.tf,
        total.searches,
        total.wf_passed,
        total.searches,
        total.tt_passed,
        total.searches,
        total.both,
        total.searches,
        total.mismatches
    ));
    let mut sn1 = vec![
        vec![spec.run_id.clone(), "version".to_string(), crate::VERSION.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "laws".to_string(), crate::law_line(), labels.clone()],
        vec![spec.run_id.clone(), "pricer".to_string(), format!("opt_pos (S8b + wave 4) via opt_pos::intraday; costs::option_clock; crate {} {}", cf21::VERSION, cf21::STEP), labels.clone()],
        vec![spec.run_id.clone(), "threads".to_string(), spec.threads.to_string(), labels.clone()],
        vec![spec.run_id.clone(), "gate".to_string(), format!("{gate:?}").replace('\n', " "), labels.clone()],
        vec![spec.run_id.clone(), "bar_source".to_string(), inp.words.line("BARSRC", &[]), labels.clone()],
        vec![spec.run_id.clone(), "stream_source".to_string(), rd.stream.source.clone(), labels.clone()],
        vec![spec.run_id.clone(), "slice".to_string(), slice.as_ref().map_or_else(|| "ALL wired values".to_string(), |s| format!("{} sha256 {}", s.name, s.sha)), labels.clone()],
        vec![spec.run_id.clone(), "money".to_string(), "options BOUGHT only: the money in is the premium (lev 1 / 1, no margin; MAIN-RULINGS-S7-S10 item 4: the pluggable margin is for SELLING legs)".to_string(), labels.clone()],
        vec![spec.run_id.clone(), "n_grid".to_string(), format!("{n_grid:?} (proxy N {proxy_n}; headline N {})", inp.instr.rows.get("headline_n").map_or("?", String::as_str)), labels.clone()],
        vec![spec.run_id.clone(), "outcome_row_bytes".to_string(), outcome_width(n_grid.len()).to_string(), labels.clone()],
        vec![spec.run_id.clone(), "wall_s".to_string(), format!("{:.1}", t0.elapsed().as_secs_f64()), labels.clone()],
    ];
    for (f, s) in inp.shas.iter().chain(env.shas.iter()) {
        sn1.push(vec![spec.run_id.clone(), format!("sha256 {f}"), s.clone(), labels.clone()]);
    }
    sn1.push(vec![spec.run_id.clone(), format!("sha256 {}", inp.instr_sha.0), inp.instr_sha.1.clone(), labels.clone()]);
    for (k, v) in &inp.instr.rows {
        sn1.push(vec![spec.run_id.clone(), format!("instrument {k}"), v.clone(), labels.clone()]);
    }
    sn1.push(vec![spec.run_id.clone(), "sha256 PART-SNIPER-v1.tsv".to_string(), part_tpl.1.clone(), labels.clone()]);
    sn1.push(vec![spec.run_id.clone(), "qty_rule".to_string(), format!("{} ({})", inp.qty_rule.id(), inp.qty_rule.label()), labels.clone()]);
    for (f, v) in &rd.sn1 {
        sn1.push(vec![spec.run_id.clone(), f.clone(), v.clone(), labels.clone()]);
    }
    db.write(&format!("SN1-RUN-{}.tsv", spec.tf), &["run", "field", "value", "labels"], &sn1)?;
    let n = db.verify()?;
    lines.push(format!("DB: {} tables audited, chain verified; run folder {}", n, spec.out.display()));
    for l in inp.words.standing(1, "printed by the tool") {
        lines.push(l);
    }
    lines.push(format!("{} done in {:.1} s", crate::ist_now(), t0.elapsed().as_secs_f64()));
    beat("done", String::new());
    Ok(lines)
}

#[cfg(test)]
mod tests {
    //! Host-side named cases of the index wiring (the pricer's own cases live in opt_pos::intraday::tests and pos_tests).
    use super::*;

    /// The 15-value index grid stores every N: 15 N x 5 levels in the cash encoding = 472 B (plan N8's "442 B" was 352 + 3 x 30 B).
    #[test]
    fn s7w4_outcome_row_width_of_the_15_value_grid() {
        assert_eq!(outcome_width(15), 472);
        assert_eq!(outcome_width(11), 352, "the cash row is unchanged");
    }

    /// s4.3a B2: a positional entry the pricer cannot look up is a DECIDE refusal by name (known at T), never a pricing error: a session
    /// without a DP-IDXCAL row (2018-09-03 is before the calendar's first session), an alias with no listed series; a listed one passes,
    /// and at or after 15:00 on an expiry day the NEXT expiry is the one looked up (s4.2 (5)).
    #[test]
    fn s7w4_positional_entry_without_a_cal_row_or_series_is_a_decide_refusal() {
        let d = |m: u32, dd: u32| idx_cal::day::from_civil(2025, m, dd);
        let mut cal = BTreeMap::new();
        cal.insert(d(8, 26), CalRow { file_read: d(8, 25), known_listed: d(8, 28), known_eff: d(8, 28), series: 1, next_eff: Some(d(9, 2)), d_exp: false });
        cal.insert(d(8, 28), CalRow { file_read: d(8, 26), known_listed: d(8, 28), known_eff: d(8, 28), series: 1, next_eff: Some(d(9, 2)), d_exp: true });
        cal.insert(d(8, 29), CalRow { file_read: d(8, 28), known_listed: d(9, 9), known_eff: d(9, 9), series: 3, next_eff: None, d_exp: false });
        let series = vec![SeriesRow { id: 1, aliases: vec![(d(8, 28), d(8, 22), d(8, 28))], true_last: d(8, 28) }, SeriesRow { id: 2, aliases: vec![(d(9, 2), d(8, 22), d(9, 2))], true_last: d(9, 2) }];
        let mut lad = String::from("file_date	form	index	expiry	ce_strikes	pe_strikes	lots	oi_gcd	oi_rows	file_cands
");
        for f in [d(8, 25), d(8, 26), d(8, 28)] {
            for e in [d(8, 28), d(9, 2)] {
                lad.push_str(&format!("{}	udiff	NIFTY	{}	2400000	2600000	75	-	-	-
", idx_cal::day::fmt(f), idx_cal::day::fmt(e)));
            }
        }
        let env = IdxEnv {
            index: "NIFTY".to_string(),
            cal,
            series,
            ladder: Ladder::parse(&lad).unwrap(),
            folders: Folders::default(),
            freeze: FreezeTable::default(),
            close: BTreeMap::new(),
            sess: Sessions::default(),
            store: PathBuf::new(),
            pack: PathBuf::new(),
            fails: Fails::default(),
            target: 50_000_000,
            shas: Vec::new(),
            pack_note: String::new(),
        };
        let at = |dd: Day, h: i64, m: i64| idx_cal::storeio::at_sod(dd, h * 3600 + m * 60);
        assert_eq!(env.entry_refusal(at(d(8, 25), 10, 0)), Some("no_cal_row"), "no DP-IDXCAL row");
        assert_eq!(env.entry_refusal(at(d(8, 26), 10, 0)), None, "the current expiry's series is listed");
        assert_eq!(env.entry_refusal(at(d(8, 28), 15, 15)), None, "15:15 on the expiry day: the NEXT expiry (listed) is looked up");
        assert_eq!(env.entry_refusal(at(d(8, 29), 10, 0)), Some("no_series"), "an alias with no listed series");
    }

    /// as_priced keeps every N and level, the ZERO_QTY cells at net 0, the sized counts and the re-walk count.
    #[test]
    fn s7w4_as_priced_maps_every_cell() {
        use opt_pos::intraday::{ILevel, IPriced, flag};
        let lv = |net: i64, f: u16| ILevel { net, flags: f, units: 75, entry_value: 100, exit_value: 120, ..ILevel::default() };
        let p = IPriced { qty: vec![975, 0], uncapped: vec![975, 0], lv: vec![vec![lv(5, flag::REWALKED), lv(6, 0)], vec![lv(0, flag::ZERO_QTY | flag::ZERO), lv(0, flag::ZERO_QTY | flag::ZERO)]], entry_first: 7, ..IPriced::default() };
        let c = as_priced(&p);
        assert_eq!(c.lv.len(), 2);
        assert_eq!((c.lv[0][0].net, c.lv[0][1].net, c.lv[1][0].net), (5, 6, 0));
        assert_eq!(c.rewalks, 1);
        assert_eq!(c.sized[1].qty, 0);
        assert!(c.lv[1][1].flags & flag::ZERO_QTY != 0);
    }
}
