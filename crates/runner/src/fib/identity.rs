//! The POOL-RUN identity of a Fib Flag Breakout run, its data tree and the
//! pair identity (FIB-1; D-6303, D-6316, D-6317).
//!
//! # Why not [`crate::identity::Run`]
//!
//! `Run` carries exactly one `InstrumentKey`, and a cross-sectional run over the
//! whole point-in-time F&O pool has none. Inventing a placeholder key would be
//! the invention `CLAUDE.md` §3 rule 1 forbids. So a FIB run keeps the nine
//! terms of §3 rule 3, in the same order and with the same one-byte tags and
//! `u32` length framing as [`crate::identity`]'s private `term` (re-implemented
//! here byte for byte and pinned by `the_term_framing_is_runner_identity_s`),
//! and reads them for a pool:
//!
//! | term | for a FIB run |
//! |---|---|
//! | mask | every value the run's layout offers (`Layout::offered_mask`) |
//! | direction | Long or Short, never Undirected (operator 16 Oct 16:04 IST) |
//! | instrument | the POOL descriptor ([`Pool::bytes`]) |
//! | timeframe | `zerodha:` and every width tried in every version ([`timeframe_word`]) |
//! | params | the profile digest, the per-run schema digest and the generation root |
//! | `data_digest` | the [`DataTree`] root over every stock-month and authority |
//! | `vocab_version` | `vocab::fib_cube::FIB_CUBE_VERSION` |
//! | commit | the build's commit |
//! | feed | `zerodha` (and the order-flow source) for stage A, `zerodha+gdfl` deep |
//!
//! The whole stream is prefixed by its own eight-byte domain tag [`DOMAIN`], so
//! no FIB identity can equal a [`crate::identity::identity`] of any run.
//!
//! # Cost
//!
//! [`fib_identity`] is BOUNDED, O(term bytes), and taken once per run.
//! [`Mmr::append`] costs O(log n) merges and [`Mmr::root`] bags O(log n) peaks.

use std::collections::BTreeSet;

use brutex_core::blake3::{Hasher, OUT_LEN, hash};
use vocab::ConditionMask;
use vocab::fib_cube::{GenerationChain, Search, fixed_numbers_bytes};

use crate::identity::{Direction, Params, RunId};

/// The eight-byte domain tag of a FIB pool-run identity.
pub const DOMAIN: &[u8] = b"BTXFIBP1";

/// The eight-byte domain tag of a FIB pair identity (FM-F007).
pub const PAIR_DOMAIN: &[u8] = b"BTXFIBQ1";

/// The eight-byte domain tag of every node of the data tree (FM-F003).
pub const TREE_DOMAIN: &[u8] = b"BTXFIBM1";

/// The feed word of a stage-A run without order flow: decisions and walks on
/// Zerodha minutes (D-6304).
pub const FEED_STAGE_A: &str = "zerodha";

/// The feed word of a deep run: Zerodha minutes decide, GDFL seconds execute
/// (D-6304).
pub const FEED_DEEP: &str = "zerodha+gdfl";

/// The nine terms in `CLAUDE.md` §3 rule 3's order. A term's tag is its
/// 1-based position, which is exactly the byte [`crate::identity`] writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Term {
    /// The values offered.
    Mask,
    /// Long or Short.
    Direction,
    /// The pool, in the instrument's place.
    Instrument,
    /// The widths tried.
    Timeframe,
    /// The profile, the schema and the generations.
    Params,
    /// The data tree's root.
    DataDigest,
    /// The schema version.
    VocabVersion,
    /// The build's commit.
    Commit,
    /// The feed.
    Feed,
}

impl Term {
    /// The term's tag byte.
    const fn tag(self) -> u8 {
        self as u8 + 1
    }
}

/// The four terms of a pair identity, tagged by 1-based position.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PairTerm {
    /// The long run's identity.
    Long,
    /// The short run's identity.
    Short,
    /// The pairing rule's version.
    RuleVersion,
    /// The digest of the fixed-number ledger.
    FixedNumbers,
}

impl PairTerm {
    /// The term's tag byte.
    const fn tag(self) -> u8 {
        self as u8 + 1
    }
}

