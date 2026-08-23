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
//! computed once per candidate entry, and every variant's exit is then one
//! table choice and three integer compares. At the shipped four rungs that is
//! `325 variants x 1,124 signals` — 365,300 lookups against 1,124 path walks,
//! not 365,300 path walks.
//!
//! **This paragraph has now been wrong three times, and each correction is why
//! the count lives in exactly one function.** It read `400 variants` and
//! `450,000`, which was `(rungs+1)^2` at twenty rungs from a two-ladder design —
//! the trailing ladder had made the nest three deep without the arithmetic
//! following it. It then read `125` and `140,500`, correct for a three-deep nest
//! and wrong the moment the arming axis made it four. See [`variants`], which is
//! the only place the number is computed and which now refuses two families of
//! cell rather than one.
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

/// A trailing take profit: the rung that arms it, and the rung it then trails
/// by.
///
/// One type rather than two `Option<usize>` fields on [`Cell`], because the two
/// are meaningless apart — an arm with no trail arms nothing, and an armed trail
/// with no arm is a plain TSL. Bundling them makes the invalid pair unspellable
/// instead of merely undocumented.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ttp {
    /// Rung on the TARGET ladder at which the trail wakes up.
    ///
    /// Indexes the target ladder because "start protecting once it has run this
    /// far" is a favourable-excursion question, and that ladder is already
    /// scaled on that distribution. No fourth ladder exists.
    pub arm: usize,
    /// Rung on the TRAILING ladder the armed order then follows the peak by.
    ///
    /// Strictly below [`Cell::tsl`] when a TSL is also set — see that field.
    pub trail: usize,
}

/// One (stop, target, TSL, TTP) variant's result over the whole slice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    /// Index into the stop ladder, or `None` for **no stop**.
    pub stop: Option<usize>,
    /// Index into the target ladder, or **no target**.
    pub target: Option<usize>,
    /// The TRAILING STOP LOSS: a trailing ladder rung, live from entry, or
    /// `None`.
    ///
    /// Follows the best price seen and fires on the give-back. It **protects a
    /// gain**: live from the first bar, it cuts a position that turns.
    ///
    /// This field was `trail`, and it carried both instruments — `arm = None`
    /// meant it was a stop loss and `arm = Some(a)` meant it was a take profit,
    /// so a cell could be one or the other and never both. See [`Self::ttp`].
    pub tsl: Option<usize>,
    /// The TRAILING TAKE PROFIT: an arming rung and a trailing rung, or `None`.
    ///
    /// # A separate field, because it is a separate order
    ///
    /// A trailing take profit **extends a winner**: dormant until the move has
    /// already paid, it then lets the position run instead of closing it at the
    /// target. That is the opposite job from [`Self::tsl`], which is why both
    /// exist — and until this field they shared one slot, so a cell could carry
    /// one or the other and the 4 on/off patterns with both were unreachable.
    ///
    /// The engine could express `SL·TP·TSL`, `SL·TP·TTP`, every "no X", and
    /// every single instrument — 12 of the 16 on/off patterns. The 4 missing
    /// ones were exactly *loose trail from entry, tighter trail once it pays*,
    /// which is a strategy an operator asks for by name.
    ///
    /// # The armed trail must be strictly TIGHTER, and that is not a
    /// simplification
    ///
    /// Once armed, both trails hang from the same running peak, so only the
    /// SMALLER distance can ever fire. A `ttp.trail` at or above `tsl` is
    /// therefore inert — the live trail already fires first — and every such
    /// cell would be byte-identical to the TSL-only cell beside it.
    ///
    /// That is the same degeneracy `variants` records refusing twice already,
    /// and refusing it a third time is what keeps the grid at **625** instead of
    /// the 1,125 an unconstrained pair would need. Ladder rungs ascend, so
    /// "tighter" is `ttp.trail < tsl` on the index.
    ///
    /// # This is the ONLY pair that composes
    ///
    /// A stop and a target are distances from ENTRY and can coexist trivially.
    /// Two trails both hang from the PEAK, so they interact — and the
    /// interaction is the whole point: before arming the position is protected
    /// loosely, after arming tightly, and no single trail rung expresses that.
    pub ttp: Option<Ttp>,
    /// Round trips taken under this variant.
    pub trades: u64,
    /// Trades that ended above water, resolving ambiguity against you.
    pub wins: u64,
    /// Total paisa per unit under the WORST reading of BOTH legs: entered at
    /// the worst price the execution bar PRINTED, ambiguity resolved as
    /// the STOP first.
    ///
    /// The entry half of that sentence is new. This was a worst-case exit off a
    /// the bar's OPEN — a best-case entry — so the figure every selector in this
    /// module ranks on was never charged for entering badly. See
    /// [`Cell::fill_cost`] for the amount and why it is reported apart.
    pub pessimistic: i64,
    /// Total paisa per unit under the BEST reading of BOTH legs: entered at the
    /// execution bar's open, ambiguity resolved as the TARGET first.
    pub optimistic: i64,
    /// Paisa given up by taking the ADVERSE FILL on both legs instead of the
    /// open, summed over trades.
    ///
    /// # Both legs, and the second one is why this is not called `entry_cost`
    ///
    /// It was, for about an hour, and the test suite refused it: with the entry
    /// bracketed but the exit still priced at a single close,
    /// [`Self::uncertainty`] came back non-zero on cells with no ambiguous bar
    /// at all. A square-off is a market order too, so it has a knowable spread
    /// of its own, and subtracting only the entry's half left the exit's half
    /// sitting in a column that claims to report the UNKNOWABLE.
    ///
    /// So this is the whole knowable spread: entering at the bar's high
    /// rather than its open, and leaving at the low rather than the
    /// open. Non-negative by construction on both legs.
    ///
    /// Stored rather than derived, because deriving it would need the third
    /// reading again at every read.
    pub fill_cost: i64,
    /// Trades whose exit came from the FIXED stop.
    ///
    /// # Three exits used to share this counter, and a reader could not separate
    /// them
    ///
    /// It read *"a trailing exit IS a stop -- it gives back part of a gain to
    /// protect the rest -- so it is counted as one"*, which was defensible while
    /// a cell held one trailing order. It is not defensible now: a cell can
    /// carry a fixed stop, a trailing STOP LOSS and a trailing TAKE PROFIT, and
    /// those are three different things happening to a position.
    ///
    /// A fixed stop is a loss taken at a distance from entry. A trailing stop
    /// loss is a gain protected. A trailing take profit is a winner let run and
    /// then closed. Merging them told the operator only "it did not time out",
    /// which is the one thing they could already see.
    pub stopped: u64,
    /// Trades closed by the trailing STOP LOSS, live from entry.
    pub trailed_stop: u64,
    /// Trades closed by the trailing TAKE PROFIT, armed at a target rung.
    pub trailed_profit: u64,
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
    /// Mean adverse excursion of **every** round trip, winners and losers.
    ///
    /// # The number [`Cell::edge_ratio`] is divided by, and why it is not `winner_mae`
    ///
    /// `winner_mae` answers *"how far did a winner go against me before it
    /// worked"* — the tightest stop that would not have killed it. It is the
    /// right question and it has one fatal property as a **ranking** key: it is
    /// computed only over trades that ended profitable, so it is structurally
    /// blind to how large a loser gets.
    ///
    /// That blindness is not theoretical. A variant with **no stop** lets its
    /// winners run further, which raises the numerator, while its losers — the
    /// ones a stop would have cut — run to the horizon and never enter the
    /// denominator at all. `crates/runner/src/validate.rs` records the
    /// measurement: the old ratio **chose the no-stop variant 61–100% of the
    /// time**, and `synthetic.rs` records two of three ranking keys doing the
    /// same.
    ///
    /// So the metric asked for the tightest stop and selected for having none.
    ///
    /// Dividing by the adverse excursion of **everything the variant took**
    /// closes it: a no-stop variant's deep losers now raise the denominator by
    /// exactly the amount a stop would have removed, so the two halves an
    /// operator actually wants — a small stop AND a large run — are both priced
    /// in one number.
    ///
    /// `winner_mae` is kept and still rendered, because *"how much did a winner
    /// make me sweat"* remains a real diagnostic. It is no longer the thing a
    /// variant is chosen by.
    pub all_mae: Ppm,
    /// The WORST adverse excursion any single trade suffered, in ppm.
    ///
    /// # A mean cannot answer the question an operator actually asks
    ///
    /// [`Self::winner_mae`] and [`Self::all_mae`] are MEANS — `adverse_won / n`
    /// and `adverse_all / trades`. They answer *"how much did a typical trade
    /// make me sweat"*, and that is a useful diagnostic and the wrong question
    /// for sizing a stop.
    ///
    /// The question is *"did ANY trade go more than X against me"*, because a
    /// stop is placed once and every trade must survive it. A mean of 30 ppm
    /// across ten thousand trades is entirely consistent with one trade that
    /// went 4,000 ppm against — and that one trade is the one that takes the
    /// account out. An operator promised "no trade ever went beyond thirty
    /// points" needs the MAXIMUM, and until this field there was no number in
    /// this workspace that could be checked against that promise.
    ///
    /// Taken over every trade, winners included: a winner that dipped hard
    /// before recovering would still have hit a stop placed under it.
    pub worst_mae: Ppm,
    /// Sum of every PROFITABLE round trip, in paisa. `TradingView`'s *gross
    /// profit*.
    ///
    /// Kept separately from [`Self::pessimistic`] because a net total cannot be
    /// decomposed: ₹10,000 net is a different proposition from ₹110,000 gross
    /// against ₹100,000 of losses, and the second is the one that fails the
    /// moment costs move.
    pub gross_win: i64,
    /// Sum of every LOSING round trip, in paisa, as a NEGATIVE number.
    ///
    /// With [`Self::gross_win`] this gives the profit factor —
    /// `gross_win / |gross_loss|` — which is the single figure a strategy report
    /// is usually judged on and which no field here could previously produce.
    pub gross_loss: i64,
    /// The largest single WINNING round trip, in paisa.
    ///
    /// The counterpart of [`Self::worst_trade`]. A total carried by one
    /// enormous winner is not the same strategy as one built from ten thousand
    /// small ones, and only these two together can tell them apart.
    pub best_trade: i64,
    /// The SMALLEST winning round trip, in paisa. Zero when nothing won.
    ///
    /// # The operator's rule is about the extremes, not the averages
    ///
    /// The rule is *"the worst losing trade is 1 and the smallest winning trade
    /// must be 2"* — a 1:2 taken over the WORST case on both sides, not over the
    /// means. An average-based ratio can be satisfied by a distribution where
    /// some winners are smaller than some losers; this one cannot, because it
    /// compares the floor of the wins against the ceiling of the losses.
    ///
    /// It is a far harder test and it is the one that was asked for. Kept
    /// alongside [`Self::best_trade`] so the whole winning range is visible:
    /// a strategy whose winners run from ₹2 to ₹150 is a different instrument
    /// from one whose winners are all ₹40.
    pub min_win: i64,
    /// Total bars held across every round trip, for the average holding time.
    ///
    /// Execution bars, so MINUTES once the one-minute execution layer is in
    /// play. A strategy whose average trade runs four minutes and one whose
    /// average runs four hours are different instruments wearing one name.
    pub bars_held: u64,
    /// The longest run of consecutive LOSING trades.
    ///
    /// What an operator has to sit through. A 60% win rate with a
    /// twenty-two-loss streak inside it is not tradeable by a human, and the
    /// win rate alone cannot show that.
    pub max_losing_streak: u32,
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
    /// Winning trades as a percentage, in hundredths. `6042` reads 60.42%.
    ///
    /// Integer, in hundredths, for the reason `CLAUDE.md` §7 gives everywhere
    /// else: a ratio that is compared must not be a float.
    #[must_use]
    pub const fn win_rate_bp(&self) -> i64 {
        if self.trades == 0 {
            return 0;
        }
        // `cast_signed` and not `as`: a trade count past `i64::MAX` cannot arise
        // from any slice this engine can hold, and a wrapping cast would turn an
        // impossible count into a negative win rate rather than refusing.
        self.wins
            .cast_signed()
            .saturating_mul(10_000)
            .saturating_div(self.trades.cast_signed())
    }

    /// Gross profit over gross loss, in hundredths. `250` reads 2.50.
    ///
    /// **The figure a strategy report is usually judged on.** Above 1.00 the
    /// winners outweigh the losers; below it they do not, whatever the net total
    /// says on a lucky sample.
    ///
    /// Returns [`i64::MAX`] when there were no losing trades at all — a real
    /// answer on a small sample and not a division to make.
    #[must_use]
    pub const fn profit_factor_bp(&self) -> i64 {
        let lost = self.gross_loss.saturating_neg();
        if lost <= 0 {
            return i64::MAX;
        }
        self.gross_win.saturating_mul(100) / lost
    }

    /// Mean profit of the WINNING trades, in paisa.
    #[must_use]
    pub const fn avg_win(&self) -> i64 {
        if self.wins == 0 {
            return 0;
        }
        self.gross_win.saturating_div(self.wins.cast_signed())
    }

    /// Mean loss of the LOSING trades, in paisa. Negative.
    #[must_use]
    pub const fn avg_loss(&self) -> i64 {
        let losers = self.trades.saturating_sub(self.wins);
        if losers == 0 {
            return 0;
        }
        self.gross_loss.saturating_div(losers.cast_signed())
    }

    /// Mean holding time, in execution bars — minutes under the 1-minute layer.
    #[must_use]
    pub const fn avg_bars_held(&self) -> u64 {
        if self.trades == 0 {
            return 0;
        }
        self.bars_held / self.trades
    }

    /// The SMALLEST win over the LARGEST loss, in hundredths. `200` reads 2.00.
    ///
    /// # A worst-case 1:2, not an average one
    ///
    /// The operator's rule is *"our maximised worst losing trade is 1 and the
    /// minimum winning trade should always be 2"*. That compares the FLOOR of
    /// the wins against the CEILING of the losses, and it is far stricter than
    /// the average-based ratio it replaced.
    ///
    /// The difference is not academic. A real sixty-minute run scored **81.19%
    /// profitable** with an average win of ₹7.57 against an average loss of
    /// ₹18.65 — and inside that, a smallest win far under its largest loss of
    /// ₹178.45. An averages ratio can be satisfied by a distribution in which
    /// many winners are smaller than many losers; this one cannot.
    ///
    /// [`i64::MAX`] when nothing lost. Zero when nothing won, which fails every
    /// positive rule and is the honest answer for a variant with no winners.
    #[must_use]
    pub const fn reward_to_risk_bp(&self) -> i64 {
        let worst_loss = self.worst_trade.saturating_neg();
        if worst_loss <= 0 {
            return i64::MAX;
        }
        self.min_win.saturating_mul(100) / worst_loss
    }

    /// What a winner ran, over what **every** trade cost in adverse excursion.
    ///
    /// The precision of the setup as a single number, in hundredths — integers
    /// rather than a float because `CLAUDE.md` §7 keeps this arithmetic in
    /// integers, and a ratio used for ranking is compared far more often than it
    /// is read.
    ///
    /// # The denominator changed, and that is the whole point
    ///
    /// This was `winner_mfe / winner_mae` — both terms over winners only. It
    /// therefore rewarded a variant for **not having a stop**: no stop lets
    /// winners run further, raising the numerator, while the losers a stop would
    /// have cut run to the horizon and never enter a winners-only denominator.
    /// `validate.rs` records the measurement — **the no-stop variant was chosen
    /// 61–100% of the time** — so a key described as *"the tightest stop that
    /// would not have killed them"* was in practice selecting for no stop at
    /// all. That is the opposite of what it is used for.
    ///
    /// [`Cell::all_mae`] is the adverse excursion of everything the variant
    /// took, so a deep loser now costs the variant exactly what a stop would
    /// have saved. Both halves of *"minimal stop, massive target"* are priced in
    /// one number, and neither can be gamed by omitting the other.
    ///
    /// # Zero is a refusal, not an infinity
    ///
    /// Zero when nothing ever went adverse — a sample too clean to rank rather
    /// than an infinite ratio, and saying so beats dividing by zero.
    #[must_use]
    pub const fn edge_ratio(&self) -> i64 {
        if self.all_mae <= 0 {
            return 0;
        }
        self.winner_mfe.saturating_mul(100) / self.all_mae
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
    /// What the WORST entry fill cost, in paisa, summed over every trade.
    ///
    /// Both readings used to enter at the execution bar's OPEN, so a
    /// `pessimistic` figure was a worst-case exit off a best-case entry — the
    /// one leg of the round trip nothing in this module ever charged.
    /// `crate::trade::walk` had carried both readings on both legs since it
    /// gained `costs::fill::Anchor`; this module carried none, and a search for
    /// `Anchor` in it returned nothing.
    ///
    /// Non-negative by construction: the adverse extreme is never a
    /// better fill than the open, on either side.
    ///
    /// Read [`Cell::fill_cost`] directly — every field on this struct is
    /// public and an accessor that only returned one would be the odd one out.
    ///
    /// The gap between the two readings that intra-bar ORDERING is responsible
    /// for — and only that.
    ///
    /// # Why the entry cost is subtracted rather than left in
    ///
    /// This is the `unknown` column, and it has always answered one question:
    /// *how much of this result turns on a path the data cannot settle?* Once
    /// the two readings also bracket the ENTRY fill, `optimistic - pessimistic`
    /// answers a second, different question — the full money bracket — and
    /// letting one number carry both would have redefined every existing report
    /// while its heading stayed the same. The entry cost is knowable; the
    /// ordering is not. Only the unknowable half belongs here.
    ///
    /// The full bracket has not gone anywhere: it is
    /// `optimistic - pessimistic`, which [`Self::bracket`] names.
    ///
    /// **Nothing selects on it, and nothing selects on
    /// [`Self::optimistic`].** [`Grid::best`] and [`Grid::sharpest`] both rank
    /// on the pessimistic figure, and
    /// `runner::grid::no_selector_can_be_moved_by_the_optimistic_figure` holds
    /// that as a property rather than a habit.
    #[must_use]
    pub const fn uncertainty(&self) -> i64 {
        self.optimistic
            .saturating_sub(self.pessimistic)
            .saturating_sub(self.fill_cost)
    }

    /// The whole spread between the best and worst readings of both legs.
    ///
    /// [`Self::fill_cost`] plus [`Self::uncertainty`]: what the adverse fills on
    /// both legs cost, which is knowable, plus what the intra-bar ordering could
    /// cost, which is not.
    #[must_use]
    pub const fn bracket(&self) -> i64 {
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
    /// One per (stop, target, trail, arm) setting, including the row with none
    /// of them.
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
    /// Candidate entries dropped because their path contained a bar the engine
    /// refused.
    ///
    /// # Degraded LOUDLY, which is the whole difference
    ///
    /// `crate::outcome::priced` and `crate::trade::round_trip` refuse a
    /// mis-assembled record at the entry and exit bars. Every bar STRICTLY
    /// BETWEEN them still reached [`crate::excursion::crossings`] unchecked, and
    /// those are the bars every level exit is decided on.
    ///
    /// Measured, one refused record among 2,250 bars: the chosen cell moved from
    /// `target(0)` at 3,210 paisa to `trail(0) + arm(1)` at **499,089** — a
    /// factor of 155, and **a different exit instrument recommended**.
    ///
    /// The whole path is dropped rather than the one bar, because a hole leaves
    /// every running maximum after it computed on incomplete data. Dropping
    /// costs sample size, which is visible; skipping would move the answer,
    /// which is not. `CLAUDE.md` §4: degrade loudly and name the reason.
    ///
    /// **Zero on every sound slice**, so a non-zero here is the signal that the
    /// store handed this run something it should not have.
    pub refused_paths: u64,
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
            .find(|c| c.stop.is_none() && c.target.is_none() && c.tsl.is_none() && c.ttp.is_none())
    }

    /// The variant with the largest pessimistic total.
    ///
    /// Pessimistic and not optimistic, for the reason [`crate::validate`] gives
    /// about selection: choosing on the flattering number picks whatever the
    /// flattering assumption helped most.
    #[must_use]
    pub fn best(&self) -> Option<&Cell> {
        self.cells.iter().max_by_key(|c| (c.pessimistic, merit(c)))
    }

    /// The tightest containment any variant of this combination achieved.
    ///
    /// # The answer to "what stop is even possible here"
    ///
    /// A screen that only says PASS or FAIL leaves the operator guessing the
    /// threshold: try twenty points, everything fails; try fifty, everything
    /// fails; try two hundred, everything passes and the rule stopped meaning
    /// anything. That is a search the TOOL should do, because it already priced
    /// all 625 variants and knows the answer.
    ///
    /// This is `min(worst_mae)` across every variant that took a trade — the
    /// smallest maximum adverse excursion the grid could achieve on this
    /// combination, whatever it cost in profit. A rule tighter than this figure
    /// cannot be met by ANY exit, so an operator reading it knows immediately
    /// whether their number is reachable or whether the combination has to go.
    ///
    /// `None` when no variant took a trade.
    #[must_use]
    pub fn tightest_containment(&self) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| c.trades > 0)
            .min_by_key(|c| (c.worst_mae, core::cmp::Reverse(c.pessimistic)))
    }

    /// The best variant that satisfies every rule, not the best variant overall.
    ///
    /// # THE GRID EXISTS TO FIND THIS, and asking `best()` throws that away
    ///
    /// `Grid::best` returns the variant with the largest total, and the screen
    /// then judged THAT one against the operator's stop. On a real run every top
    /// combination failed, because the profit-maximising cell is the one with
    /// **no stop at all** — `-/-/-+0@0` — and a variant with no stop obviously
    /// lets a trade run 1.06% against.
    ///
    /// That is the wrong question. There are 625 variants and 24 of them place a
    /// fixed stop; the operator's rule is a constraint on WHICH VARIANT to
    /// trade, not a verdict on the combination. A combination whose best-overall
    /// cell breaks the rule may have a stopped cell that keeps it, makes less,
    /// and is the one they would actually take.
    ///
    /// So the search is: among the cells that satisfy every rule, the one with
    /// the largest pessimistic total; `None` when no variant of this combination
    /// can be traded within the rules, which is a real finding about the
    /// combination rather than about the grid.
    ///
    /// Ties break exactly as [`Self::best`] does — `(pessimistic, merit)` with
    /// `max_by_key`'s last-maximum rule — so a constrained search and an
    /// unconstrained one agree wherever the constraint does not bind.
    #[must_use]
    pub fn best_within(&self, admits: impl Fn(&Cell) -> bool) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| c.trades > 0 && admits(c))
            .max_by_key(|c| (c.pessimistic, merit(c)))
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
            .max_by_key(|c| (c.edge_ratio(), c.pessimistic, merit(c)))
    }
}

