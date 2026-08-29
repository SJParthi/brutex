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
    /// Round trips whose LEVEL exit the bar gapped clean through, so the fill
    /// was the bar's open rather than the level the ladder asked for.
    ///
    /// `CLAUDE.md` §4 is why this is COUNTED rather than absorbed: a gap fill is
    /// not the price the ladder chose, and a cell whose money comes from gaps
    /// has to say so rather than presenting it as the rung working.
    ///
    /// **Zero on `synthetic::sessions(8)`, and this doc used to claim 1.8%.**
    /// That figure was never measured; instrumenting [`level_fill`] over 55
    /// million calls on that fixture fires the gap arm exactly zero times,
    /// because consecutive bars open 3 paisa apart against a span of at least
    /// 120. §3 rule 6. See [`level_fill`] for what the arm does on a fixture
    /// that can reach it.
    pub gapped: u64,
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
    /// The **95% lower confidence bound** on the win rate, in basis points.
    ///
    /// # The number `win_rate_bp` cannot produce
    ///
    /// Forty winners out of forty and twelve out of twelve are both `10_000`.
    /// They are not the same evidence and no ranking built on the raw rate can
    /// tell them apart -- so a twelve-trade fluke and a forty-trade sniper sort
    /// as equals, and the fluke usually wins on some tiebreak.
    ///
    /// This is the Wilson score interval's lower end, which answers "given what
    /// was observed, how low could the TRUE rate plausibly be". It reads:
    ///
    /// | observed | trades | `win_rate_bp` | this |
    /// |---|---|---|---|
    /// | 12 of 12 | 12 | `10_000` | `7_575` |
    /// | 40 of 40 | 40 | `10_000` | `9_123` |
    /// | 900 of 1000 | 1000 | `9_000` | `8_798` |
    ///
    /// **That ordering is the whole point.** A forty-trade perfect record beats
    /// a thousand-trade ninety-percent one, and a twelve-trade perfect record
    /// beats neither. Rarity is not penalised; INSUFFICIENCY is, and by exactly
    /// the amount the arithmetic warrants rather than by a floor someone picked.
    ///
    /// # Why not a `min_trades` floor instead
    ///
    /// A floor is a guess that applies the same number to a 55% strategy and a
    /// 100% one, when the evidence they need is wildly different. This bound
    /// scales with what was actually seen. It also cannot be gamed by finding a
    /// rarer combination, which a total-P&L ranking rewards backwards.
    ///
    /// # Cost, and why a float is allowed here
    ///
    /// One `sqrt` and a fixed number of arithmetic operations -- O(1), on the
    /// same terms as every other accessor `CLAUDE.md` §3 rule 4 bounds. §7 bans
    /// floats for PRICES; this is a statistic, and §7's own last line keeps
    /// statistical values at full precision. Only the final conversion to basis
    /// points is integral, and it FLOORS, so a reported bound is never better
    /// than the true one.
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    // SCOPED TO THIS FUNCTION, and deliberately not to the module.
    //
    // `significance`, `bootstrap` and `outcome` each take this exception at
    // module level, and they can: none of them ever sees a price. `grid` does --
    // it carries paisa in `gross_win`, `gross_loss` and every P&L field on this
    // same struct -- so a module-level allow here would switch off the lint that
    // keeps §7's integer rule enforceable for the rest of the file.
    #[expect(
        clippy::float_arithmetic,
        reason = "CLAUDE.md §7 keeps statistical values at full precision and \
                  bans floats for PRICES. This is a confidence bound over two \
                  COUNTS; no paisa figure enters it and none leaves."
    )]
    #[must_use]
    pub fn assurance_bp(&self) -> i64 {
        // 1.959964 is the two-sided 95% normal quantile. Written out rather than
        // named 1.96, because the rounded value shifts the bound in the fourth
        // basis point and this number is compared against a rule.
        const Z: f64 = 1.959_964;
        if self.trades == 0 {
            return 0;
        }
        // `as` and not `try_from`: a u64 past f64's exact-integer range still
        // converts to the nearest representable double, which for a trade count
        // is a relative error below 2^-52 and cannot move a basis point.
        #[expect(
            clippy::cast_precision_loss,
            reason = "a trade count large enough to lose f64 precision is 2^53 \
                      round trips, which no slice this engine can hold produces"
        )]
        let n = self.trades as f64;
        #[expect(
            clippy::cast_precision_loss,
            reason = "wins <= trades, so the same bound applies"
        )]
        let w = self.wins as f64;
        let p = w / n;
        let z2 = Z * Z;
        let denominator = 1.0 + z2 / n;
        let centre = p + z2 / (2.0 * n);
        let margin = Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
        let lower = (centre - margin) / denominator;
        // Clamped before the cast, and the bounds are CONSTANTS -- `clamp` panics
        // only when a bound is NaN or inverted, and neither can happen here.
        //
        // `lower` cannot itself be NaN for any reachable input: `n` is non-zero
        // past the guard above, `p` is therefore finite, the square root's
        // argument is a sum of two non-negative terms, and the denominator
        // exceeds one. The clamp is not defending against that -- it is pinning
        // the RANGE, so that a future edit to the formula cannot produce a bound
        // outside [0, 10_000] and have it silently compared against a rule.
        // Were a NaN ever to reach the cast, Rust's saturating float-to-int
        // conversion yields 0, which is the refusal we want rather than a
        // passing score.
        #[expect(
            clippy::cast_possible_truncation,
            reason = "the value is clamped to [0, 10_000] immediately above the \
                      cast, so no truncation beyond the intended floor occurs"
        )]
        let bp = (lower * 10_000.0).clamp(0.0, 10_000.0) as i64;
        bp
    }

    /// A cell of `trades` round trips at `win_rate_bp`, for a bound query.
    ///
    /// Wins are rounded UP, because a rate is a floor: an operator asking for
    /// 80% of eleven trades means nine, not 8.8.
    #[must_use]
    fn at_rate(trades: u64, win_rate_bp: i64) -> Self {
        let wins = trades
            .saturating_mul(win_rate_bp.max(0).unsigned_abs())
            .saturating_add(9_999)
            / 10_000;
        Self {
            trades,
            wins: wins.min(trades),
            ..Self::default()
        }
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

    /// What this variant would have made if EVERY winner had been its smallest
    /// and EVERY loser its largest, in paisa.
    ///
    /// # The number the two thresholds are a proxy for
    ///
    /// An operator's bound is usually stated as a pair — "at least half the
    /// trades win, and my smallest win beats my largest loss by a quarter". That
    /// pair is a SUFFICIENT CONDITION for something simpler, and the simpler
    /// thing is what they actually want to know:
    ///
    /// ```text
    /// wins × min_win  −  losses × worst_loss
    /// ```
    ///
    /// At a 50% win rate and a 1.25 ratio the arithmetic is
    /// `500 × 1.25 − 500 × 1.00 = +125` per thousand trades — profitable in the
    /// worst arrangement the sample admits. Stating the floor directly beats
    /// stating the pair, because the pair cannot express the trade an operator
    /// would happily make: a 40% win rate at a 2.0 ratio floors at `+200` and
    /// fails a 50% threshold that the weaker `+125` passes.
    ///
    /// # This is a FLOOR, not a forecast
    ///
    /// It is what the sample already did under its own worst ordering, and it
    /// says nothing about the next thousand trades. It cannot be beaten DOWN by
    /// re-ordering the same trades, which is exactly why it is worth having:
    /// `pessimistic` is the total that actually occurred, and this is the total
    /// that could not have been undercut given the same wins, losses and
    /// extremes.
    ///
    /// Zero losses makes it `wins × min_win`, which is correct: nothing lost, so
    /// nothing subtracts. Zero wins makes it negative, which is also correct.
    ///
    /// Saturates rather than wrapping. The product is taken in `i128` because
    /// `wins` and `min_win` are each free to be large, and a wrapped floor would
    /// report a losing variant as the best one in the grid.
    #[must_use]
    pub fn guaranteed_floor(&self) -> i64 {
        let losses = self.trades.saturating_sub(self.wins);
        // `max(0)` and not an `abs`: a non-negative `worst_trade` means nothing
        // lost, and a "loss" of zero must subtract zero rather than adding.
        let worst_loss = self.worst_trade.saturating_neg().max(0);
        let gain = i128::from(self.wins).saturating_mul(i128::from(self.min_win));
        let pain = i128::from(losses).saturating_mul(i128::from(worst_loss));
        let net = gain.saturating_sub(pain);
        i64::try_from(net).unwrap_or(if net.is_positive() {
            i64::MAX
        } else {
            i64::MIN
        })
    }

    /// Whether this variant clears BOTH halves of a stated bound.
    ///
    /// `min_win_rate_bp` and `min_rr_bp` are both hundredths, matching
    /// [`Self::win_rate_bp`] and [`Self::reward_to_risk_bp`]: a 50% win rate is
    /// `5_000` and a 1.25 reward-to-risk is `125`.
    ///
    /// # Why `wins > 0` is a separate clause and not an implication
    ///
    /// A variant with no winners has `min_win` of zero, so its ratio is zero and
    /// it fails any positive `min_rr_bp` — but a caller passing `min_rr_bp` of 0
    /// would admit it, and "no winners" is never what an operator means by a
    /// cleared bound. The clause makes that independent of the thresholds.
    ///
    /// `trades > 0` likewise: [`Self::win_rate_bp`] returns 0 on an empty cell,
    /// which would pass a `min_win_rate_bp` of 0.
    #[must_use]
    pub const fn clears(&self, min_win_rate_bp: i64, min_rr_bp: i64) -> bool {
        self.trades > 0
            && self.wins > 0
            && self.win_rate_bp() >= min_win_rate_bp
            && self.reward_to_risk_bp() >= min_rr_bp
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

/// The fewest round trips at which `win_rate_bp` can still clear `assurance_bp`.
///
/// # Why a search DOWNWARD needs this, and what its absence cost
///
/// A support threshold decides how often a combination may fire, and a
/// self-tuning search walks it downward looking for a rare setup. It has to stop
/// somewhere, and the honest stopping point is **the sample below which the
/// statistics can no longer support the claim being made** — not a cadence
/// somebody typed.
///
/// `crates/cli` floored its descent at one trade a week, which over 121 months
/// of one-minute bars is 561 ppm — about 526 round trips. An operator hunting a
/// setup that fires FORTY times is asking for 42 ppm, so the entire 42–561 ppm
/// band was never searched. The mask was never built, never counted, never
/// ranked. That is not the prune discarding it; it is the threshold the prune
/// was applied under, and it is the single largest reason a rare combination
/// could not be found.
///
/// # What this computes
///
/// The Wilson lower bound rises with `n` at a fixed rate, so there is a smallest
/// `n` at which a record of `win_rate_bp` still clears `assurance_bp`. Below it,
/// **no combination can pass however good it is** — so descending further buys
/// nothing, and stopping earlier discards reachable answers.
///
/// Measured on the shipped bound, 80% observed against a coin-flip floor: eleven
/// trades. Against an 80% floor: about a hundred and twenty. The difference
/// between those two is the difference between a search that can find a rare
/// setup and one that cannot.
///
/// # Bounded, and it returns the cap rather than looping
///
/// `ceiling` bounds the search so a caller cannot hang on an unsatisfiable pair
/// — an `assurance_bp` above the rate itself is never reachable at any `n`,
/// because the lower bound is always under the observed rate. The cap is
/// returned rather than an error: a caller asking "how few trades will do" wants
/// a number, and "more than this many" is that number.
#[must_use]
pub fn trades_needed_for(win_rate_bp: i64, assurance_bp: i64, ceiling: u64) -> u64 {
    if assurance_bp <= 0 {
        // No bound to clear. One trade is a sample; zero is not.
        return 1;
    }
    for trades in 1..=ceiling {
        if Cell::at_rate(trades, win_rate_bp).assurance_bp() >= assurance_bp {
            return trades;
        }
    }
    ceiling
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

    /// The variant whose SMALLEST winner most exceeds its LARGEST loser, among
    /// those that took at least `min_trades` round trips.
    ///
    /// # Three selectors, three different questions, and this is the third
    ///
    /// [`Self::best`] maximises the TOTAL, and [`Self::best_within`]'s own doc
    /// records what that costs: the profit-maximising cell is normally
    /// `-/-/-+0@0`, **the variant with no stop at all**. [`Self::sharpest`]
    /// maximises [`Cell::edge_ratio`] — winner MFE over all MAE — which is a
    /// statement about EXCURSIONS, about how far price ran while a position was
    /// open. Neither answers what a losing trade actually cost.
    ///
    /// [`Cell::reward_to_risk_bp`] is the operator's own rule: smallest win over
    /// largest loss, extremes over extremes, never mean over mean. This selector
    /// is that rule used to CHOOSE rather than to judge afterwards.
    ///
    /// # `best_within` covers the constrained case; this covers the open one
    ///
    /// [`Self::best_within`] answers "of the cells that clear my threshold, which
    /// makes the most money" — the right question once a threshold exists. This
    /// answers "which cell has the best ratio at all", which is what a report has
    /// to show when no threshold has been stated yet, and what an operator needs
    /// in order to pick one. It takes no `Rules`, so it is reachable from any
    /// crate holding a [`Grid`].
    ///
    /// # `min_trades` IS REQUIRED, because the metric has an infinity in it
    ///
    /// [`Cell::reward_to_risk_bp`] returns [`i64::MAX`] when `worst_trade` is not
    /// negative — a cell that never lost. That is the correct answer to the RATIO
    /// and a trap in a SELECTION: a cell firing twice with two winners would
    /// otherwise outrank every cell that has actually been tested, and print as
    /// the operator's best risk-reward.
    ///
    /// Two things stop it. Ties at [`i64::MAX`] break on `trades`, so among cells
    /// that never lost the larger sample wins rather than the earlier one. And
    /// the floor is a REQUIRED argument: this signature refuses to default it,
    /// because a default is how the trap comes back wearing a caller's name.
    ///
    /// `None` when no variant took `min_trades` trades with at least one winner.
    #[must_use]
    pub fn by_reward_to_risk(&self, min_trades: u64) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| c.wins > 0 && c.trades >= min_trades)
            .max_by_key(|c| (c.reward_to_risk_bp(), c.trades, c.pessimistic, merit(c)))
    }

    /// The variant with the largest guaranteed floor, among those clearing the
    /// stated bound.
    ///
    /// # The whole search, in one call
    ///
    /// Every exit setting is a candidate here, INCLUDING the one with no stop,
    /// no target, no trailing stop and no trailing take-profit — that is
    /// [`Self::baseline`]'s cell, and it competes on the same key as the rest. A
    /// combination whose best answer is "hold to the time exit and place
    /// nothing" is a real answer, and a search that could not return it would be
    /// choosing the shape of the result in advance.
    ///
    /// # Why the ranking key is the floor rather than the total
    ///
    /// [`Self::best`] maximises [`Cell::pessimistic`] — the total that actually
    /// occurred under worst-case fills. That is the right number to REPORT and
    /// the wrong one to rank a bound on, because it is order-dependent in a way
    /// the operator's rule is not: two variants with identical wins, losses and
    /// extremes can differ in `pessimistic` purely by which trades happened to
    /// be which. [`Cell::guaranteed_floor`] cannot be moved by re-ordering the
    /// same trades, so ranking on it ranks the property rather than the sample's
    /// arrangement of it.
    ///
    /// Ties break on `pessimistic`, then `merit`, so where two variants floor
    /// identically the one that actually made more money wins.
    ///
    /// # It does NOT agree with [`Self::best`] where the bound fails to bind
    ///
    /// This sentence used to end *"and a constrained search agrees with
    /// `Self::best` wherever the bound does not bind"*, copied from
    /// [`Self::best_within`] — where it IS true, because that selector uses
    /// `best`'s own key. This one does not: `best` maximises
    /// `(pessimistic, merit)` and this maximises `(guaranteed_floor, …)`, so the
    /// two disagree whenever the highest-floor cell is not the highest-total
    /// cell, bound or no bound.
    ///
    /// The crate's own `nothing_clearing_is_none_and_the_floor_outranks_the_total`
    /// is the counter-example: two cells that BOTH clear `(50, 5_000, 125)`, one
    /// making 40,000 paisa at a 26,000 floor and one making 90,000 at 14,400 —
    /// and this returns the 40,000. That disagreement is the feature. Claiming
    /// otherwise described a selector this is not.
    ///
    /// `min_trades` is required for the reason [`Self::by_reward_to_risk`]
    /// gives: without it a two-trade cell that never lost clears every bound and
    /// floors above every tested one.
    ///
    /// `None` when no variant clears — which is a finding ABOUT THE COMBINATION,
    /// not a gap in the grid, and the caller must say so rather than falling
    /// back to [`Self::best`].
    #[must_use]
    pub fn best_clearing(
        &self,
        min_trades: u64,
        min_win_rate_bp: i64,
        min_rr_bp: i64,
    ) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| c.trades >= min_trades && c.clears(min_win_rate_bp, min_rr_bp))
            .max_by_key(|c| (c.guaranteed_floor(), c.pessimistic, merit(c)))
    }

    /// The variant with the largest guaranteed floor among those a
    /// [`crate::bound::Bound`] admits.
    ///
    /// [`Self::best_clearing`] takes three loose integers; this takes the
    /// criteria as one value. Two reasons that matters, and neither is style:
    ///
    /// 1. **Two `i64` thresholds side by side transpose silently.** A caller
    ///    passing `(125, 5_000)` instead of `(5_000, 125)` compiles, runs, and
    ///    admits everything — a 1.25% win-rate floor and a 50:1 reward ratio.
    /// 2. **A bound built from an HTTP body has to survive to the report.** The
    ///    criteria a run was judged under belong beside its result, and a value
    ///    can be carried there where three arguments cannot.
    ///
    /// Every exit setting competes, [`Self::baseline`]'s no-stop-no-target cell
    /// included, for the reason [`Self::best_clearing`] gives.
    ///
    /// `None` when the bound admits nothing — a finding about the combination,
    /// which the caller must report rather than answering with [`Self::best`].
    /// [`Self::refusals`] is how to say why.
    #[must_use]
    pub fn best_under(&self, bound: &crate::bound::Bound) -> Option<&Cell> {
        self.cells
            .iter()
            .filter(|c| bound.admits(c))
            .max_by_key(|c| (c.guaranteed_floor(), c.pessimistic, merit(c)))
    }

    /// Every variant's verdict under a bound, in cell order.
    ///
    /// # An empty result is not an explanation
    ///
    /// When [`Self::best_under`] returns `None` an operator has been told that
    /// none of several thousand exit settings passed, and nothing about why.
    /// The settings could have failed on six different clauses and the answer
    /// would look identical.
    ///
    /// This is the audit trail for that: one [`crate::bound::Verdict`] per
    /// variant, each naming the clause that decided it and both sides of the
    /// comparison. A caller that tallies these can tell an operator "every
    /// setting cleared the ratio and missed the trade floor" — which is a
    /// different instruction from "nothing passed".
    ///
    /// Allocates one `Vec` sized to the grid. That is O(variants), the same
    /// order as the grid the caller already holds, and it is why this is a
    /// separate call rather than something [`Self::best_under`] always pays.
    #[must_use]
    pub fn refusals(&self, bound: &crate::bound::Bound) -> Vec<crate::bound::Verdict> {
        self.cells.iter().map(|c| bound.verdict(c)).collect()
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

/// How [`evaluate`] builds the exit grid's three ladders.
///
/// # Why a struct and not three arguments
///
/// The same reason [`Ladders`] and [`Variant`] are structs: two of these three
/// are `Option<Ppm>` and the third is a `usize`, so a caller can swap the step
/// and the forced stop without a compile error and get a grid stepped at the
/// operator's rule with the rule merged in as a step. The mistake would produce
/// a plausible-looking table.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Levels<'a> {
    /// Rungs per ladder. With a step, this alone decides the REACH — `rungs`
    /// times `step_ppm` — so raising it extends the ladder without moving a
    /// level already tried.
    ///
    /// The grid is a cross product, so its width grows roughly cubically in
    /// this. Counted with [`variants`]: 4 rungs is 625 cells, 20 is 935,361,
    /// and 40 is 27,637,321 — for ONE combination.
    pub rungs: usize,
    /// The step for all three ladders, in ppm.
    ///
    /// `Some` makes every axis ABSOLUTE: rungs at `step`, `2·step`, `3·step`,
    /// so a grid can express "two point stop, four point target". `None` keeps
    /// the quantile ladders, which are relative to each signal's own excursions
    /// and cannot express a point level at any depth — see
    /// [`crate::excursion::Ladder::stepped`].
    ///
    /// It applies to all three because a stop in points paired with a target at
    /// a percentile is a grid in which a reward-to-risk rule stated in points
    /// has no cell to land in.
    pub step_ppm: Option<Ppm>,
    /// The stop ladder, in ppm, supplied outright by the caller.
    ///
    /// # Why the stop gets its own ladder and the other axes do not
    ///
    /// Every axis shared one derived step, which made the stop a function of
    /// the instrument's median bar range. That is the right quantity for a
    /// RESOLUTION and the wrong one for a FLOOR: it reached below a point on a
    /// quiet instrument, and on one-minute execution a level closer than a
    /// typical bar's own range is hit by the entry bar itself.
    ///
    /// The stop is the one axis with a physical floor, so a caller that knows
    /// the instrument supplies it. Targets and trails keep the derived step,
    /// because nothing stops a target being far and everything argues for
    /// resolving it finely.
    ///
    /// Empty falls back to the stepped or quantile ladder, so a caller with no
    /// opinion is not forced to invent one.
    pub stops_ppm: &'a [Ppm],
    /// A stop the caller wants tried, merged into the stops ladder as an
    /// ordinary rung. `None` leaves the ladder as built.
    pub forced: Option<Ppm>,
    /// Reward-to-risk ratios, in hundredths -- `200` is 2.00.
    ///
    /// # What this collapses
    ///
    /// With `None` the targets ladder is independent of the stops and the grid
    /// is the full cross product: at a half-point step reaching twenty points
    /// that is 27,637,321 cells per combination, and 226 trillion over the 8.19
    /// million combinations 81 months produce.
    ///
    /// With `Some`, the targets ladder becomes the deduped UNION of every
    /// `stop * ratio`, one crossing table still covers all of them, and
    /// `pairs_at_a_ratio` prices only the pairs an operator would trade. The
    /// same twenty-point reach costs about 1,440 cells -- nineteen thousand
    /// times less for the same question.
    ///
    /// It is the operator's own rule expressed as a COORDINATE: "the smallest
    /// win must be at least twice the largest loss" is 1:2, and 1:2 becomes a
    /// column the search sweeps rather than a filter applied to levels chosen
    /// some other way.
    /// Sweep the targets as RATIOS of each stop rather than as an independent
    /// ladder, with the ceiling derived from the data.
    ///
    /// `true` makes the targets ladder the deduped union of every
    /// `stop * ratio`, where the ratios come from `derived_ratios` -- reaching
    /// as far as the largest favourable move any trade actually made, divided
    /// by the tightest stop. Nothing about the ladder is a typed number.
    ///
    /// `false` leaves the targets independent of the stops, which is the full
    /// cross product: at a half-point step reaching twenty points that is
    /// 27,637,321 cells per combination against about 1,440 here.
    pub ratios: bool,
}

