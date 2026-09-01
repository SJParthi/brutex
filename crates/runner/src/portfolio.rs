//! One deterministic global-position scheduler over already-built intents.
//!
//! [`GlobalSinglePositionV1`] is deliberately narrower than trade generation.
//! A caller supplies one minute's intents after it has decided whether each
//! exact entry is reachable or refused and, for a reachable intent, the last
//! timestamp through which that position occupies the portfolio. This module
//! does not read bars, choose an exit, or derive a price. It only decides which
//! already-described intent may own the one global position.
//!
//! # One lock means one lock
//!
//! Instrument, direction and signal rung are retained on every
//! [`Constituent`] for provenance, but none partitions occupancy. A long blocks
//! a short, NIFTY blocks BANKNIFTY, and a one-minute strategy blocks a
//! sixty-minute strategy while its interval is live. An entry at or before the
//! current [`GlobalSinglePositionV1::occupied_through_micros`] is blocked; only
//! the following timestamp may enter.
//!
//! # Determinism and bounds
//!
//! One minute accepts at most [`MAX_INTENTS_PER_MINUTE`] intents. A wider batch
//! is refused whole before state changes. Accepted intents are ordered by
//! ascending numeric priority and then by their strategy digest. A repeated
//! ordering key refuses the whole batch: otherwise caller order would become
//! an undeclared third tie-breaker.
//!
//! The scheduler, its ordering workspace and its result are fixed-size values.
//! No collection grows with the number of minutes or calls.

use brutex_core::instrument::InstrumentKey;
use costs::fill::Direction;

/// Largest one-based rank admitted within any one signal rung.
pub const MAX_PRIORITY_PER_RUNG: usize = 25;

/// The hard intent ceiling for one entry minute.
///
/// The authoritative surface retains up to 25 strategies for each of eight
/// signal rungs.  All eight sets can signal on the same execution minute, so a
/// global lock must admit the complete `8 * 25` batch before choosing one.  A
/// ceiling of 25 here would make seven rungs disappear before arbitration.
pub const MAX_INTENTS_PER_MINUTE: usize = SUPPORTED_RUNGS_MINUTES.len() * MAX_PRIORITY_PER_RUNG;

/// Signal rungs admitted by version one, in minutes.
///
/// This is the same eight-rung intraday surface the engine sweeps. A daily or
/// unknown rung is refused whole rather than admitted as an unrecognised key.
pub const SUPPORTED_RUNGS_MINUTES: [u16; 8] = [1, 2, 3, 5, 10, 15, 30, 60];

/// The stable identity of one strategy.
///
/// Ordering is bytewise and is the second scheduling key after priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StrategyDigest([u8; 32]);

impl StrategyDigest {
    /// Wraps the digest already computed by the strategy-producing caller.
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// The raw digest bytes.
    #[must_use]
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

/// One strategy participating in the global-position portfolio.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Constituent {
    /// One-based rank supplied by the caller; a lower number acts first.
    pub priority: u16,
    /// Stable strategy identity and the deterministic priority tie-breaker.
    pub strategy_digest: StrategyDigest,
    /// Instrument the strategy would enter.
    pub instrument: InstrumentKey,
    /// Whether the position would be long or short.
    pub direction: Direction,
    /// Signal rung in minutes, retained as provenance but not an occupancy key.
    pub rung_minutes: u16,
}

/// What the caller knows about one exact execution attempt.
///
/// The scheduler does not infer these states from prices or bars. A refused
/// input means no position is proved open. If an entry happened but later
/// pricing failed, the caller must use [`Self::Reachable`] with the conservative
/// occupancy extent so the global lock is retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    /// The entry exists and its complete occupancy interval is known.
    Reachable {
        /// Last timestamp owned by this position, inclusive.
        occupied_through_micros: i64,
    },
    /// No exact execution entry exists for this attempt.
    Unreachable,
    /// Upstream execution evidence refused the attempt before an entry opened.
    Refused,
}

/// One constituent's attempt to enter on the minute supplied to the scheduler.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Intent {
    /// Strategy that produced the attempt.
    pub constituent: Constituent,
    /// Caller-supplied execution evidence.
    pub evidence: Evidence,
}

/// Why a single intent was refused after its minute batch was admitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentRefusal {
    /// The execution-producing caller explicitly refused it.
    Upstream,
    /// Its claimed occupancy ended before its entry timestamp.
    OccupancyEndsBeforeEntry,
}

/// The scheduler's decision for one intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disposition {
    /// This intent owns the global position through the named timestamp.
    Admitted {
        /// Inclusive end of the newly occupied interval.
        occupied_through_micros: i64,
    },
    /// A position admitted on an earlier minute still owns the portfolio.
    BlockedOccupied {
        /// Inclusive end of the earlier position's interval.
        occupied_through_micros: i64,
    },
    /// Another valid intent won this same entry minute.
    BlockedSimultaneous {
        /// Digest of the admitted intent.
        admitted: StrategyDigest,
    },
    /// The exact entry was absent.
    Unreachable,
    /// The intent was explicitly invalid rather than silently dropped.
    Refused(IntentRefusal),
}

/// One canonically ordered intent and the decision made for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Decision {
    /// Original constituent, retained in the result for provenance.
    pub constituent: Constituent,
    /// Exactly one terminal outcome for the intent.
    pub disposition: Disposition,
}

