//! The audit surface: everything the newer stages measured, rendered so a human
//! can read it without opening the code.
//!
//! # The gap this closes
//!
//! [`crate::report`] renders the SWEEP, the BARS, the LADDER, the SIGNIFICANCE
//! bar and the FINDINGS. It was written before any of the execution stages
//! existed and it shows **none** of them: not the trades, not the exit grid, not
//! the sniper measure, not the walk-forward, not the legacy bottom-half rate,
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
//! A reader who takes an anchored-fold bottom-half rate as genuine CSCV/PBO has
//! been misled by a true number under the wrong name. The limit therefore sits
//! beside the figure rather than only in a document they may not open.
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

use brutex_core::instrument::Kind;

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

/// Which charge statement leads the audit: decided by what the swept
/// instrument IS, never by anything its bars say.
///
/// # The header used to have no way to know
///
/// [`render_selected`] took no instrument and printed one header for every
/// run: *"INDEX SPOT run. There is no brokerage, STT, stamp or GST"*. That is
/// true of an index level, which cannot be bought, and it was the only kind of
/// run there was. D-0506 then widened the sweep to the cash equities of the F&O
/// underlyings, and the same sentence went on being printed over a stock -- a
/// real share trade, which pays every one of those charges. The header's own
/// last paragraph predicted it: *"this header would be WRONG the day a stock is
/// swept"*.
///
/// # Why two variants and not the instrument kind
///
/// A futures or options contract is never swept (`CLAUDE.md` §1), and neither
/// statement below describes one. [`CostScope::of`] therefore answers `None`
/// for a contract, so a caller holding one must refuse rather than borrow a
/// label written for something else. D-0681.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostScope {
    /// A spot index level. It is not tradeable, so no charge exists for the
    /// figures to be gross of. The header is the one every index run has
    /// always carried, byte for byte.
    IndexSpot,
    /// A cash equity. Every charge on a share trade exists, this engine has no
    /// path that computes any of them, and so every figure -- including the
    /// ranking that chose the combination -- is GROSS OF EVERY CHARGE.
    CashEquity,
}

impl CostScope {
    /// The scope of a swept instrument kind, or `None` for a contract.
    ///
    /// One `match` on a `Copy` discriminant; no bar is read, so the answer is
    /// the same before the first bar as after the last.
    #[must_use]
    pub const fn of(kind: Kind) -> Option<Self> {
        match kind {
            Kind::Index => Some(Self::IndexSpot),
            Kind::Equity => Some(Self::CashEquity),
            Kind::Future { .. } | Kind::Option { .. } => None,
        }
    }

    /// The charge and corporate-action statement a report that is NOT an
    /// audit carries for this scope, or nothing for an index. D-0694.
    ///
    /// An audit opens with the full header [`render`] prints. Every other
    /// report that ranks a cash equity -- a stored sweep, a range table, a
    /// descent, a threshold search, a saved top list -- carries this shorter
    /// statement instead: the same GROSS OF EVERY CHARGE fact D-0681 put in
    /// the audit, then [`CORPORATE_ACTIONS_UNCHECKED`]. `CLAUDE.md` §1 asks
    /// for the first on every report that ranks an equity; the operator's
    /// answer recorded in D-0694 asks for the second beside it.
    ///
    /// Empty for an index, and that is the point rather than an omission: an
    /// index never splits (D-0018), and every index report stays byte for
    /// byte what it was.
    #[must_use]
    pub fn report_note(self) -> String {
        match self {
            Self::IndexSpot => String::new(),
            Self::CashEquity => format!("{CASH_EQUITY_GROSS}\n\n{CORPORATE_ACTIONS_UNCHECKED}\n\n"),
        }
    }
}

/// What a cash-equity report that is not an audit states about charges.
/// D-0694.
///
/// The one-paragraph form of what [`equity_header`] says at length: every
/// total is gross of every charge a share trade pays, and the result is
/// cost-excluded research with no Selection V6 or execution authority. Its
/// cost-excluded sentence is the header's own, word for word, as D-0696 asks
/// of every copy; the first version of this paragraph left out the Selection
/// V6 clause (AF-19). It names no rate, for the reason the audit header names
/// none: `docs/00-charter.md` records no source for one.
pub const CASH_EQUITY_GROSS: &str = "  CASH EQUITY. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE: brokerage,\n  \
     STT, stamp duty, exchange charges, the SEBI fee, the IPFT, DP charges\n  \
     and GST (an UNVERIFIED list) apply to a share trade and none is\n  \
     subtracted. COST-EXCLUDED RESEARCH, NOT A NET\n  \
     RESULT (D-0509, D-0525, D-0681). No equity result carries Selection V6\n  \
     or execution authority until a charter-sourced equity charge stack\n  \
     exists.";

/// What every report that ranks or audits a cash equity states about
/// corporate actions. D-0694.
///
/// D-0018 requires a suspected split or bonus -- an unexplained overnight
/// gap beyond a threshold -- to refuse its window and name the date.
/// `docs/00-charter.md` names no verified split-and-bonus source and D-0018
/// names no number, so no threshold exists and no detector runs
/// (`docs/06-limits.md` §41.3). An unadjusted 1:5 split is a fake 80%
/// overnight crash and every bar-shape condition fires on it.
///
/// The operator chose, on 2026-09-23, to keep ranking equities for discovery
/// and to say so on every such report rather than refuse them. This is the
/// sentence that says so. An index never splits, so no index report carries
/// it.
///
/// It names every action kind that moves a stock's open with no market cause:
/// split, bonus, rights issue, face-value change, demerger and dividend. It
/// named only "split, bonus or demerger" until numeric-pass16 p16num-3
/// (D-1969); an ex-dividend or ex-rights open is the same fake gap, smaller.
/// A dividend never enters P&L, because every trade has exited by its own
/// session's close and so never holds through an ex-date (`docs/06-limits.md`,
/// the D-0694 section, says so); the ex-date gap still reaches the conditions.
pub const CORPORATE_ACTIONS_UNCHECKED: &str = "  CORPORATE ACTIONS ARE UNCHECKED (D-0018, D-0694). No split, bonus,\n  \
     rights issue, face-value change, demerger or dividend detection has\n  \
     run over these bars. An overnight jump in them\n  \
     can be a corporate action rather than a market move. D-0018\n  \
     requires such a window to be refused with its date named; no\n  \
     threshold for that detector is sourced, so none was applied.";

/// The largest move from one session's last close to the next session's
/// first open, in a slice of bars. D-1540 (audit-20261003 gaps-6).
///
/// # Why a measurement and not a detector
///
/// D-0018 asks for a suspected split or bonus to refuse its window, and
/// names no threshold; `docs/00-charter.md` names no corporate-action
/// source. So nothing here decides that a move IS a corporate action. It
/// names the single largest overnight move by its session and its size, so a
/// reader of a stock's report can check that one date against the
/// exchange's record instead of being told only that nothing was checked.
///
/// # What a split does and does not do to these results
///
/// Measured over generated bars with every price halved from one session on
/// (D-1540): every trade is intraday with a forced square-off, so no trade
/// spans the split and no trade's P&L is inflated by it; post-split trades
/// are in the new price scale, so a paisa total mixes two scales. What the
/// split DOES distort is every condition that reads a previous session --
/// pivots, previous-day high and low, gap levels, the five-session ladder,
/// EMA200 -- for days after it. A ranked mask can therefore be selected on
/// a fake signal while its trades are real ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OvernightMove {
    /// The IST day (days since 1970-01-01, [`indicators::ist_day`]) of the
    /// session that OPENED at [`Self::open`].
    pub day: i64,
    /// The previous session's last close, in paisa.
    pub prior_close: i64,
    /// This session's first open, in paisa.
    pub open: i64,
    /// `(open - prior_close) / prior_close`, in parts per million, truncated
    /// toward zero.
    pub ppm: i64,
}

/// The [`OvernightMove`] of largest magnitude in `bars`, or `None` when the
/// bars hold fewer than two sessions with a positive prior close.
///
/// A tie keeps the earlier session, so the answer is a pure function of the
/// bars. One pass, O(bars), at a once-per-report boundary.
#[must_use]
pub fn largest_overnight_move(bars: &[indicators::Candle]) -> Option<OvernightMove> {
    largest_overnight_move_after(None, bars)
}

/// [`largest_overnight_move`], with the session BEFORE `bars` measured too.
///
/// # The overnight into the first signal day (p16num-1, D-2546)
///
/// `bars.windows(2)` measures only the overnights INSIDE the slice, so the
/// move from the last session before the span to the span's first open was
/// never measured — while the anchored previous-session families read exactly
/// that session out of the warm-up month. A split effective on a span's first
/// session lit `gap_down_day` and shifted every previous-day level, and the
/// D-1540 line named a different, harmless date.
///
/// `prior` is that earlier session's LAST record (a daily bar does: its
/// `close` is the session close); it is measured against `bars`' first open
/// as one more pair at the front, under the same rules — a different IST day,
/// a positive close, a tie keeps the earlier session. A `prior` on or after
/// the first bar's day is ignored rather than trusted. One extra pair, so
/// still one pass, O(bars), at a once-per-report boundary.
#[must_use]
pub fn largest_overnight_move_after(
    prior: Option<&indicators::Candle>,
    bars: &[indicators::Candle],
) -> Option<OvernightMove> {
    let mut best: Option<OvernightMove> = None;
    if let (Some(before), Some(first)) = (prior, bars.first())
        && indicators::ist_day(before.ts_micros) < indicators::ist_day(first.ts_micros)
    {
        keep_larger(&mut best, before, first);
    }
    for pair in bars.windows(2) {
        let [before, after] = pair else { continue };
        keep_larger(&mut best, before, after);
    }
    best
}

/// Measures the overnight from `before`'s close to `after`'s open into `best`
/// when the two are different IST days and the close is positive, keeping the
/// earlier of two equal magnitudes.
fn keep_larger(
    best: &mut Option<OvernightMove>,
    before: &indicators::Candle,
    after: &indicators::Candle,
) {
    let day = indicators::ist_day(after.ts_micros);
    if day == indicators::ist_day(before.ts_micros) || before.close <= 0 {
        return;
    }
    let ratio =
        i128::from(after.open.saturating_sub(before.close)) * 1_000_000 / i128::from(before.close);
    let ppm = i64::try_from(ratio).unwrap_or(if ratio < 0 { i64::MIN } else { i64::MAX });
    if best.is_none_or(|kept| ppm.unsigned_abs() > kept.ppm.unsigned_abs()) {
        *best = Some(OvernightMove {
            day,
            prior_close: before.close,
            open: after.open,
            ppm,
        });
    }
}

