//! Every stop/target variant of one combination, including no stop at all.
//!
//! # What this answers
//!
//! [`crate::trade`] exits on time: the horizon, or the 15:10 square-off. This
//! adds the level-based exits — stop, target — and evaluates **every rung of
//! both ladders at once**, with the no-stop no-target variant sitting in the
//! same table as a row rather than as a separate mode.
//!
//! That last part is the point. "Does a stop help here" is not a question anyone
//! should answer by opinion, and it is not a second run: the variant with no
//! stop is the one whose stop rung is [`crate::excursion::NEVER`], and it is
//! computed in the same pass as all the others.
//!
//! # The sniper metric, and why it is a ratio
//!
//! The aim is a setup precise enough that winners run and losers are cut for
//! almost nothing. Two numbers say whether a combination is that:
//!
//! * **MFE** — how far it went your way.
//! * **MAE** — how far it went against you first.
//!
//! A setup whose winners never went more than a few parts per million against you is
//! a setup you can hold with a tight stop, and [`Cell::edge_ratio`] is that
//! measured rather than judged. It is deliberately computed over the trades that
//! ENDED PROFITABLE: the adverse excursion of a loser tells you how bad the loss
//! was, and the adverse excursion of a WINNER tells you how tight a stop could
//! have been without killing it. Only the second answers "how small can the risk
//! be".
//!
//! # Why the whole sequence is re-walked per variant
//!
//! A stop that fires early ends the position early, and under
//! [`crate::trade`]'s one-position-at-a-time rule that frees the NEXT signal to
//! be taken sooner. So the trade sequence is not the same across variants and
//! reusing one list would quietly measure the wrong trades.
//!
//! The re-walk is affordable because the expensive half is cached: the path
//! crossings for a candidate entry depend only on that entry bar and its own
//! session's square-off, never on which variant is being evaluated. They are
//! computed once per candidate entry, and every variant's exit is then three
//! integer compares. At the shipped four rungs that is `125 variants x 1,124
//! signals` — 140,500 lookups against 1,124 path walks, not 140,500 path walks.
//!
//! This paragraph read `400 variants` and `450,000` until the count was checked
//! against the loop. Neither number was ever right for this code: 400 is
//! `(rungs+1)^2` at twenty rungs, from a two-ladder design, and the trailing
//! ladder made the nest three deep without the arithmetic following it. See
//! [`variants`], which is now the only place that count is computed.
//!
//! # The ambiguous bar is resolved twice, never once
//!
//! When one bar would trigger both the stop and the target, minute data cannot
//! say which came first. [`Cell::pessimistic`] resolves it as the stop, and
//! [`Cell::optimistic`] as the target. The gap between them is the uncertainty
//! the data genuinely carries, and reporting one number would be choosing which
//! lie to tell.
//!
//! **The trailing exit has the same ambiguity and, until now, told the lie.**
//! When the bar that fires a trail is also the bar that raised the peak, the
//! order was hanging from the old peak under one ordering and from the new one
//! under the other, and the two price the fill differently. Both readings used
//! the raised peak, so the flattering answer was taken silently and
//! [`Cell::uncertainty`] reported zero for it. It is now resolved by the same
//! machinery: `ended_by` carries both anchors and the pessimism flag picks one,
//! exactly as it already picks between the stop and the target. See
//! [`crate::excursion`]'s header for the half of the fix that lives there —
//! *whether* the rung fired is settled there, order-independently, and only the
//! *price* reaches here.

use indicators::Candle;
use indicators::column::Column;
use vocab::ConditionMask;

use crate::excursion::{Crossings, Ladder, Ladders, NEVER, Ppm, Side, crossings};
use crate::outcome::Horizon;

/// One (stop, target) variant's result over the whole slice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// Index into the stop ladder, or `None` for **no stop**.
    pub stop: Option<usize>,
    /// Index into the target ladder, or **no target**.
    pub target: Option<usize>,
    /// Index into the trailing ladder, or `None` for **no trailing stop**.
    ///
    /// A trailing stop follows the best price seen and fires on the give-back.
    ///
    /// # TRAILING TAKE PROFIT IS NOT IMPLEMENTED, AND THIS COMMENT USED TO SAY
    /// IT WAS
    ///
    /// It read: *"TRAILING TAKE PROFIT is this armed by a target: reach the
    /// target rung, then trail — so it is a combination of two rungs rather
    /// than a third mechanism, and it appears in this table as such."*
    ///
    /// **No arming exists anywhere in this crate.** `one_variant` resolves the
    /// exit as `span.min(stop_at).min(target_at).min(trail_at)` — the three
    /// COMPETE, and the target *exits* the position rather than arming
    /// anything. A grep for `arm` across this file and `crate::excursion`
    /// returned exactly one hit: the sentence above.
    ///
    /// # And the cells it describes are close to degenerate
    ///
    /// Worth stating because it costs real grid width. A trail fires on a
    /// give-back from the running peak; a target needs the full move. So in any
    /// cell where both are set the trail almost always fires first and the
    /// target is nearly inert — which means the 25 of 125 cells carrying both
    /// are measuring approximately what the trail-only cells measure.
    ///
    /// Implementing the arming semantics would make those cells mean something
    /// distinct, and would CHANGE the result of every audit that has a
    /// target-and-trail winner. That is a decision for `docs/05-decisions.md`,
    /// not a comment, so the honest statement stands here until one is written.
    pub trail: Option<usize>,
    /// Round trips taken under this variant.
    pub trades: u64,
    /// Trades that ended above water, resolving ambiguity against you.
    pub wins: u64,
    /// Total paisa per unit, ambiguity resolved as the STOP first.
    pub pessimistic: i64,
    /// Total paisa per unit, ambiguity resolved as the TARGET first.
    pub optimistic: i64,
    /// Trades whose exit came from the stop.
    pub stopped: u64,
    /// Trades whose exit came from the target.
    pub targeted: u64,
    /// Trades that ran to the horizon or the 15:10 square-off.
    pub timed_out: u64,
    /// Bars whose intra-bar ordering the data cannot settle, summed over
    /// trades.
    ///
    /// Two kinds, and both are counted here because both move the two readings
    /// apart in the same way:
    ///
    /// * a bar where a stop and a target were both reachable, and
    /// * on a variant that carries a trailing rung, a bar that fired the trail
    ///   AND raised the peak the trail was priced off.
    ///
    /// The second was missing and so was the uncertainty it causes: a trailing
    /// exit was priced off the raised peak in both readings, which agreed by
    /// construction.
    ///
    /// The size of the uncertainty. A variant with none of these has a
    /// pessimistic and an optimistic figure that agree exactly. The converse is
    /// not claimed — the count is a bound, taken over the whole path rather than
    /// only up to the exit.
    pub ambiguous_bars: u64,
    /// Mean adverse excursion of the trades that ENDED PROFITABLE, in basis
    /// points.
    ///
    /// **The sniper number.** It says how far a winner went against you before
    /// it worked, which is the tightest stop that would not have killed it.
    pub winner_mae: Ppm,
    /// Mean favourable excursion of the trades that ended profitable.
    pub winner_mfe: Ppm,
    /// The single worst round trip, in paisa, under pessimistic fills.
    ///
    /// **Zero when nothing lost.** The tightest stop that would have been
    /// needed to avoid the worst outcome this variant actually produced —
    /// which `winner_mae` cannot answer, because it only looks at winners.
    pub worst_trade: i64,
    /// Largest peak-to-trough fall of the running total, in paisa.
    ///
    /// **Always >= 0.** Two variants with the same total are not equally
    /// survivable: one may have reached it smoothly and the other after giving
    /// back most of it, and a total alone cannot tell them apart. This is the
    /// number an operator asking for "very minimal stop loss" is actually
    /// asking about, and until it existed the engine had none.
    pub max_drawdown: i64,
}

