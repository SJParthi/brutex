//! The outputs (SEARCH-SPEC 10). Line 1 of every file is the PREVIEW SEARCH line (the
//! TSVs as a `#` line), then the APPROXIMATE list. Money in paisa, printed as rupees by
//! integer division; percents from basis points, floored.

use std::fmt::Write as _;
use std::path::Path;

use crate::acct::{C0, Key, N_OPT, N_WIN, WIN_NAMES, cmp_key, opt_words};
use crate::common::{
    CORE_NAMES, CORE_WORDS, EXIT_NAMES, FAST_OF, LANE_FAST, LANE_NAMES, LANE_WORDS, N_MONTHS, SEARCH_LINE, SIDE6, TFS, fmt_date, fmt_ist,
    pct, permille, rs64, rupees, thousandths, window_label,
};
use crate::merge::{Judged, LuckVerdict, MergeOut, N_SCOPE, scope_name, unseen};
use crate::search::{Lib, SUMMARY_HEAD, unpack};

macro_rules! w {
    ($dst:expr) => {{
        let _ = writeln!($dst);
    }};
    ($dst:expr, $($arg:tt)*) => {{
        let _ = writeln!($dst, $($arg)*);
    }};
}

/// One twin's (random-entry baseline) headline figures.
#[derive(Clone, Debug, Default)]
pub(crate) struct Twin {
    pub(crate) seed: u64,
    pub(crate) key: Key,
    pub(crate) pass: usize,
    pub(crate) worst_bp: Option<i128>,
    pub(crate) hold_min: i64,
    pub(crate) hindsight: Key,
    pub(crate) words: String,
    pub(crate) secs: u64,
}

/// Everything the reports read besides the merge.
pub(crate) struct Info {
    pub(crate) command: String,
    pub(crate) tree_sha: String,
    pub(crate) started: String,
    pub(crate) threads: usize,
    pub(crate) stocks: usize,
    pub(crate) checks: Vec<String>,
    pub(crate) check23: Vec<String>,
    pub(crate) membership_label: String,
    pub(crate) membership_digest: String,
    pub(crate) cost_version: String,
    pub(crate) counts: crate::phase_a::Counts,
    pub(crate) notes: Vec<String>,
    pub(crate) nifty: String,
    pub(crate) phase_a_secs: u64,
    pub(crate) tf_secs: Vec<(usize, u64, u64, u64)>,
    pub(crate) merge_secs: u64,
    pub(crate) spool_bytes: u64,
    pub(crate) units: Vec<(usize, usize, usize, usize)>,
    pub(crate) twins: Vec<Twin>,
    pub(crate) twins_pending: bool,
    pub(crate) bare: Vec<String>,
    pub(crate) singles: Vec<String>,
    pub(crate) filters: Vec<String>,
    /// The light timeframes (indices): singles, base pairs and sequences exhaustive; triples and
    /// four-filter stacks through the build-up only.
    pub(crate) light: Vec<usize>,
}

