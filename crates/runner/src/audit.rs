//! The audit surface: everything the newer stages measured, rendered so a human
//! can read it without opening the code.
//!
//! # The gap this closes
//!
//! [`crate::report`] renders the SWEEP, the BARS, the LADDER, the SIGNIFICANCE
//! bar and the FINDINGS. It was written before any of the execution stages
//! existed and it shows **none** of them: not the trades, not the exit grid, not
//! the sniper measure, not the walk-forward, not the overfitting probability,
//! not the bootstrap.
//!
//! So the engine could measure all of it and an operator could see none of it.
//! `CLAUDE.md` §4 bans a result that hides a failure behind a success, and a
//! number nobody can read is not far off — a walk-forward that came out negative
//! and a walk-forward that never ran looked identical from outside.
//!
//! # Every section states what it CANNOT tell you
//!
//! That is not modesty, it is the difference between an audit and a brochure.
//! A reader who takes "PBO 0.08" as "this is profitable" has been misled by a
//! true number, and the only defence is to put the limit beside the figure
//! rather than in a document they will not open.
//!
//! # Plain text, and no dependency
//!
//! The same choice [`crate::report`] makes: a sweep is audited from a terminal
//! or a log file, and both of those are text. A browser view belongs under
//! `web/` and this crate cannot reach it.
//!
//! # Cost
//!
//! One `String` per render. It touches no bar and re-measures nothing — every
//! figure was already counted by the stage that produced it. `O(cells + folds)`,
//! at a once-per-run boundary.
//!
//! UNVERIFIED as a measured figure: no bench row covers this yet.

use core::fmt::Write as _;

use crate::bootstrap::Verdict;
use crate::grid::{Cell, Grid};
use crate::pbo::Pbo;
use crate::trade::Trades;
use crate::validate::Validated;

/// Column width for the label side of every row.
const LABEL: usize = 36;

/// One line: a label, a value, and a note.
fn row(out: &mut String, label: &str, value: &str, note: &str) {
    let _ = writeln!(out, "  {label:<LABEL$}{value:>16}  {note}");
}

/// A ratio in tenths of a percent, computed in integers.
///
/// `CLAUDE.md` §7 forbids a float here for the same reason [`crate::report`]
/// gives: an audit that disagreed with the counters it renders would be worse
/// than no audit.
fn permille(part: u64, whole: u64) -> String {
    if whole == 0 {
        return "-".to_owned();
    }
    let tenths = part.saturating_mul(1_000) / whole;
    format!("{}.{}%", tenths / 10, tenths % 10)
}

/// Everything, in one call.
///
/// # Why a single entry point exists
///
/// The sections below are separately callable because a caller may hold only
/// some of the inputs. But an operator asking "how did this run go" wants the
/// whole thing, and making them assemble six calls in the right order is how a
/// section quietly stops being printed — the failure this module was written to
/// remove in the first place.
///
/// Every argument is optional and an absent one is REPORTED rather than
/// skipped. A run with no walk-forward and a run whose walk-forward was omitted
/// from the render must not look the same.
///
/// # What is NOT in these figures, for a spot run
///
/// Nothing. On a spot index there is no brokerage, no STT, no stamp duty and no
/// GST, because a spot index is not tradeable and no order is placed —
/// `costs::scope::is_cost_free` returns true for `IndexSpot` and says so. The
/// only cost that exists is slippage, and it is applied: two ticks per round
/// trip through `costs::fill::worst_case_fills`.
///
/// **That changes when options land.** An option leg carries the whole charge
/// stack, and the figures here would then be gross of it until
/// `costs::trip::price` is wired. This paragraph is the record of which of
/// those two worlds a reader is in.
#[must_use]
pub fn render(
    taken: Option<&Trades>,
    exits: Option<&Grid>,
    folds: Option<&Validated>,
    overfit: Option<&Pbo>,
    boot: Option<(Option<&Verdict>, Option<&Verdict>, usize)>,
    rows: usize,
) -> String {
    let mut out = String::with_capacity(4_096);
    let _ = writeln!(out, "AUDIT");
    let _ = writeln!(
        out,
        "  INDEX SPOT run. There is no brokerage, STT, stamp or GST, because\n  \
         an INDEX is not tradeable: no order is placed, so nothing charges\n  \
         for one.\n\n  \
         THE TWO BLOCKS BELOW PRICE FILLS DIFFERENTLY, AND THIS LINE USED TO\n  \
         CLAIM THEY DID NOT. TRADES carries two ticks a round trip -- a buy a\n  \
         tick above the bar's high and a sell a tick below its low. EXIT GRID\n  \
         carries NO tick: it fills at the extremes the bar actually printed,\n  \
         because on a one-minute series a price a tick outside the bar is one\n  \
         nothing traded at, and naming it would be the invention §3 rule 1\n  \
         forbids. So the grid's totals are a shade kinder than the trade\n  \
         walk's on the same bars, and the difference is exactly two ticks per\n  \
         round trip.\n\n  \
         THIS IS SCOPED TO AN INDEX AND TO NOTHING ELSE. A STOCK spot IS\n  \
         tradeable -- you buy real shares -- so brokerage, STT, stamp, the\n  \
         exchange charge and GST all apply there, and so do they on options.\n  \
         `costs::scope::Segment` has no equity-spot variant today, so that\n  \
         charge path does not exist yet and this header would be WRONG the\n  \
         day a stock is swept."
    );
    let _ = writeln!(out);

    match taken {
        Some(x) => trades(&mut out, x),
        None => absent(&mut out, "TRADES"),
    }
    match exits {
        Some(x) => {
            grid(&mut out, x, rows);
            // THE FULL STRATEGY REPORT FOR THE CHOSEN VARIANT. The grid above is
            // 625 rows wide and answers "which exit"; this answers "and what is
            // that one actually like to trade" -- profit factor, win rate, the
            // largest single loss, the longest losing streak, and the WORST
            // adverse excursion any trade suffered. A grid row cannot carry
            // seventeen more columns, and an operator choosing a strategy needs
            // all of them for the one they picked.
            if let Some(best) = x.best() {
                strategy_report(&mut out, best, &exit_name(best));
            }
        }
        None => absent(&mut out, "EXIT GRID"),
    }
    match folds {
        Some(x) => walk_forward(&mut out, x),
        None => absent(&mut out, "WALK-FORWARD"),
    }
    match overfit {
        Some(x) => overfitting(&mut out, x),
        None => absent(&mut out, "OVERFITTING"),
    }
    match boot {
        Some((rc, spa, named)) => bootstrap(&mut out, rc, spa, named),
        None => absent(&mut out, "BOOTSTRAP"),
    }
    out
}

/// A section whose input the caller did not hold.
///
/// Printed rather than skipped. A run that HAD no walk-forward and a render
/// that was not GIVEN one are different facts, and a silently missing section
/// reads as the first.
fn absent(out: &mut String, section: &str) {
    let _ = writeln!(out, "{section}");
    let _ = writeln!(
        out,
        "  NOT SUPPLIED to this render. Absent from the report is not the same \
         as absent from the run."
    );
    let _ = writeln!(out);
}

/// How the signals of one combination became trades.
///
/// # What this section is for
///
/// `signals` is the number [`crate::outcome::edge`] would have used as its `n`.
/// `trades` is what a trader holding one position actually took. The ratio
/// between them is how much of the old figure was the same position counted
/// again, and on the measured fixture it was **16.5x** at the default horizon.
pub fn trades(out: &mut String, t: &Trades) {
    let _ = writeln!(out, "TRADES");
    row(
        out,
        "signals fired",
        &t.signals.to_string(),
        "bars the mask hit",
    );
    row(
        out,
        "round trips taken",
        &t.count().to_string(),
        &permille(t.count(), t.signals),
    );
    row(
        out,
        "  blocked, position already open",
        &t.while_open.to_string(),
        "one position at a time",
    );
    row(
        out,
        "  too late to enter",
        &t.too_late.to_string(),
        "at or past the 15:10 square-off",
    );
    if !t.reconciles() {
        let _ = writeln!(
            out,
            "  RECONCILIATION FAILED: trades + blocked + too-late does not equal \
             signals. A signal was dropped without being counted."
        );
    }
    let _ = writeln!(out);
}

/// One exit variant's four rungs, as the table's `exit` column shows it.
///
/// `NONE` is the baseline. Otherwise `stop/target/trail`, with `-` for an axis
/// this variant does not use, and `@arm` appended when the trail is ARMED —
/// which is the whole difference between a trailing stop loss and a trailing
/// take profit and so is never left implicit.
///
/// The suffix is omitted rather than rendered `@-` when there is no arm: an
/// un-armed trail is the common row, and a marker on the common row trains the
/// eye to skip the one place the marker matters.
fn exit_name(c: &Cell) -> String {
    let rung = |r: Option<usize>| r.map_or_else(|| "-".to_owned(), |i| i.to_string());
    if c.stop.is_none() && c.target.is_none() && c.tsl.is_none() && c.ttp.is_none() {
        return "NONE".to_owned();
    }
    // stop / target / TSL, then the TTP as `+arm@trail` when one is set.
    //
    // The `+` is what says a cell carries TWO trailing orders. A TSL of 3 with a
    // TTP armed at target rung 1 trailing by rung 0 reads `-/-/3+1@0`: loose
    // trail from entry, tighter trail once it pays. That pair was unreachable
    // until the fifth axis, and it is the thing an operator asks for by name --
    // so the column has to be able to say it.
    let levels = format!("{}/{}/{}", rung(c.stop), rung(c.target), rung(c.tsl));
    match c.ttp {
        Some(t) => format!("{levels}+{}@{}", t.arm, t.trail),
        None => levels,
    }
}