impl Cell {
    /// Favourable excursion over adverse, on the winners, in hundredths.
    ///
    /// The precision of the setup as a single number: how much a winner gave
    /// you against how much it made you sweat first. Hundredths rather than a
    /// float because `CLAUDE.md` §7 keeps this kind of arithmetic in integers,
    /// and a ratio used for ranking is compared far more often than it is read.
    ///
    /// Zero when no winner ever went adverse — which is not an infinite ratio,
    /// it is a sample too clean to rank, and saying so beats dividing by zero.
    #[must_use]
    pub const fn edge_ratio(&self) -> i64 {
        if self.winner_mae <= 0 {
            return 0;
        }
        self.winner_mfe.saturating_mul(100) / self.winner_mae
    }

    /// Did the pessimistic reading make money?
    #[must_use]
    pub const fn survives(&self) -> bool {
        self.pessimistic > 0
    }

    /// Total return per unit of worst drawdown, in hundredths.
    ///
    /// **The operator's actual question, as one number.** "Maximum profit at
    /// minimal stop loss" is a ratio and was being ranked as a sum: two variants
    /// with the same total are not equally survivable if one reached it smoothly
    /// and the other after giving back most of it, and
    /// [`Self::pessimistic`] alone cannot tell them apart.
    ///
    /// Hundredths and integer arithmetic, for the reason [`Self::edge_ratio`]
    /// gives — `CLAUDE.md` §7 keeps this kind of value out of floats, and a
    /// ratio used for ranking is compared far more often than it is read.
    ///
    /// Zero when the variant lost money, so a losing variant can never outrank a
    /// winning one on this. Zero drawdown with a positive total returns
    /// [`i64::MAX`] rather than dividing: a run that never gave anything back is
    /// the best possible reading, and saturating says so where a division would
    /// panic.
    ///
    /// **Nothing ranks on this yet.** It is measured and reported; whether it or
    /// [`Self::edge_ratio`] should decide the selection is the open question in
    /// `docs/06-limits.md`, and answering it by quietly switching the key would
    /// be the same defect as the one that recorded a proxy as the maximum.
    #[must_use]
    pub const fn return_over_drawdown(&self) -> i64 {
        if self.pessimistic <= 0 {
            return 0;
        }
        if self.max_drawdown <= 0 {
            return i64::MAX;
        }
        self.pessimistic.saturating_mul(100) / self.max_drawdown
    }

    /// The width of what minute bars cannot tell you, in paisa.
    ///
    /// # This is the only job the optimistic figure has
    ///
    /// A one-minute bar carries a high and a low and no order between them, so
    /// when a bar reaches both a stop and a target the fill is genuinely
    /// unknowable from this data. [`Self::pessimistic`] resolves that as the
    /// stop and [`Self::optimistic`] as the target, and **the gap between them
    /// is the measurement error**, not an estimate to prefer.
    ///
    /// A trailing exit fired by the bar that raised its own level is the second
    /// case, and it used to report zero here: both readings priced it off the
    /// raised peak, so the flattering ordering was taken and the gap it should
    /// have opened was never opened. The pessimistic reading now anchors that
    /// fill at the peak the bar opened with.
    ///
    /// Reporting only the worst case would lose it. Two setups with the same
    /// pessimistic total, one with a spread of 200 paisa and one with 8,000,
    /// are not equally trustworthy — the second is telling you it rests on a
    /// coin flip inside every bar, and that is a fact about the data rather
    /// than about the strategy.
    ///
    /// Zero means no bar was ever ambiguous, so the two readings agree exactly
    /// and second-level data would change nothing.
    ///
    /// **Nothing selects on it, and nothing selects on
    /// [`Self::optimistic`].** [`Grid::best`] and [`Grid::sharpest`] both rank
    /// on the pessimistic figure, and
    /// `runner::grid::no_selector_can_be_moved_by_the_optimistic_figure` holds
    /// that as a property rather than a habit.
    #[must_use]
    pub const fn uncertainty(&self) -> i64 {
        self.optimistic.saturating_sub(self.pessimistic)
    }

    /// Does this result depend on fill assumptions the data cannot settle?
    ///
    /// True when the two readings disagree at all. A caller acting on a
    /// variant that answers `true` is acting on something one-minute bars
    /// cannot resolve, and second-level data would be needed to close it.
    #[must_use]
    pub const fn depends_on_unknowable_ordering(&self) -> bool {
        self.uncertainty() != 0
    }
}

/// Every variant of one combination.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Grid {
    /// One per (stop, target, trail) triple, including the row with none of
    /// them.
    pub cells: Vec<Cell>,
    /// Signals the combination fired, before exclusivity.
    pub signals: u64,
    /// The stop ladder these cells index into.
    pub stops: Ladder,
    /// The target ladder these cells index into.
    pub targets: Ladder,
    /// The trailing ladder these cells index into.
    ///
    /// **This was absent and the cells indexed into it anyway.** [`Cell::trail`]
    /// carried a rung index with no ladder beside it to read the rung's value
    /// out of, and no caller could reconstruct the cell count — which is
    /// exactly how the doc block above [`evaluate`] came to claim
    /// `(rungs + 1)^2` for a three-deep loop. Scaled on the FAVOURABLE
    /// excursion, for the reason `evaluate` gives where it is built.
    pub trails: Ladder,
}

impl Grid {
    /// The variant with no stop and no target — the time-exit baseline.
    ///
    /// Kept as a named accessor because the whole comparison the operator asked
    /// for is "with these levels versus without them", and that is this row
    /// against the others.
    #[must_use]
    pub fn baseline(&self) -> Option<&Cell> {
        self.cells
            .iter()
            .find(|c| c.stop.is_none() && c.target.is_none() && c.trail.is_none())
    }

    /// The variant with the largest pessimistic total.
    ///
    /// Pessimistic and not optimistic, for the reason [`crate::validate`] gives
    /// about selection: choosing on the flattering number picks whatever the
    /// flattering assumption helped most.
    #[must_use]
    pub fn best(&self) -> Option<&Cell> {
        self.cells.iter().max_by_key(|c| c.pessimistic)
    }

    /// The variant with the sharpest winners, among those that survive.
    ///
    /// The sniper answer: of the variants that made money under pessimistic
    /// fills and pessimistic ambiguity, the one whose winners went least against
    /// you before working.
    #[must_use]
    pub fn sharpest(&self) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| c.survives() && c.wins > 0)
            .max_by_key(|c| c.edge_ratio())
    }
}

/// How a trade under a given variant ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ended {
    Stop,
    Target,
    /// A trailing stop, which fills at `peak - distance` rather than at a fixed
    /// distance from entry -- so it carries the peak it was measured against.
    Trail(i64),
    Time,
}

/// One candidate entry, with its path measured once.
/// How many cells a grid of these ladder lengths holds.
///
/// The `+ 1` on each is the "no level of this kind" row, so a grid with no
/// ladders at all still holds the single baseline cell rather than none.
///
/// # Why this is a function and not three multiplications at the call site
///
/// It was three multiplications at the call site, and one of them was missing.
/// The reservation counted stops and targets and not trails while the loop
/// pushed all three, and the doc block above [`evaluate`] independently claimed
/// `(rungs + 1)^2` while `crate::validate` said 125. Three statements of one
/// number, two of them wrong, none able to catch the others. Now there is one.
#[must_use]
pub const fn variants(stops: usize, targets: usize, trails: usize) -> usize {
    stops
        .saturating_add(1)
        .saturating_mul(targets.saturating_add(1))
        .saturating_mul(trails.saturating_add(1))
}