/// The tie-break, and it is not cosmetic.
///
/// # `max_by_key` returns the LAST maximum, so a tie was settled by position
///
/// Both selectors were a bare `max_by_key` on one key. Two consequences, both
/// measured by an adversarial fleet on real grids:
///
/// 1. **A tie discarded money.** Two cells with the same `edge_ratio` and
///    19,670 against 630 paisa: `sharpest` took the 630, because it happened to
///    sit later in `cells`. `crate::validate` then ranks every candidate mask on
///    exactly that cell's `pessimistic`, and `crate::pbo` consumes the result —
///    so a vector-order accident propagated into the overfitting probability.
///
/// 2. **A tie named a mechanism that never fired.** A trail or an arm whose rung
///    the path never reached leaves its cell byte-identical to the plain cell it
///    decorates. They tie, the decorated one is emitted later, and it wins — so
///    the audit printed a trailing take profit for a run in which nothing
///    trailed. Measured: 64 of 240 grids on `sessions(12)`.
///
/// That second one is the same defect the arming axis was added to remove,
/// recreated one level up, and it is why the tie-break runs toward FEWER claimed
/// rungs. A cell that reaches the same number with less machinery reached it
/// with less machinery, and saying otherwise is a claim the data does not carry.
///
/// # The order of the two terms
///
/// Money first. A cell that made more is better on the operator's own question,
/// and simplicity only decides between cells that are otherwise identical —
/// which is exactly the case both defects arose from.
///
/// Returns a key that SORTS ASCENDING with merit, so it composes with
/// `max_by_key` directly: fewer rungs is a larger number.
pub(crate) const fn merit(c: &Cell) -> i64 {
    // A TTP counts as TWO claims -- an arming rung and a trailing rung -- because
    // it is two decisions and a cell that reached the same number without making
    // either of them made it more simply.
    let claimed = c.stop.is_some() as i64
        + c.target.is_some() as i64
        + c.tsl.is_some() as i64
        + if c.ttp.is_some() { 2 } else { 0 };
    // Five minus the count, so the simplest cell scores highest and the fully
    // decorated one scores zero.
    5 - claimed
}