/// One ladder as `index=ppm`, the way the `exit` column indexes it.
///
/// # A RUNG INDEX IS NOT A LEVEL
///
/// The `exit` column reads `2/3/1+1@0`, and every one of those digits is a
/// SUBSCRIPT INTO A LADDER the reader could not see. The ladder is derived from
/// this combination's own excursion distribution — `Ladder::from_excursions`
/// places each rung at a quantile of what the instrument actually did — so rung
/// 2 is a different distance on BANKNIFTY than on NIFTY, a different distance in
/// a quiet month than a loud one, and a different distance for two combinations
/// on the same bars.
///
/// `stop rungs / target rungs: 4 / 4` said how MANY there were and nothing about
/// what they WERE, so "the best variant stops at rung 2" was unactionable: an
/// operator cannot place that stop on a broker screen. Printing `index=ppm`
/// makes every digit in the table resolvable without a second run.
///
/// `O(rungs)`, and `rungs` is fixed by the grid's own construction — four or
/// five, chosen before any bar is read. It is not on the per-bar or
/// per-candidate path golden rule 4 governs.
fn ladder(l: &crate::excursion::Ladder) -> String {
    if l.rungs().is_empty() {
        return "-".to_owned();
    }
    let mut s = String::new();
    for (i, r) in l.rungs().iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        let _ = write!(s, "{i}={r}");
    }
    s
}

/// The rows above the exit table: what was evaluated, what was dropped, what the
/// rung indices mean, and how to read the `exit` column.
///
/// Split out so [`grid`] stays under `clippy::too_many_lines`. Nothing is hidden
/// by the split — these rows are the table's preamble and the table's preamble
/// is one idea.
fn grid_header(out: &mut String, g: &Grid) {
    row(out, "variants evaluated", &g.cells.len().to_string(), "");
    row(
        out,
        "stop rungs / target rungs",
        &format!("{} / {}", g.stops.len(), g.targets.len()),
        "derived from this combination's own excursions",
    );
    // THE LADDERS THEMSELVES, NOT JUST THEIR LENGTHS.
    //
    // These three labels deliberately do NOT begin "stop rungs": the row above
    // starts with exactly that string, and a reader -- or a test -- matching on
    // a line prefix would bind to whichever came first. Distinct prefixes make
    // each row addressable.
    row(
        out,
        "ladder, stops (ppm)",
        &ladder(&g.stops),
        "rung index = ppm from entry",
    );
    row(
        out,
        "ladder, targets (ppm)",
        &ladder(&g.targets),
        "rung index = ppm from entry",
    );
    row(
        out,
        "ladder, trails (ppm)",
        &ladder(&g.trails),
        "give-back from the running peak, for BOTH trailing orders",
    );
    // A DROPPED PATH IS PRINTED, NEVER ONLY COUNTED INTERNALLY.
    //
    // `Grid::refused_paths` is zero on every sound slice, so a non-zero here
    // means the store handed this run a record the engine refused -- and the
    // whole point of dropping rather than skipping is that the operator sees a
    // smaller sample instead of a moved answer. A count kept and not rendered
    // would put it back to being invisible.
    if g.refused_paths > 0 {
        row(
            out,
            "PATHS DROPPED, refused bar",
            &g.refused_paths.to_string(),
            "a bar between entry and exit failed the engine's own bar check, so \
             the whole round trip was dropped rather than priced. Every figure \
             below is over a SMALLER sample, not a corrected one.",
        );
    }
    // THE EXIT COLUMN IS FIVE FIELDS AND WOULD BE UNREADABLE UNNAMED.
    //
    // `2/3/1+0@0` is a stop, a target, a trailing STOP LOSS, and then a trailing
    // TAKE PROFIT with its own arming and trailing rungs. A TSL and a TTP are
    // opposite instruments -- one cuts a position that turns, the other lets a
    // winner run -- and a cell can now carry both. A reader who cannot tell them
    // apart cannot read the table at all, so the key is printed rather than
    // documented somewhere else.
    row(
        out,
        "exit column",
        "stop/target/tsl+arm@ttp",
        "`-` is no rung. The third slot is a trailing STOP LOSS, live from \
         entry. `+a@t` appends a trailing TAKE PROFIT armed at target rung a \
         and trailing by rung t -- and a cell can carry BOTH, which is the \
         loose-then-tight pair.",
    );
}

/// The exit table's column headings.
///
/// Seventeen columns, and every one of them was already computed. `all_mae`,
/// `winner_mfe`, `worst_trade`, `max_drawdown` and `return_over_drawdown` were
/// accumulated per cell and rendered NOWHERE, and the three exit counters split
/// by `crate::grid::count_exit` would have gone the same way. A figure computed
/// and not shown is a figure the operator cannot act on, which is the same
/// outcome as not computing it and costs more.
fn grid_columns(out: &mut String) {
    let _ = writeln!(
        out,
        "  {:<15}{:>7}{:>6}{:>6}{:>6}{:>6}{:>7}{:>6}{:>14}{:>11}{:>12}{:>12}{:>9}{:>13}{:>11}{:>11}{:>9}{:>11}",
        "exit",
        "trades",
        "won",
        "stop",
        "tsl",
        "ttp",
        "target",
        "time",
        "total",
        // BESIDE `total` BECAUSE IT IS PART OF IT, NOT A FOOTNOTE TO IT.
        //
        // `total` is the pessimistic reading, and until this column existed that
        // reading entered every trade at the bar's OPEN -- a best-case entry
        // under a worst-case heading. This is what the worst entry costs, and
        // showing it next to the total is what lets a reader see that the two
        // legs are now charged the same way.
        //
        // Distinct from `unknown` on purpose: this is money the fill model
        // KNOWS was given up, where `unknown` is money whose fate one-minute
        // bars cannot settle either way.
        "fill cost",
        "worst trip",
        "drawdown",
        "ret/DD",
        "unknown",
        "winner MAE",
        "winner MFE",
        "all MAE",
        "mfe/allMAE",
    );
    let _ = writeln!(
        out,
        "  stop+tsl+ttp+target+time = trades. total/fill cost/worst trip/drawdown \
         are paisa; the MAE/MFE columns and ret/DD are ppm.\n  total is the WORST \
         reading of BOTH legs: in at the worst price the bar PRINTED, out \
         at the worse of the two orderings. entry cost is what that entry gave \
         up against the open; unknown is what the ordering could still be worth."
    );
}

/// The `ret/DD` cell, with the zero-drawdown sentinel rendered as words.
///
/// # A SENTINEL IS NOT A MEASUREMENT
///
/// `Cell::return_over_drawdown` returns [`i64::MAX`] when a variant never gave
/// anything back, because that is the best possible reading and there is nothing
/// to divide by. Printed raw it is `9223372036854775807` — nineteen digits in a
/// nine-wide column, so it overruns into `drawdown` and welds two numbers into
/// one unreadable one. Worse than unreadable, it reads as a MEASURED RATIO of
/// nine quintillion, which is not a thing that happened.
///
/// `no DD` is four characters, fits, and says what the sentinel means.
fn ret_dd(c: &Cell) -> String {
    let v = c.return_over_drawdown();
    if v == i64::MAX {
        return "no DD".to_owned();
    }
    v.to_string()
}

/// One cell as one row, `mark` naming the selector that chose it.
///
/// `mark` is empty for an ordinary row. It is how `best()` and `sharpest()` stop
/// being two sentences under a table the reader has to search: the row that IS
/// the answer says so on the row.
fn grid_row(out: &mut String, c: &Cell, mark: &str) {
    let _ = writeln!(
        out,
        "  {:<15}{:>7}{:>6}{:>6}{:>6}{:>6}{:>7}{:>6}{:>14}{:>11}{:>12}{:>12}{:>9}{:>13}{:>11}{:>11}{:>9}{:>11}{}",
        exit_name(c),
        c.trades,
        c.wins,
        c.stopped,
        c.trailed_stop,
        c.trailed_profit,
        c.targeted,
        c.timed_out,
        c.pessimistic,
        c.fill_cost,
        c.worst_trade,
        c.max_drawdown,
        ret_dd(c),
        c.uncertainty(),
        c.winner_mae,
        c.winner_mfe,
        c.all_mae,
        c.edge_ratio(),
        mark,
    );
}