impl Levels<'_> {
    /// The quantile ladders, as every caller had before a step existed.
    #[must_use]
    pub const fn derived(rungs: usize) -> Self {
        Self {
            rungs,
            step_ppm: None,
            forced: None,
            ratios: false,
            stops_ppm: &[],
        }
    }
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

/// Every `stop * ratio`, deduped and ascending — the targets ladder the ratio
/// axis needs.
///
/// # Small, and overlapping on purpose
///
/// Forty stops at six ratios is at most 240 products, and most collide: a
/// 10-point stop at 1:2 and a 20-point stop at 1:1 are both 20 points, which is
/// ONE rung. The dedup is not tidiness — two rungs of equal value would make
/// `crossings` carry a duplicate column and the grid report one variant twice.
///
/// `None` when nothing survives, which the caller reads as "fall back to the
/// ladder you would have built anyway" rather than as a grid with no targets.
/// The reward-to-risk ladder, with its CEILING taken from the data.
///
/// # Nothing here is a number anybody chose
///
/// A ratio ladder needs two decisions — how far it reaches and how it is
/// spaced — and both were typed constants until this function. The reach was
/// `1:8`, then `1:150`, each a guess: at a half-point stop 1:8 is a four-point
/// target, which refuses to look for the minimal-stop-maximum-win shape, and
/// 1:150 is a number with nothing behind it.
///
/// The honest ceiling is what the market OFFERED. `favourable` holds the
/// largest move each trade made, so the biggest of them divided by the tightest
/// stop is the largest ratio any trade could ever have paid. A rung past that
/// is a target nothing reached, priced at full cost, on every combination.
///
/// # The spacing is derived too
///
/// Steps widen with the ratio because that is where distinctions stop existing:
/// between 1:1 and 1:3 a quarter-step is a different strategy, and between
/// 1:100 and 1:110 no combination behaves differently. The thresholds are a
/// quarter of the way and a tenth of the way up the ladder's own range rather
/// than fixed figures, so a ladder reaching 1:20 and one reaching 1:400 are
/// each spaced against their own reach.
///
/// Capped at 512 rungs, which is the one bound here that is not derived. It is
/// stated: the targets ladder becomes a column in every candidate's crossing
/// table, and an unbounded ladder on a runaway excursion would make that table
/// the largest thing in the run.
fn derived_ratios(favourable: &[Ppm], stops: &[Ppm]) -> Vec<i64> {
    let (Some(&tightest), Some(&best)) = (stops.first(), favourable.iter().max()) else {
        return Vec::new();
    };
    if tightest <= 0 || best <= 0 {
        return Vec::new();
    }
    // In hundredths, the same units `Levels::ratios` and `Rules::min_rr_bp` use.
    let ceiling = i128::from(best).saturating_mul(100) / i128::from(tightest);
    let ceiling = i64::try_from(ceiling).unwrap_or(i64::MAX).max(100);

    let mut out: Vec<i64> = Vec::with_capacity(64);
    // 1:1 -- reward equal to risk. NOT a tunable: it is where the ratio axis
    // begins, the same way zero is where a number line does. Below it a "target"
    // is nearer than its own stop, which is not a strategy with a small ratio,
    // it is a cell that cannot be traded.
    let mut r = 100_i64;

    // THE SPACING IS PROPORTIONAL TO THE RATIO, AND IT USED TO BE THREE TYPED
    // TIERS.
    //
    // This read: step 25 below `max(ceiling/10, 300)`, step 100 below
    // `max(ceiling/4, 1_000)`, then `ceiling/20`. Six numbers -- 25, 100, 300,
    // 1_000, 20 and the 512 below -- deciding where the ladder is dense. The
    // surrounding doc argued, correctly, that steps must widen with the ratio
    // because "between 1:1 and 1:3 a quarter-step is a different strategy, and
    // between 1:100 and 1:110 no combination behaves differently". It then
    // hard-coded WHERE that transition happens, three times, against floors that
    // are themselves ratios nobody derived.
    //
    // A step proportional to the current ratio has that property by
    // construction: the ladder is geometric, so every rung is the same
    // PERCENTAGE apart rather than the same absolute distance, and the density
    // falls off exactly where distinctions stop existing. No threshold decides
    // when to widen, because widening is continuous.
    //
    // The divisor is `stops.len()` -- the rung count this instrument's own bar
    // ranges produced through `stop_ladder_ppm`. A series that supports a fine
    // stop ladder gets a fine ratio ladder beside it; one that supports two
    // stops gets two-ish ratios per doubling. Nothing here is typed, and the
    // ladder now scales with the instrument in both axes rather than one.
    //
    // `.max(1)` is arithmetic, not a policy: a step of zero never terminates.
    let per_doubling = i64::try_from(stops.len()).unwrap_or(1).max(1);

    // THE LOOP BOUND IS ARITHMETIC, NOT A BUDGET, AND IT USED TO BE 512.
    //
    // The old cap was typed and its doc called it "the one bound here that is
    // not derived". It no longer has to be: the ladder is geometric at rate
    // `1 + 1/per_doubling`, so reaching a ceiling that is `2^d` times the start
    // takes about `per_doubling * d` rungs, and `d` cannot exceed the 63 value
    // bits of the `i64` the ratio is held in. `per_doubling * 64` therefore
    // cannot be reached by a terminating walk and can only be reached by a
    // non-terminating one -- which is what a loop bound is for.
    //
    // 64 is the width of the integer, not a choice about ladders. The real
    // sizing happens downstream in `thin_to_budget`, which thins the targets
    // until `variants(stops, targets, trails)` fits the cell budget, so a
    // second budget here would be a second opinion about the same limit.
    let cap = per_doubling
        .saturating_mul(i64::from(i64::BITS))
        .try_into()
        .unwrap_or(usize::MAX);

    while r <= ceiling && out.len() < cap {
        out.push(r);
        let step = (r / per_doubling).max(1);
        r = r.saturating_add(step);
    }
    out
}

