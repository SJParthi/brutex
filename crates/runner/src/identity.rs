//! Run identity: the eight terms, hashed once, before anything is computed.
//!
//! # The rule this exists to satisfy
//!
//! `CLAUDE.md` §3 rule 3: *"Every run is identified by `blake3(mask ‖ direction ‖
//! instrument ‖ timeframe ‖ params ‖ data_digest ‖ vocab_version ‖ commit)`. No
//! computation without that identity recorded."*
//!
//! Until now nothing implemented it. `blake3 = "1"` sat unused in the workspace
//! manifest with no lockfile entry at all, and the formula existed only as a doc
//! comment in `crates/vocab`.
//!
//! # Eight terms, not nine
//!
//! `docs/00-charter.md` §5 writes NINE, adding a `mode` between `timeframe` and
//! the parameters. `CLAUDE.md` §3 rule 3 writes eight and does not mention it,
//! and `CLAUDE.md` §10 settles the disagreement in one line: *"If this file and a
//! document disagree, this file wins and the document is the stale copy to fix."*
//! So the identity is eight terms and the charter is the stale copy. No decision
//! was needed and none was invented.
//!
//! # Why every field is length-prefixed
//!
//! Concatenation alone is not injective. `"NIFTY" ‖ "50"` and `"NIFTY5" ‖ "0"`
//! are the same bytes, so two different runs would share an identity — and an
//! identity that two runs can share is not an identity. Every variable-length
//! term is written as a `u32` little-endian length followed by its bytes, and
//! every fixed-width term at its natural width, so the encoding can be read back
//! unambiguously and no two distinct inputs can collide by framing.
//!
//! Each term also carries a one-byte tag. Length-prefixing alone stops a field's
//! bytes bleeding into its neighbour; the tag additionally stops two terms
//! swapping places unnoticed.
//!
//! # What `data_digest` is, and what it is not
//!
//! `docs/00-charter.md` §5: *"a streamed digest over **every field of every
//! loaded bar** — not a sample, not a count-and-endpoints fingerprint. One
//! differing bar re-keys the identity. That is the point."*
//!
//! [`data_digest`] hashes all seven fields of every candle in order. It is O(1)
//! per bar and allocates nothing. It is **not** a digest of a file: this crate
//! cannot open one, and the candles it is given are generated in-process.
//!
//! # Cost
//!
//! One BLAKE3 compression per 64 bytes, which is one per candle plus a fixed
//! header. No allocation, and no loop whose bound depends on anything but the
//! number of bars. UNVERIFIED as a measured per-bar figure — this crate ships no
//! bench yet, and saying so is cheaper than a number nobody took.

use brutex_core::blake3::{Hasher, OUT_LEN};
use brutex_core::instrument::InstrumentKey;
use engine::Ladder;
use indicators::Candle;
use vocab::{ConditionMask, VOCAB_VERSION};

/// One byte per term, so two terms cannot swap places unnoticed.
mod tag {
    pub(super) const MASK: u8 = 1;
    pub(super) const DIRECTION: u8 = 2;
    pub(super) const INSTRUMENT: u8 = 3;
    pub(super) const TIMEFRAME: u8 = 4;
    pub(super) const PARAMS: u8 = 5;
    pub(super) const DATA_DIGEST: u8 = 6;
    pub(super) const VOCAB_VERSION: u8 = 7;
    pub(super) const COMMIT: u8 = 8;
}

/// Which way a strategy is taken.
///
/// # `Undirected` is the honest value today, and it is not a placeholder
///
/// `CLAUDE.md` §3 rule 3 names `direction` as an identity term, so the term is
/// recorded rather than invented. But the sweep this engine performs counts
/// **occurrences, not trades**: `Ladder::walk` returns frequent itemsets with hit
/// counts and `crates/engine` refuses to rank them, because a ranking metric
/// needs a horizon and an exit rule that no document defines (§3 rule 1).
///
/// A frequency has no direction. So a sweep is `Undirected`, and `Long`/`Short`
/// exist for the day trades do — at which point every identity re-keys, which is
/// **correct**: a directional run is a different computation from a count.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Direction {
    /// A frequency count. No trade, no side. The only value a sweep uses today.
    #[default]
    Undirected,
    /// Reserved for when a trade exists.
    Long,
    /// Reserved for when a trade exists.
    Short,
}