// The no-allocation minute path deliberately keeps its two fixed work buffers
// on the stack. Pin the combined peak below 64 KiB so widening a constituent or
// the complete 8 × 25 surface cannot turn the scoped Clippy exception below
// into an unreviewed stack-growth channel.
const _: () = assert!(
    core::mem::size_of::<[Option<Intent>; MAX_INTENTS_PER_MINUTE]>().saturating_mul(2) <= 65_536
);
const _: () = assert!(
    core::mem::size_of::<[Option<Intent>; MAX_INTENTS_PER_MINUTE]>().saturating_add(
        core::mem::size_of::<[Option<Decision>; MAX_INTENTS_PER_MINUTE]>()
    ) <= 65_536
);

/// Where every offered intent went.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counters {
    /// Intents supplied to the minute operation.
    pub offered: u64,
    /// Intents that acquired the global position.
    pub admitted: u64,
    /// Valid intents blocked by an earlier minute's position.
    pub blocked_occupied: u64,
    /// Valid intents blocked by the winner of their own minute.
    pub blocked_simultaneous: u64,
    /// Intents whose exact entry did not exist.
    pub unreachable: u64,
    /// Intents explicitly refused by either the caller or scheduler.
    pub refused: u64,
}

impl Counters {
    /// The five mutually exclusive terminal buckets.
    #[must_use]
    pub const fn accounted(self) -> u64 {
        self.admitted
            .saturating_add(self.blocked_occupied)
            .saturating_add(self.blocked_simultaneous)
            .saturating_add(self.unreachable)
            .saturating_add(self.refused)
    }

    /// Whether every offered intent reached exactly one terminal bucket.
    #[must_use]
    pub const fn reconciles(self) -> bool {
        self.offered == self.accounted()
    }

    /// Charges one decision to its exhaustive terminal bucket.
    const fn charge(&mut self, disposition: Disposition) {
        match disposition {
            Disposition::Admitted { .. } => {
                self.admitted = self.admitted.saturating_add(1);
            }
            Disposition::BlockedOccupied { .. } => {
                self.blocked_occupied = self.blocked_occupied.saturating_add(1);
            }
            Disposition::BlockedSimultaneous { .. } => {
                self.blocked_simultaneous = self.blocked_simultaneous.saturating_add(1);
            }
            Disposition::Unreachable => {
                self.unreachable = self.unreachable.saturating_add(1);
            }
            Disposition::Refused(_) => {
                self.refused = self.refused.saturating_add(1);
            }
        }
    }

    /// Adds one reconciled minute to the scheduler's running counters.
    const fn absorb(&mut self, other: Self) {
        self.offered = self.offered.saturating_add(other.offered);
        self.admitted = self.admitted.saturating_add(other.admitted);
        self.blocked_occupied = self.blocked_occupied.saturating_add(other.blocked_occupied);
        self.blocked_simultaneous = self
            .blocked_simultaneous
            .saturating_add(other.blocked_simultaneous);
        self.unreachable = self.unreachable.saturating_add(other.unreachable);
        self.refused = self.refused.saturating_add(other.refused);
    }
}

/// One accepted minute's fixed-capacity, canonically ordered result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MinuteSchedule {
    entry_micros: i64,
    decisions: [Option<Decision>; MAX_INTENTS_PER_MINUTE],
    counters: Counters,
    occupied_through_micros: Option<i64>,
}

impl MinuteSchedule {
    /// Entry timestamp shared by every intent in this result.
    #[must_use]
    pub const fn entry_micros(&self) -> i64 {
        self.entry_micros
    }

    /// Decisions in ascending priority then digest order.
    pub fn decisions(&self) -> impl Iterator<Item = &Decision> {
        self.decisions.iter().flatten()
    }

    /// This minute's reconciliation counters.
    #[must_use]
    pub const fn counters(&self) -> Counters {
        self.counters
    }

    /// Inclusive global occupancy after this minute, or `None` when flat.
    #[must_use]
    pub const fn occupied_through_micros(&self) -> Option<i64> {
        self.occupied_through_micros
    }
}

/// Why a whole minute was refused before any intent could be scheduled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MinuteRefusalKind {
    /// More than [`MAX_INTENTS_PER_MINUTE`] intents were supplied.
    TooManyIntents {
        /// Number actually supplied.
        offered: u64,
        /// Fixed maximum accepted by this version.
        maximum: u64,
    },
    /// The scheduler had already processed this timestamp or a later one.
    MinuteNotIncreasing {
        /// Last successfully processed entry timestamp.
        previous_micros: i64,
        /// Timestamp the refused batch tried to process.
        offered_micros: i64,
    },
    /// More than one constituent claimed the same ordering identity.
    DuplicateOrderingKey {
        /// Repeated one-based priority.
        priority: u16,
        /// Repeated digest; no caller order was used to choose between copies.
        strategy_digest: StrategyDigest,
    },
    /// A priority fell outside the version-one per-rung rank range.
    PriorityOutOfRange {
        /// Invalid priority; valid values are 1 through 25 inclusive within
        /// the constituent's signal rung.
        priority: u16,
        /// Strategy that supplied it.
        strategy_digest: StrategyDigest,
    },
    /// A constituent named an instrument outside the two-instrument sweep.
    InstrumentNotSweepable {
        /// Priority of the refused constituent.
        priority: u16,
        /// Strategy that supplied it.
        strategy_digest: StrategyDigest,
    },
    /// A constituent named a rung outside [`SUPPORTED_RUNGS_MINUTES`].
    UnsupportedRung {
        /// Priority of the refused constituent.
        priority: u16,
        /// Strategy that supplied it.
        strategy_digest: StrategyDigest,
        /// Unsupported rung in minutes.
        rung_minutes: u16,
    },
}