fn ratio_targets(stops: &[Ppm], ratios: &[i64]) -> Option<Ladder> {
    let mut all: Vec<Ppm> = Vec::with_capacity(stops.len().saturating_mul(ratios.len()));
    for &stop in stops {
        for &r in ratios {
            // `i128` for the same reason `pairs_at_a_ratio` uses it: a wide
            // stop times a large ratio can leave `i64`, and a wrap would put a
            // rung somewhere nobody asked for.
            let want = i128::from(stop).saturating_mul(i128::from(r)) / 100;
            if let Ok(v) = Ppm::try_from(want)
                && v > 0
            {
                all.push(v);
            }
        }
    }
    all.sort_unstable();
    all.dedup();
    Ladder::new(all)
}

/// Is target rung `ti` one of the ratios of stop rung `si`?
///
/// # Exact, not approximate
///
/// The targets ladder is built as the union of `stop * ratio` over every stop
/// and every ratio, so the product is a value that IS in the ladder rather than
/// one that has to be matched within a tolerance. A tolerance here would admit
/// neighbouring rungs at a half-point step and quietly widen the search back
/// toward the cross product it exists to avoid.
///
/// Ratios are hundredths — `200` is 2.00 — for the reason §7 gives: a ratio
/// printed beside a rule is compared by the person reading it, and a float
/// compared is a float that can disagree with itself.
///
/// # Cost
///
/// **O(ratios)**, and ratios is a handful. It runs per (stop, target) index
/// pair, which is index arithmetic in a loop that would otherwise call
/// [`one_variant`] — a walk over every candidate. Skipping that is the point.
fn pairs_at_a_ratio(stops: &[Ppm], targets: &[Ppm], si: usize, ti: usize, ratios: &[i64]) -> bool {
    let (Some(&stop), Some(&target)) = (stops.get(si), targets.get(ti)) else {
        return false;
    };
    ratios.iter().any(|&r| {
        // `i128` because a stop in ppm times a ratio in hundredths can exceed
        // `i64` on a wide ladder, and a wrap would silently match the wrong rung.
        let want = i128::from(stop).saturating_mul(i128::from(r)) / 100;
        want == i128::from(target)
    })
}

/// The stops ladder: quantiles of the observed adverse moves, plus the
/// operator's own level when one is given.
///
/// # Why this is not just `Ladder::new(derived ++ forced)`
///
/// [`crate::excursion::Ladder::new`] requires its rungs strictly ascending and
/// distinct, and a forced level can land anywhere -- below every quantile, above
/// them all, or exactly on one. Sorting and deduping is what makes any of those
/// three a legal ladder rather than a refusal. The coincident case matters most:
/// without the dedup the grid would carry two identical stop columns and report
/// one variant as two.
///
/// A `forced` of `None` returns exactly what the quantile ladder returned
/// before this existed, so a caller that does not ask for a level is not
/// changed by the option existing.
fn merged_stops(adverse: &[Ppm], rungs: usize, forced: Option<Ppm>) -> Ladder {
    merged(
        &Ladder::from_excursions(&mut adverse.to_vec(), rungs).unwrap_or_default(),
        forced,
    )
}

/// A ladder with the operator's own level folded in, sorted and deduped.
///
/// Split from [`merged_stops`] because the stepped path builds its base ladder a
/// different way and must not re-derive the quantile one to reuse the merge.
fn merged(derived: &Ladder, forced: Option<Ppm>) -> Ladder {
    let derived = derived.clone();
    let Some(level) = forced.filter(|&l| l > 0) else {
        return derived;
    };
    let mut all: Vec<Ppm> = derived.rungs().to_vec();
    all.push(level);
    all.sort_unstable();
    all.dedup();
    // `new` refuses an empty or non-ascending set; the sort and dedup above have
    // made both impossible, so the fallback is the derived ladder rather than a
    // panic -- an operator's level must never cost them the rest of the grid.
    Ladder::new(all).unwrap_or(derived)
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
    levels: Levels<'_>,
) -> Grid {
    evaluate_with(bars, column, mask, horizon, side, levels, None)
}

/// [`evaluate`], over a square-off table the caller already built.
///
/// # Why the extra door rather than a changed signature
///
/// `crate::trade::forced_exits` is a pure function of `bars`, and `evaluate` is
/// called once per CANDIDATE — so a sweep rebuilt the same table for every
/// survivor. `crate::validate` pays it twice per candidate, here and again in
/// its own direct `walk`, from a `par_iter` over the closed frequent set;
/// `crate::rank` cites a real run at 17.8 million survivors.
///
/// `evaluate` has seven callers and **four of them are in `crates/cli`**, which
/// is outside this crate. Changing its signature would edit a file this change
/// has no business touching, so the old door stays exactly as it was and passes
/// `None`. Only the hot loop in `crate::validate` passes `Some`.
#[must_use]
#[allow(
    clippy::too_many_lines,
    reason = "the four passes are one procedure and splitting them would hide \
              that the ladders are derived from the same trades the grid is \
              then measured against."
)]
pub fn evaluate_with(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    side: Side,
    levels: Levels<'_>,
    exits: Option<&[Option<crate::trade::SquareOff>]>,
) -> Grid {
    let Levels {
        rungs,
        step_ppm,
        forced,
        ratios,
        stops_ppm,
    } = levels;
    // PASS ONE: every signal that could open a position, and its path.
    // `crate::trade::walk` already applies the intraday rules, so its trades
    // give the entry bars and the time-exit bars this grid narrows.
    let direction = match side {
        Side::Long => costs::fill::Direction::Long,
        Side::Short => costs::fill::Direction::Short,
    };
    let timed = crate::trade::walk_with(bars, column, mask, horizon, direction, exits);
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
    // THE OPERATOR'S OWN STOP, MERGED IN BESIDE THE DERIVED ONES.
    //
    // # What the quantile ladder alone cannot answer
    //
    // `Ladder::from_excursions` places rungs at quantiles of the excursions
    // THIS combination produced -- with four rungs, the 20th, 40th, 60th and
    // 80th percentiles. Every one of them is relative, so the tightest stop the
    // engine will ever try is "the 20th percentile of whatever this signal
    // happens to do". On a loose signal that is 312 index points.
    //
    // An operator asking "which combinations survive a TWENTY point stop" is
    // asking a different question, and the quantile ladder cannot express it at
    // any rung count. Raising `rungs` subdivides the same distribution; it never
    // reaches below the distribution's own floor.
    //
    // So `forced` is inserted into the stops ladder as an ordinary rung. Every
    // variant that could pair with a derived stop can now pair with this one,
    // the grid widens by exactly one stop value, and nothing else changes.
    //
    // `Ladder::new` requires ascending and distinct, so the merge sorts and
    // dedups -- a forced level that coincides with a derived one is the same
    // rung and must not appear twice, or the grid would hold two identical
    // columns and a reader would see one variant reported as two.
    // ALL THREE LADDERS ARE STEPPED IN POINTS WHEN A STEP IS GIVEN.
    //
    // # Why every axis and not just the stop
    //
    // A stop stepped at one point paired with a TARGET taken from the 25th
    // percentile of the favourable move is a grid that can express "two point
    // stop" and cannot express "two point stop, four point target". The reward
    // side has to be as fine as the risk side or the reward-to-risk rule the
    // operator actually states -- 1:2, in points -- has no cell to land in.
    //
    // The trailing ladder too: a give-back stepped in points is what "trail by
    // three points" means, and a give-back at a quantile of the favourable move
    // is what "trail by a quarter of whatever this signal usually offers"
    // means. Only the first is a rule anyone states.
    //
    // `forced` still merges into the stops, so an operator's own level is tried
    // whether the ladder underneath it is stepped or derived.
    let ladder_of = |observed: &[Ppm]| match step_ppm {
        Some(width) => Ladder::stepped(width, rungs).unwrap_or_else(|| {
            Ladder::from_excursions(&mut observed.to_vec(), rungs).unwrap_or_default()
        }),
        None => Ladder::from_excursions(&mut observed.to_vec(), rungs).unwrap_or_default(),
    };
    // A caller-supplied stop ladder wins over both derivations: it is the one
    // axis where the instrument's fill behaviour, not its excursion
    // distribution, decides how tight a level can be.
    let stops = if stops_ppm.is_empty() {
        match step_ppm {
            Some(_) => merged(&ladder_of(&adverse), forced),
            None => merged_stops(&adverse, rungs, forced),
        }
    } else {
        merged(&Ladder::new(stops_ppm.to_vec()).unwrap_or_default(), forced)
    };
    // THE TARGETS LADDER IS THE UNION OF EVERY `stop * ratio`, WHEN RATIOS ARE
    // GIVEN.
    //
    // Building it as the union rather than per stop is what keeps ONE crossing
    // table covering every target an operator could ask about. `crossings`
    // precomputes each candidate's exit offsets against fixed ladders, and that
    // table is the whole reason a per-variant exit decision is three integer
    // compares; a ladder rebuilt per stop would rebuild the table with it.
    //
    // So the ladder holds every product, the crossing table covers them all
    // once, and `pairs_at_a_ratio` in the loop below picks out the cells that
    // are a stop paired with ITS ratios. The union is small: forty stops times
    // six ratios is at most 240 values, and heavily overlapping -- 10pt at 1:2
    // and 20pt at 1:1 are both 20pt, one rung.
    // The ratio ladder is derived from THIS combination's own excursions, so a
    // signal that never ran more than three points gets a ladder that stops
    // there rather than pricing four hundred rungs nothing reached.
    let ratio_set = if ratios {
        derived_ratios(&favourable, stops.rungs())
    } else {
        Vec::new()
    };
    let targets = if ratio_set.is_empty() {
        ladder_of(&favourable)
    } else {
        ratio_targets(stops.rungs(), &ratio_set).unwrap_or_else(|| ladder_of(&favourable))
    };
    // THINNED TO WHAT THE CELL COUNT CAN AFFORD, AND THIS IS THE WALL EVERY
    // OTHER BOUND MISSED.
    //
    // [`variants`] is quadratic in targets, and the target ladder is not a
    // "rung count" -- it is the deduped union of every stop times every ratio,
    // and `derived_ratios` yields up to 512 ratios. Measured on the operator's
    // own run, with the rung count already forced down to three:
    //
    // | targets | cells |
    // |---:|---:|
    // | 26 | 11,070 |
    // | 512 | 3,950,100 |
    // | 2,048 | **62,986,260** |
    //
    // So bounding the RUNG count -- which `cli::rungs_within_cell_budget` does,
    // and which `BRUTEX_GRID_RUNGS` overrides -- cannot bound this grid: it
    // constrains stops and trails while the targets multiply independently.
    // MEASURED: one rung over ONE YEAR of 60-minute bars, about 1,700 bars,
    // with `BRUTEX_GRID_RUNGS=3` and a 250-candidate screen, spent 99.7% of
    // every sample inside a single `evaluate` and never returned. D-0258
    // completed 5,249 bars in 4.26 seconds.
    //
    // Solved for T rather than clamped at a constant: with `variants` about
    // `(S+1)(T²/2)(R²/2)` once T dominates, the affordable T is
    // `2·sqrt(BUDGET / ((S+1)·R²))`. Every term is the grid's own shape, so a
    // wider stop ladder or a deeper trail ladder thins the targets instead of
    // multiplying with them.
    //
    // Thinned by EVEN STRIDE, not truncated: taking the first N would keep only
    // the tightest targets and silently delete the whole profitable end of the
    // ladder, which is the half the operator's rule is about.
    // THE TRAILING LADDER IS SCALED ON THE FAVOURABLE MOVE, not the adverse one.
    // A trailing stop is a give-back FROM A PROFIT, so the distance that makes
    // sense is a fraction of what the move actually offered -- deriving it from
    // the adverse excursion would size "how much of my gain will I return" by
    // "how much did it hurt on the way in", which are different quantities.
    let trails = ladder_of(&favourable);
    let targets = thin_to_budget(targets, stops.rungs().len(), trails.rungs().len());

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
            // THE RATIO GUARD, AND IT IS WHAT MAKES A TWENTY-POINT REACH
            // AFFORDABLE.
            //
            // # The cross product is the wrong shape for the question
            //
            // Sweeping every (stop, target) pair at a half-point step out to
            // twenty points is 27,637,321 cells per combination -- 226 trillion
            // over the 8.19 million combinations 81 months produce, which is
            // weeks of compute for a search nobody asked for. Almost every pair
            // in it is a trade nobody would take: a twenty-point stop against a
            // half-point target is not a strategy, it is a cell.
            //
            // What an operator states is a RATIO -- "the smallest win must be at
            // least twice the largest loss". So the pairs worth pricing are the
            // ones where `target == stop * ratio` for a ratio they would trade,
            // and there are a handful of those per stop rather than forty.
            //
            // # Why a guard here rather than a different loop
            //
            // `crate::excursion::crossings` precomputes each candidate's exit
            // offsets against FIXED ladders, and that table is what makes the
            // per-variant exit decision O(1). A loop that derived the target
            // ladder per stop would have to rebuild it per stop and lose that.
            //
            // The targets ladder is instead the deduped UNION of every
            // `stop * ratio`, so one crossing table still covers all of them,
            // and this guard picks out the pairs. The loop still walks the
            // product in INDEX ARITHMETIC, which is free; what it skips is
            // `one_variant`, which walks every candidate and is the whole cost.
            if let (Some(si), Some(ti)) = (stop, target)
                && !ratio_set.is_empty()
                && !pairs_at_a_ratio(stops.rungs(), targets.rungs(), si, ti, &ratio_set)
            {
                continue;
            }
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
                    None,
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
                            None,
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
/// One variant, priced, WITH one row per trade.
///
/// # Why this exists beside [`with_levels`]
///
/// `with_levels` returns a [`Cell`] — the fold. This returns the fold and the
/// rows it was folded from, so a caller can ask the question a `Cell` cannot
/// answer: was this profitable in every year, every quarter, every month?
///
/// A combination that made forty thousand in 2020 and bled steadily since has
/// the same total as one that earned it evenly over seven years, the same
/// t-statistic, and the same worst excursion. The fold cannot separate them.
/// The rows can, because each carries the entry bar's timestamp.
///
/// # Cost
///
/// One trade walk plus one variant — the same as [`with_levels`] — and a `Vec`
/// of one row per trade. Called on the handful of combinations a report names,
/// never on the millions a sweep weighs.
pub fn per_trade(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    side: Side,
    ladders: Ladders<'_>,
    variant: Chosen,
) -> Option<(Cell, Vec<TradeRow>)> {
    let mut rows = Vec::new();
    let cell = levelled(
        bars,
        column,
        mask,
        horizon,
        side,
        ladders,
        variant,
        Some(&mut rows),
        None,
    )?;
    Some((cell, rows))
}

/// One variant, priced, folded to a [`Cell`].
///
/// [`per_trade`] is the same call with the rows kept.
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
    levelled(
        bars, column, mask, horizon, side, ladders, variant, None, None,
    )
}

