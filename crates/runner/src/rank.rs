//! The best combinations, in bounded memory.
//!
//! # The wall this exists to remove
//!
//! A sweep retains every survivor. Measured on this machine: **212 bytes per
//! retained combination**, 61,125,295 combinations at 13.0 GB, and the arithmetic
//! from there is unforgiving —
//!
//! | combinations | retained |
//! |---|---|
//! | 61 million | 13 GB |
//! | 1 billion | 212 GB |
//! | 100 billion | 21,200 GB |
//!
//! No machine holds the last row, and the frequent-itemset literature says so in
//! its own words: the FIMI benchmark could not run below a threshold because
//! *the output file* exceeded its limit, not because the search was too slow.
//! The binding constraint is what you keep.
//!
//! # What this does instead
//!
//! Keeps `k` and throws the rest away as it goes. Memory becomes a function of
//! how many results you want to LOOK at, not of how many exist.
//!
//! **A `Scored` is 120 bytes** -- `ConditionMask` 48, `hits` 8, `Edge` 64. This
//! header said 80, pricing `Edge` at 24 for three fields when it has EIGHT:
//! `n`, `mean_paisa`, `wins`, `win_sum`, `loss_sum`, `mismatched`, `refused`,
//! `t`. It drifted when the payoff fields landed and nothing re-measured it --
//! the second time this one paragraph has carried a stale width, the first being
//! the 212 an earlier audit caught.
//!
//! **The peak is `chunks x keep`, not `keep`.** The walk is chunked across the
//! cores, each chunk holding its own bounded heap, so the bound moved when the
//! parallel form landed and this paragraph did not move with it. Chunks are
//! `4 x threads` per level, so on fourteen cores at `keep = 10_000` the peak is
//! about **67 MB**, not the 0.80 MB written here before. Each chunk's heap is
//! now reserved at `keep.min(part.len())`, so a SHORT chunk costs what it can
//! hold rather than what the caller might have wanted.
//!
//! The transient doubling is real and stated: the final `collect` is an
//! `ExactSizeIterator`, so a second keep-sized buffer exists while the heap's own
//! is still alive. Every term is O(keep x cores), which is the claim that
//! matters -- bounded by the MACHINE, and never by the size of the frequent set.
//!
//! # Ranked by |t|, and why the absolute value
//!
//! A combination that reliably precedes a **fall** is as tradeable as one that
//! precedes a rise; only the direction of the position differs. Ranking by `t`
//! signed would discard every short setup, so the order is on |t| and the sign
//! is carried in [`crate::outcome::Edge::mean_paisa`] for a reader to see.
//!
//! This is a stated choice, recorded in `docs/05-decisions.md`, not a
//! derivation — as with the horizon and the measure it sits beside.
//!
//! # It ranks by evidence, and the bar for that evidence is elsewhere
//!
//! A high |t| here is **not** a finding. [`crate::significance`] computes what t
//! must clear given how many hypotheses the run tested, and on a sweep of
//! sixty-one million that bar is above 6. This module orders candidates; it does
//! not bless them, and the report prints both numbers side by side so the
//! difference is impossible to miss.

use core::cmp::Ordering;
use rayon::prelude::*;

use engine::{Frontier, Sweep};
use indicators::column::Column;
use vocab::ConditionMask;

use crate::outcome::{Edge, Forward, edge};

/// The one bounded heap shape used by retained and streamed ranking alike.
type Heap<K> = std::collections::BinaryHeap<core::cmp::Reverse<K>>;

/// One combination and what it did.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scored {
    /// The combination.
    pub mask: ConditionMask,
    /// How many swept bars it fired on, from the sweep.
    pub hits: u64,
    /// What happened after those bars.
    pub edge: Edge,
}

/// Ordered by |t|, then by mask so ties are stable across processes.
///
/// `f64` is not `Ord`, and the usual escape — comparing with `partial_cmp` and
/// unwrapping — is a panic on a NaN. `total_cmp` gives a genuine total order
/// over every `f64` including NaN, so the heap needs no fallible arm and
/// `CLAUDE.md` §3 rule 5's byte-for-byte reproducibility holds without one.
///
/// # A NON-FINITE `t` SORTS LAST, AND IT USED TO SORT FIRST
///
/// `total_cmp` follows IEEE 754 totalOrder, where a positive NaN ranks ABOVE
/// every real number and above infinity — measured: `NAN.abs().total_cmp(&1e308)`
/// is `Greater`, and a best-first sort of `[3, NaN, 100, 7]` returns
/// `[NaN, 100, 7, 3]`. This ranks on the LARGEST |t| and takes the top, so a
/// degenerate sample would have been selected as the best combination in the
/// sweep, and the report would have named it without a word of complaint.
///
/// # Why it was not reachable, and why that is not a reason to leave it
///
/// `outcome::edge` computes `t` behind `if standard_error > 0.0`, and NaN fails
/// that comparison, so a degenerate sample comes back as `t = 0`. The protection
/// is real but it is INCIDENTAL: the guard is written to catch a zero spread,
/// and it catches NaN only because every comparison against NaN is false. Spell
/// the same intent as `!= 0.0` — which reads as equivalent — and a NaN goes
/// straight through to the top of this ordering.
///
/// So the demotion is here, at the ordering that would act on it, rather than
/// resting on the arithmetic upstream continuing to be written one particular
/// way. A non-finite score is not a strong result; it is the absence of one.
impl Ord for Scored {
    fn cmp(&self, other: &Self) -> Ordering {
        // `false < true`, so a finite score sorts above a non-finite one under
        // this key, and the ordering is still total: two non-finite scores
        // compare equal here and fall through to `total_cmp` and then the mask,
        // which keeps §3 rule 5's byte-for-byte reproducibility.
        let (mine, theirs) = (self.edge.t.abs(), other.edge.t.abs());
        mine.is_finite()
            .cmp(&theirs.is_finite())
            .then_with(|| mine.total_cmp(&theirs))
            .then_with(|| self.mask.words().cmp(&other.mask.words()))
    }
}