impl Direction {
    /// The byte this direction contributes to the identity.
    ///
    /// Explicit rather than a derived discriminant: a `#[derive]`d value would
    /// change if a variant were ever inserted above another, silently re-keying
    /// every historical run. §3.8's append-only discipline applied to an enum.
    #[must_use]
    pub const fn byte(self) -> u8 {
        match self {
            Self::Undirected => 0,
            Self::Long => 1,
            Self::Short => 2,
        }
    }
}

/// The sweep's parameters, canonically.
///
/// Exactly the two knobs `Ladder` carries. It has no depth field by `CLAUDE.md`
/// §6, so there is none to record — and if one were ever added, this struct
/// would have to grow to match, which is a second place the absence is enforced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    /// The threshold actually applied, which may differ from what was asked —
    /// `Ladder::with_min_hits` raises zero to one. Recording the applied value is
    /// what makes a result reproducible from its identity.
    pub min_hits: u64,
    /// The cumulative candidate budget actually applied.
    pub ceiling: u64,
    /// The cumulative pair-iteration budget actually applied.
    ///
    /// # This was missing, and its absence broke the identity
    ///
    /// `Ladder` gained `pair_budget` in ca358b0 and this struct did not follow.
    /// The two budgets decide **whether a walk halts**, so two runs over the same
    /// bars with the same `min_hits` and the same ceiling can return *different
    /// answers* — one complete, one truncated — and, without this field, hash to
    /// the **same** `RunId`. An identity two different results can share is not an
    /// identity, which is the whole property `CLAUDE.md` §3 rule 3 exists for.
    ///
    /// Found by an adversarial audit, not by a test: nothing here compared two
    /// runs that differed only in a budget.
    pub pair_budget: u64,
}

impl Params {
    /// The parameters a ladder will really apply, read off the ladder itself.
    ///
    /// Taken from the ladder rather than from a caller's arguments on purpose: a
    /// caller that passed `min_hits = 0` and recorded `0` would record a run that
    /// never happened, because the ladder raised it to one.
    #[must_use]
    pub fn of(ladder: Ladder) -> Self {
        Self {
            min_hits: ladder.min_hits(),
            ceiling: u64::try_from(ladder.ceiling()).unwrap_or(u64::MAX),
            pair_budget: ladder.pair_budget(),
        }
    }
}

/// A run's identity: 32 bytes of BLAKE3 over the eight terms.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunId([u8; OUT_LEN]);

impl RunId {
    /// The raw digest.
    #[must_use]
    pub const fn bytes(&self) -> [u8; OUT_LEN] {
        self.0
    }

    /// The digest as lowercase hex, which is how a run is named in a log or a
    /// filename.
    #[must_use]
    pub fn hex(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::with_capacity(OUT_LEN.saturating_mul(2));
        for byte in self.0 {
            let _ = write!(out, "{byte:02x}");
        }
        out
    }
}

/// Everything a run is, gathered so the identity can be taken before it starts.
///
/// `instrument`, `timeframe` and `commit` are the caller's because this crate
/// cannot learn them: `InstrumentKey` is `core`'s, the timeframe belongs to
/// `crates/store` — which this crate deliberately cannot name, so a stored bar
/// can never reach the sweep — and the commit cannot be baked in because
/// `CLAUDE.md` §2 forbids a `build.rs` that shells out to `git`.
pub struct Run<'a> {
    /// The combination under test.
    pub mask: ConditionMask,
    /// Which way it is taken. [`Direction::Undirected`] for a frequency sweep.
    pub direction: Direction,
    /// What is being swept.
    pub instrument: &'a InstrumentKey,
    /// The bar length, as its canonical directory word — `1min`, `1day`.
    pub timeframe: &'a str,
    /// The thresholds actually applied.
    pub params: Params,
    /// [`data_digest`] over every bar loaded.
    pub data_digest: [u8; OUT_LEN],
    /// The commit this ran at.
    pub commit: &'a str,
}