/// Which of the two trailing orders closed a position.
///
/// They hang from the same peak and fill by the same arithmetic, so nothing
/// about the PRICE distinguishes them. What distinguishes them is what they are
/// for: one cuts a position that turns, the other lets a winner run and then
/// closes it. An operator reading "stopped: 40" needs to know which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TrailKind {
    /// The trailing STOP LOSS, live from entry.
    Live,
    /// The trailing TAKE PROFIT, armed at a target rung.
    Armed,
}

/// How a trade under a given variant ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ended {
    Stop,
    Target,
    /// A trailing exit, which fills at `peak - distance` rather than at a fixed
    /// distance from entry.
    ///
    /// Carries BOTH the peak it hung from and the distance it trailed by, and
    /// the second half is new. It used to carry only the anchor, and
    /// `level_for` looked the distance up from a single `trail_ppm` — which was
    /// unambiguous while a cell could hold one trailing order. With a TSL and a
    /// TTP on the same cell there are two possible distances, and the one that
    /// priced the fill is a property of WHICH order fired. Carrying it here
    /// makes that unmistakable rather than a lookup that could pick the other.
    Trail {
        /// The peak the resting order hung from.
        anchor: i64,
        /// The give-back distance, in parts per million of the entry price.
        ppm: Ppm,
        /// WHICH trailing order it was.
        ///
        /// A cell can carry a trailing stop loss and a trailing take profit at
        /// once, and they are different instruments doing opposite jobs. Without
        /// this the exit counter could only say "not a timeout", merging a
        /// protected gain with a run let out -- see [`Cell::stopped`].
        kind: TrailKind,
    },
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
///
/// # The fourth axis is not a fourth factor, and TWO kinds of cell are refused
///
/// [`Cell::arm`] indexes the TARGET ladder, so a naive fourth factor would read
/// `(S+1)(T+1)(R+1)(T+1)` = 625 at four rungs. Two families inside that 625
/// cannot differ from a cell that already exists, and emitting them would make
/// the grid report one answer many times.
///
/// **An arm with no trail to arm is inert.** Those 100 cells duplicate the 25
/// trail-less ones.
///
/// **An arm at or above the target is unreachable, and this one shipped.** The
/// first version emitted `a in 0..=targets.len()` for every target setting. When
/// `arm = Some(a)` and `target = Some(t)` with `a >= t`, the target rung is
/// reached no later than the arming rung, and an armed trail fires strictly
/// after the arming bar — so `target_at(t) <= target_at(a) < armed_at(a, r)` on
/// every candidate and the trail can never fire. Those cells equal the
/// trail-less cell they sit beside, in every measured field.
///
/// It is worse than wasted width. [`Grid::best`] is `max_by_key`, which returns
/// the **last** maximum, so a tie is won by the highest arm index — and the
/// audit would print `exit = 1/0/0@1`, *a trailing take profit*, for a trade set
/// in which nothing ever armed. A label claiming a mechanism that did not run is
/// precisely the defect the arming axis was added to remove, and it was
/// recreated one level up. Measured at four rungs: **200 inert cells, 120 of
/// them exact duplicates of each other.**
///
/// So an arm is emitted only for a rung STRICTLY BELOW the target. With no
/// target every rung qualifies:
///
/// ```text
/// arms(target = None)    = T     every rung can arm
/// arms(target = Some(j)) = j     only rungs the target does not pre-empt
/// sum over all targets   = T(T+1)/2  =  10 at four rungs
/// ```
///
/// # THE FIFTH AXIS, AND A THIRD REFUSAL
///
/// [`Cell::tsl`] and [`Cell::ttp`] were one field, so a cell held a trailing
/// stop loss or a trailing take profit and never both — leaving 4 of the 16
/// on/off patterns of `(SL, TP, TSL, TTP)` unreachable. They are the ones an
/// operator asks for by name: *loose trail from entry, tighter trail once it
/// pays.*
///
/// Splitting them naively would give `(S+1)(T+1)(R+1)(1 + T·R)` = **1,125**.
/// The third refusal cuts it to 625: once armed, both trails hang from the same
/// running peak, so **only the smaller distance can ever fire**. A TTP whose
/// trailing rung is at or above the TSL's is inert, and its cell is
/// byte-identical to the TSL-only cell beside it — the same degeneracy this
/// block already records refusing twice.
///
/// Ladder rungs ascend, so the constraint is `ttp.trail < tsl` on the index:
///
/// ```text
/// per (stop, target):
///   armed rows available to a given TSL setting:
///     tsl = None     ->  R    every trailing rung is available
///     tsl = Some(i)  ->  i    only rungs strictly tighter than it
///   summed over the R+1 TSL settings        =  R(R+1)/2  =  10 at four rungs
///
///   cells = (R+1)              the TTP-less rows, one per TSL setting
///         + arms(t) * R(R+1)/2 the armed rows
///
/// (S+1) * [ (T+1)(R+1) + T(T+1)/2 * R(R+1)/2 ]
///   =  5 * [ 25 + 10*10 ]  =  5 * 125  =  625
/// ```
#[must_use]
pub const fn variants(stops: usize, targets: usize, trails: usize) -> usize {
    let target_settings = targets.saturating_add(1);
    let tsl_settings = trails.saturating_add(1);
    // Both triangular sums. Integer division is exact: of `n` and `n+1` one is
    // always even.
    let arms_over_all_targets = targets.saturating_mul(target_settings) / 2;
    let armed_rungs_over_all_tsl = trails.saturating_mul(tsl_settings) / 2;
    let per_stop = target_settings
        .saturating_mul(tsl_settings)
        .saturating_add(arms_over_all_targets.saturating_mul(armed_rungs_over_all_tsl));
    stops.saturating_add(1).saturating_mul(per_stop)
}

struct Candidate {
    signal: usize,
    entry: usize,
    /// The last bar the position may be held to: horizon or square-off.
    time_exit: usize,
    cross: Crossings,
    /// The entry fill under the WORST reading: the execution bar's adverse
    /// extreme it PRINTED: a buy at the high for a long and a sell at
    /// the low for a short.
    ///
    /// # Why this is a field and not a recomputation
    ///
    /// It was neither. Every one of the four sites that needed an entry price
    /// wrote `bars.get(i).map_or(0, |b| b.open)` — the OPEN, on both readings —
    /// so `Cell::pessimistic` was a worst-case EXIT priced off a best-case
    /// ENTRY, and no combination in this crate was ever charged for entering
    /// badly. `crate::trade::walk` had both readings on both legs since it
    /// gained `costs::fill::Anchor`; this module never did, and `grep -c Anchor
    /// crates/runner/src/grid.rs` returned 0.
    ///
    /// A field rather than a helper call at each site because the same price
    /// must reach `realised` and `peak_adverse`: computing it twice is how the
    /// two readings drifted apart in the first place.
    entry_pess: i64,
    /// The entry fill under the BEST reading: the execution bar's open, a price
    /// that printed.
    entry_opt: i64,
}

/// Both entry fills for the execution bar at `index`, worst first.
///
/// # `PrintedExtreme` and NOT `AdverseExtreme`, which is the whole distinction
///
/// `AdverseExtreme` buys at the high plus one [`costs::rate::TICK`] and sells at
/// the low minus one. That tick is a modelled slippage cost, and on a series
/// whose finest resolution is one minute it is also a claim about a price at
/// which nothing traded. The bar's four numbers are the entire record of that
/// minute; a fill placed outside them is invented, which `CLAUDE.md` §3 rule 1
/// forbids and which `costs::fill::Anchor` already objects to in its own words
/// about the adverse-extreme buy not being a price that printed.
///
/// So the worst fill here is the extreme ITSELF. Nothing is added, no spread is
/// assumed, and no percentage is applied.
///
/// It still goes through `costs::fill` rather than reading `bar.high` directly,
/// because the bracket and sub-tick checks in `costs::fill::Bar::new` are the
/// same ones `crate::trade::walk` gets, and re-deriving them here is the
/// duplication that let the entry price diverge from that module in the first
/// place.
///
/// # Refusal
///
/// A missing bar, a bar `costs::fill::Bar::new` refuses, or a fill computation
/// that errors all yield `0` — the sentinel this module already treats as *no
/// usable entry*: [`peak`] returns early on `entry <= 0` and so does
/// [`crate::excursion::crossings`]. Zero is not a price and cannot be mistaken
/// for one.
fn entry_fills(bars: &[Candle], index: usize, side: Side) -> (i64, i64) {
    let Some(bar) = bars.get(index) else {
        return (0, 0);
    };
    let open = bar.open;
    // The bracket check `costs::fill::Bar::new` runs is a real invariant, so a
    // candle whose open sits outside its own high-low is refused HERE rather
    // than priced off extremes that never contained it — the same reasoning
    // `crate::trade::price_one` gives at its own `FillBar::new`.
    let raw = brutex_core::price::Paisa::from_raw;
    let Ok(fill_bar) = costs::fill::Bar::new(raw(open), raw(bar.high), raw(bar.low)) else {
        return (0, open);
    };
    let Ok(fills) = costs::fill::fills_at(
        fill_bar,
        fill_bar,
        direction_of(side),
        costs::fill::Anchor::PrintedExtreme,
    ) else {
        return (0, open);
    };
    let worst = match side {
        Side::Long => fills.buy().raw(),
        Side::Short => fills.sell().raw(),
    };
    (worst, open)
}

