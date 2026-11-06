//! The outputs. Line 1 of every file is a PREVIEW line, then the APPROXIMATE list.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

use costs::equity_clock::HeadlineFrom;

use crate::common::{
    ACCOUNT_NAMES, Cand, EXIT_NAMES, N_MONTHS, N_RS, N_TF, PREVIEW_LINE, READING_NAMES, RS_NAMES,
    SIDE_NAMES, TFS, fmt_date, fmt_ist, pct, permille, rupees, thousandths, window_label,
};
use crate::phase_a::price;
use crate::phase_b::{Stats, misses, passes, rank_key};
use crate::{Row, Run};

macro_rules! w {
    ($dst:expr) => {{
        let _ = writeln!($dst);
    }};
    ($dst:expr, $($arg:tt)*) => {{
        let _ = writeln!($dst, $($arg)*);
    }};
}

fn name<'a>(list: &[&'a str], i: usize) -> &'a str {
    list.get(i).copied().unwrap_or("?")
}

fn tf_min(i: usize) -> i64 {
    TFS.get(i).copied().unwrap_or(0)
}

pub(crate) fn row_label(r: &Row) -> String {
    format!("{} {}min {}", name(&RS_NAMES, r.rs), tf_min(r.tf), name(&ACCOUNT_NAMES, r.acct))
}

fn months_file(r: &Row) -> String {
    format!("months-{}-{}min-{}.md", name(&RS_NAMES, r.rs), tf_min(r.tf), name(&ACCOUNT_NAMES, r.acct))
}

fn opt_pct(x: Option<i128>) -> String {
    x.map_or_else(|| "account <= 0".to_owned(), pct)
}

fn pf(x: Option<i128>) -> String {
    x.map_or_else(|| "no loss".to_owned(), thousandths)
}

fn winrate(x: Option<i128>) -> String {
    x.map_or_else(|| "no trades".to_owned(), permille)
}

fn sum_fallback_rows(run: &Run, acct: usize) -> (u64, u64) {
    run.rows
        .iter()
        .filter(|r| r.acct == acct)
        .fold((0, 0), |(a, b), r| (a + r.stats.fallback_trades, b + r.stats.fallback_stock_days))
}

fn approximate(run: &Run) -> Vec<String> {
    let c = &run.counts;
    let (fb_t, fb_d) = sum_fallback_rows(run, 2);
    vec![
        "1. Not the PSRB result. PSRB-STRAT is not built; this is a scratch tool in a scratch tree, never committed. Indicator code is the part tips' (PSRB-RSI 6d3ae223e5db and PSRB-BOLL 820f63a7a0e2 passed check 1; PSRB-STV 7b137fee7014, PSRB-WILLR cb2db489bc9d and PSRB-CCI 1222c57a7253 are still in check 1). The EMA getter scaled() is inserted in the scratch tree only. Readings are the published definitions, UNVERIFIED against TradingView's output.".to_owned(),
        "2. One recipe per rule set: AUTHOR-V2 with CPR-20BP and the LEVEL Williams %R form (long > -85, short < -15); VIDEO-SCREEN is AUTHOR-V2 with Williams %R long > -15 and short < -85, from the author's screen (OPERATOR-ADDENDUM-20261106). No other CPR definition, no CROSS form, no ablation, no VIDEO-ORIGINAL-FIXED-STOP, no top-mover selection.".to_owned(),
        "3. No walk-forward needed: nothing is learned, so every month is unseen. It is not the FNO-WFT run.".to_owned(),
        "4. N4's quote range is not loaded (QRANGE-1 not landed): fills and stop triggers use the traded range only.".to_owned(),
        "5. Fill walk simplified: the whole quantity fills in one second at its worst print (a buy at the second's high, a sell at its low); no volume-share cap, no 3% market protection, no dynamic price band (FNO-BAND), no latency floor (FNO-CLOCK), no MTM 80% square-off. The quantity is sized from the fill price itself.".to_owned(),
        "6. Leverage flat 5x for every stock (FNO-LEVERAGE not used); slots stay Rs 1 lakh margin after losses (margin not checked against equity). Ties by (time, symbol), not SIZE-FIXED-20-SEEDED's seeded order; no luck spread.".to_owned(),
        format!(
            "7. Universe: {}. Survivorship: only today's 218 names can trade; a share that left F&O before 30 Oct 2026 has no stored series. GDFL renamed series are not joined (TATACONSUM trades from its own GDFL months). {} stock-days were not members and {} had membership unknown (neither traded).",
            run.membership_label, c.member_false_days, c.member_unknown_days
        ),
        "8. Price basis: Zerodha candles and bands are corporate-action adjusted, GDFL seconds raw. Stops are mapped by the in-session ratio G/Z of the latest minute that has both (FSPEC 6.6 R1), not the exact factor, rounded to the paisa (a long's floored, a short's ceiled), not to the raw price step.".to_owned(),
        "9. Data parts still being fixed: REFOLD (the stored derived rungs, not read here), REPLACE, ZBASIS-1 (RELIANCE's minutes on another basis for 580 days, VBL, HINDUNILVR: read as stored), SIDX-1; missing months become holes (Zerodha) or minute-fallback days (GDFL).".to_owned(),
        "10. Day mask partial: M1 (the 218), M2 (a regular full session by pull::calendar::Runtime::kind_of), M3 (membership), M4 (window and the stock's span), M5 (Zerodha candles that day), 2021-02-24 never traded, 2019-05-30 a minute-fallback day for every stock. M7's other skip rows, M8 (trade-for-trade) and the FNO-DAYMASK part are not applied.".to_owned(),
        "11. Closing-auction eligibility from 2026-08-03 is inferred from where each stock-day's stored minutes end (last minute 15:14 or earlier: eligible, continuous close 15:15), not read from the dated NSE master.".to_owned(),
        "12. The previous-session close is the last traded minute's close (derive_minutes at Bucket::DAY), not NSE's official closing price.".to_owned(),
        "13. Costs per trade: each trade is its own contract note; the client-day aggregation is not applied (pre-2020-07-01 state stamp capped per trade, DP Rs 14.75 per trade, STT and stamp rounded per trade: each at least the client-day figure). FNO-COSTS's open F3 stands.".to_owned(),
        "14. Drawdown on closed-trade equity, marked after every exit in time order; no mark-to-market inside a trade.".to_owned(),
        "15. Timeframes 1, 30 and 60 minutes go beyond AD's 3/5/15 defaults (SS:348).".to_owned(),
        "16. The stop is live from the traded second after the entry's fill second.".to_owned(),
        "17. (builder) The month loader is this tool's own copy of cli::stored::load's path (the quarantine probe, BarFile::open_existing, the charter's non-regular sessions withheld), not a call into cli (kept out to keep the scratch build small); cli's extra refusal of an uncalendared bar on a withheld day is not repeated.".to_owned(),
        "18. (builder) Built in release (opt-level 3, overflow checks on) with fat LTO off and 16 codegen units (CARGO_PROFILE_RELEASE_* overrides): build time only, same arithmetic.".to_owned(),
        "19. (builder) 'Cut by 15:08' counts signal candles at or after 15:08:00 whatever the stock's position; an account's 'signals' counts the candidates it considered (signal candles before 15:08 on its sides).".to_owned(),
        "20. (builder) A candle with a missing scheduled minute is a hole (spec 3.4): on 60-minute candles one missing minute blocks that hour and taints EMA 200 for 200 candles (about 29 sessions); see the hole and taint counts in REPORT.md.".to_owned(),
        "21. (builder) A flip or exit order that finds no traded second left in the session closes at the worst print since the entry, by the broker, flagged unclosable (the square-off remainder rule applied to it).".to_owned(),
        format!(
            "22. (builder) Minute fallback fills: {} stock-days with candidates were filled from Zerodha minutes ({} on 2019-05-30, {} with the GDFL month missing or refused, {} with no traded GDFL second that day, {} unreadable). In the 18 BOTH rows together: {} trades on {} stock-days used them (per row in REPORT.md and rows.tsv).",
            c.fallback_days, c.fallback_gap_days, c.fallback_no_month_days, c.fallback_empty_days, c.fallback_read_error_days, fb_t, fb_d
        ),
        "23. (builder) The store is live while it is read (other parts may write); no digest of the bytes read is recorded, so a re-run can differ where those parts change data.".to_owned(),
    ]
}

