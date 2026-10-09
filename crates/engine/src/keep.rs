//! What a walk RETAINS, which is not the same question as how far it walks.
//!
//! # The measurement this module exists for
//!
//! A full-range sweep — `zerodha NIFTY 2019-12 .. 2026-08` — reached **7.4 GB
//! resident and was killed at exit 137**, and the cause was measured rather
//! than guessed: it is not the combination ladder. Every one of the sixteen
//! ladders in two eight-rung runs had finished or halted **within two and a
//! half minutes**, at a peak of 8.91 GB across all eight concurrent rungs with
//! 6.35 GB of that RETAINED. What ran afterwards — ranking 15 to 17 million
//! retained survivors per rung, eight rungs at once — is where the process
//! died.
//!
//! `crate::DEFAULT_CEILING`'s own table already names the term: of the five
//! summands in a level's live set, `56 · Σ_{j<k} S_j` is *"the retained
//! survivors of every earlier level"*, and its comment ends **"it exists
//! because `Sweep::levels` keeps every survivor for a ranker that wants only
//! the top `keep`."** This module is the other half of that sentence.
//!
//! # What is bounded here, and what is not
//!
//! [`Streamed`] bounds what the ENGINE holds. A walk that streams keeps two
//! levels alive at once — the one being joined from and the one being built —
//! instead of every level to the end, and records each finished level as a
//! [`Tally`], which is a [`crate::Frontier`] with its survivor vector replaced
//! by that vector's length. The survivors themselves are handed to the caller
//! at the level boundary and then dropped.
//!
//! What a CALLER holds is the caller's own sink. `runner::rank` scores by
//! forward edge, which this crate cannot compute and must not pretend to, and
//! feeds its own heap from the same boundary. This module used to offer a
//! ready-made fixed-capacity retention ordered by support, `Best`; no
//! production path ever constructed it (D-0762), its admission was O(log cap)
//! and never timed, and D-4480 removed it with its tests rather than keep an
//! unmeasured cost on a path nothing runs.
//!
//! # NOT A DEPTH PARAMETER, and the distinction is testable
//!
//! `CLAUDE.md` §6 bans a `k` on the sweep, and a retention will be read as
//! one, so: the ladder still walks upward from k=1 and its normal completion is
//! still the frequent frontier becoming empty. Its existing candidate or pair
//! budget may refuse first, loudly, exactly as on the retaining path. Nothing
//! the sink does is consulted by the join, by the subset prune, by the support
//! test or by either budget. A sink that keeps nothing and one that keeps every
//! survivor walk the **same ladder to the same depth and produce the same
//! [`Tally`] for every level** — which is not an argument, it is
//! `engine::keep::a_sink_that_keeps_nothing_and_one_that_keeps_everything_walk_the_same_ladder`.
//!
//! # The narrowing is REPORTED, never absorbed
//!
//! `CLAUDE.md` §4 bans a fallback that hides a failure. A sink that drops a
//! survivor has genuinely narrowed what the caller holds, so the number of
//! survivors handed over is carried out of the walk beside the result:
//! [`Streamed::streamed`] is every survivor the engine handed over and dropped,
//! and a caller that keeps fewer can state the difference against it.

use crate::{Excluded, Frontier, Halt};