/// The fill for a market EXIT on the bar at `index`, under one reading.
///
/// # The mirror of [`entry_fills`], and the sides swap
///
/// Getting out of a long is a SELL, so its adverse extreme is the bar's low
/// — where [`entry_fills`] takes the buy leg, this takes the sell.
/// A reading that took the same leg for both would charge a long twice for
/// buying and never for selling.
///
/// `None` when the bar is missing or `costs::fill::Bar::new` refuses it; the
/// caller falls back to the entry price, which books the trade flat rather than
/// inventing an exit.
fn exit_fill(bars: &[Candle], index: usize, side: Side, pessimistic: bool) -> Option<i64> {
    let bar = bars.get(index)?;
    if !pessimistic {
        // The open is a price that PRINTED, and it is the best a market order on
        // this bar could have done. Not the close: the close is neither extreme
        // nor a bound, which is exactly why pricing both readings at it hid the
        // spread entirely.
        return Some(bar.open);
    }
    let raw = brutex_core::price::Paisa::from_raw;
    let fill_bar = costs::fill::Bar::new(raw(bar.open), raw(bar.high), raw(bar.low)).ok()?;
    let fills = costs::fill::fills_at(
        fill_bar,
        fill_bar,
        direction_of(side),
        costs::fill::Anchor::PrintedExtreme,
    )
    .ok()?;
    Some(match side {
        Side::Long => fills.sell().raw(),
        Side::Short => fills.buy().raw(),
    })
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
/// **This block has stated the variant count wrongly twice, which is why it no
/// longer states it at all.** It claimed `(rungs + 1)^2` while the loop was
/// three deep, then `(stops+1)(targets+1)(trails+1)` = 125 while the loop became
/// four deep and two families of cell were refused. The number is
/// [`variants`]'s, and this sentence is a pointer rather than a third copy.
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
    //
    // A PATH WITH A REFUSED BAR ANYWHERE IN IT IS DROPPED, NOT PRICED. See
    // `Grid::refused_paths` for what one such bar did to a real audit. The drop
    // is counted so a reader sees a smaller sample rather than a moved answer.
    // FROM `eligible`, NOT `trades`, AND THAT IS THE WHOLE OF THE RE-WALK.
    //
    // `trades` is `crate::trade::walk`'s answer under rule 4 with LEVEL-LESS
    // exits -- the longest holds, so the most exclusion. Building candidates
    // from it left `one_variant`'s own exclusivity nothing to do: a tighter stop
    // frees the next signal only if that signal is in the list, and rule 4 had
    // already removed it. Measured by an adversarial fleet: the guard fired ZERO
    // times across 168,892 cells, while this module's header claims the sequence
    // differs per variant.
    //
    // `eligible` is every signal that could open a position, exclusivity NOT
    // applied. `one_variant` is now the only place it is applied, which is what
    // the header always said.
    let all: Vec<Candidate> = timed
        .eligible
        .iter()
        .map(|t| {
            // `entry_price` stays the OPEN and keeps its name, because the
            // `crossings` call below places the rung ladders on the excursion
            // distribution measured from it. Shifting that to the worst entry
            // would move every rung and so re-price every result already banked,
            // for a search grid that is a choice of levels to TRY rather than a
            // reported figure. What the worst entry must move is the money and
            // the MAE, and those read `entry_pess` in `one_variant`.
            let (entry_pess, entry_price) = entry_fills(bars, t.entry_bar, side);
            Candidate {
                signal: t.signal_bar,
                entry: t.entry_bar,
                time_exit: t.exit_bar,
                entry_pess,
                entry_opt: entry_price,
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
    let refused_paths =
        u64::try_from(all.iter().filter(|c| c.cross.refused() > 0).count()).unwrap_or(u64::MAX);
    let candidates: Vec<Candidate> = all.into_iter().filter(|c| c.cross.refused() == 0).collect();

    // PASS FOUR: every variant, each a sequence walk with O(1) exits.
    // THE LOOP IS FOUR DEEP AND THE RESERVATION IS NOT ITS ARITHMETIC REPEATED.
    // It once was: the trails factor was missing, so a grid reserved 5x5 = 25
    // and then pushed 125, reallocating its way to the right size every time.
    // `variants` is the one place the count is computed, and the test beside it
    // asserts the loop agrees with it rather than trusting that it does.
    let mut cells: Vec<Cell> =
        Vec::with_capacity(variants(stops.len(), targets.len(), trails.len()));
    let rungs_of = (stops.rungs(), targets.rungs(), trails.rungs());
    for s in 0..=stops.len() {
        for t in 0..=targets.len() {
            let stop = (s < stops.len()).then_some(s);
            let target = (t < targets.len()).then_some(t);
            // `i == trails.len()` is NO trailing stop loss. Every lower value is
            // a TSL at that rung, live from entry.
            for i in 0..=trails.len() {
                let tsl = (i < trails.len()).then_some(i);
                // THE THIRD REFUSAL, AND IT IS WHAT KEEPS THIS AT 625.
                //
                // Once armed, a TTP and a TSL hang from the SAME running peak,
                // so only the smaller distance can ever fire. `cap` is the
                // exclusive bound on the armed trailing rung: strictly tighter
                // than the live one, or unbounded when there is no live one.
                // Rungs ascend, so a lower index is a tighter trail.
                //
                // Without it every `ttp.trail >= tsl` cell is byte-identical to
                // the TSL-only cell beside it -- 500 inert cells at four rungs,
                // and the same degeneracy `variants` records refusing twice
                // already.
                let cap = tsl.unwrap_or(trails.len());
                // The TTP-LESS ROW: this TSL setting on its own, including the
                // all-none baseline when `i` and `t` and `s` are all their
                // no-level values.
                cells.push(one_variant(
                    bars,
                    &candidates,
                    rungs_of,
                    Variant {
                        stop,
                        target,
                        tsl,
                        ttp: None,
                    },
                    side,
                ));
                // `0..t` AND NOT `0..targets.len()`, WHICH IS THE SECOND
                // REFUSAL. `t` is `targets.len()` on the no-target row, so every
                // rung can arm there. On a row that HAS a target, only rungs
                // strictly below it arm before the target closes the position.
                for arm in 0..t {
                    for trail in 0..cap {
                        cells.push(one_variant(
                            bars,
                            &candidates,
                            rungs_of,
                            Variant {
                                stop,
                                target,
                                tsl,
                                ttp: Some(Ttp { arm, trail }),
                            },
                            side,
                        ));
                    }
                }
            }
        }
    }

    Grid {
        cells,
        signals: timed.signals,
        stops,
        targets,
        trails,
        refused_paths,
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
    /// The trailing STOP LOSS rung, live from entry.
    tsl: Option<usize>,
    /// The trailing TAKE PROFIT. When both are set, `ttp.trail < tsl` -- see
    /// [`Cell::ttp`] for why an equal-or-looser armed trail is inert.
    ttp: Option<Ttp>,
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
    variant: Chosen,
) -> Option<Cell> {
    let timed = crate::trade::walk(bars, column, mask, horizon, direction_of(side));
    if timed.eligible.is_empty() {
        return None;
    }
    // `eligible` and not `trades`, for the reason `evaluate` gives: exclusivity
    // belongs to the variant being scored, not to the level-less walk that found
    // the signals.
    let candidates: Vec<Candidate> = timed
        .eligible
        .iter()
        .map(|t| {
            // `entry_price` stays the OPEN and keeps its name, because the
            // `crossings` call below places the rung ladders on the excursion
            // distribution measured from it. Shifting that to the worst entry
            // would move every rung and so re-price every result already banked,
            // for a search grid that is a choice of levels to TRY rather than a
            // reported figure. What the worst entry must move is the money and
            // the MAE, and those read `entry_pess` in `one_variant`.
            let (entry_pess, entry_price) = entry_fills(bars, t.entry_bar, side);
            Candidate {
                signal: t.signal_bar,
                entry: t.entry_bar,
                time_exit: t.exit_bar,
                entry_pess,
                entry_opt: entry_price,
                cross: crossings(bars, t.entry_bar, t.exit_bar, entry_price, side, ladders),
            }
        })
        .collect();

    let Chosen {
        stop,
        target,
        tsl,
        ttp,
    } = variant;
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
            tsl,
            ttp,
        },
        side,
    ))
}

/// One exit setting, named rather than positional.
///
/// [`with_levels`] took `(Option<usize>, Option<usize>, Option<usize>)` and the
/// fourth axis would have made it a four-tuple of one type — four `Option`
/// rungs a caller can permute silently, where three were already two too many.
/// `crate::validate` builds one of these per fold and hands it across a window
/// boundary, so a swapped pair would score the right combination under the
/// wrong exit and report it as a walk-forward result.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Chosen {
    /// Rung on the stop ladder, or no stop.
    pub stop: Option<usize>,
    /// Rung on the target ladder, or no target.
    pub target: Option<usize>,
    /// Rung on the trailing ladder for the TRAILING STOP LOSS, live from entry.
    pub tsl: Option<usize>,
    /// The TRAILING TAKE PROFIT: the target rung that arms it and the trailing
    /// rung it then follows the peak by. See [`Cell::ttp`].
    pub ttp: Option<Ttp>,
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

/// The three prices one variant puts on one candidate's round trip.
///
/// # Why three and not two
///
/// Two of them are the answer: the worst reading of BOTH legs and the best
/// reading of BOTH legs. The third exists so those two can be told apart for
/// the right reason — it is the pessimistic EXIT priced at the optimistic
/// ENTRY, so subtracting it from the pessimistic total isolates what the entry
/// fill cost and leaves [`Cell::uncertainty`] meaning intra-bar ordering alone,
/// which is what its column has always claimed to mean.
struct Readings {
    /// Worst fills, worst ordering. What [`Grid::best`] and [`Grid::sharpest`]
    /// rank, and the only one of the three a reader is shown as a total.
    pess: i64,
    /// Best fills, best ordering.
    opt: i64,
    /// The pessimistic ORDERING at the optimistic FILLS — the same exit
    /// attribution as `pess`, priced at the open on both legs.
    ///
    /// Never reported; only differenced. `pess_best_fills - pess` is the
    /// knowable fill spread and `opt - pess_best_fills` is the unknowable
    /// ordering, which is the split [`Cell::uncertainty`] exists to preserve.
    pess_best_fills: i64,
}

/// Price one candidate's trip under all three readings.
///
/// Each tuple is one reading's `(offset, attribution, level)` — bundled because
/// the two offsets and the two levels are permutable without a compile error,
/// and swapping them silently prices the optimistic exit as the pessimistic one.
fn read_trip(
    bars: &[Candle],
    c: &Candidate,
    side: Side,
    pess: (usize, Ended, Option<Ppm>),
    opt: (usize, Ended, Option<Ppm>),
) -> Readings {
    let (pess_off, pess_by, pess_level) = pess;
    let (opt_off, opt_by, opt_level) = opt;
    Readings {
        // The trailing `true`/`false` is the READING, and the three lines are
        // the whole bracket: worst exit at worst entry, best at best, and worst
        // exit at the BEST entry so the two causes stay separable.
        pess: realised(
            bars,
            c.entry,
            pess_off,
            c.entry_pess,
            side,
            pess_by,
            pess_level,
            true,
        ),
        opt: realised(
            bars,
            c.entry,
            opt_off,
            c.entry_opt,
            side,
            opt_by,
            opt_level,
            false,
        ),
        pess_best_fills: realised(
            bars,
            c.entry,
            pess_off,
            c.entry_opt,
            side,
            pess_by,
            pess_level,
            false,
        ),
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
        tsl,
        ttp,
    } = v;
    let mut cell = Cell {
        stop,
        target,
        tsl,
        ttp,
        ..Cell::default()
    };
    let mut open_until: Option<usize> = None;
    // Reset to zero by every winner, so it measures a RUN and not a total.
    let mut losing_streak: u32 = 0;
    let mut adverse_on_winners: i64 = 0;
    let mut adverse_on_all: i64 = 0;
    let mut gain_on_winners: i64 = 0;

    for c in candidates {
        if open_until.is_some_and(|until| c.signal < until) {
            continue;
        }
        let span = c.time_exit.saturating_sub(c.entry);
        let stop_at = stop.map_or(NEVER, |r| c.cross.stop_at(r));
        let target_at = target.map_or(NEVER, |r| c.cross.target_at(r));
        // TWO TRAILING ORDERS NOW, NOT ONE, AND EACH READS ITS OWN TABLE.
        //
        // WHICH TABLE IS READ IS THE WHOLE OF THE ARMING. The TSL reads the
        // since-ENTRY give-back; the TTP reads the since-ARMING give-back, which
        // `crate::excursion` accumulates as a separate running maximum because
        // it is one. Everything downstream is identical, which is why a trailing
        // take profit needed a table and not a mechanism.
        //
        // Each carries its own anchor pair and its own distance, because the two
        // fill at different prices: `peak - tsl_ppm` and `peak - ttp_ppm` off
        // peaks recorded on different bars. Bundling them into one `Trailing`
        // keeps the pairing unbreakable -- an anchor from one order and a
        // distance from the other would price a fill nobody placed.
        let live = Trailing::live(&c.cross, tsl, trails_rungs);
        let armed = Trailing::armed(&c.cross, ttp, trails_rungs);

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
        //
        // FOUR ORDERS COMPETE NOW. Whichever fires first ends the position, and
        // an order that never fires is `NEVER`, so it drops out of the `min`
        // without a branch.
        let pess_off = span.min(stop_at).min(target_at).min(live.at).min(armed.at);
        let opt_off = pess_off;
        // PESSIMISTIC resolves an ambiguous bar as the STOP, so the stop is
        // tested first. OPTIMISTIC resolves it as the TARGET, so the target is.
        // That ordering is the entire difference between the two readings.
        let firing = Firing {
            stop_at,
            target_at,
            live,
            armed,
        };
        let pess_by = ended_by(firing, pess_off, true);
        let opt_by = ended_by(firing, opt_off, false);
        let ended = pess_by;

        // NO `entry_price` HERE ANY MORE, AND ITS ABSENCE IS THE FIX.
        //
        // This read the execution bar's OPEN and handed the SAME price to both
        // readings, so `pessimistic` was a worst-case exit off a best-case
        // entry: the one leg of the round trip that was never charged. The
        // candidate now carries both fills and each reading takes its own.
        // The two FIXED rung values. The trailing distances are not here: each
        // trailing order carries its own, in `Trailing::ppm`, because a cell can
        // now hold two of them and looking one up by kind would be able to pick
        // the other's.
        let (stop_ppm, target_ppm) = (
            stop.and_then(|r| stops_rungs.get(r).copied()),
            target.and_then(|r| targets_rungs.get(r).copied()),
        );
        let Readings {
            pess,
            opt,
            pess_best_fills,
        } = read_trip(
            bars,
            c,
            side,
            (pess_off, pess_by, level_for(pess_by, stop_ppm, target_ppm)),
            (opt_off, opt_by, level_for(opt_by, stop_ppm, target_ppm)),
        );
        // PESSIMISTIC IS THE SMALLER FIGURE, BY CONSTRUCTION AND NOT BY HABIT.
        //
        // `ended_by` chooses which exit each reading attributes the bar to, and
        // in every ordinary case its pessimistic branch is the worse fill. There
        // is one corner where it is not: a trail whose distance exceeds the
        // stop's plus the whole favourable run fills BELOW the stop, so the
        // branch labelled pessimistic would have produced the better number.
        //
        // Ordering them here settles it without teaching `ended_by` about
        // prices, and it is a property worth holding outright rather than
        // arguing for case by case: the two readings are the two ends of a
        // measurement error, so the one called pessimistic must be the low end.
        // `the_pessimistic_reading_never_beats_the_optimistic_one` asserts it;
        // until now that assertion was satisfied by the two being EQUAL on the
        // fixture, which is not the same as being ordered.
        let (pess, opt) = (pess.min(opt), pess.max(opt));

        tally_trade(&mut cell, pess, pess_off, &mut losing_streak);

        cell.trades = cell.trades.saturating_add(1);
        cell.pessimistic = cell.pessimistic.saturating_add(pess);
        cell.optimistic = cell.optimistic.saturating_add(opt);
        // The SAME exit, priced at the two entries. Their difference is the
        // entry's own cost with the ordering held fixed, which is what lets
        // `Cell::uncertainty` keep meaning ordering alone.
        cell.fill_cost = cell
            .fill_cost
            .saturating_add(pess_best_fills.saturating_sub(pess));
        accrue_risk(&mut cell, pess, (&mut running, &mut peak_equity));
        cell.ambiguous_bars = cell
            .ambiguous_bars
            .saturating_add(unorderable_bars(&c.cross, tsl, ttp));
        count_exit(&mut cell, ended);
        // EVERY TRADE'S ADVERSE EXCURSION, WINNER OR NOT.
        //
        // Outside the `pess > 0` gate on purpose. That gate is what made the old
        // ranking key blind to losers, so a variant with no stop was rewarded
        // for the very trades a stop exists to cut.
        let exit = c.entry.saturating_add(pess_off);
        // `entry_pess` AND NOT THE OPEN, BECAUSE THIS IS THE FIGURE THE RULE IS
        // JUDGED ON. `worst_mae` is what answers "did any single trade ever run
        // more than N points against me", and a trade entered at the adverse
        // extreme runs further against than the same trade entered at the open.
        // Measuring it from the best entry understated every stop requirement.
        let went_against = peak_adverse(bars, c.entry, exit, c.entry_pess, side);
        adverse_on_all = adverse_on_all.saturating_add(went_against);
        // THE MAXIMUM, NOT THE SUM. A stop is placed once and every trade must
        // survive it, so the figure that decides whether a stop is survivable is
        // the worst single excursion and not the average of ten thousand.
        if went_against > cell.worst_mae {
            cell.worst_mae = went_against;
        }

        if pess > 0 {
            cell.wins = cell.wins.saturating_add(1);
            // The same excursion, kept separately: `winner_mae` remains the
            // "how much did a WINNER make me sweat" diagnostic and is still
            // rendered. It is simply no longer what a variant is chosen by.
            adverse_on_winners = adverse_on_winners.saturating_add(went_against);
            // `entry_opt`, the mirror of the line above: the FAVOURABLE peak is
            // the optimistic quantity, so it is measured from the optimistic
            // entry. Pairing it with `entry_pess` would flatter the ratio at
            // both ends at once.
            gain_on_winners = gain_on_winners.saturating_add(peak_favourable(
                bars,
                c.entry,
                exit,
                c.entry_opt,
                side,
            ));
        }
        open_until = Some(c.entry.saturating_add(pess_off));
    }

    mean_excursions(
        &mut cell,
        adverse_on_winners,
        gain_on_winners,
        adverse_on_all,
    );
    cell
}

/// Charge one round trip to the counter for the exit that closed it.
///
/// # FIVE COUNTERS, NOT THREE
///
/// This was a `match` inside `one_variant` with the arm
/// `Ended::Stop | Ended::Trail { .. } => cell.stopped`, justified as *"a
/// trailing exit IS a stop"*. That was defensible while a cell held ONE trailing
/// order. A cell can now carry a fixed stop, a trailing STOP LOSS and a trailing
/// TAKE PROFIT, and merging all three told the operator only "it did not time
/// out" -- the one thing the table already showed.
///
/// A fixed stop is a loss taken at a distance from entry. A trailing stop loss
/// is a gain protected. A trailing take profit is a winner let run and then
/// closed. Three different things happening to a position, and an operator
/// choosing between exits needs to know which one happened.
///
/// A free function rather than an inline `match` so the five arms are in one
/// place and `every_trade_ends_by_exactly_one_of_the_five_exits` has something
/// to point at.
const fn count_exit(cell: &mut Cell, ended: Ended) {
    match ended {
        Ended::Stop => cell.stopped = cell.stopped.saturating_add(1),
        Ended::Trail {
            kind: TrailKind::Live,
            ..
        } => cell.trailed_stop = cell.trailed_stop.saturating_add(1),
        Ended::Trail {
            kind: TrailKind::Armed,
            ..
        } => cell.trailed_profit = cell.trailed_profit.saturating_add(1),
        Ended::Target => cell.targeted = cell.targeted.saturating_add(1),
        Ended::Time => cell.timed_out = cell.timed_out.saturating_add(1),
    }
}

/// Folds one round trip's result into the strategy-report counters.
///
/// # Why these are separate from the two totals
///
/// `pessimistic` and `optimistic` answer *how much did it make*. Every field
/// here answers a question a net total cannot: ₹10,000 net is a different
/// proposition from ₹110,000 gross against ₹100,000 of losses, and the second
/// fails the moment costs move. A total carried by one enormous winner is not
/// the same strategy as one built from ten thousand small ones. A 60% win rate
/// with a twenty-two-loss streak inside it is not tradeable by a human.
///
/// All of it is accumulated on the SAME reading the total is — `pess`, the
/// worst-case fill. Mixing a gross profit taken at the best case with a net
/// total taken at the worst would produce a profit factor no single set of fills
/// ever produced.
///
/// `streak` is carried by the caller because it measures a RUN: it is reset to
/// zero by every winner, which a per-trade function cannot do for itself.
const fn tally_trade(cell: &mut Cell, pess: i64, held: usize, streak: &mut u32) {
    if pess > 0 {
        cell.gross_win = cell.gross_win.saturating_add(pess);
        if pess > cell.best_trade {
            cell.best_trade = pess;
        }
        // THE FLOOR OF THE WINS. `min_win` starts at zero, which is below every
        // winner, so the first winner must set it unconditionally rather than
        // by comparison.
        if cell.min_win == 0 || pess < cell.min_win {
            cell.min_win = pess;
        }
        *streak = 0;
    } else {
        cell.gross_loss = cell.gross_loss.saturating_add(pess);
        *streak = streak.saturating_add(1);
        if *streak > cell.max_losing_streak {
            cell.max_losing_streak = *streak;
        }
    }
    // Holding time in EXECUTION bars, so minutes once the one-minute layer is in
    // play. A strategy whose average trade runs four minutes and one whose
    // average runs four hours are different instruments wearing one name.
    cell.bars_held = cell.bars_held.saturating_add(held as u64);
}

/// Fold one round trip's result into the cell's two risk figures.
///
/// # RISK, WHICH THIS ENGINE DID NOT MEASURE AT ALL
///
/// `grep -rniE 'drawdown|max_loss|worst_trade|peak_to_trough'` over the whole
/// crate returned NOTHING before these fields existed. The operator's aim is
/// maximum profit at MINIMAL LOSS and the second half had no number anywhere —
/// [`Cell::edge_ratio`] is the nearest thing and is computed over trades that
/// ENDED PROFITABLE, so it is structurally silent about how large a loser gets.
///
/// Both figures are on the PESSIMISTIC series, because a risk number taken from
/// the flattering reading is the one place optimism is least defensible.
///
/// `equity` is `(running, peak)` — scaffolding for the drawdown and deliberately
/// NOT on [`Cell`]: a caller reading a peak-so-far would be reading an artefact
/// of iteration order rather than a property of the variant.
fn accrue_risk(cell: &mut Cell, pess: i64, equity: (&mut i64, &mut i64)) {
    let (running, peak_equity) = equity;
    if pess < cell.worst_trade {
        cell.worst_trade = pess;
    }
    *running = running.saturating_add(pess);
    if *running > *peak_equity {
        *peak_equity = *running;
    }
    let dip = peak_equity.saturating_sub(*running);
    if dip > cell.max_drawdown {
        cell.max_drawdown = dip;
    }
}

/// Turns the three running excursion sums into the cell's three mean fields.
///
/// # Why the two denominators differ, and it is not an oversight
///
/// `winner_mae` and `winner_mfe` divide by `wins`, because they answer *"what
/// did a WINNER do"*. [`Cell::all_mae`] divides by `trades`, because it answers
/// *"what did everything I took cost me"* — and it is the one
/// [`Cell::edge_ratio`] ranks on, precisely so a variant cannot improve its
/// score by having losers a winners-only denominator never sees.
///
/// The `all_mae` guard is on `trades` rather than on `wins` deliberately: a
/// variant whose every trade lost has a perfectly real adverse excursion and a
/// zero numerator, and ranking it last is the honest answer rather than
/// skipping it.
fn mean_excursions(cell: &mut Cell, adverse_won: i64, gain_won: i64, adverse_all: i64) {
    if cell.wins > 0 {
        let n = i64::try_from(cell.wins).unwrap_or(1).max(1);
        cell.winner_mae = adverse_won / n;
        cell.winner_mfe = gain_won / n;
    }
    if cell.trades > 0 {
        let m = i64::try_from(cell.trades).unwrap_or(1).max(1);
        cell.all_mae = adverse_all / m;
    }
}

/// Close-to-entry move at `entry + offset`, in paisa.
#[expect(
    clippy::too_many_arguments,
    reason = "eight, and the eighth is the reading. Bundling the exit's four \
              into a struct was the alternative and it buys nothing: they are \
              already assembled together in `read_trip` and destructured \
              immediately here, so the struct would exist for one call and one \
              unpack. The permutation hazard the bundle guards against is \
              absent -- `offset` is the only bare `usize` and the other three \
              have distinct types."
)]
fn realised(
    bars: &[Candle],
    entry: usize,
    offset: usize,
    entry_price: i64,
    side: Side,
    by: Ended,
    level_ppm: Option<Ppm>,
    // Resolve an exit the bar does not price -- a square-off, or a trail with
    // no recorded peak -- against the position rather than for it.
    pessimistic: bool,
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
        // THE DISTANCE COMES FROM THE ORDER, NOT FROM A LOOKUP. `Ended::Trail`
        // carries both halves, because a cell can hold a TSL and a TTP at once
        // and they trail by different distances -- pairing one order's anchor
        // with the other's rung would price a fill nobody placed.
        (Ended::Trail { anchor, ppm, .. }, _) if anchor > 0 => {
            let give_back = paisa_of(ppm, anchor);
            let exit = match side {
                Side::Long => anchor.saturating_sub(give_back),
                Side::Short => anchor.saturating_add(give_back),
            };
            match side {
                Side::Long => exit.saturating_sub(entry_price),
                Side::Short => entry_price.saturating_sub(exit),
            }
        }
        // Everything else -- a time exit, or a trailing exit whose peak was not
        // recorded -- is a MARKET ORDER on that bar, and a market order has two
        // readings like every other fill on this path.
        //
        // # This priced at the CLOSE under both readings, and that was the last
        // unbracketed leg in the module
        //
        // The comment above used to end *"which is correct for a position
        // squared off by the clock"*, and it is the same mistake the entry made:
        // correct about WHICH BAR, silent about WHAT PRICE. A square-off is a
        // market order placed during the bar, not a guaranteed print at its
        // close -- so the honest reading is the bracket `costs::fill` already
        // draws for every other market order, and `crate::trade::walk` has drawn
        // for its own time exits since it gained `Anchor`.
        //
        // It matters more than the entry did. A stop or a target exit is at
        // least priced at the level the order rested at; a time exit had no
        // level to fall back on, and on most variants time exits are the
        // MAJORITY of trades -- the `time` column of the grid table is the
        // count. Every one of them was priced at a single mid-range print.
        _ => {
            let exit = exit_fill(bars, entry.saturating_add(offset), side, pessimistic)
                .unwrap_or(entry_price);
            match side {
                Side::Long => exit.saturating_sub(entry_price),
                Side::Short => entry_price.saturating_sub(exit),
            }
        }
    }
}