/// A whole-minute refusal and the accounting for every rejected intent.
///
/// A refused batch does not mutate scheduler state. Its own counters reconcile;
/// it is not silently folded into [`GlobalSinglePositionV1::counters`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MinuteRefusal {
    /// Exact refusal reason.
    pub kind: MinuteRefusalKind,
    /// Every offered intent counted as refused.
    pub counters: Counters,
}

/// Version one of the one-position-across-everything scheduler.
///
/// State is fixed: the last accepted minute, one inclusive occupancy boundary,
/// and six counters. It retains no intent or historical decision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GlobalSinglePositionV1 {
    last_minute_micros: Option<i64>,
    occupied_through_micros: Option<i64>,
    counters: Counters,
}

impl Default for GlobalSinglePositionV1 {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalSinglePositionV1 {
    /// A flat scheduler with no processed minute and zero counters.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            last_minute_micros: None,
            occupied_through_micros: None,
            counters: Counters {
                offered: 0,
                admitted: 0,
                blocked_occupied: 0,
                blocked_simultaneous: 0,
                unreachable: 0,
                refused: 0,
            },
        }
    }

    /// Last successfully processed entry timestamp.
    #[must_use]
    pub const fn last_minute_micros(&self) -> Option<i64> {
        self.last_minute_micros
    }

    /// Inclusive end of the live global position, if one remains recorded.
    #[must_use]
    pub const fn occupied_through_micros(&self) -> Option<i64> {
        self.occupied_through_micros
    }

    /// Reconciliation counters across accepted minute batches.
    #[must_use]
    pub const fn counters(&self) -> Counters {
        self.counters
    }

    /// Schedules every intent for one exact entry minute.
    ///
    /// A reachable intent is valid only when its occupancy ends at or after
    /// `entry_micros`. If an earlier position owns `entry_micros`, every valid
    /// intent is [`Disposition::BlockedOccupied`]. Otherwise the first valid
    /// intent in canonical order is admitted and every remaining valid intent
    /// is [`Disposition::BlockedSimultaneous`].
    ///
    /// # Errors
    ///
    /// [`MinuteRefusal`] when the batch is wider than the fixed bound, does not
    /// advance time, names an invalid priority/instrument/rung, or repeats an
    /// ordering key. No refusal changes state.
    ///
    /// Validation precedence is fixed in that order. Within a field class the
    /// smallest priority-and-digest key is reported, so shuffling invalid input
    /// cannot change either the refusal class or the constituent it names.
    #[expect(
        clippy::large_stack_arrays,
        reason = "the complete 8 × 25 minute and its result are fixed, compile-time bounded, allocation-free, and their combined live size is pinned by assertions below 64 KiB"
    )]
    pub fn schedule_minute(
        &mut self,
        entry_micros: i64,
        intents: &[Intent],
    ) -> Result<MinuteSchedule, MinuteRefusal> {
        let offered = usize_u64(intents.len());
        if intents.len() > MAX_INTENTS_PER_MINUTE {
            return Err(minute_refusal(
                MinuteRefusalKind::TooManyIntents {
                    offered,
                    maximum: usize_u64(MAX_INTENTS_PER_MINUTE),
                },
                offered,
            ));
        }
        if let Some(previous_micros) = self.last_minute_micros
            && entry_micros <= previous_micros
        {
            return Err(minute_refusal(
                MinuteRefusalKind::MinuteNotIncreasing {
                    previous_micros,
                    offered_micros: entry_micros,
                },
                offered,
            ));
        }
        if let Some((priority, strategy_digest)) = invalid_priority(intents) {
            return Err(minute_refusal(
                MinuteRefusalKind::PriorityOutOfRange {
                    priority,
                    strategy_digest,
                },
                offered,
            ));
        }
        if let Some((priority, strategy_digest)) = unsweepable_instrument(intents) {
            return Err(minute_refusal(
                MinuteRefusalKind::InstrumentNotSweepable {
                    priority,
                    strategy_digest,
                },
                offered,
            ));
        }
        if let Some((priority, strategy_digest, rung_minutes)) = unsupported_rung(intents) {
            return Err(minute_refusal(
                MinuteRefusalKind::UnsupportedRung {
                    priority,
                    strategy_digest,
                    rung_minutes,
                },
                offered,
            ));
        }
        if let Some((priority, strategy_digest)) = duplicate_ordering_key(intents) {
            return Err(minute_refusal(
                MinuteRefusalKind::DuplicateOrderingKey {
                    priority,
                    strategy_digest,
                },
                offered,
            ));
        }

        let ordered = canonical_order(intents);
        let prior_occupancy = self
            .occupied_through_micros
            .filter(|&through| entry_micros <= through);
        let mut occupied_after = prior_occupancy;
        let mut admitted: Option<StrategyDigest> = None;
        let mut decisions = [None; MAX_INTENTS_PER_MINUTE];
        let mut counters = Counters {
            offered,
            ..Counters::default()
        };

        for (slot, intent) in decisions.iter_mut().zip(ordered.into_iter().flatten()) {
            let disposition = decide(intent, entry_micros, prior_occupancy, admitted);
            if let Disposition::Admitted {
                occupied_through_micros,
            } = disposition
            {
                admitted = Some(intent.constituent.strategy_digest);
                occupied_after = Some(occupied_through_micros);
            }
            counters.charge(disposition);
            *slot = Some(Decision {
                constituent: intent.constituent,
                disposition,
            });
        }

        self.last_minute_micros = Some(entry_micros);
        self.occupied_through_micros = occupied_after;
        self.counters.absorb(counters);

        Ok(MinuteSchedule {
            entry_micros,
            decisions,
            counters,
            occupied_through_micros: occupied_after,
        })
    }
}

