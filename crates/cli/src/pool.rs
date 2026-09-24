//! UNVERIFIED performance: no named cost test or measured latency bound is established here.
//! **One combination across the whole stored surface: which stock, before it
//! moves.**
//!
//! # The question this answers, and the one it does not
//!
//! `range-rung` asks *what combination works on THIS instrument*. The objective
//! D-0506 widened the surface for asks something else: among the 213 F&O
//! underlyings, on most days ONE of them trends one way from the open and
//! never looks back. A combination that names that stock BEFORE it moves fires
//! rarely on any one instrument, loses little when wrong — the stop is tight —
//! and wins enormously when right. Its win rate is irrelevant; its tail is the
//! whole result. No single-instrument sweep can find it, because on any one
//! stock it is a handful of trades a year and the sweep's support floor prunes
//! it before it is ranked.
//!
//! So this verb runs in TWO PASSES and reports TWO TABLES, and the operator
//! reads both because either alone can mislead:
//!
//! 1. **PER SYMBOL.** Every instrument the store holds on this rung and this
//!    feed that is on the engine surface — the two indices and every F&O cash
//!    equity present — is screened exactly as `range-rung` screens one, in
//!    parallel, one run identity each. The rows are the best combination on
//!    each stock ALONE. A stock whose own best row already has a tiny worst
//!    trade and a huge best one is a candidate on its own, and that table says
//!    which.
//! 2. **POOLED.** The union of every instrument's top combinations — each
//!    with the side it was priced at — is priced on EVERY instrument, and the
//!    trades are pooled across the surface. A combination that fires three
//!    times a year on each of 200 stocks is 600 trades pooled, which is enough
//!    to rank, where the same combination on one stock was three. That is the
//!    cross-sectional selection the objective names.
//!
//! # What is pooled, and what is only bounded
//!
//! Trades, wins, the pessimistic total, gross wins and gross losses ADD across
//! instruments. The worst trade and the smallest win are the MIN across
//! instruments. The drawdown does not add and cannot be pooled from cell
//! aggregates: a pooled drawdown is a property of the merged, time-ordered
//! trade sequence, and the exit grid keeps per-cell totals, not per-trade
//! P&L. The `dd≥` column is therefore the LARGEST single-instrument drawdown
//! among those pooled — a lower bound on the pooled figure, labelled as one.
//! Exposing per-trade P&L from the grid is the change that would make it
//! exact, and `docs/06-limits.md` records it as not done.
//!
//! # What is NOT charged, and why that is stated on every report
//!
//! No cost of any kind. On the indices that is correct by charter — an index
//! is not tradeable. On a cash equity it is NOT correct: brokerage, STT, stamp
//! duty, exchange charges, the SEBI fee and GST are real, and no rate for any of
//! them is quoted here because `docs/00-charter.md` records no source for an
//! equity charge (`CLAUDE.md` §3 rule 1; D-0681). The operator
//! asked for this pass without costs so that the rare tail is visible before
//! anything is subtracted from it, and D-0506 records the cost model as
//! necessary before any stock result is acted on. Every report this verb
//! renders says so beside its totals rather than in a footnote.
//!
//! # No validation, in sample, and said so
//!
//! Every figure is in sample and the pooled table is the largest of
//! `instruments × candidates` comparisons. The report carries the same
//! `IN SAMPLE` warning the single-instrument screen carries. This is the
//! search for a candidate, not the proof of one; `range-rung` with the
//! validation stack on is the proof, per instrument, afterwards.
//!
//! # Persistence
//!
//! Pass 1 persists exactly what `range-rung` persists: one ledger row and one
//! frontier block per instrument, under that instrument's own nine-term
//! identity. The POOLED table is rendered and is NOT written to the store. A
//! pooled row has no instrument, and `CLAUDE.md` §3 rule 3 identifies a run by
//! one; inventing a name for the pool is the fabrication §3 rule 1 forbids
//! and is the exact defect `batch` refused for the same reason. A pool ledger
//! with an identity of its own — a digest over the member identities — is a
//! new store format and the subject of a later entry, not something to fake
//! here with a placeholder symbol. D-0509.
//!
//! # Cost
//!
//! Pass 1 is `instruments` screens in parallel, each what `range-rung` costs.
//! Pass 2 is `instruments × |union|` grid evaluations, each the cost of one
//! exit grid over that instrument's trades for that mask. Neither is a rule-4
//! operation: those bound the per-bar and per-candidate primitives INSIDE the
//! screen, which are unchanged. The union itself is one `HashSet` insert per
//! frontier row — O(1) expected — and the pooled fold is one pass over
//! `instruments × |union|` cells with O(1) work each. Nothing here scans the
//! store per candidate, and the catalog walk that lists the surface is paid
//! once.
//!
//! # Parallelism and reproducibility
//!
//! Both passes are `par_iter` over INSTRUMENTS with indexed `collect`, so the
//! order of every table is the sorted symbol order and never the scheduler's.
//! Pass 1's per-instrument runs are the same runs `range-rung` makes, so their
//! identities and rows are byte-identical to running each by hand. The pooled
//! fold runs sequentially over the collected cells. §3 rule 5.

use core::fmt::Write as _;
use std::collections::{BTreeMap, BTreeSet, HashSet};

use rayon::prelude::*;
use runner::grid;
use store::catalog;

use crate::frontier::{Direction, Frontier};
use crate::stored;

/// The multiple the smallest win must clear over the largest loss for a pooled
/// row to be marked as meeting the operator's tail rule.
///
/// Read from the operator's `Rules` rather than written here: `min_rr_bp` is
/// the reward-to-risk floor every single-instrument cell is admitted against,
/// and a pooled row is held to the same number so the two tables agree on what
/// "the rule" is. Zero — the floor OFF — marks every fired row as meeting it,
/// which is what OFF means.
fn tail_rule_bp(rules: crate::Rules) -> i64 {
    rules.min_rr_bp
}

/// One instrument's pass-1 outcome: the run `range-rung` would have made.
struct Screened {
    symbol: String,
    outcome: Result<crate::results::Record, String>,
}

/// One `(combination, side)` the union holds, in first-seen order.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Candidate {
    words: [u64; 6],
    direction: Direction,
}

/// What one candidate did on one instrument: the cell the screen would have
/// shown for it, or nothing when it never fired there.
type Priced = Option<grid::Cell>;

/// One candidate's figures pooled across every instrument it fired on.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Pooled {
    candidate: usize,
    fired: u64,
    trades: u64,
    wins: u64,
    net: i64,
    worst: i64,
    min_win: i64,
    gross_win: i64,
    gross_loss: i64,
    /// The largest single-instrument drawdown among those pooled. A LOWER
    /// BOUND on the pooled drawdown — see the module documentation.
    dd_bound: i64,
    /// Up to the first few symbols it fired on, for the row.
    names: Vec<String>,
}

impl Pooled {
    /// Gross wins over gross losses, in hundredths. [`i64::MAX`] when nothing
    /// was lost, which is a fact and is demoted by [`crate::ranked`] exactly as
    /// a cell's is.
    const fn profit_factor_bp(&self) -> i64 {
        if self.gross_loss == 0 {
            return i64::MAX;
        }
        // `gross_loss` is negative or zero; the magnitude is what divides.
        // HUNDREDTHS, as `grid::Cell::profit_factor_bp` is: 125 is 1.25x.
        self.gross_win.saturating_mul(100) / self.gross_loss.saturating_abs()
    }

    /// The smallest win over the largest loss, in hundredths — the operator's
    /// own rule, `min(win) >= k × max(loss)`, as a ratio. [`i64::MAX`] when
    /// nothing was lost.
    const fn tail_bp(&self) -> i64 {
        if self.worst == 0 {
            return i64::MAX;
        }
        // HUNDREDTHS, as `grid::Cell::reward_to_risk_bp` is, so it compares
        // directly with `Rules::min_rr_bp`.
        self.min_win.saturating_mul(100) / self.worst.saturating_abs()
    }