/// The APPROXIMATE list (SEARCH-SPEC 11): the preview's items 1-23 with item 3 replaced
/// by 27 and the preview check's fixes applied (check1 notes 1, 2, 4, 7), then 24-36,
/// then the builder's.
pub(crate) fn approximate(info: &Info) -> Vec<String> {
    let c = &info.counts;
    vec![
        "1. Not the PSRB result. PSRB-STRAT is not built; this is a scratch tool in a scratch tree, never committed. Indicator code is the part tips' (PSRB-RSI 6d3ae223e5db and PSRB-BOLL 820f63a7a0e2 passed check 1; PSRB-STV 7b137fee7014, PSRB-WILLR cb2db489bc9d and PSRB-CCI 1222c57a7253 are still in check 1). The EMA getter scaled() is inserted in the scratch tree only. Readings are the published definitions, UNVERIFIED against TradingView's output.".to_owned(),
        "2. The cores are the preview's three rule sets (AUTHOR-V2 with CPR-20BP and the LEVEL Williams %R form; VIDEO-SCREEN with Williams %R long > -15, short < -85) plus K1 ST+BB and K2 ST+R1X; no other CPR definition, no CROSS form, no VIDEO-ORIGINAL-FIXED-STOP, no top-mover selection.".to_owned(),
        "3. (replaced by item 27)".to_owned(),
        "4. N4's quote range is not loaded (QRANGE-1 not landed): fills and stop triggers use the traded range only.".to_owned(),
        "5. Fill walk simplified: the whole quantity fills in one second at its worst print (a buy at the second's high, a sell at its low); no volume-share cap, no 3% market protection, no dynamic price band (FNO-BAND), no latency floor (FNO-CLOCK), no MTM 80% square-off (FNO-SEC-EXEC failed check r2). The quantity is sized from the fill price itself.".to_owned(),
        "6. Leverage flat 5x for every stock (FNO-LEVERAGE not used); slots stay Rs 1 lakh margin after losses (margin not checked against equity). Ties by (time, symbol), not SIZE-FIXED-20-SEEDED's seeded order; no luck spread.".to_owned(),
        format!(
            "7. Universe: {}. Survivorship: only today's 218 names can trade; a share that left F&O before 30 Oct 2026 has no stored series. Membership is joined through the measured TATAGLOBAL to TATACONSUM handover (TATACONSUM is a member on 18 stock-days in Feb 2020, filled from Zerodha minutes); only the GDFL price series of a renamed stock is not joined. {} stock-days were not members and {} had membership unknown (neither traded).",
            info.membership_label, c.member_false_days, c.member_unknown_days
        ),
        "8. Price basis: Zerodha candles and bands are corporate-action adjusted, GDFL seconds raw. Stops are mapped by the in-session ratio G/Z of the latest minute that has both (FSPEC 6.6 R1), not the exact factor, rounded to the paisa (a long's floored, a short's ceiled), not to the raw price step.".to_owned(),
        "9. Data parts still being fixed: REFOLD (the stored derived rungs, not read here), REPLACE, ZBASIS-1 (RELIANCE's minutes on another basis for 580 days, VBL, HINDUNILVR: read as stored; PIIND's 1-minute foreign-level minutes are read as stored), SIDX-1; missing months become holes (Zerodha) or minute-fallback days (GDFL).".to_owned(),
        "10. Day mask partial: M1 (the 218), M2 (a regular full session by pull::calendar::Runtime::kind_of), M3 (membership), M4 (window and the stock's span), M5 (Zerodha candles that day), 2021-02-24 never traded, 2019-05-30 a minute-fallback day for every stock. M7's other skip rows, M8 (trade-for-trade) and the FNO-DAYMASK part are not applied. The 2021-02-24 outage day's stored minutes (145, 09:15-11:39) are withheld by the copied load path (indicators::evaluator::CHARTER_NON_REGULAR_IST_DAYS) and never fold into any reading; the calendar answers that day as an outage session, so it adds no holes either (PREVIEW-SPEC 3.3's 'its stored candles still fold' is not followed; check1 note 1).".to_owned(),
        "11. Closing-auction eligibility from 2026-08-03 is inferred from where each stock-day's stored minutes end (last minute 15:14 or earlier: eligible, continuous close 15:15), not read from the dated NSE master.".to_owned(),
        "12. The previous-session close is the last traded minute's close (derive_minutes at Bucket::DAY), not NSE's official closing price.".to_owned(),
        "13. Costs per trade: each trade is its own contract note; the client-day aggregation is not applied (pre-2020-07-01 state stamp capped per trade, DP Rs 14.75 per trade, STT and stamp rounded per trade: each at least the client-day figure). FNO-COSTS's open F3 stands.".to_owned(),
        "14. Drawdown on closed-trade equity; no mark-to-market inside a trade (the ranking's fall groups the exits of one second; the preview's bp-of-peak marks after every exit).".to_owned(),
        "15. Timeframes 1, 30 and 60 minutes go beyond AD's 3/5/15 defaults (SS:348).".to_owned(),
        "16. The stop is live from the traded second after the entry's fill second.".to_owned(),
        "17. (builder) The month loader is this tool's own copy of cli::stored::load's path (the quarantine probe, BarFile::open_existing, the charter's non-regular sessions withheld), not a call into cli; cli's extra refusal of an uncalendared bar on a withheld day is not repeated.".to_owned(),
        "18. (builder) Built in release (opt-level 3, overflow checks on) with fat LTO off and 16 codegen units (CARGO_PROFILE_RELEASE_* overrides): build time only, same arithmetic.".to_owned(),
        "19. (builder) Signal candles at or after 15:08:00 are counted and never traded; an account's 'signals' counts the records it considered.".to_owned(),
        "20. (builder) A candle with a missing scheduled minute is a hole (spec 3.4): on 60-minute candles one missing minute blocks that hour and taints EMA 200 for 200 candles (about 29 sessions).".to_owned(),
        "21. (builder) A flip or exit order that finds no traded second left in the session closes at the worst print since the entry, by the broker, flagged unclosable (the square-off remainder rule applied to it). Intraday GDFL gaps near the close cause these (the preview counted 10 square-off and 28 unclosable lines over its 54 accounts: HDFCLIFE 2024-12-19 no GDFL second 15:07:00-15:10:15, ABCAPITAL 2024-12-09 and HAL 2024-12-11 from about 15:07:52, IDEA 2024-09-19 and 2021-09-16; check1 note 2).".to_owned(),
        format!(
            "22. (builder) Minute fallback fills: {} stock-days with records were filled from Zerodha minutes ({} on 2019-05-30, {} with the GDFL month missing or refused, {} with no traded GDFL second that day, {} unreadable).",
            c.fallback_days, c.fallback_gap_days, c.fallback_no_month_days, c.fallback_empty_days, c.fallback_read_error_days
        ),
        "23. (builder) The store is live while it is read (other parts may write); no digest of the bytes read is recorded, so a re-run can differ where those parts change data.".to_owned(),
        "24. A preview search, not PSE's search: 5 cores, 74 fixed filters in 39 groups plus the operator's ADDENDUM 3/4 library (97 fixed filters in 20 groups and 14,424 candle-code sequences), 6 Supertrend exits (3 at 1 minute), 6 sides, up to three filters per combination in the exhaustive pass and four in the build-up. Not here: PSE's 999 settings and 112 exit lanes, the Before and Chain sequence terms, the build-up with random jumps, refined values, top movers, the four sizing policies, FNO-WFT's 51 schemes, White's reality check, PBO, FNO-SEC-EXEC/FNO-PORTFOLIO's replay.".to_owned(),
        "25. Readings written in the tool (DMI/ADX, MACD's signal line, Stochastic, OBV, the VWAP accumulators, relative volume by time of day, the squeeze rank, every bar, structure, gap, range, time and NIFTY test, the candle anatomy, the candle codes, the chart structures and the Fibonacci touches) are this tool's own integer formulas, not engine parts, UNVERIFIED against a charting platform; RSI, Bollinger, Williams %R, CCI, the two-band Supertrend, EMA, ATR and the named candle patterns are the engine's, the last with pattern.rs's UNVERIFIED classical thresholds.".to_owned(),
        "26. Not ready means false: a filter on a cold, tainted or flat reading, or on a window with a hole or a missing candle, previous session or NIFTY minute, is false. Minute-based filters read the stored minutes as they are: a missing minute adds nothing.".to_owned(),
        "27. Walk-forward is anchored yearly (7 test years from 2020, 82 unseen months) plus one fixed split; the learning window always starts at 2018-09; months before 2020 are never unseen. It is not FNO-WFT.".to_owned(),
        "28. Choosing reads this tool's own account, the preview's slots, not FNO-PORTFOLIO's.".to_owned(),
        "29. The core's Supertrend side is the exit lane's; on a faster-timeframe exit it is read on the lane candle that closes at T.".to_owned(),
        "30. Drawdown for ranking is the closed-trade fall in paisa compared in tenths of a percent of Rs 20 lakh; the preview's bp-of-peak is printed beside it.".to_owned(),
        "31. The luck test is a sign-flip test over 82 months judged against Rs 20 lakh for every month, plus a random-entry baseline at fixed seeds; no bootstrap of sums, no Bonferroni, no deflated maximum.".to_owned(),
        "32. FADE trades use the traded side's band of the same Supertrend; where it is already crossed at entry the trade is stopped on the next traded second ('stopped at once', counted).".to_owned(),
        "33. Controls are tried only on each window's plain top 20 per scope, not on every combination; SIZE AFTER A FALL re-prices the halved quantity at the same fills.".to_owned(),
        "34. NIFTY 50 is Zerodha's stored index minutes as they are (IDXGAP-1 not landed; a missing NIFTY minute or candle makes E08-E10 false); the previous NIFTY close is its last stored minute's close.".to_owned(),
        "35. A filter the core implies is evaluated and counted; two filters of one group are never combined.".to_owned(),
        "36. Phase A walks every candle where any core holds for every exit lane and both traded sides; a candle no core takes at a lane is never walked at that lane.".to_owned(),
        "37. (builder) The ADDENDUM 3/4 library enters as singles, as pairs with each of the 74 and with each other (different groups), as sequences alone, and as the fourth (or third) filter in the build-up; it is not in the exhaustive triples (that would be about 30 times the search). The exact counts are in the counts section.".to_owned(),
        "38. (builder) The build-up extends every entry of fewer than four fixed filters in each window's TF and TF x SIDE top 20 by one filter of another group, once, and offers the result to that window only (its seeds were chosen on that window's learning months).".to_owned(),
        "39. (builder) Candle codes: direction up when C > O, or C = O with the close in the upper half (2C >= H + L); a zero-range candle has no code; a short's code is the code of the price-mirrored candle. Sequences use the candles of today's session only (t-1 and t-2 must hold bars and not be holes).".to_owned(),
        "40. (builder) The monthly trade caps (10, 20, 40 a month, ADDENDUM 3 (1)) take records in time order and, among the records of one instant, the strongest first by the signal candle's relative volume (its volume over its 20-candle average), ties by symbol; the day caps keep the admission order. Strength cannot reorder across instants without reading ahead.".to_owned(),
        "41. (builder) Targets of 2R and 3R (ADDENDUM 3 (3)) are not built: PSRB's stop and exit are the Supertrend only (operator-inputs item 19), as ADDENDUM 3 itself says; the minimum expected move is built as M01-M03 (ATR 14 of the signal timeframe) beside A18 (daily ATR).".to_owned(),
        "42. (builder) Chart structures and Fibonacci touches use fractal swings of 2 and 3 candles confirmed n candles later, in the stock's own candle order across sessions (holes break a window); touches count from the candle after the swing's confirmation; no invalidation when price runs beyond the swing.".to_owned(),
        format!(
            "44. (builder) MEASURED COST: the exhaustive pass costs about 1 ms of CPU per unit record (all 90,053 combinations, 6 accounts), about 37 CPU-hours for every timeframe, against SEARCH-SPEC 12.4's ESTIMATE of about 7 (the kept sets average far denser than the estimated 8%), on a machine shared with other runs (load 45-115 on 14 cores; this run got about 3 cores). So the exhaustive pass (every single, pair and triple of the 74, the ADDENDUM pairs, the sequences) runs on {}; on {} the pass is light: the bare core, every single (171), every pair of the 74 (2,651) and every candle-code sequence (14,424), then the build-up from each window's top 5 per heap (one more filter of another group on each seed: pairs with an ADDENDUM filter and triples, on the seed's own account), chosen on learning months only. The control stage runs over the GLOBAL and the six TF scopes' plain top 20 of the seven test windows (the TF x SIDE scopes keep their plain picks).",
            (0..6).filter(|t| !info.light.contains(t)).map(|t| format!("{} min", TFS.get(t).copied().unwrap_or(0))).collect::<Vec<_>>().join(", "),
            info.light.iter().map(|t| format!("{} min", TFS.get(*t).copied().unwrap_or(0))).collect::<Vec<_>>().join(", ")
        ),
        "43. (builder) The ranking keys 5-7 (fewer trades, shorter holding, net per trade) decide only between equal drawdown tenths, net and accuracy (SEARCH-SPEC 13.1 default); ADDENDUM 3 (4)'s preference is shown through the printed trades per month and net per trade.".to_owned(),
    ]
}

fn head(info: &Info, first: &str) -> String {
    let mut s = String::new();
    w!(s, "{first}");
    w!(s);
    w!(s, "APPROXIMATE (every output of this preview search carries this list):");
    w!(s);
    for l in approximate(info) {
        w!(s, "- {l}");
    }
    w!(s);
    s
}