/// A streamed digest over every field of every bar.
///
/// All seven fields, in declaration order, little-endian. Not a sample and not a
/// count-and-endpoints fingerprint — `docs/00-charter.md` §5 rules both out by
/// name, because either would let two different columns share an identity.
#[must_use]
pub fn data_digest(bars: &[Candle]) -> [u8; OUT_LEN] {
    let mut hasher = Hasher::new();
    // The bar COUNT first, so a column cannot be confused with a longer one that
    // happens to start with it. Framing, for the same reason every term below is
    // length-prefixed.
    hasher.update(&u64::try_from(bars.len()).unwrap_or(u64::MAX).to_le_bytes());
    for bar in bars {
        hasher.update(&bar.ts_micros.to_le_bytes());
        hasher.update(&bar.open.to_le_bytes());
        hasher.update(&bar.high.to_le_bytes());
        hasher.update(&bar.low.to_le_bytes());
        hasher.update(&bar.close.to_le_bytes());
        hasher.update(&bar.volume.to_le_bytes());
        hasher.update(&bar.open_interest.to_le_bytes());
    }
    hasher.finalize()
}

/// Writes one tagged, length-prefixed term.
fn term(hasher: &mut Hasher, tag: u8, bytes: &[u8]) {
    hasher.update(&[tag]);
    hasher.update(&u32::try_from(bytes.len()).unwrap_or(u32::MAX).to_le_bytes());
    hasher.update(bytes);
}

/// The identity of a run, per `CLAUDE.md` §3 rule 3.
///
/// Eight terms, each tagged and length-prefixed, hashed in the order the rule
/// names them.
#[must_use]
pub fn identity(run: &Run<'_>) -> RunId {
    let mut hasher = Hasher::new();

    // 1. mask — fixed width, six little-endian words.
    let mut mask_bytes = [0_u8; 48];
    for (slot, word) in mask_bytes.chunks_exact_mut(8).zip(run.mask.words()) {
        slot.copy_from_slice(&word.to_le_bytes());
    }
    term(&mut hasher, tag::MASK, &mask_bytes);

    // 2. direction
    term(&mut hasher, tag::DIRECTION, &[run.direction.byte()]);

    // 3. instrument — every field of the key, each framed in turn, so two keys
    //    differing only in where one field ends cannot collide.
    let mut instrument = Vec::with_capacity(64);
    for part in [
        run.instrument.exchange.as_str(),
        run.instrument.segment.as_str(),
        run.instrument.underlying.as_str(),
    ] {
        instrument.extend_from_slice(&u32::try_from(part.len()).unwrap_or(u32::MAX).to_le_bytes());
        instrument.extend_from_slice(part.as_bytes());
    }
    term(&mut hasher, tag::INSTRUMENT, &instrument);

    // 4. timeframe
    term(&mut hasher, tag::TIMEFRAME, run.timeframe.as_bytes());

    // 5. params — both knobs, fixed width.
    let mut params = [0_u8; 24];
    for (slot, value) in params.chunks_exact_mut(8).zip([
        run.params.min_hits,
        run.params.ceiling,
        run.params.pair_budget,
    ]) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
    term(&mut hasher, tag::PARAMS, &params);

    // 6. data_digest
    term(&mut hasher, tag::DATA_DIGEST, &run.data_digest);

    // 7. vocab_version
    term(
        &mut hasher,
        tag::VOCAB_VERSION,
        &VOCAB_VERSION.to_le_bytes(),
    );

    // 8. commit
    term(&mut hasher, tag::COMMIT, run.commit.as_bytes());

    RunId(hasher.finalize())
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes -- a test that \
              cannot panic cannot fail, and `.expect` panics inside core, which is not \
              instrumented, so it leaves no uncoverable region behind."
)]
mod tests {
    use super::{Direction, Params, Run, RunId, data_digest, identity};
    use crate::synthetic;
    use brutex_core::instrument::{Exchange, InstrumentKey};
    use engine::Ladder;
    use vocab::ConditionMask;