/// Paisa as rupees, with two decimals and Indian 2-2-3 grouping.
///
/// An accounting unit is not an answer. Every money figure in this workspace is
/// a paisa `i64` because `CLAUDE.md` §7 forbids a float wherever a price is
/// compared — right for the engine and wrong for the operator, who should not
/// have to divide by a hundred before knowing whether a run made twenty-four
/// thousand rupees or two hundred and forty-six thousand.
///
/// Integer throughout: rupees and paise separated by division and remainder,
/// never by a float. Grouping is 2-2-3 because that is how a price is read here,
/// so a crore prints as `1,00,00,000`.
fn money(paisa: i64) -> String {
    let negative = paisa < 0;
    // `unsigned_abs` and not `abs`: `i64::MIN` has no positive counterpart and is
    // exactly the value `saturating_add` produces for a variant that lost without
    // bound. A report that panicked on the worst possible result would be the
    // rendering killing the process over the answer.
    let magnitude = paisa.unsigned_abs();
    let digits: Vec<char> = (magnitude / 100).to_string().chars().collect();
    let mut grouped = String::new();
    for (i, ch) in digits.iter().enumerate() {
        let from_right = digits.len().saturating_sub(i);
        if i > 0 && (from_right == 3 || (from_right > 3 && from_right % 2 == 1)) {
            grouped.push(',');
        }
        grouped.push(*ch);
    }
    format!(
        "{}\u{20b9}{grouped}.{:02}",
        if negative { "-" } else { "" },
        magnitude % 100
    )
}

/// The excursion block: how far price moved while a position was open.
///
/// Split from [`strategy_report`] so it stays under `clippy::too_many_lines`,
/// and because it is one idea — the money figures say what a trade RETURNED,
/// these say what it PUT YOU THROUGH to return it.
///
/// **`worst_mae` is the only bound in the whole report.** Every other figure is
/// a summary: a mean of 30 ppm across ten thousand trades is entirely consistent
/// with one trade that went 4,000 ppm against, and that one trade is the one
/// that takes the account out. A stop is placed once and every trade must
/// survive it, so the maximum is the number that decides whether a stop is
/// survivable at all.
fn excursion_block(out: &mut String, cell: &Cell) {
    let _ = writeln!(
        out,
        "\n  EXCURSIONS — how far price moved while a position was open"
    );
    for (label, value, note) in [
        (
            "WORST MAE, any single trade",
            ppm_pct(cell.worst_mae),
            "THE BOUND. No trade in this run went further against than this",
        ),
        (
            "mean MAE, all trades",
            ppm_pct(cell.all_mae),
            "the typical trade, losers included",
        ),
        (
            "mean MAE, winners only",
            ppm_pct(cell.winner_mae),
            "the tightest stop that keeps every winner",
        ),
        (
            "mean MFE, winners only",
            ppm_pct(cell.winner_mfe),
            "how far winners ran",
        ),
    ] {
        let _ = writeln!(out, "  {label:<32}{value:>18}  {note}");
    }
    let _ = writeln!(out);
}

/// The full strategy report for one exit variant, in the shape a trading
/// platform's tester prints it.
///
/// # Why every one of these is here
///
/// The audit printed a net total, a trade count and two excursion means. A
/// platform's strategy tester prints twenty figures, and the difference is not
/// decoration — each one answers a question the net total cannot:
///
/// | figure | the question it answers |
/// |---|---|
/// | profit factor | do the winners outweigh the losers, or did one lucky trade carry it |
/// | win rate | how often is this right |
/// | avg win vs avg loss | is it many small wins or one big one |
/// | **worst MAE** | **did ANY trade go further against me than my stop** |
/// | max losing streak | what must a human sit through |
/// | avg bars held | is this a four-minute trade or a four-hour one |
/// | largest win / loss | is the total carried by an outlier |
///
/// **The worst MAE is the one that decides whether a stop is survivable.** A
/// mean of 30 ppm across ten thousand trades is entirely consistent with one
/// trade that went 4,000 ppm against, and that one trade is the one that takes
/// the account out. Every other figure here is a summary; that one is a bound.
pub fn strategy_report(out: &mut String, cell: &Cell, name: &str) {
    let _ = writeln!(out, "STRATEGY REPORT — exit variant {name}");
    let losers = cell.trades.saturating_sub(cell.wins);
    let pf = cell.profit_factor_bp();

    for (label, value, note) in [
        (
            "net profit, worst-case fills",
            money(cell.pessimistic),
            "what selection ranks on",
        ),
        (
            "net profit, best-case fills",
            money(cell.optimistic),
            "both fills at the open",
        ),
        (
            "gross profit",
            money(cell.gross_win),
            "every winning trade summed",
        ),
        (
            "gross loss",
            money(cell.gross_loss),
            "every losing trade summed",
        ),
        (
            "PROFIT FACTOR",
            if pf == i64::MAX {
                "no losing trade".to_owned()
            } else {
                hundredths(pf)
            },
            "gross profit / gross loss. Below 1.00 the losers win",
        ),
        ("total closed trades", cell.trades.to_string(), ""),
        ("winning trades", cell.wins.to_string(), ""),
        ("losing trades", losers.to_string(), ""),
        (
            "PERCENT PROFITABLE",
            format!("{}%", hundredths(cell.win_rate_bp())),
            "winners / trades",
        ),
        ("avg winning trade", money(cell.avg_win()), ""),
        ("avg losing trade", money(cell.avg_loss()), ""),
        (
            "largest winning trade",
            money(cell.best_trade),
            "is the total carried by one trade?",
        ),
        (
            "largest losing trade",
            money(cell.worst_trade),
            "the single worst round trip",
        ),
        (
            "max drawdown",
            money(cell.max_drawdown),
            "deepest peak-to-trough",
        ),
        (
            "return over drawdown",
            hundredths(cell.return_over_drawdown()),
            "profit per unit of pain",
        ),
        (
            "MAX LOSING STREAK",
            cell.max_losing_streak.to_string(),
            "consecutive losers to sit through",
        ),
        (
            "avg bars held",
            cell.avg_bars_held().to_string(),
            "execution bars, so MINUTES",
        ),
    ] {
        let _ = writeln!(out, "  {label:<32}{value:>18}  {note}");
    }

    excursion_block(out, cell);
}

/// An integer in hundredths, rendered with its decimal point. `250` is `2.50`.
fn hundredths(n: i64) -> String {
    let negative = n < 0;
    let m = n.unsigned_abs();
    format!(
        "{}{}.{:02}",
        if negative { "-" } else { "" },
        m / 100,
        m % 100
    )
}

/// Parts per million as a percentage to two decimals, in integers.
fn ppm_pct(ppm: i64) -> String {
    let negative = ppm < 0;
    let m = ppm.unsigned_abs() / 100;
    format!(
        "{}{}.{:02}%",
        if negative { "-" } else { "" },
        m / 100,
        m % 100
    )
}