/// One terminal decision without any state mutation.
fn decide(
    intent: Intent,
    entry_micros: i64,
    prior_occupancy: Option<i64>,
    admitted: Option<StrategyDigest>,
) -> Disposition {
    let occupied_through_micros = match intent.evidence {
        Evidence::Reachable {
            occupied_through_micros,
        } => occupied_through_micros,
        Evidence::Unreachable => return Disposition::Unreachable,
        Evidence::Refused => return Disposition::Refused(IntentRefusal::Upstream),
    };
    if occupied_through_micros < entry_micros {
        return Disposition::Refused(IntentRefusal::OccupancyEndsBeforeEntry);
    }
    if let Some(through) = prior_occupancy {
        return Disposition::BlockedOccupied {
            occupied_through_micros: through,
        };
    }
    if let Some(winner) = admitted {
        return Disposition::BlockedSimultaneous { admitted: winner };
    }
    Disposition::Admitted {
        occupied_through_micros,
    }
}

/// Canonical fixed-width ordering workspace, selected in at most 200 × 200 probes.
#[expect(
    clippy::large_stack_arrays,
    reason = "two fixed 200-slot buffers give deterministic allocation-free ordering; their combined size is compile-time pinned below 64 KiB"
)]
fn canonical_order(intents: &[Intent]) -> [Option<Intent>; MAX_INTENTS_PER_MINUTE] {
    let mut remaining = [None; MAX_INTENTS_PER_MINUTE];
    for (slot, intent) in remaining.iter_mut().zip(intents.iter().copied()) {
        *slot = Some(intent);
    }
    let mut ordered = [None; MAX_INTENTS_PER_MINUTE];
    for destination in &mut ordered {
        let mut best: Option<Intent> = None;
        for candidate in remaining.iter().flatten().copied() {
            if best.is_none_or(|held| earlier(candidate, held)) {
                best = Some(candidate);
            }
        }
        let Some(best) = best else {
            break;
        };
        for candidate in &mut remaining {
            if *candidate == Some(best) {
                *candidate = None;
                break;
            }
        }
        *destination = Some(best);
    }
    ordered
}

/// Whether `left` precedes `right` under the complete scheduling key.
fn earlier(left: Intent, right: Intent) -> bool {
    (left.constituent.priority, left.constituent.strategy_digest)
        < (
            right.constituent.priority,
            right.constituent.strategy_digest,
        )
}

/// The smallest out-of-range priority-and-digest key, if one exists.
fn invalid_priority(intents: &[Intent]) -> Option<(u16, StrategyDigest)> {
    let mut invalid: Option<(u16, StrategyDigest)> = None;
    for intent in intents {
        let key = (
            intent.constituent.priority,
            intent.constituent.strategy_digest,
        );
        if (key.0 == 0 || usize::from(key.0) > MAX_PRIORITY_PER_RUNG)
            && invalid.is_none_or(|held| key < held)
        {
            invalid = Some(key);
        }
    }
    invalid
}

/// The canonically first constituent outside the two-instrument engine surface.
fn unsweepable_instrument(intents: &[Intent]) -> Option<(u16, StrategyDigest)> {
    let mut invalid: Option<(u16, StrategyDigest, InstrumentKey)> = None;
    for intent in intents {
        let candidate = (
            intent.constituent.priority,
            intent.constituent.strategy_digest,
            intent.constituent.instrument,
        );
        if !candidate.2.is_sweepable() && invalid.is_none_or(|held| candidate < held) {
            invalid = Some(candidate);
        }
    }
    invalid.map(|(priority, strategy_digest, _)| (priority, strategy_digest))
}

/// The canonically first constituent outside the eight supported rungs.
fn unsupported_rung(intents: &[Intent]) -> Option<(u16, StrategyDigest, u16)> {
    let mut invalid: Option<(u16, StrategyDigest, u16)> = None;
    for intent in intents {
        let candidate = (
            intent.constituent.priority,
            intent.constituent.strategy_digest,
            intent.constituent.rung_minutes,
        );
        if !supported_rung(candidate.2) && invalid.is_none_or(|held| candidate < held) {
            invalid = Some(candidate);
        }
    }
    invalid
}