/// What a stock's report says when its bars hold no overnight to measure:
/// one session, or none. Said rather than omitted, so the absence of the
/// [`overnight_line`] is never read as "no large move was found".
pub const NO_OVERNIGHT_MEASURED: &str = "  LARGEST OVERNIGHT MOVE IN THESE BARS: none measured; they hold fewer\n  \
     than two sessions (D-1540).\n\n";

/// Paisa as rupees with two decimals, in integers (`CLAUDE.md` §7).
fn rupees(paisa: i64) -> String {
    let sign = if paisa < 0 { "-" } else { "" };
    let whole = paisa.unsigned_abs();
    format!("{sign}{}.{:02}", whole / 100, whole % 100)
}

/// The line a stock's report carries under [`CORPORATE_ACTIONS_UNCHECKED`]:
/// the [`largest_overnight_move`] in its bars, with `date` naming the session.
///
/// It states a measurement and claims no threshold, because none is sourced.
#[must_use]
pub fn overnight_line(found: &OvernightMove, date: &str) -> String {
    let sign = if found.ppm < 0 { "-" } else { "+" };
    let size = found.ppm.unsigned_abs();
    format!(
        "  LARGEST OVERNIGHT MOVE IN THESE BARS: {sign}{}.{:02}% into the {date}\n  \
         session (close {} to open {}). NO THRESHOLD is applied to it\n  \
         (D-0018, D-1540); check that date against the exchange's\n  \
         corporate-action record before trusting a result that fires on or\n  \
         after it.\n\n",
        size / 10_000,
        (size % 10_000) / 100,
        rupees(found.prior_close),
        rupees(found.open),
    )
}

/// The charge statement for an index-spot run, byte-identical to the header
/// every run printed before D-0681.
///
/// Its last paragraph is left as written even though the day it warns about
/// has come: it is still true of an index, it is what every stored index
/// report already carries, and the stock case it names is now answered by
/// [`equity_header`] rather than by this text.
fn index_header(out: &mut String) {
    let _ = writeln!(
        out,
        "  INDEX SPOT run. There is no brokerage, STT, stamp or GST, because\n  \
         an INDEX is not tradeable: no order is placed, so nothing charges\n  \
         for one.\n\n  \
         NEITHER BLOCK BELOW CHARGES A TICK, AND THIS LINE USED TO SAY ONE\n  \
         OF THEM DID. Both TRADES and EXIT GRID fill at prices the bar\n  \
         actually printed -- the open on the kind reading, the printed\n  \
         extreme on the harsh one -- because a price a tick outside the bar\n  \
         is one nothing traded at, and naming it would be the invention §3\n  \
         rule 1 forbids. The sentence this replaces credited TRADES with two\n  \
         ticks a round trip through `costs::fill::worst_case_fills`, and\n  \
         NOTHING in this crate calls that function.\n\n  \
         SO EVERY TOTAL BELOW IS GROSS OF THE SPREAD. The two blocks differ\n  \
         in WHICH printed price each leg takes and in nothing else. There is\n  \
         no slippage allowance in either, and the cost of crossing the\n  \
         spread is measured nowhere in this run.\n\n  \
         THIS IS SCOPED TO AN INDEX AND TO NOTHING ELSE. A STOCK spot IS\n  \
         tradeable -- you buy real shares -- so brokerage, STT, stamp, the\n  \
         exchange charge and GST all apply there, and so do they on options.\n  \
         `costs::scope::Segment` has no equity-spot variant today, so that\n  \
         charge path does not exist yet and this header would be WRONG the\n  \
         day a stock is swept."
    );
}

