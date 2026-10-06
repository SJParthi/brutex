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
//! P&L. The `dd1max` column is therefore the LARGEST single-instrument
//! drawdown among those pooled, and it bounds the pooled figure in NEITHER
//! direction: A losing 100, then B winning 150, then A losing 100 again shows
//! 200 here while the merged sequence's drawdown is 100, and two instruments
//! losing at once can make the pooled drawdown larger than either's. It was
//! called a lower bound and labelled `dd>=`, which was false (p2misc-1,
//! D-2648).
//! Exposing per-trade P&L from the grid is the change that would make it
//! exact, and `docs/06-limits.md` records it as not done.
//!
//! # What is NOT charged, and why that is stated on every report
//!
//! No cost of any kind. On the indices that is correct by charter — an index
//! is not tradeable. On a cash equity it is NOT correct: brokerage, STT, stamp
//! duty, exchange charges, the SEBI fee, the IPFT, DP charges and GST are real
//! (an UNVERIFIED list, D-1779), and no rate for any of
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
//! Pass 1 is `instruments` screens, one at a time, each what `range-rung`
//! costs; each screen's sweep and pricing are parallel inside it.
//!
//! The union is built from ONE admission of the parent ledger and receipt
//! sidecar, O(L + R) for L ledger rows and R receipts, then one frontier-block
//! read per screened instrument, O(its rows), and one `HashSet` insert per
//! frontier row -- O(1) expected (W2-cli9-0, D-1703). It used to re-admit both
//! parent files once per instrument, O(I × (L + R)), while this header said
//! nothing here scanned the store.
//!
//! Pass 2 is, per instrument, the loads, the column, the projection onto the
//! one-minute execution series and one `SliceFacts`, `O(B_sig + B_exec)`;
//! then per union candidate one `grid::evaluate_over`, which walks EVERY row
//! of the projected column before it prices: `Θ(B_exec + cells × T)`. So
//! pass 2 is `Θ(I × (B_sig + B_exec) + I × U × (B_exec + cells × T))`, and
//! because the union U is the union of every instrument's kept frontier, U
//! grows with I (up to I × `top`): up to `Θ(I² × top × B_exec)` for the rare
//! setups this pool exists to find, where T is small and `B_exec` is large.
//! Until D-1702 this said "each the cost of one exit grid over that
//! instrument's trades",
//! which left out the `B_exec` walk (R9-cli-o1-1). None of these is a rule-4
//! operation: those bound the per-bar and per-candidate primitives INSIDE the
//! screen, which are unchanged. The pooled fold is one pass over
//! `instruments × |union|` cells with O(1) work each, and the catalog walk
//! that lists the surface is paid once. `docs/06-limits.md` states all of it.
//!
//! # Parallelism and reproducibility
//!
//! Pass 1 runs its instruments one at a time in sorted symbol order, so its
//! ledger rows and attempt tokens are written in that order (D-1701). Pass 2 is
//! `par_iter` over INSTRUMENTS with indexed `collect` and writes nothing, so
//! the order of every table is the sorted symbol order and never the
//! scheduler's. Pass 1's per-instrument runs are the same runs `range-rung`
//! makes, so their identities and rows are byte-identical to running each by
//! hand. The pooled fold runs sequentially over the collected cells. §3 rule
//! 5.

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
/// "the rule" is. Zero — the floor OFF — marks every fired row that WON
/// somewhere as meeting it, which is what OFF means; a row that never won meets
/// no rule, exactly as `grid::Cell::clears` refuses a cell with no winner
/// (p5num-3, D-2713).
fn tail_rule_bp(rules: crate::Rules) -> i64 {
    rules.min_rr_bp
}

/// One instrument's pass-1 outcome: the run `range-rung` would have made.
pub(crate) struct Screened {
    pub(crate) symbol: String,
    pub(crate) outcome: Result<crate::results::Record, String>,
}

/// One `(combination, side)` the union holds, in first-seen order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct Candidate {
    pub(crate) words: [u64; 6],
    pub(crate) direction: Direction,
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
    /// The three money totals are `i128`, summed exactly. They were `i64`
    /// `saturating_add`s, so a pooled sum past either end was printed and
    /// ranked as `i64::MAX` or `i64::MIN` as if it were the real total
    /// (h-cli-3). Each addend is one instrument's `i64` cell, and at most
    /// `usize::MAX` of them cannot leave `i128`'s range (D-1852).
    net: i128,
    worst: i64,
    min_win: i64,
    gross_win: i128,
    gross_loss: i128,
    /// The largest single-instrument drawdown among those pooled. NOT a
    /// bound on the pooled drawdown in either direction — see the module
    /// documentation (p2misc-1, D-2648).
    dd_bound: i64,
    /// Up to the first few symbols it fired on, for the row.
    names: Vec<String>,
}

impl Pooled {
    /// Gross wins over gross losses, in hundredths. [`NEVER_LOST`] when
    /// nothing was lost, which is a fact and is demoted by [`ranked`] exactly
    /// as a cell's is by [`crate::ranked`].
    ///
    /// In `i128` from the exact `i128` totals (D-1852). The `saturating_mul`
    /// is unreachable short of pooling some 10^17 instruments: `gross_win` is
    /// at most that many `i64::MAX` cells, and `i128::MAX / 100` is about
    /// 1.8 x 10^17 of them.
    const fn profit_factor_bp(&self) -> i128 {
        if self.gross_loss == 0 {
            return NEVER_LOST;
        }
        // `gross_loss` is negative or zero; the magnitude is what divides.
        // HUNDREDTHS, as `grid::Cell::profit_factor_bp` is: 125 is 1.25x.
        self.gross_win.saturating_mul(100) / self.gross_loss.saturating_abs()
    }

    /// The smallest win over the largest loss, in hundredths — the operator's
    /// own rule, `min(win) >= k × max(loss)`, as a ratio. [`NEVER_LOST`] when
    /// nothing was lost.
    ///
    /// In `i128` so `min_win × 100` is exact for every `i64` win: it was an
    /// `i64` `saturating_mul`, which clamped a win above `i64::MAX / 100`
    /// (D-1852). `i64::MAX × 100` is far inside `i128`, so this is exact.
    fn tail_bp(&self) -> i128 {
        if self.worst == 0 {
            return NEVER_LOST;
        }
        // HUNDREDTHS, as `grid::Cell::reward_to_risk_bp` is, so it compares
        // directly with `Rules::min_rr_bp`.
        i128::from(self.min_win) * 100 / i128::from(self.worst).abs()
    }

    /// Whether the tail rule holds at the operator's multiple.
    ///
    /// `wins > 0` is its own clause for the reason `grid::Cell::clears` gives:
    /// a candidate whose every pooled trade was flat has `worst == 0`, so
    /// [`Self::tail_bp`] is [`NEVER_LOST`] and cleared every multiple, and the
    /// row sorted first reading "wins 0, tail never lost". No winners is never
    /// what an operator means by a met rule (p5num-3, D-2713).
    fn meets(&self, rule_bp: i64) -> bool {
        self.fired > 0 && self.wins > 0 && self.tail_bp() >= i128::from(rule_bp)
    }

    /// The tail as the table prints it: `-` when nothing won, because a
    /// smallest win over a largest loss with no win has no numerator, and
    /// "never lost" on a row with no winner reads as the best tail there is
    /// (p5num-3, D-2713).
    fn tail_cell(&self) -> String {
        if self.wins == 0 {
            "-".to_owned()
        } else {
            ratio_cell(self.tail_bp())
        }
    }

    /// The sort key, largest first: the rule met, then the SMALLEST largest
    /// single-instrument drawdown (a ranking key, not a bound on the pooled
    /// drawdown; p2misc-1, D-2648), then the profit factor with its never-lost sentinel demoted, then
    /// the net. The drawdown leads because the objective is "very very less max
    /// drawdown" before it is anything else.
    fn key(&self, rule_bp: i64) -> (bool, i64, i128, i128) {
        (
            self.meets(rule_bp),
            self.dd_bound.saturating_neg(),
            ranked(self.profit_factor_bp()),
            self.net,
        )
    }
}

/// The never-lost sentinel of a pooled ratio. No finite ratio reaches it: the
/// largest is `i64::MAX × 100` for the tail and, for the profit factor, the
/// unreachable saturation [`Pooled::profit_factor_bp`] names.
const NEVER_LOST: i128 = i128::MAX;