/// One level's five exits, WITHOUT the survivors themselves.
///
/// # Why a second type rather than an emptied [`Frontier`]
///
/// A `Frontier` whose `frequent` had been emptied would still answer
/// [`Frontier::reconciles`], and it would answer FALSE — the five-exit identity
/// counts survivors, so a level drained of them reads as a level that lost
/// candidates. `runner::report` renders exactly that as `LOST A CANDIDATE`, a
/// defect banner. Handing a caller a value that reports a bug where there is
/// none is the failure wearing a success's clothes `CLAUDE.md` §4 bans, said
/// backwards.
///
/// So the survivors become a COUNT and the type says so in its name. Every
/// other field is carried across unchanged and [`Self::reconciles`] asks the
/// same question `Frontier::reconciles` asks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tally {
    /// The level. `k == 1` is single conditions.
    pub k: u32,
    /// How many combinations met `min_hits`. This is `frequent.len()` of the
    /// level the walk built, and it is the only thing kept from that vector.
    pub survivors: u64,
    /// Candidates the join produced at this level. See [`Frontier::generated`].
    pub generated: u64,
    /// Candidates already seen at this level. See [`Frontier::duplicates`].
    pub duplicates: u64,
    /// Positions refused before anything else was measured. See
    /// [`Frontier::excluded`].
    pub excluded: u64,
    /// Candidates the subset prune removed unevaluated. See
    /// [`Frontier::pruned`].
    pub pruned: u64,
    /// Candidates evaluated against the bars and found too rare. See
    /// [`Frontier::infrequent`].
    pub infrequent: u64,
}

impl Tally {
    /// The five exits of a level the walk has just finished with.
    #[must_use]
    pub fn of(level: &Frontier) -> Self {
        Self {
            k: level.k,
            survivors: crate::len_u64(level.frequent.len()),
            generated: level.generated,
            duplicates: level.duplicates,
            excluded: level.excluded,
            pruned: level.pruned,
            infrequent: level.infrequent,
        }
    }

    /// Every candidate the join produced ended in exactly one of five places.
    ///
    /// The same identity [`Frontier::reconciles`] states, asked of a level whose
    /// survivors are gone and whose COUNT of them is not.
    #[must_use]
    pub fn reconciles(&self) -> bool {
        let accounted = self
            .duplicates
            .saturating_add(self.excluded)
            .saturating_add(self.pruned)
            .saturating_add(self.infrequent)
            .saturating_add(self.survivors);
        accounted == self.generated
    }
}

/// The outcome of a walk that did not retain its survivors.
///
/// Field for field this is [`crate::Sweep`] with `Vec<Frontier>` replaced by
/// `Vec<Tally>` and one number added. A depth-25 walk therefore holds
/// `25 × size_of::<Tally>()` bytes of level record whatever the frontier widths
/// were, against the `56 · Σ_j |F_j|` a retaining walk holds — the term
/// `crate::DEFAULT_CEILING`'s table names as the one that matters.
/// `engine::keep::a_streamed_walk_retains_no_survivor_at_all` measures the
/// difference on a walked ladder rather than asserting it.
#[derive(Clone, Debug, Default)]
pub struct Streamed {
    /// One entry per level walked, in order. Its length **is** the depth
    /// reached, which is a result and never an input.
    pub levels: Vec<Tally>,
    /// Positions excluded before k=1, named per D-0080. Bounded by the
    /// vocabulary rather than by the data, so this one IS retained whole.
    pub excluded: Vec<Excluded>,
    /// Bars loaded. Support fractions are over this.
    pub bars: u64,
    /// The threshold actually applied, echoed so a result is self-describing.
    pub min_hits: u64,
    /// `None` when the ladder went extinct. See [`crate::Sweep::halted`].
    pub halted: Option<Halt>,
    /// **Every survivor this walk handed to the sink and then dropped.**
    ///
    /// This is the loud number `CLAUDE.md` §4 asks for. A caller that ignored
    /// the sink is holding none of these combinations, and the difference
    /// between that and a sweep which found nothing is exactly this field. It
    /// is NOT a defect count and NOT a truncation of the search: every one of
    /// them was enumerated, evaluated against the bars and offered.
    pub streamed: u64,
}

impl Streamed {
    /// The depth the ladder reached before a level came back empty.
    ///
    /// Zero when no single condition met `min_hits`. The same question
    /// [`crate::Sweep::depth`] answers, over counts rather than vectors.
    ///
    /// O(1) for the reason [`crate::Sweep::depth`] gives: only the last level
    /// recorded can be empty (D-2304, AFG-04).
    #[must_use]
    pub fn depth(&self) -> usize {
        let ended_empty = self.levels.last().is_some_and(|l| l.survivors == 0);
        self.levels.len().saturating_sub(usize::from(ended_empty))
    }

