# Numeric / O(1) audit, pass 7: the sweep engine (crates/engine, crates/vocab)

**Verdict at 1f4de71 (origin/final/all-fixes-zero): the Apriori walk is correct, and every primitive in CLAUDE.md §3 rule 4 meets its stated bound. I found 3 new items, all low: two doc-false and one latent over-reservation. Nothing is high or medium.** I audited by reading the source and tracing small vocabularies by hand. No cargo was run, because no finding was high or medium (common2 rules). Known ids p4num-*, p5num-*, p6num-* and pass-1 eng1/eng2 are not re-reported. eng1 already records checkpoints as unsealed by design (attacksweep-5). The cli journal seals them (`and_checkpoint.rs` `replay` checks `saved.seal`).

## 1. Apriori correctness

- **Anti-monotonicity holds.** `ConditionMask::hits` (vocab mask.rs) is `((self & cand) ^ cand) == 0` over all six words. That is exactly `(bits & mask) == mask`, so adding a bit can only remove hits.
- **The join is complete.** `JoinIndex::try_new` (engine lib.rs:2075-2089) keys each (k-1)-set by `without_highest`, sorts by `(prefix, mask)` and dedups. Within one block, members differ only in a top bit h > max(P), and they are ordered by ascending h. Take a frequent k-set S = {p1<…<pk}. The sets S\{pk} and S\{pk-1} are both in F_{k-1} and share the prefix {p1..pk-2}, so the pair (a,b) with a<b gives S. The meaning prune does not break this. `implication::pair_is_informative` is symmetric. A set with no dead pair has no subset with a dead pair. At k=2 the new pair is checked directly. At k≥3 the only new pair is (top(a), top(b)). So every frequent, informative set is still reached.
- **The join is injective.** A candidate's two highest bits are exactly top(a) and top(b), so a candidate recovers its own block and its own pair. `duplicates` is the literal 0 in `joined_frontier` (lib.rs:2168). Before the join, `keyed.dedup()` removes a malformed repeated mask.
- **Hand traces.**
  - (a) Live {1,2,3,4}, all frequent. k=2 is one block (prefix ∅) with 6 pairs. At k=3: block {1} holds 12,13,14 and gives 123, 124, 134. Block {2} holds 23,24 and gives 234. Block {3} holds only 34 and gives nothing. That is 4 = C(4,3) candidates and no repeat.
  - (b) The same, but 23 is infrequent. 123 comes from the pair (12,13). Its one non-parent subset is 23, which the prune probes and refuses. 234 is never generated, because block {2} holds only 24. Correct.
  - (c) The same, but position 4 is always true. It is excluded at k=1 (D-0080, `Why::AlwaysTrue`), so no k≥2 set contains it. This matches the resume check `item.hits < bars` (resume.rs:799).
- **The meaning prune is exact, not a heuristic.** I re-derived it from indicators daily.rs:205-215:
  - Support levels: s1 = 2p−H, s2 = p−R, s3 = s1−R, s4 = p−2R, s5 = s1−2R.
  - Resistance levels: r1 = 2p−L, r2 = p+R, r3 = r1+R, r4 = p+2R, r5 = r1+2R.
  - The gaps are s1−s2 = s3−s4 = p−L ≥ 0, s2−s3 = s4−s5 = H−p ≥ 0, r2−r1 = r4−r3 = H−p and r3−r2 = r5−r4 = p−L. So the chain s5≤…≤s1≤p≤r1≤…≤r5 holds whenever p ∈ [L,H], which `Unusable::CloseOutsideRange` enforces.
  - Every rung uses one `half` (daily.rs:639-640, saturating, which keeps the order). The P rung is `above_cpr_tc`/`below_cpr_bc` at `cpr_span()` = p ± |p−b| = p ± half, which is the same band.
  - So `implies` and `compatible` (implication.rs:158-201) are theorems. No informative frequent set is pruned.
- **The walk stops exactly at extinction.** The loop is `while !current.frequent.is_empty() && progress.halted.is_none()` (lib.rs:1631). The empty level is retired and recorded. `k` is bounded by popcount ≤ 384.
- **§6 has no depth control.** `Ladder` holds {min_hits, ceiling, pair_budget, support_lanes}. Neither engine nor vocab reads `env::var`. `level_slots()` and `BATCH_PER_LANE` set reservation and scheduling only. The ceiling (2^27, cumulative) and the pair budget (2^34) halt **loudly**: each sets `Halt` and marks the level partial. Neither silently truncates (D-0304 and D-1438, already known). `runner::validate::fold_ladder` (validate.rs:1786) carries the parent's ceiling, budget and lanes into each fold.