/// [`crate::ranked`] for a pooled ratio: the never-lost sentinel sorts last.
const fn ranked(ratio: i128) -> i128 {
    if ratio == NEVER_LOST {
        i128::MIN
    } else {
        ratio
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
    // ONE AT A TIME, IN SURFACE ORDER, as `sweep_rungs` runs rungs. This was a
    // rayon parallel map over the surface, which wrote every instrument's ledger row and
    // attempts in thread-completion order (GAP13-13) and ran up to the rayon
    // pool's width of sweeps at once while nothing raised
    // `SWEEPS_SHARING_THIS_MACHINE`, so each concurrent sweep took the whole
    // machine's ceiling and every core (R9-cli-o1-0). With one sweep in flight
    // the counter's 1 is the truth, and each sweep's own support lanes and
    // pricing still use every core. D-1701.
    let screened: Vec<Screened> = crate::in_input_order(&surface, |symbol| Screened {
        symbol: symbol.clone(),
        outcome: crate::one_rung(vendor_word, symbol, rung, from, to, support_ppm, None).outcome,
    });
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
pub(crate) fn head_under(
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
pub(crate) fn count(n: usize) -> u64 {
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
pub(crate) fn at_least_one_screened(screened: &[Screened], unread: &str) -> Result<(), String> {
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
     cash equity, where brokerage, STT, stamp duty, exchange charges, the SEBI fee, the\n\
     IPFT, DP charges and GST (an UNVERIFIED list) all apply and none is subtracted: every equity total is GROSS OF EVERY CHARGE.\n\
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
    let mut out = String::from(crate::STORED_POOLED_PROVENANCE);
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

pub(crate) fn render_per_symbol(out: &mut String, screened: &[Screened]) {
    use crate::columns::{left, right};
    let _ = writeln!(out, "\n  PASS 1 -- PER SYMBOL, each on its own bars");
    // SORTED BY THE MONEY, not by name: the smallest drawdown first, then the
    // worst trade closest to zero, then the net. Refusals sort last and are
    // named, never dropped.
    //
    // TWO PARTS, EXPLICITLY. This was one ascending sort over a mixed key with
    // refusals keyed `(true, ..)`, then `reverse()` -- which put every refusal
    // FIRST, the opposite of the sentence above (W2-cli9-7, D-1704). The Ok
    // rows are now sorted descending on the money key and the refusals are
    // appended after them in their input (symbol) order. Ties among Ok rows keep
    // input order too: `sort_by_key` is stable and `Reverse` does not reorder
    // equal keys.
    let mut ranked: Vec<(&Screened, &crate::results::Record)> = screened
        .iter()
        .filter_map(|s| s.outcome.as_ref().ok().map(|r| (s, r)))
        .collect();
    ranked.sort_by_key(|(_, r)| {
        core::cmp::Reverse((
            r.max_drawdown.saturating_neg(),
            r.worst_trade,
            r.pessimistic,
        ))
    });
    let mut rows: Vec<&Screened> = ranked.into_iter().map(|(s, _)| s).collect();
    rows.extend(screened.iter().filter(|s| s.outcome.is_err()));
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
        left(64).after(2),
    ];
    // THE IDENTITY EACH ROW WAS RECORDED UNDER, as its last column, so a
    // pass-1 figure can be traced to its run without leaving the page (§3 rule
    // 3). The page opens with `STORED_POOLED_PROVENANCE`, which promises
    // exactly this: identities printed beside the rows they produced.
    // R9-cli-law-3, D-1705.
    let header = [
        "symbol", "bars", "min_hits", "depth", "trades", "worst", "net", "max_dd", "ret/DD",
        "identity",
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
                r.identity_hex(),
            ],
        });
    }
    let laid = crate::columns::with_header(&columns, &header, cells);
    let _ = writeln!(out, "  {}", laid.header.trim_end());
    for (s, line) in rows.iter().zip(&laid.rows) {
        match &s.outcome {
            Err(why) => {
                let _ = writeln!(out, "  {line}REFUSED: {why}");
            }
            Ok(_) => {
                let _ = writeln!(out, "  {}", line.trim_end());
            }
        }
    }
}

/// The union of every screened instrument's frontier rows as `(mask, side)`.
///
/// One `HashSet` insert per row decides membership; the `Vec` keeps first-seen
/// order so the table is stable across runs. Instruments whose rows cannot be
/// read are returned by name with the reason rather than skipped.
///
/// # One parent snapshot, not one per instrument
///
/// Each `Frontier::of_run` on a read-only handle proves its parent through
/// `result_set::committed_receipt`, which opens the results ledger and the
/// receipt sidecar afresh and indexes both in full: O(L + R) per call, so this
/// loop was O(I × (L + R)) while the module header said nothing here scanned
/// the store (W2-cli9-0, D-1703; the base fixed it too, sweep audit OS-4,
/// D-2301, and the base's code is the one kept, D-2105). The parents are now
/// admitted ONCE, through
/// [`crate::result_set::CommittedParents`], and each instrument's receipt is
/// one expected-O(1) probe of that snapshot followed by
/// `Frontier::of_run_against_receipt`, which applies the same commit and
/// count checks `of_run` applies. The snapshot is taken after pass 1 has
/// committed every row this union reads, so it holds them.
///
/// Cost: O(L + R) once, plus one frontier-block read, O(rows), per screened
/// instrument, plus one `HashSet` insert per row. The single admission is
/// proved by `crate::pool::tests::the_union_admits_the_parent_ledger_once_not_once_per_instrument`
/// (invariant L1A-05); the per-row terms are UNVERIFIED by any bench.
pub(crate) fn union_of(
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
    // THE PARENTS ARE OPENED ONCE. `Frontier::of_run` on a read-only handle
    // cold-opens the results ledger and the receipt sidecar on every call --
    // O(history) each -- so a pool over 210 instruments paid that 210 times.
    // One snapshot here makes each instrument one hash probe in each parent
    // plus its own rows (sweep audit OS-4, D-2301). The proof is the same
    // committed-receipt gate the API's detail readers use.
    let mut parents = match crate::result_set::CommittedParents::open_read_bounded(root, u64::MAX) {
        Ok(parents) => parents,
        Err(why) => {
            for s in screened {
                if s.outcome.is_ok() {
                    unread.push((s.symbol.clone(), format!("parent ledger not opened: {why}")));
                }
            }
            return (union, unread);
        }
    };
    for s in screened {
        let Ok(record) = &s.outcome else {
            continue;
        };
        let receipt = match parents.committed(&record.identity) {
            Ok(committed) => committed.map(|committed| committed.receipt),
            Err(why) => {
                unread.push((s.symbol.clone(), why));
                continue;
            }
        };
        match frontier.of_run_against_receipt(&record.identity, receipt) {
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
/// The span is prepared as pass 1's `audit-range` prepares one -- the same
/// loaders, the same execution-series check, the same withheld days, the same
/// anchored column, the same VWAP verdict -- AND THEN PRICED WHERE PASS 1
/// PRICES: on the one-minute execution series.
///
/// # What was wrong (GAP13-15, D-1702)
///
/// This evaluated the exit grid on the coarse SIGNAL bars with the horizon
/// counted in signal bars, while pass 1 projects its column onto the 1-minute
/// execution series ([`crate::project_onto_execution`], `Sourced::Fill`) and
/// prices there with the horizon in MINUTES, its stop ladder, rung count, step
/// and floors all measured on those minutes. So a coarse rung's pass-2 cell was
/// a different model's cell under the same mask: at 60min the pool printed "no
/// candidate fired" over instruments whose pass-1 rows showed 83-94 trades.
///
/// Now the projection is the one `audit_bars_work` calls, with the same
/// arguments: an explicit execution series for a coarse rung, and the native
/// self-alignment for `1min`. The grid is built exactly as `screen` builds it,
/// from the projected bars, and the rules are `Rules::derived` over the series
/// the floors are measured on (`floors_measured_on`): the execution one.
///
/// The column is pass 1's own build, `column_withholding_at_build`, run
/// read-only: a day whose exact closing minute cannot be sourced is withheld
/// and the column rebuilt, as pass 1 does, and no preparation attempt is
/// written. Until D-1707 this pass refused such an instrument instead.
/// `the_pool_prices_a_span_exactly_where_the_audit_path_does` and
/// `pass_two_withholds_an_unsourceable_close_day_as_pass_one_does` pin the
/// behaviour.
///
/// # Cost
///
/// Per instrument: the loads, `O(B_sig + B_exec)` for the column, the projection
/// and one hoisted `SliceFacts`; then per candidate one `grid::evaluate_over`,
/// which walks every row of the projected column -- `Θ(B_exec)` -- before it
/// prices `cells × T`. `docs/06-limits.md` states the whole pass (R9-cli-o1-1).
fn price_all(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    underlying: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
    union: &[Candidate],
) -> Result<Vec<Priced>, String> {
    let PreparedSpan {
        bars,
        column,
        horizon,
        rules,
    } = prepare_span(root, vendor, underlying, rung, from, to)?;
    let bars = bars.as_slice();
    let hold = usize::try_from(horizon.as_bars()).unwrap_or(usize::MAX);
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

/// One instrument's span over one month range, prepared exactly as the screen
/// prepares one and projected onto the series its trades fill on: the 1-minute
/// execution bars at a coarser rung, the signal bars themselves at 1min.
///
/// Lifted out of [`price_all`] so `pool-oos` prepares its training and later
/// spans through the same sequence, rather than a second copy of it that could
/// drift (D-1576). Pass 2 already priced on the execution series and `pool-oos`
/// walked the signal bars, which is the drift D-1576 was written against; the
/// projection lives here so both read it (D-2105).
/// `the_pool_prepares_projects_and_prices_in_order_inside_price_all` pins the
/// sequence.
pub(crate) struct PreparedSpan {
    /// The bars trades fill on, after interior-gap days are withheld.
    pub(crate) bars: Vec<indicators::Candle>,
    /// One condition row per bar of [`Self::bars`], projected from the signal
    /// column.
    pub(crate) column: indicators::column::Column,
    /// The holding period, in execution bars, these bars imply.
    pub(crate) horizon: runner::outcome::Horizon,
    /// The floors, measured on the series positions fill on.
    pub(crate) rules: crate::Rules,
}

/// [`PreparedSpan`] for one instrument, rung and month range, or the reason it
/// could not be prepared. Nothing is substituted for a refused step.
pub(crate) fn prepare_span(
    root: &std::path::Path,
    vendor: brutex_core::vendor::Vendor,
    underlying: &str,
    rung: &'static str,
    from: (u16, u8),
    to: (u16, u8),
) -> Result<PreparedSpan, String> {
    let native = rung == crate::EXECUTION_RUNG;
    let mut span = stored::load_span(root, vendor, underlying, rung, from, to)?;
    let signal_length = stored::rung_length_micros(rung)?;
    let execution_bars = if native {
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
    let cash = stored::span_cash_closes(root, &span.key, None, &span.bars)?;
    let holed_days = crate::minute_gaps::days_with_minute_holes(
        &span.bars,
        execution_slice,
        signal_length,
        |day| stored::session_close_for(cash.as_ref(), day),
    );
    // FOLDED WHOLE, SWEPT WITHOUT THE HOLED DAYS, as the screen does. D-1781.
    let folded = span.bars.clone();
    if !holed_days.is_empty() {
        let (kept, _withheld) = crate::minute_gaps::withhold(&span.bars, &holed_days);
        span.bars = kept;
    }
    let mut withheld_days = holed_days;
    // PASS 1'S OWN BUILD, READ-ONLY: a day whose exact closing minute cannot
    // be sourced is withheld and the column rebuilt from what survives, exactly
    // as `one_rung` and `audit_range_kernel` do; `commit: None` records no
    // preparation attempt, because this pass prepares nothing new. D-1707.
    let (column, _digest) = crate::column_withholding_at_build(
        root,
        vendor,
        underlying,
        (from, to),
        crate::FoldedSeries {
            folded: &folded,
            days: &mut withheld_days,
            bars: &mut span.bars,
        },
        signal_length,
        crate::StoredPreparationBuild { rung, commit: None },
    )?;
    let execution = execution_bars.as_ref().map(|exec| crate::Execution {
        bars: &exec.bars,
        signal_length_micros: signal_length,
    });
    // THE HORIZON, THE FLOORS AND THE PROJECTION, AS `audit_range_kernel`
    // RESOLVES THEM: the horizon from the signal bars on the execution-series
    // reading, the floors from the series positions fill on.
    let horizon = crate::horizon_for(&span.bars, execution.is_some());
    let rules = crate::Rules::derived(crate::floors_measured_on(&span.bars, execution), horizon);
    let (bars, column, _note) = crate::project_onto_execution(
        &span.bars, &column, execution, native, horizon,
    )
    .map_err(|why| format!("the span could not be projected onto the execution series: {why}"))?;
    Ok(PreparedSpan {
        bars,
        column,
        horizon,
        rules,
    })
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
            // Unreachable saturation: see `Pooled::net`.
            p.net = p.net.saturating_add(i128::from(cell.pessimistic));
            p.worst = p.worst.min(cell.worst_trade);
            if cell.wins > 0 {
                p.min_win = p.min_win.min(cell.min_win);
            }
            p.gross_win = p.gross_win.saturating_add(i128::from(cell.gross_win));
            p.gross_loss = p.gross_loss.saturating_add(i128::from(cell.gross_loss));
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
         reward-to-risk floor, 0 = OFF)\n  {POOLED_DRAWDOWN_LEGEND}",
        rule_bp / 100,
        rule_bp % 100
    );
    let (columns, header) = pooled_columns();
    let entries = pooled_entries(union, pooled, rules);
    let body: Vec<Vec<String>> = entries
        .iter()
        .filter_map(|entry| entry.as_ref().ok().map(|(cells, _)| cells.clone()))
        .collect();
    let laid = crate::columns::with_header(&columns, &header, body);
    let _ = writeln!(out, "{}", laid.header.trim_end());
    let mut lines = laid.rows.iter();
    for entry in &entries {
        match entry {
            Err(why) => {
                let _ = writeln!(out, "{why}");
            }
            Ok((_, mask)) => {
                let line = lines.next().map_or("", String::as_str);
                let _ = writeln!(out, "{}", line.trim_end());
                let _ = writeln!(out, "{mask}");
            }
        }
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

/// What the pooled table's `dd1max` column is, printed under the pass-2
/// heading (p2misc-1, D-2648). It said "a lower bound on the pooled drawdown",
/// which the largest single-instrument drawdown is not.
const POOLED_DRAWDOWN_LEGEND: &str = "dd1max is the LARGEST single-instrument drawdown among those \
     pooled; it bounds the pooled drawdown in neither direction and is not that figure";

/// The pooled table's columns and header.
///
/// LAID OUT TOGETHER, as pass 1 is (D-1420). This was one `format!` of
/// adjacent specifiers, `{:>12}{:>5}` for `net` and `dd>=`, so a drawdown of
/// five digits or more ran into the net: -16462 beside 123456 printed as
/// "-16462123456", one number that is neither (GAP13-16, D-1704). Every
/// column is now separated whatever its figures, and keeps its old width as
/// a minimum.
fn pooled_columns() -> ([crate::columns::Col; 12], [&'static str; 12]) {
    use crate::columns::{left, right};
    (
        [
            right(6),
            left(5).after(1),
            right(6),
            right(8),
            right(6),
            right(12),
            right(12),
            right(10),
            right(10),
            right(12),
            right(9),
            left(0).after(2),
        ],
        [
            "rank", "side", "fired", "trades", "wins", "worst", "min_win", "tail", "pf", "net",
            "dd1max", "fired on",
        ],
    )
}

/// One entry per ranked pooled row: its cells and its mask line, or the
/// refusal a missing candidate gets, in rank order.
fn pooled_entries(
    union: &[Candidate],
    pooled: &[Pooled],
    rules: crate::Rules,
) -> Vec<Result<(Vec<String>, String), String>> {
    let rule_bp = tail_rule_bp(rules);
    let mut entries: Vec<Result<(Vec<String>, String), String>> = Vec::new();
    for (rank, p) in pooled.iter().take(rules.top.max(1)).enumerate() {
        let Some(candidate) = union.get(p.candidate) else {
            entries.push(Err(format!(
                "refused: pooled rank {} names missing candidate {}; no row was fabricated",
                rank + 1,
                p.candidate
            )));
            continue;
        };
        let more = if p.fired > count(p.names.len()) {
            format!(" +{} more", p.fired.saturating_sub(count(p.names.len())))
        } else {
            String::new()
        };
        entries.push(Ok((
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
                p.tail_cell(),
                ratio_cell(p.profit_factor_bp()),
                p.net.to_string(),
                p.dd_bound.to_string(),
                format!("{}{more}", p.names.join(" ")),
            ],
            format!(
                "       mask {}{}",
                mask_hex(candidate.words),
                if p.meets(rule_bp) { "  RULE MET" } else { "" }
            ),
        )));
    }
    entries
}

/// A ratio in hundredths as `12.34x`, or `never lost` for the sentinel.
fn ratio_cell(bp: i128) -> String {
    if bp == NEVER_LOST {
        "never lost".to_owned()
    } else {
        format!("{}.{:02}x", bp / 100, bp % 100)
    }
}

/// The six mask words as hex, so a row can be matched to a frontier row.
pub(crate) fn mask_hex(words: [u64; 6]) -> String {
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
                net.max(0).saturating_add(worst.saturating_abs())
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

    /// **A candidate that never won meets no tail rule** (p5num-3, D-2713).
    ///
    /// Every pooled trade flat: `worst == 0`, `wins == 0`, so the tail was
    /// the never-lost sentinel and `meets` held at every multiple, sorting the
    /// row first. `grid::Cell::clears` refuses the same cell by its own
    /// `wins > 0` clause.
    #[test]
    fn a_candidate_that_never_won_meets_no_tail_rule() {
        let union = candidates(2);
        let surface = vec!["AAA".to_owned(), "BBB".to_owned()];
        let flat = cell(2, 0, 0, 0, 0, 0);
        assert!(!flat.clears(0, 0), "the single-instrument rule refuses it");
        let priced = vec![
            Ok(vec![Some(flat), Some(cell(1, 1, 500, -10, 500, 10))]),
            Ok(vec![Some(flat), None]),
        ];
        let pooled = fold(&union, &surface, &priced, rules_at(300));
        assert_eq!(pooled.len(), 2);
        let never_won = pooled
            .iter()
            .find(|p| p.candidate == 0)
            .expect("the flat candidate is pooled");
        assert_eq!((never_won.wins, never_won.worst), (0, 0));
        for rule_bp in [0, 1, 300, i64::MAX] {
            assert!(!never_won.meets(rule_bp), "rule {rule_bp}");
        }
        assert_eq!(never_won.tail_cell(), "-");
        assert_eq!(
            pooled.first().map(|p| p.candidate),
            Some(1),
            "the candidate that won and met the rule sorts first"
        );
        let won = pooled
            .iter()
            .find(|p| p.candidate == 1)
            .expect("the winning candidate is pooled");
        assert!(won.meets(300));
        assert_eq!(won.tail_cell(), "50.00x");
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
        assert_eq!(never_lost.profit_factor_bp(), super::NEVER_LOST);
        assert_eq!(
            super::ranked(never_lost.profit_factor_bp()),
            i128::MIN,
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

    /// h-cli-3, D-1852: pooled money totals past either end of `i64` are the
    /// exact sums, printed and ranked as such, and both ratios are exact where
    /// an `i64` multiply clamped them. Before, three `i64::MAX` nets pooled to
    /// `i64::MAX`, and a tail of `i64::MAX × 100` hundredths clamped to
    /// `i64::MAX` -- the never-lost sentinel -- so a measured ratio rendered as
    /// "never lost" and was demoted.
    #[test]
    fn pooled_money_totals_past_i64_are_exact_not_clamped() {
        let union = candidates(2);
        let surface = vec!["AAA".to_owned(), "BBB".to_owned(), "CCC".to_owned()];
        let rich = grid::Cell {
            trades: 1,
            wins: 1,
            pessimistic: i64::MAX,
            worst_trade: -1,
            min_win: i64::MAX,
            gross_win: i64::MAX,
            gross_loss: i64::MIN,
            max_drawdown: 1,
            ..grid::Cell::default()
        };
        let poor = grid::Cell {
            trades: 1,
            wins: 0,
            pessimistic: i64::MIN,
            worst_trade: i64::MIN,
            min_win: 0,
            gross_win: 0,
            gross_loss: i64::MIN,
            max_drawdown: 2,
            ..grid::Cell::default()
        };
        let priced = vec![Ok(vec![Some(rich), Some(poor)]); 3];
        let pooled = fold(&union, &surface, &priced, rules_at(300));
        let by = |candidate: usize| {
            pooled
                .iter()
                .find(|p| p.candidate == candidate)
                .expect("pooled candidate")
        };
        let up = by(0);
        assert_eq!(up.net, 3 * i128::from(i64::MAX));
        assert_eq!(up.gross_win, 3 * i128::from(i64::MAX));
        assert_eq!(up.gross_loss, 3 * i128::from(i64::MIN));
        // 300 x (2^63 - 1) over 3 x 2^63 is 99.99..., floored.
        assert_eq!(up.profit_factor_bp(), 99);
        assert_eq!(up.tail_bp(), i128::from(i64::MAX) * 100);
        assert_ne!(up.tail_bp(), super::NEVER_LOST);
        assert!(up.meets(i64::MAX), "the exact tail clears the largest rule");
        let unfired = Pooled {
            fired: 0,
            ..up.clone()
        };
        assert!(
            !unfired.meets(0),
            "a candidate that fired nowhere meets no rule, whatever its tail"
        );
        let down = by(1);
        assert_eq!(down.net, 3 * i128::from(i64::MIN));
        assert_eq!(down.gross_loss, 3 * i128::from(i64::MIN));
        assert_eq!(down.profit_factor_bp(), 0);
        assert!(up.key(300) > down.key(300));

        let mut rules = rules_at(300);
        rules.top = 2;
        let mut out = String::new();
        super::render_pooled(&mut out, &union, &surface, &priced, &pooled, rules);
        for total in [3 * i128::from(i64::MAX), 3 * i128::from(i64::MIN)] {
            assert!(out.contains(&total.to_string()), "{total} missing:\n{out}");
        }
        assert!(out.contains("9223372036854775807.00x"), "{out}");
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
        assert_eq!(ratio_cell(super::NEVER_LOST), "never lost");
        // `i64::MAX` was the sentinel while the ratio was an `i64`; it is now a
        // finite ratio and renders as one (D-1852).
        assert_eq!(ratio_cell(i128::from(i64::MAX)), "92233720368547758.07x");
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
        assert!(!why.contains(crate::STORED_POOLED_PROVENANCE), "{why}");

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
        assert!(!why.contains(crate::STORED_POOLED_PROVENANCE), "{why}");
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
            assert!(!page.contains(crate::STORED_POOLED_PROVENANCE), "{page}");
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

    /// Pass 1 lists priced rows by the money, smallest drawdown first, ties in
    /// the order they were screened, and every refusal LAST (CE-4, D-1769).
    #[test]
    fn pass_one_puts_refusals_last_and_keeps_ties_in_screened_order() {
        let record = |dd: i64, worst: i64, net: i64| crate::results::Record {
            max_drawdown: dd,
            worst_trade: worst,
            pessimistic: net,
            ..crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES])
        };
        let row = |symbol: &str, outcome| super::Screened {
            symbol: symbol.to_owned(),
            outcome,
        };
        let screened = [
            row("REFUSED_A", Err("a".to_owned())),
            row("DEEP", Ok(record(900, -50, 10))),
            row("TIE_FIRST", Ok(record(100, -20, 5))),
            row("TIE_SECOND", Ok(record(100, -20, 5))),
            row("SHALLOW_WORSE", Ok(record(100, -80, 5))),
            row("REFUSED_B", Err("b".to_owned())),
            row("SHALLOWEST", Ok(record(10, -90, 1))),
        ];
        let mut out = String::new();
        super::render_per_symbol(&mut out, &screened);
        let order: Vec<&str> = out
            .lines()
            .filter_map(|line| line.split_whitespace().next())
            .filter(|word| screened.iter().any(|s| s.symbol == *word))
            .collect();
        assert_eq!(
            order,
            [
                "SHALLOWEST",
                "TIE_FIRST",
                "TIE_SECOND",
                "SHALLOW_WORSE",
                "DEEP",
                "REFUSED_A",
                "REFUSED_B"
            ],
            "{out}"
        );
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
                "            if let Err(why) = stored_words(vendor, None, Some(known), Some(((fy, fm), (ty, tm)))) {\n",
                "                return refuse(out, &why);\n",
                "            }\n",
                "            let text = pool::pool(vendor, known, (fy, fm), (ty, tm), h);\n",
                "            let code = work_exit(&text);\n",
                "            out.push_str(&text);\n",
                "            code\n",
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
    /// The first exits OK and is not a refusal; the second exits FAILED. An
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
                assert_eq!(*status, crate::FAILED, "{page}");
                assert!(
                    page.starts_with("refused: this build carries no verified commit stamp"),
                    "{page}"
                );
                assert!(!page.contains(crate::STORED_POOLED_PROVENANCE), "{page}");
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
                .any(|line| line.starts_with(&format!("  {:<14}{:>9}", "NIFTY", 300))),
            "pass 1 records the 300 bars it swept of NIFTY's 600 generated five-minute bars, warm-up excluded (D-1661):\n{pass_1}"
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
            !page.contains(crate::STORED_POOLED_PROVENANCE),
            "no banner over nothing:\n{page}"
        );
        assert!(crate::carries_refusal(page), "{page}");
        assert_eq!(
            status,
            crate::FAILED,
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
            "brokerage, STT, stamp duty, exchange charges, the SEBI fee, the\nIPFT, DP charges and GST (an UNVERIFIED list)",
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

    /// **`prepare_span` prepares and projects in order, and `price_all` prices
    /// only what it returns.** W2-cli9-4, D-1702, D-2105.
    ///
    /// This was `the_pool_prepares_a_span_exactly_as_the_screen_does`, and it
    /// proved less than its name: both searches were unbounded `find`s, so
    /// `stored_anchored_column(` -- absent from `screen_range_inner` -- was
    /// matched inside `audit_bars_work`, another function, and the test passed
    /// on text outside what it pinned. The equivalence it claimed is now
    /// proved by behaviour, in
    /// `the_pool_prices_a_span_exactly_where_the_audit_path_does`. What stays
    /// here is the order of the steps, each searched only within its own
    /// function's body: preparation and projection in `prepare_span`, which
    /// `pool-oos` shares since D-1576, and pricing over the projected column in
    /// `price_all`.
    #[test]
    fn the_pool_prepares_projects_and_prices_in_order_inside_price_all() {
        const PREPARE: [&str; 11] = [
            "stored::load_span(",
            "stored::rung_length_micros(",
            "validate_one_minute_execution(",
            "minute_gaps::days_with_minute_holes(",
            "minute_gaps::withhold(",
            "crate::column_withholding_at_build(",
            "crate::StoredPreparationBuild { rung, commit: None }",
            "horizon_for(",
            "floors_measured_on(",
            "crate::project_onto_execution(",
            "&span.bars, &column, execution, native, horizon,",
        ];
        const PRICE: [&str; 3] = [
            "prepare_span(root, vendor, underlying, rung, from, to)?",
            "SliceFacts::of(bars, &column)",
            "evaluate_over(bars, &column,",
        ];
        let pool = include_str!("pool.rs");
        let body_of = |head: &str| {
            let from = pool.find(head).expect("the function exists");
            pool.get(from..)
                .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to)))
                .expect("its body")
        };
        for (head, sequence) in [
            ("\npub(crate) fn prepare_span(", &PREPARE[..]),
            ("\nfn price_all(", &PRICE[..]),
        ] {
            let body = body_of(head);
            let mut last = 0;
            for step in sequence {
                let at = body
                    .get(last..)
                    .and_then(|rest| rest.find(step))
                    .map(|i| i + last);
                assert!(
                    at.is_some(),
                    "`{step}` must appear in `{head}`'s body after the previous step"
                );
                last = at.expect("asserted above");
            }
        }
        let prepare = body_of("\npub(crate) fn prepare_span(");
        let projected = prepare
            .find("let (bars, column, _note) =")
            .expect("the projection rebinds the bars and the column");
        let returned = prepare
            .find("Ok(PreparedSpan {")
            .expect("the span is returned");
        assert!(
            projected < returned,
            "a prepared span must carry the projected series, never the signal column"
        );
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
            crate::columns::assert_under(lines[at], row, &[L, R, R, R, R, R, R, R, R, L])
                .expect("separated and aligned");
        }
        assert_eq!(lines.len() - at - 1, screened.len(), "{out}");
    }

    /// A pass-1 row whose money columns are `(max_drawdown, worst, net)`.
    fn screened_row(symbol: &str, dd: i64, worst: i64, net: i64) -> super::Screened {
        super::Screened {
            symbol: symbol.to_owned(),
            outcome: Ok(crate::results::Record {
                trades: 1,
                pessimistic: net,
                worst_trade: worst,
                max_drawdown: dd,
                ..crate::results::Record::from_bytes(&[0; crate::results::STRIDE_BYTES])
            }),
        }
    }

    fn refused_row(symbol: &str) -> super::Screened {
        super::Screened {
            symbol: symbol.to_owned(),
            outcome: Err(format!("{symbol} refused for this test")),
        }
    }

    /// The symbols of pass 1's body rows, in the order they printed.
    fn printed_order(out: &str) -> Vec<String> {
        let mut lines = out
            .lines()
            .skip_while(|l| !l.trim_start().starts_with("symbol"));
        let _header = lines.next();
        lines
            .map(|l| l.split_whitespace().next().unwrap_or_default().to_owned())
            .collect()
    }

    /// **Refusals print LAST, as the comment and D-0509's table say.**
    /// W2-cli9-7, D-1704.
    ///
    /// The order was one ascending sort with refusals keyed `(true, ..)` and
    /// then `reverse()`, so the refusal printed FIRST. Mixed input, refusal
    /// placed first and in the middle so input order cannot hide it.
    #[test]
    fn pass_one_prints_its_refusals_after_every_screened_row() {
        let screened = [
            refused_row("RFIRST"),
            screened_row("BIGDD", 900, -50, 10),
            refused_row("RMIDDLE"),
            screened_row("SMALLDD", 100, -500, -20),
        ];
        let mut out = String::new();
        super::render_per_symbol(&mut out, &screened);
        assert_eq!(
            printed_order(&out),
            ["SMALLDD", "BIGDD", "RFIRST", "RMIDDLE"],
            "smallest drawdown first, refusals last in input order:\n{out}"
        );
        // The refusal lines carry their reasons, so none was dropped.
        assert!(
            out.contains("REFUSED: RFIRST refused") && out.contains("REFUSED: RMIDDLE refused")
        );
    }

    /// The money key's three terms each decide, and a full tie keeps input
    /// order (a stable sort under `Reverse`). D-1704.
    #[test]
    fn pass_one_orders_by_drawdown_then_worst_then_net_and_keeps_ties_stable() {
        let screened = [
            screened_row("TIEA", 100, -10, 5),
            screened_row("NETLOW", 100, -10, 1),
            screened_row("WORSTLOW", 100, -99, 50),
            screened_row("TIEB", 100, -10, 5),
            screened_row("DDHIGH", 101, 0, 1_000),
        ];
        let mut out = String::new();
        super::render_per_symbol(&mut out, &screened);
        assert_eq!(
            printed_order(&out),
            ["TIEA", "TIEB", "NETLOW", "WORSTLOW", "DDHIGH"],
            "{out}"
        );
    }

    /// All refused and all screened: neither part is lost or reordered.
    #[test]
    fn pass_one_keeps_an_all_refused_or_all_screened_table_whole() {
        let refused = [refused_row("B"), refused_row("A")];
        let mut out = String::new();
        super::render_per_symbol(&mut out, &refused);
        assert_eq!(printed_order(&out), ["B", "A"], "{out}");
        let screened = [screened_row("B", 5, 0, 0), screened_row("A", 1, 0, 0)];
        let mut out = String::new();
        super::render_per_symbol(&mut out, &screened);
        assert_eq!(printed_order(&out), ["A", "B"], "{out}");
    }

    /// **`net` and `dd>=` are two figures, not one.** GAP13-16, D-1704.
    ///
    /// The pooled row was `{:>12}{:>5}` for them, so a net of -16462 beside a
    /// drawdown bound of 123456 printed as `-16462123456`. Every pooled row is
    /// now laid out with its header and every column is separated.
    #[test]
    #[allow(clippy::indexing_slicing, reason = "a missing line must fail the test")]
    fn the_pooled_net_and_drawdown_bound_print_as_separate_figures() {
        use crate::columns::Align::{Left as L, Right as R};
        let union = candidates(2);
        let surface = vec!["AAA".to_owned()];
        let mut wide = cell(3, 1, -16_462, -20_000, 3_538, 123_456);
        wide.gross_win = 3_538;
        let extreme = cell(u64::MAX, u64::MAX, i64::MIN, i64::MIN, i64::MAX, i64::MAX);
        let priced = vec![Ok(vec![Some(wide), Some(extreme)])];
        let rules = rules_at(0);
        let pooled = fold(&union, &surface, &priced, rules);
        let mut out = String::new();
        super::render_pooled(&mut out, &union, &surface, &priced, &pooled, rules);
        assert!(!out.contains("-16462123456"), "{out}");
        let lines: Vec<&str> = out.lines().collect();
        let at = lines
            .iter()
            .position(|l| l.trim_start().starts_with("rank"))
            .expect("a header");
        let rows: Vec<&str> = lines[at + 1..]
            .iter()
            .copied()
            .filter(|l| !l.trim_start().starts_with("mask") && !l.trim().is_empty())
            .take(2)
            .collect();
        assert_eq!(rows.len(), 2, "{out}");
        let wide_row = rows
            .iter()
            .find(|r| r.contains("-16462"))
            .expect("the -16462 row");
        let tokens: Vec<&str> = wide_row.split_whitespace().collect();
        let net = tokens
            .iter()
            .position(|t| *t == "-16462")
            .expect("net alone");
        assert_eq!(tokens.get(net + 1), Some(&"123456"), "{wide_row}");
        // Every column under its header, at ordinary and at extreme figures.
        // The trailing `fired on` text column is one word here ("AAA").
        let header = lines[at].replace("fired on", "fired_on");
        for row in rows {
            crate::columns::assert_under(&header, row, &[R, L, R, R, R, R, R, R, R, R, R, L])
                .expect("separated and aligned");
        }
    }

    /// v4-2, GAP13-16, D-1487 (kept by D-1708 against D-1704's layout): the
    /// pooled pass-2 table at the extremes a row
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
            net: i128::from(extreme),
            // `worst` and `gross_loss` at `i64::MIN` keep both ratios finite,
            // so every cell is one word; `min_win` and `gross_win` are never
            // negative in a real fold.
            worst: i64::MIN,
            min_win: extreme.max(0),
            gross_win: i128::from(extreme.max(0)),
            gross_loss: i128::from(i64::MIN),
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

    /// A scratch store for the union tests, unique per thread.
    fn union_root(tag: &str) -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "brutex-pool-union-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("scratch store");
        root
    }

    /// One run committed in the order production commits it: frontier block,
    /// trades, receipt (when `receipt`), then the ledger row. Its one frontier
    /// row carries `mask_word` as its combination.
    fn commit_run(
        root: &std::path::Path,
        n: u8,
        symbol: &str,
        mask_word: u64,
        receipt: bool,
    ) -> crate::results::Record {
        let identity = [n; 32];
        let mut row = crate::tests::result_commit_frontier(identity, 11);
        row.mask_words = [mask_word, 0, 0, 0, 0, 0];
        crate::ensure_frontier_rows(root, &identity, &[row]).expect("frontier block");
        crate::ensure_trade_rows(
            root,
            &identity,
            &[crate::tests::result_commit_trade(identity)],
        )
        .expect("trade block");
        if receipt {
            crate::ensure_detail_receipt(root, identity, 1, 1, Direction::Short)
                .expect("detail receipt");
        }
        let mut record = crate::tests::record_for_naming();
        record.identity = identity;
        record.underlying = crate::results::field(symbol);
        record.combinations = 11;
        record.trades = 1;
        crate::ensure_run_record(root, &record).expect("ledger row");
        record
    }

    fn screened_ok(symbol: &str, record: crate::results::Record) -> super::Screened {
        super::Screened {
            symbol: symbol.to_owned(),
            outcome: Ok(record),
        }
    }

    /// **The union admits the parent ledger ONCE, not once per instrument.**
    /// W2-cli9-0, D-1703.
    ///
    /// `Frontier::of_run` proved each instrument's parent by opening and
    /// indexing the whole results ledger and receipt sidecar, so five
    /// instruments were five O(L + R) index builds. The count is read from the
    /// ledger's own open seam on this thread; `union_of` is sequential, so no
    /// other thread's open can enter it. The union itself is unchanged: three
    /// distinct masks over five instruments, in first-seen order, and the
    /// refused instrument contributes nothing and is not named as unread.
    #[test]
    fn the_union_admits_the_parent_ledger_once_not_once_per_instrument() {
        let root = union_root("once");
        let mut screened: Vec<super::Screened> = (0_u8..5)
            .map(|i| {
                let symbol = format!("SYM{i}");
                let record = commit_run(&root, 100 + i, &symbol, u64::from(i % 3) + 1, true);
                screened_ok(&symbol, record)
            })
            .collect();
        screened.insert(2, refused_row("REFUSEDSYM"));
        crate::results::OPENS.with(|n| n.set(0));
        let (union, unread) = super::union_of(&root, &screened);
        let opens = crate::results::OPENS.with(std::cell::Cell::get);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(opens, 1, "one ledger index build for five instruments");
        assert!(unread.is_empty(), "{unread:?}");
        assert_eq!(
            union.iter().map(|c| c.words[0]).collect::<Vec<_>>(),
            [1, 2, 3],
            "first-seen order, duplicates collapsed"
        );
    }

    /// A committed row whose receipt is missing is NAMED, and the others still
    /// enter the union; the receipt-less refusal is the one `of_run` made.
    #[test]
    fn a_missing_receipt_names_its_instrument_and_the_rest_still_pool() {
        let root = union_root("receiptless");
        let kept = commit_run(&root, 120, "KEPT", 7, true);
        let bare = commit_run(&root, 121, "BARE", 8, false);
        let screened = [screened_ok("KEPT", kept), screened_ok("BARE", bare)];
        let (union, unread) = super::union_of(&root, &screened);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(union.iter().map(|c| c.words[0]).collect::<Vec<_>>(), [7]);
        assert_eq!(unread.len(), 1, "{unread:?}");
        let (symbol, why) = unread.first().expect("one named instrument");
        assert_eq!(symbol, "BARE");
        assert!(why.contains("no validated detail receipt"), "{unread:?}");
    }

    /// No frontier file: every screened instrument is named, none silently
    /// dropped, and nothing is created on the read path.
    #[test]
    fn an_absent_frontier_names_every_screened_instrument() {
        let root = union_root("absent");
        let mut record = crate::tests::record_for_naming();
        record.identity = [122; 32];
        let screened = [screened_ok("ONE", record), refused_row("TWO")];
        let (union, unread) = super::union_of(&root, &screened);
        let created = crate::results::Results::path(&root).exists();
        let _ = std::fs::remove_dir_all(&root);
        assert!(union.is_empty());
        assert_eq!(unread.len(), 1, "{unread:?}");
        let (_, why) = unread.first().expect("one named instrument");
        assert!(why.starts_with("frontier not opened:"), "{unread:?}");
        assert!(!created, "a read created the ledger");
    }

    /// A frontier block whose run never reached the ledger stays hidden and
    /// is named, exactly as `of_run` hid it.
    #[test]
    fn an_uncommitted_frontier_block_is_named_not_pooled() {
        let root = union_root("uncommitted");
        let identity = [123; 32];
        crate::ensure_frontier_rows(
            &root,
            &identity,
            &[crate::tests::result_commit_frontier(identity, 11)],
        )
        .expect("prepared block");
        let mut record = crate::tests::record_for_naming();
        record.identity = identity;
        let (union, unread) = super::union_of(&root, &[screened_ok("HIDDEN", record)]);
        let _ = std::fs::remove_dir_all(&root);
        assert!(union.is_empty());
        assert_eq!(unread.len(), 1, "{unread:?}");
        let (_, why) = unread.first().expect("one named instrument");
        assert!(why.contains("hidden"), "{unread:?}");
    }

    /// A ledger the parent snapshot cannot admit names every screened
    /// instrument with that refusal; no instrument is read without its parent.
    #[test]
    fn an_unreadable_parent_ledger_names_every_screened_instrument() {
        let root = union_root("badledger");
        let record = commit_run(&root, 124, "ONE", 9, true);
        std::fs::write(crate::results::Results::path(&root), b"NOTALEDGERFILE00")
            .expect("damage the ledger");
        let (union, unread) = super::union_of(&root, &[screened_ok("ONE", record)]);
        let _ = std::fs::remove_dir_all(&root);
        assert!(union.is_empty());
        assert_eq!(unread.len(), 1, "{unread:?}");
        let (_, why) = unread.first().expect("one named instrument");
        assert!(why.starts_with("parent ledger not opened:"), "{unread:?}");
    }

    /// The cell the AUDIT PATH would price for `mask` and `side` on
    /// `underlying` at `rung`: the anchored column projected by
    /// `project_onto_execution` exactly as `audit_bars_work` calls it, the
    /// horizon and floors as `audit_range_kernel` resolves them, and the grid
    /// as `screen` builds it. Independent of `price_all`'s own body: it is
    /// assembled here from the audit path's functions.
    fn audit_path_cell(
        root: &std::path::Path,
        rung: &'static str,
        words: [u64; 6],
        side: runner::excursion::Side,
        withheld: &[i64],
    ) -> Option<grid::Cell> {
        let span = |r: &str| {
            crate::stored::load_span(
                root,
                brutex_core::vendor::Vendor::Zerodha,
                "NIFTY",
                r,
                (2025, 5),
                (2025, 5),
            )
            .expect("generated span")
        };
        let mut signal = span(rung);
        // The days pass 1 withholds, named by the caller; none on a whole store.
        signal.bars = crate::minute_gaps::withhold(&signal.bars, withheld).0;
        let native = rung == crate::EXECUTION_RUNG;
        let exec = (!native).then(|| span(crate::EXECUTION_RUNG));
        let length = crate::stored::rung_length_micros(rung).expect("rung length");
        let range = ((2025, 5), (2025, 5));
        let vendor = brutex_core::vendor::Vendor::Zerodha;
        let daily = crate::stored::load_daily_context(root, vendor, "NIFTY", range, &signal.bars)
            .expect("daily");
        let exact =
            crate::stored::load_exact_minute_context(root, vendor, "NIFTY", range, &signal.bars)
                .expect("exact minute");
        let column = crate::stored_anchored_column(
            &signal.bars,
            &daily,
            &exact,
            length,
            crate::stored::vwap_availability(&signal.key),
        )
        .expect("anchored column");
        let execution = exec.as_ref().map(|e| crate::Execution {
            bars: &e.bars,
            signal_length_micros: length,
        });
        let horizon = crate::horizon_for(&signal.bars, execution.is_some());
        let rules = crate::Rules::derived(execution.map_or(&signal.bars[..], |e| e.bars), horizon);
        let (bars, column, _) =
            crate::project_onto_execution(&signal.bars, &column, execution, native, horizon)
                .expect("projection");
        let hold = usize::try_from(horizon.as_bars()).expect("hold");
        let stops = crate::stop_ladder_ppm(&bars, hold);
        let levels = grid::Levels {
            rungs: crate::grid_rungs(&bars),
            step_ppm: Some(crate::grid_step_ppm(&bars, hold)),
            forced: (rules.max_mae_ppm > 0).then_some(rules.max_mae_ppm),
            ratios: true,
            stops_ppm: &stops,
        };
        let facts = runner::trade::SliceFacts::of(&bars, &column);
        let mask = vocab::ConditionMask::from_words(words);
        let g = grid::evaluate_over(&bars, &column, &mask, horizon, side, levels, &facts);
        crate::shown_cell(&g, rules)
            .map(|(cell, _)| cell)
            .filter(|cell| cell.trades > 0)
    }

    /// **Pass 2 prices where pass 1 prices: on the one-minute execution
    /// series, the horizon in minutes.** GAP13-15 and W2-cli9-4, D-1702.
    ///
    /// Replaces a source-order test whose unbounded `find` matched text in a
    /// function it did not name. This one runs `price_all` on a generated
    /// store, for a coarse rung (projected) and for `1min` (self-aligned),
    /// both sides, and holds every cell to the audit path's own cell, which
    /// fires. Before the fix the coarse rung was evaluated on its signal bars
    /// with a horizon in signal bars and its cell differed.
    #[test]
    fn the_pool_prices_a_span_exactly_where_the_audit_path_does() {
        let _knobs = crate::knobs::serially();
        crate::knobs::clear_all();
        crate::audited_stored::with_warmed_store(|root| {
            let union = [
                Candidate {
                    words: [0; 6],
                    direction: Direction::Long,
                },
                Candidate {
                    words: [0; 6],
                    direction: Direction::Short,
                },
            ];
            for rung in ["5min", "1min"] {
                let priced = super::price_all(
                    root,
                    brutex_core::vendor::Vendor::Zerodha,
                    "NIFTY",
                    rung,
                    (2025, 5),
                    (2025, 5),
                    &union,
                )
                .expect("pass 2 prices the generated span");
                assert_eq!(priced.len(), union.len());
                for (candidate, cell) in union.iter().zip(&priced) {
                    let side = match candidate.direction {
                        Direction::Long => runner::excursion::Side::Long,
                        Direction::Short => runner::excursion::Side::Short,
                    };
                    let reference = audit_path_cell(root, rung, candidate.words, side, &[]);
                    assert!(
                        reference.is_some_and(|c| c.trades > 0),
                        "premise: the audit path fires at {rung}"
                    );
                    assert_eq!(*cell, reference, "{rung} {side:?}");
                }
            }
        });
    }

    /// **Pass 2 withholds the day pass 1 withholds when that day's closing
    /// minute cannot be sourced, and prices the rest exactly where the audit
    /// path does.** D-1707, closing the difference D-1702 stated.
    ///
    /// The store's one-minute series stops five minutes early on one day: no
    /// interior minute is missing, so the interior-gap withholding names
    /// nothing, and the plain column build refuses on that day's closing
    /// minute (both are premises). Pass 1 withholds the day and rebuilds;
    /// before D-1707 pass 2 refused the whole instrument instead.
    #[test]
    fn pass_two_withholds_an_unsourceable_close_day_as_pass_one_does() {
        let _knobs = crate::knobs::serially();
        crate::knobs::clear_all();
        crate::audited_stored::with_unsourceable_close(5, |root, day| {
            let vendor = brutex_core::vendor::Vendor::Zerodha;
            let range = ((2025, 5), (2025, 5));
            let load = |rung: &str| {
                crate::stored::load_span(root, vendor, "NIFTY", rung, range.0, range.1)
                    .expect("generated span")
            };
            let (signal, minutes) = (load("5min"), load(crate::EXECUTION_RUNG));
            assert!(
                crate::minute_gaps::days_with_interior_gaps(&minutes.bars).is_empty(),
                "premise: the cut is at the session's end, not interior"
            );
            let refused =
                crate::stored::load_daily_context(root, vendor, "NIFTY", range, &signal.bars)
                    .and_then(|daily| {
                        let exact = crate::stored::load_exact_minute_context(
                            root,
                            vendor,
                            "NIFTY",
                            range,
                            &signal.bars,
                        )?;
                        crate::stored_anchored_column(
                            &signal.bars,
                            &daily,
                            &exact,
                            crate::stored::rung_length_micros("5min")?,
                            crate::stored::vwap_availability(&signal.key),
                        )
                    })
                    .expect_err("premise: the whole span's column build refuses");
            assert_eq!(
                crate::unsourceable_minute(&refused).map(indicators::ist_day),
                Some(day),
                "premise: it refuses on the short day's closing minute: {refused}"
            );
            let union = [
                Candidate {
                    words: [0; 6],
                    direction: Direction::Long,
                },
                Candidate {
                    words: [0; 6],
                    direction: Direction::Short,
                },
            ];
            let priced = super::price_all(root, vendor, "NIFTY", "5min", range.0, range.1, &union)
                .expect("pass 2 withholds the day and prices the rest");
            assert_eq!(priced.len(), union.len());
            for (candidate, cell) in union.iter().zip(&priced) {
                let side = match candidate.direction {
                    Direction::Long => runner::excursion::Side::Long,
                    Direction::Short => runner::excursion::Side::Short,
                };
                let reference = audit_path_cell(root, "5min", candidate.words, side, &[day]);
                assert!(
                    reference.is_some_and(|c| c.trades > 0),
                    "premise: the audit path fires on the withheld span"
                );
                assert_eq!(*cell, reference, "{side:?}");
            }
        });
    }

    /// A span pass 2 cannot load is refused for that instrument by name, not
    /// priced as "never fired". The generated store holds no 5min April.
    #[test]
    fn a_span_pass_two_cannot_load_is_refused_not_priced_empty() {
        crate::audited_stored::with_warmed_store(|root| {
            let union = [Candidate {
                words: [0; 6],
                direction: Direction::Long,
            }];
            let refused = super::price_all(
                root,
                brutex_core::vendor::Vendor::Zerodha,
                "NIFTY",
                "5min",
                (2025, 4),
                (2025, 4),
                &union,
            );
            assert!(refused.is_err(), "{refused:?}");
        });
    }

    /// The single-run promises `STORED_PROVENANCE` makes, which no page over
    /// many instruments or months may carry. R9-cli-law-3, GAP15-21, D-1705.
    const SINGLE_RUN_PROMISES: [&str; 2] = [
        "describes that instrument and that month",
        "the run identity beneath names the exact",
    ];

    /// **The pool page promises no single instrument, month or identity, and
    /// prints the identity of every row pass 1 recorded.** R9-cli-law-3,
    /// D-1705.
    ///
    /// Its opening was `STORED_PROVENANCE`, which says the run identity
    /// beneath names the exact column and that a figure describes "that
    /// instrument and that month" -- over a page of many instruments and no
    /// identity at all.
    #[test]
    fn the_pool_page_promises_no_single_run_and_prints_each_rows_identity() {
        let head = super::opening(
            "zerodha",
            "5min",
            (2025, 1),
            (2025, 5),
            None,
            &["BANKNIFTY".to_owned(), "NIFTY".to_owned()],
        );
        assert!(head.starts_with(crate::STORED_POOLED_PROVENANCE), "{head}");
        for promise in SINGLE_RUN_PROMISES {
            assert!(!head.contains(promise), "{promise}: {head}");
        }
        let mut first = screened_row("NIFTY", 10, -1, 5);
        let mut second = screened_row("BANKNIFTY", 20, -1, 5);
        for (row, byte) in [(&mut first, 0xab_u8), (&mut second, 0x01)] {
            if let Ok(record) = &mut row.outcome {
                record.identity = [byte; 32];
            }
        }
        let mut out = String::new();
        super::render_per_symbol(&mut out, &[first, second, refused_row("TCS")]);
        for (symbol, byte) in [("NIFTY", "ab"), ("BANKNIFTY", "01")] {
            let line = out
                .lines()
                .find(|l| l.trim_start().starts_with(symbol))
                .expect("its row");
            assert!(
                line.trim_end().ends_with(&byte.repeat(32)),
                "{symbol}'s row ends with its identity: {line}"
            );
        }
        let refused = out.lines().find(|l| l.contains("TCS")).expect("TCS row");
        assert!(refused.contains("REFUSED: TCS refused"), "{refused}");
    }

    /// **`ledger-v6`, `ledger-v6-replay` and `ledger-all` open with the pooled
    /// banner.** GAP15-21, D-1705.
    ///
    /// Each covers eight rungs over a span of months, and each printed the
    /// single-run banner. A feed word no vendor has refuses before any store is
    /// read, so the page is the banner, its own heading and the refusal.
    #[test]
    fn the_ledger_pages_promise_no_single_instrument_month_or_identity() {
        let _knobs = crate::knobs::serially();
        crate::knobs::clear_all();
        let root = std::env::temp_dir().join(format!(
            "brutex-pooled-banner-ledgers-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let request = crate::ledger_all::LedgerAllRequest {
            vendor: "nosuchfeed",
            from: (2025, 1),
            to: (2025, 5),
            support_ppm: 200_000,
            max_points: 50,
            root: &root,
        };
        let pages = [
            ("ledger-v6", crate::ledger_v6::ledger_v6(&request)),
            (
                "ledger-v6-replay",
                crate::ledger_v6::ledger_v6_replay(&request, (2025, 6), (2025, 7)),
            ),
            ("ledger-all", crate::ledger_all::ledger_all(&request)),
        ];
        for (verb, page) in pages {
            assert!(
                page.starts_with(crate::STORED_POOLED_PROVENANCE),
                "{verb}:\n{page}"
            );
            for promise in SINGLE_RUN_PROMISES {
                assert!(!page.contains(promise), "{verb} {promise}:\n{page}");
            }
            assert!(page.contains("refused:"), "premise, {verb}:\n{page}");
        }
        assert!(!root.exists(), "a refused feed creates no tree");
    }

    /// **`in_input_order` runs one call at a time, in input order, even when
    /// the first is the slowest.** GAP13-13, R9-cli-o1-0, D-1701.
    #[test]
    fn in_input_order_calls_each_item_alone_and_in_order() {
        let log = std::sync::Mutex::new(Vec::new());
        let out = crate::in_input_order(&[3_u64, 0, 1, 2], |&n| {
            log.lock().expect("log").push(format!("start {n}"));
            std::thread::sleep(std::time::Duration::from_millis(n * 20));
            log.lock().expect("log").push(format!("end {n}"));
            n * 10
        });
        assert_eq!(out, [30, 0, 10, 20]);
        assert_eq!(
            log.into_inner().expect("log"),
            [
                "start 3", "end 3", "start 0", "end 0", "start 1", "end 1", "start 2", "end 2"
            ]
        );
        assert!(crate::in_input_order(&[] as &[u8], |_| 0_u8).is_empty());
    }

    /// Both outer loops over `one_rung` -- `range-all`'s and pool pass 1's --
    /// go through `in_input_order` and neither is a `par_iter`. Each
    /// `one_rung` writes durable rows from inside the kernel, so a parallel
    /// outer loop writes them in completion order (GAP13-13) and runs several
    /// whole-machine sweeps at once (R9-cli-o1-0). D-1701.
    #[test]
    fn every_outer_loop_over_one_rung_runs_in_input_order() {
        let body = |source: &'static str, head: &str| -> &'static str {
            let from = source.find(head).expect("the function");
            source
                .get(from..)
                .and_then(|rest| rest.find("\n}\n").and_then(|to| rest.get(..to)))
                .expect("its body")
        };
        let rungs = body(include_str!("lib.rs"), "\nfn sweep_rungs(");
        assert!(rungs.contains("in_input_order(rungs,") && !rungs.contains("par_iter"));
        assert!(
            !rungs.contains("SharedBy::these"),
            "one sweep in flight shares nothing"
        );
        let pool = body(include_str!("pool.rs"), "\nfn run_under(");
        let pass_1 = pool
            .split_once("PASS 1:")
            .and_then(|(_, rest)| rest.split_once("PASS 2:"))
            .expect("pass 1")
            .0;
        assert!(
            pass_1.contains("crate::in_input_order(&surface,"),
            "{pass_1}"
        );
        assert!(pass_1.contains("crate::one_rung("));
        assert!(!pass_1.contains("par_iter"), "{pass_1}");
    }

    /// The drawdown of a time-ordered P&L sequence: the largest fall from a
    /// running peak, the figure a pooled drawdown would be.
    fn sequence_drawdown(pnl: &[i64]) -> i64 {
        let (mut equity, mut peak, mut worst) = (0_i64, 0_i64, 0_i64);
        for step in pnl {
            equity += step;
            peak = peak.max(equity);
            worst = worst.max(peak - equity);
        }
        worst
    }

    /// p2misc-1 (D-2648): the drawdown column is not called a bound, because
    /// the largest single-instrument drawdown bounds the merged sequence's in
    /// neither direction -- shown here both ways. On the old code the legend
    /// said "a lower bound on the pooled drawdown" and the header was `dd>=`.
    #[test]
    fn the_drawdown_column_is_not_called_a_bound() {
        let legend = super::POOLED_DRAWDOWN_LEGEND;
        assert!(!legend.contains("lower bound"), "{legend}");
        assert!(legend.contains("neither direction"), "{legend}");
        let (_, header) = super::pooled_columns();
        assert!(header.contains(&"dd1max"), "{header:?}");
        assert!(!header.iter().any(|name| name.contains(">=")), "{header:?}");
        let source = include_str!("pool.rs");
        let shipping = source.split("\nmod tests {").next().unwrap_or(source);
        assert!(
            !shipping.contains("a lower bound on the pooled"),
            "stale wording"
        );

        // Above the pooled figure: A -100, B +150, A -100.
        let a = [-100, -100];
        let merged = [-100, 150, -100];
        assert_eq!(sequence_drawdown(&a), 200, "the column prints 200");
        assert_eq!(sequence_drawdown(&[150]), 0);
        assert_eq!(sequence_drawdown(&merged), 100, "the pooled figure is 100");
        // Below the pooled figure: A and B each lose 100 at once.
        assert_eq!(sequence_drawdown(&[-100]), 100, "the column prints 100");
        assert_eq!(
            sequence_drawdown(&[-100, -100]),
            200,
            "the pooled figure is 200"
        );
        assert_eq!(sequence_drawdown(&[]), 0);
    }
}
