# eng1: numeric, look-ahead and O(1) audit of crates/engine/src/** at 331b05c

## Verdict

NO NEW FINDINGS. The engine holds no prices, floats or statistics. Every support
counter is `u64`, combined with `saturating_add`, and cannot overflow: support is
at most the bar count. The engine does not see time order, so it cannot look ahead.
The non-O(1) operations I found are already documented, in `docs/06-limits.md`
§1 (support is Θ(bars) per candidate), at 06-limits:12124 (the subset prune is
Θ(k) per candidate, D-0924) and at 06-limits:12143/13998 (`keep::Best` is
O(log cap) plus a hash probe). CLAUDE.md on disk at this commit already corrects
the stale "lane-local `kept` + `extend`" paragraph (D-1440). The copy of CLAUDE.md
injected into this session predates that correction. The on-disk text matches
`drain`.

## Findings

None. No probe was needed. Each item below was verified by reading the code. No
measurement is claimed.

## Checked and clean

- `column.rs` `Column::support` (:106 approx): one `row.hits(candidate)` per
  row, folded with `u64` `saturating_add`. There is no u32 counter and no work
  per candidate bit. `bars()` uses `u64::try_from(len).unwrap_or(u64::MAX)`.
- `column.rs` `support_fingerprinted`: the u64 packing `u64::from(hit) << bit`
  has bit < 64 (chunks of 64), so it cannot overflow. Tail padding stays zero.
  It has no production caller (pinned by D-0760).
- `column.rs` `set_positions`: base `index*64` is ≤ 320 and bit is < 64, so
  there is no overflow.
- `lib.rs` `first_level`: k=1 dedup is one `HashSet<u32>::insert` per offered
  position, pre-sized with `try_reserve(live.len())`. The bounds check
  `p >= BITS`, the liveness check through `u16::try_from(p).is_ok_and(is_live)`
  (`definition()` returns `None` out of range), then the AlwaysFalse check runs
  before AlwaysTrue, so bars=0 never reports AlwaysTrue. These match the resume
  validator, which requires `bars > 0` for AlwaysTrue.
- `lib.rs` `try_next_level_observing`: the pair budget is checked (`>=`) before
  each pair. The ceiling is checked through `admitted+emitted >= ceiling` before
  each increment, so `admitted+emitted <= ceiling` always holds and a
  `Breach::Candidates` halt carries exactly `ceiling`. That is the invariant
  `resume::validate_halt` requires. On a drain failure, `generated` and
  `emitted` are both decremented by the lost batch, so `reconciles()` still
  holds. All counters are `u64`/`usize` saturating.
- `lib.rs` `drain`: `width = ceil(len/lanes).max(1)`, so there is no divide by
  zero (`lane_count.max(1)`). `out.try_reserve(batch.len())` runs before the
  serial push loop, so `primitives::append` never allocates. Counts go into
  disjoint `chunks_mut`. The result is independent of the lane count because
  `out` is sorted canonically afterwards.
- `lib.rs` `every_non_parent_subset_is_frequent` / `join_screen` /
  `parents_informative`: the prefix join makes x and y the two highest bits. The
  `take(popcount-2)` skips exactly the two parents. The `u16::try_from(pos)` has
  pos < 384. Informativeness carries inductively to every pair through the subset
  prune.
- `lib.rs` `highest_position` / `without_highest`:
  `63 - leading_zeros` only on a nonzero word.
- `lib.rs` `Ladder::with_*`: min_hits, ceiling, pair_budget and lanes of 0 are
  raised to 1. `support_lanes()` clamps to `available_parallelism`, which is
  called once per level, not per candidate.
- `resume.rs` codec: HEADER_BYTES 192 = 8+16+32+56+56+24, and LEVEL/ITEM 56 =
  7 words. `count()` bounds use `remaining / stride` (stride is nonzero). The
  admitted sum is `checked_add`. `live.len() - offered.len()` cannot underflow
  because offered ⊆ live. The survivor check `hits >= bars` is consistent with
  the engine, because a k≥2 union of non-always-true bits has hits < bars.
  Checkpoints are unsealed by design (known: attacksweep-5).
- `keep.rs` `Best`: it ranks by integer `(hits, mask.words())` with no float and
  no NaN, and ties break deterministically. `Tally::reconciles` saturates.
- Look-ahead: the engine takes only boolean row masks and never reads prices,
  timestamps or VIX. Support and the D-0080 exclusions are measured over the
  whole loaded column. That is in-sample selection by design (CLAUDE.md §1
  requires out-of-sample validation downstream). It is not a bar-N leak inside
  the engine.