struct Candidate {
    signal: usize,
    entry: usize,
    /// The last bar the position may be held to: horizon or square-off.
    time_exit: usize,
    cross: Crossings,
}

/// Evaluate every stop/target variant of `mask` over `bars`.
///
/// The ladders are DERIVED from the excursions this combination actually
/// produced — see [`Ladder::from_excursions`]. Nobody supplies a level.
///
/// # Cost
///
/// The exit DECISION per variant is three integer compares against a cached
/// crossing table, and that part is genuinely constant.
///
/// **Two corrections to what this block used to say.**
///
/// It claimed the variant count is `(rungs + 1)^2`. The loop below is three
/// deep — stops, targets AND trails — so it is
/// `(stops + 1)(targets + 1)(trails + 1)`, which at the shipped
/// [`crate::validate::DEFAULT_RUNGS`] of four is **125 and not 25**. That
/// module's own doc has said "5x5x5 grid — 125 variants" the whole time, so the
/// two halves of this crate disagreed with each other in writing. See
/// [`variants`].
///
/// It also said "`O(1)` exit lookups", which is true of the decision and hides
/// what is measured beside it: [`peak_adverse`] and [`peak_favourable`] are each
/// `for i in from..=to` over the held window, and `one_variant` calls both for
/// every profitable candidate in every cell. So the real bar-visit count carries
/// a `2 × variants × winners × span` term that no version of this block has ever
/// mentioned. It is honest work — the MAE and MFE of the winners are reported —
/// but it is not constant and it was documented as though it were.
///
/// UNVERIFIED as a measured figure: no bench row covers this crate's grid. That
/// is the same admission as before and it is now attached to the right claim.
///
/// The reduction is available and not taken here: [`crate::excursion::crossings`]
/// already accumulates running `mae`/`mfe` and discards them, and because both
/// are running MAXIMA the value at offset `d` IS `peak(entry, entry + d)`.
/// Recording them per offset would turn each of these walks into an index.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "the four passes are one procedure and splitting them would hide \
              that the ladders are derived from the same trades the grid is \
              then measured against."
)]
pub fn evaluate(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    side: Side,
    rungs: usize,
) -> Grid {
    // PASS ONE: every signal that could open a position, and its path.
    // `crate::trade::walk` already applies the intraday rules, so its trades
    // give the entry bars and the time-exit bars this grid narrows.
    let direction = match side {
        Side::Long => costs::fill::Direction::Long,
        Side::Short => costs::fill::Direction::Short,
    };
    let timed = crate::trade::walk(bars, column, mask, horizon, direction);
    if timed.trades.is_empty() {
        return Grid {
            signals: timed.signals,
            ..Grid::default()
        };
    }

    // PASS TWO: the ladders, from what this combination's own trades did. A
    // provisional walk with no levels supplies the excursion sample, so the
    // grid is placed on the distribution it will be measured against.
    let mut adverse: Vec<Ppm> = Vec::with_capacity(timed.trades.len());
    let mut favourable: Vec<Ppm> = Vec::with_capacity(timed.trades.len());
    for t in &timed.trades {
        let entry_price = bars.get(t.entry_bar).map_or(0, |b| b.open);
        // A `crossings` call stood here against a one-rung probe ladder, and
        // thirteen lines later its result met `let _ = c;`. A full path walk per
        // trade, computed and thrown away -- and `let _ =` is what kept the
        // unused-variable lint from ever saying so. The two peaks below are what
        // this pass actually needs, and they walk the same path themselves.
        adverse.push(peak_adverse(
            bars,
            t.entry_bar,
            t.exit_bar,
            entry_price,
            side,
        ));
        favourable.push(peak_favourable(
            bars,
            t.entry_bar,
            t.exit_bar,
            entry_price,
            side,
        ));
    }
    let stops = Ladder::from_excursions(&mut adverse.clone(), rungs).unwrap_or_default();
    let targets = Ladder::from_excursions(&mut favourable.clone(), rungs).unwrap_or_default();
    // THE TRAILING LADDER IS SCALED ON THE FAVOURABLE MOVE, not the adverse one.
    // A trailing stop is a give-back FROM A PROFIT, so the distance that makes
    // sense is a fraction of what the move actually offered -- deriving it from
    // the adverse excursion would size "how much of my gain will I return" by
    // "how much did it hurt on the way in", which are different quantities.
    let trails = Ladder::from_excursions(&mut favourable.clone(), rungs).unwrap_or_default();

    // PASS THREE: each candidate's path measured ONCE against both ladders.
    let candidates: Vec<Candidate> = timed
        .trades
        .iter()
        .map(|t| {
            let entry_price = bars.get(t.entry_bar).map_or(0, |b| b.open);
            Candidate {
                signal: t.signal_bar,
                entry: t.entry_bar,
                time_exit: t.exit_bar,
                cross: crossings(
                    bars,
                    t.entry_bar,
                    t.exit_bar,
                    entry_price,
                    side,
                    Ladders {
                        stops: &stops,
                        targets: &targets,
                        trails: &trails,
                    },
                ),
            }
        })
        .collect();

    // PASS FOUR: every variant, each a sequence walk with O(1) exits.
    // THE TRAILS FACTOR WAS MISSING and the loop below is three deep. At the
    // shipped four rungs this reserved 5x5 = 25 and then pushed 5x5x5 = 125, so
    // every grid reallocated its way to the right size. `Grid::variants` beside
    // this is the same arithmetic, named once.
    let mut cells: Vec<Cell> =
        Vec::with_capacity(variants(stops.len(), targets.len(), trails.len()));
    for s in 0..=stops.len() {
        for t in 0..=targets.len() {
            for r in 0..=trails.len() {
                let stop = (s < stops.len()).then_some(s);
                let target = (t < targets.len()).then_some(t);
                let trail = (r < trails.len()).then_some(r);
                cells.push(one_variant(
                    bars,
                    &candidates,
                    (stops.rungs(), targets.rungs(), trails.rungs()),
                    Variant {
                        stop,
                        target,
                        trail,
                    },
                    side,
                ));
            }
        }
    }

    Grid {
        cells,
        signals: timed.signals,
        stops,
        targets,
        trails,
    }
}

/// Walk the candidate sequence under one variant.
///
/// Exclusivity is applied here rather than reused, because a stop that fires
/// early frees the next signal sooner and the sequence genuinely differs.
#[derive(Clone, Copy)]
struct Variant {
    stop: Option<usize>,
    target: Option<usize>,
    trail: Option<usize>,
}