/// One trailing order, resolved against one candidate's path.
///
/// # Why the three fields travel together
///
/// A cell can now hold a TSL and a TTP at once, and they differ in all three:
/// they cross on different bars, hang from peaks recorded on those different
/// bars, and trail by different distances. Any mix — one order's anchor with the
/// other's distance — prices a fill nobody placed, and it would look entirely
/// plausible. Bundling makes the mix unspellable.
///
/// An order the variant does not carry, or one whose rung the path never
/// reached, is [`Self::NEVER_FIRED`]: `at` is `NEVER`, so it drops out of the
/// exit `min` without a branch, and both anchors are `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Trailing {
    /// Which order this is. Carried so the exit counter can separate a
    /// protected gain from a run let out -- see [`Cell::stopped`].
    kind: TrailKind,
    /// The offset it fires on, or [`NEVER`].
    at: usize,
    /// The peak as it stood BEFORE the crossing bar — the pessimistic anchor.
    before: Option<i64>,
    /// The peak including the crossing bar's own extreme — the optimistic one.
    raised: Option<i64>,
    /// The give-back distance in parts per million of the entry price.
    ppm: Ppm,
}

impl Trailing {
    /// An order this variant does not carry, or one the path never reached.
    ///
    /// Takes the kind so a never-fired order still knows which slot it is, which
    /// keeps `ended_by` able to hand one back without inventing a kind.
    const fn never_fired(kind: TrailKind) -> Self {
        Self {
            kind,
            at: NEVER,
            before: None,
            raised: None,
            ppm: 0,
        }
    }