    /// Did the ladder go extinct, which is the answer `CLAUDE.md` §6 asks for?
    ///
    /// False means a level breached a budget and the walk stopped short. See
    /// [`crate::Sweep::completed`].
    #[must_use]
    pub const fn completed(&self) -> bool {
        self.halted.is_none()
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Streamed, Tally};
    use crate::{Frontier, Itemset, Ladder, column::Column};
    use vocab::ConditionMask;

    /// One itemset at a named strength, on a bit of its own.
    fn item(hits: u64, bit: u32) -> Itemset {
        Itemset {
            mask: ConditionMask::default().with_bit(bit),
            hits,
        }
    }

    /// A [`Tally`] answers the five-exit identity the [`Frontier`] answers.
    #[test]
    fn a_tally_reconciles_exactly_when_its_frontier_does() {
        let level = Frontier {
            k: 3,
            frequent: vec![item(9, 0), item(8, 1)],
            generated: 10,
            duplicates: 1,
            excluded: 2,
            pruned: 3,
            infrequent: 2,
        };
        let tally = Tally::of(&level);
        assert_eq!(tally.k, 3);
        assert_eq!(tally.survivors, 2, "the vector becomes its length");
        assert_eq!(tally.generated, 10);
        assert_eq!(tally.duplicates, 1);
        assert_eq!(tally.excluded, 2);
        assert_eq!(tally.pruned, 3);
        assert_eq!(tally.infrequent, 2);
        assert!(level.reconciles() && tally.reconciles(), "1+2+3+2+2 == 10");

        let lost = Frontier {
            generated: 99,
            ..Frontier::default()
        };
        assert!(
            !lost.reconciles() && !Tally::of(&lost).reconciles(),
            "and a level that lost a candidate is still caught after the \
             survivors are gone -- which is the whole reason this is a second \
             type rather than an emptied Frontier"
        );
    }

    /// The derived shapes a caller will actually use.
    #[test]
    fn the_result_types_answer_depth_completion_and_the_derives() {
        let mut streamed = Streamed::default();
        assert_eq!(streamed.depth(), 0, "no level, no depth");
        assert!(streamed.completed(), "and nothing halted it");
        streamed.levels.push(Tally {
            k: 1,
            survivors: 4,
            ..Tally::default()
        });
        streamed.levels.push(Tally {
            k: 2,
            survivors: 0,
            ..Tally::default()
        });
        assert_eq!(
            streamed.depth(),
            1,
            "the empty level that ends the walk is recorded and is not depth"
        );
        streamed.halted = Some(crate::Halt::default());
        assert!(!streamed.completed(), "a halted walk is not a complete one");

        // The derives are part of the surface: a caller that clones a result or
        // prints one in a refusal must not be the first to find out.
        let copy = streamed.clone();
        assert_eq!(copy.levels, streamed.levels);
        assert!(!format!("{streamed:?}").is_empty());
    }