fn head(run: &Run, first: &str) -> String {
    let mut s = String::new();
    w!(s, "{first}");
    w!(s);
    w!(s, "APPROXIMATE (every output of this preview carries this list):");
    w!(s);
    for l in approximate(run) {
        w!(s, "- {l}");
    }
    w!(s);
    s
}

fn tsv_head(run: &Run) -> String {
    let mut s = String::new();
    w!(s, "# {PREVIEW_LINE}");
    for l in approximate(run) {
        w!(s, "# APPROXIMATE {l}");
    }
    s
}

fn ranked(run: &Run) -> Vec<&Row> {
    let mut v: Vec<&Row> = run.rows.iter().collect();
    v.sort_by_key(|r| (rank_key(&r.stats), r.rs, r.tf, r.acct));
    v
}

fn miss_list(s: &Stats) -> String {
    let m = misses(s);
    if m.is_empty() {
        return "none".to_owned();
    }
    m.iter()
        .map(|(wi, bp, gap)| match (bp, gap) {
            (Some(bp), Some(gap)) => format!("{} {} (gap {} pts)", window_label(*wi), pct(*bp), pct(*gap)),
            _ => format!("{} account <= 0", window_label(*wi)),
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn summary_line(r: &Row) -> String {
    let s = &r.stats;
    format!(
        "{}: {} / {} months >= +2.5%, worst month {}, median {}, max drawdown {} ({}), net {} ({}), {} trades, win rate {}, profit factor {}",
        row_label(r),
        s.months_pass,
        N_MONTHS,
        opt_pct(s.worst_bp),
        opt_pct(s.median_bp),
        pct(s.max_dd_bp),
        rupees(s.max_dd_paisa),
        pct(s.total_bp),
        rupees(s.total_net),
        s.trades,
        winrate(s.win_pm),
        pf(s.pf_th)
    )
}

fn rs_table(run: &Run, rs: usize, s: &mut String) {
    w!(s, "| timeframe | side | months >= +2.5% | worst month | median month | max drawdown | total net | trades | win rate | profit factor |");
    w!(s, "|---|---|---|---|---|---|---|---|---|---|");
    for tf in 0..N_TF {
        for acct in 0..3 {
            if let Some(r) = run.rows.iter().find(|r| r.rs == rs && r.tf == tf && r.acct == acct) {
                let st = &r.stats;
                w!(
                    s,
                    "| {} min | {} | {} / {} | {} | {} | {} ({}) | {} ({}) | {} | {} | {} |",
                    tf_min(tf),
                    name(&ACCOUNT_NAMES, acct),
                    st.months_pass,
                    N_MONTHS,
                    opt_pct(st.worst_bp),
                    opt_pct(st.median_bp),
                    pct(st.max_dd_bp),
                    rupees(st.max_dd_paisa),
                    pct(st.total_bp),
                    rupees(st.total_net),
                    st.trades,
                    winrate(st.win_pm),
                    pf(st.pf_th)
                );
            }
        }
    }
    w!(s);
}

fn run_facts(run: &Run, s: &mut String, total_secs: u64) {
    w!(
        s,
        "Run: {} of 218 names, every stock read-only from the store (Zerodha 1-minute candles folded by pull::fold::derive_minutes; GDFL 1-second fills), window 2018-09 to 2026-10 ({} months, every one judged), account Rs 20,00,000, 20 slots of Rs 1,00,000 margin x 5 (Rs 5,00,000 a trade, quantity floor(5,00,000 / fill price)), one position per stock, costs by costs::equity_clock::price_trade (both clocks, worst readings, headline = max; table {}). Started {} IST; {} threads; phase A {} s, phase B {} s, total {} s.",
        run.stocks_run,
        N_MONTHS,
        run.cost_version,
        run.started,
        run.threads,
        run.phase_a.as_secs(),
        run.phase_b.as_secs(),
        total_secs
    );
    w!(s);
}

fn preview_results(run: &Run, total_secs: u64) -> String {
    let mut s = String::new();
    w!(s, "PREVIEW, not a Winner Gate verdict");
    w!(s);
    w!(s, "# PSRB preview: one fixed backtest over the F&O stocks, every timeframe (Fri 6 Nov 2026)");
    w!(s);
    w!(s, "{PREVIEW_LINE}. Three rule sets (VIDEO-ORIGINAL, AUTHOR-V2, and VIDEO-SCREEN from the operator addendum), six timeframes, each side alone and both together: 54 rows. What is approximate is listed at the end (APPROXIMATE).");
    w!(s);
    run_facts(run, &mut s, total_secs);
    let not_run: Vec<&str> = run.notes.iter().filter(|n| n.contains("not run")).map(String::as_str).collect();
    if !not_run.is_empty() {
        w!(s, "Not run ({}): {}.", not_run.len(), not_run.join("; "));
        w!(s);
    }
    w!(s, "The bar: a row passes only if every one of the {N_MONTHS} months nets >= +2.5% of the capital it started with (exact integer test 10,000 x net >= 250 x capital). Rows rank: passers by lowest max drawdown, then net; else fewest months below +2.5%, then the best worst month, then the lowest max drawdown.");
    w!(s);
    w!(s, "## Headline: the best row of each rule set");
    w!(s);
    let order = ranked(run);
    for rs in 0..N_RS {
        if let Some(r) = order.iter().find(|r| r.rs == rs) {
            w!(s, "- {}{}", summary_line(r), if passes(&r.stats) { " (PASSES)" } else { "" });
        }
    }
    w!(s);
    for rs in 0..N_RS {
        w!(s, "## {}", name(&RS_NAMES, rs));
        w!(s);
        rs_table(run, rs, &mut s);
    }
    let passers = order.iter().filter(|r| passes(&r.stats)).count();
    if passers > 0 {
        w!(s, "## The best three rows (pass every month at +2.5% first, then lowest max drawdown, then net profit)");
    } else {
        w!(s, "## No row passes: the three closest (fewest months below +2.5%, then best worst month, then lowest max drawdown)");
    }
    w!(s);
    for (i, r) in order.iter().take(3).enumerate() {
        w!(s, "{}. {}{}", i + 1, summary_line(r), if passes(&r.stats) { " (PASSES)" } else { "" });
        w!(s);
        w!(s, "   Months below +2.5% ({}), with each gap to +2.5%: {}", N_MONTHS - r.stats.months_pass, miss_list(&r.stats));
        w!(s);
    }
    let floor = order
        .iter()
        .filter_map(|r| r.stats.worst_bp.map(|b| (b, *r)))
        .max_by_key(|(b, r)| (*b, std::cmp::Reverse((r.rs, r.tf, r.acct))));
    if let Some((b, r)) = floor {
        w!(s, "Nearest met floor over all {} rows (the highest worst month any row holds): {} by {}.", run.rows.len(), pct(b), row_label(r));
        w!(s);
    }
    w!(s, "## Counts that shape these numbers");
    w!(s);
    let c = &run.counts;
    w!(s, "- Stock-days traded (mask passed): {}; with at least one candidate: {}; minute-fallback days among them: {}.", c.trade_days, c.candidate_days, c.fallback_days);
    w!(s, "- Holes (scheduled candles with a missing minute, withheld or refused), by timeframe 1/3/5/15/30/60 min: {}.", c.holes.iter().map(u64::to_string).collect::<Vec<_>>().join(" / "));
    for rs in 0..N_RS {
        let blocked: Vec<String> = (0..N_TF)
            .map(|tf| c.blocked_any.get(rs).and_then(|x| x.get(tf)).copied().unwrap_or(0).to_string())
            .collect();
        let sig: Vec<String> = (0..N_TF)
            .map(|tf| {
                let v = c.signals.get(rs).and_then(|x| x.get(tf));
                format!("{}+{}", v.and_then(|x| x.first()).copied().unwrap_or(0), v.and_then(|x| x.get(1)).copied().unwrap_or(0))
            })
            .collect();
        w!(s, "- {}: signal candles before 15:08 (long+short) by timeframe {}; decision candles blocked by a hole's taint {}.", name(&RS_NAMES, rs), sig.join(" / "), blocked.join(" / "));
    }
    w!(s);
    w!(s, "Files: run/REPORT.md (all 54 rows ranked, verdicts, gaps, counters), run/months-<rule set>-<tf>min-<side>.md (every month), run/trades-sample.md (40 trades), run/rows.tsv, run/months.tsv, run/counts.tsv, run/identity.txt{}.", if run.trades_tsv { ", run/trades.tsv (every admitted trade)" } else { "" });
    w!(s);
    w!(s, "## APPROXIMATE");
    w!(s);
    for l in approximate(run) {
        w!(s, "- {l}");
    }
    s
}

fn ratio(a: i128, b: i128) -> String {
    if b == 0 {
        "zero base".to_owned()
    } else if b < 0 {
        format!("negative base ({})", (1_000 * a).div_euclid(b))
    } else {
        format!("{}", (1_000 * a).div_euclid(b))
    }
}

#[allow(clippy::too_many_lines, reason = "one report")]
fn report_md(run: &Run, total_secs: u64) -> String {
    let mut s = head(run, PREVIEW_LINE);
    w!(s, "# PSRB preview REPORT (scratch tool psrb_preview, tree {})", run.tree_sha);
    w!(s);
    run_facts(run, &mut s, total_secs);
    w!(s, "Self-checks run before phase A:");
    w!(s);
    for c in &run.checks {
        w!(s, "- {c}");
    }
    w!(s);
    w!(s, "Membership: {}.", run.membership_label);
    w!(s);
    w!(s, "## Head: the rule sets against VIDEO-ORIGINAL, per timeframe and account");
    w!(s);
    w!(s, "| timeframe | account | VO trades | VO net | VO max DD | VO months >= +2.5% | VO worst | A2 trades | A2 net | A2 max DD | A2 months | A2 worst | VS trades | VS net | VS max DD | VS months | VS worst |");
    w!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for tf in 0..N_TF {
        for acct in 0..3 {
            let get = |rs: usize| run.rows.iter().find(|r| r.rs == rs && r.tf == tf && r.acct == acct);
            let cell = |r: Option<&Row>| {
                r.map_or_else(
                    || "| - | - | - | - | - ".to_owned(),
                    |r| {
                        format!(
                            "| {} | {} | {} | {} | {} ",
                            r.stats.trades,
                            pct(r.stats.total_bp),
                            pct(r.stats.max_dd_bp),
                            r.stats.months_pass,
                            opt_pct(r.stats.worst_bp)
                        )
                    },
                )
            };
            w!(s, "| {} min | {} {}{}{}|", tf_min(tf), name(&ACCOUNT_NAMES, acct), cell(get(0)), cell(get(1)), cell(get(2)));
        }
    }
    w!(s);
    w!(s, "Over VIDEO-ORIGINAL in permille (1000 = equal; a zero or negative base printed as such):");
    w!(s);
    w!(s, "| timeframe | account | A2/VO trades | A2/VO net | A2/VO max DD | VS/VO trades | VS/VO net | VS/VO max DD |");
    w!(s, "|---|---|---|---|---|---|---|---|");
    for tf in 0..N_TF {
        for acct in 0..3 {
            let get = |rs: usize| run.rows.iter().find(|r| r.rs == rs && r.tf == tf && r.acct == acct);
            if let (Some(vo), Some(a2), Some(vs)) = (get(0), get(1), get(2)) {
                w!(
                    s,
                    "| {} min | {} | {} | {} | {} | {} | {} | {} |",
                    tf_min(tf),
                    name(&ACCOUNT_NAMES, acct),
                    ratio(i128::from(a2.stats.trades), i128::from(vo.stats.trades)),
                    ratio(a2.stats.total_net, vo.stats.total_net),
                    ratio(a2.stats.max_dd_bp, vo.stats.max_dd_bp),
                    ratio(i128::from(vs.stats.trades), i128::from(vo.stats.trades)),
                    ratio(vs.stats.total_net, vo.stats.total_net),
                    ratio(vs.stats.max_dd_bp, vo.stats.max_dd_bp)
                );
            }
        }
    }
    w!(s);
    let order = ranked(run);
    w!(s, "## All {} rows, ranked (OI:29, :31)", run.rows.len());
    w!(s);
    w!(s, "| rank | row | verdict | months >= +2.5% | months below | worst month (floor held) | median | best | months > 0 | max drawdown | drawdown peak / low | total net | trades | win rate | avg win | avg loss | profit factor | charges | slots full | fallback trades / stock-days |");
    w!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for (i, r) in order.iter().enumerate() {
        let st = &r.stats;
        w!(
            s,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} ({}) | {} / {} | {} ({}) | {} | {} | {} | {} | {} | {} | {} | {} / {} |",
            i + 1,
            row_label(r),
            if passes(st) { "PASS" } else { "MISS" },
            st.months_pass,
            N_MONTHS - st.months_pass,
            opt_pct(st.worst_bp),
            opt_pct(st.median_bp),
            opt_pct(st.best_bp),
            st.months_pos,
            pct(st.max_dd_bp),
            rupees(st.max_dd_paisa),
            fmt_date(st.dd_peak_ts),
            fmt_date(st.dd_low_ts),
            pct(st.total_bp),
            rupees(st.total_net),
            st.trades,
            winrate(st.win_pm),
            st.avg_win.map_or_else(|| "no wins".to_owned(), rupees),
            st.avg_loss.map_or_else(|| "no losses".to_owned(), rupees),
            pf(st.pf_th),
            rupees(st.charges),
            r.out.slots_full,
            st.fallback_trades,
            st.fallback_stock_days
        );
    }
    w!(s);
    let floor = order
        .iter()
        .filter_map(|r| r.stats.worst_bp.map(|b| (b, *r)))
        .max_by_key(|(b, r)| (*b, std::cmp::Reverse((r.rs, r.tf, r.acct))));
    if let Some((b, r)) = floor {
        w!(s, "Nearest met floor over all rows (the highest monthly floor any row holds = its worst month): {} by {}.", pct(b), row_label(r));
        w!(s);
    }
    w!(s, "## Months below +2.5%, every row, with each gap (bp floored, so a gap is never understated)");
    w!(s);
    for r in &order {
        w!(s, "- {}: {}", row_label(r), miss_list(&r.stats));
    }
    w!(s);
    w!(s, "## Counters per row");
    w!(s);
    w!(s, "| row | signals | while open | slots full | entry not filled | zero quantity | no ratio | trades | late fills | stop | flip | time | square-off | unclosable | out of window |");
    w!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for r in &run.rows {
        let o = &r.out;
        let e = &r.stats.exits;
        let ex = |i: usize| e.get(i).copied().unwrap_or(0);
        w!(
            s,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            row_label(r),
            o.signals,
            o.while_open,
            o.slots_full,
            o.not_filled,
            o.zero_qty,
            o.no_ratio,
            r.stats.trades,
            r.stats.late,
            ex(0),
            ex(1),
            ex(2),
            ex(3),
            ex(4),
            r.stats.out_of_window
        );
    }
    w!(s);
    let c = &run.counts;
    w!(s, "## Candles, holes and taint (phase A, summed over stocks)");
    w!(s);
    w!(s, "| timeframe | candles folded | holes | missing minute | withheld | refused | padded-only buckets (no candle, no hole) | unscheduled bars |");
    w!(s, "|---|---|---|---|---|---|---|---|");
    for tf in 0..N_TF {
        let g = |a: &[u64; 6]| a.get(tf).copied().unwrap_or(0);
        w!(s, "| {} min | {} | {} | {} | {} | {} | {} | {} |", tf_min(tf), g(&c.candles), g(&c.holes), g(&c.holes_missing_minute), g(&c.holes_withheld), g(&c.holes_refused), g(&c.padded_only), g(&c.unscheduled_bars));
    }
    w!(s);
    w!(s, "| rule set | timeframe | signals long | signals short | cut by 15:08 long | cut short | decisions blocked by taint | {} |", READING_NAMES.join(" | "));
    w!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for rs in 0..N_RS {
        for tf in 0..N_TF {
            let sg = c.signals.get(rs).and_then(|x| x.get(tf));
            let ct = c.cut.get(rs).and_then(|x| x.get(tf));
            let bl = c.blocked.get(rs).and_then(|x| x.get(tf));
            let by: Vec<String> = (0..6).map(|i| bl.and_then(|x| x.get(i)).copied().unwrap_or(0).to_string()).collect();
            w!(
                s,
                "| {} | {} min | {} | {} | {} | {} | {} | {} |",
                name(&RS_NAMES, rs),
                tf_min(tf),
                sg.and_then(|x| x.first()).copied().unwrap_or(0),
                sg.and_then(|x| x.get(1)).copied().unwrap_or(0),
                ct.and_then(|x| x.first()).copied().unwrap_or(0),
                ct.and_then(|x| x.get(1)).copied().unwrap_or(0),
                c.blocked_any.get(rs).and_then(|x| x.get(tf)).copied().unwrap_or(0),
                by.join(" | ")
            );
        }
    }
    w!(s);
    w!(s, "## Stock-level counts");
    w!(s);
    w!(s, "- Zerodha months read {}, missing in the candle span {}, refused {}; GDFL months missing in the trading spans {}, refused {}; fold refusals {}; charter non-regular minutes withheld {}.", c.z_months_read, c.z_months_missing, c.z_months_refused, c.g_months_missing, c.g_months_refused, c.fold_errors, c.charter_minutes_withheld);
    w!(s, "- Regular full sessions (M2) folded {}, of them with no stored minute {}; stock-days that passed the mask {}; not members {}; membership unknown {}; no previous session {}; previous session refused {}.", c.m2_days, c.m2_days_no_minutes, c.trade_days, c.member_false_days, c.member_unknown_days, c.no_prev_days, c.prev_refused_days);
    w!(s, "- Stock-days with candidates {}; minute-fallback days {} (2019-05-30 {}, GDFL month missing or refused {}, no traded second {}, unreadable {}).", c.candidate_days, c.fallback_days, c.fallback_gap_days, c.fallback_no_month_days, c.fallback_empty_days, c.fallback_read_error_days);
    w!(s);
    w!(s, "## Each stock's span");
    w!(s);
    for sp in &run.spans {
        if !sp.is_empty() {
            w!(s, "- {sp}");
        }
    }
    w!(s);
    w!(s, "## Notes ({} lines; the first 400 shown)", run.notes.len());
    w!(s);
    for n in run.notes.iter().take(400) {
        w!(s, "- {n}");
    }
    s
}

fn rows_tsv(run: &Run) -> String {
    let mut s = tsv_head(run);
    w!(s, "rule_set\ttimeframe_min\taccount\tverdict\tmonths_pass\tmonths_below\tmonths_pos\tworst_bp\tmedian_bp\tbest_bp\tmax_dd_bp\tmax_dd_paisa\tdd_peak\tdd_low\ttotal_net_paisa\ttotal_bp\ttrades\twins\tlosses\twin_permille\tavg_win_paisa\tavg_loss_paisa\tprofit_factor_thousandths\tcharges_paisa\tgross_paisa\tsignals\twhile_open\tslots_full\tentry_not_filled\tzero_qty\tno_ratio\tlate\texit_stop\texit_flip\texit_time\texit_squareoff\texit_unclosable\tfallback_trades\tfallback_stock_days");
    let o = |x: Option<i128>| x.map_or_else(|| "NA".to_owned(), |v| v.to_string());
    for r in &run.rows {
        let st = &r.stats;
        let e = |i: usize| st.exits.get(i).copied().unwrap_or(0);
        w!(
            s,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            name(&RS_NAMES, r.rs),
            tf_min(r.tf),
            name(&ACCOUNT_NAMES, r.acct),
            if passes(st) { "PASS" } else { "MISS" },
            st.months_pass,
            N_MONTHS - st.months_pass,
            st.months_pos,
            o(st.worst_bp),
            o(st.median_bp),
            o(st.best_bp),
            st.max_dd_bp,
            st.max_dd_paisa,
            fmt_date(st.dd_peak_ts),
            fmt_date(st.dd_low_ts),
            st.total_net,
            st.total_bp,
            st.trades,
            st.wins,
            st.losses,
            o(st.win_pm),
            o(st.avg_win),
            o(st.avg_loss),
            o(st.pf_th),
            st.charges,
            st.gross,
            r.out.signals,
            r.out.while_open,
            r.out.slots_full,
            r.out.not_filled,
            r.out.zero_qty,
            r.out.no_ratio,
            st.late,
            e(0),
            e(1),
            e(2),
            e(3),
            e(4),
            st.fallback_trades,
            st.fallback_stock_days
        );
    }
    s
}

fn months_tsv(run: &Run) -> String {
    let mut s = tsv_head(run);
    w!(s, "rule_set\ttimeframe_min\taccount\tmonth\tcapital_start_paisa\tnet_paisa\tnet_bp\tpass\ttrades\twins\tcharges_paisa\tequity_end_paisa");
    for r in &run.rows {
        for (wi, m) in r.stats.months.iter().enumerate() {
            w!(
                s,
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                name(&RS_NAMES, r.rs),
                tf_min(r.tf),
                name(&ACCOUNT_NAMES, r.acct),
                window_label(wi),
                m.cap_start,
                m.net,
                m.bp.map_or_else(|| "NA".to_owned(), |b| b.to_string()),
                if m.pass { 1 } else { 0 },
                m.trades,
                m.wins,
                m.charges,
                m.eq_end
            );
        }
    }
    s
}

fn counts_tsv(run: &Run) -> String {
    let mut s = tsv_head(run);
    let c = &run.counts;
    w!(s, "section\tkey\tvalue");
    let kv = [
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
        ("candidate_days", c.candidate_days),
        ("fallback_days", c.fallback_days),
        ("fallback_days_2019_05_30", c.fallback_gap_days),
        ("fallback_days_gdfl_month_missing_or_refused", c.fallback_no_month_days),
        ("fallback_days_no_traded_second", c.fallback_empty_days),
        ("fallback_days_unreadable", c.fallback_read_error_days),
    ];
    for (k, v) in kv {
        w!(s, "stock\t{k}\t{v}");
    }
    for tf in 0..N_TF {
        let g = |a: &[u64; 6]| a.get(tf).copied().unwrap_or(0);
        let t = tf_min(tf);
        w!(s, "candles\t{t}min_folded\t{}", g(&c.candles));
        w!(s, "candles\t{t}min_holes\t{}", g(&c.holes));
        w!(s, "candles\t{t}min_holes_missing_minute\t{}", g(&c.holes_missing_minute));
        w!(s, "candles\t{t}min_holes_withheld\t{}", g(&c.holes_withheld));
        w!(s, "candles\t{t}min_holes_refused\t{}", g(&c.holes_refused));
        w!(s, "candles\t{t}min_padded_only_buckets\t{}", g(&c.padded_only));
        w!(s, "candles\t{t}min_unscheduled_bars\t{}", g(&c.unscheduled_bars));
    }
    for rs in 0..N_RS {
        for tf in 0..N_TF {
            let key = format!("{}_{}min", name(&RS_NAMES, rs), tf_min(tf));
            for side in 0..2 {
                let v = c.signals.get(rs).and_then(|x| x.get(tf)).and_then(|x| x.get(side)).copied().unwrap_or(0);
                let cut = c.cut.get(rs).and_then(|x| x.get(tf)).and_then(|x| x.get(side)).copied().unwrap_or(0);
                w!(s, "signals\t{key}_{}\t{v}", name(&SIDE_NAMES, side));
                w!(s, "cut_by_1508\t{key}_{}\t{cut}", name(&SIDE_NAMES, side));
            }
            w!(s, "blocked\t{key}_any\t{}", c.blocked_any.get(rs).and_then(|x| x.get(tf)).copied().unwrap_or(0));
            for (ri, rn) in READING_NAMES.iter().enumerate() {
                w!(s, "blocked\t{key}_{rn}\t{}", c.blocked.get(rs).and_then(|x| x.get(tf)).and_then(|x| x.get(ri)).copied().unwrap_or(0));
            }
        }
    }
    for r in &run.rows {
        let l = row_label(r).replace(' ', "_");
        let o = &r.out;
        w!(s, "row\t{l}_signals\t{}", o.signals);
        w!(s, "row\t{l}_while_open\t{}", o.while_open);
        w!(s, "row\t{l}_slots_full\t{}", o.slots_full);
        w!(s, "row\t{l}_entry_not_filled\t{}", o.not_filled);
        w!(s, "row\t{l}_zero_quantity\t{}", o.zero_qty);
        w!(s, "row\t{l}_no_ratio\t{}", o.no_ratio);
        w!(s, "row\t{l}_late_fills\t{}", r.stats.late);
        for (i, en) in EXIT_NAMES.iter().enumerate() {
            w!(s, "row\t{l}_exit_{en}\t{}", r.stats.exits.get(i).copied().unwrap_or(0));
        }
        w!(s, "row\t{l}_fallback_trades\t{}", r.stats.fallback_trades);
        w!(s, "row\t{l}_fallback_stock_days\t{}", r.stats.fallback_stock_days);
    }
    s
}

fn identity(run: &Run) -> String {
    let mut s = tsv_head(run);
    w!(s, "scratch_tree\t/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/psrb-preview-1");
    w!(s, "scratch_head\t{}", run.tree_sha);
    w!(s, "base_costs_B\tc8f7608b5cbc5383f294728e4eeb0d1208a31e97");
    w!(s, "seam\t35234cead4009299a93fcd79a3d1fffd773dcc16");
    w!(s, "seam2\tdeaa0f5c0606");
    w!(s, "tip_rsi\t6d3ae223e5db");
    w!(s, "tip_boll\t820f63a7a0e2");
    w!(s, "tip_stv\t7b137fee7014");
    w!(s, "tip_willr\tcb2db489bc9d");
    w!(s, "tip_cci\t1222c57a7253");
    w!(s, "working_copy_additions\tcrates/indicators/src/trend.rs Ema::scaled(); crates/psrb_preview; Cargo.toml member line");
    w!(s, "cost_table_version\t{}", run.cost_version);
    w!(s, "membership\t{}", run.membership_label);
    w!(s, "membership_digest\t{}", run.membership_digest);
    w!(s, "command\t{}", run.command);
    w!(s, "started_ist\t{}", run.started);
    w!(s, "threads\t{}", run.threads);
    for c in &run.checks {
        w!(s, "self_check\t{c}");
    }
    s
}

fn months_md(run: &Run, r: &Row) -> String {
    let mut s = head(run, PREVIEW_LINE);
    w!(s, "# {} : every month", row_label(r));
    w!(s);
    w!(s, "{}{}", summary_line(r), if passes(&r.stats) { " (PASSES)" } else { "" });
    w!(s);
    w!(s, "Net % = month net / capital at the month's start (floored bp); >= +2.5% is the exact test 10,000 x net >= 250 x capital. Trades are attributed to the month of their exit.");
    w!(s);
    w!(s, "| month | capital at start | net | net % | >= +2.5% | trades | wins | charges | equity at month end |");
    w!(s, "|---|---|---|---|---|---|---|---|---|");
    for (wi, m) in r.stats.months.iter().enumerate() {
        w!(
            s,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            window_label(wi),
            rupees(m.cap_start),
            rupees(m.net),
            opt_pct(m.bp),
            if m.pass { "yes" } else { "no" },
            m.trades,
            m.wins,
            rupees(m.charges),
            rupees(m.eq_end)
        );
    }
    s
}

fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

const SAMPLE_SEED: u64 = 20_261_106;

fn sample_key(c: &Cand) -> u64 {
    let id = (u64::from(c.stock) << 32) | (u64::from(c.rs) << 16) | (u64::from(c.tf) << 8) | u64::from(c.side);
    let t = u64::from_ne_bytes(c.t_signal.to_ne_bytes());
    splitmix(SAMPLE_SEED ^ splitmix(id) ^ splitmix(t ^ 0x5151_5151))
}

fn sample_md(run: &Run) -> Result<String, String> {
    let mut s = head(run, PREVIEW_LINE);
    w!(s, "# 40 trades picked by a fixed seed across stocks, rule sets and timeframes");
    w!(s);
    w!(s, "Pool: every trade admitted by any of the 54 accounts (a trade taken by two accounts counted once). Key = splitmix64 of (seed {SAMPLE_SEED}, stock, rule set, timeframe, side, signal instant); the two smallest keys of each (rule set, timeframe), then the smallest keys left, to 40. Prices in rupees; times IST. The signal candle is Zerodha (adjusted); the fills are GDFL seconds (raw) unless marked minute fallback. Charges are the headline clock's worst-reading lines (costs::equity_clock::price_trade).");
    w!(s);
    let mut pool: BTreeSet<(usize, usize)> = BTreeSet::new();
    for r in &run.rows {
        let g = r.rs * N_TF + r.tf;
        for &i in &r.out.trades {
            pool.insert((g, i));
        }
    }
    let mut keyed: Vec<(u64, usize, usize)> = pool
        .iter()
        .filter_map(|&(g, i)| run.groups.get(g).and_then(|v| v.get(i)).map(|c| (sample_key(c), g, i)))
        .collect();
    keyed.sort_unstable();
    let mut picked: Vec<(usize, usize)> = Vec::new();
    for g in 0..N_RS * N_TF {
        for &(_, gg, i) in keyed.iter().filter(|(_, gg, _)| *gg == g).take(2) {
            picked.push((gg, i));
        }
    }
    for &(_, g, i) in &keyed {
        if picked.len() >= 40 {
            break;
        }
        if !picked.contains(&(g, i)) {
            picked.push((g, i));
        }
    }
    picked.truncate(40);
    picked.sort_by_key(|&(g, i)| (g, run.groups.get(g).and_then(|v| v.get(i)).map_or(0, |c| c.t_signal)));
    w!(s, "| # | stock | rule set | tf | side | signal candle close (IST) | signal O / H / L / C | entry second | entry price | initial stop | exit kind | exit second | exit price | qty | gross | charges | net | fill basis |");
    w!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    let mut charges_rows: Vec<String> = Vec::new();
    for (n, &(g, i)) in picked.iter().enumerate() {
        let Some(c) = run.groups.get(g).and_then(|v| v.get(i)) else { continue };
        let stock = run.syms.iter().find(|x| x.rank == c.stock).map_or("?", |x| x.name.as_str());
        let r2 = |p: i64| rupees(i128::from(p));
        w!(
            s,
            "| {} | {} | {} | {} min | {} | {} | {} / {} / {} / {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            n + 1,
            stock,
            name(&RS_NAMES, usize::from(c.rs)),
            tf_min(usize::from(c.tf)),
            name(&SIDE_NAMES, usize::from(c.side)),
            fmt_ist(c.t_signal),
            r2(c.sig[0]),
            r2(c.sig[1]),
            r2(c.sig[2]),
            r2(c.sig[3]),
            fmt_ist(c.entry_us),
            r2(c.entry_px),
            r2(c.stop0),
            name(&EXIT_NAMES, c.exit_kind.index()),
            fmt_ist(c.exit_us),
            r2(c.exit_px),
            c.qty,
            r2(c.gross),
            r2(c.charges),
            r2(c.net),
            if c.fallback { "minute fallback (Zerodha)" } else { "GDFL second" }
        );
        let p = price(c)?;
        let (clock, lines) = match p.headline_from() {
            HeadlineFrom::Today => ("today", p.today_worst()),
            HeadlineFrom::Dated => ("dated", p.dated_worst()),
            HeadlineFrom::Both => ("both equal", p.dated_worst()),
        };
        charges_rows.push(format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            n + 1,
            clock,
            rupees(i128::from(lines.brokerage.raw())),
            rupees(i128::from(lines.exchange.raw())),
            rupees(i128::from(lines.sebi.raw())),
            rupees(i128::from(lines.stt.raw())),
            rupees(i128::from(lines.stamp.raw())),
            rupees(i128::from(lines.gst.raw())),
            rupees(i128::from(lines.dp.raw())),
            rupees(i128::from(lines.dp_gst.raw())),
            rupees(i128::from(lines.square_off.raw())),
            rupees(i128::from(lines.square_off_gst.raw())),
            rupees(i128::from(lines.total.raw())),
            format!("dated worst {} / today worst {}", rupees(i128::from(p.dated_worst().total.raw())), rupees(i128::from(p.today_worst().total.raw())))
        ));
    }
    w!(s);
    w!(s, "## Every charge of each sampled trade (headline clock, worst readings)");
    w!(s);
    w!(s, "| # | headline clock | brokerage | exchange | SEBI | STT | stamp | GST | DP | DP GST | square-off | square-off GST | total | both clocks |");
    w!(s, "|---|---|---|---|---|---|---|---|---|---|---|---|---|---|");
    for l in charges_rows {
        w!(s, "{l}");
    }
    Ok(s)
}

fn write_text(dir: &Path, file: &str, body: &str) -> Result<(), String> {
    let p = dir.join(file);
    std::fs::write(&p, body).map_err(|e| format!("{}: {e}", p.display()))
}

fn trades_tsv(run: &Run, dir: &Path) -> Result<u64, String> {
    let p = dir.join("trades.tsv");
    let f = std::fs::File::create(&p).map_err(|e| format!("{}: {e}", p.display()))?;
    let mut b = std::io::BufWriter::with_capacity(1 << 20, f);
    let err = |e: std::io::Error| format!("{}: {e}", p.display());
    b.write_all(tsv_head(run).as_bytes()).map_err(err)?;
    b.write_all(b"row\tstock\tside\tsignal_ist\tsignal_open\tsignal_high\tsignal_low\tsignal_close\tentry_ist\tentry_paisa\tinitial_stop_paisa\texit_kind\texit_ist\texit_paisa\tqty\tgross_paisa\tcharges_paisa\tnet_paisa\tfallback\tlate\n").map_err(err)?;
    let mut n = 0_u64;
    for r in &run.rows {
        let label = row_label(r);
        let Some(g) = run.groups.get(r.rs * N_TF + r.tf) else { continue };
        for &i in &r.out.trades {
            let Some(c) = g.get(i) else { continue };
            let stock = run.syms.iter().find(|x| x.rank == c.stock).map_or("?", |x| x.name.as_str());
            writeln!(
                b,
                "{label}\t{stock}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                name(&SIDE_NAMES, usize::from(c.side)),
                fmt_ist(c.t_signal),
                c.sig[0],
                c.sig[1],
                c.sig[2],
                c.sig[3],
                fmt_ist(c.entry_us),
                c.entry_px,
                c.stop0,
                name(&EXIT_NAMES, c.exit_kind.index()),
                fmt_ist(c.exit_us),
                c.exit_px,
                c.qty,
                c.gross,
                c.charges,
                c.net,
                u8::from(c.fallback),
                u8::from(c.late)
            )
            .map_err(err)?;
            n += 1;
        }
    }
    b.flush().map_err(err)?;
    Ok(n)
}

pub(crate) fn write_all(run: &Run, dir: &Path, t0: Instant) -> Result<(), String> {
    let mut total_secs = t0.elapsed().as_secs();
    if run.trades_tsv {
        let n = trades_tsv(run, dir)?;
        eprintln!("psrb_preview: trades.tsv {n} lines");
        total_secs = t0.elapsed().as_secs();
    }
    for r in &run.rows {
        write_text(dir, &months_file(r), &months_md(run, r))?;
    }
    write_text(dir, "trades-sample.md", &sample_md(run)?)?;
    write_text(dir, "rows.tsv", &rows_tsv(run))?;
    write_text(dir, "months.tsv", &months_tsv(run))?;
    write_text(dir, "counts.tsv", &counts_tsv(run))?;
    write_text(dir, "identity.txt", &identity(run))?;
    write_text(dir, "REPORT.md", &report_md(run, total_secs))?;
    write_text(dir, "PREVIEW-RESULTS.md", &preview_results(run, total_secs))?;
    Ok(())
}