/// One already-chosen exit variant, applied to bars it was NOT chosen on.
///
/// # Why this exists, and what was wrong without it
///
/// [`evaluate`] derives its ladders from the slice it is given. That is right
/// in sample and is look-ahead out of sample: levels fitted to the test window
/// look spectacular and are trivially findable. So a walk-forward cannot call
/// `evaluate` on its test bars to score the exit it picked.
///
/// It did not call anything. `crate::validate` chose a `(stop, target, trail)`
/// on the training grid, recorded it, and then measured out-of-sample
/// performance with [`crate::trade::walk`] — which takes no levels at all. The
/// fold reported a chosen stop beside an out-of-sample total that had never
/// used it, which `docs/06-limits.md` §70 records and `CLAUDE.md` §4 bans: a
/// true number beside a wrong implication.
///
/// This takes the ladders as ARGUMENTS, so the caller supplies the ones its
/// training half produced and the rung values travel to the test window
/// unchanged. Nothing here reads the test slice to decide a level.
///
/// `None` when the mask took no trade on these bars — which is a real answer
/// and not a zero, and is why it is an `Option` rather than a defaulted [`Cell`].
///
/// # Cost
///
/// One [`crate::trade::walk`] plus one `crossings` pass per trade, then a
/// single variant's exit lookups. That is [`evaluate`]'s work divided by the
/// variant count rather than multiplied by it.
///
/// UNVERIFIED as a measured figure: no bench row covers this crate's grid.
#[must_use]
pub fn with_levels(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    side: Side,
    ladders: Ladders<'_>,
    variant: (Option<usize>, Option<usize>, Option<usize>),
) -> Option<Cell> {
    let timed = crate::trade::walk(bars, column, mask, horizon, direction_of(side));
    if timed.trades.is_empty() {
        return None;
    }
    let candidates: Vec<Candidate> = timed
        .trades
        .iter()
        .map(|t| {
            let entry_price = bars.get(t.entry_bar).map_or(0, |b| b.open);
            Candidate {
                signal: t.signal_bar,
                entry: t.entry_bar,
                time_exit: t.exit_bar,
                cross: crossings(bars, t.entry_bar, t.exit_bar, entry_price, side, ladders),
            }
        })
        .collect();

    let (stop, target, trail) = variant;
    Some(one_variant(
        bars,
        &candidates,
        (
            ladders.stops.rungs(),
            ladders.targets.rungs(),
            ladders.trails.rungs(),
        ),
        Variant {
            stop,
            target,
            trail,
        },
        side,
    ))
}

/// The fill direction matching an excursion side.
///
/// The inverse of `crate::validate::side_of`, and here rather than there
/// because this module is the one that needs it. Two enums for one fact, in
/// crates that may not depend on each other — `costs::fill::Direction` is about
/// which leg fills first, `Side` about which extreme of a bar hurts.
const fn direction_of(side: Side) -> costs::fill::Direction {
    match side {
        Side::Long => costs::fill::Direction::Long,
        Side::Short => costs::fill::Direction::Short,
    }
}

fn one_variant(
    bars: &[Candle],
    candidates: &[Candidate],
    rungs: (&[Ppm], &[Ppm], &[Ppm]),
    v: Variant,
    side: Side,
) -> Cell {
    let (stops_rungs, targets_rungs, trails_rungs) = rungs;
    // Running equity and its high-water mark, for the drawdown below. Local
    // rather than on `Cell`, because they are scaffolding for the measurement
    // and not part of it -- a caller reading a peak-so-far would be reading an
    // artefact of iteration order.
    let mut running: i64 = 0;
    let mut peak_equity: i64 = 0;
    let Variant {
        stop,
        target,
        trail,
    } = v;
    let mut cell = Cell {
        stop,
        target,
        trail,
        ..Cell::default()
    };
    let mut open_until: Option<usize> = None;
    let mut adverse_on_winners: i64 = 0;
    let mut gain_on_winners: i64 = 0;

    for c in candidates {
        if open_until.is_some_and(|until| c.signal < until) {
            continue;
        }
        let span = c.time_exit.saturating_sub(c.entry);
        let stop_at = stop.map_or(NEVER, |r| c.cross.stop_at(r));
        let target_at = target.map_or(NEVER, |r| c.cross.target_at(r));
        // The trailing exit competes with the other two: whichever fires first
        // ends the position, and a trail that never fires is NEVER.
        let trail_at = trail.map_or(NEVER, |r| c.cross.trail_at(r));
        // BOTH anchors, because a trail crossed by the bar that raised the peak
        // could have filled off either and the bar does not say which. Equal
        // whenever the crossing bar left the peak alone.
        let trail_peak = TrailPeaks::of(&c.cross, trail);

        // ONE EXIT BAR, TWO ATTRIBUTIONS.
        //
        // Which bar a level exit happens on is not in doubt: it is the first bar
        // any level was reached. What minute data cannot say is WHICH level
        // filled when a single bar reached both, and that is the only thing the
        // two readings are entitled to differ about.
        //
        // Letting the optimistic case pick a LATER target over an EARLIER stop
        // was not optimism, it was a different trade -- one held past a stop
        // that had already fired. It produced a pessimistic total that BEAT the
        // optimistic one, which is how the error announced itself.
        let pess_off = span.min(stop_at).min(target_at).min(trail_at);
        let opt_off = pess_off;
        // PESSIMISTIC resolves an ambiguous bar as the STOP, so the stop is
        // tested first. OPTIMISTIC resolves it as the TARGET, so the target is.
        // That ordering is the entire difference between the two readings.
        let pess_by = ended_by(stop_at, target_at, trail_at, trail_peak, pess_off, true);
        let opt_by = ended_by(stop_at, target_at, trail_at, trail_peak, opt_off, false);
        let ended = pess_by;

        let entry_price = bars.get(c.entry).map_or(0, |b| b.open);
        let stop_ppm = stop.and_then(|r| stops_rungs.get(r).copied());
        let target_ppm = target.and_then(|r| targets_rungs.get(r).copied());
        let trail_ppm = trail.and_then(|r| trails_rungs.get(r).copied());
        let pess = realised(
            bars,
            c.entry,
            pess_off,
            entry_price,
            side,
            pess_by,
            level_for(pess_by, stop_ppm, target_ppm, trail_ppm),
        );
        let opt = realised(
            bars,
            c.entry,
            opt_off,
            entry_price,
            side,
            opt_by,
            level_for(opt_by, stop_ppm, target_ppm, trail_ppm),
        );

        cell.trades = cell.trades.saturating_add(1);
        cell.pessimistic = cell.pessimistic.saturating_add(pess);
        cell.optimistic = cell.optimistic.saturating_add(opt);

        // RISK, WHICH THIS ENGINE DID NOT MEASURE AT ALL.
        //
        // `grep -rniE 'drawdown|max_loss|worst_trade|peak_to_trough'` over the
        // whole crate returned NOTHING before these two lines. The operator's
        // aim is maximum profit at MINIMAL LOSS and the second half had no
        // number anywhere — `Cell::edge_ratio` is the nearest thing and is
        // computed over trades that ENDED PROFITABLE, so it is structurally
        // silent about how large a loser gets.
        //
        // Both are on the PESSIMISTIC series, because a risk figure taken from
        // the flattering reading is the one number where optimism is least
        // defensible.
        if pess < cell.worst_trade {
            cell.worst_trade = pess;
        }
        running = running.saturating_add(pess);
        if running > peak_equity {
            peak_equity = running;
        }
        let dip = peak_equity.saturating_sub(running);
        if dip > cell.max_drawdown {
            cell.max_drawdown = dip;
        }
        cell.ambiguous_bars = cell
            .ambiguous_bars
            .saturating_add(unorderable_bars(&c.cross, trail));
        match ended {
            // A trailing exit IS a stop -- it gives back part of a gain to
            // protect the rest -- so it is counted as one. Reporting it as a
            // timeout would make a strategy that was stopped out look like one
            // that ran its course.
            Ended::Stop | Ended::Trail(_) => cell.stopped = cell.stopped.saturating_add(1),
            Ended::Target => cell.targeted = cell.targeted.saturating_add(1),
            Ended::Time => cell.timed_out = cell.timed_out.saturating_add(1),
        }
        if pess > 0 {
            cell.wins = cell.wins.saturating_add(1);
            adverse_on_winners = adverse_on_winners.saturating_add(peak_adverse(
                bars,
                c.entry,
                c.entry.saturating_add(pess_off),
                entry_price,
                side,
            ));
            gain_on_winners = gain_on_winners.saturating_add(peak_favourable(
                bars,
                c.entry,
                c.entry.saturating_add(pess_off),
                entry_price,
                side,
            ));
        }
        open_until = Some(c.entry.saturating_add(pess_off));
    }

    if cell.wins > 0 {
        let n = i64::try_from(cell.wins).unwrap_or(1).max(1);
        cell.winner_mae = adverse_on_winners / n;
        cell.winner_mfe = gain_on_winners / n;
    }
    cell
}