impl PartialOrd for Scored {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Scored {}

/// The kept combinations, and how many were weighed to find them.
#[derive(Clone, Debug)]
pub struct Ranked {
    /// Best first under [`Self::lens`].
    pub top: Vec<Scored>,
    /// The closed members of [`Self::top`], in the same order.
    ///
    /// This is deliberately rank-all, cut to top-N, then filter; it does not
    /// backfill from below the cut and therefore preserves the historical CLI
    /// selection exactly.
    pub closed_top: Vec<Scored>,
    /// Every combination the sweep produced — the number this kept `top` out of.
    pub considered: u64,
    /// The ordering that made the hard cut.
    pub lens: Lens,
    /// Frequent masks proved redundant by an equal-support immediate superset.
    pub(crate) redundant: u64,
    /// Whether every non-empty level had a successor that decided closure.
    pub(crate) closure_complete: bool,
}

impl Default for Ranked {
    fn default() -> Self {
        Self {
            top: Vec::new(),
            closed_top: Vec::new(),
            considered: 0,
            lens: Lens::Detectability,
            redundant: 0,
            closure_complete: true,
        }
    }
}

/// One ranked value beside the closure verdict known at its retirement.
#[derive(Clone, Copy, Debug)]
struct Marked<K> {
    ranked: K,
    closed: bool,
}

impl<K: Ord> Ord for Marked<K> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.ranked.cmp(&other.ranked)
    }
}

impl<K: Ord> PartialOrd for Marked<K> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<K: Ord> PartialEq for Marked<K> {
    fn eq(&self, other: &Self) -> bool {
        self.ranked == other.ranked
    }
}

impl<K: Ord> Eq for Marked<K> {}

/// A bounded edge ranker fed at the instant each frontier retires.
pub struct Accumulator {
    keep: usize,
    considered: u64,
    lens: Lens,
    redundant: u64,
    closure_complete: bool,
    held: Held,
}

/// The ordering chosen for one [`Accumulator`].
enum Held {
    Detectability(Heap<Marked<Scored>>),
    Payoff(Heap<Marked<ByPayoff>>),
    Path(Heap<Marked<ByPath>>),
    Asymmetry(Heap<Marked<ByAsymmetry>>),
}

impl Accumulator {
    /// An empty ranker under `lens`, retaining at most `keep` rows.
    #[must_use]
    pub fn new(keep: usize, lens: Lens) -> Self {
        let held = match lens {
            Lens::Detectability => Held::Detectability(Heap::with_capacity(keep)),
            Lens::Payoff => Held::Payoff(Heap::with_capacity(keep)),
            Lens::Path => Held::Path(Heap::with_capacity(keep)),
            Lens::Asymmetry => Held::Asymmetry(Heap::with_capacity(keep)),
        };
        Self {
            keep,
            considered: 0,
            lens,
            redundant: 0,
            closure_complete: true,
            held,
        }
    }

    /// Score a retired level after its immediate successor is known.
    ///
    /// `Some(empty)` proves normal extinction. `None` on a non-empty level is
    /// a halted partial frontier: it is still scored for honest partial-run
    /// reporting, but none of its masks is called closed because an unbuilt
    /// successor could change that statement.
    pub(crate) fn offer_retired(
        &mut self,
        level: &Frontier,
        next: Option<&Frontier>,
        column: &Column,
        forward: &Forward,
    ) {
        self.considered = self
            .considered
            .saturating_add(u64::try_from(level.frequent.len()).unwrap_or(u64::MAX));
        if level.frequent.is_empty() {
            return;
        }
        let (redundant, closure_known) = if let Some(next) = next {
            let redundant = crate::closed::redundant_between(level, next);
            self.redundant = self
                .redundant
                .saturating_add(u64::try_from(redundant.len()).unwrap_or(u64::MAX));
            (redundant, true)
        } else {
            self.closure_complete = false;
            (std::collections::HashSet::with_capacity(0), false)
        };
        if self.keep == 0 {
            return;
        }
        match &mut self.held {
            Held::Detectability(heap) => offer_part::<Scored>(
                heap,
                &level.frequent,
                column,
                forward,
                self.keep,
                &redundant,
                closure_known,
            ),
            Held::Path(heap) => offer_part::<ByPath>(
                heap,
                &level.frequent,
                column,
                forward,
                self.keep,
                &redundant,
                closure_known,
            ),
            Held::Payoff(heap) => offer_part::<ByPayoff>(
                heap,
                &level.frequent,
                column,
                forward,
                self.keep,
                &redundant,
                closure_known,
            ),
            Held::Asymmetry(heap) => offer_part::<ByAsymmetry>(
                heap,
                &level.frequent,
                column,
                forward,
                self.keep,
                &redundant,
                closure_known,
            ),
        }
    }

    /// Finish the ranking, strongest first, with the full considered count.
    #[must_use]
    pub fn finish(self) -> Ranked {
        let (top, closed_top) = match self.held {
            Held::Detectability(heap) => ordered(heap),
            Held::Payoff(heap) => ordered(heap),
            Held::Path(heap) => ordered(heap),
            Held::Asymmetry(heap) => ordered(heap),
        };
        Ranked {
            top,
            closed_top,
            considered: self.considered,
            lens: self.lens,
            redundant: self.redundant,
            closure_complete: self.closure_complete,
        }
    }
}