    /// **What a caller keeps is not a depth control**, which is `CLAUDE.md`
    /// §6's whole point.
    ///
    /// Two walks over one column, one handing every level to a sink that keeps
    /// nothing and one to a sink that keeps every survivor. Every level, every
    /// counter and the depth itself must be identical: retention is the
    /// CALLER's and is read by nothing inside the walk. This was asked of a cap
    /// of zero against a generous cap on `keep::Best` until D-4480 removed that
    /// type; the two extremes of any retention are these two sinks.
    #[test]
    fn a_sink_that_keeps_nothing_and_one_that_keeps_everything_walk_the_same_ladder() {
        let live: Vec<u32> = (0..8).collect();
        let spec: Vec<Vec<u32>> = (0..64_u32)
            .map(|bar| (0..8_u32).filter(|b| bar % (b + 2) != 0).collect())
            .collect();
        let masks: Vec<ConditionMask> = spec
            .iter()
            .map(|bits| {
                bits.iter()
                    .fold(ConditionMask::default(), |m, &b| m.with_bit(b))
            })
            .collect();
        let column = Column::from_rows(&masks);

        let walk = |keep: bool| {
            let mut kept: Vec<Itemset> = Vec::new();
            let mut offered: u64 = 0;
            let out = Ladder::with_min_hits(1).walk_column_streamed(
                &column,
                &live,
                &mut |level, _, _| {
                    offered = offered
                        .saturating_add(u64::try_from(level.frequent.len()).unwrap_or(u64::MAX));
                    if keep {
                        kept.extend_from_slice(&level.frequent);
                    }
                },
            );
            (out, kept, offered)
        };

        let (tight, none_kept, offered_tight) = walk(false);
        let (loose, all_kept, offered_loose) = walk(true);

        assert_eq!(tight.levels, loose.levels, "the same levels, exit for exit");
        assert_eq!(tight.depth(), loose.depth(), "and the same depth");
        assert_eq!(tight.bars, loose.bars);
        assert_eq!(tight.min_hits, loose.min_hits);
        assert_eq!(tight.halted, loose.halted);
        assert_eq!(tight.streamed, loose.streamed);
        assert!(
            tight.depth() > 2,
            "the fixture must climb, or it proves nothing"
        );

        assert!(
            none_kept.is_empty(),
            "a sink that keeps nothing keeps nothing"
        );
        assert_eq!(
            offered_tight, tight.streamed,
            "and was offered exactly what the loud number says was handed over"
        );
        assert_eq!(
            offered_loose, offered_tight,
            "both sinks saw the same offers"
        );
        assert_eq!(
            u64::try_from(all_kept.len()).unwrap_or(u64::MAX),
            loose.streamed,
            "while a sink that keeps everything keeps every one of them"
        );
    }

    /// The engine holds no survivor after a streamed walk, and says how many.
    ///
    /// [`Streamed`] has no `Vec<Itemset>` anywhere in it, so this measures the
    /// only thing there is to measure: the retaining walk's own held bytes
    /// against a level record that is `size_of::<Tally>()` per level whatever
    /// the frontier widths were.
    #[test]
    fn a_streamed_walk_retains_no_survivor_at_all() {
        let live: Vec<u32> = (0..8).collect();
        let spec: Vec<Vec<u32>> = (0..64_u32)
            .map(|bar| (0..8_u32).filter(|b| bar % (b + 2) != 0).collect())
            .collect();
        let masks: Vec<ConditionMask> = spec
            .iter()
            .map(|bits| {
                bits.iter()
                    .fold(ConditionMask::default(), |m, &b| m.with_bit(b))
            })
            .collect();
        let column = Column::from_rows(&masks);

        let retained = Ladder::with_min_hits(1).walk_column(&column, &live, &|_, _, _| {});
        let held_bytes: usize = retained
            .levels
            .iter()
            .map(|l| l.frequent.capacity().saturating_mul(size_of::<Itemset>()))
            .sum();

        let streamed =
            Ladder::with_min_hits(1).walk_column_streamed(&column, &live, &mut |_, _, _| {});
        let record_bytes = streamed.levels.len().saturating_mul(size_of::<Tally>());

        assert!(
            held_bytes > 0,
            "the retaining walk must actually hold something, or there is \
             nothing to have saved"
        );
        assert!(
            record_bytes < held_bytes,
            "a level record of {record_bytes} B must be smaller than the \
             {held_bytes} B of survivors it replaces"
        );
        assert_eq!(
            streamed.streamed,
            u64::try_from(retained.all_frequent().count()).unwrap_or(u64::MAX),
            "and every survivor the retaining walk KEPT was handed over and \
             counted rather than silently dropped"
        );
        assert_eq!(
            size_of::<Tally>(),
            size_of::<Itemset>(),
            "A WHOLE LEVEL NOW COSTS WHAT ONE SURVIVOR USED TO -- 56 bytes, \
             fixed however wide the frontier was. Pinned against `Itemset` \
             rather than against the number, so a mask widening moves both and \
             a field added to either fails the build rather than drifting."
        );
    }
}