/// Writes one tagged, length-prefixed term: a tag byte, a `u32` little-endian
/// length, the bytes. Byte for byte the framing of [`crate::identity`].
fn term(hasher: &mut Hasher, tag: u8, bytes: &[u8]) {
    hasher.update(&[tag]);
    hasher.update(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
    hasher.update(bytes);
}

/// A refusal of this module, by name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FibIdentityRefusal {
    /// A development pool with no name.
    PoolEmpty,
    /// A development pool whose names are not sorted and unique.
    PoolNamesNotSorted {
        /// The position of the first name not above the one before.
        at: usize,
    },
    /// A data-tree leaf at or before the leaf appended last.
    LeafOutOfOrder {
        /// The offending key.
        key: LeafKey,
    },
}

impl FibIdentityRefusal {
    /// The refusal's stable name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::PoolEmpty => "pool_empty",
            Self::PoolNamesNotSorted { .. } => "pool_names_not_sorted",
            Self::LeafOutOfOrder { .. } => "leaf_out_of_order",
        }
    }
}

impl core::fmt::Display for FibIdentityRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "fib identity refused ({}): {self:?}", self.name())
    }
}

impl std::error::Error for FibIdentityRefusal {}

/// The pool a run trades: the `instrument` term of a pooled run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pool<'a> {
    /// The point-in-time F&O membership (PIT-1): the membership file's digest
    /// and the traded-surface rule's word.
    PointInTime {
        /// The membership file's digest.
        membership_digest: [u8; OUT_LEN],
        /// The traded-surface rule, by its word.
        surface_rule: &'a str,
    },
    /// A development pool: its label and its names, sorted and unique.
    Development {
        /// The pool's label.
        label: &'a str,
        /// The names, sorted and unique.
        names: &'a [&'a str],
    },
}

/// Appends `bytes` with a `u32` little-endian length in front.
fn frame(out: &mut Vec<u8>, bytes: &[u8]) {
    out.extend_from_slice(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
    out.extend_from_slice(bytes);
}

impl Pool<'_> {
    /// The canonical bytes: a kind word, then the pool's own fields, framed.
    ///
    /// # Errors
    ///
    /// [`FibIdentityRefusal::PoolEmpty`] for a development pool with no name and
    /// [`FibIdentityRefusal::PoolNamesNotSorted`] unless its names are sorted
    /// and unique — one pool has one spelling.
    pub fn bytes(&self) -> Result<Vec<u8>, FibIdentityRefusal> {
        let mut out = Vec::new();
        match self {
            Self::PointInTime {
                membership_digest,
                surface_rule,
            } => {
                frame(&mut out, b"pit");
                frame(&mut out, membership_digest);
                frame(&mut out, surface_rule.as_bytes());
            }
            Self::Development { label, names } => {
                if names.is_empty() {
                    return Err(FibIdentityRefusal::PoolEmpty);
                }
                let next = names.iter().skip(1);
                if let Some(at) = names.iter().zip(next).position(|(a, b)| a >= b) {
                    return Err(FibIdentityRefusal::PoolNamesNotSorted { at: at + 1 });
                }
                frame(&mut out, b"development");
                frame(&mut out, label.as_bytes());
                out.extend_from_slice(
                    &u32::try_from(names.len()).unwrap_or(u32::MAX).to_le_bytes(),
                );
                for name in *names {
                    frame(&mut out, name.as_bytes());
                }
            }
        }
        Ok(out)
    }
}

/// Everything a pooled FIB run is, gathered so the identity is taken before
/// anything is computed. No `Default`: every term is named by the run.
#[derive(Clone, Copy, Debug)]
pub struct FibRun<'a> {
    /// Every value the run's layout offers (`Layout::offered_mask`).
    pub mask: ConditionMask,
    /// The one direction this run searches (operator 16 Oct 16:04 IST).
    pub direction: Search,
    /// The pool's canonical bytes ([`Pool::bytes`]).
    pub pool: &'a [u8],
    /// `zerodha:` and every width tried ([`timeframe_word`]).
    pub timeframe: &'a str,
    /// Every profile value, folded by [`fib_params`].
    pub params: Params,
    /// The BLAKE3 of the per-run descriptor (`super::run_schema_digest`).
    pub schema_digest: [u8; OUT_LEN],
    /// The ordered chain of generation-version digests ([`generation_root`]).
    pub generation_root: [u8; OUT_LEN],
    /// The data tree's root ([`DataTree::root`]).
    pub data_digest: [u8; OUT_LEN],
    /// The schema version (`vocab::fib_cube::FIB_CUBE_VERSION`).
    pub vocab_version: u32,
    /// The commit this ran at.
    pub commit: &'a str,
    /// [`FEED_DEEP`], or [`stage_a_feed`]'s word.
    pub feed: &'a str,
}