/// The best `keep` combinations by |t|, in memory proportional to `keep`.
///
/// # Why a heap and not a sort
///
/// Sorting needs every scored combination resident at once, which is the 21 TB
/// this module exists to avoid. A bounded min-heap holds `keep` and discards on
/// arrival: the smallest |t| currently held is the admission price, and anything
/// below it is dropped without ever being stored.
///
/// # Cost
///
/// Per combination: one [`edge`] pass over the column, then O(log keep) to
/// admit or O(1) to reject. Memory is O(keep) and independent of how many
/// combinations exist, which is the whole point.
///
/// The edge pass is O(column) per combination and that is not a defect —
/// measuring what a combination did requires looking at the bars it fired on,
/// and there is no way to know that without visiting them. It is O(1) per
/// (bar, combination), which is what `CLAUDE.md` §3 rule 4 requires.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn rank(sweep: &Sweep, column: &Column, forward: &Forward, keep: usize) -> Ranked {
    rank_by(sweep, column, forward, keep, Lens::Detectability)
}

/// Which question decides who survives the cut.
///
/// # Why the cut needs a choice at all
///
/// `keep` is a HARD boundary: everything under it is discarded before any exit
/// grid is built, and `crates/cli`'s own comment on the cap says the loss cannot
/// be recovered — *"No tier ladder, no rule and no report can recover that. They
/// all filter cells, and the cells were never computed."* So the ordering that
/// decides the cut decides what the whole downstream pipeline is even able to
/// consider.
///
/// [`Self::Detectability`] asks *how reliably does this differ from zero*. That
/// is the right question for "is there an effect here". It is the wrong question
/// for "would this be profitable with a tight stop", because a setup whose
/// losers are small and whose winners are large can have a mean near zero.
///
/// Neither lens is correct in general and neither replaces the other, which is
/// why this is a parameter rather than a new default.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lens {
    /// Rank by `|t|` — the historical behaviour, unchanged and bit-identical.
    Detectability,
    /// Rank by [`crate::outcome::Edge::payoff_bp`], ties broken by `|t|`.
    ///
    /// The tie-break matters more than it looks. Payoff alone is magnificent on
    /// four observations, and [`i64::MAX`] on any sample that never lost — so a
    /// combination firing twice with two winners would otherwise outrank
    /// everything. Falling through to `|t|` puts the better-evidenced of two
    /// equal payoffs first, and the significance bar downstream still applies.
    Payoff,
    /// Rank by [`crate::outcome::Edge::path_ratio_bp`], ties broken by `|t|`.
    ///
    /// # The question the other two cannot ask
    ///
    /// Both lenses above read the NET move. `Detectability` asks how reliably
    /// it differs from zero; `Payoff` asks how big the wins are against the
    /// losses. Neither sees the PATH, so neither can tell a combination that
    /// runs straight to its exit from one that dips five points first. Those
    /// need opposite stops, and this cut decides which combinations ever meet
    /// an exit grid at all.
    ///
    /// MEASURED consequence of that blindness: a 15-minute NIFTY rung earning
    /// 23%% of its own sqrt-of-time expectation while the 30-minute rung earned
    /// 92%% on the same span. The combinations that close the gap are ordinary
    /// on a mean and excellent with a tight stop, and they were cut here.
    ///
    /// Same tie-break as [`Self::Payoff`] and for the same reason: a path
    /// ratio is [`i64::MAX`] on any sample that never traded against its
    /// entry, so two hits with two clean runs would otherwise outrank
    /// everything. `|t|` puts the better-evidenced of two equal ratios first.
    Path,
    /// Rank by [`crate::outcome::Edge::worst_reward_risk_bp`] — the operator's
    /// own rule — ties broken by the LARGEST win, then by `|t|`. D-0593.
    ///
    /// # The question no other lens here can ask
    ///
    /// The three lenses above are all built from counts, sums and `|t|`, so
    /// none of them can state *"min(win) >= 3x max(loss)"*: an extremum is not
    /// recoverable from a sum. [`Self::Payoff`] answers the nearest available
    /// question — MEAN win over MEAN loss — which is the statistic the rule
    /// names and rejects, because one catastrophic loss hides behind many small
    /// ones in a denominator.
    ///
    /// The gap is not cosmetic. `grid::Cell::reward_to_risk_bp` already
    /// computed the true min/max ratio, but a `Cell` exists only after an exit
    /// grid is built, and the `|t|` cut that decides which combinations ever
    /// reach a grid happens first. The rule was computable only downstream of
    /// the gate it needed to pass, so a rare asymmetric winner was cut before
    /// anything could notice it satisfied the rule.
    ///
    /// Not the default, and deliberately: `|t|` is the right cut when the
    /// question is whether an edge is real, and this one is right when the
    /// question is whether it is SHAPED correctly. Both are legitimate and they
    /// order the same candidates differently.
    Asymmetry,
}

/// One combination, ordered by payoff first and evidence second.
///
/// A newtype rather than a flag on [`Scored`], so the historical ordering is
/// literally the same code it always was and cannot drift while the new one is
/// edited.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ByPayoff(Scored);

impl Eq for ByPayoff {}