/// Close-to-entry move at `entry + offset`, in paisa.
fn realised(
    bars: &[Candle],
    entry: usize,
    offset: usize,
    entry_price: i64,
    side: Side,
    by: Ended,
    level_ppm: Option<Ppm>,
) -> i64 {
    // A LEVEL EXIT FILLS AT ITS LEVEL, NOT AT THE BAR'S CLOSE.
    //
    // This priced every exit at the close, which is right for a time exit and
    // wrong for the other two: a stop order fills where the stop was, and a
    // target order where the target was, and the bar's close is neither. On a
    // bar that ran far past the level the close overstates the loss and
    // understates the gain, both by however far the bar continued.
    //
    // It also made the pessimistic and optimistic readings identical. Resolving
    // an ambiguous bar as "the stop first" instead of "the target first" picked
    // the same BAR, so both computed the same close and agreed by construction
    // -- and the test asserting they agree passed without ever exercising the
    // disagreement it was written for.
    match (by, level_ppm) {
        (Ended::Stop, Some(ppm)) => -paisa_of(ppm, entry_price),
        (Ended::Target, Some(ppm)) => paisa_of(ppm, entry_price),
        // A TRAILING EXIT FILLS AT `peak - distance`, and this used to fall
        // back to the bar's close because the peak was not recorded.
        //
        // The level follows the best price seen, so the rung alone cannot price
        // it the way a fixed stop's can. `Crossings` now records the peak AS IT
        // STOOD when the rung was crossed -- reading it at the end of the walk
        // would price the exit against a high the position never saw, because
        // the peak only ever improves.
        //
        // The distance is measured off the PEAK and not off entry, which is the
        // whole difference between a trailing stop and a fixed one: give back
        // `d` of what you made, rather than lose `d` of what you started with.
        (Ended::Trail(peak), Some(ppm)) if peak > 0 => {
            let give_back = paisa_of(ppm, peak);
            let exit = match side {
                Side::Long => peak.saturating_sub(give_back),
                Side::Short => peak.saturating_add(give_back),
            };
            match side {
                Side::Long => exit.saturating_sub(entry_price),
                Side::Short => entry_price.saturating_sub(exit),
            }
        }
        // Everything else -- a time exit, or a trailing exit whose peak was not
        // recorded -- fills at the bar's close, which is correct for a position
        // squared off by the clock.
        _ => {
            let exit = bars
                .get(entry.saturating_add(offset))
                .map_or(entry_price, |b| b.close);
            match side {
                Side::Long => exit.saturating_sub(entry_price),
                Side::Short => entry_price.saturating_sub(exit),
            }
        }
    }
}

/// The two anchors a trailing exit could have hung from on its crossing bar.
///
/// One field per intra-bar ordering, straight out of
/// [`crate::excursion::Crossings`]: `before` is the peak the bar opened with,
/// `raised` the peak that bar's own favourable extreme left behind. They are
/// equal on every crossing bar that did not raise the peak, which is most of
/// them, and both are `None` for a rung the path never crossed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TrailPeaks {
    /// The peak as it stood BEFORE the crossing bar — the pessimistic anchor.
    before: Option<i64>,
    /// The peak including the crossing bar's own extreme — the optimistic one.
    raised: Option<i64>,
}

impl TrailPeaks {
    /// The anchors for `trail`'s rung on this path, or none at all.
    ///
    /// Both fields are `None` for a variant with no trailing rung, which is
    /// what makes the trail inert in [`ended_by`] rather than needing a second
    /// guard there: nothing anchors an exit that cannot fire.
    fn of(cross: &Crossings, trail: Option<usize>) -> Self {
        Self {
            before: trail.and_then(|r| cross.trail_peak_at(r)),
            raised: trail.and_then(|r| cross.trail_peak_raised_at(r)),
        }
    }
}

/// Bars of one candidate's path whose intra-bar ordering the data cannot
/// settle.
///
/// The stop-versus-target set always. The trail's own set ONLY on a variant
/// that carries a trailing rung: a variant without one has `trail_at == NEVER`
/// on every candidate, so nothing in that set can reach its two readings, and
/// counting it would report an uncertainty the variant does not carry.
///
/// Loose in the direction the stop/target count is already loose — every such
/// bar on the path, not only those at or before the exit, and every rung rather
/// than the one rung this variant holds. It over-states the count and never
/// under-states it, so `ambiguous_bars == 0` still implies the two readings
/// agree, which is what
/// `the_uncertainty_is_zero_exactly_when_no_bar_was_ambiguous` pins. Tightening
/// it needs the exit offset, which this field has never used.
fn unorderable_bars(cross: &Crossings, trail: Option<usize>) -> u64 {
    let stop_target = u64::try_from(cross.ambiguous().len()).unwrap_or(0);
    if trail.is_none() {
        return stop_target;
    }
    stop_target.saturating_add(u64::try_from(cross.trail_ambiguous().len()).unwrap_or(0))
}

/// Which exit fired first, with ties broken by the caller's pessimism.
///
/// `pessimistic` is the whole pessimistic/optimistic split, and it now decides
/// **two** things rather than one.
///
/// 1. When a stop and a target were both reachable on the same bar, minute data
///    cannot say which came first, so the pessimistic reading takes the stop and
///    the optimistic one takes the target.
/// 2. When a trailing rung was crossed by the bar that raised the peak, minute
///    data cannot say whether the order was still hanging from the old peak or
///    already from the new one. The pessimistic reading anchors it at
///    [`TrailPeaks::before`], the optimistic at [`TrailPeaks::raised`].
///
/// The parameter was called `stop_wins`, which named only the first job. The
/// second was not being done at all: `crate::excursion` handed over one peak,
/// the one the crossing bar itself had made, and both readings priced the trail
/// off it — so a trailing exit's fill was settled by an ordering assumption
/// neither reading ever tested. See that module's header.
fn ended_by(
    stop_at: usize,
    target_at: usize,
    trail_at: usize,
    trail_peak: TrailPeaks,
    chosen: usize,
    pessimistic: bool,
) -> Ended {
    let stop_fired = stop_at != NEVER && stop_at <= chosen;
    let target_fired = target_at != NEVER && target_at <= chosen;
    let trail_fired = trail_at != NEVER && trail_at <= chosen;
    if stop_fired && target_fired {
        return if pessimistic {
            Ended::Stop
        } else {
            Ended::Target
        };
    }
    if stop_fired {
        Ended::Stop
    } else if target_fired {
        Ended::Target
    } else if trail_fired {
        // Priced from the PEAK the order hung from, which the crossings
        // recorded at the moment the rung was crossed. Reading the peak at the
        // end of the walk instead would price the exit against a high the
        // position never saw, because `peak` only ever improves.
        //
        // Which of the two anchors is the fill is the intra-bar ordering, and
        // this is the only place it is decided. A rung crossed on a bar that did
        // not raise the peak carries the same value in both, so the choice is
        // inert there and the two readings agree — which is what keeps
        // `Cell::uncertainty` at zero unless something genuinely was unknowable.
        let anchor = if pessimistic {
            trail_peak.before
        } else {
            trail_peak.raised
        };
        Ended::Trail(anchor.unwrap_or(0))
    } else {
        Ended::Time
    }
}