    /// The TRAILING STOP LOSS, reading the since-ENTRY give-back table.
    fn live(cross: &Crossings, tsl: Option<usize>, trails_rungs: &[Ppm]) -> Self {
        let Some(r) = tsl else {
            return Self::never_fired(TrailKind::Live);
        };
        Self {
            kind: TrailKind::Live,
            at: cross.trail_at(r),
            before: cross.trail_peak_at(r),
            raised: cross.trail_peak_raised_at(r),
            ppm: trails_rungs.get(r).copied().unwrap_or(0),
        }
    }

    /// The TRAILING TAKE PROFIT, reading the since-ARMING give-back table.
    ///
    /// A different table and not a filtered view of the other one: the
    /// since-arming retreat is its own running maximum, for the reason
    /// `crate::excursion::Crossings::armed` gives at length.
    fn armed(cross: &Crossings, ttp: Option<Ttp>, trails_rungs: &[Ppm]) -> Self {
        let Some(t) = ttp else {
            return Self::never_fired(TrailKind::Armed);
        };
        Self {
            kind: TrailKind::Armed,
            at: cross.armed_at(t.arm, t.trail),
            before: cross.armed_peak_at(t.arm, t.trail),
            raised: cross.armed_peak_raised_at(t.arm, t.trail),
            ppm: trails_rungs.get(t.trail).copied().unwrap_or(0),
        }
    }

    /// Did this order fire at or before `chosen`?
    const fn fired_by(&self, chosen: usize) -> bool {
        self.at != NEVER && self.at <= chosen
    }

    /// The `Ended` this order produces, anchored for the given reading.
    const fn ended(&self, pessimistic: bool) -> Ended {
        let anchor = if pessimistic {
            self.before
        } else {
            self.raised
        };
        Ended::Trail {
            // A crossing with no recorded anchor cannot be priced off a peak, so
            // zero falls through to `realised`'s close-priced arm rather than
            // inventing a level.
            anchor: match anchor {
                Some(a) => a,
                None => 0,
            },
            ppm: self.ppm,
            kind: self.kind,
        }
    }
}