/// The identity's direction byte for a search: Long or Short, never
/// Undirected.
#[must_use]
pub const fn direction_of(search: Search) -> Direction {
    match search {
        Search::Long => Direction::Long,
        Search::Short => Direction::Short,
    }
}

/// The params of a FIB run: no `engine::Ladder` is walked, so the ladder's
/// three fields are zero — a claim, not a placeholder — and every profile
/// value is folded through [`Params::with_policy`] in the caller's fixed,
/// documented order.
#[must_use]
pub fn fib_params(profile: &[u64]) -> Params {
    Params {
        min_hits: 0,
        ceiling: 0,
        pair_budget: 0,
        policy: 0,
    }
    .with_policy(profile)
}

/// The timeframe term: `zerodha:` followed by the sorted minutes of every
/// width tried in every version of the chain (R7), as `1min,2min,4min`.
#[must_use]
pub fn timeframe_word(chain: &GenerationChain) -> String {
    let widths: BTreeSet<u16> = chain
        .versions()
        .flat_map(|version| version.parts().timeframes.iter().copied())
        .collect();
    let listed: Vec<String> = widths
        .iter()
        .map(|minutes| format!("{minutes}min"))
        .collect();
    format!("{FEED_STAGE_A}:{}", listed.join(","))
}

/// The stage-A feed word: `zerodha`, plus `+<word>` when an order-flow source
/// is used.
#[must_use]
pub fn stage_a_feed(order_flow: Option<&str>) -> String {
    order_flow.map_or_else(
        || FEED_STAGE_A.to_owned(),
        |word| format!("{FEED_STAGE_A}+{word}"),
    )
}

/// The identity of a pooled FIB run: [`DOMAIN`], then the nine terms, each
/// tagged and length-prefixed, in `CLAUDE.md` §3 rule 3's order.
#[must_use]
pub fn fib_identity(run: &FibRun<'_>) -> RunId {
    let mut hasher = Hasher::new();
    hasher.update(DOMAIN);
    let words = run.mask.words();
    let mut mask = Vec::with_capacity(words.len() * size_of::<u64>());
    for word in words {
        mask.extend_from_slice(&word.to_le_bytes());
    }
    term(&mut hasher, Term::Mask.tag(), &mask);
    term(
        &mut hasher,
        Term::Direction.tag(),
        &[direction_of(run.direction).byte()],
    );
    term(&mut hasher, Term::Instrument.tag(), run.pool);
    term(&mut hasher, Term::Timeframe.tag(), run.timeframe.as_bytes());
    let mut params = Vec::new();
    for value in [
        run.params.min_hits,
        run.params.ceiling,
        run.params.pair_budget,
        run.params.policy,
    ] {
        params.extend_from_slice(&value.to_le_bytes());
    }
    params.extend_from_slice(&run.schema_digest);
    params.extend_from_slice(&run.generation_root);
    term(&mut hasher, Term::Params.tag(), &params);
    term(&mut hasher, Term::DataDigest.tag(), &run.data_digest);
    term(
        &mut hasher,
        Term::VocabVersion.tag(),
        &run.vocab_version.to_le_bytes(),
    );
    term(&mut hasher, Term::Commit.tag(), run.commit.as_bytes());
    term(&mut hasher, Term::Feed.tag(), run.feed.as_bytes());
    RunId::from_digest(hasher.finalize())
}

/// The BLAKE3 of the fixed-number ledger (`vocab::fib_cube::fixed_numbers_bytes`).
#[must_use]
pub fn fixed_numbers_digest() -> [u8; OUT_LEN] {
    hash(&fixed_numbers_bytes())
}

