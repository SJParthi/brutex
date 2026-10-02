# Cloud fix lane 2

Handoff only. Branch `fix-queue` is never merged. Source: the batch-2 audit queue (state/c4/wave2-after-merge.json on the local drive), 2 Oct 2026. Base every fix on origin/main.

**Done means:** a test that fails before the fix and passes after; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` and the CI gates green; for a cost finding, the operation is O(1) or the honest bound is named in docs/06-limits.md. One branch and one PR per item, named `fix/cloud-<id>`.

## ET-masks-evaluation-sweep-8 · medium cost · engine

**Where:** `crates/engine/benches/ratio.rs:475`

**Finding:** Claimed: docs/04-invariants.md C-E-08: 'One pair of the level join costs the same whatever the frontier holds'. DEFAULT_PAIR_BUDGET doc (lib.rs:760): per-pair cost is 'a mask union and a popcount, measured by C-E-08' Actual: The bench times a stand-in `a.union(b).popcount()` loop. The production pair also runs exhausted(), the O(k) subset prune, parents_informative and batch.push, and its cost rises with k and with frontier size (cache)

**Evidence:**
  - crates/engine/benches/ratio.rs:476-497 builds a Vec of single-bit masks and times an all-pairs loop of `a.union(b).popcount()`. Its comment at ratio.rs:478 says it joins "exactly as `next_level` does: union, then popcount". That is false today, because production has no popcount per pair: lib.rs:1848-1854 says "No popcount filter, because the grouping already IS that filter".
  - crates/engine/benches/ratio.rs:475-505 (C-E-08) builds single-bit masks (482) and times only `black_box(a).union(black_box(b)).popcount()` over all pairs. `every_subset_is_frequent` (2312-2314) is `set_positions(cand).all(|b| frequent.contains(&cand.without_bit(b)))`. Its doc says "Walks the SET bits, k of them" (2157) and "has no isolated bench row" / UNVERIFIED (2177-2182).

**Expected fix and test:** None

## W3-engine1-1 · medium cost · engine

**Where:** `crates/engine/src/lib.rs:2024`

**Finding:** JoinIndex::try_new and sort_canonically (per-level re-sorts) (crates/engine/src/lib.rs:2024): per per level (per frontier member), the cost is O(|F| log |F|) per level, i.e. O(log |F|) comparisons of 12-word (keyed) or 7-word (out) keys per member; it grows with previous frontier width |F_{k-1}| (keyed sort) and new survivor count |F_k| (output sort); offered positions at k=1. Auditor verdict: undocumented-scan. Documented: In code only: lib.rs:1792-1796 says 'So the key is computed and sorted on explicitly here: |F| log |F| per level against the |F|^2/2 it removes.' docs/06-limits.md has nothing (grep JoinIndex|sort_canonically|log |F| ret ...

**Evidence:**
  - lib.rs:2024: `keyed.sort_unstable_by_key(|(prefix, mask)| (prefix.words(), mask.words()));` sorts |F_{k-1}| pairs of 12-word keys. JoinIndex::try_new is called once per level at lib.rs:1701. lib.rs:1997: `sort_canonically(&mut out);` sorts the level's survivors. lib.rs:2322 is `v.sort_unstable_by_key(|i| (i.mask.words(), i.hits));`.
  - lib.rs:2024 in `JoinIndex::try_new`: `keyed.sort_unstable_by_key(|(prefix, mask)| (prefix.words(), mask.words())); keyed.dedup();` lib.rs:1997 `sort_canonically(&mut out);`, with lib.rs:2322 `v.sort_unstable_by_key(|i| (i.mask.words(), i.hits));` The code states this itself at lib.rs:1792-1796: "|F| log |F| per level against the |F|²/2 it removes".

**Expected fix and test:** None

## ET-masks-evaluation-sweep-3 · low bug · engine

**Where:** `CLAUDE.md:150`

**Finding:** Documentation and law drift. CLAUDE.md's result-append paragraph describes a lane-local `kept` plus `extend` in drain that does not exist; drain pushes serially (lib.rs:2301-2307). Engine comments still describe the removed `seen` set and a `parts` vector. every_subset_is_frequent's doc talks of a '384-probe loop' and an 'O(1) bound', but it walks k set bits. 06-limits §5 prices a `seen` set that is gone and names HashSet::try_reserve. cli batch.rs calls DEFAULT_CEILING '1 << 26 ... 8 GiB', but it is 1 << 27 (lib.rs:742).

**Evidence:**
  - In C2 crates/engine/src/lib.rs:2272-2310, drain does `out.try_reserve(batch.len())` (2284). Workers write into disjoint `counts` slices (2288-2293). Then one serial loop `out.push` (2301-2307), with the comment "No worker owns a growing result vector" (2299-2300). CLAUDE.md:150-154 still says `Vec::extend` at k>=2 and describes a lane-local `kept` folded with "one `extend` per chunk".
  - The code itself is O(1): drain pushes into `out` after reserving room for the whole batch (lib.rs:2284 `out.try_reserve(batch.len())`), so this is not an O(1) violation. CLAUDE.md:150-155 is wrong. Drain (lib.rs:2272-2310) spawns scoped workers that write support counts into disjoint `counts` slices (lib.rs:2287-2298). It then pushes survivors into `out` one at a time, in candidate order (lib.rs:2301-2307).

**Expected fix and test:** None

## ET-o1-proof-coverage-0 · low bug · engine

**Where:** `crates/engine/benches/ratio.rs:346`

**Finding:** The Gate 8 rows for duplicate rejection and result append time stand-in collections, never the engine code. An O(n) regression in either production operation ships with Gate 8 green.

**Evidence:**
  - crates/engine/benches/ratio.rs:346-353 `offered_of` builds its own `std::collections::HashSet<u32>`. C-E-10 (ratio.rs:370-409) times `insert(0)` on that bench-local set. C-E-11 (ratio.rs:427-457) pushes into a bench-local `Vec<Itemset>`. The production operations are elsewhere: `offered` HashSet at crates/engine/src/lib.rs:1405, `insert` at :1426, `out.push` in `drain` at :2303.
  - In crates/engine/benches/ratio.rs, C-E-10 builds its own `std::collections::HashSet<u32>` (`offered_of`, :346-353) and times `insert(0)` on it (:380-391). C-E-11 pushes into a local `Vec<Itemset>` (:431-441). Neither calls `Ladder::first_level` (production dedup at crates/engine/src/lib.rs:1405-1429) or `drain` (production append at lib.rs:2302-2308).

**Expected fix and test:** None

## ET-masks-evaluation-sweep-5 · low bug · engine

**Where:** `crates/engine/src/lib.rs:1829`

**Finding:** The pair budget is checked once per outer row, so a halt can report pairs above the budget by up to block size minus 1. With every bit frequent at k=2, the block is the whole frontier. This is not a correctness defect, but the budget is soft, and neither Halt::pairs nor docs/06-limits.md says so.

**Evidence:**
  - CODE: crates/engine/src/lib.rs:1827-1837 checks `pairs_walked + pairs >= self.pair_budget` once, before each outer row `a` of a prefix block. The inner loop at lib.rs:1838-1839 then walks the whole row (block.len()-1-offset pairs) with no further budget check. JoinIndex::try_new is at lib.rs:2012-2025. Positions 6, 19, 25 and 235-241 among others were excluded as NotLive, which is why |F1| stops at 328.
  - CODE (C2 @ 9f5c13c7): crates/engine/src/lib.rs:1829 checks `pairs_walked.saturating_add(pairs) >= self.pair_budget` once, before each outer row of `keyed.chunk_by(...)`. The inner loop at lib.rs:1837-1838 then runs `pairs = pairs.saturating_add(1)` for every remaining block member without checking again. The walk loop is at lib.rs:1520-1562.

**Expected fix and test:** None

## ET-masks-evaluation-sweep-6 · low bug · engine

**Where:** `crates/engine/src/lib.rs:2063`

**Finding:** The doc says 'no branch on the answer', but the body is `if b.hits(mask) { n.saturating_add(1) } else { n }`. The function is test/bench-only.

**Evidence:**
  - crates/engine/src/lib.rs:2061-2071 at 9f5c13c7. The doc (2063-2064) says "no allocation, no early exit, no branch on the answer". The body (2067-2070) is `bar_bits.iter().fold(0_u64, |n, b| if b.hits(mask) { n.saturating_add(1) } else { n })`, which is an if/else in the source. lib.rs:3276, 4548, 4562, 4639, 4853, all after `mod tests` at lib.rs:2376.
  - The doc comment at crates/engine/src/lib.rs:2063-2064 says "no allocation, no early exit, no branch on the answer". The body at lib.rs:2067-2070 is `bar_bits.iter().fold(0_u64, |n, b| if b.hits(mask) { n.saturating_add(1) } else { n })`, which is a source-level branch on the answer.

**Expected fix and test:** None

## ET-o1-proof-coverage-2 · low bug · engine

**Where:** `crates/engine/src/lib.rs:2312`

**Finding:** The per-candidate subset prune does k hash probes. Its cost grows 101x from k=2 to k=328. There is no Gate 8 row; the code comment says 'UNVERIFIED' and docs/06-limits.md does not mention it.

**Evidence:**
  - crates/engine/src/lib.rs:2312-2314 is `set_positions(cand).all(|b| frequent.contains(&cand.without_bit(b)))`. crates/engine/src/lib.rs:1904 calls it once for every emitted candidate, which is once per join pair. The function's own doc is at lib.rs:2155-2187. It sits before `BATCH_PER_LANE` (lib.rs:2216), so it is attached to the wrong item. lib.rs:2157 says it "Walks the SET bits, k of them".
  - crates/engine/src/lib.rs:2312-2314 is `set_positions(cand).all(|b| frequent.contains(&cand.without_bit(b)))`. `set_positions` (crates/engine/src/column.rs:39-57) yields each set bit, so a candidate that passes costs popcount = k hash probes. The call site is lib.rs:1904, once per emitted candidate. A grep of crates/engine/benches/ratio.rs for subset/prune finds only C-E-04 (ratio.rs:596-640).

**Expected fix and test:** None

## W3-engine1-0 · low cost · engine

**Where:** `crates/engine/src/lib.rs:2312`

**Finding:** every_subset_is_frequent (crates/engine/src/lib.rs:2312): per per candidate emitted by the prefix join (called at lib.rs:1904), the cost is Theta(k) expected hash probes per candidate, k <= ConditionMask::BITS = 384; it grows with candidate depth k: one MaskSet probe per set bit (each probe hashes 7 words through a 128-bit multiply, lib.rs:224-237 and 282-285, then compares 48 bytes). Auditor verdict: false-o1-claim. Documented: Contradictory. The walk doc at lib.rs:1080-1082 says 'up to `k` subset probes (O(k), and `k` is bounded by the vocabulary width)'.

**Evidence:**
  - *The cost grows with k.** lib.rs:2312-2314 is `fn every_subset_is_frequent(cand: &ConditionMask, frequent: &MaskSet) -> bool { set_positions(cand).all(|b| frequent.contains(&cand.without_bit(b))) }`. `set_positions` (column.rs:39-57) yields exactly popcount = k positions. Each probe hashes 7 words, the length prefix plus 6, through a 128-bit multiply (lib.rs:224-237, 282-285).
  - (1) lib.rs:2312-2314 reads `fn every_subset_is_frequent(cand: &ConditionMask, frequent: &MaskSet) -> bool { set_positions(cand).all(|b| frequent.contains(&cand.without_bit(b))) }`. set_positions (column.rs:39-57) yields each set bit once, so a candidate that survives makes k MaskSet probes. Each probe hashes the 56-byte key through MaskHasher::write/write_u64 (lib.rs:224-237, 282-285).

**Expected fix and test:** None

## W3-engine1-3 · low bug · engine

**Where:** `crates/engine/src/lib.rs:1829`

**Finding:** Ladder::try_next_level_observing (pair budget check) (crates/engine/src/lib.rs:1829): Off-by-one at the pair budget: a walk whose pair need equals the budget exactly is reported as halted, with a partial level and Breach::Pairs, although the level is complete. The walk then stops short of extinction. Code path: The check `if pairs_walked.saturating_add(pairs) >= self.pair_budget` runs at the START of every outer row, including the last row of a block, which has no pairs to walk. So once the final pair brings the count to the budget, the next row halts. Probe run (throwaway worktree at 29025433): bars {0,1,2},{0,1,2},{} with live [0,1,2]. Unbounded: k1=3, k2=3, k3=1, k4=0, completed.

**Evidence:**
  - The check at crates/engine/src/lib.rs:1829 is `if pairs_walked.saturating_add(pairs) >= self.pair_budget { halted = Some(self.halt(.., Breach::Pairs)); break 'join; }`. docs/20-sweep-resume.md:108-110 states it outright: "A checkpoint without a named halt requires `pairs < pair_budget`.
  - In crates/engine/src/lib.rs:1827-1836, the check `if pairs_walked.saturating_add(pairs) >= self.pair_budget { halted = Some(self.halt(k, ..., Breach::Pairs)); break 'join; }` runs at the start of every outer row, `for (offset, (_, a)) in block.iter().enumerate()`. Ceiling 4, exactly the need, completes (lib.rs:1624 checks `admitted + emitted >= ceiling` only when a candidate is about to be admitted).

**Expected fix and test:** None

## AC-whp-tb-5 · low bug · engine

**Where:** `crates/engine/src/lib.rs:2043`

**Finding:** For k>=2, `joined_frontier` builds every level with `duplicates: 0` as a literal. Nothing counts a repeat. Three test assertions still present `duplicates == 0` as 'the witness' that a prefix join emitting the same k-set twice would trip, and none of them can fail. The DEFAULT_PAIR_BUDGET doc (769) and the join comment (1787) call it a 'measured zero', which is false: it is assumed. Distinctness is actually caught elsewhere, by the brute-force set comparisons (E-02, the production-shape sweep, sweep_readiness_oracle), so the risk is low. The E-06 row's 'no duplicate candidate is evaluated' still rests partly on dead assertions.

**Evidence:**
  - **The k≥2 counter is a fixed zero.** Every k≥2 level is built by `joined_frontier`: success at `fix/c2-final:crates/engine/src/lib.rs:1999`, allocation failure at 1660.
  - That function writes the literal `duplicates: 0,` (lib.rs:2054).

**Expected fix and test:** None

## ET-masks-evaluation-sweep-2 · low bug · engine

**Where:** `crates/engine/src/resume.rs:135`

**Finding:** validate_for requires support_lanes to match, but lanes are scheduling-only (lib.rs:912-914; engine test a_support_lane_bound_changes_only_scheduling), and the run identity leaves them out. The cli sets lanes = available_parallelism / sharing. So an interrupted `sweep-stored` AND checkpoint is refused on resume on a machine or container with a different core count, even though the answer would be identical. The refusal is loud, not a wrong result.

**Evidence:**
  - `crates/engine/src/resume.rs:161-176`: `validate_for` refuses when `self.ladder.support_lanes != ladder.support_lanes` (line 169). `crates/engine/src/lib.rs:910-914`: the lane count is documented as "a scheduling sentinel, never a depth or answer parameter".
  - The comparison is at crates/engine/src/resume.rs:169 (`self.ladder.support_lanes != ladder.support_lanes` inside `validate_for`, lines 156-176), not :135. `resume_checkpointed` calls it at resume.rs:487. `Params::of` (crates/runner/src/identity.rs:197-209) folds `min_hits`, `ceiling` and `pair_budget`.

**Expected fix and test:** None

## W1-greeks1-0 · low bug · greeks

**Where:** `crates/greeks/src/solver.rs:250`

**Finding:** Contract::implied_volatility (crates/greeks/src/solver.rs:250): A discount factor overflows although every input is inside its bound (rate or carry = −7.1 with years_to_expiry = 100: 7.1·100 > ln(f64::MAX) = 709.78). The solver then returns an arbitrage refusal with intrinsic = inf, where the correct refusal is NotRepresentable. The put is affected under rate overflow and the call under carry overflow. MAX_RATE's doc (bsm.rs:82-86) says the overflow is contained 'on the way OUT' by Contract::greeks, but on the solver path the arbitrage check runs first. pull cannot produce such a rate; a direct consumer of the crate (tickvault) can Code path: bsm.rs:247 `let carry_discount = (-carry * years).exp();` and bsm.rs:256 `discounted_strike: strike * (-rate * years).exp()` are computed without a guard. bsm.rs:411/415 `(self.forward - self.discounted_strike).max(0.0)` / `(self.discounted_strike - self.forward).max(0.0)` = inf. solver.rs:251 `if price <= intrinsic { return Err(GreeksError::PriceBelowIntrinsic { price, intrinsic });

**Evidence:**
  - How it happens: bsm.rs:247 `let carry_discount = (-carry * years).exp();` and bsm.rs:256 `discounted_strike: strike * (-rate * years).exp()` have no guard. no_arbitrage_bounds (bsm.rs:411/415), `(self.discounted_strike - self.forward).max(0.0)`, then yields inf.
  - With S=K=100, T=100: at r=-7.1 the put's IV(10) returns PriceBelowIntrinsic{price:10, intrinsic:inf}, the call's IV returns NotRepresentable, and greeks(put) returns NotRepresentable. bsm.rs:247 `let carry_discount = (-carry * years).exp();` and bsm.rs:256 `discounted_strike: strike * (-rate * years).exp()` are computed unguarded after `signed_bounded(..., MAX_RATE)`.

**Expected fix and test:** None

## W1-greeks1-1 · low bug · greeks

**Where:** `crates/greeks/src/solver.rs:391`

**Finding:** Checked::bisect (crates/greeks/src/solver.rs:391): Low-volatility boundary. The docs claim the 64-halving final bracket (5−1e-6)/2^64 ≈ 2.71e-19 is 'narrower than one unit in the last place of any volatility in that band' (solver.rs:19-20, 385-386; docs/06-limits.md §29). That is false for σ < 2^-9 ≈ 0.00195 Code path: Arithmetic: MIN_VOLATILITY = 1e-6 lies in [2^-20, 2^-19), so ulp = 2^-72 ≈ 2.12e-22. The returned midpoint `0.5 * (low + high)` (solver.rs:402) can sit 1.36e-19 from the root, about 640 ulps. ulp(σ) ≥ 2.71e-19 needs σ ≥ 2^-9. The behaviour is harmless next to MAX_RELATIVE_UNCERTAINTY = 1e-3 (relative error ≤ 1.4e-13 at σ = 1e-6), but the stated property does not hold. This is a false doc claim, not a wrong answer

**Evidence:**
  - solver.rs:19-20 says "`(5 − 1e-6) / 2^64 ≈ 2.7e-19`, narrower than one unit in the last place of any volatility in that band". solver.rs:385-386 says "the final bracket is narrower than one unit in the last place of anything inside it". The same sentence appears in docs/06-limits.md:1956-1957 and docs/05-decisions.md:3603-3604.
  - *What the code does.** In the read-only C2 tree at 9f5c13c7, `crates/greeks/src/solver.rs:391-403` starts at `low = MIN_VOLATILITY` (`1.0e-6`, line 125) and `high = MAX_VOLATILITY` (`5.0`, line 131). `solver.rs:19-20` says "narrower than one unit in the last place of any volatility in that band". `solver.rs:385-386` says "the final bracket is narrower than one unit in the last place of anything inside it".

**Expected fix and test:** None