fn tsv_head(info: &Info) -> String {
    let mut s = String::new();
    w!(s, "# {SEARCH_LINE}");
    for l in approximate(info) {
        w!(s, "# APPROXIMATE {l}");
    }
    s
}

/// A recipe in plain words.
pub(crate) fn recipe_words(id: u64) -> String {
    let r = unpack(id);
    let tf = TFS.get(r.ti).copied().unwrap_or(0);
    let fast = FAST_OF.get(r.ti).copied().flatten().and_then(|f| TFS.get(f).copied());
    let lane = LANE_WORDS.get(r.lane).copied().unwrap_or("?");
    let lane = if LANE_FAST.get(r.lane).copied().unwrap_or(false) {
        format!("{lane} ({} min)", fast.unwrap_or(0))
    } else {
        lane.to_owned()
    };
    format!(
        "{tf} min candles, core {} ({}), exit {} {} (stop on the band trailed tighter, exit on the first close across it, 15:07:59 / 15:10), side {}, filters: {}; {}",
        CORE_NAMES.get(r.core).copied().unwrap_or("?"),
        CORE_WORDS.get(r.core).copied().unwrap_or("?"),
        LANE_NAMES.get(r.lane).copied().unwrap_or("?"),
        lane,
        SIDE6.get(r.side).copied().unwrap_or("?"),
        r.combo.words(),
        opt_words(r.opt)
    )
}

/// A short label.
pub(crate) fn recipe_label(id: u64) -> String {
    let r = unpack(id);
    format!(
        "{}min {} {} {} [{}] opt{}",
        TFS.get(r.ti).copied().unwrap_or(0),
        CORE_NAMES.get(r.core).copied().unwrap_or("?").split(' ').next().unwrap_or("?"),
        LANE_NAMES.get(r.lane).copied().unwrap_or("?"),
        SIDE6.get(r.side).copied().unwrap_or("?"),
        r.combo.ids(),
        r.opt
    )
}

/// One month of a judged account.
struct MLine {
    mi: usize,
    cap: i128,
    trades: u32,
    wins: u32,
    gross: i64,
    charges: i64,
    net: i64,
    bp: Option<i128>,
    pass: bool,
    eq: i128,
}

fn mlines(j: &Judged) -> Vec<MLine> {
    let mut cap = C0;
    let mut v = Vec::new();
    for mi in j.a..j.b {
        let Some(m) = j.row.get(mi) else { continue };
        let net = i128::from(m.net);
        let bp = (cap > 0).then(|| (10_000 * net).div_euclid(cap));
        let pass = m.trades > 0 && cap > 0 && 40 * net >= cap;
        v.push(MLine { mi, cap, trades: m.trades, wins: m.wins, gross: m.gross, charges: m.charges, net: m.net, bp, pass, eq: cap + net });
        cap += net;
    }
    v
}

/// The figures of a judged account in one line.
fn figures(j: &Judged, with_book: bool) -> String {
    let k = &j.key;
    let ml = mlines(j);
    let months = j.b - j.a;
    let pass = ml.iter().filter(|m| m.pass).count();
    let mut bps: Vec<i128> = ml.iter().map(|m| m.bp.unwrap_or(i128::MIN)).collect();
    bps.sort_unstable();
    let worst = bps.first().copied().filter(|x| *x != i128::MIN);
    let median = bps.get(bps.len().saturating_sub(1) / 2).copied().filter(|x| *x != i128::MIN);
    let t = i64::from(k.trades);
    let tpm = if months > 0 { (10 * t).div_euclid(i64::try_from(months).unwrap_or(1)) } else { 0 };
    let hold = if t > 0 { (k.hold / 60).div_euclid(t) } else { 0 };
    let win = if t > 0 { (1_000 * i64::from(k.wins)).div_euclid(t) } else { 0 };
    let cpt = if t > 0 { k.charges.div_euclid(t) } else { 0 };
    let npt = if t > 0 { k.net.div_euclid(t) } else { 0 };
    let share = if k.gross > 0 { format!("{} of gross", permille(i128::from((1_000 * k.charges).div_euclid(k.gross)))) } else { "gross <= 0".to_owned() };
    let book = if with_book {
        let c = &j.counters;
        let pf = if c.gross_loss > 0 { thousandths((1_000 * c.gross_win).div_euclid(c.gross_loss)) } else { "no loss".to_owned() };
        let aw = if c.wins > 0 { rupees(c.gross_win.div_euclid(i128::from(c.wins))) } else { "no wins".to_owned() };
        let al = if c.losses > 0 { rupees((-c.gross_loss).div_euclid(i128::from(c.losses))) } else { "no losses".to_owned() };
        format!(
            ", profit factor {pf}, average win {aw}, average loss {al}, max drawdown of peak {} ({}, peak {} low {})",
            pct(j.dd_bp),
            rupees(j.dd_paisa),
            if j.dd_peak_ts > 0 { fmt_date(j.dd_peak_ts) } else { "start".to_owned() },
            if j.dd_low_ts > 0 { fmt_date(j.dd_low_ts) } else { "-".to_owned() }
        )
    } else {
        String::new()
    };
    format!(
        "{pass} of {months} months >= +2.5%, worst month {}, median month {}, max drawdown {}.{}% of Rs 20 lakh ({}), net {} ({} of Rs 20 lakh), {} trades ({}.{} a month), average holding {} min, win rate {}, gross {}, charges {} ({} a trade, {}), net per trade {}{}",
        worst.map_or_else(|| "account <= 0".to_owned(), pct),
        median.map_or_else(|| "account <= 0".to_owned(), pct),
        k.dd / 10,
        k.dd % 10,
        rs64(k.fall),
        rs64(k.net),
        pct((10_000 * i128::from(k.net)).div_euclid(C0)),
        k.trades,
        tpm / 10,
        tpm % 10,
        hold,
        permille(i128::from(win)),
        rs64(k.gross),
        rs64(k.charges),
        rs64(cpt),
        share,
        rs64(npt),
        book
    )
}

fn gaps(j: &Judged) -> String {
    let v: Vec<String> = mlines(j)
        .iter()
        .filter(|m| !m.pass)
        .map(|m| match m.bp {
            Some(bp) => format!("{} {} (gap {} pts, {} trades)", window_label(m.mi), pct(bp), pct(250 - bp), m.trades),
            None => format!("{} account <= 0", window_label(m.mi)),
        })
        .collect();
    if v.is_empty() { "none".to_owned() } else { v.join("; ") }
}

fn worst_of(j: &Judged) -> Option<i128> {
    mlines(j).iter().map(|m| m.bp.unwrap_or(i128::MIN)).min().filter(|x| *x != i128::MIN)
}

fn months_table(j: &Judged, s: &mut String) {
    w!(s, "| month | recipe traded | capital at start | trades | wins | gross | charges | net | net % | >= +2.5% | gap to +2.5% | equity at end |");
    w!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|");
    for m in mlines(j) {
        let year = m.mi.saturating_sub(16) / 12;
        let rid = j.ids.get(if j.ids.len() > 1 { year.min(6) } else { 0 }).copied().flatten();
        w!(
            s,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            window_label(m.mi),
            rid.map_or_else(|| "-".to_owned(), recipe_label),
            rupees(m.cap),
            m.trades,
            m.wins,
            rs64(m.gross),
            rs64(m.charges),
            rs64(m.net),
            m.bp.map_or_else(|| "account <= 0".to_owned(), pct),
            if m.pass { "yes" } else { "no" },
            m.bp.map_or_else(|| "-".to_owned(), |b| if m.pass { "-".to_owned() } else { pct(250 - b) }),
            rupees(m.eq)
        );
    }
    w!(s);
}