/// The charge statement for a cash-equity run. D-0681.
///
/// Every clause is a fact about THIS code or about what a share trade pays,
/// and no clause names a rate: `docs/00-charter.md` records no source for an
/// equity charge rate, and quoting one would be the invention `CLAUDE.md` §3
/// rule 1 forbids. The tick paragraph is the same fact the index header
/// states, because both blocks fill at printed prices whatever the instrument.
///
/// It ends with [`CORPORATE_ACTIONS_UNCHECKED`], beside the charge statement
/// and before any figure, because both are facts about what the totals below
/// are made of. D-0694.
fn equity_header(out: &mut String) {
    let _ = writeln!(
        out,
        "  CASH EQUITY run. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE.\n  \
         A stock IS tradeable -- you buy real shares -- so brokerage, STT,\n  \
         stamp duty, exchange charges, the SEBI fee, the IPFT, DP charges\n  \
         and GST (an UNVERIFIED list) all apply to a share trade. This engine has no equity charge path:\n  \
         `costs::scope::Segment` has no equity variant, so NONE of those\n  \
         charges is subtracted anywhere in this run, and the ranking that\n  \
         chose this combination is on GROSS returns.\n\n  \
         THIS IS COST-EXCLUDED RESEARCH, NOT A NET RESULT (D-0509, D-0525,\n  \
         D-0681). No equity result carries Selection V6 or execution\n  \
         authority until a charter-sourced equity charge stack exists.\n\n  \
         NEITHER BLOCK BELOW CHARGES A TICK EITHER. Both TRADES and EXIT GRID\n  \
         fill at prices the bar actually printed -- the open on the kind\n  \
         reading, the printed extreme on the harsh one -- so every total is\n  \
         also GROSS OF THE SPREAD, and the cost of crossing the spread is\n  \
         measured nowhere in this run.\n\n\
         {CORPORATE_ACTIONS_UNCHECKED}"
    );
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
/// **The spread, and this paragraph used to claim otherwise.** On a spot index
/// there is no brokerage, no STT, no stamp duty and no GST, because a spot index
/// is not tradeable and no order is placed — `costs::scope::is_cost_free`
/// returns true for `IndexSpot` and says so. That half stands and is unchanged.
///
/// The half that did not stand: this block read *"the only cost that exists is
/// slippage, and it is applied: two ticks per round trip through
/// `costs::fill::worst_case_fills`"*. **Nothing in this crate calls that
/// function.** Every reference to it under `crates/runner` is a doc comment or a
/// record of its own removal — `trade.rs:255`, `:505`, `:533`. `trade::walk`
/// prices its two brackets with `Anchor::Open` and `Anchor::PrintedExtreme`
/// (`trade.rs:560`, `:565`), and `crate::grid` has always used the latter, so
/// **no tick is added on any leg of any path**.
///
/// The consequence is a bound, not a rounding: `trade.rs`' own measured table
/// records two horizons flipping from profitable to −27,883p and −8,658p on
/// identical bars once the tick was charged. Every figure below is gross of the
/// spread and the size of that omission is UNMEASURED.
///
/// **That changes when options land.** An option leg carries the whole charge
/// stack, and the figures here would then be gross of it until
/// `costs::trip::price` is wired. This paragraph is the record of which of
/// those two worlds a reader is in.
///
/// # What is NOT in these figures, for a cash-equity run
///
/// **Every charge.** A stock is bought as real shares, so brokerage, STT,
/// stamp duty, exchange charges, the SEBI fee, the IPFT, DP charges and GST
/// all apply (an UNVERIFIED list: no charter source enumerates the equity
/// charge stack, D-1779), and no path
/// in this engine computes any of them. The `scope` argument is what tells the
/// header which of the two statements is true; it was absent until D-0681, and
/// the index statement was printed over stocks for as long as it was.
#[must_use]
pub fn render(
    scope: CostScope,
    taken: Option<&Trades>,
    exits: Option<&Grid>,
    folds: Option<&Validated>,
    overfit: Option<&Pbo>,
    boot: Option<(Option<&Verdict>, Option<&Verdict>, Option<usize>)>,
    rows: usize,
) -> String {
    render_selected(scope, taken, exits, None, folds, overfit, boot, rows)
}

/// [`render`] with the policy-selected cell named explicitly.
///
/// A grid's unconstrained [`Grid::best`] and a caller's best cell within its
/// risk rules can differ. The durable result must report the latter when one was
/// selected; falling back to `best()` here would put one exit in the ledger and
/// a different strategy report below the same audit. `None` preserves the
/// unconstrained historical rendering for callers that apply no policy.
///
/// `scope` chooses the charge statement that leads the report and the words
/// of the strategy report's two ranked totals, which [`strategy_report`]
/// calls net profit for an index and a gross total for a stock. Every other
/// byte below the header is the same for either scope.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the cost scope joins seven independent optional sections; bundling them would hide which section a caller did not supply, which this render exists to name"
)]
pub fn render_selected(
    scope: CostScope,
    taken: Option<&Trades>,
    exits: Option<&Grid>,
    selected: Option<&Cell>,
    folds: Option<&Validated>,
    overfit: Option<&Pbo>,
    boot: Option<(Option<&Verdict>, Option<&Verdict>, Option<usize>)>,
    rows: usize,
) -> String {
    let mut out = String::with_capacity(4_096);
    let _ = writeln!(out, "AUDIT");
    match scope {
        CostScope::IndexSpot => index_header(&mut out),
        CostScope::CashEquity => equity_header(&mut out),
    }
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
            if let Some(chosen) = selected.or_else(|| x.best()) {
                if selected.is_some() {
                    row(
                        &mut out,
                        "durable selected exit",
                        &exit_name(chosen),
                        "the final admitted screen row; it may differ from the grid's unconstrained BEST",
                    );
                }
                strategy_report(&mut out, chosen, &exit_name(chosen), scope);
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
        "past the proved session boundary or missing a priceable path",
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
    // whole point of pricing nothing rather than pricing a guess is that the
    // operator sees a smaller sample instead of an invented path. The position
    // is still HELD -- see `grid::blocks_without_pricing` -- so the sequence is
    // unchanged and only the tally is short. A count kept and not rendered
    // would put it back to being invisible.
    if g.refused_paths > 0 {
        row(
            out,
            "PATHS UNPRICED, refused bar",
            &g.refused_paths.to_string(),
            "a bar between entry and exit failed the engine's own bar check, so \
             the round trip could not be priced. It STILL HELD THE POSITION and \
             blocked the next signal, so the sample below is smaller by these \
             and the sequence is unchanged. A tighter exit can lose a later \
             signal to one of these that a looser exit hid.",
        );
    }
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
        "  {:<15}{:>7}{:>6}{:>6}{:>6}{:>6}{:>7}{:>6}{:>14}{:>11}{:>6}{:>12}{:>12}{:>9}{:>13}{:>11}{:>11}{:>9}{:>11}",
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
        // A LEVEL EXIT THE BAR OPENED PAST, so the fill was the bar's open and
        // not the level the ladder asked for. Counted rather than absorbed
        // because §4 refuses a fallback that hides a failure: a cell whose money
        // comes from gaps is not the rung working, and a reader who cannot tell
        // the two apart is reading the ladder's intention as its result.
        "gap",
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
         are paisa; winner MAE/MFE and all MAE are ppm; ret/DD and mfe/allMAE are\n  ratios in hundredths (250 = 2.50).\n  total is the WORST \
         reading of BOTH legs: in at the worst price the bar PRINTED, out \
         at the worse of the two orderings. fill cost is what that entry gave \
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
        "  {:<15}{:>7}{:>6}{:>6}{:>6}{:>6}{:>7}{:>6}{:>14}{:>11}{:>6}{:>12}{:>12}{:>9}{:>13}{:>11}{:>11}{:>9}{:>11}{}",
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
        c.gapped,
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
            // Z1-slice00-F1, D-2537. This note said "the tightest stop that
            // keeps every winner". The value is `adverse_won / n`, a MEAN, so a
            // stop placed there cuts every winner that went further than the
            // average one. D-0281 corrected the same claim on `cli results` and
            // left this copy owed. The bound is the WORST row above.
            "a mean, NOT a stop level: winners past it would be cut",
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
/// | max winning streak | and what it strings together when it works |
/// | avg bars held | is this a four-minute trade or a four-hour one |
/// | largest win / loss | is the total carried by an outlier |
///
/// **The worst MAE is the one that decides whether a stop is survivable.** A
/// mean of 30 ppm across ten thousand trades is entirely consistent with one
/// trade that went 4,000 ppm against, and that one trade is the one that takes
/// the account out. Every other figure here is a summary; that one is a bound.
///
/// # The two ranked totals are named by the scope
///
/// An index's two totals are "net profit": net of the losing trades, and no
/// charge exists for them to be gross of. A stock's are not net of anything
/// its trades pay, and the header above them says "NOT A NET RESULT", so for
/// [`CostScope::CashEquity`] they are "total P&L" and each says it is gross of
/// every charge. Every other row is the same for either scope. D-0681.
pub fn strategy_report(out: &mut String, cell: &Cell, name: &str, scope: CostScope) {
    let _ = writeln!(out, "STRATEGY REPORT — exit variant {name}");
    let losers = cell.trades.saturating_sub(cell.wins);
    let pf = cell.profit_factor_bp();
    let [(worst, worst_note), (best, best_note)] = match scope {
        CostScope::IndexSpot => [
            ("net profit, worst-case fills", "what selection ranks on"),
            ("net profit, best-case fills", "both fills at the open"),
        ],
        CostScope::CashEquity => [
            (
                "total P&L, worst-case fills",
                "what selection ranks on, gross of every charge",
            ),
            (
                "total P&L, best-case fills",
                "both fills at the open, gross of every charge",
            ),
        ],
    };

    for (label, value, note) in [
        (worst, money(cell.pessimistic), worst_note),
        (best, money(cell.optimistic), best_note),
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
        mean_row(
            "avg winning trade",
            cell.wins == 0,
            cell.avg_win(),
            "no winning trade",
        ),
        mean_row(
            "avg losing trade",
            losers == 0,
            cell.avg_loss(),
            "no losing trade",
        ),
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
            ratio_or_no_dd(cell.return_over_drawdown()),
            "profit per unit of pain",
        ),
        (
            "MAX LOSING STREAK",
            cell.max_losing_streak.to_string(),
            "consecutive losers to sit through",
        ),
        (
            "MAX WINNING STREAK",
            cell.max_winning_streak.to_string(),
            "consecutive winners it strung together",
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

/// One average row of the STRATEGY REPORT, or a dash when its side is empty.
///
/// A MEAN OVER NO TRADES IS NOT `0.00` (p5num-4, D-2712). `avg_win` and
/// `avg_loss` return 0 when their side is empty, and printed through [`money`]
/// that read as a measured mean. The dash says nothing was averaged and the note
/// says why, as the PROFIT FACTOR row already does.
fn mean_row(
    label: &'static str,
    empty: bool,
    mean: i64,
    why: &'static str,
) -> (&'static str, String, &'static str) {
    if empty {
        (label, "-".to_owned(), why)
    } else {
        (label, money(mean), "")
    }
}

/// A return-over-drawdown in hundredths, or `no DD` for the zero-drawdown
/// sentinel.
///
/// The sentinel is [`i64::MAX`], which [`hundredths`] printed as
/// `92233720368547758.07` in the STRATEGY REPORT — a measured ratio that never
/// happened. Same words as the grid's [`ret_dd`] cell (p5num-4, D-2712).
fn ratio_or_no_dd(ratio: i64) -> String {
    if ratio == i64::MAX {
        "no DD".to_owned()
    } else {
        hundredths(ratio)
    }
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
    let key = |&(i, c): &(usize, &Cell)| {
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
    };
    // ONLY THE SHOWN ROWS ARE SORTED (o1runner-11, D-1195). The key is total --
    // the index breaks every tie -- so selecting the `keep` smallest first and
    // sorting just those prints exactly the rows, in exactly the order, a full
    // sort did: O(n + keep log keep) rather than O(n log n) over every cell.
    let shown = ordered.len().min(keep);
    if shown > 0 && shown < ordered.len() {
        ordered.select_nth_unstable_by_key(shown - 1, key);
    }
    let (head, tail) = ordered.split_at_mut(shown);
    head.sort_by_key(key);

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

    for &(_, c) in head.iter() {
        grid_row(out, c, mark_for(c));
    }
    if !tail.is_empty() {
        let _ = writeln!(
            out,
            "  ... {} further variant(s) NOT SHOWN. The grid is complete; this \
             table is not.",
            tail.len()
        );
        // A SELECTOR'S OWN ROW IS NEVER DROPPED BY THE CUT.
        //
        // `keep` is a display bound, and `best()`/`sharpest()` rank the WHOLE
        // grid -- so the row the report names as the answer could sit at
        // position 400 of 625 and be truncated away, leaving a sentence naming a
        // variant no row described. Printing it below the cut costs two lines
        // and removes the one way this table could contradict the report around
        // it.
        //
        // The tail is unsorted now, and at most two of its cells carry a mark
        // (one per selector), so those are put in key order by one comparison
        // instead of by sorting the tail.
        let mut marked = tail.iter().filter(|&&(_, c)| !mark_for(c).is_empty());
        let (first, second) = (marked.next(), marked.next());
        let (first, second) = match (first, second) {
            (Some(a), Some(b)) if key(b) < key(a) => (Some(b), Some(a)),
            pair => pair,
        };
        for &(_, c) in first.into_iter().chain(second) {
            grid_row(out, c, mark_for(c));
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
             and {} ppm for. Ratio {}. Both are means over the winners: a stop \
             at that adverse figure would have cut every winner that went \
             further.",
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
    // A REFUSAL AND A SHORT SLICE ARE DIFFERENT FACTS AND USED TO PRINT THE SAME
    // LINE.
    //
    // An empty `folds` meant "this span could not be split" and nothing else,
    // so a walk that DECLINED to run rendered as one that had nothing to run on.
    // Those call for opposite responses: a short span is answered by a longer
    // one, a refusal is answered by fixing what it names. `Validated::refused`
    // carries the reason and this prints it verbatim rather than summarising it,
    // because the reason names which parts of the report are still trustworthy.
    if let Some(why) = v.refused.as_deref() {
        row(out, "folds", "REFUSED", "no fold was run, and this is why:");
        let _ = writeln!(out, "    {why}");
        let _ = writeln!(out);
        return;
    }
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
        // THE SIDE EACH FOLD CHOSE, and it is a column because the fold now
        // CHOOSES it. It used to be handed one direction for every candidate,
        // taken from a rank over the whole span -- so there was nothing per
        // fold to report. Each fold reads its winner's side off its own
        // training window now, and a fold that decides something and does not
        // say what it decided leaves a reader unable to check it.
        "  {:<6}{:>7}{:>10}{:>10}{:>12}{:>14}{:>14}{:>15}",
        "fold", "side", "train", "test", "candidates", "in-sample", "oos no exit", "oos with exit"
    );
    for f in &v.folds {
        let _ = writeln!(
            out,
            "  {:<6}{:>7}{:>10}{:>10}{:>12}{:>14}{:>14}{:>15}",
            f.index,
            f.chosen_side.map_or("-", |d| d.as_str()),
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

/// The legacy anchored-fold bottom-half diagnostic.
pub fn overfitting(out: &mut String, p: &Pbo) {
    let _ = writeln!(out, "OVERFITTING");
    row(
        out,
        "CSCV/PBO authority",
        "NOT MEASURED",
        "this run has anchored walk-forward folds, not combinatorially symmetric partitions",
    );
    match p.probability() {
        None => {
            row(
                out,
                "legacy bottom-half rate",
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
                "legacy bottom-half rate",
                &format!("{}.{}%", ppm / 10_000, (ppm / 1_000) % 10),
                if p.is_noise() {
                    "at or above 50% across these anchored folds only"
                } else {
                    "below 50% across these anchored folds only"
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
        "  NON-AUTHORITATIVE: this is not CSCV/PBO and cannot establish lack \
         of overfitting, generalisation or profitability. It describes only \
         the supplied anchored walk-forward folds."
    );
    let _ = writeln!(out);
}

/// The three bootstrap tests, side by side.
pub fn bootstrap(
    out: &mut String,
    rc: Option<&Verdict>,
    spa: Option<&Verdict>,
    named: Option<usize>,
) {
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
    // A REFUSED STEPDOWN IS NOT A STEPDOWN THAT NAMED NOTHING (p4num-1,
    // D-2617). A family too short for the block (D-1990) has no Romano-Wolf
    // answer; printing it as "names 0" told the reader a stepdown had run and
    // rejected nothing.
    let shown = named.map_or_else(|| "REFUSED".to_owned(), |n| n.to_string());
    let _ = writeln!(
        out,
        "  {:<28}{:>12}{:>12}{:>10}",
        "Romano-Wolf (named)", "-", "-", shown
    );
    let _ = writeln!(out);
    match named {
        Some(named) => {
            let _ = writeln!(
                out,
                "  The first two ask whether ANYTHING here is real. Only Romano-Wolf \
                 says WHICH, and it names {named}."
            );
        }
        None => {
            let _ = writeln!(
                out,
                "  The first two ask whether ANYTHING here is real. Only Romano-Wolf \
                 says WHICH, and it was REFUSED: the family's aligned periods are too \
                 few for the resampling block, so it named nothing because it ran on \
                 nothing."
            );
        }
    }

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
        // THE ACTUAL COUNT, THEN THE BAND. `calibration` names the measured
        // row the sample falls in, so 57 periods printed "30 periods: ..."
        // and the count itself was never shown (ET-strategies-trades-ranking-
        // costs-6, D-1498). The band stays: it is the measurement.
        let _ = writeln!(
            out,
            "  sample: {} period(s); nearest measured row at or below it -- {}",
            v.periods,
            v.calibration()
        );
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
        bootstrap(&mut out, Some(&short), None, Some(1));
        assert!(out.contains("37.1%"), "the measured rate is shown:\n{out}");
        assert!(
            out.contains("  sample: 3 period(s);"),
            "and the count itself:\n{out}"
        );
        // BETWEEN TWO ROWS, THE COUNT IS NOT THE ROW'S (D-1498). 57 periods
        // takes the 30-period row and must still say 57.
        let between = crate::bootstrap::Verdict {
            periods: 57,
            ..short
        };
        let mut said = String::new();
        bootstrap(&mut said, Some(&between), None, Some(1));
        assert!(said.contains("  sample: 57 period(s);"), "{said}");
        assert!(said.contains("30 periods: measured 13.3%"), "{said}");
        assert!(
            out.contains("READ THE TWO ROWS ABOVE"),
            "and a short sample is called out rather than left to the reader:\n{out}"
        );

        let long = crate::bootstrap::Verdict {
            periods: 400,
            ..short
        };
        let mut out = String::new();
        bootstrap(&mut out, Some(&long), None, Some(1));
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
    use super::{
        CASH_EQUITY_GROSS, CORPORATE_ACTIONS_UNCHECKED, CostScope, bootstrap, grid, overfitting,
        render_selected, strategy_report, trades, walk_forward,
    };
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
            occupancy: Vec::new(),
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
            occupancy: Vec::new(),
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
    fn an_unmeasured_legacy_rate_is_not_rendered_as_zero_or_pbo() {
        // "Never overfit" and "never measured" are different facts, and zero
        // reads as the first. This is the assertion that stops an audit from
        // reporting a perfect score for a test that never ran.
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&[]));
        assert_eq!(cell(&out, "legacy bottom-half rate"), "NOT");
        assert_eq!(cell(&out, "CSCV/PBO authority"), "NOT");
        assert!(out.contains("not the same as zero"));
        assert!(
            !out.contains("0.0%"),
            "an unmeasured legacy rate must not render as a number at all"
        );
    }

    #[test]
    fn a_half_legacy_rate_is_bounded_to_the_supplied_anchored_folds() {
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
            out.contains("at or above 50% across these anchored folds only"),
            "the compatibility rate must name its exact narrow population"
        );
        assert!(out.contains("this is not CSCV/PBO"));
    }

    #[test]
    fn every_section_states_what_it_cannot_tell_you() {
        // The difference between an audit and a brochure. A reader who takes a
        // true number for a claim it does not support has been misled by it,
        // and the limit has to sit beside the figure.
        let mut out = String::new();
        overfitting(&mut out, &probability_of_overfitting(&[]));
        assert!(out.contains("cannot establish"));
        assert!(out.contains("profitability"));

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
        let out = super::render(CostScope::IndexSpot, None, None, None, None, None, 10);
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
        let out = super::render(CostScope::IndexSpot, None, None, None, None, None, 10);
        assert!(out.contains("no brokerage"));

        // NEITHER BLOCK CHARGES A TICK, AND THE HEADER MUST SAY SO.
        //
        // THIS ASSERTION USED TO PIN A CLAIM THAT HAD STOPPED BEING TRUE, and
        // the shape is worth keeping because it is the second time this exact
        // line has done it. It first asserted `"Slippage IS in these figures"` —
        // one claim over the whole report, true while both halves used
        // `Anchor::AdverseExtreme`. When the grid moved to `PrintedExtreme` that
        // sentence became false for one half, so the assertion was split into
        // "the trade walk still carries the tick" and "the grid does not".
        //
        // The trade walk then moved to `PrintedExtreme` as well — `trade.rs:565`
        // — and the first half became false in its turn. `worst_case_fills` has
        // no caller anywhere under `crates/runner`: every reference to it in
        // this crate is a doc comment or a record of its own removal. So the
        // test went on requiring the report to state a cost the engine had
        // stopped charging, which is the failure wearing a success's clothes
        // that `CLAUDE.md` §4 bans.
        //
        // The fix is to assert the PROPERTY rather than the sentence: no tick on
        // either path, and the resulting omission named rather than implied.
        assert!(
            out.contains("NEITHER BLOCK BELOW CHARGES A TICK"),
            "the header must state that no tick is charged on EITHER path, \
             because both `trade::walk` and `grid` now fill at printed prices"
        );
        assert!(
            out.contains("GROSS OF THE SPREAD"),
            "and it must name what that omits, or a reader takes a gross figure \
             for a net one"
        );
        assert!(
            !out.contains("carries two ticks"),
            "the two-tick claim must not return while `worst_case_fills` has no \
             caller -- wire the tick first, then say so"
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

    /// The index header every run carried before D-0681, spelled out line by
    /// line rather than through the code's own string continuations, so a
    /// change to either spelling is a failure here.
    const INDEX_HEADER_BEFORE_D0681: &str = concat!(
        "AUDIT\n",
        "  INDEX SPOT run. There is no brokerage, STT, stamp or GST, because\n",
        "  an INDEX is not tradeable: no order is placed, so nothing charges\n",
        "  for one.\n",
        "\n",
        "  NEITHER BLOCK BELOW CHARGES A TICK, AND THIS LINE USED TO SAY ONE\n",
        "  OF THEM DID. Both TRADES and EXIT GRID fill at prices the bar\n",
        "  actually printed -- the open on the kind reading, the printed\n",
        "  extreme on the harsh one -- because a price a tick outside the bar\n",
        "  is one nothing traded at, and naming it would be the invention §3\n",
        "  rule 1 forbids. The sentence this replaces credited TRADES with two\n",
        "  ticks a round trip through `costs::fill::worst_case_fills`, and\n",
        "  NOTHING in this crate calls that function.\n",
        "\n",
        "  SO EVERY TOTAL BELOW IS GROSS OF THE SPREAD. The two blocks differ\n",
        "  in WHICH printed price each leg takes and in nothing else. There is\n",
        "  no slippage allowance in either, and the cost of crossing the\n",
        "  spread is measured nowhere in this run.\n",
        "\n",
        "  THIS IS SCOPED TO AN INDEX AND TO NOTHING ELSE. A STOCK spot IS\n",
        "  tradeable -- you buy real shares -- so brokerage, STT, stamp, the\n",
        "  exchange charge and GST all apply there, and so do they on options.\n",
        "  `costs::scope::Segment` has no equity-spot variant today, so that\n",
        "  charge path does not exist yet and this header would be WRONG the\n",
        "  day a stock is swept.\n",
        "\n",
    );

    /// A populated render under one scope: trades, a grid, an explicit
    /// selection, folds, PBO and a bootstrap slot, so the body below the
    /// header is not merely the absent-section text.
    fn populated_render(scope: CostScope) -> String {
        let taken = Trades {
            eligible: Vec::new(),
            occupancy: Vec::new(),
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
        let g = populated_grid();
        let selected = g.cells.get(1).expect("the stop-only policy selection");
        render_selected(
            scope,
            Some(&taken),
            Some(&g),
            Some(selected),
            Some(&Validated::default()),
            Some(&probability_of_overfitting(&[])),
            Some((None, None, Some(0))),
            10,
        )
    }

    /// AN INDEX RUN'S HEADER IS UNCHANGED, BYTE FOR BYTE. D-0681 gave the
    /// render a scope so a stock would stop being told it pays no brokerage;
    /// it did not reword a single byte of what every index report says.
    #[test]
    fn an_index_audit_header_is_byte_identical_to_the_one_before_d0681() {
        for out in [
            super::render(CostScope::IndexSpot, None, None, None, None, None, 10),
            populated_render(CostScope::IndexSpot),
        ] {
            assert_eq!(
                out.get(..INDEX_HEADER_BEFORE_D0681.len()),
                Some(INDEX_HEADER_BEFORE_D0681),
                "the index header drifted:\n{out}"
            );
            assert!(
                out.get(INDEX_HEADER_BEFORE_D0681.len()..)
                    .is_some_and(|body| body.starts_with("TRADES\n")),
                "the header must end exactly where TRADES begins:\n{out}"
            );
        }
    }

    /// A CASH-EQUITY RUN IS LABELLED GROSS OF EVERY CHARGE, AND NEVER AS AN
    /// INDEX. D-0681.
    ///
    /// Before the scope existed every stock audit opened with *"INDEX SPOT
    /// run. There is no brokerage, STT, stamp or GST"* -- a true sentence
    /// about an index, printed over real share trades that pay every one of
    /// those. The engine ranks equities on gross returns (D-0509, D-0525); the
    /// defect was only that the report said the charges did not exist.
    #[test]
    fn an_equity_audit_is_gross_of_every_charge_and_never_labelled_an_index() {
        for out in [
            super::render(CostScope::CashEquity, None, None, None, None, None, 10),
            populated_render(CostScope::CashEquity),
        ] {
            assert!(
                out.starts_with(
                    "AUDIT\n  CASH EQUITY run. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE.\n"
                ),
                "the gross statement must be the first thing the header says:\n{out}"
            );
            for index_claim in ["INDEX SPOT run", "no brokerage", "There is no"] {
                assert!(
                    !out.contains(index_claim),
                    "{index_claim:?} is the index's statement and false of a \
                     share trade:\n{out}"
                );
            }
            for charge in SHARE_TRADE_CHARGES {
                assert!(
                    out.contains(charge),
                    "{charge} applies to a share trade and must be named:\n{out}"
                );
            }
            for disclosure in [
                "and GST (an UNVERIFIED list) all apply to a share trade",
                "This engine has no equity charge path",
                "the ranking that\n  chose this combination is on GROSS returns",
                "COST-EXCLUDED RESEARCH, NOT A NET RESULT",
                "No equity result carries Selection V6 or execution\n  authority",
                "NEITHER BLOCK BELOW CHARGES A TICK",
                "GROSS OF THE SPREAD",
            ] {
                assert!(
                    out.contains(disclosure),
                    "the equity header must say {disclosure:?}:\n{out}"
                );
            }
        }
    }

    /// The strategy report's two ranked-total rows, label and note, as an
    /// index prints them and as a stock prints them. A label is matched with
    /// the padding the report gives it, so a swap cannot leave a column
    /// misaligned by the difference in length.
    const INDEX_TOTALS: [(&str, &str); 2] = [
        ("net profit, worst-case fills", "what selection ranks on"),
        ("net profit, best-case fills", "both fills at the open"),
    ];
    const EQUITY_TOTALS: [(&str, &str); 2] = [
        (
            "total P&L, worst-case fills",
            "what selection ranks on, gross of every charge",
        ),
        (
            "total P&L, best-case fills",
            "both fills at the open, gross of every charge",
        ),
    ];

    /// One ranked-total row's label, as the report pads it.
    fn padded(label: &str) -> String {
        format!("  {label:<32}")
    }

    /// THE SCOPE CHANGES THE HEADER AND THE TWO RANKED-TOTAL ROWS' WORDS, AND
    /// NOTHING ELSE. Every other byte below the header is the same for either
    /// scope, so no figure can differ between an index report and an equity
    /// report except by the bars that produced it. The two rows are the ones
    /// an index calls "net profit": a stock's totals are gross of every
    /// charge, so there they are named as totals and say so.
    #[test]
    fn the_scope_changes_the_header_and_the_ranked_total_rows_words_only() {
        let index = populated_render(CostScope::IndexSpot);
        let equity = populated_render(CostScope::CashEquity);
        let body = |out: &str| {
            out.split_once("\nTRADES\n")
                .map(|(_, rest)| rest.to_owned())
        };
        assert_ne!(index, equity, "the two scopes must print different headers");
        let index_body = body(&index).expect("the index render carries TRADES");
        let mut equity_body = body(&equity).expect("the equity render carries TRADES");
        for ((equity_label, equity_note), (index_label, index_note)) in
            EQUITY_TOTALS.into_iter().zip(INDEX_TOTALS)
        {
            for (equity_words, index_words) in [
                (padded(equity_label), padded(index_label)),
                (format!("  {equity_note}\n"), format!("  {index_note}\n")),
            ] {
                assert_eq!(
                    equity_body.matches(&equity_words).count(),
                    1,
                    "the equity report says {equity_words:?} once:\n{equity_body}"
                );
                equity_body = equity_body.replacen(&equity_words, &index_words, 1);
            }
        }
        assert_eq!(
            index_body, equity_body,
            "every other byte below the header must be identical across scopes"
        );
    }

    /// A SENTINEL OR AN EMPTY MEAN IS NEVER PRINTED AS A MEASUREMENT IN THE
    /// STRATEGY REPORT (p5num-4, D-2712).
    ///
    /// An all-winner variant has no drawdown, so `return_over_drawdown` is the
    /// `i64::MAX` sentinel; it has no loser, so `avg_loss` is 0. A variant that
    /// never won has `avg_win` 0. All three printed as numbers.
    #[test]
    fn the_strategy_report_names_its_sentinels_and_empty_means_in_words() {
        let row = |out: &str, label: &str| -> String {
            out.lines()
                .find(|line| line.starts_with(&format!("  {label}")))
                .map(str::to_owned)
                .expect("the report carries the row")
        };
        let all_won = crate::grid::Cell {
            trades: 3,
            wins: 3,
            pessimistic: 300,
            optimistic: 450,
            gross_win: 300,
            best_trade: 150,
            min_win: 50,
            ..crate::grid::Cell::default()
        };
        assert_eq!(all_won.return_over_drawdown(), i64::MAX);
        let mut out = String::new();
        strategy_report(&mut out, &all_won, "SL·TP", CostScope::IndexSpot);
        assert!(!out.contains("92233720368547758"), "{out}");
        let ret = row(&out, "return over drawdown");
        assert!(ret.contains("no DD"), "{ret}");
        let loss = row(&out, "avg losing trade");
        assert!(loss.contains("no losing trade"), "{loss}");
        assert!(!loss.contains("0.00"), "{loss}");
        assert!(row(&out, "avg winning trade").contains("1.00"), "{out}");

        let never_won = crate::grid::Cell {
            trades: 2,
            wins: 0,
            pessimistic: -200,
            optimistic: -100,
            gross_loss: -200,
            worst_trade: -150,
            max_drawdown: 200,
            ..crate::grid::Cell::default()
        };
        let mut out = String::new();
        strategy_report(&mut out, &never_won, "SL·TP", CostScope::IndexSpot);
        let win = row(&out, "avg winning trade");
        assert!(win.contains("no winning trade"), "{win}");
        assert!(!win.contains("0.00"), "{win}");
        assert!(row(&out, "avg losing trade").contains("-₹1.00"), "{out}");
        // A measured ratio still prints as one.
        let measured = crate::grid::Cell {
            max_drawdown: 100,
            ..all_won
        };
        let mut out = String::new();
        strategy_report(&mut out, &measured, "SL·TP", CostScope::IndexSpot);
        assert!(row(&out, "return over drawdown").contains("3.00"), "{out}");
    }

    /// A STOCK'S STRATEGY REPORT NEVER CALLS A TOTAL NET PROFIT, AND SAYS THE
    /// ONE SELECTION RANKS ON IS GROSS OF EVERY CHARGE.
    ///
    /// The equity header says "THIS IS COST-EXCLUDED RESEARCH, NOT A NET
    /// RESULT", and the report under it printed a rupee "net profit" as the
    /// figure selection ranks on. That is the reading `CLAUDE.md` §1 requires
    /// every equity report to rule out. D-0681.
    #[test]
    fn an_equity_strategy_report_names_its_totals_gross_and_never_net_profit() {
        let equity = populated_render(CostScope::CashEquity);
        let report = equity
            .split_once("STRATEGY REPORT")
            .map(|(_, rest)| rest)
            .expect("the populated render carries a strategy report");
        assert!(
            !report.to_lowercase().contains("net profit"),
            "a stock's totals are gross of every charge, not net:\n{report}"
        );
        let ranked = report
            .lines()
            .find(|line| line.contains("worst-case fills"))
            .expect("the row selection ranks on");
        assert!(
            ranked
                .trim_start()
                .starts_with("total P&L, worst-case fills")
                && ranked.ends_with("what selection ranks on, gross of every charge"),
            "the ranked row says it is a gross total: {ranked}"
        );
        let index = populated_render(CostScope::IndexSpot);
        assert!(
            index.contains("  net profit, worst-case fills"),
            "an index report keeps the words it has always printed:\n{index}"
        );
    }

    /// The scope is the instrument's KIND, and a contract has none.
    #[test]
    fn the_cost_scope_is_decided_by_the_kind_and_a_contract_has_none() {
        use brutex_core::instrument::{Expiry, Kind, OptionSide};
        use brutex_core::price::Paisa;

        assert_eq!(CostScope::of(Kind::Index), Some(CostScope::IndexSpot));
        assert_eq!(CostScope::of(Kind::Equity), Some(CostScope::CashEquity));
        let expiry = Expiry::new(2026, 9, 29).expect("a real expiry date");
        assert_eq!(
            CostScope::of(Kind::Future { expiry }),
            None,
            "a futures contract is never swept and neither header describes one"
        );
        assert_eq!(
            CostScope::of(Kind::Option {
                expiry,
                strike: Paisa::from_raw(2_500_000),
                side: OptionSide::Put,
            }),
            None,
            "an options contract is never swept and neither header describes one"
        );
    }

    /// Both scopes, so a property of the charge statement is checked on the
    /// statement every index run prints and on the one every stock run prints.
    const SCOPES: [CostScope; 2] = [CostScope::IndexSpot, CostScope::CashEquity];

    /// Every charge the equity statement says a share trade pays, as it names
    /// them.
    const SHARE_TRADE_CHARGES: [&str; 9] = [
        "brokerage",
        "STT",
        "stamp duty",
        "exchange charges",
        "SEBI fee",
        "IPFT",
        "DP charges",
        "GST",
        "UNVERIFIED list",
    ];

    /// The first rate unit or currency `text` names, or `None`.
    ///
    /// `docs/00-charter.md` sources no equity charge rate, so none may be
    /// printed (`CLAUDE.md` §3 rule 1). A rate in digits is caught by the
    /// digit checks each caller makes; a rate written in words carries no
    /// digit, and "Rs twenty an order" passed every test in this module until
    /// this function named the currency as the rupee, the paisa and its
    /// plural, the sign, `Rs` and `INR`. The last two are matched as whole
    /// words, not substrings, because as substrings they sit inside ordinary
    /// words: `rs` in "harsh", which the equity statement says, and in "bars",
    /// which [`CORPORATE_ACTIONS_UNCHECKED`] says, and `inr` in "inroad".
    ///
    /// Outside this check: a currency not named above, and a number spelled
    /// out with no unit and no named currency beside it. The statement's own
    /// "the harsh one" rules out banning number words.
    fn rate_named(text: &str) -> Option<&'static str> {
        let lower = text.to_ascii_lowercase();
        [
            "%",
            "per cent",
            "percent",
            "basis point",
            "bps",
            "rupee",
            "paisa",
            "paise",
            "₹",
        ]
        .into_iter()
        .find(|unit| lower.contains(unit))
        .or_else(|| {
            lower
                .split(|c: char| !c.is_ascii_alphanumeric())
                .find_map(|word| ["rs", "inr"].into_iter().find(|currency| word == *currency))
        })
    }

    /// The charge statement alone: everything between the `AUDIT` heading and
    /// the `TRADES` heading, which is exactly what the scope writes.
    ///
    /// Panics rather than answering an empty string, because an empty
    /// statement has no lines and would satisfy every per-line assertion below
    /// for the wrong reason.
    fn charge_statement(out: &str) -> &str {
        out.strip_prefix("AUDIT\n")
            .and_then(|rest| rest.split_once("\nTRADES\n"))
            .map(|(statement, _)| statement)
            .filter(|statement| !statement.trim().is_empty())
            .expect("a render opens with AUDIT, then its charge statement, then TRADES")
    }

    /// THE CHARGE STATEMENT IS ONE INDENTED BLOCK, SO A LIFTED `AUDIT`
    /// SECTION KEEPS ALL OF IT.
    ///
    /// `range-all` and `range-rung` discard each rung's audit report and keep
    /// only sections lifted out of it (`cli`'s `validation_note`, through
    /// `section_note`), and that lift ends a section at its first unindented,
    /// non-blank line. The equity statement is four paragraphs split by blank
    /// lines. One line of it written at column zero would end the lift there,
    /// and a `range-all` table would keep "GROSS OF EVERY CHARGE" while
    /// dropping "COST-EXCLUDED RESEARCH, NOT A NET RESULT": a disclaimer cut
    /// away from the figures it qualifies, which is the §4 shape the lift was
    /// written to close.
    #[test]
    fn every_charge_statement_line_is_indented_so_a_lifted_audit_block_keeps_it_whole() {
        for scope in SCOPES {
            for out in [
                super::render(scope, None, None, None, None, None, 10),
                populated_render(scope),
            ] {
                let statement = charge_statement(&out);
                assert!(
                    statement.contains("\n\n  "),
                    "{scope:?}: the statement must hold a blank line between \
                     paragraphs, the case a blank-line terminator would cut:\n{out}"
                );
                for line in statement.lines() {
                    assert!(
                        line.is_empty() || line.starts_with("  "),
                        "{scope:?}: {line:?} is not indented, so a lifted AUDIT \
                         block would end on it:\n{out}"
                    );
                }
                assert_eq!(
                    out.lines()
                        .skip(1)
                        .find(|line| !line.trim().is_empty() && !line.starts_with(' ')),
                    Some("TRADES"),
                    "{scope:?}: the first unindented line after AUDIT must be the \
                     next heading, so the lift ends exactly where the statement \
                     does:\n{out}"
                );
            }
        }
    }

    /// NO LINE OF EITHER CHARGE STATEMENT READS AS A REFUSAL.
    ///
    /// Nine `cli` command arms take their exit code from a scan of the rendered
    /// page (`carries_refusal`): a line that starts `refused` at column zero,
    /// or whose text after its indentation starts `REFUSED. `, `REFUSED -- ` or
    /// `RESULT NOT RECORDED`. The equity statement is written in capitals on
    /// purpose, which is the case that scan reads. A line of it beginning with
    /// one of those words would give every completed stock audit a failing exit
    /// code, and `cli audit ... && <next step>` would stop on a run that
    /// finished. Checked after trimming and in either case -- stricter than
    /// the scan -- so a spelling the scan learns later cannot start matching
    /// this text unseen.
    #[test]
    fn no_charge_statement_line_reads_as_a_refusal() {
        for scope in SCOPES {
            let out = populated_render(scope);
            let statement = charge_statement(&out);
            for line in statement.lines() {
                let opening = line.trim_start().to_ascii_lowercase();
                for refusal in ["refused", "result not recorded"] {
                    assert!(
                        !opening.starts_with(refusal),
                        "{scope:?}: {line:?} opens like a refusal, so a completed \
                         audit would be read as a refused one:\n{out}"
                    );
                }
            }
        }
    }

    /// THE EQUITY STATEMENT NAMES EVERY CHARGE AND QUOTES NO RATE.
    ///
    /// `docs/00-charter.md` records no source for an equity charge rate, so a
    /// rate printed here would be the invention `CLAUDE.md` §3 rule 1 forbids,
    /// however familiar the number. The only digits the statement may carry are
    /// the decision references that justify it (`D-0509`) and the name of the
    /// selection policy it withholds (`Selection V6`); no per cent sign, no
    /// basis points and no currency may appear.
    ///
    /// Both halves of that heading are checked here. This test once checked a
    /// single charge, `STT`, as a guard that the fixture was the right
    /// statement, so a statement that dropped "the SEBI fee" still passed it;
    /// each of [`SHARE_TRADE_CHARGES`] is required now. "No currency" is checked
    /// as the forms [`rate_named`] names, whose own comment states what that
    /// leaves out, and the phrases below prove the check is not vacuous.
    #[test]
    fn the_equity_charge_statement_names_no_rate() {
        let out = super::render(CostScope::CashEquity, None, None, None, None, None, 10);
        let statement = charge_statement(&out);
        for charge in SHARE_TRADE_CHARGES {
            assert!(
                statement.contains(charge),
                "{charge} applies to a share trade, so the statement must name \
                 it:\n{out}"
            );
        }
        assert_eq!(
            rate_named(statement),
            None,
            "the statement names a charge rate the charter does not source:\n{out}"
        );
        for (rate, unit) in [
            ("a charge of 1% a side", "%"),
            ("three basis points a side", "basis point"),
            ("twenty rupees an order", "rupee"),
            ("ten paise a share", "paise"),
            ("Rs twenty an order", "rs"),
            ("Rs. twenty an order", "rs"),
            ("twenty INR an order", "inr"),
            ("₹20 an order", "₹"),
        ] {
            assert_eq!(rate_named(rate), Some(unit), "{rate:?} names a rate");
        }
        assert_eq!(
            rate_named("the harsh fill over these bars made no inroad"),
            None,
            "a currency abbreviation inside an ordinary word is not a currency"
        );
        let numbered: Vec<&str> = statement
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
            .filter(|token| token.bytes().any(|b| b.is_ascii_digit()))
            .collect();
        assert!(
            numbered.contains(&"D-0681"),
            "the statement must cite the decision that wrote it: {numbered:?}"
        );
        for token in numbered {
            let decision = token
                .strip_prefix("D-")
                .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()));
            assert!(
                decision || token == "V6",
                "{token:?} is a number that is neither a decision reference nor \
                 the Selection V6 name, and could be read as a rate:\n{out}"
            );
        }
    }

    /// THE STATEMENT IS DECIDED BY THE SCOPE ALONE, AND A RERUN IS THE SAME
    /// BYTES (`CLAUDE.md` §3 rule 5).
    ///
    /// [`CostScope`]'s own contract is that the statement is decided by what
    /// the swept instrument IS, never by anything its bars say. So a render
    /// given no section and one given every section open with the same
    /// statement, and two renders of the same inputs are identical.
    #[test]
    fn the_charge_statement_depends_on_the_scope_alone_and_a_rerun_is_byte_identical() {
        for scope in SCOPES {
            let bare = super::render(scope, None, None, None, None, None, 10);
            let first = populated_render(scope);
            let second = populated_render(scope);
            assert_eq!(
                first, second,
                "{scope:?}: the same inputs must render the same bytes"
            );
            assert_eq!(
                charge_statement(&bare),
                charge_statement(&first),
                "{scope:?}: the statement must not depend on what the run produced"
            );
        }
    }

    /// A CASH-EQUITY AUDIT SAYS CORPORATE ACTIONS ARE UNCHECKED, BESIDE ITS
    /// CHARGE STATEMENT, AND AN INDEX AUDIT NEVER DOES. D-0694.
    ///
    /// No split, bonus, rights, face-value, demerger or dividend detector
    /// exists (D-0018 names no threshold
    /// and the charter names no source), so an overnight jump in a stock's
    /// bars can be a corporate action. The operator chose to keep ranking
    /// stocks and to say so; an index never splits and says nothing.
    #[test]
    fn an_equity_audit_states_corporate_actions_are_unchecked_and_an_index_audit_does_not() {
        for out in [
            super::render(CostScope::CashEquity, None, None, None, None, None, 10),
            populated_render(CostScope::CashEquity),
        ] {
            let header = out
                .split_once("\nTRADES\n")
                .map(|(header, _)| header)
                .expect("an audit carries its TRADES section");
            assert!(
                header.ends_with(&format!("{CORPORATE_ACTIONS_UNCHECKED}\n")),
                "the statement must close the charge header, before any figure:\n{out}"
            );
            for fact in [
                "CORPORATE ACTIONS ARE UNCHECKED (D-0018, D-0694)",
                "No split, bonus,\n  rights issue, face-value change, demerger or dividend detection has",
                "can be a corporate action rather than a market move",
                "no\n  threshold for that detector is sourced",
            ] {
                assert!(header.contains(fact), "missing {fact:?}:\n{out}");
            }
            assert_eq!(
                out.matches("CORPORATE ACTIONS").count(),
                1,
                "said once, in the header:\n{out}"
            );
        }
        for out in [
            super::render(CostScope::IndexSpot, None, None, None, None, None, 10),
            populated_render(CostScope::IndexSpot),
        ] {
            for equity_only in ["CORPORATE ACTIONS", "D-0694", "demerger"] {
                assert!(
                    !out.contains(equity_only),
                    "an index never splits, so {equity_only:?} is not its statement:\n{out}"
                );
            }
        }
    }

    /// THE NON-AUDIT NOTE IS GROSS, THEN CORPORATE ACTIONS, FOR A STOCK, AND
    /// NOTHING AT ALL FOR AN INDEX. D-0694.
    #[test]
    fn a_report_note_is_gross_then_corporate_actions_for_a_stock_and_empty_for_an_index() {
        assert_eq!(
            CostScope::IndexSpot.report_note(),
            "",
            "an index report must stay byte for byte what it was"
        );
        let note = CostScope::CashEquity.report_note();
        assert_eq!(
            note,
            format!("{CASH_EQUITY_GROSS}\n\n{CORPORATE_ACTIONS_UNCHECKED}\n\n")
        );
        assert!(
            note.starts_with("  CASH EQUITY. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE"),
            "{note}"
        );
        for fact in [
            "brokerage",
            "STT",
            "stamp duty",
            "exchange charges",
            "SEBI fee",
            "IPFT",
            "DP charges",
            "GST (an UNVERIFIED list)",
            "COST-EXCLUDED RESEARCH, NOT A NET RESULT",
            "No equity result carries Selection V6",
            "D-0681",
            "CORPORATE ACTIONS ARE UNCHECKED (D-0018, D-0694)",
        ] {
            // Read as words: a phrase wrapped across two indented lines is
            // still the phrase.
            let flat = note.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(flat.contains(fact), "missing {fact:?}:\n{note}");
        }
        assert!(
            note.find("GROSS OF EVERY CHARGE") < note.find("CORPORATE ACTIONS"),
            "the corporate-action statement sits beside and after the charge one:\n{note}"
        );
    }

    /// THE CORPORATE-ACTION SENTENCE KEEPS THE CHARGE STATEMENT'S PROPERTIES
    /// WHEREVER IT TRAVELS. D-0694.
    ///
    /// The four tests above pin the audit header, where
    /// [`CORPORATE_ACTIONS_UNCHECKED`] closes the equity statement. It also
    /// travels without that header: into every stored banner through
    /// [`CostScope::report_note`], into a ranked stock's FINDINGS block, the
    /// pool and the research inventory. So the sentence and the note are
    /// checked on their own, for the same three properties: every line blank
    /// or indented, so a lifted block keeps it whole; no line opening like a
    /// refusal, so no completed stock run exits as a failure; and no rate --
    /// no unit or currency that [`rate_named`] finds, the check the equity
    /// statement gets, and its only digits are decision references, D-0018
    /// and D-0694 in the sentence, and the name Selection V6 in the gross
    /// paragraph, because `docs/00-charter.md` sources no split threshold
    /// (`docs/06-limits.md` §41.3) and no charge rate. The name is the one
    /// D-0696's rate check in `cli` also allows; the gross paragraph carries
    /// it since AF-19.
    #[test]
    fn the_corporate_action_sentence_keeps_the_charge_statement_properties_wherever_it_travels() {
        let note = CostScope::CashEquity.report_note();
        for (what, text) in [
            ("the sentence", CORPORATE_ACTIONS_UNCHECKED),
            ("the gross paragraph", CASH_EQUITY_GROSS),
            ("the note", note.as_str()),
        ] {
            assert!(text.lines().count() > 1, "{what} spans lines:\n{text}");
            for line in text.lines() {
                assert!(
                    line.is_empty() || line.starts_with("  "),
                    "{what}: {line:?} is not indented, so a lifted block would end on it"
                );
                let opening = line.trim_start().to_ascii_lowercase();
                for refusal in ["refused", "result not recorded"] {
                    assert!(
                        !opening.starts_with(refusal),
                        "{what}: {line:?} opens like a refusal"
                    );
                }
            }
            assert_eq!(rate_named(text), None, "{what} names a rate:\n{text}");
            for token in text
                .replace("Selection V6", "Selection")
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
                .filter(|token| token.bytes().any(|b| b.is_ascii_digit()))
            {
                // `V6` is the Selection V6 the gross paragraph withholds, the one
                // name the header test allows too (D-0696). The sentence names no
                // policy, so it may carry none.
                assert!(
                    token
                        .strip_prefix("D-")
                        .is_some_and(|n| n.len() == 4 && n.bytes().all(|b| b.is_ascii_digit()))
                        || (what != "the sentence"
                            && token == "V6"
                            && text.contains("Selection V6")),
                    "{what}: {token:?} is a number that is neither a decision reference nor \
                     the Selection V6 name:\n{text}"
                );
            }
        }
        for cited in ["D-0018", "D-0694"] {
            assert!(
                CORPORATE_ACTIONS_UNCHECKED.contains(cited),
                "the sentence cites {cited}"
            );
        }
        assert!(
            !CORPORATE_ACTIONS_UNCHECKED.ends_with('\n'),
            "every caller ends the line itself, so the sentence carries no newline of its own"
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
            "the sniper figure is the winners' MEAN adverse excursion, and it \
             must appear as a number"
        );
        assert!(
            out.contains("depend on intra-bar ordering"),
            "four ambiguous bars must be reported as uncertainty, not hidden"
        );
    }

    /// THE WINNERS' MEAN ADVERSE EXCURSION IS NEVER CALLED A STOP LEVEL.
    ///
    /// Z1-slice00-F1, D-2537. Two winners that went 0 and 100 ppm against have
    /// a `winner_mae` of 50; a stop at 50 cuts the second. The strategy report
    /// noted that row "the tightest stop that keeps every winner" and the
    /// SHARPEST line said "the tightest stop that would not have killed a
    /// winner" -- both fail this test on the old strings. Every permutation of
    /// zero, one and both winners, and a cell with no winners at all, is
    /// rendered, through both surfaces.
    #[test]
    fn the_winner_mae_is_never_called_a_stop_level() {
        let row = |out: &str, label: &str| -> String {
            let head = format!("  {label}");
            for line in out.lines() {
                if line.starts_with(&head) {
                    return line.to_owned();
                }
            }
            String::new()
        };
        for (wins, winner_mae, worst_mae) in [(0_u64, 0, 0), (1, 0, 0), (1, 100, 100), (2, 50, 100)]
        {
            let cell = crate::grid::Cell {
                trades: 2,
                wins,
                pessimistic: 100,
                optimistic: 100,
                winner_mae,
                winner_mfe: 400,
                worst_mae,
                ..crate::grid::Cell::default()
            };
            let mut out = String::new();
            strategy_report(&mut out, &cell, "SL·TP", CostScope::IndexSpot);
            assert!(!out.contains("tightest stop"), "{out}");
            let mean = row(&out, "mean MAE, winners only");
            assert!(
                mean.contains("mean") && mean.contains("NOT a stop"),
                "{mean}"
            );
            assert!(
                row(&out, "WORST MAE, any single trade").contains("THE BOUND"),
                "{out}"
            );
        }
        let mut out = String::new();
        grid(&mut out, &populated_grid(), 10);
        assert!(out.contains("SHARPEST"), "{out}");
        assert!(!out.contains("tightest stop"), "{out}");
        assert!(out.contains("Both are means"), "{out}");
    }

    /// The risk-policy winner can differ from the unconstrained money maximum;
    /// the strategy report must follow the durable selection in that case.
    #[test]
    fn an_explicit_selected_cell_overrides_the_grids_unconstrained_best() {
        let g = populated_grid();
        let selected = g.cells.get(1).expect("the stop-only policy selection");
        assert_ne!(
            g.best(),
            Some(selected),
            "the fixture must separate policy selection from unconstrained best"
        );
        let out = render_selected(
            CostScope::IndexSpot,
            None,
            Some(&g),
            Some(selected),
            None,
            None,
            None,
            10,
        );
        assert!(
            out.contains("durable selected exit") && out.contains("0/-/-"),
            "the report must name the explicitly selected stop-only variant: {out}"
        );
        assert!(
            out.contains("₹10.21"),
            "the selected cell's own strategy report must be rendered: {out}"
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
            refused: None,
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
    fn a_measured_legacy_rate_prints_its_denominator_median_and_limit() {
        // A rate over three usable folds out of four is a different number from
        // one over all four, and a reader must also see that it is not PBO.
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
            out.contains("below 50% across these anchored folds only"),
            "the rate must not escape the folds that supplied it"
        );
        assert!(out.contains("NON-AUTHORITATIVE"));
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
        bootstrap(&mut out, Some(&clears), Some(&fails), Some(2));

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
            occupancy: Vec::new(),
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
            CostScope::IndexSpot,
            Some(&taken),
            Some(&grid_in),
            Some(&folds),
            Some(&over),
            Some((None, None, Some(0))),
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
        bootstrap(&mut out, None, None, None);
        assert!(out.contains("REFUSED"));
        assert!(
            !out.contains("yes"),
            "a test that was refused must never render as clearing"
        );
    }

    /// p4num-1 / D-2617: a refused stepdown is printed as REFUSED on its own
    /// row and in its sentence, never as "names 0"; a stepdown that ran and
    /// rejected nothing still prints 0.
    #[test]
    fn a_refused_romano_wolf_is_never_printed_as_naming_zero() {
        let mut refused = String::new();
        bootstrap(&mut refused, None, None, None);
        let row = refused
            .lines()
            .find(|line| line.contains("Romano-Wolf (named)"))
            .expect("the stepdown row");
        assert!(row.trim_end().ends_with("REFUSED"), "{row}");
        assert!(!refused.contains("names 0"), "{refused}");
        assert!(refused.contains("it was REFUSED"), "{refused}");

        let mut ran = String::new();
        bootstrap(&mut ran, None, None, Some(0));
        let row = ran
            .lines()
            .find(|line| line.contains("Romano-Wolf (named)"))
            .expect("the stepdown row");
        assert!(row.trim_end().ends_with('0'), "{row}");
        assert!(ran.contains("it names 0."), "{ran}");
    }

    /// p5num-4 / D-2712: an all-winner variant with no drawdown prints `no DD`,
    /// not the `i64::MAX` sentinel as a ratio, and no average of nothing.
    #[test]
    fn the_strategy_report_never_prints_a_sentinel_or_an_empty_mean_as_measured() {
        let all_won = crate::grid::Cell {
            trades: 3,
            wins: 3,
            pessimistic: 900,
            optimistic: 900,
            gross_win: 900,
            best_trade: 300,
            min_win: 300,
            max_drawdown: 0,
            ..crate::grid::Cell::default()
        };
        assert_eq!(all_won.return_over_drawdown(), i64::MAX);
        let mut out = String::new();
        super::strategy_report(&mut out, &all_won, "x", CostScope::IndexSpot);
        assert!(!out.contains("92233720368547758"), "{out}");
        let row = |label: &str| {
            out.lines()
                .find(|l| l.contains(label))
                .unwrap_or_default()
                .to_owned()
        };
        assert!(row("return over drawdown").contains("no DD"), "{out}");
        assert!(row("avg losing trade").contains("no losing trade"), "{out}");
        assert!(!row("avg winning trade").contains('-'), "{out}");

        let none_won = crate::grid::Cell {
            trades: 2,
            wins: 0,
            pessimistic: -200,
            optimistic: -200,
            gross_loss: -200,
            worst_trade: -100,
            max_drawdown: 200,
            ..crate::grid::Cell::default()
        };
        let mut out = String::new();
        super::strategy_report(&mut out, &none_won, "y", CostScope::IndexSpot);
        let line = out
            .lines()
            .find(|l| l.contains("avg winning trade"))
            .unwrap_or_default();
        assert!(line.contains("no winning trade"), "{out}");
        assert!(!line.contains("0.00"), "{out}");
    }

    /// o1runner-11 / D-1195: the cut table sorts only its shown rows, and
    /// must print exactly the rows, in exactly the order, a full sort printed.
    /// The full render (`keep` past the grid) sorts everything, so it is the
    /// reference for every shorter `keep`, ties included.
    #[test]
    fn a_cut_grid_table_shows_exactly_the_full_tables_leading_rows() {
        let mut g = populated_grid();
        let template = g.cells.clone();
        for (n, pessimistic) in [5, -3, 5, 0, 5, -3, 9, 0, 1, 5, -7, 9]
            .into_iter()
            .enumerate()
        {
            let mut cell = template
                .get(n % template.len())
                .copied()
                .unwrap_or_default();
            cell.pessimistic = pessimistic;
            g.cells.push(cell);
        }
        // A winless top earner: `best()` and not `sharpest()`, so the two
        // selectors mark DIFFERENT rows and their order below the cut is tested.
        let mut rich = template.first().copied().unwrap_or_default();
        rich.pessimistic = 9_000_000;
        rich.wins = 0;
        g.cells.push(rich);
        let (best, sharp) = (g.best().copied(), g.sharpest().copied());
        assert!(best.is_some() && sharp.is_some() && best != sharp);
        let rows = |out: &str| -> Vec<String> {
            out.lines()
                .filter(|l| is_grid_row(l) && !l.contains("NOT SHOWN"))
                .map(str::to_owned)
                .collect()
        };
        let mut full = String::new();
        grid(&mut full, &g, g.cells.len() + 1);
        let full = rows(&full);
        assert_eq!(full.len(), g.cells.len());
        for keep in 0..=g.cells.len() {
            let mut out = String::new();
            grid(&mut out, &g, keep);
            let cut = rows(&out);
            assert_eq!(
                cut.get(..keep),
                full.get(..keep),
                "keep={keep}: the shown rows differ from the full table's"
            );
            // Below the cut only the selectors' rows, in full-table order.
            let below: Vec<&String> = full
                .iter()
                .skip(keep)
                .filter(|l| l.contains("<-"))
                .collect();
            let printed: Vec<&String> = cut.iter().skip(keep).collect();
            assert_eq!(printed, below, "keep={keep}: rows below the cut");
        }
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
        const COLUMNS: [(&str, usize); 19] = [
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
            // BESIDE `fill cost` BECAUSE BOTH ARE FILL QUALITY, and deliberately
            // NOT beside the five exit counters -- those sum to `trades` and a
            // gap is a property OF a stop or target exit, not a sixth kind.
            ("gap", 6),
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

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "the exception every test module in this workspace takes."
)]
mod overnight_tests {
    use super::{OvernightMove, largest_overnight_move, overnight_line};
    use indicators::Candle;

    /// Thirty generated sessions with every price halved from session 20 on:
    /// an unadjusted 1:2 split, which is what D-0018 says a stock's bars do.
    fn split_at_twenty() -> (Vec<Candle>, usize) {
        let raw = crate::synthetic::sessions(30);
        let first = 20 * 375;
        let split_day = indicators::ist_day(raw[first].ts_micros);
        let bars = raw
            .iter()
            .map(|b| {
                if indicators::ist_day(b.ts_micros) >= split_day {
                    Candle::new(
                        b.ts_micros,
                        b.open / 2,
                        b.high / 2,
                        b.low / 2,
                        b.close / 2,
                        b.volume,
                        b.open_interest,
                    )
                } else {
                    *b
                }
            })
            .collect();
        (bars, first)
    }

    /// AN UNADJUSTED SPLIT IS NAMED BY ITS DATE AND ITS SIZE, WITH NO
    /// THRESHOLD. gaps-6, D-1540.
    #[test]
    fn an_unadjusted_split_is_the_largest_overnight_move_and_is_named_by_its_session() {
        let (bars, first) = split_at_twenty();
        let found = largest_overnight_move(&bars).expect("thirty sessions have overnights");
        assert_eq!(found.day, indicators::ist_day(bars[first].ts_micros));
        assert_eq!(found.prior_close, bars[first - 1].close);
        assert_eq!(found.open, bars[first].open);
        assert!(
            (-510_000..=-490_000).contains(&found.ppm),
            "a 1:2 split is a move of about -50%: {found:?}"
        );
    }

    /// A SPLIT ON THE FIRST SIGNAL DAY IS THE NAMED OVERNIGHT MOVE WHEN THE
    /// SESSION BEFORE THE SPAN IS GIVEN. p16num-1, D-2546.
    ///
    /// The span starts ON the split session, so `bars.windows(2)` never sees
    /// the pre-split close and the old measure named a small in-span overnight
    /// instead. With the prior session's daily record (close 20,002.00) and a
    /// first open of 10,001.00 the named day is the first signal day and the
    /// move is -500,000 ppm. A prior on the first day itself, or after it, is
    /// ignored; no prior, an empty span and a non-positive prior close each
    /// fall back to the in-span answer.
    #[test]
    fn a_split_on_the_first_signal_day_is_the_named_overnight_move() {
        let (bars, first) = split_at_twenty();
        let span = &bars[first..];
        let first_day = indicators::ist_day(span[0].ts_micros);
        let in_span = largest_overnight_move(span).expect("ten sessions have overnights");
        assert_ne!(in_span.day, first_day, "the old measure cannot see it");
        assert_eq!(
            super::largest_overnight_move_after(None, span),
            Some(in_span)
        );

        let previous = bars[first - 1];
        let daily = |ts: i64, close: i64| Candle::new(ts, close, close, close, close, 0, i64::MIN);
        let first_open = Candle::new(
            span[0].ts_micros,
            1_000_100,
            1_000_100,
            1_000_100,
            1_000_100,
            0,
            i64::MIN,
        );
        let mut seeded_span = span.to_vec();
        seeded_span[0] = first_open;
        let seeded = super::largest_overnight_move_after(
            Some(&daily(previous.ts_micros, 2_000_200)),
            &seeded_span,
        )
        .expect("a seeded span has an overnight");
        assert_eq!(seeded.day, first_day);
        assert_eq!(seeded.prior_close, 2_000_200);
        assert_eq!(seeded.open, 1_000_100);
        assert_eq!(seeded.ppm, -500_000);

        // A prior on the first signal day, or later, is not an overnight.
        for ts in [span[0].ts_micros - 1, span[0].ts_micros, span[1].ts_micros] {
            assert!(indicators::ist_day(ts) >= first_day, "fixture: {ts}");
            let same = super::largest_overnight_move_after(Some(&daily(ts, 2_000_200)), span);
            assert_eq!(same, Some(in_span), "prior at {ts} is ignored");
        }
        // A non-positive prior close is skipped like any other.
        for close in [0, -1, i64::MIN] {
            let skipped =
                super::largest_overnight_move_after(Some(&daily(previous.ts_micros, close)), span);
            assert_eq!(skipped, Some(in_span), "close {close}");
        }
        // An empty span has nothing to measure the prior against.
        assert_eq!(
            super::largest_overnight_move_after(Some(&daily(previous.ts_micros, 2_000_200)), &[]),
            None
        );
        // One bar plus a prior is exactly one overnight.
        let one = super::largest_overnight_move_after(
            Some(&daily(previous.ts_micros, 2_000_200)),
            &seeded_span[..1],
        );
        assert_eq!(one.map(|m| (m.day, m.ppm)), Some((first_day, -500_000)));
        // A tie keeps the EARLIER session: the seed, measured first.
        let tie = [
            Candle::new(span[0].ts_micros, 100, 100, 100, 100, 0, i64::MIN),
            Candle::new(span[375].ts_micros, 200, 200, 200, 200, 0, i64::MIN),
        ];
        let tied = super::largest_overnight_move_after(Some(&daily(previous.ts_micros, 50)), &tie)
            .expect("two overnights");
        assert_eq!(tied.day, first_day, "+100% then +100%: the earlier is kept");
    }

    /// ONLY A SESSION BOUNDARY IS AN OVERNIGHT MOVE. A jump inside a session
    /// is a market move this measure does not describe.
    #[test]
    fn an_intraday_jump_is_not_an_overnight_move_and_one_session_has_none() {
        let mut bars = crate::synthetic::sessions(3);
        let inside = 375 + 100;
        let bar = bars[inside];
        bars[inside] = Candle::new(
            bar.ts_micros,
            bar.open * 3,
            bar.high * 3,
            bar.low * 3,
            bar.close * 3,
            bar.volume,
            bar.open_interest,
        );
        let found = largest_overnight_move(&bars).expect("three sessions");
        assert!(
            found.ppm.abs() < 10_000,
            "the tripled bar sits inside a session and must not be reported: {found:?}"
        );
        assert_eq!(
            largest_overnight_move(&crate::synthetic::sessions(1)),
            None,
            "one session has no overnight"
        );
        assert_eq!(largest_overnight_move(&[]), None);
    }

    /// THE LINE STATES THE MEASURED FACT, CLAIMS NO THRESHOLD, AND NEVER
    /// READS AS A REFUSAL.
    #[test]
    fn the_overnight_line_names_date_size_and_prices_and_claims_no_threshold() {
        let line = overnight_line(
            &OvernightMove {
                day: 0,
                prior_close: 200_000,
                open: 100_000,
                ppm: -500_000,
            },
            "2024-06-14",
        );
        let flat = line.split_whitespace().collect::<Vec<_>>().join(" ");
        for fact in [
            "LARGEST OVERNIGHT MOVE IN THESE BARS",
            "-50.00%",
            "2024-06-14",
            "2000.00",
            "1000.00",
            "NO THRESHOLD",
        ] {
            assert!(flat.contains(fact), "missing {fact:?}:\n{line}");
        }
        assert!(line.ends_with('\n'), "{line}");
        for each in line.lines().filter(|each| !each.is_empty()) {
            assert!(each.starts_with("  "), "indented like the note: {each:?}");
        }
        let small = overnight_line(
            &OvernightMove {
                day: 0,
                prior_close: 100_000,
                open: 99_995,
                ppm: -50,
            },
            "2024-06-14",
        );
        assert!(
            small.contains("-0.00%"),
            "a sub-basis-point fall keeps its sign:\n{small}"
        );
    }
}