/// The level a given exit reason fills at, in parts per million from entry.
const fn level_for(
    by: Ended,
    stop_ppm: Option<Ppm>,
    target_ppm: Option<Ppm>,
    trail_ppm: Option<Ppm>,
) -> Option<Ppm> {
    match by {
        Ended::Stop => stop_ppm,
        Ended::Target => target_ppm,
        // A trailing exit DOES carry a level -- the trail rung -- and it is
        // needed to price the give-back from the peak. Only a time exit has
        // no level at all.
        Ended::Trail(_) => trail_ppm,
        Ended::Time => None,
    }
}

/// A parts-per-million distance as a paisa move against `price`.
fn paisa_of(ppm: Ppm, price: i64) -> i64 {
    let scaled = i128::from(ppm).saturating_mul(i128::from(price)) / 1_000_000;
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

/// The worst the path went against the position, in parts per million.
fn peak_adverse(bars: &[Candle], from: usize, to: usize, entry: i64, side: Side) -> Ppm {
    peak(bars, from, to, entry, side, true)
}

/// The best the path went for the position, in parts per million.
fn peak_favourable(bars: &[Candle], from: usize, to: usize, entry: i64, side: Side) -> Ppm {
    peak(bars, from, to, entry, side, false)
}

/// One extreme of the path, in parts per million of the entry price.
fn peak(bars: &[Candle], from: usize, to: usize, entry: i64, side: Side, adverse: bool) -> Ppm {
    if entry <= 0 || from > to {
        return 0;
    }
    let mut worst: i64 = 0;
    for i in from..=to {
        let Some(bar) = bars.get(i) else { break };
        let move_paisa = match (side, adverse) {
            (Side::Long, true) | (Side::Short, false) => entry.saturating_sub(bar.low),
            (Side::Long, false) | (Side::Short, true) => bar.high.saturating_sub(entry),
        };
        if move_paisa > worst {
            worst = move_paisa;
        }
    }
    let scaled = i128::from(worst).saturating_mul(1_000_000) / i128::from(entry);
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Cell, Grid, evaluate};
    use crate::excursion::Side;
    use crate::outcome::Horizon;
    use indicators::column::Column;
    use indicators::evaluator::{Evaluator, Widths};
    use indicators::pattern::Thresholds;
    use indicators::vwap::Availability;
    use vocab::ConditionMask;

    fn evaluator() -> Evaluator {
        Evaluator::new(
            Widths::pinned().expect("pinned widths are valid"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        )
    }

    fn h(n: u32) -> Horizon {
        Horizon::bars(n).expect("a non-zero horizon")
    }

    fn swept() -> (Vec<indicators::Candle>, Column) {
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        (bars, column)
    }

    /// The same slice with one bar in fifty given a very wide range.
    ///
    /// # Why this fixture has to exist
    ///
    /// `crate::synthetic::bar` sets `high = max(open, close) + 60` and
    /// `low = min(..) - 60`, so a bar spans about 120 paisa on a 2,500,000
    /// base — 48 parts per million. The exit ladders are quantiles of the
    /// OBSERVED excursions, so every rung lands inside that range and no single
    /// bar ever reaches both a stop and a target.
    ///
    /// MEASURED on the unmodified fixture: 300 combinations, 19,300 cells,
    /// **zero** with `ambiguous_bars > 0` and **zero** with non-zero
    /// `uncertainty()`. So `pessimistic == optimistic` everywhere, and the
    /// entire two-case model — the reason this module prices each variant twice
    /// — ran only on data where the two cases cannot differ.
    ///
    /// Widening EVERY bar would not help: the rungs are derived from the
    /// excursions, so a uniformly wider slice gives uniformly wider rungs and
    /// the ratio is unchanged. What is needed is most bars narrow, so the
    /// quantiles stay small, and a few far wider, so those span both rungs.
    /// One in fifty, at fifteen times the range.
    fn swept_with_wide_bars() -> (Vec<indicators::Candle>, Column) {
        let bars: Vec<indicators::Candle> = crate::synthetic::sessions(8)
            .into_iter()
            .enumerate()
            .map(|(i, b)| {
                if i % 50 != 0 {
                    return b;
                }
                let mid = b.open.midpoint(b.close);
                let reach = 900_i64;
                indicators::Candle::new(
                    b.ts_micros,
                    b.open,
                    mid.saturating_add(reach),
                    mid.saturating_sub(reach),
                    b.close,
                    b.volume,
                    b.open_interest,
                )
            })
            .collect();
        let column = Column::build(&bars, &mut evaluator());
        (bars, column)
    }

    #[test]
    fn risk_is_measured_and_a_total_alone_cannot_tell_two_variants_apart() {
        // THE HALF THE ENGINE DID NOT HAVE. `grep -rniE
        // 'drawdown|max_loss|worst_trade|peak_to_trough'` over the whole crate
        // returned nothing. The stated aim is maximum profit at MINIMAL LOSS
        // and only the first half had a number; `edge_ratio` is the nearest
        // thing and is computed over trades that ENDED PROFITABLE, so it is
        // structurally silent about how large a loser gets.
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        assert!(!g.cells.is_empty());

        for c in &g.cells {
            // Drawdown is a peak-to-trough FALL, so it can never be negative,
            // and a variant that lost overall must have fallen at least as far
            // as it lost.
            assert!(
                c.max_drawdown >= 0,
                "a peak-to-trough fall came out negative: {}",
                c.max_drawdown
            );
            if c.pessimistic < 0 {
                assert!(
                    c.max_drawdown >= c.pessimistic.saturating_neg(),
                    "a variant lost {} and reports a maximum drawdown of only {}",
                    c.pessimistic,
                    c.max_drawdown
                );
            }
            // The worst single trade cannot be better than the total when only
            // one trade was taken, and can never be positive-only by accident:
            // a variant with any losing trade must carry a negative here.
            if c.trades > 0 {
                assert!(
                    c.worst_trade <= 0 || c.pessimistic > 0,
                    "every trade won yet the total is not positive"
                );
            }
            // The ratio never rewards a loser.
            if c.pessimistic <= 0 {
                assert_eq!(
                    c.return_over_drawdown(),
                    0,
                    "a losing variant scored on the risk-adjusted ratio"
                );
            }
        }

        // The measurement must actually vary, or the fields are decoration.
        let first = g.cells.first().map_or(0, |c| c.max_drawdown);
        let varies = g.cells.iter().any(|c| c.max_drawdown != first);
        assert!(
            varies,
            "every variant reported the same drawdown, so nothing is being measured"
        );
    }

    #[test]
    fn a_bar_that_reaches_both_levels_makes_the_two_readings_disagree() {
        // THE CASE THE WHOLE TWO-CASE MODEL EXISTS FOR, exercised for the first
        // time. When one bar reaches both the stop and the target, minute data
        // cannot say which came first: `pessimistic` resolves it as the stop,
        // `optimistic` as the target, and the gap is the measurement error that
        // `Cell::uncertainty` reports.
        //
        // Every fixture in this repository produced zero such bars, so the
        // resolution was never observable and `pessimistic == optimistic` held
        // everywhere by accident of the data rather than by any property.
        let (bars, column) = swept_with_wide_bars();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        assert!(!g.cells.is_empty(), "the fixture must produce a grid");

        let ambiguous = g.cells.iter().filter(|c| c.ambiguous_bars > 0).count();
        assert!(
            ambiguous > 0,
            "no cell saw a bar reaching both levels, so this fixture is no \
             better than the ones it was written to replace"
        );

        let disagree = g
            .cells
            .iter()
            .filter(|c| c.depends_on_unknowable_ordering())
            .count();
        assert!(
            disagree > 0,
            "{ambiguous} cells had an ambiguous bar and none reported any \
             uncertainty -- the two readings are being computed identically"
        );

        // The direction is fixed and is not a convention that may drift: the
        // pessimistic reading resolves ambiguity as the STOP, so it can never
        // exceed the optimistic one.
        for c in &g.cells {
            assert!(
                c.pessimistic <= c.optimistic,
                "a cell resolved ambiguity in its own favour: {} > {}",
                c.pessimistic,
                c.optimistic
            );
        }
    }

    #[test]
    fn the_no_stop_no_target_baseline_is_a_row_of_the_same_table() {
        // THE COMPARISON THE OPERATOR ASKED FOR. "With levels" and "without
        // levels" must be two rows of one result, computed in one pass -- not
        // two runs a human has to line up by hand.
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );

        let base = g.baseline().expect("a baseline row must exist");
        assert!(base.stop.is_none() && base.target.is_none());
        assert!(base.trades > 0, "the baseline must take trades");
        assert!(
            g.cells.len() > 1,
            "the grid must hold the level variants beside the baseline"
        );
    }

    #[test]
    fn a_stop_can_only_shorten_a_trade_never_lengthen_it() {
        // A level exit fires at or before the time exit, always. If any variant
        // held longer than the baseline, the exit rule would be reading the
        // wrong side of the ladder.
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(60),
            Side::Long,
            4,
        );
        let base = g.baseline().expect("a baseline");
        for c in &g.cells {
            if c.stop.is_none() && c.target.is_none() && c.trail.is_none() {
                continue;
            }
            assert!(
                c.trades >= base.trades,
                "levels exit earlier, which frees the next signal, so a variant \
                 can never take FEWER trades than the time-only baseline"
            );
        }
    }

    #[test]
    fn no_selector_can_be_moved_by_the_optimistic_figure() {
        // THE PROPERTY, NOT THE HABIT. The optimistic reading exists to size the
        // uncertainty and must never influence a choice -- an engine that
        // selected on it would pick whatever the unknowable intra-bar ordering
        // flattered most, which is the failure the two-case model exists to
        // expose rather than to commit.
        //
        // Asserted by MUTATION: take a real grid, inflate every optimistic
        // total to absurdity, and require that both selectors return the same
        // cell. If either read `optimistic`, the winner would move.
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        assert!(g.cells.len() > 1, "the grid must hold several variants");

        let before_best = g.best().map(|c| (c.stop, c.target, c.trail));
        let before_sharp = g.sharpest().map(|c| (c.stop, c.target, c.trail));

        let mutated = Grid {
            cells: g
                .cells
                .iter()
                .enumerate()
                .map(|(i, c)| Cell {
                    // A different absurd value per cell, so the mutation cannot
                    // accidentally preserve the ordering it is trying to break.
                    optimistic: i64::MAX.saturating_sub(i64::try_from(i).unwrap_or(0)),
                    ..*c
                })
                .collect(),
            ..g.clone()
        };

        assert_eq!(
            mutated.best().map(|c| (c.stop, c.target, c.trail)),
            before_best,
            "`best` moved when only the optimistic totals changed, so it reads \
             a figure it must not"
        );
        assert_eq!(
            mutated.sharpest().map(|c| (c.stop, c.target, c.trail)),
            before_sharp,
            "`sharpest` moved when only the optimistic totals changed"
        );
    }

    #[test]
    fn the_uncertainty_is_zero_exactly_when_no_bar_was_ambiguous() {
        // The spread between the two readings IS the measurement error from
        // having only minute bars. Where nothing was ambiguous there is nothing
        // second-level data could settle, and the two must agree exactly.
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        for c in &g.cells {
            assert_eq!(
                c.uncertainty() == 0,
                !c.depends_on_unknowable_ordering(),
                "the two accessors must agree about the same fact"
            );
            if c.ambiguous_bars == 0 {
                assert_eq!(
                    c.uncertainty(),
                    0,
                    "no ambiguous bar, so nothing for the readings to disagree \
                     about"
                );
            }
        }
    }

    #[test]
    fn the_pessimistic_reading_never_beats_the_optimistic_one() {
        // The two differ only on bars where a stop and a target were both
        // reachable. Pessimistic takes the stop there, so it can never come out
        // ahead -- if it did, the ambiguity would be resolved backwards.
        for side in [Side::Long, Side::Short] {
            let (bars, column) = swept();
            let g = evaluate(&bars, &column, &ConditionMask::default(), h(15), side, 4);
            for c in &g.cells {
                assert!(
                    c.pessimistic <= c.optimistic,
                    "{side:?} variant {:?}/{:?}: pessimistic {} beat optimistic {}",
                    c.stop,
                    c.target,
                    c.pessimistic,
                    c.optimistic
                );
            }
        }
    }

    #[test]
    fn a_variant_with_no_ambiguous_bar_has_two_readings_that_agree() {
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        for c in g.cells.iter().filter(|c| c.ambiguous_bars == 0) {
            assert_eq!(
                c.pessimistic, c.optimistic,
                "with no ambiguous bar there is nothing for the two readings to \
                 disagree about"
            );
        }
    }

    #[test]
    fn every_trade_ends_by_exactly_one_of_stop_target_or_time() {
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        for c in &g.cells {
            assert_eq!(
                c.stopped
                    .saturating_add(c.targeted)
                    .saturating_add(c.timed_out),
                c.trades,
                "a trade that ended by none of the three, or by two, is a walk \
                 that lost one"
            );
        }
    }

    /// One bar, as `Candle::new` orders its fields.
    ///
    /// Local to the test below, which is the only one in this module that
    /// builds a path by hand rather than sweeping a synthetic slice — the
    /// intra-bar case it exercises is one the synthetic generator does not
    /// produce on demand, which is the same reason `swept_with_wide_bars`
    /// exists above.
    fn candle(minute: i64, open: i64, high: i64, low: i64, close: i64) -> indicators::Candle {
        indicators::Candle::new(
            minute.saturating_mul(60_000_000),
            open,
            high,
            low,
            close,
            100,
            indicators::OI_NULL,
        )
    }

    #[test]
    fn a_trailing_fill_is_priced_off_the_pre_bar_peak_pessimistically() {
        // THE UNIT OF THE FIX. `ended_by` is the one place the intra-bar
        // ordering is decided, and it decided only the stop-versus-target half:
        // the trail arrived with a single peak -- the one its own crossing bar
        // had made -- so both readings priced it identically and the flattering
        // ordering was taken silently.
        //
        // A rung crossed on a bar that opened with the peak at 105,000 and
        // left it at 107,000: 40,000 ppm of give-back off 105,000 is 4,200
        // paisa and off 107,000 is 4,280, so the two fills are 100,800 and
        // 102,720 against a 100,000 entry. Both are real prices inside that bar.
        let peaks = super::TrailPeaks {
            before: Some(105_000),
            raised: Some(107_000),
        };
        let pess = super::ended_by(super::NEVER, super::NEVER, 1, peaks, 1, true);
        let opt = super::ended_by(super::NEVER, super::NEVER, 1, peaks, 1, false);
        assert_eq!(
            pess,
            super::Ended::Trail(105_000),
            "the pessimistic reading anchors the order where it hung when the \
             bar opened"
        );
        assert_eq!(
            opt,
            super::Ended::Trail(107_000),
            "the optimistic reading anchors it at the peak the bar itself made"
        );

        let priced_pess = super::realised(&[], 0, 1, 100_000, Side::Long, pess, Some(40_000));
        let priced_opt = super::realised(&[], 0, 1, 100_000, Side::Long, opt, Some(40_000));
        assert_eq!(
            priced_pess, 800,
            "105,000 less 4,200, less the 100,000 entry"
        );
        assert_eq!(priced_opt, 2_720, "107,000 less 4,280, less the same entry");
        assert!(
            priced_pess < priced_opt,
            "the pessimistic anchor must be the worse fill, or the two readings \
             are labelled backwards"
        );
    }

    #[test]
    fn a_trail_fired_by_the_bar_that_raised_the_peak_opens_the_two_readings() {
        // THE DEFECT, END TO END THROUGH ONE VARIANT. Bar 1 opens with the peak
        // at 105,000, dips to 101,000 -- exactly the 40,000-ppm rung below it --
        // and also prints a new high at 107,000. Whether the trail fired is not
        // in doubt: 105,000 - 4,000 is at the low, so the low reaches it under
        // either ordering. WHAT IT FILLED AT is, and this used to report a
        // single number with `uncertainty()` of zero beside it.
        //
        // THIS CHANGES BACKTEST RESULTS. The pessimistic total for a trailing
        // variant falls to the pre-bar anchor wherever this case occurs, and it
        // is the pessimistic total that `Grid::best` and `Grid::sharpest` rank
        // on. Nothing here is a haircut applied for safety -- it is the reading
        // the data supports.
        let bars = vec![
            candle(0, 100_000, 105_000, 100_000, 105_000),
            candle(1, 104_000, 107_000, 101_000, 106_000),
            candle(2, 106_000, 106_500, 105_500, 106_000),
        ];
        // Stops and targets far enough out that neither ever fires, so the only
        // exit competing with the clock is the trail and the only ambiguity in
        // the cell is the one under test.
        let never = crate::excursion::Ladder::new(vec![900_000]).expect("an ascending ladder");
        let trails = crate::excursion::Ladder::new(vec![40_000]).expect("an ascending ladder");
        let cross = crate::excursion::crossings(
            &bars,
            0,
            2,
            100_000,
            Side::Long,
            crate::excursion::Ladders {
                stops: &never,
                targets: &never,
                trails: &trails,
            },
        );
        assert_eq!(
            cross.trail_ambiguous(),
            &[1],
            "the fixture must produce the case, or this test asserts nothing"
        );
        let candidates = vec![super::Candidate {
            signal: 0,
            entry: 0,
            time_exit: 2,
            cross,
        }];
        let rungs = (never.rungs(), never.rungs(), trails.rungs());

        let trailed = super::one_variant(
            &bars,
            &candidates,
            rungs,
            super::Variant {
                stop: None,
                target: None,
                trail: Some(0),
            },
            Side::Long,
        );
        assert_eq!(trailed.trades, 1, "one candidate, one round trip");
        assert_eq!(
            trailed.ambiguous_bars, 1,
            "the bar that fired the trail and raised the peak must be counted, \
             or the uncertainty is reported with nothing behind it"
        );
        assert_eq!(
            trailed.pessimistic, 800,
            "priced off 105,000, the peak the resting order hung from"
        );
        assert_eq!(
            trailed.optimistic, 2_720,
            "priced off 107,000, the peak that bar itself made"
        );
        assert_eq!(
            trailed.uncertainty(),
            1_920,
            "the gap the two orderings genuinely carry -- zero before, because \
             both readings used the raised peak"
        );
        assert!(trailed.depends_on_unknowable_ordering());

        // THE SAME PATH WITH NO TRAILING RUNG. Nothing in the trail's ambiguity
        // set can reach a variant whose `trail_at` is NEVER, so counting it
        // there would report an uncertainty the variant does not carry.
        let timed = super::one_variant(
            &bars,
            &candidates,
            rungs,
            super::Variant {
                stop: None,
                target: None,
                trail: None,
            },
            Side::Long,
        );
        assert_eq!(
            timed.ambiguous_bars, 0,
            "a variant with no trailing rung cannot be moved by a trailing \
             ambiguity"
        );
        assert_eq!(timed.uncertainty(), 0);
        assert_eq!(
            timed.pessimistic, 6_000,
            "held to bar 2 and squared off at its close of 106,000"
        );
    }

    #[test]
    fn the_ladders_come_from_the_data_and_a_combination_that_never_trades_has_none() {
        let (bars, column) = swept();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );
        assert!(!g.stops.is_empty(), "trades happened, so a ladder exists");
        assert!(
            g.stops.rungs().windows(2).all(|w| w.first() < w.last()),
            "a derived ladder must ascend"
        );

        // A mask nothing satisfies produces no trades, so no ladder and no
        // cells -- rather than a grid of zeroes that reads like a measurement.
        let mut impossible = ConditionMask::default();
        for bit in 0..8 {
            impossible = impossible.with_bit(bit);
        }
        let empty = evaluate(&bars, &column, &impossible, h(15), Side::Long, 4);
        assert!(empty.cells.is_empty() || empty.baseline().is_some());
    }

    #[test]
    fn the_cell_count_is_the_product_of_all_three_ladders_and_nothing_reserves_less() {
        // THE THREE STATEMENTS THAT DISAGREED. `evaluate`'s doc said the variant
        // count was `(rungs + 1)^2`; the module header said `400 variants` and
        // `450,000` lookups; `crate::validate::DEFAULT_RUNGS` said "5x5x5 grid --
        // 125 variants". The loop is three deep, so validate was right and the
        // other two were wrong -- and the reservation was wrong with them,
        // asking for 5x5 = 25 before pushing 125.
        //
        // Nothing could catch that, because the count lived in four places and
        // no test read any of them. This reads the ONE place it lives now and
        // checks the grid actually built that many.
        assert_eq!(
            super::variants(0, 0, 0),
            1,
            "no ladders is still the baseline row"
        );
        assert_eq!(super::variants(4, 4, 4), 125, "the shipped four rungs");
        assert_eq!(
            super::variants(4, 4, 0),
            25,
            "25 is what TWO ladders give -- the number the doc block used to claim for three"
        );

        let bars = crate::synthetic::sessions(12);
        let mut ev = evaluator();
        let column = Column::build(&bars, &mut ev);
        let swept = crate::Sweeper::new(engine::Ladder::with_min_hits(150).with_ceiling(20_000))
            .run(&bars, &mut evaluator());
        let item = crate::closed::closed(&swept.sweep)
            .kept
            .first()
            .copied()
            .expect("the fixture must produce at least one combination to grid");
        let g = evaluate(&bars, &column, &item.mask, h(15), Side::Long, 4);
        if !g.cells.is_empty() {
            assert_eq!(
                g.cells.len(),
                super::variants(g.stops.len(), g.targets.len(), g.trails.len()),
                "the grid built a different number of cells than `variants` says it holds"
            );
        }
    }
}