fn verdict_line(m: &MergeOut, lv: &LuckVerdict, info: &Info) -> String {
    let h = m.chains.first().and_then(|v| v.first());
    let Some(h) = h else { return "no headline chain (no pick)".to_owned() };
    let pass = mlines(h).iter().filter(|x| x.pass).count();
    let all = pass == 82;
    let twins = if info.twins.is_empty() {
        "random-entry baseline: pending".to_owned()
    } else {
        let beats = info.twins.iter().filter(|t| cmp_key(&h.key, &t.key) == std::cmp::Ordering::Less).count();
        format!(
            "random entry: beats {beats} of {} seed(s) run{}",
            info.twins.len(),
            if info.twins_pending { " (the other planned seeds still running)" } else { "" }
        )
    };
    let sign = if lv.clears {
        format!("sign test clears ({} of 1,000 draws reach {} months; the bar L95 is {})", lv.beaten, lv.observed, lv.l95)
    } else {
        format!(
            "sign test does not clear (the best of the tried series by sign luck alone passes {} of 82 months in half the draws; {} of 1,000 draws reach the chain's {} months; the bar L95 is {})",
            lv.median, lv.beaten, lv.observed, lv.l95
        )
    };
    let beats_all = !info.twins.is_empty() && info.twins.iter().all(|t| cmp_key(&h.key, &t.key) == std::cmp::Ordering::Less);
    let verdict = if all && lv.clears && beats_all { "PASS" } else { "MISS" };
    format!("{verdict}: {pass} of 82 unseen months >= +2.5%; luck: {sign}; {twins}")
}

/// SEARCH-RESULTS.md and REPORT.md share this body.
#[allow(clippy::too_many_lines, reason = "one report")]
fn body(m: &MergeOut, info: &Info, lib: &Lib, lv: &LuckVerdict, full: bool) -> String {
    let mut s = String::new();
    let tried_total = m.tried_b + m.tried_buildup + m.tried_ctl;
    w!(s, "## Headline: the GLOBAL pick #1 chain on the 82 unseen months (2020-01..2026-10)");
    w!(s);
    w!(s, "**{}**", verdict_line(m, lv, info));
    w!(s);
    if let Some(h) = m.chains.first().and_then(|v| v.first()) {
        w!(s, "- Figures: {}.", figures(h, true));
        w!(s, "- Exits: {}.", EXIT_NAMES.iter().zip(h.counters.exits.iter()).map(|(n, c)| format!("{n} {c}")).collect::<Vec<_>>().join(", "));
        w!(s, "- Blocked by the controls: day loss {}, month loss {}, target lock {}, trade cap {}; half-size trades {}.", h.counters.blocked[0], h.counters.blocked[1], h.counters.blocked[2], h.counters.blocked[3], h.counters.half_trades);
        w!(s);
        w!(s, "### The chosen recipe each year (chosen on 2018-09 .. the year before only)");
        w!(s);
        for (wi, id) in h.ids.iter().enumerate() {
            let (a, b) = unseen(wi);
            match id {
                Some(id) => {
                    let jj = m.judged.get(wi * N_SCOPE).and_then(|v| v.first());
                    w!(s, "{}. **{}** ({} .. {}): {}", wi + 1, WIN_NAMES.get(wi).copied().unwrap_or("?"), window_label(a), window_label(b - 1), recipe_words(*id));
                    if let Some(e) = m.ctl.get(wi * N_SCOPE).and_then(|h| h.v.first()) {
                        let k = &e.key;
                        w!(
                            s,
                            "   - learning months (2018-09 .. {}): {} of {} months >= +2.5%, max drawdown {}.{}% of Rs 20 lakh, net {}, {} trades",
                            window_label(crate::acct::PREFIX.get(wi).copied().unwrap_or(16) - 1),
                            k.months - k.below,
                            k.months,
                            k.dd / 10,
                            k.dd % 10,
                            rs64(k.net),
                            k.trades
                        );
                    }
                    if let Some(jj) = jj {
                        w!(s, "   - its unseen year on a fresh Rs 20 lakh: {}", figures(jj, true));
                    }
                }
                None => w!(s, "{}. {}: no pick", wi + 1, WIN_NAMES.get(wi).copied().unwrap_or("?")),
            }
        }
        w!(s);
        w!(s, "### Every unseen month of the headline chain");
        w!(s);
        months_table(h, &mut s);
    }
    w!(s, "## The top 5 GLOBAL chains (pick #k of every year, one account)");
    w!(s);
    w!(s, "| k | months >= +2.5% of 82 | worst month | max drawdown (tenths of Rs 20 lakh) | net | trades | avg holding min | win rate |");
    w!(s, "|---|---|---|---|---|---|---|---|");
    if let Some(v) = m.chains.first() {
        for (k, j) in v.iter().enumerate() {
            let t = i64::from(j.key.trades);
            w!(
                s,
                "| {} | {} | {} | {}.{}% | {} | {} | {} | {} |",
                k + 1,
                mlines(j).iter().filter(|x| x.pass).count(),
                worst_of(j).map_or_else(|| "-".to_owned(), pct),
                j.key.dd / 10,
                j.key.dd % 10,
                rs64(j.key.net),
                j.key.trades,
                if t > 0 { (j.key.hold / 60).div_euclid(t) } else { 0 },
                if t > 0 { permille(i128::from((1_000 * i64::from(j.key.wins)).div_euclid(t))) } else { "-".to_owned() }
            );
        }
    }
    w!(s);
    w!(s, "The plain GLOBAL pick #1 chain (no account control, from the phase B rows): {}.", figures(&m.plain_chain, false));
    w!(s);
    w!(s, "## The fixed split (choose on 2018-09..2022-12, judge on 2023-01..2026-10, 46 months)");
    w!(s);
    for (k, j) in m.fixed.iter().enumerate() {
        w!(s, "{}. {}", k + 1, j.ids.first().copied().flatten().map_or_else(|| "-".to_owned(), recipe_words));
        w!(s, "   - {}", figures(j, true));
        w!(s, "   - months below +2.5% with each gap: {}", gaps(j));
    }
    w!(s);
    w!(s, "## Each timeframe's own chain (scope TF, pick #1)");
    w!(s);
    for ti in 0..6 {
        if let Some(j) = m.chains.get(1 + ti).and_then(|v| v.first()) {
            w!(s, "- {} min: {}.", TFS.get(ti).copied().unwrap_or(0), figures(j, true));
        }
    }
    w!(s);
    w!(s, "## The luck test");
    w!(s);
    w!(s, "- Tried: {} series in phase B (exhaustive {} + build-up {}) and {} in the control stage ({} recipes x {} option sets, option 0 counted in phase B): {} in all (about 2^{}).", m.tried_b + m.tried_buildup, m.tried_b, m.tried_buildup, m.tried_ctl, m.recipes_ctl, N_OPT, tried_total, 64 - tried_total.leading_zeros());
    w!(s, "- Per cell: {} combinations (SEARCH-SPEC 6.3's {} = 1 + {} singles + {} pairs + {} triples; plus {} ADDENDUM singles, {} ADDENDUM x 74 pairs, {} ADDENDUM pairs and {} candle-code sequences), 6 accounts each.", lib.counts.per_cell(), lib.counts.spec_cell(), lib.counts.singles, lib.counts.pairs, lib.counts.triples, lib.counts.l2_singles, lib.counts.l2_base_pairs, lib.counts.l2_pairs, lib.counts.seqs);
    if !info.light.is_empty() {
        w!(
            s,
            "- The light timeframes ({}): {} combinations per cell (1 + {} singles + {} pairs of the 74 + {} sequences), then the build-up; the exhaustive count above is for {}.",
            info.light.iter().map(|t| format!("{} min", TFS.get(*t).copied().unwrap_or(0))).collect::<Vec<_>>().join(", "),
            1 + lib.counts.singles + lib.counts.l2_singles + lib.counts.pairs + lib.counts.seqs,
            lib.counts.singles + lib.counts.l2_singles,
            lib.counts.pairs,
            lib.counts.seqs,
            (0..6).filter(|t| !info.light.contains(t)).map(|t| format!("{} min", TFS.get(t).copied().unwrap_or(0))).collect::<Vec<_>>().join(", ")
        );
    }
    w!(s, "- Sign-flip test (1,000 draws, seed 20261106, C0 = Rs 20 lakh every month): the headline chain has {} months with 40 x net >= Rs 20 lakh; {} draws of 1,000 reach it by luck; the median best-by-luck is {} months; L95 (the smallest count that would clear) is {}; {}.", lv.observed, lv.beaten, lv.median, lv.l95, if lv.clears { "CLEARS" } else { "does NOT clear" });
    if info.twins.is_empty() {
        w!(s, "- Random-entry baseline: pending (the same search with the filters replaced by random filters of the same sizes, 3 fixed seeds, runs after this report; REPORT-FINAL.md carries it).");
    } else {
        w!(s, "- Random-entry baseline (the whole search re-run with every filter replaced by a random filter of the same size; cores, exits, sides and timeframes real):");
        w!(s);
        w!(s, "| seed | GLOBAL chain months >= +2.5% of 82 | worst month | max drawdown | net | trades | avg holding min | the real chain ranks above it | hindsight best (98 months) | run time |");
        w!(s, "|---|---|---|---|---|---|---|---|---|---|");
        let real = m.chains.first().and_then(|v| v.first()).map(|j| j.key);
        for t in &info.twins {
            let above = real.is_some_and(|r| cmp_key(&r, &t.key) == std::cmp::Ordering::Less);
            let tt = i64::from(t.key.trades);
            w!(
                s,
                "| {} | {} | {} | {}.{}% | {} | {} | {} | {} | {} months pass, net {} | {} s |",
                t.seed,
                t.pass,
                t.worst_bp.map_or_else(|| "-".to_owned(), pct),
                t.key.dd / 10,
                t.key.dd % 10,
                rs64(t.key.net),
                t.key.trades,
                if tt > 0 { (t.key.hold / 60).div_euclid(tt) } else { 0 },
                if above { "yes" } else { "no" },
                t.hindsight.months - t.hindsight.below,
                rs64(t.hindsight.net),
                t.secs
            );
        }
    }
    w!(s);
    w!(s, "## Closest misses (every chain of every scope and k, ranked on its unseen months)");
    w!(s);
    let mut all: Vec<(Key, usize, usize)> = Vec::new();
    for (si, v) in m.chains.iter().enumerate() {
        for (k, j) in v.iter().enumerate() {
            let mut key = j.key;
            key.id = u64::try_from(si * 8 + k).unwrap_or(0);
            all.push((key, si, k));
        }
    }
    all.sort_by(|a, b| cmp_key(&a.0, &b.0));
    for (n, (_, si, k)) in all.iter().take(20).enumerate() {
        let Some(j) = m.chains.get(*si).and_then(|v| v.get(*k)) else { continue };
        w!(s, "{}. {} pick #{}: {}", n + 1, scope_name(*si), k + 1, figures(j, true));
        w!(s, "   - months below +2.5% with each gap: {}", gaps(j));
        w!(s, "   - nearest floor met (its worst unseen month): {}", worst_of(j).map_or_else(|| "-".to_owned(), pct));
    }
    w!(s);
    let best_floor = m
        .chains
        .iter()
        .enumerate()
        .flat_map(|(si, v)| v.iter().enumerate().map(move |(k, j)| (si, k, j)))
        .filter_map(|(si, k, j)| worst_of(j).map(|w| (w, si, k, j)))
        .max_by_key(|(w, si, k, _)| (*w, std::cmp::Reverse((*si, *k))));
    if let Some((wb, si, k, j)) = best_floor {
        w!(s, "Over all chains, the highest worst unseen month (the nearest floor met) is {} by {} pick #{} (max drawdown {}.{}% of Rs 20 lakh, {} trades).", pct(wb), scope_name(si), k + 1, j.key.dd / 10, j.key.dd % 10, j.key.trades);
        w!(s);
    }
    w!(s, "## The best recipes by THE RANKING, chosen with hindsight (every month in-sample: never a headline)");
    w!(s);
    if let Some(h) = m.plain.get(7 * N_SCOPE) {
        for (n, e) in h.v.iter().enumerate() {
            let k = &e.key;
            let t = i64::from(k.trades);
            w!(
                s,
                "{}. {} -- {} of {} months >= +2.5%, max drawdown {}.{}% of Rs 20 lakh, net {}, {} trades, avg holding {} min, win rate {}",
                n + 1,
                recipe_words(k.id),
                k.months - k.below,
                k.months,
                k.dd / 10,
                k.dd % 10,
                rs64(k.net),
                k.trades,
                if t > 0 { (k.hold / 60).div_euclid(t) } else { 0 },
                if t > 0 { permille(i128::from((1_000 * i64::from(k.wins)).div_euclid(t))) } else { "-".to_owned() }
            );
        }
    }
    w!(s);
    if full {
        w!(s, "## Every window's GLOBAL picks #1..#5 (with controls) against the plain pick");
        w!(s);
        for wi in 0..7 {
            w!(s, "### {}", WIN_NAMES.get(wi).copied().unwrap_or("?"));
            w!(s);
            if let Some(v) = m.judged.get(wi * N_SCOPE) {
                for (k, j) in v.iter().enumerate() {
                    w!(s, "{}. {}", k + 1, j.ids.first().copied().flatten().map_or_else(|| "-".to_owned(), recipe_words));
                    w!(s, "   - unseen: {}", figures(j, true));
                }
            }
            if let Some(e) = m.plain.get(wi * N_SCOPE).and_then(|h| h.v.first()) {
                let (a, b) = unseen(wi);
                let k = crate::acct::key_range(e.row.as_deref().unwrap_or(&crate::acct::ROW0), a, b, e.key.id);
                w!(s, "- plain pick #1: {} -- unseen {} of {} months, net {}, {} trades", recipe_words(e.key.id), k.months - k.below, k.months, rs64(k.net), k.trades);
            }
            w!(s);
        }
        w!(s, "ADD-RISK point 3: \"these rules cap bad days and bank good months; they cannot turn a losing edge into a winning one. The walk-forward shows whether they help on unseen months.\"");
        w!(s);
    }
    w!(s, "## Counts");
    w!(s);
    let c = &info.counts;
    w!(s, "- Phase A: {} stocks, {} s; spool {} bytes; stock-days traded (mask passed) {}, with records {}; minute-fallback days {}.", info.stocks, info.phase_a_secs, info.spool_bytes, c.trade_days, c.candidate_days, c.fallback_days);
    for ti in 0..6 {
        let rec: Vec<String> = (0..6).map(|l| c.records.get(ti).and_then(|x| x.get(l)).copied().unwrap_or(0).to_string()).collect();
        let once: Vec<String> = (0..6)
            .map(|l| {
                let a = c.at_once.get(ti).and_then(|x| x.get(l));
                format!("{}/{}", a.and_then(|x| x.first()).copied().unwrap_or(0), a.and_then(|x| x.get(1)).copied().unwrap_or(0))
            })
            .collect();
        w!(
            s,
            "- {} min: candles {}, holes {}, decision candles {} (within 10 clean candles of a hole {}, within 200 {}); records by lane X1..X6 {}; stopped at once WITH/FADE by lane {}.",
            TFS.get(ti).copied().unwrap_or(0),
            c.candles.get(ti).copied().unwrap_or(0),
            c.holes.get(ti).copied().unwrap_or(0),
            c.decisions.get(ti).copied().unwrap_or(0),
            c.taint10.get(ti).copied().unwrap_or(0),
            c.taint200.get(ti).copied().unwrap_or(0),
            rec.join(" / "),
            once.join(" / ")
        );
    }
    for (ti, secs, tried, bu) in &info.tf_secs {
        w!(s, "- Phase B {} min: {} s, {} rows exhaustive, {} rows build-up.", TFS.get(*ti).copied().unwrap_or(0), secs, tried, bu);
    }
    w!(s, "- Merge: {} s.", info.merge_secs);
    w!(s, "- NIFTY: {}.", info.nifty);
    w!(s);
    s
}