impl PartialOrd for ByPayoff {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ByPayoff {
    fn cmp(&self, other: &Self) -> Ordering {
        let (mine, theirs) = (self.0.edge.payoff_bp(), other.0.edge.payoff_bp());
        // Then `|t|`, on the same finite-first total order `Scored` uses, so two
        // equal payoffs are separated by evidence rather than by mask bytes --
        // and the mask still breaks a true tie, which is what keeps §3 rule 5's
        // byte-for-byte reproducibility.
        mine.cmp(&theirs).then_with(|| self.0.cmp(&other.0))
    }
}

/// What a heap of some ordering needs to hold and hand back a [`Scored`].
trait Ranked1: Ord + Sized {
    fn wrap(scored: Scored) -> Self;
    fn unwrap(self) -> Scored;
}

impl Ranked1 for Scored {
    fn wrap(scored: Scored) -> Self {
        scored
    }
    fn unwrap(self) -> Scored {
        self
    }
}

impl Ranked1 for ByPayoff {
    fn wrap(scored: Scored) -> Self {
        Self(scored)
    }
    fn unwrap(self) -> Scored {
        self.0
    }
}

/// One combination, ordered by what its PATH offered and evidence second.
///
/// Mirrors [`ByPayoff`] exactly, on [`crate::outcome::Edge::path_ratio_bp`]
/// instead of `payoff_bp`. The tie-break through `Scored` is the same and for
/// the same reason: the ratio is [`i64::MAX`] on any sample that never traded
/// against its entry, so evidence has to break those ties or two lucky hits
/// outrank a thousand.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ByPath(Scored);

impl Eq for ByPath {}

impl PartialOrd for ByPath {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ByPath {
    fn cmp(&self, other: &Self) -> Ordering {
        let (mine, theirs) = (self.0.edge.path_ratio_bp(), other.0.edge.path_ratio_bp());
        mine.cmp(&theirs).then_with(|| self.0.cmp(&other.0))
    }
}

impl Ranked1 for ByPath {
    fn wrap(scored: Scored) -> Self {
        Self(scored)
    }
    fn unwrap(self) -> Scored {
        self.0
    }
}

/// One combination, ordered by the operator's own rule — D-0593.
///
/// A newtype for the reason [`ByPayoff`] is one: the historical orderings stay
/// literally the same code and cannot drift while this one is edited.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ByAsymmetry(Scored);

impl Eq for ByAsymmetry {}

impl PartialOrd for ByAsymmetry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ByAsymmetry {
    /// Smallest win over largest loss, then the SIZE of the largest win, then
    /// `|t|`.
    ///
    /// # Why the second term is not `|t|`, unlike every other lens here
    ///
    /// [`ByPayoff`] and [`ByPath`] fall straight through to `|t|`, and for them
    /// that is right: both keys saturate at [`i64::MAX`] on a sample that never
    /// lost, so ties are the common case and evidence is the honest separator.
    ///
    /// This key saturates the same way and ties just as often — but falling to
    /// `|t|` there would undo the whole lens. `|t|` is `mean / (sd / sqrt(n))`,
    /// and a rare asymmetric winner is a large-dispersion, near-zero-mean
    /// sample: the winners that make it valuable are exactly what inflates `sd`.
    /// Two combinations tied at "never lost" would therefore be separated by
    /// preferring the one whose wins are *smaller and more uniform*, which is
    /// the opposite of the question being asked.
    ///
    /// `max_win_paisa` breaks that tie on the thing the operator is hunting —
    /// how much it pays when it pays — and `|t|` remains the LAST term so the
    /// order is still total and reproducible under §3 rule 5.
    fn cmp(&self, other: &Self) -> Ordering {
        let (mine, theirs) = (
            self.0.edge.worst_reward_risk_bp(),
            other.0.edge.worst_reward_risk_bp(),
        );
        mine.cmp(&theirs)
            .then_with(|| {
                self.0
                    .edge
                    .max_win_paisa
                    .total_cmp(&other.0.edge.max_win_paisa)
            })
            .then_with(|| self.0.cmp(&other.0))
    }
}

impl Ranked1 for ByAsymmetry {
    fn wrap(scored: Scored) -> Self {
        Self(scored)
    }
    fn unwrap(self) -> Scored {
        self.0
    }
}

/// [`rank`], under a chosen [`Lens`].
///
/// The historical call is [`Lens::Detectability`] and produces byte-identical
/// output to what it always did — the ordering is the same `impl Ord for
/// Scored`, reached through a wrapper that adds nothing.
#[must_use]
pub fn rank_by(
    sweep: &Sweep,
    column: &Column,
    forward: &Forward,
    keep: usize,
    lens: Lens,
) -> Ranked {
    let mut ranked = Accumulator::new(keep, lens);
    for (index, level) in sweep.levels.iter().enumerate() {
        ranked.offer_retired(
            level,
            sweep.levels.get(index.saturating_add(1)),
            column,
            forward,
        );
    }
    ranked.finish()
}

/// Admit one scored value into a heap already holding at most `keep`.
///
/// The admission price is the weakest currently held. Comparing before pushing
/// is what keeps the heap at `keep` rather than letting it grow and trimming
/// afterwards -- the trim-after form allocates the whole result set, which is
/// the thing this module refuses to do.
fn admit<K: Ord>(heap: &mut Heap<K>, keep: usize, scored: K) {
    if heap.len() < keep {
        heap.push(core::cmp::Reverse(scored));
        return;
    }
    let weakest_is_weaker = heap.peek().is_some_and(|core::cmp::Reverse(w)| *w < scored);
    if weakest_is_weaker {
        heap.pop();
        heap.push(core::cmp::Reverse(scored));
    }
}

/// The best `keep` of one contiguous run of itemsets, scored.
///
/// One bounded heap, exactly as the whole walk used to be. This is the unit of
/// parallel work: it touches nothing outside the slice it was handed, so any
/// number of these run at once without coordination.
fn top_of<K: Ranked1>(
    part: &[engine::Itemset],
    column: &Column,
    forward: &Forward,
    keep: usize,
    redundant: &std::collections::HashSet<ConditionMask>,
    closure_known: bool,
) -> Vec<Marked<K>> {
    // `keep.min(part.len())` AND NOT `keep`, WHICH WAS A REAL COST.
    //
    // A chunk cannot yield more rows than it holds, so reserving `keep` on a
    // short chunk reserves for rows that cannot exist — and the over-reservation
    // survives into the returned `Vec`, which inherits the heap's capacity, and
    // every part is live at once before the merge.
    //
    // Measured against the defaults: `chunk_size` collapses to a width of 1 when
    // the frequent set is smaller than four per thread, so a 40-survivor sweep on
    // fourteen cores is FORTY chunks. At `keep = 10_000` and 120 bytes a
    // `Scored`, reserving `keep` each was 48 MB to rank forty rows, against the
    // 1.2 MB the single heap this replaced would have used.
    let mut heap: Heap<Marked<K>> = Heap::with_capacity(keep.min(part.len()));
    for itemset in part {
        admit(
            &mut heap,
            keep,
            Marked {
                ranked: K::wrap(Scored {
                    mask: itemset.mask,
                    hits: itemset.hits,
                    edge: edge(column, forward, &itemset.mask),
                }),
                closed: closure_known && !redundant.contains(&itemset.mask),
            },
        );
    }
    heap.into_iter().map(|core::cmp::Reverse(s)| s).collect()
}

/// How many itemsets one parallel chunk carries.
///
/// # Chosen from the CORE COUNT, not from the data
///
/// Peak memory across the pass is `chunks x keep`, so a chunk size fixed in
/// itemsets would make it a function of `|frequent|` -- which at the measured
/// 17.8 million survivors is exactly the whole-result-set allocation this
/// module exists to refuse. Sizing the other way round, from the number of
/// threads, keeps the peak proportional to the MACHINE.
///
/// Four chunks per thread rather than one: the cost of a chunk varies with how
/// many of its masks hit, so equal-sized chunks are not equal-cost, and a few
/// spare chunks let rayon's work-stealing fill a core that finished early.
/// Four is the smallest multiple that measurably does so and the largest that
/// keeps `chunks x keep` inside a tenth of what one bar column costs.
///
/// At least one, because a chunk of zero itemsets would divide by zero above
/// and never terminate below.
fn chunk_size(total: usize) -> usize {
    let want = rayon::current_num_threads().saturating_mul(4).max(1);
    total.div_ceil(want).max(1)
}

/// Score one frontier in parallel and merge its bounded chunk heaps into `heap`.
fn offer_part<K: Ranked1 + Send>(
    heap: &mut Heap<Marked<K>>,
    itemsets: &[engine::Itemset],
    column: &Column,
    forward: &Forward,
    keep: usize,
    redundant: &std::collections::HashSet<ConditionMask>,
    closure_known: bool,
) {
    let width = chunk_size(itemsets.len());
    let parts: Vec<Vec<Marked<K>>> = itemsets
        .par_chunks(width)
        .map(|part| top_of::<K>(part, column, forward, keep, redundant, closure_known))
        .collect();
    for scored in parts.into_iter().flatten() {
        admit(heap, keep, scored);
    }
}

/// Drain one bounded heap into the public best-first order.
fn ordered<K: Ranked1>(heap: Heap<Marked<K>>) -> (Vec<Scored>, Vec<Scored>) {
    let mut top: Vec<Marked<K>> = heap
        .into_iter()
        .map(|core::cmp::Reverse(scored)| scored)
        .collect();
    top.sort_unstable_by(|a, b| b.cmp(a));
    let mut all = Vec::with_capacity(top.len());
    let mut closed = Vec::with_capacity(top.len());
    for marked in top {
        let scored = marked.ranked.unwrap();
        all.push(scored);
        if marked.closed {
            closed.push(scored);
        }
    }
    (all, closed)
}

/// One pass over the frequent set, keeping the best `keep` under `K`'s ordering.
///
/// # Spread across every core, and byte-identical to the pass it replaces
///
/// `edge` is `O(bars)` and is called once per frequent itemset, so this pass is
/// `O(|frequent| x bars)` -- the largest single cost after the enumeration, and
/// the enumeration is the half that cannot be spread (gate 22 pins
/// `crates/engine` to `vocab` alone). This half has no such constraint.
///
/// **`CLAUDE.md` §3 rule 5 is satisfied by the ORDERING, not by the schedule.**
/// [`Scored::cmp`] falls through to `self.mask.words().cmp(..)` and the masks in
/// a frequent set are unique, so no two values can compare `Equal`. A strict
/// total order has exactly one sorted sequence whatever order its elements
/// arrived in -- which is why merging per-chunk heaps cannot move a result, and
/// why `sort_unstable_by` is still sound here despite the name.
///
/// The argument is not trusted on its own:
/// `a_parallel_walk_is_byte_identical_to_a_sequential_one` runs both forms over
/// the same sweep and compares every field of every row.
#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{Accumulator, ByAsymmetry, Lens, Scored, rank, rank_by};
    use crate::outcome::{Edge, Horizon, forward};
    use crate::{Sweeper, synthetic};
    use engine::Ladder;
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

