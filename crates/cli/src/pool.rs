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
//! Pass 1 runs each instrument as a `crate::ordered::map` lane and pass 2 is
//! `par_iter` over INSTRUMENTS with indexed `collect`, so the order of every
//! table is the sorted symbol order and never the scheduler's, and pass 1's
//! shared ledger and evidence writes land in an order fixed by the surface
//! (D-1556).
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
/// store and reads that shape from the source: every mention of `out` after
/// the head is `writeln!(out, ..)`, `&mut out` handed to a renderer, or
/// `Ok(out)`, and every return is `out` itself.
/// `the_pool_verb_hands_on_run_unders_page_untouched` reads [`run`]'s body:
/// it ends by returning this function's result as it is, and builds no page
/// of its own by any phrasing that test lists.
///
/// When every instrument on the surface refuses in pass 1, the page is not
/// returned: [`at_least_one_screened`] refuses the verb instead, with every
/// instrument's reason and the head's `unread` blocks (D-0696).
///
/// **The root is supplied for the head, the union and pass 2, not pass 1.**
/// Pass 1 screens each instrument through `crate::one_rung`, exactly as
/// `range-rung` does, and that reads the store root from the environment. An
/// in-process test therefore drives this function only on a surface that is
/// empty, where it returns after the head. A surface with an instrument on it
/// is driven through the verb itself, by
/// `the_pool_verb_prints_its_whole_page_on_a_generated_store`, from a child
/// process whose environment names a generated store. That reaches this
/// function only in a stamped build -- one of a tree equal to HEAD, as a
/// clean checkout is -- because [`run`] checks the stamp first and an
/// unstamped build refuses there. The stamp never kept a clean build's test
/// out of [`run`]; the store root read from the environment is what keeps an
/// in-process test off a non-empty surface (D-0696).
fn run_under(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    vendor_word: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> Result<String, String> {
    let (mut out, surface, unread) = head_under(root, vendor_word, rung, from, to, support_ppm)?;
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
    //
    // EACH INSTRUMENT IS AN ORDERED LANE. The page was already in surface
    // order; the attempt tokens and ledger rows each `one_rung` writes followed
    // completion order until D-1556 (audit-20261003 hunt-conc-1). They now
    // land in an order fixed by the surface, `crate::ordered::WINDOW`
    // instruments at a time.
    let screened: Vec<Screened> = crate::ordered::map(&surface, |symbol| Screened {
        symbol: symbol.clone(),
        outcome: crate::one_rung(vendor_word, symbol, rung, from, to, support_ppm, None).outcome,
    })?;
    let screened_ok = screened.iter().filter(|s| s.outcome.is_ok()).count();
    crate::note(
        &telemetry::Event::info("cli.pool", "pass 1 finished")
            .with("screened", count(screened_ok))
            .with("refused", count(screened.len().saturating_sub(screened_ok))),
    );
    at_least_one_screened(&screened, &unread)?;
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
/// empty-surface line. They were written in [`run`], which no test then
/// drove, and the tests drove [`surface_under`] and [`not_on_the_surface`]
/// one at a time: deleting the one line that put NOT ON THE SURFACE on the
/// page left every test green, and the page dropped a misfiled holding with
/// no word. Split out for the reason `batch::sweep_under` is, so a test
/// drives the page's own head on a scratch store.
///
/// The catalog files a month only under a feed and a rung directory spelt
/// exactly as this engine spells them, so the empty-surface line says what
/// it found at that spelling, and [`not_catalogued`] counts the `.bin` files
/// under any other (D-0696).
///
/// The third value is `unread`: the NOT ON THE SURFACE and NOT CATALOGUED
/// blocks alone, exactly as the head carries them after the opening, so a
/// pool that refuses in pass 1 still says what it did not read, without the
/// banner (D-0696).
/// A surface with only misfiled holdings refuses with these blocks before
/// the opening is built. A store with no such holdings remains a page.
fn head_under(
    root: &std::path::Path,
    vendor_word: &str,
    rung: &str,
    from: (u16, u8),
    to: (u16, u8),
    support_ppm: Option<u64>,
) -> Result<(String, Vec<String>, String), String> {
    let vendor = crate::parse_vendor(vendor_word)?;
    let Surface {
        symbols: surface,
        elsewhere,
        unrecognised,
        unoffered,
    } = surface_under(root, vendor, rung)?;
    let mut unread = String::new();
    not_on_the_surface(&mut unread, &elsewhere);
    not_catalogued(&mut unread, unrecognised);
    not_walked(&mut unread, &unoffered);
    if surface.is_empty() && !elsewhere.is_empty() {
        return Err(format!(
            "no instrument is on the surface for {vendor_word} at {rung}; the catalog \
             holds swept instruments only at paths their loads do not read. Nothing \
             was screened or pooled.\n{}",
            unread.trim_end()
        ));
    }
    let mut out = opening(vendor_word, rung, from, to, support_ppm, &surface);
    out.push_str(&unread);
    if surface.is_empty() {
        let _ = writeln!(
            out,
            "  0 instruments on the surface for {vendor_word} at {rung}. The catalog finds \
             no month of any swept index or F&O cash equity at the path its own load \
             reads on this feed and rung, spelt exactly, so there is nothing to screen \
             and nothing to pool. Nothing was read."
        );
    }
    Ok((out, surface, unread))
}

/// The `.bin` files at a month's depth the catalog could not file because their feed or rung
/// directory is spelt as no feed or rung this engine knows, under the
/// opening. Nothing when there are none. D-0696.
///
/// `store::catalog` compares both directories exactly and only counts a
/// file it cannot place, so `bars/Zerodha/...` and a `60MIN` rung directory
/// were dropped from the page with no word. On a case-insensitive volume a
/// load of `zerodha` opens `Zerodha`, so the page said the store held no
/// month the load reads while that load read one (found by a review, which
/// measured the operator's store volume as case-insensitive). The census
/// counts such files store-wide and does not keep their names, so the block
/// gives the two counts, and says what a case-insensitive volume does with
/// them.
fn not_catalogued(out: &mut String, (feeds, rungs): (u64, u64)) {
    if feeds == 0 && rungs == 0 {
        return;
    }
    let _ = writeln!(
        out,
        "\n  NOT CATALOGUED: the store holds {feeds} .bin file(s) at a month's depth under a feed directory \
         and {rungs} under a rung directory\n  whose name is no feed or rung this engine \
         knows, spelt exactly. None was screened or pooled.\n  On a case-insensitive \
         volume a directory spelt as a feed or rung in another case is the one a load\n  \
         opens, so a month counted here can be one this page reads or says is not held.\n"
    );
}

/// The catalog's `unoffered_report`, indented like the other blocks the pool
/// names without reading, under a blank line. Nothing when it is empty.
/// D-0769.
fn not_walked(out: &mut String, unoffered: &str) {
    if unoffered.is_empty() {
        return;
    }
    out.push('\n');
    for line in unoffered.lines() {
        let _ = writeln!(out, "  {line}");
    }
}

/// `usize` as the `u64` a telemetry field takes, saturating rather than
/// wrapping on a platform where that could differ.
fn count(n: usize) -> u64 {
    u64::try_from(n).unwrap_or(u64::MAX)
}

/// Nothing, when pass 1 screened at least one instrument or had none to
/// screen; the pool's refusal when every instrument on the surface refused.
/// D-0696.
///
/// `range_over` refuses when every rung refuses, for the reason its comment
/// gives: the stored banner says real bars were read, and printed over
/// nothing with a zero exit it is the failure wearing a success's clothes
/// `CLAUDE.md` §4 bans. The pool printed that banner, a `REFUSED` row for each
/// instrument and "nothing to pool", and exited OK, because an indented
/// `REFUSED` row is not a refusal to `carries_refusal` (found by a review).
/// So when every instrument refused, the verb refuses and prints no page.
///
/// **A refused instrument is not always one that was never screened.** Pass
/// 1 keeps each instrument's `one_rung(..).outcome`, and an `Err` there can
/// follow a sweep that halted, or one whose result was not recorded. The
/// first version of this refusal said "nothing was screened" over those and
/// kept the first instrument's reason alone, so a fault every instrument hit,
/// such as a ledger no result could be filed into, reached the page as one
/// line or not at all, and the NOT ON THE SURFACE and NOT CATALOGUED blocks
/// were dropped (found by a review). The outcome is a sentence and carries no
/// mark of how far the instrument ran, so this names every instrument with
/// its own reason, claims nothing about what ran, and ends with `unread`,
/// the head's two blocks.
fn at_least_one_screened(screened: &[Screened], unread: &str) -> Result<(), String> {
    if screened.is_empty() || screened.iter().any(|s| s.outcome.is_ok()) {
        return Ok(());
    }
    let mut why = format!(
        "none of the {} instrument(s) on the surface came through pass 1 with a result, \
         so nothing can be pooled. No completed result could be confirmed. Each \
         instrument's own reason is below.\n",
        screened.len()
    );
    for s in screened {
        if let Err(reason) = &s.outcome {
            let _ = writeln!(why, "  {}: {reason}", s.symbol);
        }
    }
    why.push_str(unread);
    Err(why.trim_end().to_owned())
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
        unrecognised: (holdings.census.unknown_vendor, holdings.census.unknown_rung),
        unoffered: holdings.census.unoffered_report(),
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
    /// The store's `.bin` files at a month's depth under a feed directory, and under a rung
    /// directory, that is spelt as no feed or rung this engine knows: the
    /// catalog census's `unknown_vendor` and `unknown_rung`, store-wide.
    unrecognised: (u64, u64),
    /// The catalog's own `unoffered_report`, store-wide: the entries it saw
    /// below `bars/` and offered to nobody, or empty. D-0769.
    unoffered: String,
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
    use crate::columns::{left, right};
    let _ = writeln!(out, "\n  PASS 1 -- PER SYMBOL, each on its own bars");
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
    // LAID OUT TOGETHER (D-1420). Raw paisa at `i64::MIN` is 20 characters
    // in a 12-character column, and a 14-character symbol filled its column,
    // so `worst`, `net` and `max_dd` could read as one number.
    let columns = [
        left(14),
        right(9),
        right(9),
        right(6),
        right(8),
        right(12),
        right(12),
        right(12),
        right(10),
    ];
    let header = [
        "symbol", "bars", "min_hits", "depth", "trades", "worst", "net", "max_dd", "ret/DD",
    ];
    let mut cells: Vec<Vec<String>> = Vec::new();
    for s in &rows {
        cells.push(match &s.outcome {
            Err(_) => vec![s.symbol.clone()],
            Ok(r) => vec![
                s.symbol.clone(),
                r.bars.to_string(),
                r.min_hits.to_string(),
                r.depth.to_string(),
                r.trades.to_string(),
                r.worst_trade.to_string(),
                r.pessimistic.to_string(),
                r.max_drawdown.to_string(),
                crate::return_over_drawdown_cell(r.pessimistic, r.max_drawdown),
            ],
        });
    }
    let laid = crate::columns::with_header(&columns, &header, cells);
    let _ = writeln!(out, "  {}", laid.header);
    for (s, line) in rows.iter().zip(&laid.rows) {
        match &s.outcome {
            Err(why) => {
                let _ = writeln!(out, "  {line}REFUSED: {why}");
            }
            Ok(_) => {
                let _ = writeln!(out, "  {line}");
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
    let shown: Vec<(usize, &Pooled, Option<&Candidate>)> = pooled
        .iter()
        .take(rules.top.max(1))
        .enumerate()
        .map(|(rank, p)| (rank, p, union.get(p.candidate)))
        .collect();
    let laid = laid_pooled(&shown);
    let _ = writeln!(out, "  {}  fired on", laid.header);
    let mut lines = laid.rows.iter();
    for &(rank, p, candidate) in &shown {
        let (Some(candidate), Some(line)) = (candidate, candidate.and_then(|_| lines.next()))
        else {
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
            "  {line}  {}{}",
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

/// The pass-2 rows whose candidate exists, laid out under their header.
///
/// LAID OUT TOGETHER (v4-2, GAP13-16, D-1487). This was one `format!` of
/// adjacent width specifiers, the shape D-1420 removed from every other cli
/// table and left here: a 12-wide `net` beside a 5-wide `dd>=`, or raw paisa
/// at `i64::MIN` (20 characters) in `worst`, `min_win` or `net`, ran into its
/// neighbour and slid every column after it off its header. The widths stay as
/// minimums, so a table whose figures fit renders as it did.
fn laid_pooled(shown: &[(usize, &Pooled, Option<&Candidate>)]) -> crate::columns::Laid {
    use crate::columns::{left, right};
    let columns = [
        right(4),
        left(5).after(1),
        right(6),
        right(8),
        right(6),
        right(12),
        right(12),
        right(10),
        right(10),
        right(12),
        right(5),
    ];
    let header = [
        "rank", "side", "fired", "trades", "wins", "worst", "min_win", "tail", "pf", "net", "dd>=",
    ];
    let cells: Vec<Vec<String>> = shown
        .iter()
        .filter_map(|&(rank, p, candidate)| {
            candidate.map(|candidate| {
                vec![
                    (rank + 1).to_string(),
                    match candidate.direction {
                        Direction::Long => "long",
                        Direction::Short => "short",
                    }
                    .to_owned(),
                    p.fired.to_string(),
                    p.trades.to_string(),
                    p.wins.to_string(),
                    p.worst.to_string(),
                    p.min_win.to_string(),
                    ratio_cell(p.tail_bp()),
                    ratio_cell(p.profit_factor_bp()),
                    p.net.to_string(),
                    p.dd_bound.to_string(),
                ]
            })
        })
        .collect();
    crate::columns::with_header(&columns, &header, cells)
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

    /// A scratch store holding two empty months, `2026-06` and `2026-07`,
    /// under each `bars/zerodha/<dir>/60min` directory named, for
    /// `surface_under` to list.
    ///
    /// TWO MONTHS, because `store::catalog::walk` lists one holding per month
    /// and a directory is named once however many months it holds. With one
    /// month in each directory, a surface that named a directory once per
    /// month passed every test here (found by a review, D-0696).
    fn store_holding(tag: &str, dirs: &[&str]) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("brutex-pool-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in dirs {
            let at = root.join(format!("bars/zerodha/{dir}/60min"));
            std::fs::create_dir_all(&at).expect("dirs");
            for month in ["2026-06.bin", "2026-07.bin"] {
                std::fs::write(at.join(month), b"").expect("a file the catalog lists");
            }
        }
        let held = store::catalog::walk(&root)
            .expect("the scratch store is walked")
            .held;
        assert_eq!(
            held.len(),
            2 * dirs.len(),
            "premise: the catalog lists each directory's two months"
        );
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
    /// test then drove, so deleting it left every test green while the page
    /// dropped a misfiled holding with no word. This drives `head_under`, the
    /// page's head as `run` prints it.
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

        // Beside the instrument it names: the pool still runs over RELIANCE.
        // `unread` is the block alone, which a refusal in pass 1 carries.
        let (page, surface, unread) =
            head("page-beside", &["NSE/CASH/RELIANCE", "BSE/CASH/RELIANCE"]);
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
        assert_eq!(unread, block("BSE/CASH/RELIANCE", "RELIANCE"));
        assert!(!crate::carries_refusal(&page), "{page}");

        // Alone: refused, with the same directory reason and no stored banner.
        let root = store_holding("page-alone", &["NSE/INDEX/RELIANCE"]);
        let why = super::head_under(&root, "zerodha", "60min", (2026, 7), (2026, 7), None)
            .expect_err("all holdings are misfiled");
        std::fs::remove_dir_all(root).expect("scratch removed");
        assert!(why.starts_with("no instrument is on the surface"), "{why}");
        assert!(
            why.ends_with(block("NSE/INDEX/RELIANCE", "RELIANCE").trim_end()),
            "{why}"
        );
        assert!(!why.contains(crate::STORED_PROVENANCE), "{why}");

        // Every holding at its own path: the opening alone, as before D-0696.
        let (page, _, unread) = head("page-clean", &["NSE/CASH/RELIANCE", "NSE/INDEX/NIFTY"]);
        assert_eq!(page, opening(&["NIFTY", "RELIANCE"]));
        assert_eq!(unread, "");

        // A directory name carrying a newline is named on one line, and the
        // page it heads is not a refusal (D-0696). Raw, the name put
        // `refused: forged/...` at column zero, as the review measured.
        let (page, surface, _) = head(
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

        // A SYMBOL directory carrying a newline names no instrument --
        // `Symbol::new` admits no control character -- so the surface skips
        // it without a word, as it skips every word that resolves to nothing,
        // and the page is the opening over the instrument beside it and
        // nothing else.
        let (page, surface, _) = head(
            "page-forged-symbol",
            &["NSE/CASH/RELIANCE", "NSE/CASH/X\nrefused: forged"],
        );
        assert_eq!(surface, vec!["RELIANCE".to_owned()]);
        assert_eq!(page, opening(&["RELIANCE"]));
        assert!(!crate::carries_refusal(&page), "{page}");
    }

    /// **The pool page counts the `.bin` files at a month's depth its catalog cannot file under a
    /// feed or a rung, and its empty-surface line says what it found at the
    /// exact spelling.** D-0696.
    ///
    /// `store::catalog` compares the feed and rung directories exactly, so a
    /// month under `bars/Zerodha/...` or a `60MIN` directory was neither on the
    /// surface nor named, and the page said the store held no month at the
    /// path a load reads, although on a case-insensitive volume a load of
    /// `zerodha` opens `Zerodha` (found by a review). Here one month sits
    /// under `Zerodha` and one under another feed's `60MIN`, each beside
    /// `notes.bin`, whose stem is counted before it is parsed. No two names
    /// differ only in case under one parent, so the page is the same on
    /// either kind of filesystem.
    #[test]
    fn the_pool_page_counts_the_months_its_catalog_cannot_file() {
        let root =
            std::env::temp_dir().join(format!("brutex-pool-uncatalogued-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for rel in [
            "Zerodha/NSE/CASH/RELIANCE/60min/2026-07.bin",
            "dhan/NSE/INDEX/NIFTY/60MIN/2026-07.bin",
            "Zerodha/NSE/CASH/RELIANCE/60min/notes.bin",
            "dhan/NSE/INDEX/NIFTY/60MIN/notes.bin",
        ] {
            let at = root.join("bars").join(rel);
            std::fs::create_dir_all(at.parent().expect("a parent")).expect("dirs");
            std::fs::write(&at, b"").expect("a file the catalog counts");
        }
        let census = store::catalog::walk(&root).expect("walked").census;
        let head = super::head_under(&root, "zerodha", "60min", (2026, 7), (2026, 7), None);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(
            (census.unknown_vendor, census.unknown_rung, census.spot),
            (2, 2, 0),
            "premise: the catalog counts the unparsed stems too"
        );
        let (page, surface, unread) = head.expect("the head renders");
        assert!(surface.is_empty(), "{surface:?}");
        let mut block = String::new();
        super::not_catalogued(&mut block, (2, 2));
        assert_eq!(unread, block, "`unread` is the block alone");
        assert!(
            block.starts_with(
                "\n  NOT CATALOGUED: the store holds 2 .bin file(s) at a month's depth under a feed directory \
                 and 2 under a rung directory\n"
            ) && block.contains("On a case-insensitive volume"),
            "{block}"
        );
        assert_eq!(
            page,
            format!(
                "{}{block}  0 instruments on the surface for zerodha at 60min. The catalog \
                 finds no month of any swept index or F&O cash equity at the path its own \
                 load reads on this feed and rung, spelt exactly, so there is nothing to \
                 screen and nothing to pool. Nothing was read.\n",
                super::opening("zerodha", "60min", (2026, 7), (2026, 7), None, &[])
            ),
            "the opening, the count, then the empty-surface line"
        );
        assert!(!crate::carries_refusal(&page), "{page}");
        let mut quiet = String::new();
        super::not_catalogued(&mut quiet, (0, 0));
        assert!(quiet.is_empty(), "no block when every month was filed");
        for counts in [(1, 0), (0, 1)] {
            let mut one = String::new();
            super::not_catalogued(&mut one, counts);
            assert!(one.contains("NOT CATALOGUED"), "{counts:?}: {one}");
        }
    }

    /// **The pool page names what the catalog saw and offered to nobody.**
    /// D-0769.
    ///
    /// The surface read only `unknown_vendor` and `unknown_rung` from the
    /// census, so a linked symbol directory, which the catalog stops
    /// offering at D-0766, was absent from the page without a line (found by a
    /// review). A store with none of those entries gets no block.
    #[test]
    fn the_pool_page_names_the_entries_the_catalog_did_not_offer() {
        let root =
            std::env::temp_dir().join(format!("brutex-pool-unoffered-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let cash = root.join("bars/zerodha/NSE/CASH");
        std::fs::create_dir_all(cash.join("RELIANCE/60min")).expect("dirs");
        std::fs::write(cash.join("RELIANCE/60min/2026-07.bin"), b"").expect("a month");
        std::os::unix::fs::symlink(cash.join("RELIANCE"), cash.join("TCS")).expect("a link");
        let head = super::head_under(&root, "zerodha", "60min", (2026, 7), (2026, 7), None);
        let _ = std::fs::remove_dir_all(&root);
        let (page, _, unread) = head.expect("the head renders");
        let census = store::catalog::Census {
            seen: 2,
            spot: 1,
            linked: 1,
            ..store::catalog::Census::default()
        };
        let mut block = String::new();
        super::not_walked(&mut block, &census.unoffered_report());
        assert_eq!(unread, block, "`unread` is the block alone");
        assert!(
            block.starts_with(
                "\n  NOT OFFERED: below bars/ the catalog could not read 0 director(ies) or \
                 entr(ies), did not follow 1 symbolic link(s)"
            ) && block.ends_with(".\n"),
            "{block}"
        );
        assert!(page.contains(&block), "{page}");
        let mut quiet = String::new();
        super::not_walked(&mut quiet, "");
        assert!(quiet.is_empty(), "no block when nothing went unoffered");
    }

    #[test]
    fn a_pool_with_only_misfiled_holdings_refuses_and_keeps_their_reasons() {
        let root = store_holding(
            "all-misfiled-refusal",
            &["NSE/INDEX/RELIANCE", "BSE/CASH/RELIANCE"],
        );
        let why = super::run_under(
            &root,
            brutex_core::vendor::Vendor::Zerodha,
            "zerodha",
            "60min",
            (2026, 6),
            (2026, 7),
            None,
        )
        .expect_err("a store with only misfiled holdings is refused");
        std::fs::remove_dir_all(root).expect("scratch removed");
        assert!(why.starts_with("no instrument is on the surface"), "{why}");
        assert!(
            why.contains("NOT ON THE SURFACE: 2 held director(ies)"),
            "{why}"
        );
        for dir in ["NSE/INDEX/RELIANCE", "BSE/CASH/RELIANCE"] {
            assert!(why.contains(&named(dir, "RELIANCE")), "{why}");
        }
        assert!(!why.contains(crate::STORED_PROVENANCE), "{why}");
    }

    /// **The page `pool` prints IS its head, and every later line is
    /// appended.** D-0696.
    ///
    /// The test above drives `head_under`, and `run` -- which no test then
    /// drove -- was what put that head on the page. The review replaced
    /// `run`'s `out` with the bare opening after `head_under` returned,
    /// dropping the NOT ON THE SURFACE block from the real page, and every
    /// test stayed green. Everything after the stamp, feed, rung and root
    /// checks is now `run_under`, which this drives on a scratch store whose
    /// surface is empty, where it returns after the head. A surface with an
    /// instrument on it is screened through `one_rung`, which reads the root
    /// from the environment, so no in-process test drives that path.
    /// `the_pool_verb_prints_its_whole_page_on_a_generated_store` drives it
    /// from a child process, in a stamped build only, and here it is held by
    /// the shape of the source in every build: `out` is bound from
    /// `head_under` once, the page writes neither the opening nor the block
    /// itself, every `Ok(` and every `return` after the head returns `out`,
    /// and the body's last value is `Ok(out)`.
    ///
    /// **Every mention of `out` after the head is an append or the return.**
    /// A list of refused rewrites missed one it did not name --
    /// `std::mem::replace(&mut out, String::new())` stayed green (measured by
    /// the review, D-0696) -- so each mention of the name is checked against
    /// three shapes instead: `writeln!(out, ..)`, `&mut out` handed to
    /// `render_per_symbol` or `render_pooled`, and `Ok(out)`. A rewrite, an
    /// alias or a closure over the page has to name it, and any other shape
    /// fails. What the two renderers do with the page is held by
    /// `the_renderers_only_append_to_the_page_they_are_handed`, on one input
    /// each.
    #[test]
    #[expect(
        clippy::too_many_lines,
        reason = "one reading of `run_under`'s body in order, from the head it binds to its \
                  return, the refusal before pass 1's table among them; split, each half \
                  would have to find the body again"
    )]
    fn the_pool_page_is_its_head_and_then_only_appends() {
        let root = store_holding("run-under", &[]);
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
        let (head, surface, _) =
            super::head_under(&root, "zerodha", "60min", (2026, 7), (2026, 7), None)
                .expect("the head renders");
        let _ = std::fs::remove_dir_all(&root);
        assert!(
            surface.is_empty(),
            "premise: nothing to screen: {surface:?}"
        );
        assert!(
            !head.contains("NOT ON THE SURFACE"),
            "a fresh store: {head}"
        );
        assert_eq!(page, head, "the page of an empty surface is its head");
        // A fresh store remains a page; the all-misfiled store is tested separately.
        assert!(!crate::carries_refusal(&page), "{page}");

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
        let bound = "let (mut out, surface, unread) = head_under(root, vendor_word, rung, from, to, support_ppm)?;";
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
        // EVERY RETURN IS THE PAGE. The list above refuses rewriting `out`;
        // it did not refuse returning something else. The review returned a
        // fresh "nothing to pool" string from the empty-union branch, and the
        // 69 tests in its set passed.
        assert!(
            after.matches("Ok(").count() >= 2,
            "premise: both of `run_under`'s later returns were read:\n{after}"
        );
        assert_eq!(
            after.matches("Ok(").count(),
            after.matches("Ok(out)").count(),
            "every `Ok(` after the head returns `out` itself"
        );
        assert_eq!(
            after.matches("return").count(),
            after.matches("return Ok(out);").count(),
            "every early return after the head returns `out` itself"
        );
        assert!(
            after.trim_end().ends_with("\n    Ok(out)"),
            "the page is the body's last value"
        );
        // EVERY MENTION OF THE PAGE IS AN APPEND OR THE RETURN. The list above
        // names rewrites, and `std::mem::replace(&mut out, String::new())` was
        // not among them.
        let (writes, renders, returns) = mentions_of_the_page(after);
        assert!(
            writes > 0 && renders == 2 && returns >= 2,
            "premise: the scan read the page's appends and returns \
             ({writes} writes, {renders} renders, {returns} returns)"
        );
        // A POOL WHOSE EVERY INSTRUMENT REFUSED IS REFUSED, before pass 1's
        // table is rendered (D-0696). The driven test sees it only in a
        // stamped build, so its place is read here in every build.
        assert_eq!(
            after
                .matches(
                    "\n    at_least_one_screened(&screened, &unread)?;\n    \
                     render_per_symbol(&mut out, &screened);\n"
                )
                .count(),
            1,
            "pass 1's refusals are checked once, right before its table"
        );
    }

    /// **A pool whose every instrument refused in pass 1 is refused, naming
    /// every instrument's reason and what the head did not read.** D-0696.
    ///
    /// `range_over` refuses when every rung refuses. The pool printed its
    /// banner, a `REFUSED` row per instrument and "nothing to pool", and exited
    /// OK (found by a review). One instrument screened is enough for a page,
    /// and an empty surface is the head's own case, not this refusal. The
    /// refusal said "nothing was screened" and kept the first instrument's
    /// reason alone, so a second instrument's -- here a result that was not
    /// recorded -- and the head's blocks were dropped (found by a later
    /// review).
    #[test]
    fn a_pool_whose_every_instrument_refused_is_refused() {
        let refused = |symbol: &str, why: &str| super::Screened {
            symbol: symbol.to_owned(),
            outcome: Err(why.to_owned()),
        };
        let screened = |symbol: &str| super::Screened {
            symbol: symbol.to_owned(),
            outcome: Ok(crate::results::Record::from_bytes(
                &[0; crate::results::STRIDE_BYTES],
            )),
        };
        let unread = "\n  NOT ON THE SURFACE: 1 held director(ies) ...\n    `BSE/CASH/RELIANCE`\n\n  NOT CATALOGUED: 2 .bin files\n";
        let why = super::at_least_one_screened(
            &[
                refused("NIFTY", "its month could not be read"),
                refused("RELIANCE", "the result was not recorded: no ledger"),
                refused("BANKNIFTY", "REFUSED -- the streamed ladder halted"),
            ],
            unread,
        )
        .expect_err("every instrument refused");
        assert_eq!(
            why,
            "none of the 3 instrument(s) on the surface came through pass 1 with a result, \
             so nothing can be pooled. No completed result could be confirmed. Each \
             instrument's own reason is below.\n  NIFTY: its month could not be read\n  \
             RELIANCE: the result was not recorded: no ledger\n  BANKNIFTY: REFUSED -- the streamed ladder halted\n\n  NOT ON THE SURFACE: 1 \
             held director(ies) ...\n    `BSE/CASH/RELIANCE`\n\n  NOT CATALOGUED: 2 .bin files"
        );
        assert!(!why.contains("nothing was screened"), "{why}");
        assert!(crate::carries_refusal(&format!("refused: {why}\n")));
        for one_screened in [
            vec![refused("NIFTY", "no bar"), screened("RELIANCE")],
            vec![screened("NIFTY"), refused("RELIANCE", "no bar")],
            vec![screened("NIFTY")],
            Vec::new(),
        ] {
            assert_eq!(super::at_least_one_screened(&one_screened, unread), Ok(()));
        }
    }

    /// **`pool` refuses a month off the calendar and a span that runs
    /// backwards, before it reads the store.** D-0696.
    ///
    /// Month 13 parsed as a `u8` and nothing checked the span, so each
    /// instrument refused them on its own, and a store with no instrument on
    /// the surface printed its page over them and exited OK (found by a
    /// review). The arm refuses both before `pool::pool` runs.
    #[test]
    fn the_pool_arm_refuses_a_month_off_the_calendar_and_a_backwards_span() {
        for (from, to, why) in [
            (("2026", "13"), ("2026", "13"), "MONTH must be 1..=12"),
            (("2026", "0"), ("2026", "7"), "MONTH must be 1..=12"),
            (("2026", "7"), ("2027", "13"), "MONTH must be 1..=12"),
            (
                ("2026", "7"),
                ("2026", "6"),
                "the range runs backwards: 2026-07 is after 2026-06. Give FROM first and \
                 TO second. Nothing was read.",
            ),
            (
                ("2027", "1"),
                ("2026", "12"),
                "the range runs backwards: 2027-01 is after 2026-12. Give FROM first and \
                 TO second. Nothing was read.",
            ),
        ] {
            let mut page = String::new();
            let status = crate::pool_arm(&mut page, "zerodha", "60min", from, to, "auto");
            assert_eq!(status, crate::MISUSED, "{page}");
            assert!(
                page.starts_with(&format!("refused: {why}\n")),
                "{from:?}..{to:?}: {page}"
            );
            assert!(!page.contains(crate::STORED_PROVENANCE), "{page}");
        }
    }

    /// How often `after` -- `run_under`'s body past the head -- names the
    /// page `out` to append to it, to hand it to a renderer, and to return
    /// it. Any other mention of the name fails. A mention inside a longer
    /// word -- `outcome` -- is not the name.
    fn mentions_of_the_page(after: &str) -> (usize, usize, usize) {
        let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let (mut writes, mut renders, mut returns) = (0_usize, 0_usize, 0_usize);
        for (at, _) in after.match_indices("out") {
            let lead = after.get(..at).unwrap_or_default();
            let tail = after.get(at + "out".len()..).unwrap_or_default();
            if lead.ends_with(ident) || tail.starts_with(ident) {
                continue;
            }
            let lead = lead.trim_end();
            let write = lead.ends_with("writeln!(") && tail.starts_with(',');
            let render = (lead.ends_with("render_per_symbol(&mut")
                || lead.ends_with("render_pooled(&mut"))
                && tail.starts_with(',');
            let returned = lead.ends_with("Ok(") && tail.starts_with(')');
            assert!(
                write || render || returned,
                "`{} out` after the head neither appends to the page nor returns it",
                lead.rsplit('\n').next().unwrap_or_default().trim_start()
            );
            writes += usize::from(write);
            renders += usize::from(render);
            returns += usize::from(returned);
        }
        (writes, renders, returns)
    }

    /// **Each renderer `run_under` hands the page to only appends to it.**
    /// D-0696.
    ///
    /// `the_pool_page_is_its_head_and_then_only_appends` admits `&mut out`
    /// passed to [`super::render_per_symbol`] and [`super::render_pooled`], so
    /// what those two do with a page that already holds its head is part of
    /// the claim. Each is handed a page and must leave it as the prefix of
    /// what it returns. One input each, a refused row and a priced one: the
    /// renderers are driven here, not read from their source.
    #[test]
    fn the_renderers_only_append_to_the_page_they_are_handed() {
        let head = "HEAD, AS `head_under` WROTE IT\n";

        let mut out = head.to_owned();
        super::render_per_symbol(
            &mut out,
            &[super::Screened {
                symbol: "AAA".to_owned(),
                outcome: Err("refused for this test".to_owned()),
            }],
        );
        assert!(
            out.starts_with(head) && out.contains("refused for this test"),
            "pass 1 keeps the head and appends its rows:\n{out}"
        );

        let union = candidates(1);
        let surface = vec!["AAA".to_owned(), "BBB".to_owned()];
        let priced = vec![
            Ok(vec![Some(cell(3, 1, 900, -100, 1_000, 150))]),
            Err("not priced for this test".to_owned()),
        ];
        let rules = rules_at(300);
        let pooled = fold(&union, &surface, &priced, rules);
        let mut out = head.to_owned();
        super::render_pooled(&mut out, &union, &surface, &priced, &pooled, rules);
        assert!(
            out.starts_with(head)
                && out.contains("PASS 2 -- POOLED")
                && out.contains("not priced for this test"),
            "pass 2 keeps the head and appends its table:\n{out}"
        );
    }

    /// **`run` hands on `run_under`'s page untouched.** D-0696.
    ///
    /// No test drove `run` when the review made it return the bare opening in
    /// place of `run_under`'s page, and every test passed.
    /// `the_pool_verb_prints_its_whole_page_on_a_generated_store` now drives
    /// it, but only in a stamped build: `run` checks the commit stamp before
    /// anything else, and a build of a tree that is not HEAD -- a mutation run
    /// among them -- refuses there. So its body is read here, in every build:
    /// the checks, and then `run_under`'s result as its tail and its only
    /// page. The tail is pinned exactly and `run_under` is called once; that
    /// nothing else in the body builds or returns a page is a list of refused
    /// phrasings, among them `Ok(` and `return`.
    #[test]
    fn the_pool_verb_hands_on_run_unders_page_untouched() {
        let source = include_str!("pool.rs");
        let from = source.find("\nfn run(").expect("run");
        let run = source
            .get(from..)
            .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to + 2)))
            .expect("its body");
        assert!(
            run.contains("crate::commit_stamp()") && run.contains("crate::store_root()?"),
            "premise: the body read is `run`'s:\n{run}"
        );
        assert!(
            run.ends_with(
                "\n    run_under(&root, vendor, vendor_word, rung, from, to, support_ppm)\n}"
            ),
            "`run` ends by returning `run_under`'s page as it is:\n{run}"
        );
        assert_eq!(
            run.matches("run_under(").count(),
            1,
            "`run` calls `run_under` once"
        );
        for other in [
            "Ok(",
            "return",
            "opening(",
            "head_under(",
            "not_on_the_surface(",
            "format!(",
            "String::",
            "writeln!(",
            "push_str(",
        ] {
            assert!(
                !run.contains(other),
                "`{other}` in `run`: its page is `run_under`'s alone"
            );
        }
    }

    /// **`pool` and its dispatch arm print `run`'s page as it is.** D-0696.
    ///
    /// The test above stops at `run`. Two links sit between it and what the
    /// binary prints: `pool` turns `run`'s result into text, and `lib.rs`'s
    /// `pool_arm` appends that text to the output. With `pool`'s
    /// `Ok(text) => text` returning an empty string, the whole `cli` library
    /// suite stayed green (measured by the review).
    /// `the_pool_verb_prints_its_whole_page_on_a_generated_store` catches that
    /// in a stamped build. A build of a tree that is not HEAD, which is what a
    /// mutation run builds, takes that test's stamp-refusal branch, and there
    /// `pool`'s `Ok` arm is never reached. So both bodies are read here as
    /// well, in every build: `pool` is pinned whole, `pool_arm`'s arm that runs
    /// the verb is pinned line for line, and every other mention of `out` in
    /// `pool_arm` is its parameter or a refusal.
    #[test]
    fn the_pool_verb_and_its_arm_print_runs_page_as_it_is() {
        let body_of = |source: &'static str, head: &str| {
            let from = source.find(head).expect("the function");
            source
                .get(from..)
                .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to + 2)))
                .expect("its body")
        };
        let pool = body_of(include_str!("pool.rs"), "\npub fn pool(");
        assert_eq!(
            pool,
            concat!(
                "\npub fn pool(\n",
                "    vendor_word: &str,\n",
                "    rung: &'static str,\n",
                "    from: (u16, u8),\n",
                "    to: (u16, u8),\n",
                "    support_ppm: Option<u64>,\n",
                ") -> String {\n",
                "    match run(vendor_word, rung, from, to, support_ppm) {\n",
                "        Ok(text) => text,\n",
                "        Err(why) => format!(\"refused: {why}\\n\"),\n",
                "    }\n",
                "}",
            ),
            "`pool` prints `run`'s page, or `run`'s refusal, and nothing else"
        );

        let arm = body_of(include_str!("lib.rs"), "\nfn pool_arm(");
        assert!(
            arm.contains(concat!(
                "        (Ok(fy), Ok(fm), Ok(ty), Ok(tm), Ok(h)) if (fy, fm) <= (ty, tm) => {\n",
                "            let text = pool::pool(vendor, known, (fy, fm), (ty, tm), h);\n",
                "            let refused = carries_refusal(&text);\n",
                "            out.push_str(&text);\n",
                "            if refused { MISUSED } else { OK }\n",
                "        }\n",
            )),
            "`pool_arm` appends `pool`'s page whole, and exits on what it says:\n{arm}"
        );
        assert_eq!(arm.matches("pool::pool(").count(), 1, "{arm}");
        // EVERY OTHER MENTION OF THE OUTPUT IS A REFUSAL. The arm above could
        // append the page and a later line clear it, so each mention of the
        // name is the parameter, the one append, or `refuse(out, ..)`.
        let ident = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let (mut appends, mut refusals) = (0_usize, 0_usize);
        for (at, _) in arm.match_indices("out") {
            let lead = arm.get(..at).unwrap_or_default();
            let tail = arm.get(at + "out".len()..).unwrap_or_default();
            if lead.ends_with(ident) || tail.starts_with(ident) {
                continue;
            }
            let parameter = tail.starts_with(": &mut String,");
            let append = tail.starts_with(".push_str(&text);");
            let refusal = lead.trim_end().ends_with("refuse(") && tail.starts_with(',');
            assert!(
                parameter || append || refusal,
                "`{}out{}` in `pool_arm` neither appends the page nor refuses",
                lead.rsplit('\n').next().unwrap_or_default().trim_start(),
                tail.split('\n').next().unwrap_or_default()
            );
            appends += usize::from(append);
            refusals += usize::from(refusal);
        }
        assert_eq!(appends, 1, "the page is appended once:\n{arm}");
        assert!(refusals >= 3, "premise: the refusals were read:\n{arm}");
    }

    /// **The `pool` verb prints its whole page, driven end to end on a
    /// generated store.** D-0696.
    ///
    /// Every test above drives one piece of the page (the head, `run_under`
    /// on an empty surface, each renderer) or reads the source. The reason
    /// given for driving no more, that no test build passes `run`'s
    /// commit-stamp check, was false: the build script stamps a tree equal to
    /// HEAD, and a clean checkout, CI's among them, is one. What keeps an
    /// in-process test off a surface with an instrument on it is the store
    /// root, which `run` and pass 1's `one_rung` read from the environment.
    /// So this test runs itself again as a child, as
    /// `public_generated_probe_and_screen_agree_with_durable_results` runs
    /// itself. The child inherits no `BRUTEX_` variable from the shell that
    /// ran the suite, and is given three: `BRUTEX_STORE`, naming a generated
    /// store, `BRUTEX_LOG_DIR`, inside that store, and
    /// `BRUTEX_TEST_POOL_VERB_PAGE`, the marker that makes it the child. The
    /// child dispatches `pool` through `crate::dispatch`, the path
    /// the binary takes through `pool_arm`, `pool`, `run` and `run_under`,
    /// twice:
    ///
    /// * at `5min` over the generated NIFTY month, at a fixed support, where
    ///   pass 1 screens NIFTY and pass 2 prices the union on it: the page is
    ///   `head_under`'s head, then pass 1's table with NIFTY's 600 bars, then
    ///   pass 2's table over that one instrument, ending with the in-sample
    ///   warning;
    /// * at `60min` over an unreadable `NSE/CASH/RELIANCE` month beside
    ///   `BSE/CASH/RELIANCE`, where the one instrument on the surface refuses
    ///   in pass 1: the verb refuses, naming RELIANCE and the month it could
    ///   not read, then the head's NOT ON THE SURFACE block naming the BSE
    ///   directory, and prints nothing else of the page. It printed the head,
    ///   pass 1's refused row and the nothing-to-pool line, and exited OK
    ///   (D-0696).
    ///
    /// The first exits OK and is not a refusal; the second exits MISUSED. An
    /// unstamped build refuses both before a bar is read, and there the child
    /// requires the stamp refusal and that nothing was recorded.
    #[test]
    fn the_pool_verb_prints_its_whole_page_on_a_generated_store()
    -> Result<(), Box<dyn std::error::Error>> {
        const CHILD: &str = "BRUTEX_TEST_POOL_VERB_PAGE";
        if std::env::var_os(CHILD).is_some() {
            return pool_pages_on(&crate::store_root()?);
        }
        crate::audited_stored::with_warmed_store(|root| {
            // RELIANCE's 60min month at its own path, unreadable, and the same
            // symbol under BSE, which the page names and does not read.
            for dir in ["NSE/CASH/RELIANCE", "BSE/CASH/RELIANCE"] {
                let at = root.join(format!("bars/zerodha/{dir}/60min"));
                std::fs::create_dir_all(&at)?;
                std::fs::write(at.join("2026-07.bin"), b"")?;
            }
            let mut child = std::process::Command::new(std::env::current_exe()?);
            child.args([
                "--exact",
                "pool::tests::the_pool_verb_prints_its_whole_page_on_a_generated_store",
                "--nocapture",
                "--test-threads=1",
            ]);
            // NO KNOB FROM THE SHELL THAT RAN THE SUITE: the child's pages
            // depend on the generated store alone.
            for (name, _) in std::env::vars_os() {
                if name.to_string_lossy().starts_with("BRUTEX_") {
                    child.env_remove(name);
                }
            }
            let child = child
                .env(CHILD, "generated")
                .env("BRUTEX_STORE", root)
                .env("BRUTEX_LOG_DIR", root.join("logs"))
                .output()?;
            assert!(
                child.status.success(),
                "{}{}",
                String::from_utf8_lossy(&child.stdout),
                String::from_utf8_lossy(&child.stderr)
            );
            assert!(String::from_utf8_lossy(&child.stdout).contains("1 passed"));
            Ok(())
        })
    }

    /// The child of `the_pool_verb_prints_its_whole_page_on_a_generated_store`,
    /// on the generated store its environment names.
    fn pool_pages_on(root: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        // THE HEADS, BEFORE EITHER RUN: the head is what the page says before
        // a bar is read.
        let priced_head =
            super::head_under(root, "zerodha", "5min", (2025, 5), (2025, 5), Some(200_000))?;
        let empty_head = super::head_under(root, "zerodha", "60min", (2026, 7), (2026, 7), None)?;
        let dispatch = |words: &[&str]| {
            let args: Vec<String> = words.iter().map(ToString::to_string).collect();
            let mut page = String::new();
            let status = crate::dispatch(&args, &mut page);
            (status, page)
        };
        let priced = dispatch(&[
            "pool", "zerodha", "5min", "2025", "5", "2025", "5", "200000",
        ]);
        let emptied = dispatch(&["pool", "zerodha", "60min", "2026", "7", "2026", "7", "auto"]);
        if crate::commit_stamp().is_none() {
            for (status, page) in [&priced, &emptied] {
                assert_eq!(*status, crate::MISUSED, "{page}");
                assert!(
                    page.starts_with("refused: this build carries no verified commit stamp"),
                    "{page}"
                );
                assert!(!page.contains(crate::STORED_PROVENANCE), "{page}");
            }
            assert!(
                !crate::results::Results::path(root).exists(),
                "an unstamped pool records nothing"
            );
            return Ok(());
        }
        both_passes_over_nifty(&priced_head, priced.0, &priced.1)?;
        every_instrument_refused_beside_bse(&empty_head, emptied.0, &emptied.1)
    }

    /// BOTH PASSES, over the generated NIFTY month: the head, pass 1's table
    /// with NIFTY on its 600 bars, and pass 2's table over that instrument.
    fn both_passes_over_nifty(
        (head, surface, _): &(String, Vec<String>, String),
        status: u8,
        page: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(surface, &vec!["NIFTY".to_owned()], "premise: {head}");
        assert!(
            page.starts_with(head.as_str()),
            "the page opens with `head_under`'s head:\n{page}"
        );
        let rest = page.get(head.len()..).unwrap_or_default();
        let (pass_1, pass_2) = rest
            .split_once("\n  PASS 2 -- POOLED across 1 instrument(s), ")
            .ok_or_else(|| format!("pass 2 pools the one instrument pass 1 screened:\n{rest}"))?;
        assert!(
            pass_1.starts_with("\n  PASS 1 -- PER SYMBOL, each on its own bars\n"),
            "pass 1 follows the head:\n{rest}"
        );
        assert!(
            pass_1
                .lines()
                .any(|line| line.starts_with(&format!("  {:<14}{:>9}", "NIFTY", 600))),
            "pass 1 screened NIFTY's 600 generated five-minute bars:\n{pass_1}"
        );
        assert!(
            !pass_1.contains("REFUSED") && !pass_1.contains("NOT READ"),
            "pass 1 refused nothing and its frontier was read:\n{pass_1}"
        );
        assert!(
            !pass_2.contains("NOT PRICED"),
            "pass 2 priced NIFTY:\n{pass_2}"
        );
        assert!(
            pass_2.ends_with(crate::IN_SAMPLE_WARNING),
            "the page ends with pass 2's table and its warning:\n{rest}"
        );
        assert!(!crate::carries_refusal(page), "{page}");
        assert_eq!(status, crate::OK, "{page}");
        Ok(())
    }

    /// EVERY INSTRUMENT REFUSED, over one unreadable month beside a BSE
    /// directory: the verb refuses, naming RELIANCE and the month it could
    /// not read, then the head's block naming the BSE directory, and prints
    /// nothing else of the page that head would have opened.
    fn every_instrument_refused_beside_bse(
        (head, surface, unread): &(String, Vec<String>, String),
        status: u8,
        page: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(surface, &vec!["RELIANCE".to_owned()], "premise: {head}");
        assert!(
            head.contains(&named("BSE/CASH/RELIANCE", "RELIANCE")),
            "premise: the head names the BSE directory:\n{head}"
        );
        let why = page
            .strip_prefix(
                "refused: none of the 1 instrument(s) on the surface came through pass 1 \
                 with a result, so nothing can be pooled. No completed result could be \
                 confirmed. Each instrument's own reason is below.\n  RELIANCE: ",
            )
            .ok_or_else(|| format!("the verb refuses, naming RELIANCE:\n{page}"))?;
        let (reason, rest) = why
            .split_once('\n')
            .ok_or_else(|| format!("RELIANCE's reason is one line:\n{page}"))?;
        assert!(
            reason.contains("no header slot survived"),
            "premise: pass 1 read the unreadable month and refused it for that: {why}"
        );
        assert_eq!(
            rest,
            format!("{}\n", unread.trim_end()),
            "then the head's block naming the BSE directory, and nothing else:\n{page}"
        );
        assert!(
            unread.contains(&named("BSE/CASH/RELIANCE", "RELIANCE")),
            "premise: {unread}"
        );
        assert!(
            !page.contains(crate::STORED_PROVENANCE),
            "no banner over nothing:\n{page}"
        );
        assert!(crate::carries_refusal(page), "{page}");
        assert_eq!(
            status,
            crate::MISUSED,
            "a pool whose every instrument refused is a refusal of the pool"
        );
        Ok(())
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

    /// D-1420: the pass-1 table at the extremes a row can carry. Raw paisa at
    /// `i64::MIN` is 20 characters in a 12-character column, so before the
    /// columns were laid out together `worst`, `net` and `max_dd` ran into one
    /// another and every column after them left its header.
    #[test]
    #[allow(clippy::indexing_slicing, reason = "a missing line must fail the test")]
    fn the_per_symbol_table_keeps_extreme_figures_apart_and_under_their_headers() {
        use crate::columns::Align::{Left as L, Right as R};
        let record = |extreme: i64, count: u64| crate::results::Record {
            bars: count,
            min_hits: count,
            depth: u32::MAX,
            trades: count,
            pessimistic: extreme,
            worst_trade: extreme,
            max_drawdown: extreme,
            ..crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES])
        };
        let screened = [
            super::Screened {
                symbol: "WAAREEENERGY_X".to_owned(),
                outcome: Ok(record(i64::MIN, u64::MAX)),
            },
            super::Screened {
                symbol: "NIFTY".to_owned(),
                outcome: Ok(record(i64::MAX, 0)),
            },
            super::Screened {
                symbol: "TORNTPHARM".to_owned(),
                outcome: Ok(record(-1, 1)),
            },
            super::Screened {
                symbol: "SIXTEEN_CHARS_XY".to_owned(),
                outcome: Err("its month could not be read".to_owned()),
            },
        ];
        let mut out = String::new();
        super::render_per_symbol(&mut out, &screened);
        let lines: Vec<&str> = out.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.trim_start().starts_with("symbol"))
            .expect("a header");
        for row in &lines[at + 1..] {
            if let Some(refused) = row.find("REFUSED:") {
                assert!(
                    row[..refused].ends_with(' '),
                    "a refusal touches its symbol: {row}"
                );
                continue;
            }
            crate::columns::assert_under(lines[at], row, &[L, R, R, R, R, R, R, R, R])
                .expect("separated and aligned");
        }
        assert_eq!(lines.len() - at - 1, screened.len(), "{out}");
    }

    /// v4-2, GAP13-16, D-1487: the pooled pass-2 table at the extremes a row
    /// can carry. Raw paisa at `i64::MIN` is 20 characters in a 12-character
    /// column; before the table was laid out together `worst`, `min_win`,
    /// `net` and `dd>=` ran into one another and slid off their headers.
    #[test]
    #[allow(clippy::indexing_slicing, reason = "a missing line must fail the test")]
    fn the_pooled_table_keeps_extreme_figures_apart_and_under_their_headers() {
        use crate::columns::Align::{Left as L, Right as R};
        let union = candidates(3);
        let pooled_at = |candidate: usize, extreme: i64, count: u64| Pooled {
            candidate,
            fired: count,
            trades: count,
            wins: count,
            net: extreme,
            // `worst` and `gross_loss` at `i64::MIN` keep both ratios finite,
            // so every cell is one word; `min_win` and `gross_win` are never
            // negative in a real fold.
            worst: i64::MIN,
            min_win: extreme.max(0),
            gross_win: extreme.max(0),
            gross_loss: i64::MIN,
            dd_bound: extreme,
            names: vec!["AAA".to_owned()],
        };
        let pooled = vec![
            pooled_at(0, i64::MIN, u64::MAX),
            pooled_at(1, i64::MAX, u64::MAX),
            pooled_at(2, -1, 1),
            pooled_at(7, 0, 1),
        ];
        let mut rules = rules_at(300);
        rules.top = pooled.len();
        let surface = vec!["AAA".to_owned()];
        let priced: Vec<Result<Vec<super::Priced>, String>> = vec![Ok(Vec::new())];
        let mut out = String::new();
        super::render_pooled(&mut out, &union, &surface, &priced, &pooled, rules);
        let lines: Vec<&str> = out.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.trim_start().starts_with("rank"))
            .expect("a header");
        let header = lines[at]
            .strip_suffix("  fired on")
            .expect("the trailing label");
        let mut rows = 0;
        for row in &lines[at + 1..] {
            if row.trim_start().starts_with("mask ") || row.is_empty() {
                continue;
            }
            if row.starts_with("refused: pooled rank 4 names missing candidate 7") {
                rows += 1;
                continue;
            }
            let Some(row) = row
                .strip_suffix("  AAA +18446744073709551614 more")
                .or_else(|| row.strip_suffix("  AAA"))
            else {
                break;
            };
            crate::columns::assert_under(header, row, &[R, L, R, R, R, R, R, R, R, R, R])
                .expect("separated and aligned");
            rows += 1;
        }
        assert_eq!(rows, pooled.len(), "{out}");
        assert!(out.contains(&i64::MIN.to_string()), "{out}");
    }
}