fn write_text(dir: &Path, file: &str, body: &str) -> Result<(), String> {
    let p = dir.join(file);
    std::fs::write(&p, body).map_err(|e| format!("{}: {e}", p.display()))
}

/// Every output of one search (real or twin).
#[allow(clippy::too_many_lines, reason = "every output")]
pub(crate) fn write_all(dir: &Path, m: &MergeOut, info: &Info, lib: &Lib, lv: &LuckVerdict, final_: bool) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    // SEARCH-RESULTS.md
    let mut s = String::new();
    w!(s, "PREVIEW SEARCH, not a Winner Gate verdict");
    w!(s);
    w!(s, "# PSRB preview search: results (Fri 6 Nov 2026)");
    w!(s);
    w!(s, "{SEARCH_LINE}. Five cores (K1 ST+BB, K2 ST+R1X, and the preview's VIDEO-ORIGINAL, AUTHOR-V2, VIDEO-SCREEN), the 74 fixed filters of SEARCH-SPEC 3 plus the operator's ADDENDUM 3/4 library (candle anatomy, candle-code sequences, named patterns, chart structures, Fibonacci, minimum move), six Supertrend exits (the stop and exit are always the Supertrend, operator-inputs item 19), six timeframes, six sides (LONG, SHORT, BOTH and FADE), the account controls of ADD-RISK and the trade caps, every result net of all costs on Rs 20 lakh with 20 slots of Rs 5 lakh exposure. Chosen on earlier months only, judged on unseen months only. APPROXIMATE items at the end.");
    w!(s);
    w!(s, "The bar: every UNSEEN month >= +2.5% net of its month-start capital (a no-trade month fails); then lowest max drawdown, net profit, entry accuracy, fewer trades, shorter holding.");
    w!(s);
    w!(s, "Run: {} stocks, started {} IST, {} threads; phase A {} s; phase B {} s; merge {} s.", info.stocks, info.started, info.threads, info.phase_a_secs, info.tf_secs.iter().map(|x| x.1).sum::<u64>(), info.merge_secs);
    w!(s);
    w!(s, "Self-check 2.3 (the bare cores re-create the preview's 54 rows, every column): {}.", if info.check23.iter().all(|l| l.contains("EQUAL")) { format!("{} of {} EQUAL", info.check23.len(), info.check23.len()) } else { "SEE REPORT".to_owned() });
    w!(s);
    s.push_str(&body(m, info, lib, lv, false));
    w!(s, "## APPROXIMATE");
    w!(s);
    for l in approximate(info) {
        w!(s, "- {l}");
    }
    let name = if final_ { "SEARCH-RESULTS-FINAL.md" } else { "SEARCH-RESULTS.md" };
    write_text(dir, name, &s)?;
    // REPORT.md (REPORT-FINAL.md after the twins)
    let mut r = head(info, SEARCH_LINE);
    w!(r, "# PSRB preview search REPORT (scratch tool psrb_search, tree {})", info.tree_sha);
    w!(r);
    w!(r, "Command: `{}`", info.command);
    w!(r);
    w!(r, "Self-checks run before phase A:");
    w!(r);
    for c in &info.checks {
        w!(r, "- {c}");
    }
    w!(r);
    w!(r, "Self-check 2.3, the bare cores against the preview's rows.tsv:");
    w!(r);
    for c in &info.check23 {
        w!(r, "- {c}");
    }
    w!(r);
    w!(r, "Membership: {}. Cost table {}.", info.membership_label, info.cost_version);
    w!(r);
    r.push_str(&body(m, info, lib, lv, true));
    w!(r, "## Units (timeframe, core, exit lane): records");
    w!(r);
    for (ti, core, lane, n) in &info.units {
        w!(r, "- {} min {} X{}: {}", TFS.get(*ti).copied().unwrap_or(0), CORE_NAMES.get(*core).copied().unwrap_or("?"), lane + 1, n);
    }
    w!(r);
    w!(r, "## Notes ({} lines; the first 200 shown)", info.notes.len());
    w!(r);
    for n in info.notes.iter().take(200) {
        w!(r, "- {n}");
    }
    write_text(dir, if final_ { "REPORT-FINAL.md" } else { "REPORT.md" }, &r)?;
    if final_ {
        return Ok(());
    }
    // chains.tsv
    let mut t = tsv_head(info);
    w!(t, "scope\tk\tmonth\trecipe\tcapital_start_paisa\ttrades\twins\tgross_paisa\tcharges_paisa\tnet_paisa\tnet_bp\tpass\tgap_bp\tequity_end_paisa");
    for (si, v) in m.chains.iter().enumerate() {
        for (k, j) in v.iter().enumerate() {
            for ml in mlines(j) {
                let year = ml.mi.saturating_sub(16) / 12;
                let rid = j.ids.get(year.min(6)).copied().flatten();
                w!(
                    t,
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                    scope_name(si),
                    k + 1,
                    window_label(ml.mi),
                    rid.map_or_else(|| "-".to_owned(), recipe_label),
                    ml.cap,
                    ml.trades,
                    ml.wins,
                    ml.gross,
                    ml.charges,
                    ml.net,
                    ml.bp.map_or_else(|| "NA".to_owned(), |b| b.to_string()),
                    u8::from(ml.pass),
                    ml.bp.map_or_else(|| "NA".to_owned(), |b| (250 - b).to_string()),
                    ml.eq
                );
            }
        }
    }
    write_text(dir, "chains.tsv", &t)?;
    // picks.tsv
    let mut t = tsv_head(info);
    w!(t, "window\tscope\trank\twith_controls\trecipe\twords\tlearn_months\tlearn_below\tlearn_dd_tenths\tlearn_net_paisa\tlearn_trades\tlearn_win_permille\tlearn_hold_min\tunseen_months\tunseen_below\tunseen_dd_tenths\tunseen_net_paisa\tunseen_trades");
    for wi in 0..N_WIN {
        for si in 0..N_SCOPE {
            for (heap, ctl) in [(m.plain.get(wi * N_SCOPE + si), 0_u8), (m.ctl.get(wi * N_SCOPE + si), 1)] {
                let Some(h) = heap else { continue };
                for (rank, e) in h.v.iter().enumerate() {
                    let k = &e.key;
                    let tt = i64::from(k.trades);
                    let un = if wi < 7 {
                        let (a, b) = unseen(wi);
                        if ctl == 1 {
                            m.judged.get(wi * N_SCOPE + si).and_then(|v| v.get(rank)).map(|j| j.key)
                        } else {
                            Some(crate::acct::key_range(e.row.as_deref().unwrap_or(&crate::acct::ROW0), a, b, k.id))
                        }
                    } else {
                        None
                    };
                    w!(
                        t,
                        "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                        WIN_NAMES.get(wi).copied().unwrap_or("?"),
                        scope_name(si),
                        rank + 1,
                        ctl,
                        recipe_label(k.id),
                        recipe_words(k.id),
                        k.months,
                        k.below,
                        k.dd,
                        k.net,
                        k.trades,
                        if tt > 0 { (1_000 * i64::from(k.wins)).div_euclid(tt) } else { 0 },
                        if tt > 0 { (k.hold / 60).div_euclid(tt) } else { 0 },
                        un.map_or(0, |u| u.months),
                        un.map_or(0, |u| u.below),
                        un.map_or(0, |u| u.dd),
                        un.map_or(0, |u| u.net),
                        un.map_or(0, |u| u.trades)
                    );
                }
            }
        }
    }
    write_text(dir, "picks.tsv", &t)?;
    // recipes.tsv: every recipe in any heap, its 98-month row
    let mut t = tsv_head(info);
    w!(t, "recipe\tmonth\ttrades\twins\tnet_paisa\tgross_paisa\tcharges_paisa\thold_s\tfall_paisa");
    let mut seen = std::collections::BTreeSet::new();
    for h in m.plain.iter().chain(m.ctl.iter()) {
        for e in &h.v {
            if !seen.insert(e.key.id) {
                continue;
            }
            let Some(row) = e.row.as_ref() else { continue };
            for (mi, x) in row.iter().enumerate() {
                if x.trades == 0 {
                    continue;
                }
                w!(t, "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}", recipe_label(e.key.id), window_label(mi), x.trades, x.wins, x.net, x.gross, x.charges, x.hold, x.fall);
            }
        }
    }
    write_text(dir, "recipes.tsv", &t)?;
    // trades-chains.tsv
    let mut t = tsv_head(info);
    w!(t, "chain\trecipe\tstock_rank\tside_traded\tsignal_ist\tentry_ist\tentry_paisa\tinitial_stop_paisa\texit_kind\texit_ist\texit_paisa\tqty\thold_s\tgross_paisa\tcharges_paisa\tnet_paisa\thalf_size\tfallback");
    let mut chain_trades: Vec<(String, &(crate::spool::Record, crate::acct::Took, u64))> = Vec::new();
    for (si, v) in m.chains.iter().enumerate() {
        if let Some(j) = v.first() {
            for x in &j.trades {
                chain_trades.push((format!("{} pick #1", scope_name(si)), x));
            }
        }
    }
    if let Some(j) = m.fixed.first() {
        for x in &j.trades {
            chain_trades.push(("FIXED pick #1".to_owned(), x));
        }
    }
    for (name, (rec, took, id)) in &chain_trades {
        let rr = unpack(*id);
        let which = usize::from(rr.side >= 3);
        let tr = rec.tr.get(which).copied().unwrap_or_default();
        let long = (rec.side == 0) == (which == 0);
        w!(
            t,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            name,
            recipe_label(*id),
            rec.stock,
            if long { "LONG" } else { "SHORT" },
            fmt_ist(rec.t),
            fmt_ist(tr.entry_us),
            tr.entry_px,
            tr.stop0,
            EXIT_NAMES.get(tr.kind.index()).copied().unwrap_or("?"),
            fmt_ist(tr.exit_us),
            tr.exit_px,
            if took.half { crate::common::HALF_EXPOSURE_PAISA.div_euclid(tr.entry_px.max(1)) } else { tr.qty },
            (tr.exit_us - tr.entry_us).div_euclid(1_000_000),
            took.gross,
            took.charges,
            took.net,
            u8::from(took.half),
            u8::from(tr.fallback)
        );
    }
    write_text(dir, "trades-chains.tsv", &t)?;
    // months-<chain>.md
    let mut list: Vec<(String, &Judged)> = Vec::new();
    if let Some(j) = m.chains.first().and_then(|v| v.first()) {
        list.push(("GLOBAL-pick1".to_owned(), j));
    }
    if let Some(j) = m.fixed.first() {
        list.push(("FIXED-pick1".to_owned(), j));
    }
    for ti in 0..6 {
        if let Some(j) = m.chains.get(1 + ti).and_then(|v| v.first()) {
            list.push((format!("TF{}-pick1", TFS.get(ti).copied().unwrap_or(0)), j));
        }
    }
    for (name, j) in &list {
        let mut x = head(info, SEARCH_LINE);
        w!(x, "# {name}: every unseen month");
        w!(x);
        w!(x, "{}", figures(j, true));
        w!(x);
        for id in j.ids.iter().flatten() {
            w!(x, "- {}", recipe_words(*id));
        }
        w!(x);
        months_table(j, &mut x);
        write_text(dir, &format!("months-{name}.md"), &x)?;
    }
    // bare-cores.tsv, singles.tsv, filters.tsv
    let mut t = tsv_head(info);
    w!(t, "{SUMMARY_HEAD}");
    for l in &info.bare {
        w!(t, "{l}");
    }
    write_text(dir, "bare-cores.tsv", &t)?;
    let mut t = tsv_head(info);
    w!(t, "{SUMMARY_HEAD}");
    for l in &info.singles {
        w!(t, "{l}");
    }
    write_text(dir, "singles.tsv", &t)?;
    let mut t = tsv_head(info);
    w!(t, "timeframe_min\tcore\texit\tfilter\tcore_records\ttrue_count\tpermille");
    for l in &info.filters {
        w!(t, "{l}");
    }
    write_text(dir, "filters.tsv", &t)?;
    // luck.tsv
    let mut t = tsv_head(info);
    w!(t, "key\tvalue");
    w!(t, "tried_phase_b_exhaustive\t{}", m.tried_b);
    w!(t, "tried_phase_b_buildup\t{}", m.tried_buildup);
    w!(t, "tried_control_stage\t{}", m.tried_ctl);
    w!(t, "series_offered_to_the_sign_test\t{}", m.luck.offered);
    w!(t, "series_evaluated_after_pruning\t{}", m.luck.evaluated);
    w!(t, "observed_months\t{}", lv.observed);
    w!(t, "draws_reaching_observed\t{}", lv.beaten);
    w!(t, "clears\t{}", u8::from(lv.clears));
    w!(t, "l95\t{}", lv.l95);
    w!(t, "median_best\t{}", lv.median);
    for (b, x) in m.luck.best.iter().enumerate() {
        w!(t, "best_b_{b}\t{x}");
    }
    write_text(dir, "luck.tsv", &t)?;
    // controls.tsv
    let mut t = tsv_head(info);
    w!(t, "window\tscope\tplain_pick\tplain_unseen_net_paisa\tplain_unseen_below\tpick_with_controls\toption_set\tunseen_net_paisa\tunseen_below\tblocked_day_loss\tblocked_month_loss\tblocked_target\tblocked_cap\thalf_trades");
    for wi in 0..7 {
        for si in 0..N_SCOPE {
            let p = m.plain.get(wi * N_SCOPE + si).and_then(|h| h.v.first());
            let c = m.judged.get(wi * N_SCOPE + si).and_then(|v| v.first());
            let (a, b) = unseen(wi);
            let pk = p.map(|e| crate::acct::key_range(e.row.as_deref().unwrap_or(&crate::acct::ROW0), a, b, e.key.id));
            w!(
                t,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                WIN_NAMES.get(wi).copied().unwrap_or("?"),
                scope_name(si),
                p.map_or_else(|| "-".to_owned(), |e| recipe_label(e.key.id)),
                pk.map_or(0, |k| k.net),
                pk.map_or(0, |k| k.below),
                c.and_then(|j| j.ids.first().copied().flatten()).map_or_else(|| "-".to_owned(), recipe_label),
                c.and_then(|j| j.ids.first().copied().flatten()).map_or_else(|| "-".to_owned(), |id| opt_words(unpack(id).opt)),
                c.map_or(0, |j| j.key.net),
                c.map_or(0, |j| j.key.below),
                c.map_or(0, |j| j.counters.blocked[0]),
                c.map_or(0, |j| j.counters.blocked[1]),
                c.map_or(0, |j| j.counters.blocked[2]),
                c.map_or(0, |j| j.counters.blocked[3]),
                c.map_or(0, |j| j.counters.half_trades)
            );
        }
    }
    write_text(dir, "controls.tsv", &t)?;
    // counts.tsv
    let mut t = tsv_head(info);
    let c = &info.counts;
    w!(t, "section\tkey\tvalue");
    for (k, v) in [
        ("zerodha_months_read", c.z_months_read),
        ("zerodha_months_missing_in_span", c.z_months_missing),
        ("zerodha_months_refused", c.z_months_refused),
        ("gdfl_months_missing_in_trading_span", c.g_months_missing),
        ("gdfl_months_refused", c.g_months_refused),
        ("fold_refusals", c.fold_errors),
        ("charter_minutes_withheld", c.charter_minutes_withheld),
        ("m2_days", c.m2_days),
        ("m2_days_no_minutes", c.m2_days_no_minutes),
        ("trade_days", c.trade_days),
        ("member_false_days", c.member_false_days),
        ("member_unknown_days", c.member_unknown_days),
        ("no_prev_session_days", c.no_prev_days),
        ("prev_session_refused_days", c.prev_refused_days),
        ("record_days", c.candidate_days),
        ("fallback_days", c.fallback_days),
    ] {
        w!(t, "stock\t{k}\t{v}");
    }
    for ti in 0..6 {
        let tf = TFS.get(ti).copied().unwrap_or(0);
        let g = |a: &[u64; 6]| a.get(ti).copied().unwrap_or(0);
        w!(t, "candles\t{tf}min_folded\t{}", g(&c.candles));
        w!(t, "candles\t{tf}min_holes\t{}", g(&c.holes));
        w!(t, "candles\t{tf}min_holes_missing_minute\t{}", g(&c.holes_missing_minute));
        w!(t, "candles\t{tf}min_padded_only_buckets\t{}", g(&c.padded_only));
        w!(t, "candles\t{tf}min_decisions\t{}", g(&c.decisions));
        w!(t, "candles\t{tf}min_taint10\t{}", g(&c.taint10));
        w!(t, "candles\t{tf}min_taint200\t{}", g(&c.taint200));
        for lane in 0..6 {
            for core in 0..5 {
                for side in 0..2 {
                    let ix = crate::phase_a::sig_ix(ti, lane, core, side);
                    w!(t, "signals\t{tf}min_X{}_{}_{}\t{}", lane + 1, CORE_NAMES.get(core).copied().unwrap_or("?").split(' ').next().unwrap_or("?"), if side == 0 { "LONG" } else { "SHORT" }, c.signals.get(ix).copied().unwrap_or(0));
                    w!(t, "cut_by_1508\t{tf}min_X{}_{}_{}\t{}", lane + 1, CORE_NAMES.get(core).copied().unwrap_or("?").split(' ').next().unwrap_or("?"), if side == 0 { "LONG" } else { "SHORT" }, c.cut.get(ix).copied().unwrap_or(0));
                }
            }
            let a = c.at_once.get(ti).and_then(|x| x.get(lane));
            let f = c.filled.get(ti).and_then(|x| x.get(lane));
            w!(t, "records\t{tf}min_X{}\t{}", lane + 1, c.records.get(ti).and_then(|x| x.get(lane)).copied().unwrap_or(0));
            w!(t, "filled\t{tf}min_X{}_WITH\t{}", lane + 1, f.and_then(|x| x.first()).copied().unwrap_or(0));
            w!(t, "filled\t{tf}min_X{}_FADE\t{}", lane + 1, f.and_then(|x| x.get(1)).copied().unwrap_or(0));
            w!(t, "stopped_at_once\t{tf}min_X{}_WITH\t{}", lane + 1, a.and_then(|x| x.first()).copied().unwrap_or(0));
            w!(t, "stopped_at_once\t{tf}min_X{}_FADE\t{}", lane + 1, a.and_then(|x| x.get(1)).copied().unwrap_or(0));
        }
    }
    write_text(dir, "counts.tsv", &t)?;
    // identity.txt
    let mut t = tsv_head(info);
    w!(t, "scratch_tree\t/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/psrb-preview-search-1");
    w!(t, "scratch_head\t{}", info.tree_sha);
    w!(t, "tips\tRSI 6d3ae223e5db, BOLL 820f63a7a0e2, STV 7b137fee7014, SEAM-2 deaa0f5c0606, WILLR cb2db489bc9d, CCI 1222c57a7253, B c8f7608b5cbc");
    w!(t, "working_copy_additions\tcrates/indicators/src/trend.rs Ema::scaled(); crates/psrb_preview (T1, sha256 equal); crates/psrb_search; Cargo.toml member lines; Cargo.lock entries");
    w!(t, "cost_table_version\t{}", info.cost_version);
    w!(t, "membership\t{}", info.membership_label);
    w!(t, "membership_digest\t{}", info.membership_digest);
    w!(t, "seeds\tluck 20261106; twins 1, 2, 3; sample 20261106");
    w!(t, "command\t{}", info.command);
    w!(t, "started_ist\t{}", info.started);
    w!(t, "threads\t{}", info.threads);
    for c in &info.checks {
        w!(t, "self_check\t{c}");
    }
    write_text(dir, "identity.txt", &t)?;
    // trades-sample.md: 40 trades of the chosen recipes, fixed seed
    let mut x = head(info, SEARCH_LINE);
    w!(x, "# 40 trades of the chosen recipes (the GLOBAL, TF and FIXED pick #1 chains), picked by a fixed seed");
    w!(x);
    w!(x, "Key = splitmix64 of (seed 20261106, stock, signal instant, recipe); the 40 smallest keys. Prices in rupees; times IST; the signal candle is Zerodha (adjusted), the fills GDFL seconds (raw) unless marked minute fallback. Charges are the headline clock's worst-reading total.");
    w!(x);
    let mut keyed: Vec<(u64, usize)> = chain_trades
        .iter()
        .enumerate()
        .map(|(i, (_, (rec, _, id)))| (crate::common::draw(20_261_106, u64::from(rec.stock), u64::from_ne_bytes(rec.t.to_ne_bytes()), *id), i))
        .collect();
    keyed.sort_unstable();
    keyed.dedup_by_key(|k| k.0);
    w!(x, "| # | chain | recipe | stock rank | side traded | signal close | signal O / H / L / C | entry | entry price | initial stop | exit | exit time | exit price | qty | gross | charges | net |");
    w!(x, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (n, (_, i)) in keyed.iter().take(40).enumerate() {
        let Some((name, (rec, took, id))) = chain_trades.get(*i) else { continue };
        let rr = unpack(*id);
        let which = usize::from(rr.side >= 3);
        let tr = rec.tr.get(which).copied().unwrap_or_default();
        let long = (rec.side == 0) == (which == 0);
        w!(
            x,
            "| {} | {} | {} | {} | {} | {} | {} / {} / {} / {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            n + 1,
            name,
            recipe_label(*id),
            rec.stock,
            if long { "LONG" } else { "SHORT" },
            fmt_ist(rec.t),
            rs64(rec.sig[0]),
            rs64(rec.sig[1]),
            rs64(rec.sig[2]),
            rs64(rec.sig[3]),
            fmt_ist(tr.entry_us),
            rs64(tr.entry_px),
            rs64(tr.stop0),
            EXIT_NAMES.get(tr.kind.index()).copied().unwrap_or("?"),
            fmt_ist(tr.exit_us),
            rs64(tr.exit_px),
            if took.half { crate::common::HALF_EXPOSURE_PAISA.div_euclid(tr.entry_px.max(1)) } else { tr.qty },
            rs64(took.gross),
            rs64(took.charges),
            rs64(took.net)
        );
    }
    w!(x);
    if keyed.is_empty() {
        w!(x, "(the chains took no trade)");
    }
    write_text(dir, "trades-sample.md", &x)?;
    Ok(())
}