/// The exit grid: with levels against without them.
///
/// # The comparison, as a table rather than two runs
///
/// The first row is the baseline — no stop, no target, no TSL and no TTP — and
/// every other row is a level variant. They were computed in one pass over the
/// same trades, so the comparison is exact rather than two runs a reader lines
/// up by hand.
///
/// `keep` bounds the rows printed. Whatever it drops is stated, because a table
/// that quietly showed the best twelve of a hundred would read as the whole
/// grid — and the two rows the selectors chose are printed BELOW the cut when
/// they fall past it, because a table that hides its own answer is worse than a
/// table that is merely short.
pub fn grid(out: &mut String, g: &Grid, keep: usize) {
    let _ = writeln!(out, "EXIT GRID");
    if g.cells.is_empty() {
        row(out, "variants", "-", "the combination took no trades");
        let _ = writeln!(out);
        return;
    }
    grid_header(out, g);
    let _ = writeln!(out);
    grid_columns(out);

    // The baseline first, always, then the rest by pessimistic total. A reader
    // comparing "with levels" against "without" must not have to hunt for the
    // row that is the comparison.
    let mut ordered: Vec<(usize, &Cell)> = g.cells.iter().enumerate().collect();
    ordered.sort_by_key(|&(i, c)| {
        // `arm` is not tested: it cannot be set without a trail, so a cell with
        // no trail has none, and adding the clause would be a guard against a
        // state `crate::grid::evaluate` does not emit.
        let base = c.stop.is_none() && c.target.is_none() && c.tsl.is_none() && c.ttp.is_none();
        // `Reverse`, NOT `-c.pessimistic`, AND THE DIFFERENCE IS AN ABORT.
        //
        // `crate::grid` accumulates this field with `saturating_add`, whose floor
        // is exactly `i64::MIN` -- so the one value the accumulator is DESIGNED to
        // produce when a variant loses without bound is the one value that has no
        // positive counterpart. Under `overflow-checks = true` the negation panics,
        // and the release profile sets `panic = "abort"`, so the process dies with
        // no message, no partial report and no name for what happened. Sorting a
        // results table is not a thing that should be able to kill the process.
        //
        // `Reverse` orders descending with no arithmetic at all, so there is
        // nothing left to overflow. `costs::moneyness` reaches for `checked_neg`
        // at the same hazard; here the negation can be deleted outright rather
        // than guarded, which is the better of the two.
        //
        // THE KEY IS `Grid::best`'s KEY, PLUS THE INDEX, AND THAT IS THE POINT.
        //
        // It used to be `Reverse(c.pessimistic)` alone while `best()` ranked on
        // `(pessimistic, merit)` and settled full ties by `max_by_key`'s
        // last-maximum rule. So the top row of this table and the cell the rest
        // of the report calls the best answer were chosen by DIFFERENT
        // comparators, and on any tie they disagreed -- the table naming one
        // variant while `validate` and `pbo` consumed another. Descending on the
        // index reproduces last-maximum exactly, so row one IS `best()`.
        (
            !base,
            core::cmp::Reverse((c.pessimistic, crate::grid::merit(c), i)),
        )
    });

    // Marks are resolved by IDENTITY, not by re-running the comparator: the
    // selectors return a `&Cell` into `g.cells`, so pointer equality answers
    // "is this row the one it chose" without a second ranking that could differ
    // from the first.
    let best = g.best().map(core::ptr::from_ref);
    let sharp = g.sharpest().map(core::ptr::from_ref);
    let mark_for = |c: &Cell| -> &'static str {
        let p = core::ptr::from_ref(c);
        match (best == Some(p), sharp == Some(p)) {
            (true, true) => "  <- best() AND SHARPEST",
            (true, false) => "  <- best()",
            (false, true) => "  <- SHARPEST",
            (false, false) => "",
        }
    };

    let shown = ordered.len().min(keep);
    for &(_, c) in ordered.iter().take(keep) {
        grid_row(out, c, mark_for(c));
    }
    if ordered.len() > shown {
        let _ = writeln!(
            out,
            "  ... {} further variant(s) NOT SHOWN. The grid is complete; this \
             table is not.",
            ordered.len().saturating_sub(shown)
        );
        // A SELECTOR'S OWN ROW IS NEVER DROPPED BY THE CUT.
        //
        // `keep` is a display bound, and `best()`/`sharpest()` rank the WHOLE
        // grid -- so the row the report names as the answer could sit at
        // position 400 of 625 and be truncated away, leaving a sentence naming a
        // variant no row described. Printing it below the cut costs two lines
        // and removes the one way this table could contradict the report around
        // it.
        for &(_, c) in ordered.iter().skip(shown) {
            let mark = mark_for(c);
            if !mark.is_empty() {
                grid_row(out, c, mark);
            }
        }
    }
    let _ = writeln!(out);

    // THE UNCERTAINTY COLUMN IS NOT AN OPTIMISTIC ESTIMATE. It is the width
    // of what minute bars cannot settle -- the gap between resolving an
    // ambiguous bar as the stop and as the target. Two variants with the same
    // worst-case total and different spreads are not equally trustworthy.
    let unresolved = g
        .cells
        .iter()
        .filter(|c| c.depends_on_unknowable_ordering())
        .count();
    if unresolved > 0 {
        let _ = writeln!(
            out,
            "  {unresolved} variant(s) depend on intra-bar ordering this data \
             cannot settle. The `unknown` column is how much -- it is a
             MEASUREMENT ERROR, not an upside. Only second-level data closes it."
        );
    } else {
        let _ = writeln!(
            out,
            "  No variant depends on intra-bar ordering: every exit was
           unambiguous, so second-level data would change nothing here."
        );
    }
    let _ = writeln!(out);

    // THE SNIPER LINE. Of the variants that survived pessimistic fills AND
    // pessimistic ambiguity, the one whose winners went least against you.
    //
    // NAMED THROUGH `exit_name`. It reported three numbers and never said WHICH
    // VARIANT produced them, so the one row an operator would actually place on
    // a broker screen was described and not identified -- and with `keep`
    // truncating the table there was no way to find it by matching the figures
    // either.
    if let Some(sharp) = g.sharpest() {
        let _ = writeln!(
            out,
            "  SHARPEST: variant {}. Winners went {} ppm against before working, \
             and {} ppm for. Ratio {}. That adverse figure is the tightest stop \
             that would not have killed a winner.",
            exit_name(sharp),
            sharp.winner_mae,
            sharp.winner_mfe,
            sharp.edge_ratio()
        );
    } else {
        let _ = writeln!(
            out,
            "  NO SHARPEST VARIANT: nothing survived pessimistic fills, so there \
             is no setup to call precise. That is a finding, not a gap."
        );
    }
    let _ = writeln!(out);
}

/// Walk-forward: what was chosen on the past, and what it did on the future.
pub fn walk_forward(out: &mut String, v: &Validated) {
    let _ = writeln!(out, "WALK-FORWARD");
    if v.folds.is_empty() {
        row(
            out,
            "folds",
            "-",
            "the slice was too short to split, so nothing was validated",
        );
        let _ = writeln!(out);
        return;
    }
    row(out, "folds", &v.folds.len().to_string(), "");
    row(
        out,
        "  that chose a combination",
        &v.decided().to_string(),
        "",
    );
    // PRINTED, NOT MERELY COUNTED. `halted_folds` exists because the field was
    // recorded per fold and summed nowhere; a sum that is computed and never
    // rendered repeats the same defect one level up. The note is conditional
    // and the row is not: a zero must be visible, or the reader cannot tell a
    // walk that truncated nothing from a build that never checked.
    row(
        out,
        "  that HALTED on a budget",
        &v.halted_folds().to_string(),
        if v.halted_folds() == 0 {
            "every fold went extinct on its own"
        } else {
            "PARTIAL -- these folds ranked a truncated candidate set"
        },
    );
    row(
        out,
        "  still positive out of sample",
        &v.held_up().to_string(),
        "under pessimistic fills",
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  {:<6}{:>10}{:>10}{:>12}{:>14}{:>14}{:>15}",
        "fold", "train", "test", "candidates", "in-sample", "oos no exit", "oos with exit"
    );
    for f in &v.folds {
        let _ = writeln!(
            out,
            "  {:<6}{:>10}{:>10}{:>12}{:>14}{:>14}{:>15}",
            f.index,
            f.train_bars,
            f.test_bars,
            f.considered,
            f.in_sample.worst,
            f.out_of_sample.worst,
            // THE COLUMN `held_up` ACTUALLY JUDGES, and it was not on this table.
            //
            // `held_up` counts a fold as holding up when `out_of_sample_exit` is
            // above zero, falling back to the no-exit walk only when the fold
            // chose no exit. The table printed the NO-EXIT total under a heading
            // reading "out-of-sample", so a run could report "still positive out
            // of sample: 4" above four NEGATIVE figures and look like it was
            // lying. Neither number was wrong; they are different quantities, and
            // nothing said so.
            //
            // Both are on the table now. A dash means the fold chose no exit
            // levels, which is when the two columns are the same measurement.
            f.out_of_sample_exit
                .map_or_else(|| "--".to_owned(), |v| v.to_string())
        );
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  A fold is ONE DRAW. Holding up out of sample on one window is one \
         comparison survived, not a proof -- see OVERFITTING below."
    );
    let _ = writeln!(out);
}

/// The probability that the selection procedure is fooling itself.
pub fn overfitting(out: &mut String, p: &Pbo) {
    let _ = writeln!(out, "OVERFITTING");
    match p.probability() {
        None => {
            row(
                out,
                "PBO",
                "NOT MEASURED",
                "no fold could be ranked -- not the same as zero",
            );
            if p.unrankable > 0 {
                row(
                    out,
                    "  folds with too few candidates",
                    &p.unrankable.to_string(),
                    "one candidate is not a choice",
                );
            }
        }
        Some(ppm) => {
            row(
                out,
                "PBO",
                &format!("{}.{}%", ppm / 10_000, (ppm / 1_000) % 10),
                if p.is_noise() {
                    "AT OR ABOVE 50% -- the selection is a coin flip"
                } else {
                    "below 50% -- selection carries information"
                },
            );
            row(out, "  folds ranked", &p.folds.to_string(), "");
            row(
                out,
                "  winner below median out of sample",
                &p.overfit_folds.to_string(),
                "",
            );
            row(
                out,
                "  median placement",
                &format!(
                    "{}.{}%",
                    p.median_placement / 10_000,
                    (p.median_placement / 1_000) % 10
                ),
                "0% = still best, 100% = worst",
            );
            if p.unrankable > 0 {
                row(
                    out,
                    "  folds NOT ranked",
                    &p.unrankable.to_string(),
                    "excluded from the denominator",
                );
            }
        }
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  A LOW PBO DOES NOT MEAN PROFITABLE. It means the SELECTION is not \
         noise-fitting. A procedure can reliably pick the best of a hundred \
         worthless combinations and score perfectly while losing on every one."
    );
    let _ = writeln!(out);
}