/// Constant eight-way rung membership without a data-dependent scan.
const fn supported_rung(rung_minutes: u16) -> bool {
    matches!(rung_minutes, 1 | 2 | 3 | 5 | 10 | 15 | 30 | 60)
}

/// The smallest repeated priority-and-digest key, if one exists.
fn duplicate_ordering_key(intents: &[Intent]) -> Option<(u16, StrategyDigest)> {
    let mut duplicate: Option<(u16, StrategyDigest)> = None;
    for (at, left) in intents.iter().enumerate() {
        let left_key = (left.constituent.priority, left.constituent.strategy_digest);
        for right in intents.iter().skip(at.saturating_add(1)) {
            let right_key = (
                right.constituent.priority,
                right.constituent.strategy_digest,
            );
            if left_key == right_key && duplicate.is_none_or(|held| left_key < held) {
                duplicate = Some(left_key);
            }
        }
    }
    duplicate
}

/// One whole-batch refusal with every offered intent reconciled as refused.
const fn minute_refusal(kind: MinuteRefusalKind, offered: u64) -> MinuteRefusal {
    MinuteRefusal {
        kind,
        counters: Counters {
            offered,
            admitted: 0,
            blocked_occupied: 0,
            blocked_simultaneous: 0,
            unreachable: 0,
            refused: offered,
        },
    }
}

/// A platform-sized bounded count rendered in the scheduler's counter width.
fn usize_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
#[expect(
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test assertions need observable failures; exhaustive fixtures remain at four strategies"
)]
mod tests {
    use super::{
        Constituent, Counters, Decision, Disposition, Evidence, GlobalSinglePositionV1, Intent,
        IntentRefusal, MAX_INTENTS_PER_MINUTE, MAX_PRIORITY_PER_RUNG, MinuteRefusalKind,
        SUPPORTED_RUNGS_MINUTES, StrategyDigest,
    };
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use costs::fill::Direction;