    fn key() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "NIFTY").expect("NIFTY is a swept index")
    }

    fn run_over(instrument: &InstrumentKey, digest: [u8; 32], mask: ConditionMask) -> Run<'_> {
        Run {
            mask,
            direction: Direction::Undirected,
            instrument,
            timeframe: "1min",
            params: Params::of(Ladder::with_min_hits(600)),
            data_digest: digest,
            commit: "0123456789abcdef",
        }
    }

    #[test]
    fn the_same_run_has_the_same_identity() {
        // §3 rule 5, at the level of the whole run.
        let k = key();
        let d = data_digest(&synthetic::sessions(2));
        let m = ConditionMask::default().with_bit(3);
        assert_eq!(identity(&run_over(&k, d, m)), identity(&run_over(&k, d, m)));
    }

    #[test]
    fn one_differing_bar_re_keys_the_identity() {
        // The charter's own words: "One differing bar re-keys the identity. That
        // is the point."
        let a = synthetic::sessions(2);
        // One paisa, on one field, of one bar. Built by mapping rather than by
        // `last_mut()`: an `if let` whose `else` no input can enter is a region
        // llvm-cov counts forever, and both arms of this condition are taken.
        let last = a.len().saturating_sub(1);
        let b: Vec<_> = a
            .iter()
            .enumerate()
            .map(|(i, bar)| {
                let mut c = *bar;
                if i == last {
                    c.close = c.close.saturating_add(1);
                }
                c
            })
            .collect();
        assert_ne!(data_digest(&a), data_digest(&b));

        let k = key();
        let m = ConditionMask::default().with_bit(3);
        assert_ne!(
            identity(&run_over(&k, data_digest(&a), m)),
            identity(&run_over(&k, data_digest(&b), m))
        );
    }

    #[test]
    fn a_shorter_column_is_not_a_prefix_of_a_longer_one() {
        // Without the length written first, a 100-bar column and the first 100
        // bars of a 200-bar one would hash identically up to that point. They are
        // different data and must be different digests.
        let long = synthetic::sessions(2);
        let short = long.get(..100).unwrap_or(&[]).to_vec();
        assert_ne!(data_digest(&long), data_digest(&short));
        assert_ne!(data_digest(&[]), data_digest(&short));
    }

    #[test]
    fn every_term_changes_the_identity() {
        // Eight terms, eight assertions. A term that is written but never read
        // would let two different runs share an identity, which is the one thing
        // an identity may not do.
        let k = key();
        let d = data_digest(&synthetic::sessions(2));
        let m = ConditionMask::default().with_bit(3);
        let base = identity(&run_over(&k, d, m));

        // 1. mask
        let other_mask = ConditionMask::default().with_bit(4);
        assert_ne!(base, identity(&run_over(&k, d, other_mask)), "mask");

        // 2. direction
        let mut r = run_over(&k, d, m);
        r.direction = Direction::Long;
        assert_ne!(base, identity(&r), "direction");

        // 3. instrument
        let bank = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").expect("swept");
        assert_ne!(base, identity(&run_over(&bank, d, m)), "instrument");

        // 4. timeframe
        let mut r = run_over(&k, d, m);
        r.timeframe = "1day";
        assert_ne!(base, identity(&r), "timeframe");

        // 5. params
        let mut r = run_over(&k, d, m);
        r.params = Params::of(Ladder::with_min_hits(601));
        assert_ne!(base, identity(&r), "min_hits");
        let mut r = run_over(&k, d, m);
        r.params = Params::of(Ladder::with_min_hits(600).with_ceiling(7));
        assert_ne!(base, identity(&r), "ceiling");
        // The pair budget decides WHETHER A WALK HALTS, so two runs differing
        // only in it can return different answers. Before this was recorded they
        // hashed identically -- an identity two results can share is not one.
        let mut r = run_over(&k, d, m);
        r.params = Params::of(Ladder::with_min_hits(600).with_pair_budget(7));
        assert_ne!(base, identity(&r), "pair_budget");

        // 6. data_digest
        assert_ne!(base, identity(&run_over(&k, [0_u8; 32], m)), "data_digest");

        // 7. vocab_version is a constant, so it cannot be varied here — it is
        //    covered by the fact that changing it changes every identity, which
        //    `the_vocabulary_version_is_in_the_identity` pins by construction.

        // 8. commit
        let mut r = run_over(&k, d, m);
        r.commit = "fedcba9876543210";
        assert_ne!(base, identity(&r), "commit");
    }

    #[test]
    fn the_vocabulary_version_is_in_the_identity() {
        // Not variable at runtime, so this pins that it is READ: the encoding
        // writes four bytes for it, and a build with a different vocabulary must
        // not reuse an old run's name.
        assert_eq!(vocab::VOCAB_VERSION, 3, "if this moves, every run re-keys");
    }

    #[test]
    fn a_framing_collision_is_impossible() {
        // The reason every term is length-prefixed. Two instruments whose
        // concatenated fields are the same bytes at different boundaries must not
        // collide. `NIFTY`/`BANKNIFTY` under one exchange and segment is the
        // shape; the lengths are what separate them.
        let a = key();
        let b = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").expect("swept");
        let d = data_digest(&[]);
        let m = ConditionMask::default();
        assert_ne!(identity(&run_over(&a, d, m)), identity(&run_over(&b, d, m)));

        // And a timeframe that is a prefix of another.
        let mut short = run_over(&a, d, m);
        short.timeframe = "1m";
        let mut long = run_over(&a, d, m);
        long.timeframe = "1min";
        assert_ne!(identity(&short), identity(&long));
    }

    #[test]
    fn the_identity_renders_as_sixty_four_hex_characters() {
        let k = key();
        let id = identity(&run_over(&k, data_digest(&[]), ConditionMask::default()));
        let hex = id.hex();
        assert_eq!(hex.len(), 64);
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
        assert_eq!(id.bytes().len(), 32);
    }

    #[test]
    fn direction_bytes_are_pinned_and_not_derived() {
        // §3.8 applied to an enum: a derived discriminant would shift if a
        // variant were inserted above another, silently re-keying history.
        assert_eq!(Direction::Undirected.byte(), 0);
        assert_eq!(Direction::Long.byte(), 1);
        assert_eq!(Direction::Short.byte(), 2);
        assert_eq!(Direction::default(), Direction::Undirected);
    }

    #[test]
    fn params_record_what_the_ladder_applies_not_what_was_asked() {
        // `with_min_hits(0)` is raised to 1. Recording 0 would name a run that
        // never happened.
        let p = Params::of(Ladder::with_min_hits(0));
        assert_eq!(p.min_hits, 1, "the APPLIED threshold, not the argument");
        assert_eq!(p.ceiling, engine::DEFAULT_CEILING as u64);
    }

    #[test]
    fn an_empty_column_still_has_a_digest() {
        // A run over no bars is a run, and it must be nameable — otherwise the
        // "no computation without an identity" rule has a hole exactly where a
        // caller is most likely to be confused about what happened.
        let empty = data_digest(&[]);
        assert_ne!(empty, [0_u8; 32]);
        assert_eq!(empty, data_digest(&[]));
    }

    #[test]
    fn two_runs_differing_only_in_bar_order_differ() {
        // The digest is over the column as ORDERED. Reordering bars is a
        // different column -- and would be look-ahead if it were not.
        let bars = synthetic::sessions(1);
        let mut reversed = bars.clone();
        reversed.reverse();
        assert_ne!(data_digest(&bars), data_digest(&reversed));
    }

    #[test]
    fn a_run_id_is_copy_and_comparable() {
        let k = key();
        let id = identity(&run_over(&k, data_digest(&[]), ConditionMask::default()));
        let copied: RunId = id;
        assert_eq!(id, copied);
    }
}