/// Every order that could end one candidate's trade, resolved against its path.
///
/// Bundled because [`ended_by`] took six loose parameters and would have taken
/// eight — and the four `usize` offsets among them are permutable without a
/// compile error.
#[derive(Clone, Copy, Debug)]
struct Firing {
    stop_at: usize,
    target_at: usize,
    /// The trailing STOP LOSS, live from entry.
    live: Trailing,
    /// The trailing TAKE PROFIT, armed at a target rung.
    armed: Trailing,
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
///
/// An ARMED variant is charged the armed set and not the plain one, for the
/// reason the plain one is not charged to a trail-less variant: the two
/// crossings happen on different bars, so a bar that made the since-entry trail
/// unknowable need not have made the since-arming one unknowable at all.
fn unorderable_bars(cross: &Crossings, tsl: Option<usize>, ttp: Option<Ttp>) -> u64 {
    let stop_target = u64::try_from(cross.ambiguous().len()).unwrap_or(0);
    // EACH ORDER IS CHARGED ITS OWN SET, AND A CELL CARRYING BOTH IS CHARGED
    // BOTH. The two crossings happen on different bars, so a bar that made the
    // since-entry trail unknowable need not have made the since-arming one
    // unknowable -- and a cell exposed to both is exposed to both.
    let mut trailing = 0_usize;
    if tsl.is_some() {
        trailing = trailing.saturating_add(cross.trail_ambiguous().len());
    }
    if ttp.is_some() {
        trailing = trailing.saturating_add(cross.armed_ambiguous().len());
    }
    stop_target.saturating_add(u64::try_from(trailing).unwrap_or(0))
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
///    [`Trailing::before`], the optimistic at [`Trailing::raised`].
///
/// The parameter was called `stop_wins`, which named only the first job. The
/// second was not being done at all: `crate::excursion` handed over one peak,
/// the one the crossing bar itself had made, and both readings priced the trail
/// off it — so a trailing exit's fill was settled by an ordering assumption
/// neither reading ever tested. See that module's header.
fn ended_by(f: Firing, chosen: usize, pessimistic: bool) -> Ended {
    let Firing {
        stop_at,
        target_at,
        live,
        armed,
    } = f;
    let stop_fired = stop_at != NEVER && stop_at <= chosen;
    let target_fired = target_at != NEVER && target_at <= chosen;
    // TWO TRAILING ORDERS, AND THE TIGHTER ONE WINS AN EXACT TIE.
    //
    // A cell can carry a TSL and a TTP at once, and `evaluate` only emits the
    // pair when the armed trail is STRICTLY tighter. So on a bar both reach, the
    // armed one is the nearer level and is what filled -- it is not an
    // unknowable ordering but an arithmetic fact about which order sits closer
    // to the peak. `at` is compared first because an order that fires EARLIER
    // ended the position before the other was reachable at all.
    let trail = match (live.fired_by(chosen), armed.fired_by(chosen)) {
        (true, true) => {
            match armed.at.cmp(&live.at) {
                core::cmp::Ordering::Greater => live,
                // The armed order fired FIRST, or on the same bar. On the same
                // bar the tighter distance is the nearer level, and `evaluate`
                // guarantees that is the armed one -- so both arms are `armed`
                // for two different reasons, and merging them is clippy being
                // right about the code rather than about the reasoning.
                core::cmp::Ordering::Less | core::cmp::Ordering::Equal => armed,
            }
        }
        (true, false) => live,
        (false, true) => armed,
        (false, false) => Trailing::never_fired(TrailKind::Live),
    };
    let trail_fired = trail.at != NEVER;
    if stop_fired && target_fired {
        return if pessimistic {
            Ended::Stop
        } else {
            Ended::Target
        };
    }
    // A STOP AND A TRAIL ON ONE BAR IS ALSO UNORDERABLE, AND THIS BRANCH USED TO
    // DENY IT.
    //
    // `if stop_fired { Ended::Stop }` came first and unconditionally, so a bar
    // that reached both was priced as the stop in BOTH readings. They agreed by
    // construction, `Cell::uncertainty` reported zero, and
    // `depends_on_unknowable_ordering` answered false -- for a bar whose ordering
    // is exactly as unknowable as the stop-versus-target case two lines above,
    // which this module already resolves twice. Measured: 46 of 68 round trips
    // on one cell, 80 of the 325 cells affected.
    //
    // The pessimistic reading takes the STOP and the optimistic the TRAIL,
    // because a trail fires on a give-back FROM A PROFIT while a stop fires at a
    // fixed loss from entry, so in the ordinary case the stop is the worse fill.
    //
    // THE CORNER WHERE THAT ORDERING IS WRONG IS HANDLED ELSEWHERE, on purpose.
    // A trail whose distance exceeds the stop's plus the whole run so far fills
    // BELOW the stop, and then "pessimistic" would have named the better one.
    // Rather than reason about it here without the prices, `one_variant` orders
    // the two realised figures after computing them, so `pessimistic` is the
    // smaller by construction whichever branch produced it.
    if stop_fired && trail_fired {
        return if pessimistic {
            Ended::Stop
        } else {
            trail.ended(false)
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
        trail.ended(pessimistic)
    } else {
        Ended::Time
    }
}

/// The level a given exit reason fills at, in parts per million from entry.
const fn level_for(by: Ended, stop_ppm: Option<Ppm>, target_ppm: Option<Ppm>) -> Option<Ppm> {
    match by {
        Ended::Stop => stop_ppm,
        Ended::Target => target_ppm,
        // A TRAILING EXIT CARRIES ITS OWN DISTANCE AND IS NOT LOOKED UP HERE.
        //
        // It used to take a `trail_ppm` argument, which was unambiguous while a
        // cell could hold ONE trailing order. With a TSL and a TTP on the same
        // cell there are two distances, and a lookup by kind could return the
        // one that did not fire -- pricing a fill nobody placed, plausibly.
        // `Ended::Trail` now carries the anchor and the distance together.
        Ended::Trail { .. } | Ended::Time => None,
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
        // THE SAME REFUSAL THE CROSSING WALK MAKES, AND FOR THE SAME REASON.
        //
        // This is a second walk over the same window, so a record that could not
        // move a rung must not be allowed to set the reported MAE or MFE either.
        // It is belt-and-braces rather than redundant: `evaluate` already drops
        // any candidate whose path carries a refused bar, but `peak` is `pub(crate)`
        // through two wrappers and a future caller could reach it with a path
        // that was never filtered.
        if bar.check().is_err() {
            continue;
        }
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

    /// A VARIANT CANNOT WIN BY HAVING NO STOP, WHICH IS WHAT IT USED TO DO.
    ///
    /// # The measurement this closes
    ///
    /// `edge_ratio` is the key `Grid::sharpest` ranks on, and it is described as
    /// *"the tightest stop that would not have killed them"*. It divided
    /// `winner_mfe` by `winner_mae` — **both over winners only** — so a variant
    /// with no stop was rewarded twice: its winners ran further, raising the
    /// numerator, and the losers a stop would have cut ran to the horizon and
    /// never entered a winners-only denominator.
    ///
    /// `crates/runner/src/validate.rs` records what that cost: the old key
    /// **chose the no-stop variant 61–100% of the time**, and
    /// `crates/runner/src/synthetic.rs` records two of three ranking keys doing
    /// the same. A metric asked for the tightest stop selected for having none.
    ///
    /// # Why this is a constructed pair and not a real grid
    ///
    /// The property is about the FORMULA, so the fixture is two cells that
    /// differ only in the way the loophole exploited. Running a real grid and
    /// asserting which variant won would test the market fixture as much as the
    /// metric, and would pass or fail for reasons this test is not about.
    #[test]
    fn a_stopless_variant_cannot_outrank_a_stopped_one_by_hiding_its_losers() {
        // THE STOPPED VARIANT. Winners ran 300 and sweated 100; losers were cut
        // early, so across every trade the mean adverse excursion stays 120.
        let stopped = Cell {
            trades: 10,
            wins: 5,
            winner_mfe: 300,
            winner_mae: 100,
            all_mae: 120,
            ..Cell::default()
        };

        // THE STOPLESS VARIANT. Its winners ran further because nothing cut them
        // — 400 against the same 100 sweat — which is exactly the shape that
        // used to win. But its losers ran to the horizon, so the adverse
        // excursion over EVERYTHING it took is 600.
        let stopless = Cell {
            trades: 10,
            wins: 5,
            winner_mfe: 400,
            winner_mae: 100,
            all_mae: 600,
            ..Cell::default()
        };

        // UNDER THE OLD KEY the stopless variant wins: 400/100 beats 300/100.
        // That comparison is written out rather than described, because it is
        // the defect and a reader should be able to see it.
        assert!(
            stopless.winner_mfe * 100 / stopless.winner_mae
                > stopped.winner_mfe * 100 / stopped.winner_mae,
            "the fixture must reproduce the OLD behaviour or this test proves \
             nothing about the change"
        );

        // UNDER THE NEW KEY the stopped variant wins, because the losers the
        // stop cut are now in the denominator: 300/120 = 250 against
        // 400/600 = 66.
        assert!(
            stopped.edge_ratio() > stopless.edge_ratio(),
            "a variant that lets its losers run must not outrank one that cuts \
             them: stopped {} vs stopless {}",
            stopped.edge_ratio(),
            stopless.edge_ratio()
        );

        // AND THE ZERO CASE IS A REFUSAL, NOT AN INFINITY. Nothing ever went
        // adverse, so there is no ratio to take — saying so beats dividing by
        // zero and beats reporting an unbounded score.
        let spotless = Cell {
            trades: 3,
            wins: 3,
            winner_mfe: 500,
            all_mae: 0,
            ..Cell::default()
        };
        assert_eq!(
            spotless.edge_ratio(),
            0,
            "a sample too clean to rank scores zero rather than infinity"
        );
    }

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
            if c.stop.is_none() && c.target.is_none() && c.tsl.is_none() {
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

        let before_best = g.best().map(|c| (c.stop, c.target, c.tsl));
        let before_sharp = g.sharpest().map(|c| (c.stop, c.target, c.tsl));

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
            mutated.best().map(|c| (c.stop, c.target, c.tsl)),
            before_best,
            "`best` moved when only the optimistic totals changed, so it reads \
             a figure it must not"
        );
        assert_eq!(
            mutated.sharpest().map(|c| (c.stop, c.target, c.tsl)),
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
            // SHARPER THAN THE EQUALITY IT REPLACES, NOT WEAKER.
            //
            // This read `pessimistic == optimistic`, which held only while both
            // readings entered at the same price. They no longer do: the worst
            // reading enters at the adverse extreme, so the two differ by the
            // entry cost even when nothing about the path is unknowable.
            //
            // The property the test was always protecting is that with no
            // ambiguous bar there is nothing UNKNOWABLE left — and that is now
            // asserted directly, together with the stronger claim that the whole
            // remaining gap is accounted for. An equality would have passed a
            // cell whose two causes cancelled; this cannot.
            assert_eq!(
                c.uncertainty(),
                0,
                "with no ambiguous bar there is nothing for the two readings to \
                 disagree about that the data could settle"
            );
            assert_eq!(
                c.bracket(),
                c.fill_cost,
                "with no ambiguous bar the entire spread is the entry fill, and \
                 any remainder is a third cause nothing is measuring"
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
                    .saturating_add(c.trailed_stop)
                    .saturating_add(c.trailed_profit)
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
        let live = super::Trailing {
            at: 1,
            before: Some(105_000),
            raised: Some(107_000),
            ppm: 40_000,
            kind: super::TrailKind::Live,
        };
        let firing = super::Firing {
            stop_at: super::NEVER,
            target_at: super::NEVER,
            live,
            armed: super::Trailing::never_fired(super::TrailKind::Armed),
        };
        let pess = super::ended_by(firing, 1, true);
        let opt = super::ended_by(firing, 1, false);
        assert_eq!(
            pess,
            super::Ended::Trail {
                anchor: 105_000,
                ppm: 40_000,
                kind: super::TrailKind::Live,
            },
            "the pessimistic reading anchors the order where it hung when the \
             bar opened"
        );
        assert_eq!(
            opt,
            super::Ended::Trail {
                anchor: 107_000,
                ppm: 40_000,
                kind: super::TrailKind::Live,
            },
            "the optimistic reading anchors it at the peak the bar itself made"
        );

        let priced_pess = super::realised(&[], 0, 1, 100_000, Side::Long, pess, None, true);
        let priced_opt = super::realised(&[], 0, 1, 100_000, Side::Long, opt, None, false);
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

    /// The three bars and two ladders the trailing-ambiguity case needs.
    ///
    /// Bar 1 is the whole fixture: it opens with the peak at 105,000, dips to
    /// 101,000 — exactly the 40,000-ppm rung below that peak — and also prints a
    /// new high at 107,000. Stops and targets sit at 900,000 ppm so neither can
    /// ever fire, leaving the trail as the only exit competing with the clock
    /// and the trail's own ordering as the only ambiguity in the cell.
    ///
    /// Extracted so the test that uses it stays inside the line budget without
    /// any of its assertions being dropped to fit.
    fn trail_ambiguity_fixture() -> (
        Vec<indicators::Candle>,
        crate::excursion::Ladder,
        crate::excursion::Ladder,
    ) {
        let bars = vec![
            candle(0, 100_000, 105_000, 100_000, 105_000),
            candle(1, 104_000, 107_000, 101_000, 106_000),
            candle(2, 106_000, 106_500, 105_500, 106_000),
        ];
        let never = crate::excursion::Ladder::new(vec![900_000]).expect("an ascending ladder");
        let trails = crate::excursion::Ladder::new(vec![40_000]).expect("an ascending ladder");
        (bars, never, trails)
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
        let (bars, never, trails) = trail_ambiguity_fixture();
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
        // The fills come from `entry_fills` rather than being written in, so the
        // fixture cannot drift from what `evaluate` would actually build for
        // this bar. Both variants below take `Side::Long`.
        let (entry_pess, entry_opt) = super::entry_fills(&bars, 0, Side::Long);
        let candidates = vec![super::Candidate {
            signal: 0,
            entry: 0,
            time_exit: 2,
            cross,
            entry_pess,
            entry_opt,
        }];
        let rungs = (never.rungs(), never.rungs(), trails.rungs());

        let trailed = super::one_variant(
            &bars,
            &candidates,
            rungs,
            super::Variant {
                stop: None,
                target: None,
                tsl: Some(0),
                ttp: None,
            },
            Side::Long,
        );
        assert_eq!(trailed.trades, 1, "one candidate, one round trip");
        assert_eq!(
            trailed.ambiguous_bars, 1,
            "the bar that fired the trail and raised the peak must be counted, \
             or the uncertainty is reported with nothing behind it"
        );
        // Bar 0 opens at 100,000 with a high of 105,000, so the worst entry is a
        // the high itself -- 105,000 -- and gives up 5,000 against the open. Nothing
        // is added to it: a tick beyond the high is a price the bar never printed.
        // 800 was this exit with the entry at the open; both terms are written
        // out because a single new number would record the sum and lose which
        // half of the round trip moved.
        assert_eq!(
            trailed.fill_cost, 5_000,
            "in at the printed high, and charged"
        );
        assert_eq!(
            trailed.pessimistic,
            800 - 5_000,
            "priced off 105,000, the peak the order hung from, less that entry"
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
                tsl: None,
                ttp: None,
            },
            Side::Long,
        );
        assert_eq!(
            timed.ambiguous_bars, 0,
            "a variant with no trailing rung cannot be moved by a trailing \
             ambiguity"
        );
        assert_eq!(timed.uncertainty(), 0);
        // 5,510 AND NOT 5,005, AND THE DIFFERENCE IS THE WHOLE POINT OF THIS
        // VARIANT SITTING BESIDE THE TRAILING ONE.
        //
        // They share an entry bar, so both are charged the same 5,005 to get in.
        // They do NOT share an exit: the trailing variant fills at the peak its
        // resting order hung from, a level, which has no spread of its own. This
        // one is squared off by the clock -- a MARKET order on bar 2, whose open
        // is 106,000 and whose low is 105,500, so the adverse fill is 105,500 and
        // getting out costs a further 500.
        //
        // A test asserting the two were equal is what this replaces, and it was
        // wrong for a reason worth keeping: it assumed the charge belonged to the
        // entry alone, which is exactly the assumption that left the exit leg
        // unbracketed in the first place.
        assert_eq!(
            timed.fill_cost,
            5_000 + 500,
            "in at the extreme AND out at it -- a level exit is charged only once"
        );
        assert_eq!(
            trailed.fill_cost, 5_000,
            "a level exit has no spread of its own"
        );
        assert_eq!(
            timed.pessimistic,
            6_000 - 5_500,
            "squared off on bar 2 at its printed low of 105,500, not its close"
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
        // 325 AND NOT 125 SINCE THE ARMING AXIS LANDED, and not 525 either: an
        // arm with no trail to arm, and an arm the target pre-empts, are both
        // refused rather than counted. See `variants` for the arithmetic and
        // `arming_tests` for what printing them would have cost.
        assert_eq!(super::variants(4, 4, 4), 625, "the shipped four rungs");
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

    #[test]
    fn each_exit_counter_can_only_be_moved_by_the_order_that_owns_it() {
        // WHAT THIS REPLACES. `Ended::Stop | Ended::Trail { .. } => cell.stopped`
        // folded three different things into one number, defended as "a trailing
        // exit IS a stop". That held while a cell carried ONE trailing order. A
        // cell now carries a fixed stop, a trailing stop loss and a trailing
        // take profit, and the merged counter told the operator only "it did not
        // time out" -- which the timeout column already said.
        //
        // The test is stated as four implications plus four witnesses, and the
        // witnesses are the half that bites. Without them every implication is
        // vacuously true on a grid whose counters are all zero, which is exactly
        // what a broken `count_exit` would produce.
        let (bars, column) = swept_with_wide_bars();
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            h(15),
            Side::Long,
            4,
        );

        let (mut saw_stop, mut saw_tsl, mut saw_ttp, mut saw_target) = (false, false, false, false);
        for c in &g.cells {
            if c.stop.is_none() {
                assert_eq!(
                    c.stopped, 0,
                    "a variant with no fixed stop cannot report a stop-out"
                );
            }
            if c.tsl.is_none() {
                assert_eq!(
                    c.trailed_stop, 0,
                    "a variant with no trailing stop loss cannot report one firing"
                );
            }
            if c.ttp.is_none() {
                assert_eq!(
                    c.trailed_profit, 0,
                    "a variant with no trailing take profit cannot report one firing"
                );
            }
            if c.target.is_none() {
                assert_eq!(c.targeted, 0, "a variant with no target cannot hit one");
            }
            saw_stop |= c.stopped > 0;
            saw_tsl |= c.trailed_stop > 0;
            saw_ttp |= c.trailed_profit > 0;
            saw_target |= c.targeted > 0;
        }

        assert!(
            saw_stop,
            "no cell fired a fixed stop -- the grid proves nothing"
        );
        assert!(saw_tsl, "no cell fired a trailing stop loss");
        assert!(saw_ttp, "no cell fired a trailing take profit");
        assert!(saw_target, "no cell hit a target");
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod arming_tests {
    use super::{Cell, Variant, evaluate, one_variant, variants};
    use crate::excursion::{Ladder, Ladders, Side, crossings};
    use crate::outcome::Horizon;
    use indicators::Candle;
    use indicators::column::Column;
    use vocab::ConditionMask;

    /// One cell's four rungs, as a sortable key. A named alias because four
    /// `Option<usize>` in a row is exactly the shape `clippy::type_complexity`
    /// exists to refuse, and `Chosen` is the production answer to the same
    /// objection.
    type Setting = (
        Option<usize>,
        Option<usize>,
        Option<usize>,
        Option<super::Ttp>,
    );

    fn candle(minute: i64, open: i64, high: i64, low: i64, close: i64) -> Candle {
        Candle::new(
            minute.saturating_mul(60_000_000),
            open,
            high,
            low,
            close,
            100,
            indicators::OI_NULL,
        )
    }

    fn ladder(rungs: &[i64]) -> Ladder {
        Ladder::new(rungs.to_vec()).expect("an ascending ladder")
    }

    /// The same evaluator the sibling test module builds, spelled out here.
    ///
    /// `Availability::Absent` for the reason `CLAUDE.md` §3 rule 7 gives: the
    /// VWAP availability probe reads the whole slice, so a caller that indexes
    /// rather than streams inherits no protection from it.
    fn evaluator() -> indicators::evaluator::Evaluator {
        indicators::evaluator::Evaluator::new(
            indicators::evaluator::Widths::pinned().expect("pinned widths are valid"),
            indicators::vwap::Availability::Absent,
            indicators::pattern::Thresholds::CLASSICAL,
        )
    }

    /// THE AXIS HAS TO CHANGE AN ANSWER, OR IT IS FOUR TIMES THE GRID FOR
    /// NOTHING.
    ///
    /// This is the test that would have caught the old state of affairs. The
    /// doc on `Cell::trail` claimed a trailing take profit existed and it did
    /// not: the target COMPETED with the trail rather than arming it. Under
    /// that code the two cells below are byte-identical, because `arm` did not
    /// exist to make them differ.
    ///
    /// The path: a shallow wobble against the position early, then a strong run,
    /// then a give-back.
    ///
    /// * The un-armed trail (a trailing STOP LOSS) fires on the early wobble and
    ///   takes a small loss.
    /// * The armed trail (a trailing TAKE PROFIT) does not exist during the
    ///   wobble, so the position survives to make the run, and exits on the
    ///   give-back with a gain.
    ///
    /// That is the difference the operator's "minimal stop, massive target" is
    /// asking for, and it is now measurable in one table rather than argued.
    #[test]
    fn an_armed_trail_and_an_unarmed_one_reach_opposite_conclusions() {
        // ENTRY AT 1,000,000 PAISA SO ONE PPM IS ONE PAISA, and the fixture can
        // be read without converting in your head. The first draft of this test
        // used 100,000 and confused the two, which the assertions caught.
        let entry = 1_000_000_i64;
        let bars = vec![
            // bar 0: a shallow wobble. The pre-bar peak is the entry, so the low
            // is a 200 ppm retreat -- over the 150 ppm trail rung, which fires
            // the UN-ARMED trail here.
            candle(0, entry, entry + 100, entry - 200, entry - 200),
            // bar 1: the run. 5,000 ppm favourable, over the 3,000 ppm target
            // rung, so the trail ARMS at the end of this bar.
            candle(1, entry - 200, entry + 5_000, entry - 200, entry + 5_000),
            // bar 2: give back 200 ppm from the 5,000 peak. The armed trail
            // fires here, holding a large gain.
            candle(
                2,
                entry + 5_000,
                entry + 5_000,
                entry + 4_800,
                entry + 4_800,
            ),
        ];
        let never = ladder(&[900_000]);
        let targets = ladder(&[3_000]);
        let trails = ladder(&[150]);
        let cross = crossings(
            &bars,
            0,
            2,
            entry,
            Side::Long,
            Ladders {
                stops: &never,
                targets: &targets,
                trails: &trails,
            },
        );
        assert_eq!(
            cross.trail_at(0),
            0,
            "the fixture must fire the un-armed trail on bar 0, or this test \
             asserts nothing"
        );
        assert_eq!(cross.target_at(0), 1, "and arm on bar 1");
        assert_eq!(cross.armed_at(0, 0), 2, "and fire the armed trail on bar 2");

        // The fills come from `entry_fills` rather than being written in, so the
        // fixture cannot drift from what `evaluate` would actually build for
        // this bar. Both tests below take `Side::Long`.
        let (entry_pess, entry_opt) = super::entry_fills(&bars, 0, Side::Long);
        let candidates = vec![super::Candidate {
            signal: 0,
            entry: 0,
            time_exit: 2,
            cross,
            entry_pess,
            entry_opt,
        }];
        let rungs = (never.rungs(), targets.rungs(), trails.rungs());
        let tsl = one_variant(
            &bars,
            &candidates,
            rungs,
            Variant {
                stop: None,
                target: None,
                tsl: Some(0),
                ttp: None,
            },
            Side::Long,
        );
        let ttp = one_variant(
            &bars,
            &candidates,
            rungs,
            Variant {
                stop: None,
                target: None,
                tsl: None,
                ttp: Some(super::Ttp { arm: 0, trail: 0 }),
            },
            Side::Long,
        );

        assert!(
            tsl.pessimistic < 0,
            "the trailing STOP LOSS is cut by the early wobble: {}",
            tsl.pessimistic
        );
        assert!(
            ttp.pessimistic > 0,
            "the trailing TAKE PROFIT does not exist during the wobble, so it \
             survives to the run: {}",
            ttp.pessimistic
        );
        assert_ne!(
            tsl, ttp,
            "if these two cells agree the fourth axis is decoration"
        );
    }

    /// THREE FAMILIES OF UNREACHABLE CELL ARE REFUSED, WHICH IS WHY 625 AND NOT
    /// 1,125.
    ///
    /// The refusals, in the order they were found:
    ///
    /// 1. **An arm with no trail to arm** — nothing is armed, so the cell
    ///    duplicates the trail-less one beside it.
    /// 2. **An arm at or above the target** — the target closes the position
    ///    before the arm can arm, so the trail can never fire. This one SHIPPED,
    ///    200 cells wide, and made the audit print a trailing take profit for a
    ///    run in which nothing armed.
    /// 3. **An armed trail no tighter than the live one** — both hang from the
    ///    same peak once armed, so only the smaller distance can ever fire.
    ///
    /// The third is what the fifth axis costs and what keeps it affordable:
    /// splitting TSL from TTP naively gives 1,125 cells, and refusing the inert
    /// pairs gives 625.
    ///
    /// Both halves are asserted: the arithmetic in `variants`, and the loop in
    /// `evaluate` that must agree with it.
    #[test]
    fn the_grid_refuses_every_cell_that_cannot_differ() {
        assert_eq!(variants(4, 4, 4), 625, "5 * [25 + 10*10]");
        assert_eq!(variants(0, 0, 0), 1, "no ladders is still the baseline row");
        assert_eq!(
            variants(1, 1, 1),
            2 * (2 * 2 + 1),
            "one rung an axis: 2 stops x [2 targets * 2 tsl settings + 1 arm * 1 \
             armed rung] = 10"
        );

        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            Horizon::bars(15).expect("a non-zero horizon"),
            Side::Long,
            4,
        );
        assert!(!g.cells.is_empty(), "the fixture must produce a grid");
        assert_eq!(
            g.cells.len(),
            variants(g.stops.len(), g.targets.len(), g.trails.len()),
            "the loop and the arithmetic must agree, or the reservation is a \
             guess"
        );
        // THE THIRD REFUSAL, WHICH IS THE FIFTH AXIS'S WHOLE PRICE.
        //
        // A TTP alone is legitimate -- that is the arming axis as it shipped.
        // What cannot exist is a TTP whose trailing rung is at or above a TSL on
        // the same cell: once armed both hang from the same peak, the live one
        // fires first, and the armed one is inert.
        assert!(
            g.cells.iter().all(|c| match (c.tsl, c.ttp) {
                (Some(live), Some(t)) => t.trail < live,
                _ => true,
            }),
            "an armed trail no tighter than the live one can never fire, and \
             every such cell duplicates the TSL-only cell beside it"
        );
        // AND THE SECOND REFUSAL STILL HOLDS: an arm at or above the target is
        // pre-empted by it.
        assert!(
            g.cells.iter().all(|c| match (c.target, c.ttp) {
                (Some(t), Some(ttp)) => ttp.arm < t,
                _ => true,
            }),
            "an arm the target pre-empts can never fire"
        );
        // AND NO TWO CELLS CARRY THE SAME SETTING. A duplicate would mean the
        // loop emitted one variant twice, which is how the trail factor came to
        // be missing from the reservation in the first place.
        let mut seen: Vec<Setting> = g
            .cells
            .iter()
            .map(|c| (c.stop, c.target, c.tsl, c.ttp))
            .collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(before, seen.len(), "every cell is a distinct setting");
    }

    /// THE BASELINE IS STILL FINDABLE AND STILL UNIQUE.
    ///
    /// `Grid::baseline` searches for the row with no stop, no target and no
    /// trail. The fourth axis could have introduced a second such row -- the
    /// arm-with-no-trail cells -- and `find` would have returned whichever came
    /// first. It does not, because those cells are never emitted, and this pins
    /// that rather than trusting it.
    #[test]
    fn exactly_one_cell_is_the_baseline() {
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            Horizon::bars(15).expect("a non-zero horizon"),
            Side::Long,
            4,
        );
        let bases: Vec<&Cell> = g
            .cells
            .iter()
            .filter(|c| {
                c.stop.is_none() && c.target.is_none() && c.tsl.is_none() && c.ttp.is_none()
            })
            .collect();
        assert_eq!(bases.len(), 1, "one baseline, not one per arm setting");
        assert_eq!(g.baseline(), bases.first().copied());
        assert_eq!(
            g.baseline().and_then(|c| c.ttp),
            None,
            "and it carries no trailing take profit"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod rewalk_tests {
    use super::evaluate;
    use crate::excursion::Side;
    use crate::outcome::Horizon;
    use indicators::column::Column;
    use vocab::ConditionMask;

    fn evaluator() -> indicators::evaluator::Evaluator {
        indicators::evaluator::Evaluator::new(
            indicators::evaluator::Widths::pinned().expect("pinned widths are valid"),
            indicators::vwap::Availability::Absent,
            indicators::pattern::Thresholds::CLASSICAL,
        )
    }

    /// A TIGHTER EXIT TAKES MORE TRADES, AND FOR MONTHS IT COULD NOT.
    ///
    /// # The defect, measured
    ///
    /// This module's header says the sequence is re-walked per variant because
    /// *"a stop that fires early ends the position early, and under
    /// `crate::trade`'s one-position-at-a-time rule that frees the NEXT signal
    /// to be taken sooner"*.
    ///
    /// It could not. `evaluate` built its candidates from `Trades::trades`,
    /// which is the level-less walk's answer with rule 4 already applied — the
    /// longest possible holds, so the most exclusion. A tighter stop frees the
    /// next signal only if that signal is in the candidate list, and rule 4 had
    /// removed it. An adversarial fleet measured `one_variant`'s `open_until`
    /// guard firing **zero times across 168,892 cells**: every one of the 625
    /// exit variants measured the time-exit baseline's trade set.
    ///
    /// So the whole exit grid compared 625 ways of pricing ONE sequence of
    /// trades, while claiming to compare 625 sequences.
    ///
    /// # The property, not a number
    ///
    /// A tightest-stop cell must take **at least as many** round trips as the
    /// baseline, and on any fixture where a stop ever fires early, strictly
    /// more. Asserting a specific count would pin this fixture; asserting the
    /// ordering fails the moment the candidate list goes back to being
    /// pre-excluded, whatever the fixture.
    #[test]
    fn a_tighter_exit_can_take_a_trade_the_baseline_had_no_room_for() {
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let g = evaluate(
            &bars,
            &column,
            &ConditionMask::default(),
            Horizon::bars(15).expect("a non-zero horizon"),
            Side::Long,
            4,
        );
        let base = g.baseline().copied().expect("the baseline row exists");
        assert!(base.trades > 0, "the fixture must trade at all");

        // The most-constrained cell: tightest stop, tightest target, tightest
        // TSL. Every one of its exits fires at or before the baseline's, so it
        // can only free signals, never consume more.
        let tightest = g
            .cells
            .iter()
            .filter(|c| c.stop == Some(0) && c.target == Some(0) && c.tsl == Some(0))
            .max_by_key(|c| c.trades)
            .copied()
            .expect("a cell with every tightest rung exists");

        assert!(
            tightest.trades >= base.trades,
            "an exit that never fires LATER than the baseline cannot take fewer \
             round trips: {} against the baseline's {}",
            tightest.trades,
            base.trades
        );
        assert!(
            tightest.trades > base.trades,
            "the tightest exit must take STRICTLY more round trips than the \
             time-exit baseline on this fixture -- it exits sooner, so it frees \
             signals rule 4 blocked. Equal counts is exactly what the grid \
             produced when its candidates came from the already-excluded list, \
             and it is what made all 625 cells measure one trade set. Got {} \
             against {}",
            tightest.trades,
            base.trades
        );
    }

    /// THE ELIGIBLE UNIVERSE CONTAINS THE TAKEN TRADES, ALWAYS.
    ///
    /// `eligible` is every signal that cleared rules 1, 1b, 1c and 2; `trades`
    /// is the subset rule 4 admitted. A `trades` entry absent from `eligible`
    /// would mean the two lists were built by different rules, which is the way
    /// this refactor could have gone wrong without any count looking odd.
    #[test]
    fn every_taken_trade_was_eligible_and_the_counters_still_reconcile() {
        let bars = crate::synthetic::sessions(8);
        let column = Column::build(&bars, &mut evaluator());
        let t = crate::trade::walk(
            &bars,
            &column,
            &ConditionMask::default(),
            Horizon::bars(15).expect("a non-zero horizon"),
            costs::fill::Direction::Long,
        );
        assert!(
            t.eligible.len() >= t.trades.len(),
            "the universe cannot be smaller than the selection: {} against {}",
            t.eligible.len(),
            t.trades.len()
        );
        assert!(
            t.trades.iter().all(|x| t.eligible.contains(x)),
            "every taken trade must appear in the eligible universe, or the two \
             lists were built by different rules"
        );
        assert!(
            t.eligible.len() > t.trades.len(),
            "on this fixture the empty mask fires on every swept bar, so rule 4 \
             MUST block some of them -- an equal pair means `eligible` is being \
             filtered by exclusivity too, which is the defect this exists to \
             prevent"
        );
        // AND THE THREE-WAY RECONCILIATION IS UNMOVED. Routing the early
        // refusals through `blocked` changed which counter a blocked-and-late
        // signal lands in; it must not change that every signal lands in
        // exactly one.
        assert!(
            t.reconciles(),
            "trades + blocked + too-late must still equal signals"
        );
    }
}