/// The identity of a WINNER candidate (FM-F007): one long run and one short
/// run, the pairing rule's version and the fixed-number ledger's digest, under
/// [`PAIR_DOMAIN`]. Swapping the two runs changes it; so does either side.
#[must_use]
pub fn pair_identity(
    long: &RunId,
    short: &RunId,
    pairing_rule_version: u32,
    fixed_numbers: &[u8; OUT_LEN],
) -> RunId {
    let mut hasher = Hasher::new();
    hasher.update(PAIR_DOMAIN);
    term(&mut hasher, PairTerm::Long.tag(), &long.bytes());
    term(&mut hasher, PairTerm::Short.tag(), &short.bytes());
    term(
        &mut hasher,
        PairTerm::RuleVersion.tag(),
        &pairing_rule_version.to_le_bytes(),
    );
    term(&mut hasher, PairTerm::FixedNumbers.tag(), fixed_numbers);
    RunId::from_digest(hasher.finalize())
}

/// What a node of the data tree is, tagged so a leaf can never pose as a
/// parent or a bagging step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Node {
    /// The root of a tree with no leaf.
    Empty,
    /// A leaf: its key and digest.
    Leaf,
    /// Two equal-height peaks merged.
    Parent,
    /// One step of bagging the peaks, right to left.
    Bag,
}

/// The digest of one node: [`TREE_DOMAIN`], the node's tag, then its parts.
fn node(kind: Node, parts: &[&[u8]]) -> [u8; OUT_LEN] {
    let mut hasher = Hasher::new();
    hasher.update(TREE_DOMAIN);
    hasher.update(&[kind as u8]);
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize()
}

/// An append-only Merkle Mountain Range (FM-F003): perfect trees of
/// decreasing height, merged as equal heights meet, bagged right to left.
/// There is no odd-leaf rule to state — an odd leaf simply stays a peak.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mmr {
    /// The peaks, left to right, each with its height.
    peaks: Vec<(u32, [u8; OUT_LEN])>,
    /// How many leaves were appended.
    leaves: u64,
    /// How many merges every append so far has done: the structural work
    /// counter the bench reads (no static, no atomic; gate 17).
    merges: u64,
}

impl Mmr {
    /// An empty range.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            peaks: Vec::new(),
            leaves: 0,
            merges: 0,
        }
    }

    /// Appends one leaf node's digest: O(log n) merges.
    pub fn append(&mut self, leaf: [u8; OUT_LEN]) {
        let mut carry = (0, leaf);
        while let Some((height, left)) = self.peaks.last().copied()
            && height == carry.0
        {
            self.peaks.pop();
            carry = (height + 1, node(Node::Parent, &[&left, &carry.1]));
            self.merges += 1;
        }
        self.peaks.push(carry);
        self.leaves += 1;
    }

    /// The root: the peaks bagged right to left, or the empty root.
    #[must_use]
    pub fn root(&self) -> [u8; OUT_LEN] {
        let mut peaks = self.peaks.iter().rev();
        let Some((_, last)) = peaks.next() else {
            return node(Node::Empty, &[]);
        };
        peaks.fold(*last, |acc, (_, peak)| node(Node::Bag, &[peak, &acc]))
    }

    /// The peaks, left to right, for the run manifest.
    #[must_use]
    pub fn peaks(&self) -> Vec<[u8; OUT_LEN]> {
        self.peaks.iter().map(|(_, peak)| *peak).collect()
    }

    /// How many leaves were appended.
    #[must_use]
    pub const fn leaves(&self) -> u64 {
        self.leaves
    }

    /// How many merges all appends have done.
    #[must_use]
    pub const fn merges(&self) -> u64 {
        self.merges
    }
}

impl Default for Mmr {
    fn default() -> Self {
        Self::new()
    }
}

/// The root of an ORDER-DEPENDENT tree over plain leaf digests (the chain of
/// generation versions, for one).
#[must_use]
pub fn merkle(leaves: &[[u8; OUT_LEN]]) -> [u8; OUT_LEN] {
    let mut mmr = Mmr::new();
    for leaf in leaves {
        mmr.append(node(Node::Leaf, &[leaf]));
    }
    mmr.root()
}