    fn scored(t: f64, bit: u32) -> Scored {
        Scored {
            mask: ConditionMask::default().with_bit(bit),
            hits: 1,
            edge: Edge {
                n: 2,
                mismatched: 0,
                refused: 0,
                mean_paisa: t,
                t,
                // The payoff split is not what this fixture is about, and
                // spreading the default keeps a later field from breaking it.
                ..Edge::default()
            },
        }
    }

    /// The two lenses keep DIFFERENT combinations, which is the point.
    ///
    /// A lens that reordered without changing who survives the cut would be
    /// cosmetic. This asserts the sniper — small losers, large winners, mean
    /// near zero — is the one `Detectability` discards and `Payoff` keeps.
    #[test]
    fn the_two_lenses_keep_different_combinations_at_the_cut() {
        // The sniper: nine losers of -10 and one winner of +90. Mean 0, so its
        // `t` is 0 and it sits at the BOTTOM of a detectability ranking.
        let sniper = Scored {
            mask: ConditionMask::default().with_bit(1),
            hits: 10,
            edge: Edge {
                n: 10,
                wins: 1,
                win_sum: 90.0,
                losses: 9,
                loss_sum: -90.0,
                t: 0.0,
                ..Edge::default()
            },
        };
        // The grinder: a strong, reliable, small edge. High `t`, poor payoff.
        let grinder = Scored {
            mask: ConditionMask::default().with_bit(2),
            hits: 10,
            edge: Edge {
                n: 10,
                wins: 9,
                win_sum: 90.0,
                losses: 1,
                loss_sum: -90.0,
                t: 9.0,
                ..Edge::default()
            },
        };

        assert!(
            grinder > sniper,
            "under Detectability the grinder wins -- t 9.0 against 0.0"
        );
        assert!(
            super::ByPayoff(sniper) > super::ByPayoff(grinder),
            "under Payoff the sniper wins -- 9.00 against 0.11"
        );
    }