## 2. ConditionMask width, retired and void bits, thresholds, overflow

- **Width.** WORDS = 6 gives 384 bits, against COUNT = 370 and NEXT_FREE = 370. Two const asserts guard this (lib.rs:418, table.rs:1440).
- **Bits past the table.** `with_bit`, `without_bit` and `get` ignore b ≥ 384. `first_level` refuses `p >= BITS || !is_live` as `Why::NotLive` with `support: None`, so the empty-mask trap is closed. Bits 370-383 set in bar rows never enter a candidate, so they cannot change a hit.
- **Retired and void bits.** Retired 6, 19 and 25, and void 235-273, are `!is_live`, so they become NotLive. `LIVE` and `only_live` give the same answer. On resume, the `allowed` mask (resume.rs:219-234) refuses any survivor bit outside live minus excluded.
- **Threshold.** `hits >= min_hits` is used at both k=1 (lib.rs:1555) and in `drain` (:2389). `min_hits` 0 is raised to 1. The checks run in order: AlwaysFalse (hits==0), then AlwaysTrue (hits==bars), then frequent. So bars==0 never reports AlwaysTrue.
- **Overflow.** `support` is a u64 `saturating_add` of 0/1 per row, bounded by bars ≤ usize. Counters are saturating, and the admitted sum on resume is `checked_add`. `popcount` is u32 ≤ 384. `k.saturating_add(1)` is ≤ 385. Nothing overflows.

## 3. Per-operation cost (§3 rule 4)

| op | code | bound | doc claim | verdict |
|---|---|---|---|---|
| bar lookup | engine `Column.rows: Vec<ConditionMask>`, read in row order; store `Layout::offset_of` (store layout.rs:409) | O(1) per bar (checked mul/add) | 07-o1 layer table; 06-limits §1 | ok |
| condition lookup | vocab `definition` = `TABLE.get(index)` (table.rs:1578); `index_of` uses a compile-time FNV open-address table with 2,048 slots (:1659-1700) | O(1); measured ≤4 hit and ≤7 miss probes | table.rs doc + test | ok |
| mask evaluation | `ConditionMask::hits`, six fixed words, no branch; `Column::support` makes one `hits` per row (column.rs:105) | O(1) per bar; Θ(bars) per candidate, declared | 07-o1:74-75 (C-V-01..03, C-E-02/09) | ok |
| duplicate rejection | k=1: `primitives::offer` → pre-sized `HashSet<u32>::insert` (lib.rs:141); k≥2: none (injective) | expected or amortised O(1); absent at k≥2 | CLAUDE.md §3.4; 06-limits:12200 (D-1440) | ok |
| result append | `primitives::append` → `out.push` into `out`, which was reserved for the batch (`exhausted` → `cannot_grow`, then `drain` `try_reserve(batch.len())`) | O(1), with no allocation on the push path | CLAUDE.md §3.4 (tracked copy, lib.rs:152-158) | ok |
| subset prune | `every_non_parent_subset_is_frequent`: k−2 `MaskSet` probes | Θ(k) per candidate, declared | 06-limits:12179 (D-0924), C-E-12 | ok, declared |
| meaning prune | `parents_informative`: two six-word top-bit scans and one const match | O(1) | lib.rs doc | ok |
| level indexing | `JoinIndex` sort plus `sort_canonically` | O(\|F\| log \|F\|) per level | 06-limits:12190 | ok, declared |
| pair and ceiling checks | integer compares, plus a `try_reserve` that is a capacity test | O(1) per pair | lib.rs doc | ok |
| expression eval (vocab) | `Expression::run`, a fixed tiered stack | O(len) ≤ 1,151 | bounded by MAX_INSTRUCTIONS | ok |

No per-candidate or per-bar step scans undeclared. The `MaskHash` folded multiply has a fixed seed. Its keys are masks derived from data, not chosen by an attacker, and the docs already say "expected O(1)".

## 4. Memory

- `out` is reserved at `min(|F_{k-1}|, ceiling)`.
- The batch is `lanes × 8192`, where lanes ≤ `available_parallelism`.
- Resume decode bounds every count by the bytes remaining and caps the level count at ≤ 385.
- `keep::Best::try_with_capacity` is fallible and has no production caller.
- The one gap is p7num-3.

## New findings