/// One twin's summary line (written into twin-S/summary.tsv and read back by the final step).
pub(crate) fn twin_of(seed: u64, m: &MergeOut, secs: u64) -> Twin {
    let Some(h) = m.chains.first().and_then(|v| v.first()) else {
        return Twin { seed, secs, ..Twin::default() };
    };
    let t = i64::from(h.key.trades);
    let hindsight = m.plain.get(7 * N_SCOPE).and_then(|x| x.v.first()).map(|e| e.key).unwrap_or_default();
    Twin {
        seed,
        key: h.key,
        pass: mlines(h).iter().filter(|x| x.pass).count(),
        worst_bp: worst_of(h),
        hold_min: if t > 0 { (h.key.hold / 60).div_euclid(t) } else { 0 },
        hindsight,
        words: h.ids.iter().flatten().map(|id| recipe_label(*id)).collect::<Vec<_>>().join(" | "),
        secs,
    }
}

pub(crate) fn twin_tsv(t: &Twin) -> String {
    let k = &t.key;
    let h = &t.hindsight;
    format!(
        "seed\t{}\nstanding\t{}\nbelow\t{}\nworst_n\t{}\nworst_d\t{}\ndd\t{}\nfall\t{}\nnet\t{}\nwins\t{}\ntrades\t{}\nhold\t{}\ngross\t{}\ncharges\t{}\nmonths\t{}\nid\t{}\npass\t{}\nh_months\t{}\nh_below\t{}\nh_net\t{}\nsecs\t{}\nwords\t{}\n",
        t.seed, k.standing, k.below, k.worst_n, k.worst_d, k.dd, k.fall, k.net, k.wins, k.trades, k.hold, k.gross, k.charges, k.months, k.id, t.pass, h.months, h.below, h.net, t.secs, t.words
    )
}