    /// Payoff ties fall through to evidence, not to mask bytes.
    ///
    /// Without this, a combination that fired twice and won twice reports
    /// `i64::MAX` and outranks every real finding in the run.
    #[test]
    fn an_equal_payoff_is_broken_by_evidence() {
        let thin = Scored {
            mask: ConditionMask::default().with_bit(1),
            hits: 2,
            edge: Edge {
                n: 2,
                wins: 2,
                win_sum: 20.0,
                loss_sum: 0.0,
                t: 0.5,
                ..Edge::default()
            },
        };
        let thick = Scored {
            mask: ConditionMask::default().with_bit(2),
            hits: 900,
            edge: Edge {
                n: 900,
                wins: 900,
                win_sum: 9_000.0,
                loss_sum: 0.0,
                t: 12.0,
                ..Edge::default()
            },
        };
        assert_eq!(
            thin.edge.payoff_bp(),
            thick.edge.payoff_bp(),
            "both never lost, so both are unbounded and the payoffs tie"
        );
        assert!(
            super::ByPayoff(thick) > super::ByPayoff(thin),
            "a tie on payoff is settled by evidence, or two winning trades would \
             outrank nine hundred"
        );
    }

    #[test]
    fn the_order_is_on_the_absolute_t_so_a_short_setup_is_not_discarded() {
        let up = scored(3.0, 1);
        let down = scored(-9.0, 2);
        assert!(
            down > up,
            "a combination that reliably precedes a FALL is as tradeable as one \
             that precedes a rise; ranking on signed t would throw every short \
             setup away"
        );
    }

    #[test]
    fn ties_break_on_the_mask_so_the_order_is_stable_across_processes() {
        let a = scored(4.0, 1);
        let b = scored(4.0, 2);
        assert_ne!(a.cmp(&b), core::cmp::Ordering::Equal);
        assert_eq!(a.cmp(&a), core::cmp::Ordering::Equal);
    }

    #[test]
    fn a_nan_t_is_ordered_rather_than_panicking() {
        // `partial_cmp().unwrap()` is the usual way to put an f64 in a heap and
        // it is a panic waiting for a degenerate sample. `total_cmp` orders NaN
        // rather than refusing to.
        let nan = scored(f64::NAN, 1);
        let real = scored(5.0, 2);
        assert_ne!(nan.cmp(&real), core::cmp::Ordering::Equal);
        assert_eq!(nan.cmp(&nan), core::cmp::Ordering::Equal);
    }

    /// A NON-FINITE SCORE LOSES TO EVERY REAL ONE, INCLUDING A TINY ONE.
    ///
    /// # What the test above does not say
    ///
    /// It asserts a NaN is ORDERED rather than panicking, and that was the whole
    /// question while the ordering was `total_cmp` alone. It is silent on the
    /// DIRECTION, and the direction is what selects a combination: `total_cmp`
    /// follows IEEE 754 totalOrder, so a positive NaN ranks above every real
    /// number and above infinity. Measured directly:
    /// `NAN.abs().total_cmp(&1e308)` is `Greater`, and a best-first sort of
    /// `[3, NaN, 100, 7]` returns `[NaN, 100, 7, 3]`.
    ///
    /// This crate ranks on the LARGEST |t| and takes the top, so a degenerate
    /// sample would have been chosen as the best combination in the sweep and
    /// reported as the answer.
    ///
    /// # Infinity too, and it is the likelier of the two
    ///
    /// A NaN needs `0.0 / 0.0`. An infinity needs only a mean divided by a
    /// standard error that underflowed, which is one degenerate window away on
    /// any real series. Both are the absence of a result, not a strong one.
    #[test]
    fn a_non_finite_score_ranks_below_every_finite_one() {
        let tiny = scored(f64::MIN_POSITIVE, 9);
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let degenerate = scored(bad, 1);
            assert_eq!(
                degenerate.cmp(&tiny),
                core::cmp::Ordering::Less,
                "{bad} must lose to the smallest real score, or a degenerate \
                 sample is selected as the sweep's answer"
            );
            assert_eq!(
                tiny.cmp(&degenerate),
                core::cmp::Ordering::Greater,
                "and the comparison must be antisymmetric"
            );
        }
        // Still a TOTAL order: two non-finite scores are separated by the mask,
        // so the sort stays deterministic across processes as §3 rule 5 needs.
        let (a, b) = (scored(f64::NAN, 1), scored(f64::NAN, 2));
        assert_ne!(a.cmp(&b), core::cmp::Ordering::Equal);
        assert_eq!(a.cmp(&a), core::cmp::Ordering::Equal);
    }