/// The three bootstrap tests, side by side.
pub fn bootstrap(out: &mut String, rc: Option<&Verdict>, spa: Option<&Verdict>, named: usize) {
    let _ = writeln!(out, "BOOTSTRAP");
    let _ = writeln!(
        out,
        "  {:<28}{:>12}{:>12}{:>10}",
        "test", "statistic", "p-value", "clears 5%"
    );
    for (name, v) in [("White's Reality Check", rc), ("Hansen's SPA", spa)] {
        match v {
            Some(x) => {
                let _ = writeln!(
                    out,
                    "  {:<28}{:>12.3}{:>12.4}{:>10}",
                    name,
                    x.statistic,
                    x.p_value,
                    if x.clears() { "yes" } else { "NO" }
                );
            }
            None => {
                let _ = writeln!(out, "  {name:<28}{:>12}{:>12}{:>10}", "-", "REFUSED", "-");
            }
        }
    }
    let _ = writeln!(
        out,
        "  {:<28}{:>12}{:>12}{:>10}",
        "Romano-Wolf (named)", "-", "-", named
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "  The first two ask whether ANYTHING here is real. Only Romano-Wolf \
         says WHICH, and it names {named}."
    );

    // WHAT THE SAMPLE SIZE WAS WORTH, BESIDE THE P-VALUE IT PRODUCED.
    //
    // A p-value keeps no record of how much evidence produced it, so the two
    // rows above render identically at three periods and at three hundred while
    // meaning entirely different things. `docs/06-limits.md` §77 measures the
    // gap -- 37.1% false positives at three periods against a nominal 5%.
    //
    // This crate picks no minimum-period floor: which floor is a number
    // `CLAUDE.md` §3 rule 1 forbids it inventing, with no charter source to take
    // it from. §3 rule 6 asks instead that the limit be stated where it will be
    // read, and a limit recorded only in a document is not stated to the person
    // looking at the verdict. So it prints here. D-0172.
    if let Some(v) = rc.or(spa) {
        let _ = writeln!(out);
        let _ = writeln!(out, "  sample: {}", v.calibration());
        if v.periods < 100 {
            let _ = writeln!(
                out,
                "  READ THE TWO ROWS ABOVE WITH THAT IN MIND -- at this sample \
                 size these tests fire falsely far more often than 5%."
            );
        }
    }
    let _ = writeln!(out);
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {

    /// THE VERDICT'S SAMPLE SIZE IS PRINTED WHERE IT WILL BE READ.
    ///
    /// A limit recorded only in `docs/06-limits.md` is not stated to the person
    /// looking at the p-value. At a short sample the render must say so beside
    /// the number, not somewhere else.
    #[test]
    fn a_short_sample_is_named_beside_the_p_value_it_produced() {
        let short = crate::bootstrap::Verdict {
            statistic: 3.0,
            p_value: 0.01,
            draws: 1_000,
            strategies: 2,
            periods: 3,
        };
        let mut out = String::new();
        bootstrap(&mut out, Some(&short), None, 1);
        assert!(out.contains("37.1%"), "the measured rate is shown:\n{out}");
        assert!(
            out.contains("READ THE TWO ROWS ABOVE"),
            "and a short sample is called out rather than left to the reader:\n{out}"
        );

        let long = crate::bootstrap::Verdict {
            periods: 400,
            ..short
        };
        let mut out = String::new();
        bootstrap(&mut out, Some(&long), None, 1);
        assert!(out.contains("nominal"), "a long sample says so:\n{out}");
        assert!(
            !out.contains("READ THE TWO ROWS ABOVE"),
            "and is NOT warned about -- a warning on every verdict is a warning \
             nobody reads:\n{out}"
        );
    }

    /// A LOSING VARIANT MUST NOT BE ABLE TO KILL THE PROCESS.
    ///
    /// `crate::grid` accumulates `Cell::pessimistic` with `saturating_add`, whose
    /// floor is exactly `i64::MIN`. This function used to sort on
    /// `-c.pessimistic`, and `i64::MIN` has no positive counterpart: under
    /// `overflow-checks = true` that negation panics, and the release profile sets
    /// `panic = "abort"`, so the process would die with no message and no partial
    /// report. The one value the accumulator is DESIGNED to produce on unbounded
    /// loss was the one value the sort key could not take.
    ///
    /// Sorting a results table is not a thing that should be able to abort a
    /// process, so the negation is gone rather than guarded.
    #[test]
    fn the_grid_table_orders_a_saturated_loss_without_negating_it() {
        let cell = |stop: Option<usize>, pess: i64| crate::grid::Cell {
            stop,
            trades: 1,
            pessimistic: pess,
            optimistic: pess,
            ..crate::grid::Cell::default()
        };
        let g = crate::grid::Grid {
            // The base row (no stop, no target, no trail) must still lead, and the
            // two rungs must still order best-first beneath it.
            cells: vec![cell(None, -5), cell(Some(0), i64::MIN), cell(Some(1), 100)],
            signals: 3,
            ..crate::grid::Grid::default()
        };

        let mut out = String::new();
        grid(&mut out, &g, 10);

        assert!(!out.is_empty(), "the table rendered rather than aborting");
        let best = out.find("100").expect("the profitable rung is shown");
        let worst = out
            .find(&i64::MIN.to_string())
            .expect("the saturated rung is shown rather than dropped");
        assert!(
            best < worst,
            "the better rung must sort above the saturated one:\n{out}"
        );
    }
    use super::{bootstrap, grid, overfitting, trades, walk_forward};
    use crate::pbo::{Placement, probability_of_overfitting};
    use crate::trade::{Trade, Trades};
    use crate::validate::Validated;

    /// The value column of a labelled row, so an assertion reads the number
    /// rather than any text that happens to contain it.
    ///
    /// Written because two assertions in `crate::report` were once satisfied by
    /// the wrong thing: `contains("NO")` is true of the string "NOT RECORDED".
    /// A test that passes for the wrong reason is worse than no test.
    fn cell(text: &str, label: &str) -> String {
        text.lines()
            .find(|l| l.trim_start().starts_with(label))
            .map(|l| {
                l.get(2 + super::LABEL..)
                    .unwrap_or("")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_owned()
            })
            .unwrap_or_default()
    }

    #[test]
    fn the_trades_section_shows_how_much_of_the_signal_count_was_overlap() {
        let t = Trades {
            eligible: Vec::new(),
            trades: vec![
                Trade {
                    signal_bar: 0,
                    entry_bar: 1,
                    exit_bar: 5,
                    best: 10,
                    worst: -5,
                    forced: false,
                };
                3
            ],
            signals: 50,
            while_open: 45,
            too_late: 2,
        };
        let mut out = String::new();
        trades(&mut out, &t);
        assert_eq!(cell(&out, "signals fired"), "50");
        assert_eq!(cell(&out, "round trips taken"), "3");
        assert!(
            out.contains("blocked, position already open"),
            "the overlap must be visible, not inferred"
        );
    }

    #[test]
    fn a_walk_that_lost_a_signal_says_so_rather_than_reconciling_silently() {
        // The counts deliberately do not add up. An audit that printed them
        // without noticing would report a walk that dropped a signal as a
        // healthy one.
        let t = Trades {
            eligible: Vec::new(),
            trades: Vec::new(),
            signals: 100,
            while_open: 1,
            too_late: 1,
        };
        let mut out = String::new();
        trades(&mut out, &t);
        assert!(
            out.contains("RECONCILIATION FAILED"),
            "a walk whose counts do not sum must say so"
        );
    }

    #[test]
    fn an_unmeasured_pbo_is_not_rendered_as_zero() {
        // "Never overfit" and "never measured" are different facts, and zero
        // reads as the first. This is the assertion that stops an audit from
        // reporting a perfect score for a test that never ran.
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&[]));
        assert_eq!(cell(&out, "PBO"), "NOT");
        assert!(out.contains("not the same as zero"));
        assert!(
            !out.contains("0.0%"),
            "an unmeasured PBO must not render as a number at all"
        );
    }

    #[test]
    fn a_coin_flip_pbo_is_named_as_one() {
        let folds = [
            Placement {
                candidates: 3,
                winner_rank: 0,
            },
            Placement {
                candidates: 3,
                winner_rank: 2,
            },
        ];
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&folds));
        assert!(
            out.contains("coin flip"),
            "a PBO at or above half must be called what it is"
        );
    }

    #[test]
    fn every_section_states_what_it_cannot_tell_you() {
        // The difference between an audit and a brochure. A reader who takes a
        // true number for a claim it does not support has been misled by it,
        // and the limit has to sit beside the figure.
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&[]));
        assert!(out.contains("DOES NOT MEAN PROFITABLE"));

        let mut w = String::new();
        walk_forward(&mut w, &Validated::default());
        assert!(w.contains("nothing was validated"));

        let mut g = String::new();
        grid(&mut g, &crate::grid::Grid::default(), 10);
        assert!(g.contains("took no trades"));
    }

    #[test]
    fn one_call_renders_every_section_and_names_the_ones_it_was_not_given() {
        // A section that is silently missing reads as "the run did not do this".
        // A run that HAD no walk-forward and a render that was not GIVEN one are
        // different facts, and only one of them is a finding.
        let out = super::render(None, None, None, None, None, 10);
        for section in [
            "TRADES",
            "EXIT GRID",
            "WALK-FORWARD",
            "OVERFITTING",
            "BOOTSTRAP",
        ] {
            assert!(
                out.contains(section),
                "{section} is missing from the render"
            );
        }
        assert_eq!(
            out.matches("NOT SUPPLIED").count(),
            5,
            "every absent section must say it was absent from the RENDER rather \
             than from the run"
        );
    }

    #[test]
    fn an_index_run_says_charges_do_not_exist_and_names_where_they_do() {
        // The caveat I carried for most of a session was WRONG for spot: a spot
        // index is not tradeable, so no order is placed and there is no
        // brokerage, STT, stamp or GST to be gross of. `costs::scope` says so.
        // The header records which of the two worlds a reader is in, because
        // the answer changes the moment options land.
        let out = super::render(None, None, None, None, None, 10);
        assert!(out.contains("no brokerage"));

        // THE TWO BLOCKS PRICE FILLS DIFFERENTLY, AND THE HEADER MUST SAY SO.
        //
        // This asserted `"Slippage IS in these figures"` — a single claim over
        // the whole report, true when both halves used `Anchor::AdverseExtreme`.
        // The exit grid now fills at `PrintedExtreme`, the bar's own high and
        // low with no tick added, because a price a tick outside a one-minute
        // bar is one nothing traded at. `crate::trade::walk` still carries the
        // tick. So one sentence covering both became false for one of them, and
        // the test that would have caught it was asserting the sentence rather
        // than the difference.
        assert!(
            out.contains("two ticks a round trip"),
            "the trade walk's slippage must still be stated"
        );
        assert!(
            out.contains("EXIT GRID\n         carries NO tick") || out.contains("carries NO tick"),
            "and the grid's absence of one must be stated beside it, or a reader \
             compares two totals believing they were priced the same way"
        );

        // THE SCOPE IS THE POINT. "No brokerage" is true of an INDEX and false
        // of a STOCK -- a stock spot is tradeable, you buy real shares, and
        // every charge applies. A header saying "spot" would be silently wrong
        // the day a stock is swept, which is the stale-doc failure this
        // repository keeps finding.
        assert!(
            out.contains("INDEX SPOT run"),
            "the header must name INDEX specifically, never just spot"
        );
        assert!(
            out.contains("STOCK spot IS"),
            "the header must state that a stock spot DOES carry charges"
        );
        assert!(
            out.contains("no equity-spot variant"),
            "the gap must be named: `costs::scope::Segment` cannot represent a \
             stock at all, so the charge path does not exist yet"
        );
    }

    /// A grid with real cells, a baseline and a survivor.
    ///
    /// Built by hand rather than by sweeping, so the numbers are known and the
    /// assertions can be exact. Every earlier test in this module exercised the
    /// EMPTY path -- an empty grid, a default `Validated`, `None` verdicts --
    /// which left the populated renderers at 79% coverage. A renderer that has
    /// never rendered anything is not a tested renderer.
    fn populated_grid() -> crate::grid::Grid {
        crate::grid::Grid {
            cells: vec![
                // The baseline: no levels at all.
                crate::grid::Cell {
                    trades: 40,
                    wins: 18,
                    pessimistic: -900,
                    optimistic: -900,
                    timed_out: 40,
                    winner_mae: 240,
                    winner_mfe: 300,
                    ..crate::grid::Cell::default()
                },
                // A stop-only variant: exercises the "-" rendering for a rung
                // that is absent while another is present.
                crate::grid::Cell {
                    stop: Some(0),
                    trades: 51,
                    wins: 24,
                    pessimistic: 1_021,
                    optimistic: 1_240,
                    stopped: 22,
                    timed_out: 29,
                    winner_mae: 96,
                    winner_mfe: 210,
                    ..crate::grid::Cell::default()
                },
                // A target-and-trail variant with NO stop, so the other two "-"
                // renderings are exercised too.
                crate::grid::Cell {
                    target: Some(0),
                    tsl: Some(1),
                    trades: 44,
                    wins: 20,
                    pessimistic: 300,
                    optimistic: 300,
                    targeted: 25,
                    timed_out: 19,
                    winner_mae: 150,
                    winner_mfe: 200,
                    ..crate::grid::Cell::default()
                },
                // A survivor with sharp winners.
                crate::grid::Cell {
                    stop: Some(0),
                    target: Some(1),
                    trades: 62,
                    wins: 37,
                    pessimistic: 4_880,
                    optimistic: 5_102,
                    stopped: 15,
                    targeted: 30,
                    timed_out: 17,
                    ambiguous_bars: 4,
                    winner_mae: 41,
                    winner_mfe: 380,
                    ..crate::grid::Cell::default()
                },
            ],
            signals: 1_124,
            ..crate::grid::Grid::default()
        }
    }

    #[test]
    fn a_populated_grid_puts_the_baseline_first_and_names_the_sharpest() {
        // The baseline row IS the comparison the operator asked for, so it must
        // never be buried among variants sorted by profit -- a reader must not
        // have to hunt for the row that answers "with levels or without".
        let mut out = String::new();
        grid(&mut out, &populated_grid(), 10);

        let base_at = out.find("NONE").expect("the baseline row must be present");
        let variant_at = out.find("0/1/-").expect("the level row must be present");
        assert!(
            base_at < variant_at,
            "the baseline must be printed before the variants, not sorted among \
             them by profit"
        );
        assert!(
            out.contains("SHARPEST"),
            "a grid with a surviving variant must name it"
        );
        assert!(
            out.contains("41 ppm against"),
            "the sniper figure is the tightest stop that would not have killed a \
             winner, and it must appear as a number"
        );
        assert!(
            out.contains("depend on intra-bar ordering"),
            "four ambiguous bars must be reported as uncertainty, not hidden"
        );
    }

    #[test]
    fn a_grid_where_nothing_survives_says_so_rather_than_naming_a_loser() {
        // `sharpest` returns None when no variant made money under pessimistic
        // fills. Printing the "best of a bad set" would read as a recommendation.
        let losing = crate::grid::Grid {
            cells: vec![crate::grid::Cell {
                trades: 10,
                wins: 2,
                pessimistic: -5_000,
                optimistic: -5_000,
                ..crate::grid::Cell::default()
            }],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &losing, 10);
        assert!(out.contains("NO SHARPEST VARIANT"));
        assert!(
            out.contains("a finding, not a gap"),
            "nothing surviving is a result, and must not read as missing data"
        );
    }

    #[test]
    fn a_populated_walk_forward_prints_a_row_per_fold() {
        let v = Validated {
            folds: vec![
                crate::validate::FoldResult {
                    index: 0,
                    train_bars: 1_000,
                    purged: 15,
                    test_bars: 500,
                    considered: 207,
                    priced: 207,
                    halted: None,
                    chosen_exit: Some(crate::grid::Chosen {
                        stop: Some(0),
                        ..crate::grid::Chosen::default()
                    }),
                    chosen_exit_total: Some(1_000),
                    out_of_sample_exit: Some(500),
                    chosen: Some(vocab::ConditionMask::default()),
                    in_sample: crate::validate::Summary {
                        trades: 30,
                        best: 900,
                        worst: 400,
                        forced: 2,
                    },
                    out_of_sample: crate::validate::Summary {
                        trades: 12,
                        best: 300,
                        worst: -150,
                        forced: 1,
                    },
                    ..crate::validate::FoldResult::default()
                },
                crate::validate::FoldResult {
                    index: 1,
                    train_bars: 2_000,
                    purged: 15,
                    test_bars: 500,
                    considered: 311,
                    priced: 311,
                    halted: None,
                    chosen_exit: Some(crate::grid::Chosen {
                        stop: Some(1),
                        target: Some(2),
                        ..crate::grid::Chosen::default()
                    }),
                    chosen_exit_total: Some(2_000),
                    out_of_sample_exit: Some(-500),
                    chosen: Some(vocab::ConditionMask::default()),
                    in_sample: crate::validate::Summary {
                        trades: 55,
                        best: 1_400,
                        worst: 800,
                        forced: 3,
                    },
                    out_of_sample: crate::validate::Summary {
                        trades: 20,
                        best: 600,
                        worst: 220,
                        forced: 2,
                    },
                    ..crate::validate::FoldResult::default()
                },
            ],
        };
        let mut out = String::new();
        walk_forward(&mut out, &v);

        assert_eq!(cell(&out, "folds"), "2");
        assert_eq!(cell(&out, "still positive out of sample"), "1");
        // THE ROW THIS ASSERTED IS GONE, and its absence is the fix rather than
        // a regression. It read "candidates NOT ranked 44" and its message here
        // said a search that looks at the first N and calls it exhaustive is the
        // defect. That was right about the defect and wrong about the remedy:
        // the budget was reporting the count it discarded while discarding the
        // best candidate along with it, so the honest-looking line was what made
        // the wrong answer survivable. The cap is deleted, every candidate is
        // priced, and `runner::validate::the_chosen_combination_is_the_best_of_every_candidate_and_not_of_a_prefix`
        // holds the property this row used to gesture at.
        assert!(
            !out.contains("candidates NOT ranked"),
            "the candidate budget no longer exists, so nothing may report one"
        );
        assert!(
            out.contains("ONE DRAW"),
            "the walk must state that one fold is not a proof"
        );
    }

    #[test]
    fn a_measured_pbo_prints_its_denominator_and_its_median() {
        // A PBO over three usable folds out of ten is a different number from
        // one over ten, and a reader who cannot see the denominator cannot tell
        // them apart.
        let folds = [
            Placement {
                candidates: 5,
                winner_rank: 0,
            },
            Placement {
                candidates: 5,
                winner_rank: 1,
            },
            Placement {
                candidates: 5,
                winner_rank: 4,
            },
            Placement {
                candidates: 1,
                winner_rank: 0,
            },
        ];
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&folds));

        assert_eq!(cell(&out, "folds ranked"), "3");
        assert!(
            out.contains("folds NOT ranked"),
            "the unrankable fold must be named and excluded from the denominator"
        );
        assert!(out.contains("median placement"));
        assert!(
            out.contains("selection carries information"),
            "one of three below median is under half, so the procedure is not a \
             coin flip and must be described as such"
        );
    }

    #[test]
    fn a_completed_bootstrap_prints_each_test_and_whether_it_cleared() {
        let clears = crate::bootstrap::Verdict {
            statistic: 3.42,
            p_value: 0.004,
            draws: 1_000,
            periods: 250,
            strategies: 20,
        };
        let fails = crate::bootstrap::Verdict {
            statistic: 0.81,
            p_value: 0.612,
            draws: 1_000,
            periods: 250,
            strategies: 20,
        };
        let mut out = String::new();
        bootstrap(&mut out, Some(&clears), Some(&fails), 2);

        assert!(out.contains("White's Reality Check"));
        assert!(out.contains("Hansen's SPA"));
        assert!(
            out.contains("0.0040"),
            "the p-value must be printed to enough places to be read"
        );
        assert!(
            out.contains("Only Romano-Wolf \nsays WHICH") || out.contains("says WHICH"),
            "the report must say which test answers which question"
        );
    }

    #[test]
    fn one_call_with_everything_supplied_renders_every_section_populated() {
        // The single entry point, exercised with real inputs rather than None.
        // `render(None, ...)` proved the absent path; this proves the present
        // one, and between them every branch of the dispatcher is taken.
        let taken = Trades {
            eligible: Vec::new(),
            trades: vec![
                Trade {
                    signal_bar: 0,
                    entry_bar: 1,
                    exit_bar: 5,
                    best: 10,
                    worst: -5,
                    forced: false,
                };
                3
            ],
            signals: 50,
            while_open: 45,
            too_late: 2,
        };
        let grid_in = populated_grid();
        let folds = Validated::default();
        let over = probability_of_overfitting(&[]);
        let out = super::render(
            Some(&taken),
            Some(&grid_in),
            Some(&folds),
            Some(&over),
            Some((None, None, 0)),
            10,
        );

        assert!(
            !out.contains("NOT SUPPLIED"),
            "every section was supplied, so none may claim otherwise"
        );
        for section in [
            "TRADES",
            "EXIT GRID",
            "WALK-FORWARD",
            "OVERFITTING",
            "BOOTSTRAP",
        ] {
            assert!(out.contains(section), "{section} missing");
        }
    }

    #[test]
    fn a_refused_bootstrap_is_shown_as_refused_and_never_as_a_pass() {
        let mut out = String::new();
        bootstrap(&mut out, None, None, 0);
        assert!(out.contains("REFUSED"));
        assert!(
            !out.contains("yes"),
            "a test that was refused must never render as clearing"
        );
    }

    #[test]
    fn a_grid_table_that_hides_rows_says_how_many() {
        // A table showing the best twelve of a hundred reads as the whole grid
        // unless it says otherwise. `CLAUDE.md` section 4: never silently.
        let g = crate::grid::Grid {
            cells: vec![crate::grid::Cell::default(); 30],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 5);
        assert!(
            out.contains("NOT SHOWN"),
            "a truncated table must name what it dropped"
        );
        assert!(out.contains("25 further"));
    }

    /// Is this rendered line a row of the exit grid, rather than a heading or a
    /// line of the units note?
    ///
    /// # Why three tests navigate by this instead of by a line offset
    ///
    /// They used to count lines from the headings — `head + 2` for the first
    /// row, `head + 3` for the top variant. That encodes the units note's LINE
    /// COUNT as a constant, in three places, with nothing declaring it. The day
    /// the note grew a second line explaining what `total` now means, all three
    /// failed at once and pointed at the table's alignment, which was correct.
    ///
    /// The shape test cannot drift the same way: every grid row carries the exit
    /// name and then the trade count, so its second whitespace-separated token
    /// is an integer. No heading or prose line has that shape — the units note's
    /// second token is `=` on one line and `is` on the other.
    fn is_grid_row(line: &str) -> bool {
        line.starts_with("  ")
            && line
                .split_whitespace()
                .nth(1)
                .is_some_and(|t| t.parse::<i64>().is_ok())
    }

    /// The exit table's heading line and its data rows must line up.
    ///
    /// # Why a test and not a shared constant
    ///
    /// `grid_columns` and `grid_row` carry two copies of one per-field
    /// width spec — eighteen since `fill cost` joined them — and they CANNOT share a constant: `writeln!` requires a
    /// literal format string, so a `const GRID_FMT` is not expressible in Rust.
    /// The duplication is unavoidable, and an unavoidable duplication with
    /// nothing checking it is a table whose headings sit over the wrong numbers
    /// — a defect that renders perfectly and reads as data.
    ///
    /// `COLUMNS` below is a third copy and the only one that is CHECKED, which
    /// is what makes it a specification rather than another duplicate. Two
    /// properties are measured, and the second is the one that already bit:
    ///
    /// 1. every heading ends flush at its field's last column, so a width
    ///    edited in one renderer and not the other fails here;
    /// 2. every field has a space before its value, so the widest number a
    ///    column can hold still leaves a separator. `unknown` was ten wide and
    ///    `uncertainty()` returned `1111111110` — ten digits, no space, welded
    ///    to `ret/DD`. Two numbers with nothing between them are one unreadable
    ///    number, and the report would have shipped that way.
    #[test]
    fn every_exit_column_heading_ends_where_its_number_ends() {
        // The spec, first thing in the function: `clippy::items_after_statements`
        // and, more usefully, a reader who wants to know what the table claims
        // to be does not have to scroll past a fixture to find out.
        const COLUMNS: [(&str, usize); 18] = [
            ("exit", 15),
            ("trades", 7),
            ("won", 6),
            ("stop", 6),
            ("tsl", 6),
            ("ttp", 6),
            ("target", 7),
            ("time", 6),
            ("total", 14),
            ("fill cost", 11),
            ("worst trip", 12),
            ("drawdown", 12),
            ("ret/DD", 9),
            ("unknown", 13),
            ("winner MAE", 11),
            ("winner MFE", 11),
            ("all MAE", 9),
            ("mfe/allMAE", 11),
        ];
        // Values wide enough to force the separator check to mean something. A
        // default cell prints zeros, which fit any width and prove nothing.
        let loud = crate::grid::Cell {
            trades: 123_456,
            wins: 65_432,
            stopped: 12_345,
            trailed_stop: 9_876,
            trailed_profit: 5_432,
            targeted: 21_098,
            timed_out: 74_705,
            pessimistic: -987_654_321,
            optimistic: 123_456_789,
            // Ten digits in an eleven-wide field: one space left, which is the
            // separator property this fixture exists to squeeze.
            fill_cost: 1_234_567_890,
            worst_trade: -87_654_321,
            max_drawdown: 76_543_210,
            winner_mae: 65_432,
            winner_mfe: 543_210,
            all_mae: 98_765,
            ..crate::grid::Cell::default()
        };
        // A SECOND ROW THAT EXERCISES THE SENTINEL. `return_over_drawdown`
        // returns `i64::MAX` when a variant never gave anything back, and
        // printed raw that is nineteen digits in a nine-wide column. A fixture
        // with a non-zero drawdown never reaches it, which is exactly how the
        // defect survived the first version of this test.
        let smooth = crate::grid::Cell {
            stop: Some(1),
            trades: 40,
            wins: 40,
            pessimistic: 4_242,
            optimistic: 4_242,
            max_drawdown: 0,
            ..crate::grid::Cell::default()
        };
        let g = crate::grid::Grid {
            cells: vec![loud, smooth],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 64);

        let lines: Vec<&str> = out.lines().collect();
        let head = lines
            .iter()
            .position(|l| l.contains("mfe/allMAE"))
            .expect("the exit table must print its headings");
        let heading = *lines.get(head).expect("heading line");
        // EVERY row is checked -- a column only overflows on the value that is
        // widest, and that value is rarely on row one.
        //
        // FOUND BY SHAPE AND NOT BY OFFSET, WHICH IS A REPAIR.
        //
        // This skipped a fixed two lines past the headings, so the day the units
        // note grew from one line to two, this test and the two below all failed
        // together -- pointing at the table's alignment, which was fine, rather
        // than at the note, which had moved. An offset into rendered prose is a
        // dependency on the prose's line count that nothing declares.
        let rows: Vec<&str> = lines
            .iter()
            .skip(head)
            .filter(|l| is_grid_row(l))
            .copied()
            .collect();
        assert_eq!(rows.len(), 2, "both fixture cells must render:\n{out}");

        let h: Vec<char> = heading.chars().collect();
        let mut start = 2_usize; // the two-space indent both lines carry
        for (name, width) in COLUMNS {
            let end = start.saturating_add(width);
            if start > 2 {
                let cut: String = h
                    .get(end.saturating_sub(name.len())..end)
                    .expect("the heading line is shorter than the spec says")
                    .iter()
                    .collect();
                assert_eq!(
                    cut, name,
                    "heading `{name}` does not end at column {end}\n{heading}"
                );
                for data in &rows {
                    let d: Vec<char> = data.chars().collect();
                    assert_eq!(
                        d.get(start),
                        Some(&' '),
                        "column `{name}` has no space before its value -- it \
                         ran into the column on its left\n{heading}\n{data}"
                    );
                }
                assert_eq!(
                    h.get(start),
                    Some(&' '),
                    "heading `{name}` fills its field and touches the one \
                     before it\n{heading}"
                );
            }
            start = end;
        }
    }

    /// A cell that carries a stop, is profitable, and survives — so both
    /// selectors have something to point at.
    fn winner(
        stop: Option<usize>,
        pessimistic: i64,
        mfe: crate::excursion::Ppm,
    ) -> crate::grid::Cell {
        crate::grid::Cell {
            stop,
            trades: 10,
            wins: 7,
            pessimistic,
            optimistic: pessimistic,
            winner_mae: 100,
            winner_mfe: mfe,
            all_mae: 100,
            ..crate::grid::Cell::default()
        }
    }

    /// The rung ladders are printed, not just their lengths.
    ///
    /// # The digit was a subscript into a table nobody could see
    ///
    /// `exit` reads `2/3/-`, and 2 is an index into a ladder derived from this
    /// combination's own excursions — so it is a different distance on every
    /// instrument, every month and every combination. "Stop at rung 2" is not
    /// something an operator can place on a broker screen.
    #[test]
    fn the_rung_ladders_are_printed_as_index_equals_ppm() {
        let g = crate::grid::Grid {
            cells: vec![crate::grid::Cell::default()],
            stops: crate::excursion::Ladder::new(vec![300, 700]).expect("ascending"),
            targets: crate::excursion::Ladder::new(vec![900]).expect("ascending"),
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 8);
        assert!(
            out.contains("0=300 1=700"),
            "the stop ladder must resolve:\n{out}"
        );
        assert!(
            out.contains("0=900"),
            "the target ladder must resolve:\n{out}"
        );
        // An empty ladder renders as `-` rather than as a blank that reads like
        // a missing row. `trails` is unset on this fixture.
        assert!(
            out.contains("ladder, trails (ppm)"),
            "an unset ladder still gets its row:\n{out}"
        );
        // The three labels must not collide with the count row's prefix, or a
        // reader matching on a line prefix binds to whichever came first.
        assert!(out.contains("stop rungs / target rungs"));
        for label in ["ladder, stops", "ladder, targets", "ladder, trails"] {
            assert!(
                !label.starts_with("stop rungs"),
                "{label} shadows the count row's prefix"
            );
        }
    }

    /// The row the rest of the report calls the answer says so on the row.
    #[test]
    fn the_rows_the_selectors_chose_are_marked_in_the_table() {
        let g = crate::grid::Grid {
            cells: vec![
                crate::grid::Cell::default(),
                winner(Some(0), 500, 900),
                winner(Some(1), 900, 400),
            ],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 8);
        assert!(out.contains("<- best()"), "best() must be marked:\n{out}");
        assert!(
            out.contains("<- SHARPEST"),
            "sharpest must be marked:\n{out}"
        );
        // The two selectors rank on different keys, so on this fixture they
        // choose DIFFERENT cells -- 900 total against 900 mfe. A mark that
        // named one row twice would prove nothing about either.
        assert!(
            !out.contains("best() AND SHARPEST"),
            "this fixture is built so the two disagree:\n{out}"
        );
    }

    /// Both selectors on one cell get one combined mark, not two rows.
    #[test]
    fn one_cell_chosen_by_both_selectors_is_marked_once() {
        let g = crate::grid::Grid {
            cells: vec![crate::grid::Cell::default(), winner(Some(0), 900, 900)],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 8);
        assert_eq!(
            out.matches("best() AND SHARPEST").count(),
            1,
            "one cell, one combined mark:\n{out}"
        );
    }

    /// `keep` is a display bound and must never hide the report's own answer.
    #[test]
    fn a_chosen_row_below_the_cut_is_printed_anyway() {
        // The baseline sorts first unconditionally, and `keep` is 1 -- so the
        // only row the cut admits is the baseline, and both selectors chose
        // something else. Without the rescue the report names a variant that no
        // row in the table describes.
        let g = crate::grid::Grid {
            cells: vec![crate::grid::Cell::default(), winner(Some(2), 900, 900)],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 1);
        assert!(
            out.contains("NOT SHOWN"),
            "the cut must be disclosed:\n{out}"
        );
        assert!(
            out.contains("best() AND SHARPEST"),
            "a truncated table must still print the row it calls the answer:\n{out}"
        );
        // And it must be that cell's own row, identified by its exit name.
        assert!(
            out.contains("2/-/-"),
            "the rescued row must carry the variant's exit name:\n{out}"
        );
    }

    /// The sniper line identifies the variant it describes.
    #[test]
    fn the_sharpest_line_names_the_variant_and_not_only_its_numbers() {
        let g = crate::grid::Grid {
            cells: vec![crate::grid::Cell::default(), winner(Some(3), 900, 900)],
            ..crate::grid::Grid::default()
        };
        let mut out = String::new();
        grid(&mut out, &g, 8);
        assert!(
            out.contains("SHARPEST: variant 3/-/-"),
            "three numbers with no variant name are unactionable:\n{out}"
        );
    }

    /// The table's top variant row IS `Grid::best()`, ties included.
    ///
    /// # A surviving mutant is what put this here
    ///
    /// The sort key was `Reverse(c.pessimistic)` while `best()` ranked on
    /// `(pessimistic, merit)` and settled full ties by `max_by_key`'s
    /// LAST-maximum rule. Two different comparators over one grid: on any tie
    /// the table's top row and the cell `validate` and `pbo` actually consume
    /// were different cells, and the report named one while the pipeline used
    /// the other.
    ///
    /// Adding `merit` was not enough — dropping the trailing index from the key
    /// left every audit test green, because `sort_by_key` is STABLE and keeps
    /// the FIRST of a tie while `max_by_key` returns the LAST. This fixture is
    /// two cells tied on both terms so only the index can separate them.
    #[test]
    fn the_top_variant_row_is_the_cell_best_actually_returns() {
        // Same total, same claimed-rung count, so `(pessimistic, merit)` ties
        // and `best()` takes the later of the two.
        let g = crate::grid::Grid {
            cells: vec![
                crate::grid::Cell::default(),
                winner(Some(4), 900, 900),
                winner(Some(7), 900, 900),
            ],
            ..crate::grid::Grid::default()
        };
        let chosen = super::exit_name(g.best().expect("a non-empty grid has a best"));
        assert_eq!(chosen, "7/-/-", "the fixture must tie so the index decides");

        let mut out = String::new();
        grid(&mut out, &g, 8);
        let lines: Vec<&str> = out.lines().collect();
        let head = lines
            .iter()
            .position(|l| l.contains("mfe/allMAE"))
            .expect("headings");
        // head+0 headings, head+1 units note, head+2 the baseline row (which
        // sorts first unconditionally), head+3 the top VARIANT row.
        // The SECOND grid row: the baseline sorts first unconditionally. Found by
        // shape rather than by a fixed offset past the units note -- see
        // `is_grid_row` for what a fixed offset cost.
        let top = lines
            .iter()
            .skip(head)
            .filter(|l| is_grid_row(l))
            .nth(1)
            .copied()
            .expect("a variant row");
        assert!(
            top.trim_start().starts_with(&chosen),
            "the top variant row must be the cell `best()` returns, not the \
             other side of the tie\nbest(): {chosen}\nrow:    {top}\n{out}"
        );
        assert!(
            top.contains("<- best()"),
            "and it must be the row carrying the mark\n{top}"
        );
    }

    /// Two variants tied on money: the table leads with the SIMPLER one.
    ///
    /// # The second surviving mutant this table produced
    ///
    /// Deleting `merit` from the sort key left every other audit test green,
    /// because the tie fixture beside this one ties on merit as well and only
    /// the index separates its cells. So the table was free to lead with the
    /// more decorated of two identical results while `Grid::best` — which does
    /// carry `merit` — returned the other.
    ///
    /// That is not cosmetic. `crate::grid::merit`'s own doc records the measured
    /// case: a trail whose rung the path never reached leaves its cell
    /// byte-identical to the plain cell it decorates, they tie, and naming the
    /// decorated one advertises a mechanism that never fired. 64 of 240 grids on
    /// `sessions(12)`. The table has to break that tie the same way, or it
    /// re-creates the defect one layer up in the only place the operator looks.
    #[test]
    fn the_top_row_prefers_the_simpler_of_two_variants_tied_on_money() {
        let plain = winner(Some(4), 900, 900);
        let decorated = crate::grid::Cell {
            target: Some(2),
            tsl: Some(1),
            ..winner(Some(4), 900, 900)
        };
        // The decorated cell sits LATER, so an index-only tie-break would put it
        // first and a merit-carrying one would not.
        let g = crate::grid::Grid {
            cells: vec![crate::grid::Cell::default(), plain, decorated],
            ..crate::grid::Grid::default()
        };
        let chosen = super::exit_name(g.best().expect("a non-empty grid has a best"));
        assert_eq!(
            chosen, "4/-/-",
            "the fixture must tie on money so merit decides"
        );

        let mut out = String::new();
        grid(&mut out, &g, 8);
        let lines: Vec<&str> = out.lines().collect();
        let head = lines
            .iter()
            .position(|l| l.contains("mfe/allMAE"))
            .expect("headings");
        // The SECOND grid row: the baseline sorts first unconditionally. Found by
        // shape rather than by a fixed offset past the units note -- see
        // `is_grid_row` for what a fixed offset cost.
        let top = lines
            .iter()
            .skip(head)
            .filter(|l| is_grid_row(l))
            .nth(1)
            .copied()
            .expect("a variant row");
        assert!(
            top.trim_start().starts_with(&chosen),
            "the table must lead with the variant that reached the same total \
             with less machinery\nbest(): {chosen}\nrow:    {top}\n{out}"
        );
    }
}