/// [`with_levels`], told the square-off table instead of rebuilding it.
///
/// # The allocation this removes is per CANDIDATE, and no ratio gate can see it
///
/// `trade::forced_exits` is a pure function of the bars: two allocations the
/// size of the slice plus one reverse pass, and it says so itself — *"Constant
/// per candidate, so no ratio gate could see it."* [`with_levels`] reaches it
/// through `trade::walk`, which passes `None`, so **every candidate in a
/// parallel map rebuilt the identical table**.
///
/// `crate::validate` hoists it above its in-sample `par_iter` and has since the
/// in-sample pass was parallelised. The out-of-sample pass was left behind
/// because this function had no parameter to hoist INTO — `evaluate_with`
/// gained one and `with_levels` did not. That pass is, by its own comment,
/// *"roughly three quarters of what is left"*.
///
/// Scale from the module's own figures: 11,013 candidates per fold over 91,874
/// bars is about **2 x 10^9 redundant element writes per fold**, none of which
/// changes an answer.
///
/// Same result as [`with_levels`] for the same inputs — the table is derived
/// from `bars` either way, so passing the caller's copy is an identity.
#[must_use]
#[expect(
    clippy::too_many_arguments,
    reason = "the seven `with_levels` already takes, plus the table it is being \
              handed. Bundling them would change a public surface to avoid a \
              lint about one added parameter."
)]
pub fn with_levels_using(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    side: Side,
    ladders: Ladders<'_>,
    variant: Chosen,
    exits: &[Option<crate::trade::SquareOff>],
) -> Option<Cell> {
    levelled(
        bars,
        column,
        mask,
        horizon,
        side,
        ladders,
        variant,
        None,
        Some(exits),
    )
}

/// [`with_levels`] and [`per_trade`] share this; only the collector differs.
#[expect(
    clippy::too_many_arguments,
    reason = "eight, and the eighth is the collector. The seven before it are \
              `with_levels`'s own signature, which is public and unchanged; \
              bundling them would change that surface to avoid a lint about a \
              private helper."
)]
fn levelled(
    bars: &[Candle],
    column: &Column,
    mask: &ConditionMask,
    horizon: Horizon,
    side: Side,
    ladders: Ladders<'_>,
    variant: Chosen,
    trades: Option<&mut Vec<TradeRow>>,
    exits: Option<&[Option<crate::trade::SquareOff>]>,
) -> Option<Cell> {
    // `walk_with` AND NOT `walk`, so a caller that already holds the square-off
    // table can hand it over. `walk` is `walk_with(.., None)` and rebuilds it.
    let timed = crate::trade::walk_with(bars, column, mask, horizon, direction_of(side), exits);
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
        // A PATH WITH A REFUSED BAR IN IT IS NOT A CHEAPER MEASUREMENT, IT IS A
        // DIFFERENT ONE. `evaluate` drops these and this did not, so the public
        // `with_levels` and `per_trade` paths priced them.
        //
        // `crossings` skips a bar it cannot read WITHOUT advancing the running
        // maxima, so every excursion figure after the hole is computed on a
        // path that is missing part of itself -- and `Cell` has no field in
        // which to say so, which makes the result indistinguishable from a
        // clean one.
        //
        // The magnitude is already measured in this file, above `Grid::gapped`:
        // ONE refused record among 2,250 bars moved the chosen cell from
        // `target(0)` at 3,210 paisa to `trail(0) + arm(1)` at 499,089 -- a
        // factor of 155, and a DIFFERENT exit instrument recommended. It
        // reaches walk-forward scoring through `validate` and the operator's
        // per-trade table through `cli`.
        //
        // Degrading loudly is `CLAUDE.md` S4's requirement and the count is
        // already carried: `Timed::eligible` minus what survives here is
        // exactly the refused set, and `evaluate` reports it as `refused_paths`.
        .filter(|c| c.cross.refused() == 0)
        .collect();

    // EVERY path refused is not "no trades" -- it is a measurement that could
    // not be taken, and returning `None` says so with the same voice the
    // emptiness check above uses rather than reporting a clean zero.
    if candidates.is_empty() {
        return None;
    }

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
        trades,
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
    ///
    /// Carries the gap flag because this is the book the fold counts from: a
    /// level exit the bar opened past is counted once, here, rather than three
    /// times across the readings.
    pess: Priced,
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
    pess: (usize, Ended, Option<Level>),
    opt: (usize, Ended, Option<Level>),
) -> Readings {
    let (pess_off, pess_by, pess_level) = pess;
    let (opt_off, opt_by, opt_level) = opt;
    // THE ANCHOR IS `entry_opt` ON ALL THREE, and only `charged` moves. Every
    // rung in `Crossings` is a ppm distance from the open, so that is the only
    // price a resting order can be reconstructed against; which fill the trip is
    // CHARGED to is the separate question these three readings exist to bracket.
    //
    // # The two ATTRIBUTIONS are ordered here, at the common entry
    //
    // `ended_by` deliberately does NOT resolve one corner — a trail whose
    // distance exceeds the stop's plus the whole run so far fills BELOW the
    // stop, and then the branch labelled pessimistic names the BETTER exit. Its
    // comment defers that: *"`one_variant` orders the two realised figures after
    // computing them, so `pessimistic` is the smaller by construction whichever
    // branch produced it."*
    //
    // That deferral was sound while the two figures differed only by
    // attribution. It stopped being sound the moment the level arms started
    // charging the entry spread, because `ordered` compares `pess` at
    // `entry_pess` against `opt` at `entry_opt` — and the spread MASKS the
    // inversion it was there to catch. MEASURED after that change: the swap
    // fired **0 times in 17.9 million round trips**, and 2.4% of cells reported
    // a NEGATIVE `Cell::uncertainty`, which is documented as a measurement error
    // and cannot be one. Against the parent commit: zero negatives in 2.5M
    // cells. The regression was mine and this is where it is repaired.
    //
    // So the worse ATTRIBUTION is chosen first, both readings at the open, and
    // only then is it charged to the worse entry. Still three `realised` calls
    // and one branch — no loop, no allocation, §3 rule 4 untouched. On a time
    // exit the two are identical and the else-branch keeps the old behaviour.
    let at_open = EntryPrices {
        charged: c.entry_opt,
        anchor: c.entry_opt,
    };
    let a = realised(
        bars, c.entry, pess_off, at_open, side, pess_by, pess_level, false,
    );
    let b = realised(
        bars, c.entry, opt_off, at_open, side, opt_by, opt_level, false,
    );
    let (worse, best_fills, optimistic) = if b.paisa < a.paisa {
        ((opt_off, opt_by, opt_level), b.paisa, a.paisa)
    } else {
        ((pess_off, pess_by, pess_level), a.paisa, b.paisa)
    };
    let (worse_off, worse_by, worse_level) = worse;
    Readings {
        // The worse attribution, now charged to the worse entry. `fill_cost` is
        // then the entry spread of ONE attribution and `uncertainty` the
        // attribution gap at ONE entry — both non-negative by construction
        // rather than by hope.
        pess: realised(
            bars,
            c.entry,
            worse_off,
            EntryPrices {
                charged: c.entry_pess,
                anchor: c.entry_opt,
            },
            side,
            worse_by,
            worse_level,
            true,
        ),
        opt: optimistic,
        pess_best_fills: best_fills,
    }
}

/// One completed round trip as a row, from the values [`one_variant`] already
/// holds.
///
/// Extracted so `one_variant` stays inside its line budget without dropping the
/// comments that explain why the row is emitted where it is — a budget met by
/// deleting reasoning is a budget met by making the next reader guess.
fn row_of(bars: &[Candle], c: &Candidate, pnl: i64, adverse: Ppm) -> TradeRow {
    TradeRow {
        ts_micros: bars.get(c.entry).map_or(0, |b| b.ts_micros),
        pnl,
        adverse,
        // The ppm figure taken back to paisa at THIS trade's own entry, which is
        // the price it was measured against in the first place -- so the round
        // trip is exact rather than referenced against a span-wide price that is
        // wrong at both ends of a long span.
        adverse_paisa: paisa_of(adverse, c.entry_pess),
    }
}