    /// Whether the tail rule holds at the operator's multiple.
    const fn meets(&self, rule_bp: i64) -> bool {
        self.fired > 0 && self.tail_bp() >= rule_bp
    }

    /// The sort key, largest first: the rule met, then the SMALLEST drawdown
    /// bound, then the profit factor with its never-lost sentinel demoted, then
    /// the net. The drawdown leads because the objective is "very very less max
    /// drawdown" before it is anything else.
    fn key(&self, rule_bp: i64) -> (bool, i64, i64, i64) {
        (
            self.meets(rule_bp),
            self.dd_bound.saturating_neg(),
            crate::ranked(self.profit_factor_bp()),
            self.net,
        )
    }
}

/// The verb, with every refusal rendered the way the CLI prints one.
#[must_use]
pub fn pool(
    vendor_word: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> String {
    match run(vendor_word, rung, from, to, support_ppm) {
        Ok(text) => text,
        Err(why) => format!("refused: {why}\n"),
    }
}

fn run(
    vendor_word: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> Result<String, String> {
    // THE COMMIT IS CHECKED FIRST, BEFORE ANY BAR IS READ. Pass 1 records one
    // identity per instrument and each needs the stamp.
    crate::commit_stamp().ok_or_else(|| {
        "this build carries no verified commit stamp, so §3 rule 3's run identity \
         cannot be recorded and no instrument will be screened. Restore every \
         Rust/Cargo input to HEAD (normally by committing the intended change), \
         then rebuild. An explicit BRUTEX_COMMIT is accepted only when it exactly \
         equals clean HEAD"
            .to_owned()
    })?;
    let vendor = crate::parse_vendor(vendor_word)?;
    crate::swept_rung(rung)?;
    let root = crate::store_root()?;
    run_under(&root, vendor, vendor_word, rung, from, to, support_ppm)
}

/// The page, from its head to its last pooled row, with the store root
/// supplied rather than read from the environment. D-0696.
///
/// [`run`] checks the commit stamp, the feed, the rung and the root, and then
/// hands everything else here. The head [`head_under`] renders is the page's
/// first bytes, and `out` is bound from it once and only appended to after:
/// `the_pool_page_is_its_head_and_then_only_appends` drives this on a scratch
/// store and reads that shape from the source. [`run`] itself stays out of
/// reach of a test for the reason it always was -- the stamp check comes
/// first -- and holds no line of the page.
///
/// **The root is supplied for the head, the union and pass 2, not pass 1.**
/// Pass 1 screens each instrument through `crate::one_rung`, exactly as
/// `range-rung` does, and that reads the store root from the environment. A
/// test therefore drives this function on a surface that is empty, where it
/// returns after the head; a surface with an instrument on it is reached from
/// here by no test.
fn run_under(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    vendor_word: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> Result<String, String> {
    let (mut out, surface) = head_under(root, vendor_word, rung, from, to, support_ppm)?;
    if surface.is_empty() {
        return Ok(out);
    }
    crate::note(
        &telemetry::Event::info(
            "cli.pool",
            "pass 1: screening every instrument on the surface",
        )
        .with("feed", vendor_word)
        .with("rung", rung)
        .with("instruments", count(surface.len())),
    );

    // ── PASS 1: every instrument, exactly as `range-rung` screens one ──
    let screened: Vec<Screened> = surface
        .par_iter()
        .map(|symbol| Screened {
            symbol: symbol.clone(),
            outcome: crate::one_rung(vendor_word, symbol, rung, from, to, support_ppm, None)
                .outcome,
        })
        .collect();
    let screened_ok = screened.iter().filter(|s| s.outcome.is_ok()).count();
    crate::note(
        &telemetry::Event::info("cli.pool", "pass 1 finished")
            .with("screened", count(screened_ok))
            .with("refused", count(screened.len().saturating_sub(screened_ok))),
    );
    render_per_symbol(&mut out, &screened);

    // ── PASS 2: the union of every top combination, on every instrument ──
    let (union, unread) = union_of(root, &screened);
    if !unread.is_empty() {
        let _ = writeln!(
            out,
            "\n  FRONTIER ROWS NOT READ for {} instrument(s); their combinations did not \
             enter the union and the pooled table is over the rest:",
            unread.len()
        );
        for (symbol, why) in &unread {
            let _ = writeln!(out, "    {symbol}: {why}");
        }
    }
    if union.is_empty() {
        let _ = writeln!(
            out,
            "\n  POOLED: nothing to pool. No screened instrument left a frontier row, so \
             the union of top combinations is empty."
        );
        return Ok(out);
    }
    crate::note(
        &telemetry::Event::info("cli.pool", "pass 2: pricing the union on every instrument")
            .with("candidates", count(union.len()))
            .with("instruments", count(surface.len())),
    );
    let priced: Vec<Result<Vec<Priced>, String>> = surface
        .par_iter()
        .map(|symbol| price_all(root, vendor, symbol, rung, from, to, &union))
        .collect();
    let rules = crate::Rules::operator();
    let pooled = fold(&union, &surface, &priced, rules);
    crate::note(
        &telemetry::Event::info("cli.pool", "pass 2 finished")
            .with("pooled_rows", count(pooled.len()))
            .with(
                "meeting_rule",
                count(
                    pooled
                        .iter()
                        .filter(|p| p.meets(tail_rule_bp(rules)))
                        .count(),
                ),
            ),
    );
    render_pooled(&mut out, &union, &surface, &priced, &pooled, rules);
    Ok(out)
}

/// Everything the page says before a bar is read, and the surface pass 1
/// screens, with the store root supplied rather than read from the
/// environment. D-0696.
///
/// The opening, the directories [`not_on_the_surface`] names, and the
/// empty-surface line. They were written in [`run`], which checks the commit
/// stamp first and so no test reaches, and the tests drove
/// [`surface_under`] and [`not_on_the_surface`] one at a time: deleting the
/// one line that put NOT ON THE SURFACE on the page left every test green,
/// and the page dropped a misfiled holding with no word. Split out for the
/// reason `batch::sweep_under` is, so a test drives the page's own head on a
/// scratch store.
fn head_under(
    root: &std::path::Path,
    vendor_word: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> Result<(String, Vec<String>), String> {
    let vendor = crate::parse_vendor(vendor_word)?;
    let Surface {
        symbols: surface,
        elsewhere,
    } = surface_under(root, vendor, rung)?;
    let mut out = opening(vendor_word, rung, from, to, support_ppm, &surface);
    not_on_the_surface(&mut out, &elsewhere);
    if surface.is_empty() {
        let _ = writeln!(
            out,
            "  0 instruments on the surface for {vendor_word} at {rung}. The store holds \
             no month of any swept index or F&O cash equity at the path its own load \
             reads on this feed and rung, so there is nothing to screen and nothing to \
             pool. Nothing was read."
        );
    }
    Ok((out, surface))
}

/// `usize` as the `u64` a telemetry field takes, saturating rather than
/// wrapping on a platform where that could differ.
fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Every symbol the store holds on this feed and rung that is on the engine
/// surface, sorted and deduplicated.
///
/// `swept_index` is the one authority — the two indices by name and the F&O
/// cash equities by `FNO_INDEX` less its five index names — so a stored equity
/// off the F&O list, a reference index, an F&O index other than the two, or a
/// contract is skipped here without a second list.
///
/// # One entry per instrument, and only at the path its own load reads
///
/// This kept `h.symbol`, the directory's own spelling, and asked `swept_index`
/// only whether the WORD resolved. Two defects followed, both counted in the
/// pooled totals rather than refused:
///
/// * `NSE/CASH/reliance` beside `NSE/CASH/RELIANCE` (or `NSE/INDEX/RELIANCE`)
///   listed `["RELIANCE", "reliance"]`. Both resolve to `NSE-RELIANCE`, pass 1
///   loads the same canonical bars twice, and `fold` sums that instrument's
///   trades, wins and net twice.
/// * A `BSE/...` holding was listed, because the exchange directory was never
///   read, and `swept_index` resolves the bare name to an NSE key. The opening
///   line counted it among the POOL's instruments while pass 1 read NSE paths.
///
/// The first repair compared the exchange and segment exactly and folded the
/// symbol's case, and said it listed an instrument only where "the file a load
/// of that key opens" is. That was false both ways (D-0696): on the
/// case-sensitive filesystem CI runs, `NSE/CASH/Reliance` was listed as
/// `RELIANCE` though the canonical load opens nothing there, and on a
/// case-insensitive one `nse/CASH/RELIANCE` was dropped with no word though
/// that load opens it.
///
/// So all three directories are now compared exactly with the resolved key's
/// own path segments, through [`stored::misfiled`], the one rule `sweep-all`
/// applies too. A holding at that path is the instrument, listed once by its
/// canonical symbol. A holding whose symbol directory resolves to a swept key
/// anywhere else -- another exchange, the other segment, another spelling --
/// is not listed and is NAMED in [`Surface::elsewhere`], once per directory,
/// so the report says what it did not read rather than dropping it silently.
/// A holding whose word does not resolve at all -- an off-list equity, a
/// reference index, an F&O index, a contract -- is skipped here without a
/// word, as it always was: it is not an instrument this engine sweeps.
fn surface_under(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    rung: &str,
) -> Result<Surface, String> {
    let holdings = catalog::walk(root).map_err(|why| why.to_string())?;
    let mut symbols = BTreeSet::new();
    let mut elsewhere = BTreeMap::new();
    for h in holdings
        .held
        .iter()
        .filter(|h| h.vendor == vendor && h.timeframe.as_str() == rung)
    {
        let Ok(key) = stored::swept_index(&h.symbol) else {
            continue;
        };
        match stored::misfiled(&key, &h.exchange, &h.segment, &h.symbol) {
            None => {
                symbols.insert(key.underlying.as_str().to_owned());
            }
            Some(why) => {
                elsewhere
                    .entry((h.exchange.as_str(), h.segment.as_str(), h.symbol.as_str()))
                    .or_insert(why);
            }
        }
    }
    Ok(Surface {
        symbols: symbols.into_iter().collect(),
        elsewhere: elsewhere.into_values().collect(),
    })
}

/// What one feed and rung of the store holds, split into what the pool reads
/// and what it names without reading. D-0696.
#[derive(Debug, PartialEq, Eq)]
struct Surface {
    /// One canonical symbol per swept instrument held at its own path, sorted.
    symbols: Vec<String>,
    /// One [`stored::misfiled`] sentence per held directory whose symbol names
    /// a swept instrument at a path that instrument's load does not read,
    /// sorted by directory.
    elsewhere: Vec<String>,
}

/// The holdings the surface names and does not read, under the opening they
/// qualify. Nothing when there are none, so a store whose every holding is at
/// its own path renders exactly as it did before D-0696.
fn not_on_the_surface(out: &mut String, elsewhere: &[String]) {
    if elsewhere.is_empty() {
        return;
    }
    let _ = writeln!(
        out,
        "\n  NOT ON THE SURFACE: {} held director(ies) name a swept instrument at a path \
         its load does not read.\n  Nothing under them was screened or pooled:",
        elsewhere.len()
    );
    for why in elsewhere {
        let _ = writeln!(out, "    {why}");
    }
    out.push('\n');
}

/// The pool's charge statement, the opening's own lines. D-0696.
///
/// It quoted "STT alone is 0.025% of every sell -- D-0506", a rate
/// `docs/00-charter.md` gives no source for, and D-0696 withdraws it. Its
/// words are now the audit header's own charge list and cost-excluded
/// sentence, and it names no rate. The test
/// `sweep_wiring_tests::every_equity_charge_statement_is_the_audit_headers_own_and_names_no_rate`
/// holds it to the header `runner::audit::render` prints.
pub(crate) const EQUITY_TOTALS_GROSS: &str = "NO COST OF ANY KIND IS CHARGED. Correct on an index by charter; NOT correct on a\n\
     cash equity, where brokerage, STT, stamp duty, exchange charges, the SEBI fee and\n\
     GST all apply and none is subtracted: every equity total is GROSS OF EVERY CHARGE.\n\
     COST-EXCLUDED RESEARCH, NOT A NET RESULT (D-0509, D-0525, D-0681). No equity result\n\
     carries Selection V6 or execution authority until a charter-sourced equity charge\n\
     stack exists.";

fn opening(
    vendor_word: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
    surface: &[String],
) -> String {
    let mut out = String::from(crate::STORED_PROVENANCE);
    let _ = writeln!(
        out,
        "feed {vendor_word} · POOL over {} instrument(s) · {rung} · {}-{:02}..{}-{:02} · support {}",
        surface.len(),
        from.0,
        from.1,
        to.0,
        to.1,
        crate::support_word(support_ppm)
    );
    let _ = writeln!(
        out,
        "WHICH STOCK, BEFORE IT MOVES. Pass 1 screens every instrument alone; pass 2 prices\n\
         the union of their top combinations on every instrument and POOLS the trades.\n\
         {EQUITY_TOTALS_GROSS}\n\
         Every total below is gross, in sample, unvalidated, and the largest of\n\
         instruments × candidates comparisons. It finds a candidate; `range-rung` with\n\
         validation on is the proof."
    );
    // BESIDE THE CHARGE STATEMENT, WHENEVER A STOCK IS IN THE POOL. A pool of
    // the two indices never carries it: an index never splits (D-0018). D-0694.
    if stored::any_cash_equity(surface.iter().map(String::as_str)) {
        let _ = writeln!(out, "{}", runner::audit::CORPORATE_ACTIONS_UNCHECKED);
    }
    out
}

fn render_per_symbol(out: &mut String, screened: &[Screened]) {
    let _ = writeln!(out, "\n  PASS 1 -- PER SYMBOL, each on its own bars");
    let _ = writeln!(
        out,
        "  {:<14}{:>9}{:>9}{:>6}{:>8}{:>12}{:>12}{:>12}{:>10}",
        "symbol", "bars", "min_hits", "depth", "trades", "worst", "net", "max_dd", "ret/DD"
    );
    // SORTED BY THE MONEY, not by name: the smallest drawdown first, then the
    // worst trade closest to zero, then the net. Refusals sort last and are
    // named, never dropped.
    let mut rows: Vec<&Screened> = screened.iter().collect();
    rows.sort_by_key(|s| match &s.outcome {
        Ok(r) => (
            false,
            r.max_drawdown.saturating_neg(),
            r.worst_trade,
            r.pessimistic,
        ),
        Err(_) => (true, i64::MIN, i64::MIN, i64::MIN),
    });
    rows.reverse();
    for s in rows {
        match &s.outcome {
            Err(why) => {
                let _ = writeln!(out, "  {:<14}REFUSED: {why}", s.symbol);
            }
            Ok(r) => {
                let _ = writeln!(
                    out,
                    "  {:<14}{:>9}{:>9}{:>6}{:>8}{:>12}{:>12}{:>12}{:>10}",
                    s.symbol,
                    r.bars,
                    r.min_hits,
                    r.depth,
                    r.trades,
                    r.worst_trade,
                    r.pessimistic,
                    r.max_drawdown,
                    crate::return_over_drawdown_cell(r.pessimistic, r.max_drawdown),
                );
            }
        }
    }
}

/// The union of every screened instrument's frontier rows as `(mask, side)`.
///
/// One `HashSet` insert per row decides membership; the `Vec` keeps first-seen
/// order so the table is stable across runs. Instruments whose rows cannot be
/// read are returned by name with the reason rather than skipped.
fn union_of(
    root: &std::path::Path,
    screened: &[Screened],
) -> (Vec<Candidate>, Vec<(String, String)>) {
    let mut seen: HashSet<Candidate> = HashSet::new();
    let mut union: Vec<Candidate> = Vec::new();
    let mut unread: Vec<(String, String)> = Vec::new();
    let mut frontier = match Frontier::open_read(root) {
        Ok(f) => f,
        Err(why) => {
            for s in screened {
                if s.outcome.is_ok() {
                    unread.push((s.symbol.clone(), format!("frontier not opened: {why}")));
                }
            }
            return (union, unread);
        }
    };
    for s in screened {
        let Ok(record) = &s.outcome else {
            continue;
        };
        match frontier.of_run(&record.identity) {
            Ok((rows, damage)) => {
                if let Some(why) = damage {
                    unread.push((s.symbol.clone(), why));
                    continue;
                }
                if let Err(why) = seen
                    .try_reserve(rows.len())
                    .and_then(|()| union.try_reserve(rows.len()))
                {
                    unread.push((
                        s.symbol.clone(),
                        format!("frontier allocation refused: {why}"),
                    ));
                    continue;
                }
                for row in rows {
                    let candidate = Candidate {
                        words: row.mask_words,
                        direction: row.direction,
                    };
                    if seen.insert(candidate) {
                        union.push(candidate);
                    }
                }
            }
            Err(why) => unread.push((s.symbol.clone(), why)),
        }
    }
    (union, unread)
}

/// Every candidate priced on one instrument, in union order.
///
/// The span is prepared exactly as the screen prepares one — the same loaders,
/// the same execution-series check, the same withheld days, the same anchored
/// column, the same VWAP verdict — so a cell here is the cell `range-rung`
/// would show for that mask on that instrument. `the_pool_prepares_a_span_exactly_as_the_screen_does`
/// pins the sequence.
fn price_all(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    underlying: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    union: &[Candidate],
) -> Result<Vec<Priced>, String> {
    let mut span = stored::load_span(root, vendor, underlying, rung, from, to)?;
    let signal_length = stored::rung_length_micros(rung)?;
    let execution_bars = if rung == crate::EXECUTION_RUNG {
        None
    } else {
        Some(stored::load_span(
            root,
            vendor,
            underlying,
            crate::EXECUTION_RUNG,
            from,
            to,
        )?)
    };
    let execution_slice = execution_bars
        .as_ref()
        .map_or(span.bars.as_slice(), |exec| exec.bars.as_slice());
    crate::validate_one_minute_execution(execution_slice).map_err(|why| {
        format!(
            "the {} execution span is malformed: {why}. Nothing was priced.",
            crate::EXECUTION_RUNG
        )
    })?;
    let holed_days = crate::minute_gaps::days_with_interior_gaps(execution_slice);
    if !holed_days.is_empty() {
        let (kept, _withheld) = crate::minute_gaps::withhold(&span.bars, &holed_days);
        span.bars = kept;
    }
    let daily = stored::load_daily_context(root, vendor, underlying, (from, to), &span.bars)?;
    let exact_minute =
        stored::load_exact_minute_context(root, vendor, underlying, (from, to), &span.bars)?;
    let availability = stored::vwap_availability(&span.key);
    let column = crate::stored_anchored_column(
        &span.bars,
        &daily,
        &exact_minute,
        signal_length,
        availability,
    )?;
    let bars = span.bars.as_slice();
    let horizon = crate::horizon_for(bars, rung != crate::EXECUTION_RUNG);
    let hold = usize::try_from(horizon.as_bars()).unwrap_or(usize::MAX);
    let rules = crate::Rules::derived(bars, horizon);
    let stop_rungs = crate::stop_ladder_ppm(bars, hold);
    let levels = grid::Levels {
        rungs: crate::grid_rungs(bars),
        step_ppm: Some(crate::grid_step_ppm(bars, hold)),
        forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
        ratios: true,
        stops_ppm: &stop_rungs,
    };
    let facts = runner::trade::SliceFacts::of(bars, &column);
    Ok(union
        .iter()
        .map(|candidate| {
            let mask = vocab::ConditionMask::from_words(candidate.words);
            let side = match candidate.direction {
                Direction::Long => runner::excursion::Side::Long,
                Direction::Short => runner::excursion::Side::Short,
            };
            let g = grid::evaluate_over(bars, &column, &mask, horizon, side, levels, &facts);
            crate::shown_cell(&g, rules)
                .map(|(cell, _admitted)| cell)
                .filter(|cell| cell.trades > 0)
        })
        .collect())
}

/// UNVERIFIED performance: no named cost test or measured latency bound is established here.
/// Pool every candidate's cells across the instruments that priced.
///
/// One pass over `instruments × candidates` with O(1) work per cell. An
/// instrument whose pricing refused contributes to no candidate and is named
/// by the renderer, never folded in as zeros.
fn fold(
    union: &[Candidate],
    surface: &[String],
    priced: &[Result<Vec<Priced>, String>],
    rules: crate::Rules,
) -> Vec<Pooled> {
    /// Symbols named on a pooled row before "and N more".
    const NAMED: usize = 5;
    let mut pooled: Vec<Pooled> = (0..union.len())
        .map(|candidate| Pooled {
            candidate,
            fired: 0,
            trades: 0,
            wins: 0,
            net: 0,
            worst: 0,
            min_win: i64::MAX,
            gross_win: 0,
            gross_loss: 0,
            dd_bound: 0,
            names: Vec::new(),
        })
        .collect();
    for (symbol, cells) in surface.iter().zip(priced) {
        let Ok(cells) = cells else {
            continue;
        };
        for (p, cell) in pooled.iter_mut().zip(cells) {
            let Some(cell) = cell else {
                continue;
            };
            p.fired = p.fired.saturating_add(1);
            p.trades = p.trades.saturating_add(cell.trades);
            p.wins = p.wins.saturating_add(cell.wins);
            p.net = p.net.saturating_add(cell.pessimistic);
            p.worst = p.worst.min(cell.worst_trade);
            if cell.wins > 0 {
                p.min_win = p.min_win.min(cell.min_win);
            }
            p.gross_win = p.gross_win.saturating_add(cell.gross_win);
            p.gross_loss = p.gross_loss.saturating_add(cell.gross_loss);
            p.dd_bound = p.dd_bound.max(cell.max_drawdown);
            if p.names.len() < NAMED {
                p.names.push(symbol.clone());
            }
        }
    }
    // A candidate that won nowhere has no smallest win; zero, not the sentinel.
    for p in &mut pooled {
        if p.wins == 0 {
            p.min_win = 0;
        }
    }
    pooled.retain(|p| p.fired > 0);
    let rule_bp = tail_rule_bp(rules);
    pooled.sort_by_key(|p| core::cmp::Reverse(p.key(rule_bp)));
    pooled
}

fn render_pooled(
    out: &mut String,
    union: &[Candidate],
    surface: &[String],
    priced: &[Result<Vec<Priced>, String>],
    pooled: &[Pooled],
    rules: crate::Rules,
) {
    let refused: Vec<(&String, &String)> = surface
        .iter()
        .zip(priced)
        .filter_map(|(s, p)| p.as_ref().err().map(|why| (s, why)))
        .collect();
    let priced_ok = surface.len().saturating_sub(refused.len());
    let rule_bp = tail_rule_bp(rules);
    let _ = writeln!(
        out,
        "\n  PASS 2 -- POOLED across {priced_ok} instrument(s), {} candidate (mask, side) pair(s)",
        union.len()
    );
    let _ = writeln!(
        out,
        "  tail = smallest win / largest loss; rule = tail >= {}.{:02}x (the operator's \
         reward-to-risk floor, 0 = OFF)\n  dd>= is the LARGEST single-instrument drawdown \
         among those pooled: a lower bound on the pooled drawdown, not the figure itself",
        rule_bp / 100,
        rule_bp % 100
    );
    let _ = writeln!(
        out,
        "  {:>4} {:<5}{:>6}{:>8}{:>6}{:>12}{:>12}{:>10}{:>10}{:>12}{:>5}  fired on",
        "rank", "side", "fired", "trades", "wins", "worst", "min_win", "tail", "pf", "net", "dd>="
    );
    for (rank, p) in pooled.iter().take(rules.top.max(1)).enumerate() {
        let Some(candidate) = union.get(p.candidate) else {
            let _ = writeln!(
                out,
                "refused: pooled rank {} names missing candidate {}; no row was fabricated",
                rank + 1,
                p.candidate
            );
            continue;
        };
        let _ = writeln!(
            out,
            "  {:>4} {:<5}{:>6}{:>8}{:>6}{:>12}{:>12}{:>10}{:>10}{:>12}{:>5}  {}{}",
            rank + 1,
            match candidate.direction {
                Direction::Long => "long",
                Direction::Short => "short",
            },
            p.fired,
            p.trades,
            p.wins,
            p.worst,
            p.min_win,
            ratio_cell(p.tail_bp()),
            ratio_cell(p.profit_factor_bp()),
            p.net,
            p.dd_bound,
            p.names.join(" "),
            if p.fired > count(p.names.len()) {
                format!(" +{} more", p.fired.saturating_sub(count(p.names.len())))
            } else {
                String::new()
            }
        );
        let _ = writeln!(
            out,
            "       mask {}{}",
            mask_hex(candidate.words),
            if p.meets(rule_bp) { "  RULE MET" } else { "" }
        );
    }
    if pooled.is_empty() {
        let _ = writeln!(
            out,
            "  no candidate fired on any instrument under the shown cell; nothing to rank"
        );
    }
    if !refused.is_empty() {
        let _ = writeln!(
            out,
            "\n  NOT PRICED in pass 2 for {} instrument(s); they are absent from every \
             pooled row above:",
            refused.len()
        );
        for (symbol, why) in refused {
            let _ = writeln!(out, "    {symbol}: {why}");
        }
    }
    out.push_str(crate::IN_SAMPLE_WARNING);
}

/// A ratio in hundredths as `12.34x`, or `never lost` for the sentinel.
fn ratio_cell(bp: i64) -> String {
    if bp == i64::MAX {
        "never lost".to_owned()
    } else {
        format!("{}.{:02}x", bp / 100, bp % 100)
    }
}

/// The six mask words as hex, so a row can be matched to a frontier row.
fn mask_hex(words: [u64; 6]) -> String {
    let mut out = String::with_capacity(6 * 17);
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            out.push(':');
        }
        let _ = write!(out, "{w:016x}");
    }
    out
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes — a test \
              that cannot panic cannot fail. `.expect` panics inside core, which \
              llvm-cov does not instrument, so no dead region is left behind"
)]
mod tests {
    use super::{Candidate, Direction, Pooled, fold, mask_hex, ratio_cell, tail_rule_bp};

    /// The operator's rules with the tail multiple PINNED, so no environment
    /// knob on the machine running the test can change what "the rule" is.
    fn rules_at(min_rr_bp: i64) -> crate::Rules {
        let mut rules = crate::Rules::operator();
        rules.min_rr_bp = min_rr_bp;
        rules
    }
    use runner::grid;

    fn cell(trades: u64, wins: u64, net: i64, worst: i64, min_win: i64, dd: i64) -> grid::Cell {
        grid::Cell {
            trades,
            wins,
            pessimistic: net,
            worst_trade: worst,
            min_win,
            gross_win: if wins > 0 {
                net.max(0).saturating_add(worst.abs())
            } else {
                0
            },
            gross_loss: worst,
            max_drawdown: dd,
            ..grid::Cell::default()
        }
    }

    fn candidates(n: usize) -> Vec<Candidate> {
        (0..n)
            .map(|i| Candidate {
                words: [
                    u64::try_from(i).unwrap_or(u64::MAX).saturating_add(1),
                    0,
                    0,
                    0,
                    0,
                    0,
                ],
                direction: if i % 2 == 0 {
                    Direction::Long
                } else {
                    Direction::Short
                },
            })
            .collect()
    }

    #[test]
    fn a_missing_pooled_candidate_is_reported_not_indexed() {
        let union = candidates(1);
        let surface = vec!["AAA".to_owned()];
        let priced = vec![Ok(vec![Some(cell(2, 1, 100, -10, 110, 10))])];
        let rules = rules_at(300);
        let pooled = fold(&union, &surface, &priced, rules);
        let mut out = String::new();
        super::render_pooled(&mut out, &[], &surface, &priced, &pooled, rules);
        assert!(out.contains("refused: pooled rank 1 names missing candidate 0"));
        assert!(!out.contains("       mask "));
    }

    /// **Pooling adds what adds and takes the extreme of what does not.**
    ///
    /// Two instruments, one candidate: trades, wins, net, gross win and gross
    /// loss are sums; the worst trade and the smallest win are minima; the
    /// drawdown is the MAX of the two, because it is a bound and not a sum.
    #[test]
    fn the_fold_adds_totals_and_takes_the_extremes() {
        let union = candidates(1);
        let surface = vec!["AAA".to_owned(), "BBB".to_owned()];
        let priced = vec![
            Ok(vec![Some(cell(3, 1, 900, -100, 1_000, 150))]),
            Ok(vec![Some(cell(2, 1, 400, -50, 450, 300))]),
        ];
        let pooled = fold(&union, &surface, &priced, rules_at(300));
        assert_eq!(pooled.len(), 1);
        let p = pooled.first().expect("one pooled row");
        assert_eq!((p.fired, p.trades, p.wins, p.net), (2, 5, 2, 1_300));
        assert_eq!(p.worst, -100, "the largest loss across instruments");
        assert_eq!(p.min_win, 450, "the smallest win across instruments");
        assert_eq!(p.dd_bound, 300, "the LARGEST single drawdown, a bound");
        assert_eq!(p.names, vec!["AAA", "BBB"]);
    }

    /// **A refused instrument and a candidate that never fired both stay out.**
    ///
    /// A refusal is not a row of zeros: it must not lower a minimum or count
    /// as fired. A candidate with no cell anywhere is dropped from the ranking
    /// rather than ranked as a zero-trade winner.
    #[test]
    fn refusals_and_unfired_candidates_are_never_folded_in() {
        let union = candidates(2);
        let surface = vec!["AAA".to_owned(), "BBB".to_owned(), "CCC".to_owned()];
        let priced = vec![
            Ok(vec![Some(cell(1, 1, 500, -10, 500, 10)), None]),
            Err("the 1min execution span is malformed".to_owned()),
            Ok(vec![None, None]),
        ];
        let pooled = fold(&union, &surface, &priced, rules_at(300));
        assert_eq!(pooled.len(), 1, "only the candidate that fired somewhere");
        let p = pooled.first().expect("one pooled row");
        assert_eq!(p.candidate, 0);
        assert_eq!(p.fired, 1);
        assert_eq!(p.names, vec!["AAA"]);
    }

    /// **The ranking is the rule, then the drawdown bound, then the profit
    /// factor with its never-lost sentinel demoted, then the net.**
    #[test]
    fn the_ranking_leads_with_the_rule_and_the_drawdown() {
        // 3.00x, pinned: the smallest win must be three times the largest loss.
        let rule_bp = tail_rule_bp(rules_at(300));
        assert_eq!(rule_bp, 300, "the rule is read from `Rules`, in hundredths");
        let meets = Pooled {
            candidate: 0,
            fired: 1,
            trades: 4,
            wins: 1,
            net: 100,
            worst: -10,
            min_win: 40,
            gross_win: 130,
            gross_loss: -30,
            dd_bound: 500,
            names: vec![],
        };
        let smaller_dd_but_fails = Pooled {
            candidate: 1,
            worst: -100,
            min_win: 10,
            dd_bound: 5,
            ..meets.clone()
        };
        let never_lost = Pooled {
            candidate: 2,
            worst: 0,
            gross_loss: 0,
            dd_bound: 500,
            ..meets.clone()
        };
        assert_eq!(meets.tail_bp(), 400, "40 over 10 is 4.00x");
        assert!(meets.meets(rule_bp));
        assert_eq!(smaller_dd_but_fails.tail_bp(), 10, "10 over 100 is 0.10x");
        assert!(!smaller_dd_but_fails.meets(rule_bp));
        assert_eq!(never_lost.profit_factor_bp(), i64::MAX);
        assert_eq!(
            crate::ranked(never_lost.profit_factor_bp()),
            i64::MIN,
            "never lost is demoted, as a cell's is"
        );
        assert!(
            meets.key(rule_bp) > smaller_dd_but_fails.key(rule_bp),
            "the rule outranks a smaller drawdown"
        );
        assert!(
            meets.key(rule_bp) > never_lost.key(rule_bp),
            "same drawdown, so the demoted sentinel loses to a measured ratio"
        );
        // With the rule OFF, the drawdown bound decides between the first two.
        assert!(
            smaller_dd_but_fails.key(0) > meets.key(0),
            "rule off: the smaller drawdown leads"
        );
    }

    /// **The tail and profit-factor cells render as multiples, and the
    /// sentinel as words.**
    #[test]
    fn ratios_render_as_multiples_and_the_sentinel_as_words() {
        assert_eq!(ratio_cell(1_234), "12.34x");
        assert_eq!(
            ratio_cell(125),
            "1.25x",
            "the default `min_rr_bp` renders as itself"
        );
        assert_eq!(ratio_cell(0), "0.00x");
        assert_eq!(ratio_cell(i64::MAX), "never lost");
        let p = Pooled {
            candidate: 0,
            fired: 1,
            trades: 1,
            wins: 1,
            net: 0,
            worst: -25,
            min_win: 100,
            gross_win: 100,
            gross_loss: -25,
            dd_bound: 0,
            names: vec![],
        };
        assert_eq!(p.tail_bp(), 400, "100 over 25 is 4.00x");
        assert_eq!(p.profit_factor_bp(), 400);
    }

    /// **A mask renders as six fixed-width hex words, so a pooled row can be
    /// matched to the frontier row it came from.**
    #[test]
    fn a_mask_renders_as_six_fixed_width_hex_words() {
        let text = mask_hex([1, 0, 0, 0, 0, 0x10]);
        assert_eq!(text.split(':').count(), 6);
        assert!(text.starts_with("0000000000000001:"));
        assert!(text.ends_with(":0000000000000010"));
    }

    /// **An empty or absent store pools nothing and says so.**
    #[test]
    fn an_empty_store_lists_no_surface() {
        let mut root = std::env::temp_dir();
        root.push(format!("brutex-pool-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch root");
        let surface = super::surface_under(&root, brutex_core::vendor::Vendor::Zerodha, "60min")
            .expect("an empty store is not an error");
        assert!(
            surface.symbols.is_empty() && surface.elsewhere.is_empty(),
            "{surface:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// **The surface is `swept_index`, not the catalog: a stored equity off the
    /// F&O list, a reference index and an F&O index other than the two are
    /// listed by the store and skipped here.**
    ///
    /// `FINNIFTY` and `MIDCPNIFTY` are F&O underlyings, and until D-0682 each
    /// reached this surface through the cash arm — as a stock that does not
    /// exist — whichever segment directory the store filed it under.
    #[test]
    fn the_surface_is_the_swept_index_and_not_everything_stored() {
        let mut root = std::env::temp_dir();
        root.push(format!("brutex-pool-surface-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for (segment, symbol) in [
            ("INDEX", "NIFTY"),
            ("INDEX", "INDIAVIX"),
            ("CASH", "RELIANCE"),
            ("CASH", "ZZQXNOTFNO"),
            ("CASH", "FINNIFTY"),
            ("INDEX", "MIDCPNIFTY"),
        ] {
            let dir = root.join(format!("bars/zerodha/NSE/{segment}/{symbol}/60min"));
            std::fs::create_dir_all(&dir).expect("dirs");
            std::fs::write(dir.join("2026-07.bin"), b"").expect("a file the catalog lists");
        }
        assert!(!brutex_core::universe::FNO_INDEX.contains("ZZQXNOTFNO"));
        for index in ["FINNIFTY", "MIDCPNIFTY"] {
            assert!(
                brutex_core::universe::FNO_INDEX.contains(index)
                    && brutex_core::universe::FNO_INDEX_UNDERLYINGS.contains(&index),
                "{index} must be an F&O index underlying, or its absence below proves nothing"
            );
        }
        let surface = super::surface_under(&root, brutex_core::vendor::Vendor::Zerodha, "60min")
            .expect("listed");
        assert_eq!(
            surface.symbols,
            vec!["NIFTY".to_owned(), "RELIANCE".to_owned()]
        );
        assert!(surface.elsewhere.is_empty(), "{surface:?}");
        let other_rung = super::surface_under(&root, brutex_core::vendor::Vendor::Zerodha, "15min")
            .expect("listed");
        assert!(
            other_rung.symbols.is_empty() && other_rung.elsewhere.is_empty(),
            "the rung filters too"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A scratch store holding one empty `2026-07` file under each
    /// `bars/zerodha/<dir>/60min` directory named, for `surface_under` to list.
    fn store_holding(tag: &str, dirs: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("brutex-pool-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in dirs {
            let at = root.join(format!("bars/zerodha/{dir}/60min"));
            std::fs::create_dir_all(&at).expect("dirs");
            std::fs::write(at.join("2026-07.bin"), b"").expect("a file the catalog lists");
        }
        root
    }

    /// **An F&O index filed under a case variant is still not on the surface.**
    ///
    /// `swept_index` folds case before core's predicate, so `finnifty` is
    /// refused exactly as `FINNIFTY` is. D-0682 pinned only upper-case
    /// directories here.
    #[test]
    fn a_case_variant_fno_index_directory_is_not_on_the_surface() {
        let root = store_holding(
            "index-case",
            &[
                "NSE/CASH/finnifty",
                "NSE/INDEX/NiftyNxt50",
                "NSE/CASH/midcpnifty",
            ],
        );
        let surface = super::surface_under(&root, brutex_core::vendor::Vendor::Zerodha, "60min")
            .expect("listed");
        assert!(
            surface.symbols.is_empty() && surface.elsewhere.is_empty(),
            "no F&O index is a swept instrument, so none is named either: {surface:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The surface of a scratch store holding `dirs`, and the store removed.
    fn surface_of(tag: &str, dirs: &[&str]) -> super::Surface {
        let root = store_holding(tag, dirs);
        let surface = super::surface_under(&root, brutex_core::vendor::Vendor::Zerodha, "60min")
            .expect("listed");
        let _ = std::fs::remove_dir_all(&root);
        surface
    }

    /// The exact sentence `pool` names a misfiled directory by.
    fn named(dir: &str, key: &str) -> String {
        let (exchange, rest) = dir.split_once('/').expect("exchange");
        let (segment, symbol) = rest.split_once('/').expect("segment");
        let key = crate::stored::swept_index(key).expect("a swept key");
        crate::stored::misfiled(&key, exchange, segment, symbol).expect("misfiled")
    }

    /// **One instrument is one surface entry, and only the directory its own
    /// key reads counts; every other directory naming it is named, not read.**
    ///
    /// The surface kept each directory's raw spelling, so `NSE/CASH/reliance`
    /// beside `NSE/INDEX/RELIANCE` listed `["RELIANCE", "reliance"]`: both
    /// resolve to `NSE-RELIANCE`, pass 1 loaded the same bars twice, and `fold`
    /// summed that stock's trades, wins and net twice. `NSE/INDEX/RELIANCE` is
    /// not even the file a RELIANCE load opens, and `NSE/CASH/NIFTY` is not the
    /// file a NIFTY load opens.
    ///
    /// No store below holds two directories that differ only in case under one
    /// parent. On a case-insensitive filesystem those are ONE directory, and
    /// the catalog would list whichever spelling was created first -- the
    /// first version of this test did exactly that, so its answer depended on
    /// the filesystem it ran on.
    #[test]
    fn one_instrument_is_one_surface_entry_at_the_path_its_key_reads() {
        let surface = surface_of(
            "spellings",
            &[
                "NSE/CASH/RELIANCE",
                "NSE/INDEX/RELIANCE",
                "BSE/CASH/RELIANCE",
                "NSE/CASH/NIFTY",
                "NSE/INDEX/BANKNIFTY",
            ],
        );
        assert_eq!(
            surface.symbols,
            vec!["BANKNIFTY".to_owned(), "RELIANCE".to_owned()]
        );
        assert_eq!(
            surface.elsewhere,
            vec![
                named("BSE/CASH/RELIANCE", "RELIANCE"),
                named("NSE/CASH/NIFTY", "NIFTY"),
                named("NSE/INDEX/RELIANCE", "RELIANCE"),
            ],
            "each misfiled directory is named once, in directory order"
        );

        // One spelling only, and it is not the canonical one: not listed under
        // a name it does not carry, and named instead.
        let surface = surface_of("one-spelling", &["NSE/CASH/reliance"]);
        assert!(surface.symbols.is_empty(), "{surface:?}");
        assert_eq!(
            surface.elsewhere,
            vec![named("NSE/CASH/reliance", "RELIANCE")]
        );
    }

    /// **A share held only under the index segment is not on the surface.**
    /// D-0696.
    ///
    /// Beside `NSE/CASH/RELIANCE` a misfiled `NSE/INDEX/RELIANCE` maps to the
    /// same canonical entry, so its exclusion cannot be seen: a surface that
    /// skipped the segment check for equities alone passed every test above.
    /// Held alone, it would list RELIANCE while pass 1 reads
    /// `NSE/CASH/RELIANCE`, which is the defect the surface was changed for.
    #[test]
    fn a_share_held_only_under_the_index_segment_is_not_on_the_surface() {
        for (tag, dir) in [
            ("share-as-index", "NSE/INDEX/RELIANCE"),
            ("share-as-index-lower", "NSE/INDEX/reliance"),
        ] {
            let surface = surface_of(tag, &[dir]);
            assert!(surface.symbols.is_empty(), "{dir}: {surface:?}");
            assert_eq!(surface.elsewhere, vec![named(dir, "RELIANCE")], "{dir}");
            assert!(
                surface.elsewhere.first().is_some_and(|why| {
                    why.contains("a load of `NSE-RELIANCE` reads `NSE/CASH/RELIANCE`")
                }),
                "{dir}: the sentence names the path that is read: {surface:?}"
            );
        }
    }

    /// **A directory spelt otherwise than the writer spells it is not on the
    /// surface, on any filesystem.** D-0696.
    ///
    /// The first repair compared the exchange and segment exactly and folded
    /// the symbol's case, so `NSE/CASH/Reliance` was listed as RELIANCE -- and
    /// on a case-sensitive filesystem the canonical load then refused it as
    /// absent -- while `nse/CASH/RELIANCE` was dropped with no word although a
    /// case-insensitive filesystem opens it under the canonical load. Each is
    /// now held to the writer's spelling in all three directories and named.
    #[test]
    fn a_case_variant_directory_of_a_swept_instrument_is_named_and_not_read() {
        for (tag, dir, word) in [
            ("mixed-symbol", "NSE/CASH/Reliance", "RELIANCE"),
            ("lower-exchange", "nse/CASH/RELIANCE", "RELIANCE"),
            ("mixed-segment", "NSE/Cash/RELIANCE", "RELIANCE"),
            ("lower-index", "NSE/INDEX/nifty", "NIFTY"),
            ("lower-index-segment", "NSE/index/BANKNIFTY", "BANKNIFTY"),
        ] {
            let surface = surface_of(tag, &[dir]);
            assert!(surface.symbols.is_empty(), "{dir}: {surface:?}");
            assert_eq!(surface.elsewhere, vec![named(dir, word)], "{dir}");
        }
    }

    /// **BSE is not on the surface.** `CLAUDE.md` §1: BSE is neither swept nor
    /// pulled. The exchange directory was never read, and `swept_index`
    /// resolves a bare name to an NSE key, so a store holding only BSE files
    /// opened its report with `POOL over 2 instrument(s)`. Each is named as a
    /// directory that is not where its NSE namesake is read from.
    #[test]
    fn a_bse_holding_is_not_on_the_surface() {
        let surface = surface_of("bse", &["BSE/CASH/RELIANCE", "BSE/INDEX/NIFTY"]);
        assert!(surface.symbols.is_empty(), "{surface:?}");
        assert_eq!(
            surface.elsewhere,
            vec![
                named("BSE/CASH/RELIANCE", "RELIANCE"),
                named("BSE/INDEX/NIFTY", "NIFTY"),
            ]
        );
    }

    /// **The report names what the surface did not read, and says nothing
    /// when every holding is at its own path.** D-0696.
    #[test]
    fn the_report_names_each_directory_the_surface_did_not_read() {
        let mut quiet = String::new();
        super::not_on_the_surface(&mut quiet, &[]);
        assert!(quiet.is_empty(), "no block when nothing was left out");

        let elsewhere = vec![
            named("BSE/CASH/RELIANCE", "RELIANCE"),
            named("NSE/INDEX/RELIANCE", "RELIANCE"),
        ];
        let mut page = String::new();
        super::not_on_the_surface(&mut page, &elsewhere);
        assert!(
            page.starts_with("\n  NOT ON THE SURFACE: 2 held director(ies)"),
            "{page}"
        );
        for why in &elsewhere {
            assert!(page.contains(&format!("\n    {why}\n")), "{page}");
        }
        assert!(
            !crate::carries_refusal(&page),
            "naming a directory is not a refusal of the pool:\n{page}"
        );
    }

    /// **The pool page itself names each directory it did not read, under its
    /// opening.** D-0696.
    ///
    /// The test above renders the block alone, and the surface tests list it
    /// alone. The line that put the block on the page sat in `run`, which no
    /// test reaches without a verified commit stamp, so deleting it left every
    /// test green while the page dropped a misfiled holding with no word. This
    /// drives `head_under`, the page's head as `run` prints it.
    #[test]
    fn the_pool_page_names_each_directory_it_did_not_read() {
        let head = |tag: &str, dirs: &[&str]| {
            let root = store_holding(tag, dirs);
            let head = super::head_under(&root, "zerodha", "60min", (2026, 7), (2026, 7), None)
                .expect("the head renders");
            let _ = std::fs::remove_dir_all(&root);
            head
        };
        let opening = |surface: &[&str]| {
            let owned: Vec<String> = surface.iter().map(|s| (*s).to_owned()).collect();
            super::opening("zerodha", "60min", (2026, 7), (2026, 7), None, &owned)
        };
        let block = |dir: &str, key: &str| {
            format!(
                "\n  NOT ON THE SURFACE: 1 held director(ies) name a swept instrument at a path \
                 its load does not read.\n  Nothing under them was screened or pooled:\n    {}\n\n",
                named(dir, key)
            )
        };
        let empty = "  0 instruments on the surface for zerodha at 60min.";

        // Beside the instrument it names: the pool still runs over RELIANCE.
        let (page, surface) = head("page-beside", &["NSE/CASH/RELIANCE", "BSE/CASH/RELIANCE"]);
        assert_eq!(surface, vec!["RELIANCE".to_owned()]);
        assert_eq!(
            page,
            format!(
                "{}{}",
                opening(&["RELIANCE"]),
                block("BSE/CASH/RELIANCE", "RELIANCE")
            ),
            "the opening, then the directory it did not read, and nothing else"
        );
        assert!(!crate::carries_refusal(&page), "{page}");

        // Alone: named, and then the empty-surface line.
        let (page, surface) = head("page-alone", &["NSE/INDEX/RELIANCE"]);
        assert!(surface.is_empty(), "{surface:?}");
        assert!(
            page.starts_with(&format!(
                "{}{}{empty}",
                opening(&[]),
                block("NSE/INDEX/RELIANCE", "RELIANCE")
            )),
            "{page}"
        );

        // Every holding at its own path: the opening alone, as before D-0696.
        let (page, _) = head("page-clean", &["NSE/CASH/RELIANCE", "NSE/INDEX/NIFTY"]);
        assert_eq!(page, opening(&["NIFTY", "RELIANCE"]));

        // A directory name carrying a newline is named on one line, and the
        // page it heads is not a refusal (D-0696). Raw, the name put
        // `refused: forged/...` at column zero, as the review measured.
        let (page, surface) = head(
            "page-forged",
            &["NSE/CASH/RELIANCE", "X\nrefused: forged/CASH/RELIANCE"],
        );
        assert_eq!(surface, vec!["RELIANCE".to_owned()]);
        assert!(
            page.contains("\n    `X\\nrefused: forged/CASH/RELIANCE` is not where"),
            "{page}"
        );
        assert!(
            !page.lines().any(|line| line.starts_with("refused")),
            "{page}"
        );
        assert!(!crate::carries_refusal(&page), "{page}");
    }

    /// **The page `pool` prints IS its head, and every later line is
    /// appended.** D-0696.
    ///
    /// The test above drives `head_under`, and `run` -- which no test reaches,
    /// because it checks the commit stamp first -- was what put that head on
    /// the page. The review replaced `run`'s `out` with the bare opening after
    /// `head_under` returned, dropping the NOT ON THE SURFACE block from the
    /// real page, and every test stayed green. Everything after the stamp,
    /// feed, rung and root checks is now `run_under`, which this drives on a
    /// scratch store whose surface is empty, where it returns after the head.
    /// A surface with an instrument on it is screened through `one_rung`,
    /// which reads the root from the environment, so that path is held here by
    /// the shape of the source instead: `out` is bound from `head_under` once,
    /// is never assigned again, and the page writes neither the opening nor
    /// the block itself.
    #[test]
    fn the_pool_page_is_its_head_and_then_only_appends() {
        let root = store_holding("run-under", &["NSE/INDEX/RELIANCE", "BSE/CASH/RELIANCE"]);
        let page = super::run_under(
            &root,
            brutex_core::vendor::Vendor::Zerodha,
            "zerodha",
            "60min",
            (2026, 7),
            (2026, 7),
            None,
        )
        .expect("the page renders");
        let (head, surface) =
            super::head_under(&root, "zerodha", "60min", (2026, 7), (2026, 7), None)
                .expect("the head renders");
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            surface.is_empty(),
            "premise: nothing to screen: {surface:?}"
        );
        assert!(
            head.contains("NOT ON THE SURFACE: 2 held director(ies)"),
            "premise: the head names both directories:\n{head}"
        );
        assert_eq!(page, head, "the page of an empty surface is its head");

        let source = include_str!("pool.rs");
        let from = source.find("\nfn run_under(").expect("run_under");
        let body = source
            .get(from..)
            .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to)))
            .expect("its body");
        assert!(
            body.len() > 2_000,
            "the body found is {} bytes, so the anchors moved and this reads nothing",
            body.len()
        );
        let bound =
            "let (mut out, surface) = head_under(root, vendor_word, rung, from, to, support_ppm)?;";
        assert_eq!(
            body.matches(bound).count(),
            1,
            "out comes from the head once"
        );
        let after = body
            .split_once(bound)
            .map(|(_, rest)| rest)
            .unwrap_or_default();
        for rewrite in [
            "out =",
            "out.clear()",
            "out.truncate(",
            "out.replace_range(",
            "out.drain(",
            "mem::take(&mut out)",
            "opening(",
            "not_on_the_surface(",
            "head_under(",
        ] {
            assert!(
                !after.contains(rewrite),
                "`{rewrite}` after the head: the page must be its head and then appends"
            );
        }
        assert!(
            body.split_once(bound)
                .is_some_and(|(before, _)| !before.contains("out")),
            "nothing is written before the head is bound"
        );
    }

    /// **The pool quotes no charge rate.** `CLAUDE.md` §3 rule 1: every claim
    /// about a cost traces to `docs/00-charter.md`, which records no source for
    /// an equity charge. The report said "STT alone is 0.025% of every sell".
    /// It still says every equity total is gross of every charge, and names
    /// the charges.
    #[test]
    fn the_pool_report_names_the_equity_charges_and_quotes_no_rate() {
        let page = super::opening("zerodha", "60min", (2025, 1), (2025, 2), Some(20_000), &[]);
        // The support word above it is a percentage of bars, not a charge.
        let (_, charges) = page.split_once("WHICH STOCK").expect("the paragraph");
        assert!(
            !charges.contains('%'),
            "no rate without a charter source:\n{charges}"
        );
        assert!(!charges.contains("0.025"), "{charges}");
        for claim in [
            "NO COST OF ANY KIND IS CHARGED",
            "brokerage, STT, stamp duty, exchange charges, the SEBI fee and\nGST",
            "every equity total is GROSS OF EVERY CHARGE",
            "COST-EXCLUDED RESEARCH, NOT A NET RESULT (D-0509, D-0525, D-0681)",
            "No equity result\ncarries Selection V6",
            super::EQUITY_TOTALS_GROSS,
        ] {
            assert!(page.contains(claim), "missing {claim:?}:\n{page}");
        }
        // The paragraph is the same on every pool page, whatever the surface:
        // an index-only surface and an empty one carry the equity sentence too.
        for surface in [vec!["BANKNIFTY".to_owned(), "NIFTY".to_owned()], vec![]] {
            let other = super::opening(
                "zerodha",
                "60min",
                (2025, 1),
                (2025, 2),
                Some(20_000),
                &surface,
            );
            let (_, same) = other.split_once("WHICH STOCK").expect("the paragraph");
            assert_eq!(same, charges, "{surface:?}");
        }
    }

    /// **A pool holding a stock says corporate actions are unchecked, beside
    /// its charge statement; a pool of the two indices never does.** D-0694.
    #[test]
    fn a_pool_with_a_stock_states_corporate_actions_are_unchecked_and_an_index_pool_does_not() {
        let text = |surface: &[&str]| {
            let owned: Vec<String> = surface.iter().map(|s| (*s).to_owned()).collect();
            super::opening("zerodha", "60min", (2026, 1), (2026, 6), None, &owned)
        };
        for surface in [
            &["NIFTY", "RELIANCE"][..],
            &["RELIANCE"][..],
            &["BANKNIFTY", "TCS", "NIFTY"][..],
        ] {
            let out = text(surface);
            assert!(
                out.contains(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
                "{surface:?}:\n{out}"
            );
            assert!(
                out.find("NO COST OF ANY KIND IS CHARGED")
                    < out.find(runner::audit::CORPORATE_ACTIONS_UNCHECKED),
                "beside and after the charge statement:\n{out}"
            );
        }
        for surface in [&["NIFTY", "BANKNIFTY"][..], &["NIFTY"][..], &[][..]] {
            let out = text(surface);
            assert!(
                !out.contains("CORPORATE ACTIONS"),
                "an index never splits, so an index pool never says so: {surface:?}\n{out}"
            );
        }
    }

    /// **The pool prepares a span exactly as the screen does.**
    ///
    /// The sequence of loaders and checks in `price_all` is the sequence in
    /// `screen_range_inner`, in the same order, so a cell the pool prices is
    /// the cell `range-rung` would show. Pinned by name because the two are
    /// separate functions and a loader added to one and not the other would
    /// price a different column without any test noticing.
    #[test]
    fn the_pool_prepares_a_span_exactly_as_the_screen_does() {
        const SEQUENCE: [&str; 10] = [
            "stored::load_span(",
            "stored::rung_length_micros(",
            "validate_one_minute_execution(",
            "minute_gaps::days_with_interior_gaps(",
            "minute_gaps::withhold(",
            "stored::load_daily_context(",
            "stored::load_exact_minute_context(",
            "stored::vwap_availability(",
            "stored_anchored_column(",
            "horizon_for(",
        ];
        let pool = include_str!("pool.rs");
        let lib = include_str!("lib.rs");
        let screen_at = lib
            .find("fn screen_range_inner(")
            .expect("the screen exists");
        let price_at = pool.find("fn price_all(").expect("the pool prices");
        let mut last_pool = price_at;
        let mut last_screen = screen_at;
        for step in SEQUENCE {
            let in_pool = pool[last_pool..].find(step).map(|i| i + last_pool);
            let in_screen = lib[last_screen..].find(step).map(|i| i + last_screen);
            assert!(
                in_pool.is_some() && in_screen.is_some(),
                "`{step}` must appear in both, after the previous step: pool={in_pool:?} screen={in_screen:?}"
            );
            last_pool = in_pool.expect("asserted above");
            last_screen = in_screen.expect("asserted above");
        }
    }
}