/// The ordered chain of a direction's generation versions as one digest (R1):
/// [`merkle`] over the BLAKE3 of each version's per-run descriptor, oldest
/// first.
#[must_use]
pub fn generation_root(chain: &GenerationChain) -> [u8; OUT_LEN] {
    let leaves: Vec<[u8; OUT_LEN]> = chain
        .versions()
        .map(|version| hash(&version.run_descriptor()))
        .collect();
    merkle(&leaves)
}

/// What a data-tree leaf digests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LeafKind {
    /// One stock-month of Zerodha minutes (`runner::identity::data_digest`):
    /// stage A.
    Minutes,
    /// One stock-month deep: Zerodha minutes decide and GDFL seconds execute
    /// (`runner::identity::data_digest_with_execution`).
    Deep,
    /// One authority file (calendar, PIT membership, DATA-1 factors, DATA-2
    /// status, OF-1, the charge rate-set version).
    Authority,
}

impl LeafKind {
    /// The kind's canonical name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Minutes => "minutes",
            Self::Deep => "deep",
            Self::Authority => "authority",
        }
    }
}

/// A data-tree leaf's key. Ordered MONTH-MAJOR — year-month first, then leaf
/// kind, then series id — so a new month only appends (FM-F003).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LeafKey {
    /// The month, as `year x 100 + month` (`202503`), or the authority's own
    /// period.
    pub year_month: u32,
    /// What the leaf digests.
    pub kind: LeafKind,
    /// The series (a stock's series id, or an authority's name).
    pub series: String,
}

impl LeafKey {
    /// The key's canonical bytes.
    fn bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.year_month.to_le_bytes());
        frame(&mut out, self.kind.name().as_bytes());
        frame(&mut out, self.series.as_bytes());
        out
    }
}

/// The data tree of a run: an [`Mmr`] of keyed leaves in month-major key
/// order, whose root is the run identity's `data_digest` (FM-F003).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataTree {
    /// The range.
    mmr: Mmr,
    /// The last key appended.
    last: Option<LeafKey>,
}

impl DataTree {
    /// An empty tree.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            mmr: Mmr::new(),
            last: None,
        }
    }

    /// Appends one leaf, after every leaf already in: O(log n).
    ///
    /// # Errors
    ///
    /// [`FibIdentityRefusal::LeafOutOfOrder`] for a key at or before the last
    /// one: a repeated key, or one that would make the root depend on the
    /// order leaves were listed.
    pub fn append(
        &mut self,
        key: LeafKey,
        digest: &[u8; OUT_LEN],
    ) -> Result<(), FibIdentityRefusal> {
        if self.last.as_ref().is_some_and(|last| *last >= key) {
            return Err(FibIdentityRefusal::LeafOutOfOrder { key });
        }
        self.mmr.append(node(Node::Leaf, &[&key.bytes(), digest]));
        self.last = Some(key);
        Ok(())
    }

    /// The tree of leaves listed in ANY order — by a directory walk, by
    /// workers finishing in any order: sorted by key first, so the root is
    /// independent of the listing.
    ///
    /// # Errors
    ///
    /// [`FibIdentityRefusal::LeafOutOfOrder`] for a key listed twice.
    pub fn of_unsorted(
        mut leaves: Vec<(LeafKey, [u8; OUT_LEN])>,
    ) -> Result<Self, FibIdentityRefusal> {
        leaves.sort();
        let mut tree = Self::new();
        for (key, digest) in leaves {
            tree.append(key, &digest)?;
        }
        Ok(tree)
    }

    /// The root.
    #[must_use]
    pub fn root(&self) -> [u8; OUT_LEN] {
        self.mmr.root()
    }

    /// The peaks, left to right, for the run manifest.
    #[must_use]
    pub fn peaks(&self) -> Vec<[u8; OUT_LEN]> {
        self.mmr.peaks()
    }

    /// The range underneath, for its counters.
    #[must_use]
    pub const fn mmr(&self) -> &Mmr {
        &self.mmr
    }
}

impl Default for DataTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