pub(crate) fn twin_read(text: &str) -> Twin {
    let mut t = Twin::default();
    for line in text.lines() {
        let mut it = line.splitn(2, '\t');
        let (Some(k), Some(v)) = (it.next(), it.next()) else { continue };
        let i = || v.parse::<i128>().unwrap_or(0);
        match k {
            "seed" => t.seed = v.parse().unwrap_or(0),
            "standing" => t.key.standing = v.parse().unwrap_or(1),
            "below" => t.key.below = v.parse().unwrap_or(0),
            "worst_n" => t.key.worst_n = i(),
            "worst_d" => t.key.worst_d = i(),
            "dd" => t.key.dd = v.parse().unwrap_or(0),
            "fall" => t.key.fall = v.parse().unwrap_or(0),
            "net" => t.key.net = v.parse().unwrap_or(0),
            "wins" => t.key.wins = v.parse().unwrap_or(0),
            "trades" => t.key.trades = v.parse().unwrap_or(0),
            "hold" => t.key.hold = v.parse().unwrap_or(0),
            "gross" => t.key.gross = v.parse().unwrap_or(0),
            "charges" => t.key.charges = v.parse().unwrap_or(0),
            "months" => t.key.months = v.parse().unwrap_or(0),
            "id" => t.key.id = v.parse().unwrap_or(0),
            "pass" => t.pass = v.parse().unwrap_or(0),
            "h_months" => t.hindsight.months = v.parse().unwrap_or(0),
            "h_below" => t.hindsight.below = v.parse().unwrap_or(0),
            "h_net" => t.hindsight.net = v.parse().unwrap_or(0),
            "secs" => t.secs = v.parse().unwrap_or(0),
            "words" => t.words = v.to_owned(),
            _ => {}
        }
    }
    if t.key.worst_d > 0 {
        t.worst_bp = Some((10_000 * t.key.worst_n).div_euclid(t.key.worst_d));
    }
    t
}
