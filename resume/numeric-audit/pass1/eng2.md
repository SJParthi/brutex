# eng2: Apriori join / subset prune (crates/engine) + crates/vocab, numeric pass 1

Commit 331b05c. Audit only.

## Verdict

NO NEW FINDINGS. The slice has no floating point at all (no `f32`/`f64` outside tests). Every non-test `as` cast is a split or a const-bounded narrowing. The join, the subset prune, the implication prune, the mask algebra, the expression evaluator and the grammar cursor all held up under adversarial reading and under a probe that actually ran. The non-O(1) items in this area are already documented: the Θ(k) subset prune, the per-level sorts, `Expression::evaluate` at Θ(len), `Best::offer` at O(log cap), and the cursor's sibling compare. The code matches what those entries say (see o1eng2.md Part B rows e-9, e-11/12, e-20, e-22, e-23).

## Probe (ran)

`$SCRATCH/probes/eng2` has path dependencies on `crates/engine` and `crates/vocab`, built with `CARGO_TARGET_DIR=$SCRATCH/target`.

1. **Apriori against brute force, implication prune included.** 60 random columns of 50 to 349 bars, with min_hits from 1 to 20. The brute force goes over every subset of the offered alphabet. It drops always-true and always-false singletons, drops any set holding a non-informative pair (`vocab::implication::pair_is_informative`), and keeps a set when support >= min_hits.
2. **`Cursor` checkpoint round trip.** The probe encodes and decodes the cursor after every `advance` call, with budgets of 1 to 7 nodes, for 20,000 calls. Each emitted `Expression` also goes through `encode` then `decode`.
3. **Commutative canonical parse.** Pairs of sources that differ only in the order of commutative siblings should encode to the same bytes.

Verbatim output:
```
live used: [0, 1, 2, 3, 60, 61, 74, 75, 76, 77, 80, 81, 300]
apriori vs brute force: cases=60 kept_total=21214 mismatches=0
cursor: candidates=6718 work=65340 roundtrip_fail=0 decode_refused=0
parse "0 & 1" == "1 & 0": true
parse "(0 | 60) & 1" == "1 & (60 | 0)": true
parse "!0 | 1 & 60" == "60 & 1 | !0": true
```
Second run: an alphabet spread across all six mask words, passed unsorted, with word-boundary bits and pivot-chain bits 182 and 186. Bits 255 and 256 were filtered out as not live.
```
live used: [5, 63, 64, 127, 128, 191, 192, 319, 320, 369, 182, 186]
apriori vs brute force: cases=60 kept_total=78919 mismatches=0
```

## Checked and clean

- `ConditionMask` (vocab/src/mask.rs)
  - `with_bit`, `without_bit` and `get` match on `b >> 6` over all 6 words and ignore positions >= 384. Every caller validates positions first: `first_level` checks `p >= BITS || !is_live` and names it `Why::NotLive`; `Cursor::new`, `Expression::decode` and `parse` refuse non-live bits.
  - `hits` is branchless, 6/6/5 ops. `popcount`, `union`, `intersect` and `is_empty` each cover all six words. `WIDTH_IS_SUFFICIENT` is a const assert that COUNT (370) <= 384.
- **Join** (`try_next_level_observing`, `JoinIndex::try_new`, lib.rs:1783-2135)
  - The prefix grouping by `without_highest` is injective, so no dedup set is needed. Verified by brute-force equality, including bits in words 0 to 5.
  - The sort key `(prefix.words(), mask.words())` makes equal prefixes contiguous for `chunk_by`.
  - The pair budget is checked before each pair is counted. The ceiling (`exhausted`) is checked before each pair is admitted. `cannot_grow` is a capacity test, amortised O(1).
  - On a drain failure, the `generated`/`emitted` rollback matches what `resume::validate` reconciles.
- **Subset prune** (`every_non_parent_subset_is_frequent`)
  - It skips exactly the two highest bits. Those are the parents' distinguishing bits h_i and h_j, because every block member is P ∪ {h} with h > max(P).
  - The cost is k−2 MaskSet probes, Θ(k), already documented.
- **Implication prune** (`parents_informative`, vocab/src/implication.rs)
  - Only the (h_i, h_j) pair needs checking at level k. Every other pair lies inside a parent that is already in the frontier, and informativeness is anti-monotone.
  - Checked that `implies` and `compatible` are exact under the integer arithmetic in indicators `DailyLevels::from_previous_session`:
    - p = floor((H+L+C)/3) stays in [L, H].
    - r1 ≥ p ≥ s1, r2 ≥ r1, r3 ≥ r2, r4 = r3+(r2−r1) ≥ r3, r5 ≥ r4. The s side mirrors this.
    - `cpr_span().1 = p + half` exactly, so rung 5 (60/61, bare edges) sits between s1 ± half and r1 ± half.
    - `saturating_add`/`saturating_sub` on the bands keep the order.
  - So the prune never removes a genuinely informative pair.
- `MaskHasher`: wrapping u128 multiply-fold. It affects only how a membership probe is distributed; correctness rests on full-key equality. Fixed seed, deterministic.
- `drain`: each lane writes its own disjoint `counts` chunk, then one serial append. `width = ceil(batch/lanes)`, and `lane_count.max(1)` rules out a division by zero.
- `Column::support` and `crate::support` use a saturating u64 count. In `support_fingerprinted`, the `sum` of distinct shifted bits equals an OR, with no overflow. It has no production caller.
- `first_level`: dedup comes before the liveness check, AlwaysFalse/AlwaysTrue is decided against `bars` exactly, and bars == 0 reports every position as AlwaysFalse.
- `keep::Best`: the strict total order `(hits, words)` breaks ties deterministically, and the masks set stays in step through evictions and refusals. The heap at O(log cap) is documented.
- `resume`:
  - Decoder counts are bounded by the remaining bytes, with `count/stride` division and no multiply.
  - `validate_level` uses `checked_add` reconciliation, requires popcount == k, refuses `hits >= bars` (which is correct for k ≥ 2 and for the k = 1 AlwaysTrue exclusion), and requires strictly ascending canonical order.
  - Halt cross-checks pass.
- `Expression` (vocab/src/expression.rs)
  - The scratch tiers 8/64/576 cover the maximum stack height ceil(len/2) at every length.
  - `decode` checks canonical sibling order with the same `Instruction` Ord the parser's `binary` rotation uses.
  - Evaluation uses strong-Kleene logic: unknown never becomes a signal, and an undersized stack can only answer `Unknown`.
  - `encode` uses `u16::try_from(len)`, and len ≤ 1151 by construction.
- `Cursor` (vocab/src/expression_search.rs)
  - `count + 3` ≤ 387 and `length` ≤ 1151, so there is no u16 overflow. `place` is checked; `depth − 1` is safe because every accepted arm yields depth ≥ 1.
  - On backtrack, `starts`/`depths` are reset. `decode` refuses out-of-range digits and rebuilds `starts`/`depths` with the same `place`.
  - Round trip confirmed by the probe.
- `table::index_of`: FNV-1a with linear probing over 2048 slots at load ≤ 0.18. It ends at the first empty slot and is bounded by `MAX_NAME_BYTES` up front.
- `Tolerance::covers` works in i128. `milli * range` ≤ 2^126 and `delta * 1000` ≤ 2^74, so there is no overflow. A non-positive range refuses.
- Look-ahead: none in scope. The engine only counts support over the column it is handed. In-sample vs out-of-sample separation lives with the callers in runner and cli, which are other slices.