### p7num-1 (low, doc-false): an orphaned "NOT `mut`, AND THAT IS THE PROOF" comment now describes `pruned`
- **Where.** crates/engine/src/lib.rs:1850-1855:
  `// NOT \`mut\`, AND THAT IS THE PROOF. Nothing increments it any more: … a compiler error is a better guard than a counter.` followed by `let mut pruned: u64 = 0;`
- **Why it is wrong.** The comment was written for a local `duplicates` binding. That binding was later removed, and the zero is now a literal in `joined_frontier` (:2168). Read in place, the comment claims that `pruned`, which is `mut` and is incremented at :2003, is not mutable. It also claims a compile-error guard that no longer exists: nothing would fail to compile if a duplicate appeared.
- **Repro.** Not run (text check, `sed -n 1846,1856p crates/engine/src/lib.rs`).
- **Fix.** Delete the five lines, or move a corrected sentence to `joined_frontier` saying "`duplicates` is a literal 0 because the join is injective (D-1440); the tests witness it by distinctness".

### p7num-2 (low, doc-false): the `Ladder::walk` cost section still charges the deleted per-candidate `HashSet` probe
- **Where.** crates/engine/src/lib.rs:1182-1186:
  `Per candidate the work is one \`HashSet\` probe (O(1)), up to \`k\` subset probes (O(k) …), and one support count … The join is O(|F|²) in the previous frontier`
- **Why it is wrong.** The per-candidate `HashSet` probe at k≥2 was the `seen` dedup set, which D-1440 deleted. The subset probes number k−2, not k: the parents are skipped (lib.rs:2426-2431, 06-limits:12179). The join is Σ_block |B|²/2 over prefix blocks, not a flat |F|². This rustdoc section is labelled "Cost, stated rather than implied (§3.6)", so it contradicts 06-limits and CLAUDE.md §3.4 ("no dedup operation on that path").
- **Repro.** Not run (text check).
- **Fix.** Replace it with: "k=1: one expected-O(1) `HashSet<u32>` insert per offered position; k≥2: no dedup probe (injective join), k−2 expected-O(1) subset probes, an O(1) meaning check, and one Θ(bars) support count; the join walks Σ|B|²/2 pairs over prefix blocks B."

### p7num-3 (low, latent): k=1 reserves its survivor vector from the caller's list length, not from the vocabulary bound
- **Where.** crates/engine/src/lib.rs:1501 `let mut first: Vec<Itemset> = reserved(live.len())?;`. The other two reservations from the same count, :1508 `offered.try_reserve(live.len())` and :1494 `excluded_positions`, can legitimately need it.
- **Why it is wrong.**
  - The k=1 survivors are distinct live positions, so they number at most `vocab::table::COUNT` (370). But `first` is pre-sized to `live.len()`, at 56 B per entry against 4 B of input (14×). Repeated positions are legal input: they are counted in `duplicates` and must not cost anything.
  - So a long caller list with repeats makes `try_reserve_exact` either allocate memory the level can never use, or refuse, and `walk_into` then reports `Breach::Memory` at k=0 for a run that needs ≤ 370 slots. That is a refusal that "describes the caller rather than the data", the same reasoning `with_ceiling` uses for raising zero.
  - **No production caller passes more than 370 entries** (`runner::live_positions` = `Evaluator::positions()`; and `validate_for` pins resumed `live`), so no wrong output exists at 1f4de71.
- **Repro.** Not run. Reasoning: `walk(&[row], &vec![62; N])` with N large enough that 56·N exceeds the allocator's limit returns `halted = Some(Halt{k:0, breach: Memory})`, while the same walk with N = 1 completes.
- **Fix.** `reserved(live.len().min(vocab::table::COUNT))?` for `first`. Leave `excluded_positions` and `offered` as they are, or bound them by the distinct count. Add a test with a 10^6-entry list that is all one position.

## Verification of prior items in this scope (state at 1f4de71)

| item | state | evidence |
|---|---|---|
| pass-5 O(1) primitive table (mask eval, k=1 dedup, k≥2 none, append, retention) | FIXED (still holds) | lib.rs:141/149, column.rs:105, `drain` :2359-2410; CLAUDE.md:152-158 matches `drain` |
| D-1438 exact pair budget | FIXED | the check runs before each pair at lib.rs:1947; `Halt.pairs == pair_budget` on a Pairs breach |
| D-1440 seen-set removal / literal duplicates | FIXED (code); doc residue is p7num-1/2 | lib.rs:2168 |
| D-0924 Θ(k) subset prune declared | FIXED | 06-limits:12179, lib.rs:2402-2425 |
| o1engine-20 `Best` O(log cap) declared | FIXED | 06-limits:12197 |