    fn instrument(bank: bool) -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, if bank { "BANKNIFTY" } else { "NIFTY" })
            .expect("both portfolio fixtures are swept NSE indices")
    }

    fn constituent(
        digest: u8,
        priority: u16,
        bank: bool,
        direction: Direction,
        rung_minutes: u16,
    ) -> Constituent {
        Constituent {
            priority,
            strategy_digest: StrategyDigest::new([digest; 32]),
            instrument: instrument(bank),
            direction,
            rung_minutes,
        }
    }

    fn intent(constituent: Constituent, evidence: Evidence) -> Intent {
        Intent {
            constituent,
            evidence,
        }
    }

    fn reachable(constituent: Constituent, through: i64) -> Intent {
        intent(
            constituent,
            Evidence::Reachable {
                occupied_through_micros: through,
            },
        )
    }

    fn decisions(schedule: &super::MinuteSchedule) -> Vec<Decision> {
        schedule.decisions().copied().collect()
    }

    fn permutations(values: &mut [Intent], at: usize, out: &mut Vec<Vec<Intent>>) {
        if at == values.len() {
            out.push(values.to_vec());
            return;
        }
        for selected in at..values.len() {
            values.swap(at, selected);
            permutations(values, at.saturating_add(1), out);
            values.swap(at, selected);
        }
    }

    #[test]
    fn every_shuffle_has_the_same_priority_then_digest_answer() {
        let mut source = [
            reachable(constituent(4, 2, false, Direction::Long, 1), 4),
            reachable(constituent(3, 1, true, Direction::Short, 60), 3),
            reachable(constituent(1, 1, false, Direction::Short, 15), 2),
            reachable(constituent(2, 1, true, Direction::Long, 5), 5),
        ];
        let mut shuffled = Vec::new();
        permutations(&mut source, 0, &mut shuffled);
        assert_eq!(shuffled.len(), 24, "all four-strategy permutations");

        let mut baseline_scheduler = GlobalSinglePositionV1::new();
        let baseline = baseline_scheduler
            .schedule_minute(0, &source)
            .expect("four unique intents fit");
        for order in shuffled {
            let mut scheduler = GlobalSinglePositionV1::new();
            let got = scheduler
                .schedule_minute(0, &order)
                .expect("the permutation cannot change validity");
            assert_eq!(got, baseline, "input order became an undeclared key");
        }

        let got = decisions(&baseline);
        let keys: Vec<(u16, u8)> = got
            .iter()
            .map(|decision| {
                (
                    decision.constituent.priority,
                    decision.constituent.strategy_digest.bytes()[0],
                )
            })
            .collect();
        assert_eq!(keys, [(1, 1), (1, 2), (1, 3), (2, 4)]);
        assert!(matches!(got[0].disposition, Disposition::Admitted { .. }));
        assert!(got[1..].iter().all(|decision| matches!(
            decision.disposition,
            Disposition::BlockedSimultaneous { admitted }
                if admitted == StrategyDigest::new([1; 32])
        )));
    }

    #[test]
    fn one_global_lock_crosses_direction_instrument_and_rung() {
        let opening = reachable(constituent(1, 1, false, Direction::Long, 1), 2);
        let mut scheduler = GlobalSinglePositionV1::new();
        let first = scheduler
            .schedule_minute(0, &[opening])
            .expect("the opening minute fits");
        assert_eq!(first.counters().admitted, 1);

        let while_open = [
            reachable(constituent(2, 1, true, Direction::Short, 60), 4),
            reachable(constituent(3, 2, false, Direction::Short, 15), 3),
        ];
        let blocked = scheduler
            .schedule_minute(1, &while_open)
            .expect("both distinct intents fit");
        assert_eq!(blocked.counters().blocked_occupied, 2);
        assert!(blocked.decisions().all(|decision| matches!(
            decision.disposition,
            Disposition::BlockedOccupied {
                occupied_through_micros: 2
            }
        )));
        assert_eq!(scheduler.occupied_through_micros(), Some(2));
    }

    #[test]
    fn the_exit_minute_blocks_and_the_following_minute_enters() {
        let first = reachable(constituent(1, 1, false, Direction::Long, 1), 12);
        let next = reachable(constituent(2, 1, true, Direction::Short, 60), 20);
        let mut scheduler = GlobalSinglePositionV1::new();
        scheduler
            .schedule_minute(10, &[first])
            .expect("opening intent");

        let on_exit = scheduler
            .schedule_minute(12, &[next])
            .expect("exit-minute intent");
        assert!(matches!(
            decisions(&on_exit)[0].disposition,
            Disposition::BlockedOccupied {
                occupied_through_micros: 12
            }
        ));

        let after_exit = scheduler
            .schedule_minute(13, &[next])
            .expect("following-minute intent");
        assert!(matches!(
            decisions(&after_exit)[0].disposition,
            Disposition::Admitted {
                occupied_through_micros: 20
            }
        ));
    }

    #[test]
    fn unreachable_refused_and_invalid_intents_are_never_silent() {
        let intents = [
            intent(
                constituent(1, 1, false, Direction::Long, 1),
                Evidence::Unreachable,
            ),
            intent(
                constituent(2, 2, true, Direction::Short, 5),
                Evidence::Refused,
            ),
            reachable(constituent(3, 3, true, Direction::Long, 60), -1),
        ];
        let mut scheduler = GlobalSinglePositionV1::new();
        let got = scheduler
            .schedule_minute(0, &intents)
            .expect("three distinct intents fit");
        let got = decisions(&got);
        assert_eq!(got[0].disposition, Disposition::Unreachable);
        assert_eq!(
            got[1].disposition,
            Disposition::Refused(IntentRefusal::Upstream)
        );
        assert_eq!(
            got[2].disposition,
            Disposition::Refused(IntentRefusal::OccupancyEndsBeforeEntry)
        );
    }

    #[test]
    #[expect(
        clippy::large_stack_arrays,
        reason = "the 201-row adversarial fixture must cross the exact 200-row compiled boundary"
    )]
    fn wider_than_all_eight_top_twenty_fives_refuses_whole_and_changes_no_state() {
        let repeated = reachable(constituent(1, 1, false, Direction::Long, 1), 2);
        let too_many = [repeated; MAX_INTENTS_PER_MINUTE + 1];
        let mut scheduler = GlobalSinglePositionV1::new();
        let refusal = scheduler
            .schedule_minute(0, &too_many)
            .expect_err("two hundred and one intents must refuse before duplicate checking");
        assert_eq!(
            refusal.kind,
            MinuteRefusalKind::TooManyIntents {
                offered: 201,
                maximum: 200
            }
        );
        assert!(refusal.counters.reconciles());
        assert_eq!(refusal.counters.refused, 201);
        assert_eq!(scheduler, GlobalSinglePositionV1::new());

        let retry = scheduler
            .schedule_minute(0, &[repeated])
            .expect("a refused batch leaves its minute reusable");
        assert_eq!(retry.counters().admitted, 1);
    }

    #[test]
    fn duplicate_key_and_nonincreasing_time_refuse_without_mutation() {
        let a = reachable(constituent(7, 1, false, Direction::Long, 1), 4);
        let b = reachable(constituent(7, 1, true, Direction::Short, 60), 9);
        let mut scheduler = GlobalSinglePositionV1::new();
        let duplicate = scheduler
            .schedule_minute(0, &[a, b])
            .expect_err("equal scheduling keys need no hidden third key");
        assert!(matches!(
            duplicate.kind,
            MinuteRefusalKind::DuplicateOrderingKey {
                priority: 1,
                strategy_digest
            } if strategy_digest == StrategyDigest::new([7; 32])
        ));
        assert_eq!(duplicate.counters.refused, 2);
        assert!(duplicate.counters.reconciles());
        assert_eq!(scheduler, GlobalSinglePositionV1::new());

        scheduler
            .schedule_minute(3, &[a])
            .expect("first chronological minute");
        let snapshot = scheduler;
        let backward = scheduler
            .schedule_minute(3, &[b])
            .expect_err("the whole minute must be supplied in one call");
        assert!(matches!(
            backward.kind,
            MinuteRefusalKind::MinuteNotIncreasing {
                previous_micros: 3,
                offered_micros: 3
            }
        ));
        assert_eq!(scheduler, snapshot);
    }

    #[test]
    fn invalid_surface_refuses_atomically_in_fixed_precedence() {
        let invalid_instrument =
            InstrumentKey::index(Exchange::Nse, "INDIAVIX").expect("the reference index exists");
        assert!(!invalid_instrument.is_sweepable());

        let mut bad_priority = reachable(constituent(9, 0, false, Direction::Long, 1), 4);
        bad_priority.constituent.instrument = invalid_instrument;
        bad_priority.constituent.rung_minutes = 4;

        let mut bad_instrument = reachable(constituent(1, 1, false, Direction::Long, 1), 4);
        bad_instrument.constituent.instrument = invalid_instrument;
        bad_instrument.constituent.rung_minutes = 4;

        let bad_rung = reachable(constituent(2, 1, false, Direction::Short, 4), 4);
        let duplicate_a = reachable(constituent(3, 2, false, Direction::Long, 1), 4);
        let duplicate_b = reachable(constituent(3, 2, true, Direction::Short, 60), 5);
        let mut all = [
            duplicate_b,
            bad_rung,
            bad_instrument,
            duplicate_a,
            bad_priority,
        ];
        let mut shuffled = Vec::new();
        permutations(&mut all, 0, &mut shuffled);
        let scheduler = GlobalSinglePositionV1::new();
        for order in shuffled {
            let mut attempt = scheduler;
            let refusal = attempt
                .schedule_minute(0, &order)
                .expect_err("priority validation is first after capacity and time");
            assert_eq!(
                refusal.kind,
                MinuteRefusalKind::PriorityOutOfRange {
                    priority: 0,
                    strategy_digest: StrategyDigest::new([9; 32])
                }
            );
            assert!(refusal.counters.reconciles());
            assert_eq!(attempt, scheduler, "a batch refusal must be atomic");
        }

        let mut attempt = scheduler;
        let instrument = attempt
            .schedule_minute(0, &[bad_rung, duplicate_b, bad_instrument, duplicate_a])
            .expect_err("instrument validation precedes rung and duplicate validation");
        assert_eq!(
            instrument.kind,
            MinuteRefusalKind::InstrumentNotSweepable {
                priority: 1,
                strategy_digest: StrategyDigest::new([1; 32])
            }
        );
        assert_eq!(attempt, scheduler);

        let mut attempt = scheduler;
        let rung = attempt
            .schedule_minute(0, &[duplicate_b, bad_rung, duplicate_a])
            .expect_err("rung validation precedes duplicate validation");
        assert_eq!(
            rung.kind,
            MinuteRefusalKind::UnsupportedRung {
                priority: 1,
                strategy_digest: StrategyDigest::new([2; 32]),
                rung_minutes: 4
            }
        );
        assert_eq!(attempt, scheduler);

        let mut attempt = scheduler;
        let duplicate = attempt
            .schedule_minute(0, &[duplicate_b, duplicate_a])
            .expect_err("duplicate validation follows the field surface");
        assert!(matches!(
            duplicate.kind,
            MinuteRefusalKind::DuplicateOrderingKey {
                priority: 2,
                strategy_digest
            } if strategy_digest == StrategyDigest::new([3; 32])
        ));
        assert_eq!(attempt, scheduler);

        let upper = reachable(constituent(8, 26, false, Direction::Long, 1), 2);
        let mut attempt = scheduler;
        assert!(matches!(
            attempt
                .schedule_minute(0, &[upper])
                .expect_err("priority twenty-six exceeds the constituent ceiling")
                .kind,
            MinuteRefusalKind::PriorityOutOfRange { priority: 26, .. }
        ));
        assert_eq!(attempt, scheduler);
    }

    #[test]
    fn every_supported_rung_and_both_priority_boundaries_are_admitted() {
        assert_eq!(SUPPORTED_RUNGS_MINUTES, [1, 2, 3, 5, 10, 15, 30, 60]);
        assert_eq!(MAX_PRIORITY_PER_RUNG, 25);
        assert_eq!(MAX_INTENTS_PER_MINUTE, 200);
        let intents = [
            reachable(constituent(1, 1, false, Direction::Long, 1), 2),
            reachable(constituent(2, 2, true, Direction::Short, 2), 3),
            reachable(constituent(3, 3, false, Direction::Short, 3), 4),
            reachable(constituent(4, 4, true, Direction::Long, 5), 6),
            reachable(constituent(5, 5, false, Direction::Long, 10), 11),
            reachable(constituent(6, 6, true, Direction::Short, 15), 16),
            reachable(constituent(7, 7, false, Direction::Short, 30), 31),
            reachable(constituent(8, 25, true, Direction::Long, 60), 61),
        ];
        let mut scheduler = GlobalSinglePositionV1::new();
        let got = scheduler
            .schedule_minute(1, &intents)
            .expect("all eight shipped rungs and priorities 1 through 25 are valid");
        assert_eq!(got.counters().offered, 8);
        assert_eq!(got.counters().admitted, 1);
        assert_eq!(got.counters().blocked_simultaneous, 7);
        assert_eq!(got.counters().refused, 0);
        assert!(got.counters().reconciles());
        assert!(scheduler.counters().reconciles());
    }

    #[test]
    fn all_eight_per_rung_top_twenty_fives_reach_one_global_arbitration() {
        let mut intents = Vec::with_capacity(MAX_INTENTS_PER_MINUTE);
        for (rung_index, rung_minutes) in SUPPORTED_RUNGS_MINUTES.into_iter().enumerate() {
            for priority in 1_u16..=25 {
                let ordinal = rung_index
                    .saturating_mul(MAX_PRIORITY_PER_RUNG)
                    .saturating_add(usize::from(priority));
                let digest = u8::try_from(ordinal).expect("the complete surface is at most 200");
                intents.push(reachable(
                    constituent(
                        digest,
                        priority,
                        rung_index % 2 == 1,
                        if ordinal % 2 == 0 {
                            Direction::Long
                        } else {
                            Direction::Short
                        },
                        rung_minutes,
                    ),
                    10,
                ));
            }
        }
        intents.reverse();

        let mut scheduler = GlobalSinglePositionV1::new();
        let got = scheduler
            .schedule_minute(0, &intents)
            .expect("all 8 × 25 unique constituents are the complete legal minute batch");
        assert_eq!(got.decisions().count(), MAX_INTENTS_PER_MINUTE);
        assert_eq!(got.counters().offered, 200);
        assert_eq!(got.counters().admitted, 1);
        assert_eq!(got.counters().blocked_simultaneous, 199);
        assert_eq!(got.counters().accounted(), 200);
        assert!(got.counters().reconciles());
    }

    /// Exhaust every absent/reachable/unreachable/refused shape for four
    /// strategies at each of six entry minutes, under flat, expiring-now and
    /// still-open portfolio state: 6 × 3 × 4^4 = 4,608 schedules.
    #[test]
    fn four_strategies_across_six_minutes_exhaust_every_terminal_shape() {
        const SHAPES: usize = 4 * 4 * 4 * 4;
        let members = [
            constituent(1, 2, false, Direction::Long, 1),
            constituent(2, 1, true, Direction::Short, 60),
            constituent(3, 1, false, Direction::Short, 15),
            constituent(4, 3, true, Direction::Long, 5),
        ];

        for minute in 0_i64..6 {
            for prior_end in [None, Some(minute), Some(minute.saturating_add(2))] {
                for encoded in 0..SHAPES {
                    let mut word = encoded;
                    let mut offered = Vec::new();
                    let mut reachable_count = 0_u64;
                    let mut unreachable_count = 0_u64;
                    let mut refused_count = 0_u64;
                    for member in members {
                        match word % 4 {
                            0 => {}
                            1 => {
                                offered.push(reachable(
                                    member,
                                    minute.saturating_add(i64::from(member.rung_minutes)),
                                ));
                                reachable_count = reachable_count.saturating_add(1);
                            }
                            2 => {
                                offered.push(intent(member, Evidence::Unreachable));
                                unreachable_count = unreachable_count.saturating_add(1);
                            }
                            _ => {
                                offered.push(intent(member, Evidence::Refused));
                                refused_count = refused_count.saturating_add(1);
                            }
                        }
                        word /= 4;
                    }
                    offered.reverse();

                    let mut scheduler = GlobalSinglePositionV1 {
                        last_minute_micros: Some(minute.saturating_sub(1)),
                        occupied_through_micros: prior_end,
                        counters: Counters::default(),
                    };
                    let got = scheduler
                        .schedule_minute(minute, &offered)
                        .expect("four unique strategies fit and time advances");
                    let occupied = prior_end.is_some_and(|through| minute <= through);
                    let expected_admitted = u64::from(!occupied && reachable_count > 0);
                    let expected = Counters {
                        offered: u64::try_from(offered.len()).unwrap_or(u64::MAX),
                        admitted: expected_admitted,
                        blocked_occupied: if occupied { reachable_count } else { 0 },
                        blocked_simultaneous: if occupied {
                            0
                        } else {
                            reachable_count.saturating_sub(expected_admitted)
                        },
                        unreachable: unreachable_count,
                        refused: refused_count,
                    };
                    assert_eq!(got.counters(), expected, "minute={minute} shape={encoded}");
                    assert!(
                        got.counters().reconciles(),
                        "minute={minute} shape={encoded}"
                    );
                    assert_eq!(
                        got.decisions()
                            .filter(|decision| {
                                matches!(decision.disposition, Disposition::Admitted { .. })
                            })
                            .count(),
                        usize::try_from(expected_admitted).unwrap_or(usize::MAX),
                        "at most one valid intent can win minute={minute} shape={encoded}"
                    );
                    let ordered: Vec<(u16, StrategyDigest)> = got
                        .decisions()
                        .map(|decision| {
                            (
                                decision.constituent.priority,
                                decision.constituent.strategy_digest,
                            )
                        })
                        .collect();
                    assert!(
                        ordered.windows(2).all(|pair| pair[0] < pair[1]),
                        "priority then digest must totally order minute={minute} shape={encoded}: {ordered:?}"
                    );
                }
            }
        }
    }

    #[test]
    #[expect(
        clippy::large_stack_arrays,
        reason = "constructing the exact fixed-width empty result is the property under test"
    )]
    fn scheduler_state_and_results_are_fixed_width_values() {
        assert!(!core::mem::needs_drop::<GlobalSinglePositionV1>());
        assert!(!core::mem::needs_drop::<super::MinuteSchedule>());
        assert_eq!(
            super::MinuteSchedule {
                entry_micros: 0,
                decisions: [None; MAX_INTENTS_PER_MINUTE],
                counters: Counters::default(),
                occupied_through_micros: None,
            }
            .decisions()
            .count(),
            0
        );
    }
}