fn one_variant(
    bars: &[Candle],
    candidates: &[Candidate],
    rungs: (&[Ppm], &[Ppm], &[Ppm]),
    v: Variant,
    side: Side,
    // Where to put one row per trade, or `None` to fold only. See the emission
    // site for why a `Cell` cannot answer "was it profitable in every year".
    mut trades: Option<&mut Vec<TradeRow>>,
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
        let (pess, opt) = ordered(&mut cell, pess, opt);

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
        // ONE ROW PER TRADE, WHEN A CALLER ASKS FOR THEM.
        //
        // # The question a Cell cannot answer
        //
        // `Cell` is a fold: totals, counts and two maxima over every trade. It
        // answers "what did this variant make over eighty-one months" and "how
        // far did the worst trade run", and both are exactly right.
        //
        // It cannot answer "was it profitable in EVERY year", and the
        // difference is not small. A combination that made forty thousand in
        // 2020 and bled steadily since shows the same total as one that earned
        // it evenly across seven years, the same t-stat and the same worst
        // excursion. Nothing in the fold separates them, and the first is
        // worthless.
        //
        // So the rows are emitted on request, carrying the entry bar's
        // TIMESTAMP, and a caller buckets them by day, week, month, quarter,
        // half or year without re-walking anything.
        //
        // Emitted HERE rather than beside `tally_trade` because `went_against`
        // is computed below it — a row pushed earlier carried an adverse figure
        // from the previous trade, which is the kind of defect that renders
        // perfectly and reads as data.
        //
        // # Cost
        //
        // One `push` per trade when the collector is present and a null check
        // when it is not, on a path that already writes to `cell`. Callers ask
        // for it on the handful of combinations they report, never on the
        // millions they weigh.
        if let Some(rows) = trades.as_deref_mut() {
            rows.push(row_of(bars, c, pess, went_against));
        }
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
              absent -- every argument now has a distinct type. It once said \
              `offset` was the only bare `usize`, which was already false with \
              `entry` on the line above it; the bare `i64` that made the pair \
              genuinely confusable is gone too, into `EntryPrices`."
)]
fn realised(
    bars: &[Candle],
    entry: usize,
    offset: usize,
    fills: EntryPrices,
    side: Side,
    by: Ended,
    level: Option<Level>,
    // Resolve an exit the bar does not price -- a square-off, or a trail with
    // no recorded peak -- against the position rather than for it.
    pessimistic: bool,
) -> Priced {
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
    // THE LEVEL IS A PRICE, AND THE MONEY IS `EXIT - ENTRY`.
    //
    // These were two arms with a hand-written `-` and `+`:
    //
    //     (Ended::Stop,   Some(ppm)) => -paisa_of(ppm, entry_price),
    //     (Ended::Target, Some(ppm)) =>  paisa_of(ppm, entry_price),
    //
    // which booked a FIXED DISTANCE from whichever entry price the reading was
    // handed. But the crossing that decided WHEN the order fired measured every
    // rung from the entry bar's OPEN, while the pessimistic reading FILLS at the
    // printed extreme -- so the position was credited the entry bar's own
    // `high - open` (long) on every level exit, for free.
    //
    // # What that cost, measured on `synthetic::sessions(8)`
    //
    // A 455-trade 1:3 cell booked `+328,877` paisa where filling at the printed
    // extreme against the same level gives `-661,913`. The SIGN was wrong.
    //
    // Worse, and this is why the arms had to go rather than be adjusted:
    // `min_win` and `worst_trade` became pure functions of the two RUNGS, so
    // `Cell::reward_to_risk_bp` -- the operator's *"smallest win at least three
    // times the largest loss"* -- was data-independent. MEASURED: six different
    // 1:3 cells across three bits, some profitable at `+778,377` and some
    // losing at `-251,207`, ALL reported exactly `299`. A rule that returns the
    // ladder's own ratio whatever the market did carries no information, and
    // `Levels::ratios` sweeps precisely that coordinate.
    //
    // The sign now comes out of the two prices. There is nowhere left to write
    // a `-` or a `+`, which is the point.
    match (by, level) {
        (Ended::Stop | Ended::Target, Some(level)) => {
            let resting = level_price(level, fills.anchor, side);
            let Some(bar) = bars.get(entry.saturating_add(offset)) else {
                // The refusal the time arm below already takes: book the trip
                // FLAT rather than invent an exit. Unreachable in production --
                // `crossings` recorded this offset off a real bar -- and
                // reachable from a test with an empty slice, which is what keeps
                // the coverage floor honest rather than allowlisted.
                return Priced {
                    paisa: 0,
                    gapped: false,
                };
            };
            let (exit, gapped) = level_fill(bar, resting);
            let exit = stop_slippage(exit, bar, side, level.kind, pessimistic);
            let paisa = match side {
                Side::Long => exit.saturating_sub(fills.charged),
                Side::Short => fills.charged.saturating_sub(exit),
            };
            Priced { paisa, gapped }
        }
        // A TRAILING EXIT FILLS AT `peak - distance`, and this used to fall
        // back to the bar's close because the peak was not recorded.
        //
        // The level follows the best price seen, so the rung alone cannot price
        // it the way a fixed stop's can. `Crossings` now records the peak AS IT
        // STOOD when the rung was crossed -- reading it at the end of the walk
        // would price the exit against a high the position never saw, because
        // the peak only ever improves.
        //
        // The ANCHOR is the peak and the DISTANCE is a fraction of the entry
        // open, which is the whole difference between a trailing stop and a
        // fixed one: give back `d` of what you made, rather than lose `d` of
        // what you started with. THE DISTANCE COMES FROM THE ORDER, NOT FROM A
        // LOOKUP. `Ended::Trail` carries both halves, because a cell can hold a
        // TSL and a TTP at once and they trail by different distances -- pairing
        // one order's anchor with the other's rung would price a fill nobody
        // placed.
        //
        // # TWO BASES FOR ONE DISTANCE, AND THIS ARM HAD THE WRONG ONE
        //
        // `excursion::BarMoves::of` computes the retreat as `ppm_of(retreat,
        // entry)` -- a fraction of the price `crossings` was handed, which is
        // the entry bar's OPEN. That ppm is what the rung is tested against, so
        // the order fires when the give-back reaches `ppm x ENTRY`. This arm
        // then filled it at `ppm x PEAK`, and the two diverge by exactly the
        // fraction the peak ran from entry.
        //
        // On the shipped fixture the rung is 40,000 ppm: the bar's 4,000-paisa
        // retreat is 40,000 ppm of the entry (100,000) and FIRES, and 38,095 ppm
        // of the peak (105,000) and would not. The arm priced a fill for an
        // order that, on its own basis, was never touched -- and the fill landed
        // at 100,800 against a printed low of 101,000, 200 paisa below anything
        // that traded.
        //
        // # It was a TILT, not a haircut
        //
        // The same arithmetic under-books longs and over-books shorts by the
        // same fraction, because `paisa_of(ppm, peak) > paisa_of(ppm, entry)`
        // when the peak has run. MEASURED on an exact mirror: the long books
        // +800 where +1,000 is correct and the short books +1,200. That biases
        // the long-versus-short comparison `Grid::best` and `Grid::sharpest`
        // rank on, in opposite directions, so it cannot be waved through as
        // conservative.
        //
        // `fills.anchor` is the entry open, and its own doc already says it is
        // *"the only price a level can be reconstructed against"*. This arm was
        // the one place in the match that never read it.
        (Ended::Trail { anchor, ppm, .. }, _) if anchor > 0 => {
            let give_back = paisa_of(ppm, fills.anchor);
            let resting = match side {
                Side::Long => anchor.saturating_sub(give_back),
                Side::Short => anchor.saturating_add(give_back),
            };
            // AND BOUNDED BY THE EXIT BAR, exactly as the level arm above is.
            // With the basis corrected a long's trail fill can no longer sit
            // below the exit bar's low -- the crossing test IS `peak - low >=
            // ceil(ppm x entry / 1e6) >= paisa_of(ppm, entry)`, the same
            // flooring argument `level_fill` already makes for stops. What
            // remains reachable is a genuine GAP, and that is counted.
            let Some(bar) = bars.get(entry.saturating_add(offset)) else {
                return Priced {
                    paisa: 0,
                    gapped: false,
                };
            };
            let (exit, gapped) = level_fill(bar, resting);
            // A TRAIL IS A STOP, so it slips like one. Same arm, same reason.
            let exit = stop_slippage(exit, bar, side, Resting::Stop, pessimistic);
            let paisa = match side {
                Side::Long => exit.saturating_sub(fills.charged),
                Side::Short => fills.charged.saturating_sub(exit),
            };
            Priced { paisa, gapped }
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
                .unwrap_or(fills.charged);
            let paisa = match side {
                Side::Long => exit.saturating_sub(fills.charged),
                Side::Short => fills.charged.saturating_sub(exit),
            };
            Priced {
                paisa,
                gapped: false,
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

/// One completed round trip, as a caller who wants to bucket them sees it.
///
/// # Three fields, and the timestamp is the one that matters
///
/// `Cell` already carries every total and both maxima. What it cannot carry is
/// WHEN each trade happened, and that is the whole difference between "this
/// made money over seven years" and "this made money in every one of them".
///
/// `pnl` is the PESSIMISTIC reading -- worst fills on both legs -- because that
/// is the figure every selector in this crate ranks on, and a per-period table
/// built on the optimistic one would disagree with the total above it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradeRow {
    /// The entry bar's stamp, in microseconds since the epoch.
    pub ts_micros: i64,
    /// Paisa per unit under the worst reading of both legs.
    pub pnl: i64,
    /// How far this trade ran AGAINST entry, in ppm -- its own maximum.
    pub adverse: Ppm,
    /// How far it ran against IN PAISA, at this trade's own entry price.
    ///
    /// # Why both units, and why this one removes a whole class of error
    ///
    /// Ppm is what the engine measures in and is right: it is relative to each
    /// trade's own entry, so it is comparable across a span where the index
    /// went from 15,000 to 26,000.
    ///
    /// But an operator states a rule in POINTS -- "no trade beyond ten points"
    /// -- and comparing that against ppm needs a price. Converting the RULE at
    /// a span-wide reference makes the same rule mean ten points at one end of
    /// the span and seventeen at the other, which is not the rule anyone said.
    ///
    /// Carrying the paisa figure alongside removes the question: the rule is
    /// compared against what this trade actually gave up, at the price it
    /// actually traded at. No reference, no approximation, no instrument it is
    /// wrong for.
    pub adverse_paisa: i64,
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
const fn level_for(by: Ended, stop_ppm: Option<Ppm>, target_ppm: Option<Ppm>) -> Option<Level> {
    match by {
        Ended::Stop => match stop_ppm {
            Some(ppm) => Some(Level {
                kind: Resting::Stop,
                ppm,
            }),
            None => None,
        },
        Ended::Target => match target_ppm {
            Some(ppm) => Some(Level {
                kind: Resting::Target,
                ppm,
            }),
            None => None,
        },
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

/// One trip's two readings, ordered low-then-high, counting a gap once.
///
/// # The ordering stands, and it is not what was wrong
///
/// Before the level arms were repaired the two readings differed only by scaling
/// ONE fixed ppm against two entry prices, so on a long TARGET exit the branch
/// labelled pessimistic produced the LARGER number and this line EXCHANGED the
/// two fields. MEASURED: `cell.pessimistic` came back equal to the sum over
/// `entry_opt` and `cell.optimistic` to the sum over `entry_pess`.
/// [`Cell::fill_cost`] was then identically ZERO on every level exit — 45% of
/// trades on the shipped fixture — and [`Cell::uncertainty`] reported that
/// absence as intra-bar ORDERING.
///
/// With the exit now a real price the pessimistic reading is charged at the
/// worse entry against the same level, so it is genuinely the low end and the
/// swap no longer fires on a level-only cell. It is KEPT because it is what
/// makes `pessimistic <= optimistic` hold unconditionally, and three separate
/// assertions depend on that.
///
/// The gap is counted here and nowhere else, off the pessimistic book, so a
/// level the bar opened past is counted once per trip rather than once per
/// reading.
fn ordered(cell: &mut Cell, pess: Priced, opt: i64) -> (i64, i64) {
    if pess.gapped {
        cell.gapped = cell.gapped.saturating_add(1);
    }
    (pess.paisa.min(opt), pess.paisa.max(opt))
}

/// A parts-per-million distance as a paisa move against `price`.
fn paisa_of(ppm: Ppm, price: i64) -> i64 {
    let scaled = i128::from(ppm).saturating_mul(i128::from(price)) / 1_000_000;
    i64::try_from(scaled).unwrap_or(i64::MAX)
}

/// The two entry prices one reading needs.
///
/// # Why a level cannot be scaled against the price that filled
///
/// Every rung inside [`crate::excursion::Crossings`] is a ppm distance from the
/// price handed to `crossings`, and that price is the entry bar's OPEN — see the
/// call sites in `evaluate` and `levelled`, and `excursion::ppm_of`. So the
/// resting order sat at a distance from the OPEN, whichever reading is being
/// booked. Re-scaling that same ppm against the printed extreme, which is what
/// this module did, puts the order at a price nobody placed it at and credits
/// the position the entry bar's own `high - open` for free.
#[derive(Clone, Copy, Debug)]
struct EntryPrices {
    /// The fill this reading charges the round trip against: `entry_pess` on the
    /// pessimistic book, `entry_opt` on the optimistic one.
    charged: i64,
    /// `Candidate::entry_opt`, on EVERY reading, because it is the only price a
    /// level can be reconstructed against.
    anchor: i64,
}

/// Which resting order a level belongs to.
///
/// A stop is a MARKET order on trigger and a target a LIMIT order; that is the
/// whole of what a gap does differently to each, and it is why the two are named
/// rather than carried as a sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resting {
    /// Sits against the position: below a long, above a short.
    Stop,
    /// Sits with the position: above a long, below a short.
    Target,
}

/// One resting order: which kind, and how far from the crossing anchor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Level {
    kind: Resting,
    ppm: Ppm,
}

/// One reading's money, and whether the bar priced it or gapped past it.
#[derive(Clone, Copy, Debug)]
struct Priced {
    paisa: i64,
    gapped: bool,
}

/// Where the resting order sat, as a PRICE, against the crossing anchor.
fn level_price(level: Level, anchor: i64, side: Side) -> i64 {
    let distance = paisa_of(level.ppm, anchor);
    match (level.kind, side) {
        (Resting::Stop, Side::Long) | (Resting::Target, Side::Short) => {
            anchor.saturating_sub(distance)
        }
        (Resting::Stop, Side::Short) | (Resting::Target, Side::Long) => {
            anchor.saturating_add(distance)
        }
    }
}

/// The price the order actually filled at, and whether the bar gapped through it.
///
/// # Why the level is a floor and not a guarantee
///
/// `crossings` advances a cursor when the running excursion crosses the rung,
/// and that excursion is a running maximum — so the recorded bar is the bar
/// whose own extreme cleared it. `ppm_of` and [`paisa_of`] both floor toward
/// zero, so the reconstructed level sits at or INSIDE that extreme: for a long
/// stop, `resting >= bar.low` always. MEASURED by instrumenting this function:
/// **55,050,412 calls** over `synthetic::sessions(8)`, both sides, across
/// `derived(4)`, `derived(8)`, a stepped ladder and the shape `crates/cli`
/// passes — ZERO on the unreachable side.
///
/// What is not guaranteed is the other end. A bar that OPENED past the level was
/// already through at its first print, and the first print is the open — worse
/// than the level for a stop, better for a target, and a price that printed in
/// both cases. `Candle::check` guarantees `low <= open <= high`, so the fallback
/// cannot itself be an invention.
///
/// # The gap arm is UNREACHABLE on that fixture, and this doc used to claim a
/// rate for it
///
/// It read *"MEASURED at 224 of 12,494, 1.8%"*. That figure is **wrong** and no
/// run produced it: on `synthetic::sessions(8)` the gap arm fires **zero** times
/// in 55 million calls, because consecutive bars open 3 paisa apart against a
/// span of at least 120 — a bar cannot open past a level it did not gap to.
/// `CLAUDE.md` §3 rule 6 is why the number is removed rather than adjusted.
///
/// The arm itself is correct and was exercised deliberately: on a hand-built
/// gappy fixture it fires on 245 of 252 level exits and prices at the exit bar's
/// open exactly once per trip. What is missing is a SHIPPED test that can see it
/// move — the only one naming the field asserts `cell.gapped == 0`, which passes
/// whether the arm works or not.
/// A stop's worst-case fill, and it is one of the bar's OWN four numbers.
///
/// # `level_fill` answers with a price that never printed
///
/// It returns the resting level whenever the bar's range contains it, and that
/// level is `anchor ± paisa_of(level.ppm, anchor)` — pure arithmetic at
/// one-paisa granularity. Nothing says the market traded there. Every other fill
/// on this path is one of the four printed numbers: `crate::trade` states it
/// outright — *"NO TICK. THE FOUR NUMBERS OF THE BAR ARE THE WHOLE OF WHAT IS
/// KNOWN"* — and [`exit_fill`] prices a market exit at the open or the printed
/// extreme. The level arms were the exception, and the operator's standing rule
/// is that there are none.
///
/// # A stop is a stop-MARKET order, and that is why only stops move
///
/// A stop triggers at its level and then fills at whatever the book offers
/// next, which can be worse and frequently is. Booking it AT the trigger assumes
/// zero slippage on the one order type that carries no price guarantee at all.
/// The worst it could have filled is the bar's own adverse extreme — a number
/// that printed.
///
/// **A target is a LIMIT order.** It fills at its price or better and never
/// worse, so the level is already the conservative answer and no printed price
/// improves on it. Forcing the adverse extreme there would book a long's
/// profit-take at the bar's LOW, which is not a worst case but a different
/// trade. The asymmetry is the content of this function, and applying one rule
/// to both would be the invention §3 rule 1 forbids.
///
/// # Pessimistic only
///
/// The optimistic reading keeps the level, because a stop CAN fill at its
/// trigger and the bracket exists to hold both readings. Collapsing them would
/// hide the range the data genuinely carries, which is the defect
/// [`Cell::pessimistic`] and [`Cell::optimistic`] were split to end.
///
/// # What this costs, measured
///
/// `the_entry_spread_is_the_whole_fill_cost_of_a_level_exit` used to assert
/// that a level exit carried NO exit-side spread. It now carries one on the
/// stop side, and that test says so. Two more tests moved with it. D-0378.
///
/// O(1): one compare and a branch, on a path that has already read the bar.
const fn stop_slippage(
    exit: i64,
    bar: &Candle,
    side: Side,
    kind: Resting,
    pessimistic: bool,
) -> i64 {
    if !pessimistic || !matches!(kind, Resting::Stop) {
        return exit;
    }
    // ASSIGNED, NOT COMPARED, AND THE COMPARE THAT WAS HERE COULD NOT FIRE.
    //
    // This read `if bar.low < exit { bar.low } else { exit }` with a comment
    // claiming a gap fill "can sit on the far side of the extreme". It cannot.
    // `level_fill` returns either the resting level -- which it has just tested
    // lies INSIDE `[low, high]` -- or `bar.open`, and `Candle::check` guarantees
    // `low <= open <= high`. So `exit >= bar.low` held on every path and the
    // false arm was unreachable: a mutant replacing the whole body with this
    // assignment SURVIVED, which §4 makes a build failure rather than a nit.
    //
    // Writing what it does also states the stronger claim honestly. On a gap the
    // reading is now "triggered at the open, filled at the bar's low", which is
    // what a stop-market order through a gap actually suffers; `gapped` records
    // that it happened and the price no longer pretends otherwise.
    match side {
        // A long exits by SELLING, so its adverse extreme is the low; a short
        // exits by buying, so it is the high.
        Side::Long => bar.low,
        Side::Short => bar.high,
    }
}

fn level_fill(bar: &Candle, resting: i64) -> (i64, bool) {
    if resting >= bar.low && resting <= bar.high {
        return (resting, false);
    }
    (bar.open, true)
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
    use super::{Cell, Grid, Levels, evaluate};

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
    /// The operator's own worked example, reproduced exactly.
    ///
    /// *"If 1000 trades, 500 win, 500 loss, if the max stop loss is 1 ratio,
    /// then our minimum winning ratio should be minimum 1.25."*
    #[test]
    fn the_operators_thousand_trade_example_floors_above_zero() {
        // Largest loss ₹1.00, smallest win ₹1.25, half the trades win.
        let c = Cell {
            trades: 1_000,
            wins: 500,
            min_win: 125,
            worst_trade: -100,
            ..Cell::default()
        };

        assert_eq!(c.win_rate_bp(), 5_000, "50.00%, in hundredths");
        assert_eq!(c.reward_to_risk_bp(), 125, "1.25, in hundredths");

        // 500 x 125 - 500 x 100 = 62,500 - 50,000.
        assert_eq!(
            c.guaranteed_floor(),
            12_500,
            "even with every winner at its smallest and every loser at its \
             largest, the sample makes 12,500 paisa"
        );
        assert!(
            c.clears(5_000, 125),
            "and it clears the bound it was built to sit exactly on"
        );

        // THE BOUND IS TIGHT ON BOTH SIDES. One hundredth under either
        // threshold and the same cell fails, so neither clause is decoration.
        assert!(!c.clears(5_001, 125), "a hair more win rate and it fails");
        assert!(!c.clears(5_000, 126), "a hair more ratio and it fails");

        // AND THE FLOOR CROSSES ZERO EXACTLY WHERE THE RATIO REACHES 1.00.
        let level = Cell { min_win: 100, ..c };
        assert_eq!(level.reward_to_risk_bp(), 100, "1.00");
        assert_eq!(
            level.guaranteed_floor(),
            0,
            "at 50% and 1:1 the worst arrangement breaks exactly even -- which \
             is why the operator's margin is 1.25 and not 1.00"
        );
    }

    /// The floor is defined at both edges of the loss distribution, and it
    /// saturates rather than wrapping.
    #[test]
    fn the_guaranteed_floor_handles_no_losers_no_winners_and_overflow() {
        // NOTHING LOST. Nothing subtracts, so the floor is the whole gain.
        let unbeaten = Cell {
            trades: 10,
            wins: 10,
            min_win: 700,
            worst_trade: 0,
            ..Cell::default()
        };
        assert_eq!(unbeaten.guaranteed_floor(), 7_000);

        // A POSITIVE `worst_trade` IS STILL NOT A LOSS. `max(0)` must stop it
        // being subtracted as a negative and ADDING to the floor.
        let all_up = Cell {
            worst_trade: 500,
            ..unbeaten
        };
        assert_eq!(
            all_up.guaranteed_floor(),
            7_000,
            "a non-negative worst trade subtracts nothing, it does not add"
        );

        // NOTHING WON. Every trade is a loser at the worst size.
        let routed = Cell {
            trades: 8,
            wins: 0,
            min_win: 0,
            worst_trade: -250,
            ..Cell::default()
        };
        assert_eq!(routed.guaranteed_floor(), -2_000);
        assert!(
            !routed.clears(0, 0),
            "a cell with no winners must fail even a bound of zero and zero -- \
             that is what the `wins > 0` clause is for"
        );

        // SATURATION, NOT WRAPPING. Unreachable from any real slice, and the
        // fields are public so it is reachable from here -- which is the point:
        // a wrapped floor would report a ruinous variant as the best in the grid.
        let vast = Cell {
            trades: u64::MAX,
            wins: u64::MAX,
            min_win: i64::MAX,
            worst_trade: 0,
            ..Cell::default()
        };
        assert_eq!(vast.guaranteed_floor(), i64::MAX, "saturates high");
        let ruinous = Cell {
            trades: u64::MAX,
            wins: 0,
            min_win: 0,
            worst_trade: i64::MIN + 1,
            ..Cell::default()
        };
        assert_eq!(ruinous.guaranteed_floor(), i64::MIN, "and saturates low");
    }

    /// The bound must be able to return the variant with NO stop, no target and
    /// no trailing anything, because that is sometimes the answer.
    #[test]
    fn the_bound_ranks_the_no_exit_variant_alongside_every_other() {
        // NO STOP, NO TARGET, NO TRAIL -- the time-exit baseline. It clears the
        // bound comfortably.
        let bare = Cell {
            trades: 200,
            wins: 120,
            min_win: 400,
            worst_trade: -200,
            pessimistic: 30_000,
            stop: None,
            target: None,
            tsl: None,
            ttp: None,
            ..Cell::default()
        };
        // A STOPPED VARIANT that makes far more money and does NOT clear: its
        // smallest winner is under its largest loser.
        let stopped = Cell {
            trades: 200,
            wins: 120,
            min_win: 150,
            worst_trade: -900,
            pessimistic: 900_000,
            stop: Some(2),
            ..Cell::default()
        };

        let g = Grid {
            cells: vec![stopped, bare],
            ..Grid::default()
        };

        assert_eq!(
            g.baseline().map(|c| c.pessimistic),
            Some(30_000),
            "the no-exit cell must be findable as the baseline"
        );
        assert_eq!(
            g.best().map(|c| c.pessimistic),
            Some(900_000),
            "and `best` must still take the bigger total"
        );
        assert_eq!(
            g.best_clearing(50, 5_000, 125).map(|c| c.pessimistic),
            Some(30_000),
            "the bound must return the NO-EXIT variant, because it is the one \
             that clears -- a search that could not return it would be choosing \
             the answer's shape in advance"
        );
    }

    /// No variant clearing is a finding about the combination, and the floor
    /// decides ties before the total does.
    #[test]
    fn nothing_clearing_is_none_and_the_floor_outranks_the_total() {
        let short = Cell {
            trades: 5,
            wins: 4,
            min_win: 900,
            worst_trade: -100,
            ..Cell::default()
        };
        let g = Grid {
            cells: vec![short],
            ..Grid::default()
        };
        assert!(
            short.clears(5_000, 125),
            "the cell clears on the RATIOS -- only the trade floor excludes it"
        );
        assert_eq!(
            g.best_clearing(50, 5_000, 125),
            None,
            "and too few trades must refuse rather than fall back to `best`"
        );

        // TWO CLEARING CELLS. The one with the smaller total has the higher
        // floor, because its losses are capped tighter.
        let steady = Cell {
            trades: 100,
            wins: 60,
            min_win: 500,
            worst_trade: -100,
            pessimistic: 40_000,
            ..Cell::default()
        };
        let swingy = Cell {
            trades: 100,
            wins: 60,
            min_win: 500,
            worst_trade: -390,
            pessimistic: 90_000,
            ..Cell::default()
        };
        // steady: 60x500 - 40x100 = 26,000. swingy: 60x500 - 40x390 = 14,400.
        assert_eq!(steady.guaranteed_floor(), 26_000);
        assert_eq!(swingy.guaranteed_floor(), 14_400);
        let g2 = Grid {
            cells: vec![swingy, steady],
            ..Grid::default()
        };
        assert_eq!(
            g2.best_clearing(50, 5_000, 125).map(|c| c.pessimistic),
            Some(40_000),
            "the higher FLOOR wins even though the other made more money, \
             because the floor cannot be moved by re-ordering the same trades"
        );
    }

    /// The bound-taking selector agrees with the loose-argument one, and the
    /// refusal list explains an empty answer.
    #[test]
    fn the_bound_selects_and_the_refusals_say_why_when_it_cannot() {
        use crate::bound::{Bound, Verdict};

        let bound = Bound::new(50, 5_000, 125, 1);

        let passes = Cell {
            trades: 200,
            wins: 120,
            min_win: 400,
            worst_trade: -200,
            pessimistic: 30_000,
            ..Cell::default()
        };
        // FAILS ON THE RATIO ONLY: 150 over 900 is 0.16, well under 1.25.
        let thin = Cell {
            trades: 200,
            wins: 120,
            min_win: 150,
            worst_trade: -900,
            pessimistic: 900_000,
            ..Cell::default()
        };
        // FAILS ON SAMPLE SIZE ONLY.
        let brief = Cell {
            trades: 9,
            wins: 8,
            min_win: 800,
            worst_trade: -100,
            pessimistic: 5_000,
            ..Cell::default()
        };

        let g = Grid {
            cells: vec![thin, brief, passes],
            ..Grid::default()
        };

        assert_eq!(
            g.best_under(&bound).map(|c| c.pessimistic),
            Some(30_000),
            "the bound must return the admitted cell, not the richest one"
        );
        assert_eq!(
            g.best_under(&bound).map(|c| c.pessimistic),
            g.best_clearing(50, 5_000, 125).map(|c| c.pessimistic),
            "and it must agree with the loose-argument form it replaces"
        );

        // THE REFUSAL LIST IS ONE VERDICT PER VARIANT, IN CELL ORDER, and each
        // names a DIFFERENT clause -- which is the whole point: an operator
        // told only "nothing passed" cannot tell these three cases apart.
        assert_eq!(
            g.refusals(&bound).as_slice(),
            [
                Verdict::RewardToRiskShort {
                    got: 16,
                    needs: 125
                },
                Verdict::TooFewTrades { took: 9, needs: 50 },
                Verdict::Admitted,
            ],
            "one verdict per cell, in cell order, each naming a DIFFERENT \
             clause -- an operator told only \"nothing passed\" cannot tell \
             these three cases apart"
        );

        // NOTHING ADMITTED IS `None`, AND THE REFUSALS STILL EXPLAIN IT.
        let hopeless = Grid {
            cells: vec![thin, brief],
            ..Grid::default()
        };
        assert_eq!(hopeless.best_under(&bound), None);
        assert!(
            hopeless.refusals(&bound).iter().all(|v| !v.admitted()),
            "an empty selection must be matched by a refusal for every cell"
        );
        assert_eq!(
            Grid::default().refusals(&bound),
            Vec::new(),
            "and no cells refuses nothing"
        );
    }

    /// The operator's rule picks a DIFFERENT cell than the total does, and the
    /// gap between them is the whole reason the selector exists.
    ///
    /// `best()` maximises `pessimistic`, and `best_within`'s own doc records
    /// where that lands: the no-stop cell, which by construction has the largest
    /// loser in the grid. An operator whose rule is "my smallest win beats my
    /// largest loss three times over" is never shown that cell's ratio, because
    /// nothing selects on it.
    #[test]
    fn the_reward_to_risk_selector_refuses_the_cell_the_total_would_pick() {
        // THE DISCIPLINED CELL. Smallest winner ₹6.00 against a largest loser of
        // ₹2.00 — exactly 1:3 — and it makes ₹500 over forty round trips.
        let disciplined = Cell {
            trades: 40,
            wins: 20,
            min_win: 600,
            worst_trade: -200,
            pessimistic: 50_000,
            ..Cell::default()
        };

        // THE STOPLESS CELL. It makes ten times the money, and it does it while
        // its smallest winner (₹1.00) is a tenth of its largest loser (₹10.00).
        // This is the cell `best()` returns.
        let stopless = Cell {
            trades: 40,
            wins: 30,
            min_win: 100,
            worst_trade: -1_000,
            pessimistic: 500_000,
            ..Cell::default()
        };

        let g = Grid {
            cells: vec![disciplined, stopless],
            ..Grid::default()
        };

        // THE FIXTURE MUST REPRODUCE THE PROBLEM or the test proves nothing.
        assert_eq!(
            g.best().map(|c| c.pessimistic),
            Some(500_000),
            "`best` must still take the larger total -- this selector adds a \
             question, it does not change that one"
        );
        assert_eq!(disciplined.reward_to_risk_bp(), 300, "1:3, in hundredths");
        assert_eq!(stopless.reward_to_risk_bp(), 10, "1:0.1");

        assert_eq!(
            g.by_reward_to_risk(10).map(|c| c.pessimistic),
            Some(50_000),
            "the operator's rule must return the disciplined cell, and it is \
             the one making TEN TIMES LESS money -- that is the trade being made"
        );
    }

    /// The floor is the argument, and it is required because the metric is
    /// unbounded above.
    #[test]
    fn the_trade_floor_excludes_a_sample_too_small_to_have_a_largest_loser() {
        let tested = Cell {
            trades: 40,
            wins: 20,
            min_win: 600,
            worst_trade: -200,
            ..Cell::default()
        };
        // TWO TRADES, BOTH WINNERS, ONE TINY LOSS. Its ratio is 9:1 and it is
        // evidence of nothing.
        let lucky = Cell {
            trades: 2,
            wins: 2,
            min_win: 900,
            worst_trade: -100,
            ..Cell::default()
        };
        assert!(
            lucky.reward_to_risk_bp() > tested.reward_to_risk_bp(),
            "the fixture must make the small sample WIN on the raw metric, or \
             the floor is not being tested"
        );

        let g = Grid {
            cells: vec![tested, lucky],
            ..Grid::default()
        };
        assert_eq!(
            g.by_reward_to_risk(10).map(|c| c.trades),
            Some(40),
            "a floor of ten must exclude the two-trade cell"
        );
        assert_eq!(
            g.by_reward_to_risk(1).map(|c| c.trades),
            Some(2),
            "and a floor of one must admit it -- the floor is the CALLER'S \
             statement, not a constant hidden in here"
        );
    }

    /// `i64::MAX` is the right ratio and the wrong sort key, so the tie is
    /// broken on sample size rather than on position in the vector.
    #[test]
    fn among_cells_that_never_lost_the_larger_sample_wins() {
        let brief = Cell {
            trades: 3,
            wins: 3,
            min_win: 100,
            worst_trade: 0,
            ..Cell::default()
        };
        let long = Cell {
            trades: 300,
            wins: 300,
            min_win: 100,
            worst_trade: 0,
            ..Cell::default()
        };
        assert_eq!(brief.reward_to_risk_bp(), i64::MAX, "nothing lost");
        assert_eq!(long.reward_to_risk_bp(), i64::MAX, "nor here");

        // `brief` FIRST, so a selector that merely took the last maximum would
        // pass by accident. It is second on the reversed vector below.
        let forward = Grid {
            cells: vec![brief, long],
            ..Grid::default()
        };
        let reversed = Grid {
            cells: vec![long, brief],
            ..Grid::default()
        };
        assert_eq!(forward.by_reward_to_risk(1).map(|c| c.trades), Some(300));
        assert_eq!(
            reversed.by_reward_to_risk(1).map(|c| c.trades),
            Some(300),
            "vector order must not decide which of two infinities is returned"
        );
    }

    /// A variant with no winner has no smallest win, and an empty grid has no
    /// answer. Both are `None` rather than a zero that would sort.
    #[test]
    fn a_variant_with_no_winner_and_an_empty_grid_both_refuse() {
        let loser_only = Cell {
            trades: 20,
            wins: 0,
            min_win: 0,
            worst_trade: -500,
            ..Cell::default()
        };
        let g = Grid {
            cells: vec![loser_only],
            ..Grid::default()
        };
        assert_eq!(
            g.by_reward_to_risk(1),
            None,
            "a cell that never won must not be selectable on a REWARD ratio"
        );
        assert_eq!(Grid::default().by_reward_to_risk(1), None, "and no cells");
    }

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
            Levels::derived(4),
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
            Levels::derived(4),
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
            Levels::derived(4),
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
            Levels::derived(4),
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
            Levels::derived(4),
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
            Levels::derived(4),
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

    /// A MEASUREMENT ERROR CANNOT BE NEGATIVE, AND THIS ONCE WAS.
    ///
    /// `Cell::uncertainty` is `optimistic - pessimistic - fill_cost`, rendered
    /// as the `unknown` column and described by `audit.rs` as *"a MEASUREMENT
    /// ERROR, not an upside"*. A width cannot be less than nothing, and
    /// `depends_on_unknowable_ordering` is literally `uncertainty() != 0`, so a
    /// negative one is counted as ambiguity while displaying an impossible
    /// number.
    ///
    /// # It went negative because a repair removed the thing that hid it
    ///
    /// `ended_by` defers one corner — a trail filling below the stop, where the
    /// branch labelled pessimistic names the BETTER exit — to the `min`/`max` in
    /// `ordered`. That worked while the two readings differed only by
    /// attribution. Once the level arms began charging the entry spread, the two
    /// figures were priced at DIFFERENT entries and the spread MASKED the
    /// inversion: MEASURED, the swap fired **0 times in 17.9 million round
    /// trips**, and 2.4% of cells reported a negative width. `read_trip` now
    /// orders the two ATTRIBUTIONS at the common entry before charging one to
    /// the worse entry, which restores the deferral `ended_by` relies on.
    ///
    /// Both halves are asserted: the width is non-negative, and it plus
    /// `fill_cost` is exactly the bracket, so neither term can absorb the other.
    /// # The shape matters, and `derived(4)` is the WRONG one
    ///
    /// Written first over `Levels::derived(4)` and `derived(8)`, this test
    /// passed with the defect deliberately restored — so it proved nothing. The
    /// inversion needs a TRAIL that can fill past a STOP, which the plain
    /// quantile ladders do not produce. The shape below is the one
    /// `crates/cli` actually passes: a step, a forced operator stop, real stop
    /// rungs, and `ratios: true`. On that shape 2.4% of cells reported a
    /// negative width.
    ///
    /// A fixture chosen for convenience rather than for the defect is how the
    /// frontier's duplicate check went missing behind a test that asserted it,
    /// twice in one session.
    #[test]
    fn a_measurement_error_is_never_negative() {
        const STOPS: [super::Ppm; 4] = [8, 16, 32, 64];
        for side in [Side::Long, Side::Short] {
            let (bars, column) = swept();
            for rungs in [
                Levels {
                    rungs: 4,
                    step_ppm: Some(20),
                    stops_ppm: &STOPS,
                    forced: Some(30),
                    ratios: true,
                },
                Levels::derived(4),
            ] {
                let g = evaluate(
                    &bars,
                    &column,
                    &ConditionMask::default(),
                    h(15),
                    side,
                    rungs,
                );
                for c in &g.cells {
                    assert!(
                        c.uncertainty() >= 0,
                        "{side:?} variant {:?}/{:?}/{:?}: unknown is {} — a width \
                         cannot be less than nothing, and `unknown` is rendered \
                         as a measurement error",
                        c.stop,
                        c.target,
                        c.tsl,
                        c.uncertainty()
                    );
                    assert!(
                        c.fill_cost >= 0,
                        "{side:?}: fill cost {} is negative, and it is documented \
                         non-negative on both legs",
                        c.fill_cost
                    );
                    assert_eq!(
                        c.uncertainty().saturating_add(c.fill_cost),
                        c.bracket(),
                        "{side:?}: the two causes must sum to the whole spread, \
                         or one of them is absorbing the other"
                    );
                }
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
            let g = evaluate(
                &bars,
                &column,
                &ConditionMask::default(),
                h(15),
                side,
                Levels::derived(4),
            );
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
            Levels::derived(4),
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
            Levels::derived(4),
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

        // A TRAIL CARRIES ITS OWN ANCHOR — the PEAK — but its DISTANCE is a
        // fraction of `EntryPrices::anchor`, the entry open, because that is the
        // basis `excursion` measured the give-back in when it decided the order
        // fired. This arm used to scale the ppm against the peak instead, and
        // the two diverge by the fraction the peak had run.
        let at = super::EntryPrices {
            charged: 100_000,
            anchor: 100_000,
        };
        // REAL BARS, NOT `&[]`. The arm now bounds its fill by the exit bar, so
        // an empty slice books the trip FLAT. Bar 1 spans both resting prices,
        // so neither is a gap and the arithmetic is what is under test.
        let priced = [
            candle(0, 100_000, 105_000, 100_000, 105_000),
            candle(1, 104_000, 107_000, 100_500, 106_000),
        ];
        let priced_pess = super::realised(&priced, 0, 1, at, Side::Long, pess, None, true).paisa;
        let priced_opt = super::realised(&priced, 0, 1, at, Side::Long, opt, None, false).paisa;
        // 40,000 ppm OF THE ENTRY is 4,000 — not 4,200, which is 40,000 ppm of
        // the peak. The old figures pinned the wrong basis: a fill for an order
        // that, on the basis the rung was tested in, was never touched.
        assert_eq!(
            priced_pess, 500,
            "the trail level is 101,000 -- 105,000 peak less 4,000, which is \
             40,000 ppm of the 100,000 ENTRY -- but a TRAIL IS A STOP and fills \
             at the bar's low of 100,500 under the pessimistic reading. 500 from \
             the entry, not the 1,000 a fill at the trigger claimed. D-0378"
        );
        assert_eq!(
            priced_opt, 3_000,
            "107,000 peak less the same 4,000, less the same entry"
        );
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
    /// Six bars: one trade that ends on its target, one that ends on its stop.
    ///
    /// Priced at 1,000,000 paisa so **one ppm is one paisa** and the fixture
    /// reads without arithmetic. `spread_a` and `spread_b` are the entry bars'
    /// own `high - open` — the quantity the defect handed to the position.
    fn one_to_three_fixture(spread_a: i64, spread_b: i64) -> Vec<indicators::Candle> {
        vec![
            // Candidate 1 enters here. Its high is the pessimistic fill.
            candle(0, 1_000_000, 1_000_000 + spread_a, 1_000_000, 1_000_100),
            // Favourable 3,500 >= the 3,000 target rung, so `target_at(0) = 1`.
            candle(1, 1_000_100, 1_003_500, 1_000_000, 1_003_000),
            candle(2, 1_003_000, 1_003_000, 1_002_900, 1_003_000),
            // Candidate 2 enters here; signal 3 clears candidate 1's exit.
            candle(3, 1_000_000, 1_000_000 + spread_b, 1_000_000, 1_000_000),
            // Adverse 1,500 >= the 1,000 stop rung, so `stop_at(0) = 1`.
            candle(4, 1_000_000, 1_000_000, 998_500, 998_600),
            candle(5, 998_600, 998_700, 998_500, 998_600),
        ]
    }

    /// The 1:3 cell over one pair of entry spreads.
    fn one_to_three_cell(spread_a: i64, spread_b: i64) -> super::Cell {
        let bars = one_to_three_fixture(spread_a, spread_b);
        let stops = crate::excursion::Ladder::new(vec![1_000]).expect("an ascending ladder");
        let targets = crate::excursion::Ladder::new(vec![3_000]).expect("an ascending ladder");
        let trails = crate::excursion::Ladder::new(vec![900_000]).expect("an ascending ladder");
        let ladders = crate::excursion::Ladders {
            stops: &stops,
            targets: &targets,
            trails: &trails,
        };
        let candidates: Vec<super::Candidate> = [(0_usize, 2_usize), (3, 5)]
            .into_iter()
            .map(|(entry, time_exit)| {
                // Taken from `entry_fills` rather than written in, so the
                // fixture cannot drift from what `evaluate` actually builds.
                let (entry_pess, entry_opt) = super::entry_fills(&bars, entry, Side::Long);
                super::Candidate {
                    signal: entry,
                    entry,
                    time_exit,
                    cross: crate::excursion::crossings(
                        &bars,
                        entry,
                        time_exit,
                        entry_opt,
                        Side::Long,
                        ladders,
                    ),
                    entry_pess,
                    entry_opt,
                }
            })
            .collect();
        super::one_variant(
            &bars,
            &candidates,
            (stops.rungs(), targets.rungs(), trails.rungs()),
            super::Variant {
                stop: Some(0),
                target: Some(0),
                tsl: None,
                ttp: None,
            },
            Side::Long,
            None,
        )
    }

    /// THE RATIO LADDER IS GEOMETRIC AND ITS DENSITY COMES FROM THE STOPS.
    ///
    /// It was three typed tiers — step 25, then 100, then `ceiling/20`, with the
    /// transitions at `max(ceiling/10, 300)` and `max(ceiling/4, 1_000)`, capped
    /// at 512. Six numbers deciding where the ladder is dense on an instrument
    /// none of them had seen.
    ///
    /// The property that replaces them: every rung is the same PERCENTAGE above
    /// the last, so density falls off continuously and no threshold decides when
    /// to widen. This asserts the SHAPE rather than any figure, because the
    /// figures are now the instrument's.
    #[test]
    fn the_ratio_ladder_widens_geometrically_and_scales_with_the_stop_count() {
        // Reach is `best favourable / tightest stop`, so this pair fixes the
        // ceiling at 1:50 while letting the stop COUNT vary independently.
        let favourable = [50_000_i64];
        let coarse = super::derived_ratios(&favourable, &[1_000, 2_000]);
        let fine = super::derived_ratios(&favourable, &[1_000_i64; 12]);

        assert!(coarse.len() >= 2, "even two stops give a usable ladder");
        assert!(
            fine.len() > coarse.len(),
            "more stop rungs must buy more ratio rungs: {} against {}",
            fine.len(),
            coarse.len()
        );

        for ladder in [&coarse, &fine] {
            assert_eq!(
                ladder.first().copied(),
                Some(100),
                "every ladder starts at 1:1"
            );
            assert!(
                ladder.windows(2).all(|w| matches!(w, [a, b] if b > a)),
                "strictly increasing: {ladder:?}"
            );
            // GEOMETRIC: each gap is at least as wide as the one before it,
            // because the step is a fraction of the CURRENT ratio. A typed-tier
            // ladder is flat inside a tier and jumps between them; this one
            // cannot narrow anywhere.
            let gaps: Vec<i64> = ladder
                .windows(2)
                .filter_map(|w| match w {
                    [a, b] => Some(b - a),
                    _ => None,
                })
                .collect();
            assert!(
                gaps.windows(2).all(|g| matches!(g, [a, b] if b >= a)),
                "steps must never narrow as the ratio grows: {gaps:?}"
            );
            assert!(
                ladder.last().is_some_and(|&last| last <= 5_000),
                "and none reaches past what the market offered: {ladder:?}"
            );
        }
    }

    /// A 1:3 stop:target cell must not report a 3:1 reward-to-risk.
    ///
    /// # The defect this exists to keep dead
    ///
    /// `realised` booked a level exit as a FIXED ppm distance from the entry
    /// price — `-paisa_of(ppm, entry)` for a stop, `+paisa_of(ppm, entry)` for a
    /// target. So `min_win` and `worst_trade` were pure functions of the two
    /// RUNGS, and `Cell::reward_to_risk_bp` — the operator's *"smallest win at
    /// least three times the largest loss"* — returned the ladder's own axis
    /// whatever the bars did.
    ///
    /// MEASURED before the fix, on `synthetic::sessions(8)`: six different 1:3
    /// cells across three condition bits, some profitable at `+778,377` paisa
    /// and some losing at `-251,207`, ALL reported exactly `299`. And
    /// `Levels::ratios` sweeps precisely that coordinate, so the rule the whole
    /// search is pointed at carried no information about the market at all.
    ///
    /// The second assertion is the load-bearing one: no edit can make it pass
    /// again without reintroducing a hand-written sign.
    #[test]
    fn a_one_to_three_cell_does_not_report_three() {
        let a = one_to_three_cell(200, 600);
        let b = one_to_three_cell(50, 900);

        assert_eq!(
            (a.trades, a.wins),
            (2, 1),
            "one winner and one loser, or the ratio is vacuous"
        );
        assert_eq!(a.targeted, 1, "the winner must have ended ON the target");
        assert_eq!(a.stopped, 1, "the loser must have ended ON the stop");

        // 1_003_000 target level, entered at 1_000_200; 999_000 stop level,
        // entered at 1_000_600. Both levels are inside their bar's range.
        //
        // THE TARGET FILLS AT ITS LEVEL AND THE STOP DOES NOT, which is D-0378.
        // A target is a LIMIT order: it fills at its price or better, so the
        // level is already its worst case. A stop is a stop-MARKET order: it
        // triggers at the level and fills at whatever comes next, so its worst
        // case is the bar's own low -- 998,500 here, five hundred paisa through
        // the trigger. `min_win` is unchanged; `worst_trade` carries the
        // slippage that a fill AT the trigger assumed away.
        assert_eq!(a.min_win, 2_800, "the target level less the WORSE entry");
        assert_eq!(
            a.worst_trade, -2_100,
            "the stop bar's LOW less the worse entry -- 998,500 - 1,000,600. It \
             was -1,600 while a stop-market order was booked at its trigger"
        );
        assert_eq!(
            a.reward_to_risk_bp(),
            133,
            "2,800 over 2,100. The defect reported 300 -- the ladder's own axis"
        );
        assert!(
            a.reward_to_risk_bp() < 300,
            "a 1:3 ladder still reported its own ratio: {}",
            a.reward_to_risk_bp()
        );
        assert_ne!(
            a.reward_to_risk_bp(),
            b.reward_to_risk_bp(),
            "two different sets of bars under ONE ladder produced ONE ratio, so \
             the number is a restatement of the axis and not a measurement of \
             the data"
        );
    }

    /// A level exit's fill cost is the entry spread PLUS the stop's own slippage.
    ///
    /// # This test asserted the opposite, and its name still records that
    ///
    /// It was `the_entry_spread_is_the_whole_fill_cost_of_a_level_exit`, and
    /// the claim held only while a stop was booked AT its trigger — which
    /// assumed zero slippage on the one order type that carries no price
    /// guarantee. D-0378 prices a stop at the bar's adverse extreme under the
    /// pessimistic reading, so a level exit now carries an exit-side spread too,
    /// and it is exactly the distance the market ran through the trigger.
    ///
    /// Before an earlier fix this column was identically ZERO on the level path:
    /// the spread reached `fill_cost` only through `paisa_of(ppm, entry_pess) -
    /// paisa_of(ppm, entry_opt)`, which truncates to nothing at every rung the
    /// engine ships. A column documented as *"the whole knowable spread"*, on
    /// 45% of trades, that could not move. It moves twice now.
    #[test]
    fn a_level_exit_carries_the_entry_spread_and_the_stop_s_slippage() {
        let cell = one_to_three_cell(200, 600);
        assert_eq!(
            cell.fill_cost, 1_300,
            "200 on the target's entry bar, 600 on the stop's, and 500 more \
             where the stop bar's low ran through the trigger. It was 800 while \
             the exit side was assumed free"
        );
        assert_eq!(
            cell.gapped, 0,
            "both levels sit inside their exit bar, so neither is a gap fill"
        );
    }

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
            None,
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
        // THE GIVE-BACK IS 40,000 ppm OF THE ENTRY OPEN, WHICH IS 4,000 — not
        // 4,200, which is the same ppm of the peak. These three figures pinned
        // the peak basis, and it is the basis `excursion` never used: the rung
        // fires when the retreat reaches `ppm x entry`, so pricing the fill at
        // `ppm x peak` bought a fill for an order that was never touched. On
        // this fixture the old arithmetic put the long's fill at 100,800 against
        // a printed low of 101,000 — 200 paisa below anything that traded.
        assert_eq!(
            trailed.pessimistic,
            1_000 - 5_000,
            "105,000 peak less 4,000 (40,000 ppm of the ENTRY), less that entry"
        );
        assert_eq!(
            trailed.optimistic, 3_000,
            "107,000, the peak that bar itself made, less the same 4,000"
        );
        assert_eq!(
            trailed.uncertainty(),
            2_000,
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
            None,
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
            Levels::derived(4),
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
        let empty = evaluate(
            &bars,
            &column,
            &impossible,
            h(15),
            Side::Long,
            Levels::derived(4),
        );
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
        let g = evaluate(
            &bars,
            &column,
            &item.mask,
            h(15),
            Side::Long,
            Levels::derived(4),
        );
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
            Levels::derived(4),
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

    /// A sniper with forty perfect trades OUTRANKS a grinder with a thousand.
    ///
    /// # The ordering this pins is the entire reason the bound exists
    ///
    /// The operator's requirement is "very few trades, but every one assured".
    /// A raw win rate cannot express it: it scores 12-of-12 and 40-of-40
    /// identically at 100%, so the ranking has to fall back on trade count or
    /// total P&L, and BOTH of those reward volume -- the opposite of the ask.
    ///
    /// The Wilson lower bound resolves it without a `min_trades` floor. If this
    /// ordering ever inverts, the engine has gone back to rewarding volume and
    /// the whole sniper premise is broken, so it is asserted as a chain rather
    /// than as three independent values.
    #[test]
    fn few_and_certain_outranks_many_and_merely_good() {
        let sniper = Cell {
            trades: 40,
            wins: 40,
            ..Cell::default()
        };
        let grinder = Cell {
            trades: 1_000,
            wins: 900,
            ..Cell::default()
        };
        let fluke = Cell {
            trades: 12,
            wins: 12,
            ..Cell::default()
        };

        // The raw rate CANNOT separate the sniper from the fluke. Asserted so
        // the next reader sees why a second statistic was needed at all.
        assert_eq!(
            sniper.win_rate_bp(),
            fluke.win_rate_bp(),
            "the raw win rate is blind to sample size -- this is the defect"
        );
        assert!(
            sniper.win_rate_bp() > grinder.win_rate_bp(),
            "and it ranks the sniper above the grinder for the wrong reason"
        );

        // The bound separates all three, in the order the operator wants.
        assert!(
            sniper.assurance_bp() > grinder.assurance_bp(),
            "forty perfect trades must beat a thousand at ninety percent: \
             {} vs {}",
            sniper.assurance_bp(),
            grinder.assurance_bp()
        );
        assert!(
            grinder.assurance_bp() > fluke.assurance_bp(),
            "and a thousand at ninety percent must beat twelve perfect ones, \
             because twelve is not yet evidence: {} vs {}",
            grinder.assurance_bp(),
            fluke.assurance_bp()
        );
    }

    /// The bound is never better than the observed rate, and never negative.
    ///
    /// # Why both ends are pinned
    ///
    /// A LOWER bound that exceeded the point estimate would be a bound in name
    /// only, and every rule built on it would admit more than it promised --
    /// exactly the fallback-that-hides-a-failure `CLAUDE.md` §4 bans. The zero
    /// end matters because the value is cast to `i64` for comparison and a
    /// negative would compare as "worse than impossible" rather than refusing.
    #[test]
    fn the_bound_is_a_bound_at_both_ends() {
        for (wins, trades) in [
            (0_u64, 1_u64),
            (0, 100),
            (1, 1),
            (1, 2),
            (50, 100),
            (99, 100),
            (100, 100),
            (7, 9),
        ] {
            let cell = Cell {
                trades,
                wins,
                ..Cell::default()
            };
            let bound = cell.assurance_bp();
            assert!(
                bound >= 0,
                "a bound below zero cannot be compared honestly: {wins}/{trades} gave {bound}"
            );
            assert!(
                bound <= cell.win_rate_bp(),
                "a LOWER bound above the observed rate is not a bound: \
                 {wins}/{trades} gave {bound} against an observed {}",
                cell.win_rate_bp()
            );
        }
    }

    /// No trades is no evidence, and is stated as zero rather than as a divide.
    #[test]
    fn an_empty_cell_has_no_assurance_and_does_not_divide_by_zero() {
        assert_eq!(Cell::default().assurance_bp(), 0);
    }

    /// More of the same evidence RAISES the bound, monotonically.
    ///
    /// # The property that makes it usable as a ranking key
    ///
    /// If the bound could fall as more confirming trades arrived, an operator
    /// could improve a candidate's rank by DELETING trades from it, and the
    /// ranking would be selecting for small samples rather than for confidence.
    /// Asserted across the whole ladder rather than at one point, because a
    /// single pair can pass on an arithmetic accident.
    #[test]
    fn more_confirming_trades_can_only_raise_the_bound() {
        let mut previous = 0;
        for trades in 1_u64..=200 {
            let cell = Cell {
                trades,
                wins: trades,
                ..Cell::default()
            };
            let bound = cell.assurance_bp();
            assert!(
                bound >= previous,
                "a perfect record of {trades} scored {bound}, below the {previous} \
                 that {} scored -- the bound must not fall as evidence accrues",
                trades - 1
            );
            previous = bound;
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod arming_tests {
    use super::{Cell, Levels, Variant, evaluate, one_variant, variants};
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
            None,
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
            None,
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
            Levels::derived(4),
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
            Levels::derived(4),
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
    use super::{Levels, evaluate};
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
            Levels::derived(4),
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

/// The target ladder, thinned so the grid fits a bounded cell count.
///
/// See the call site for the measurement that made this necessary. Solves
/// `variants(S, T, R) <= BUDGET` for `T` rather than clamping at a constant, so
/// the bound moves with the grid's own shape instead of being another number
/// somebody chose.
///
/// Thinning is by EVEN STRIDE across the sorted ladder, never by truncation:
/// the ladder ascends, so keeping a prefix would keep only the tightest targets
/// and delete every wide one — the half an operator asking for "massive winning
/// side" is actually looking for.
fn thin_to_budget(targets: Ladder, stops: usize, trails: usize) -> Ladder {
    /// Cells one grid may reserve. `Cell` is about 152 bytes, so this is a few
    /// megabytes per candidate — affordable on every core at once, which is
    /// what `screen` and the walk-forward now do.
    const BUDGET: usize = 24_000;

    let have = targets.rungs().len();
    if have <= 2 || variants(stops, have, trails) <= BUDGET {
        return targets;
    }
    // variants ~= (S+1)(T+1)(R+1) + (S+1)(T(T+1)/2)(R(R+1)/2); the quadratic
    // term dominates once T is large, so solve that and floor at two.
    let s1 = stops.saturating_add(1).max(1);
    let r_term = trails.saturating_add(1).saturating_mul(trails).max(2) / 2;
    let denom = s1.saturating_mul(r_term).max(1);
    let want = (BUDGET / denom).max(1);
    // THE LARGEST T WITH T(T+1)/2 <= want, WALKED IN INTEGERS.
    //
    // The closed form is `(sqrt(8·want + 1) − 1) / 2`, and `sqrt` is float
    // arithmetic, which `[workspace.lints.clippy]` denies across this
    // workspace — the same rule `CLAUDE.md` §7 states for prices. Walking is
    // exact, needs no cast, and is bounded twice over: by `have`, the ladder it
    // is thinning, and by the triangular sum crossing `want`. No iteration can
    // exceed the number of rungs that already exist.
    let mut keep = 2_usize;
    while keep < have {
        let next = keep.saturating_add(1);
        if next.saturating_mul(next.saturating_add(1)) / 2 > want {
            break;
        }
        keep = next;
    }
    if keep >= have {
        return targets;
    }
    let stride = have.div_ceil(keep).max(1);
    let thinned: Vec<Ppm> = targets.rungs().iter().step_by(stride).copied().collect();
    Ladder::new(thinned).unwrap_or(targets)
}