    #[test]
    fn keeping_zero_weighs_everything_and_retains_nothing() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, &column, Horizon::DEFAULT);

        let r = rank(&out.sweep, &column, &f, 0);
        assert!(r.top.is_empty());
        assert_eq!(
            r.considered,
            u64::try_from(out.sweep.all_frequent().count()).unwrap_or(u64::MAX),
            "every combination is still counted, because 'kept none of four' and \
             'kept none of sixty-one million' are different facts"
        );
    }

    /// Feeding levels as the engine retires them is the same EDGE cut as
    /// ranking a retained sweep afterwards, under both public lenses.
    #[test]
    fn an_incremental_ranker_matches_the_retained_ranker_under_both_lenses() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, &column, Horizon::DEFAULT);

        for lens in [Lens::Detectability, Lens::Payoff] {
            for keep in [0_usize, 25] {
                let expected = rank_by(&out.sweep, &column, &f, keep, lens);
                let mut incremental = Accumulator::new(keep, lens);
                for (index, level) in out.sweep.levels.iter().enumerate() {
                    incremental.offer_retired(
                        level,
                        out.sweep.levels.get(index.saturating_add(1)),
                        &column,
                        &f,
                    );
                }
                let got = incremental.finish();
                assert_eq!(got.considered, expected.considered);
                assert_eq!(
                    got.top, expected.top,
                    "lens {lens:?} at keep {keep} must make one global cut, not one per level"
                );
                assert_eq!(got.closed_top, expected.closed_top);
                assert_eq!(got.redundant, expected.redundant);
                assert_eq!(got.closure_complete, expected.closure_complete);
            }
        }
    }

    /// §3 rule 5, measured rather than argued.
    ///
    /// The chunked pass and the one-heap pass must agree BYTE for byte, not
    /// approximately: same rows, same order, same float bit patterns. The
    /// reason it holds is that `Scored::cmp` ends on the mask and masks are
    /// unique, so a strict total order has exactly one sorted sequence whatever
    /// order its elements arrived in. This runs both and checks.
    #[test]
    fn a_parallel_walk_is_byte_identical_to_a_sequential_one() {
        use super::admit;
        use crate::outcome::edge;
        use std::collections::BinaryHeap;

        let bars = synthetic::sessions(12);
        let out = Sweeper::new(Ladder::with_min_hits(400).with_ceiling(200_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, &column, Horizon::DEFAULT);
        let keep = 40;

        let parallel = rank(&out.sweep, &column, &f, keep);

        // THE SEQUENTIAL REFERENCE, written out rather than referenced: it is
        // the algorithm this module carried before the chunking, and a reader
        // comparing the two should be able to see both.
        let mut heap: BinaryHeap<core::cmp::Reverse<Scored>> = BinaryHeap::with_capacity(keep);
        let mut considered: u64 = 0;
        for itemset in out.sweep.all_frequent() {
            considered = considered.saturating_add(1);
            admit(
                &mut heap,
                keep,
                Scored {
                    mask: itemset.mask,
                    hits: itemset.hits,
                    edge: edge(&column, &f, &itemset.mask),
                },
            );
        }
        let mut expect: Vec<Scored> = heap.into_iter().map(|core::cmp::Reverse(s)| s).collect();
        expect.sort_unstable_by(|a, b| b.cmp(a));

        assert!(
            !expect.is_empty(),
            "the fixture must produce rows, or this test proves nothing about \
             either walk"
        );
        assert_eq!(parallel.considered, considered, "same count");
        assert_eq!(parallel.top.len(), expect.len(), "same number of rows");

        for (rank_index, (got, want)) in parallel.top.iter().zip(&expect).enumerate() {
            assert_eq!(
                got.mask.words(),
                want.mask.words(),
                "row {rank_index}: a different combination"
            );
            assert_eq!(got.hits, want.hits, "row {rank_index}: hits");
            assert_eq!(got.edge.n, want.edge.n, "row {rank_index}: n");
            // BIT PATTERNS, NOT `==`. Two floats can compare equal and differ in
            // their representation, and §3 rule 5 is about the BYTES.
            assert_eq!(
                got.edge.t.to_bits(),
                want.edge.t.to_bits(),
                "row {rank_index}: t is not bit-identical"
            );
            assert_eq!(
                got.edge.mean_paisa.to_bits(),
                want.edge.mean_paisa.to_bits(),
                "row {rank_index}: mean is not bit-identical"
            );
        }
    }

    /// The chunk width is sized from the machine and never zero.
    #[test]
    fn a_chunk_is_never_empty_however_little_there_is_to_do() {
        use super::chunk_size;
        assert!(chunk_size(0) >= 1, "an empty sweep must not divide by zero");
        assert!(chunk_size(1) >= 1);
        assert!(chunk_size(7) >= 1);
        // AND IT SHRINKS AS THE WORK GROWS, which is what makes peak memory a
        // function of the core count rather than of the frequent set.
        assert!(
            chunk_size(10_000_000) >= chunk_size(10),
            "a larger sweep must not produce a SMALLER chunk"
        );
    }

    #[test]
    fn the_kept_set_is_the_best_by_absolute_t_and_bounded_by_keep() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, &column, Horizon::DEFAULT);
        let total = out.sweep.all_frequent().count();
        assert!(total > 100, "the fixture must produce enough to rank");

        let keep = 25;
        let r = rank(&out.sweep, &column, &f, keep);
        assert_eq!(r.top.len(), keep, "the heap must fill and stay bounded");
        assert_eq!(usize::try_from(r.considered).unwrap_or(usize::MAX), total);

        // Best first, and every kept one is at least as strong as the next.
        // `zip` with `skip(1)` rather than `windows(2)` and a `let..else`: the
        // else arm of that pattern cannot fire, and an arm no run reaches is
        // the coverage hole §9 refuses.
        for (a, b) in r.top.iter().zip(r.top.iter().skip(1)) {
            assert!(
                a.edge.t.abs() >= b.edge.t.abs(),
                "the output must be ordered best first"
            );
        }

        // AND NOTHING DISCARDED WAS BETTER THAN WHAT WAS KEPT, which is the only
        // property that makes a bounded heap equivalent to sorting everything.
        //
        // The first form of this counted combinations strictly better than the
        // worst kept and allowed up to `keep` of them. An audit showed that has
        // a slot of slack BY CONSTRUCTION -- at most keep-1 kept rows can be
        // strictly better than the worst kept row -- so it produced the same
        // number on correct and defective code and proved nothing. The property
        // is now asserted directly: every combination NOT kept must be no better
        // than the weakest one that was.
        let worst_kept = r.top.last().map_or(0.0, |s: &Scored| s.edge.t.abs());
        let kept: std::collections::HashSet<[u64; 6]> =
            r.top.iter().map(|s| s.mask.words()).collect();
        // A filter chain rather than a loop with an `if`: the increment inside
        // that `if` is a line no CORRECT run executes, and an unexecuted line is
        // the coverage hole §9 refuses. The predicate is evaluated on every
        // discarded combination either way.
        let discarded_better = out
            .sweep
            .all_frequent()
            .filter(|i| !kept.contains(&i.mask.words()))
            .filter(|i| crate::outcome::edge(&column, &f, &i.mask).t.abs() > worst_kept)
            .count();
        assert_eq!(
            discarded_better, 0,
            "{discarded_better} DISCARDED combinations beat the weakest one kept \
             -- a bounded heap that drops a qualifying entry is not equivalent \
             to sorting everything, it is just a faster way to be wrong"
        );
    }

    #[test]
    fn asking_for_more_than_exists_returns_everything_and_no_padding() {
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, &column, Horizon::DEFAULT);
        let total = out.sweep.all_frequent().count();

        let r = rank(&out.sweep, &column, &f, total.saturating_mul(2));
        assert_eq!(r.top.len(), total, "no padding, no duplication");
    }

    #[test]
    fn ranking_is_idempotent_across_runs() {
        // §3 rule 5. A `BinaryHeap`'s iteration order is not sorted, so the
        // final sort is what makes this true -- and it is what a `HashSet`-
        // derived order would break.
        let bars = synthetic::sessions(8);
        let out = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut evaluator());
        let column = Column::build(&bars, &mut evaluator());
        let f = forward(&bars, &column, Horizon::DEFAULT);

        let first = rank(&out.sweep, &column, &f, 20);
        for _ in 0..4 {
            let again = rank(&out.sweep, &column, &f, 20);
            let a: Vec<_> = first.top.iter().map(|s| s.mask.words()).collect();
            let b: Vec<_> = again.top.iter().map(|s| s.mask.words()).collect();
            assert_eq!(a, b, "two rankings of one input disagreed");
        }
    }

    /// One `Scored` carrying a stated edge, with a distinct mask per name.
    fn shaped(bit: u32, edge: Edge) -> Scored {
        Scored {
            mask: ConditionMask::default().with_bit(bit),
            hits: edge.n,
            edge,
        }
    }

    /// **The inversion this lens exists to correct — D-0593.**
    ///
    /// Two combinations, both real:
    ///
    /// * the RARE ASYMMETRIC WINNER — forty observations, twelve wins, the
    ///   smallest of them 60 paisa against a largest loss of 20, and one win
    ///   of 4,000. That is the operator's shape: min(win) = 3x max(loss).
    /// * the GRINDER — a thousand observations, a tiny consistent mean, and a
    ///   largest loss bigger than its smallest win.
    ///
    /// The grinder wins on `|t|` by construction, because `|t|` is
    /// `mean / (sd / sqrt(n))` and the winner's own tail inflates its `sd`.
    /// Under `Asymmetry` the order must invert. Both halves are asserted: a
    /// test that only checked the new order would pass just as well if the old
    /// one had never been a problem.
    #[test]
    fn the_asymmetry_lens_inverts_the_order_that_t_puts_a_rare_winner_in() {
        let winner = Edge {
            n: 40,
            wins: 12,
            win_sum: 5_000.0,
            losses: 28,
            loss_sum: -560.0,
            min_win_paisa: 60.0,
            max_win_paisa: 4_000.0,
            max_loss_paisa: 20.0,
            t: 1.10,
            ..Edge::default()
        };
        let grinder = Edge {
            n: 1_000,
            wins: 600,
            win_sum: 6_000.0,
            losses: 400,
            loss_sum: -3_600.0,
            min_win_paisa: 5.0,
            max_win_paisa: 30.0,
            max_loss_paisa: 40.0,
            t: 6.40,
            ..Edge::default()
        };

        assert_eq!(
            winner.worst_reward_risk_bp(),
            300,
            "60 paisa smallest win over a 20 paisa largest loss is exactly 3.00x"
        );
        assert_eq!(
            grinder.worst_reward_risk_bp(),
            12,
            "5 over 40 is 0.12x -- the grinder fails the operator's rule outright"
        );

        // The mean-based statistic disagrees with the min/max one on this very
        // pair, which is the whole reason `payoff_bp` could not stand in: the
        // grinder's MEAN win over MEAN loss is 1.11x while its smallest win
        // against its largest loss is 0.12x.
        assert!(
            grinder.payoff_bp() > grinder.worst_reward_risk_bp(),
            "a mean ratio always flatters a dispersed sample: {} vs {}",
            grinder.payoff_bp(),
            grinder.worst_reward_risk_bp()
        );

        let (w, g) = (shaped(1, winner), shaped(2, grinder));

        assert!(
            g > w,
            "under |t| the grinder outranks the rare winner -- this is the \
             defect, and if it ever stops being true this test is measuring \
             nothing"
        );
        assert!(
            ByAsymmetry(w) > ByAsymmetry(g),
            "under Asymmetry the rare winner must come first"
        );
    }

    /// The two ends of the ratio, which are samples and not errors.
    #[test]
    fn the_asymmetry_ratio_reports_both_ends_rather_than_dividing() {
        let never_lost = Edge {
            n: 3,
            wins: 3,
            min_win_paisa: 10.0,
            max_win_paisa: 900.0,
            max_loss_paisa: 0.0,
            ..Edge::default()
        };
        assert_eq!(
            never_lost.worst_reward_risk_bp(),
            i64::MAX,
            "nothing lost is an unbounded ratio, reported rather than divided"
        );

        let never_won = Edge {
            n: 9,
            losses: 9,
            min_win_paisa: 0.0,
            max_loss_paisa: 500.0,
            ..Edge::default()
        };
        assert_eq!(
            never_won.worst_reward_risk_bp(),
            0,
            "nothing won is the floor -- it is not asymmetric, it is absent"
        );
    }
}
