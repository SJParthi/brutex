# o1eng2: O(1) audit of vocab, engine, indicators, runner, costs, greeks at HEAD 1087e54

## Verdict

At 1087e54 every CLAUDE.md §3 rule 4 claim holds against the code, read line by line. That includes the
result-append paragraph, which the prior pass found FALSE: the tracked CLAUDE.md (lines 152-162) now
describes the real `drain`, meaning disjoint `counts` slices plus a serial `primitives::append`.
Caveat: the CLAUDE.md copy injected into this agent's context is OLDER than the tracked file. It still
says "lane-local `kept` … one `extend` per chunk", and `git show HEAD:CLAUDE.md` does not. Rule 7
(no look-ahead because of the fold's shape) holds for both `Column::build` paths.

Of the prior pass's non-O(1) or cost NEW items, all but one are fixed or documented:

- FIXED: o1engine-19/22/23/24/40, o1runner-1..11, W3-runner3-4.
- Still present: `set_positions` doc drift (now KNOWN, previously reported as fixed).

The probes measured the following:

- **Mask evaluation.** Flat in candidate width: k=300 vs k=1 time ratio 0.998.
- **Ladder walk.** Linear in bars (16n/n = 13.89).
- **k=1 dedup and append.** Flat per op once memory effects are separated out.
- **Subset prune.** Θ(k) per pair, as 06-limits documents: about 18 ns per probe.
- **`Expression::evaluate`.** Θ(len) per bar and flat in bars, as documented.

New findings: three low and one info, all documentation or bound statements. No high or medium
violation.

- **The main new one (o1eng2-1).** `runner::outcome::WindowExtremes` and `OverlapWindow` assume that
  exits never move backwards ("`forward` never issues one"). On a prefix-median cadence that flips, they
  do: a probe measured 111 backwards exits among 227 priced bars. That breaks the stated amortised-O(1)
  premise. It also breaks a correctness premise of the Newey–West drain, which is UNVERIFIED in effect.

## Part A: CLAUDE.md §3 rules 4 and 7, checked literally against HEAD

| claim | verdict | evidence (HEAD) |
|---|---|---|
| `Column::support` is row-major, with exactly one fixed six-word `hits` per bar, independent of width | TRUE | engine/src/column.rs:106-110 `self.rows.iter().fold(0_u64, \|hits, row\| { hits.saturating_add(u64::from(row.hits(candidate))) })`. vocab/src/mask.rs `hits`: six `(a&c)^c` terms then `(d0\|…\|d5)==0`, no loop. Probe: k=300/k=1 = **0.998** |
| k=1 dedup is one pre-sized `HashSet<u32>::insert` per offered position | TRUE | lib.rs:1506-1507 `let mut offered: HashSet<u32> = HashSet::new(); offered.try_reserve(live.len())?;`, then :1528 `if !primitives::offer(&mut offered, p)`. lib.rs:141-143 `offered.insert(position)` (SipHash RandomState, expected O(1)) |
| k≥2: injective prefix join, no dedup set | TRUE | lib.rs:1783-2066: the join loop holds no set. `joined_frontier` writes `duplicates: 0` as a literal (lib.rs:2156-2172) |
| Result append: one `Vec::push` via `primitives::append` into an `out` reserved for the whole batch; workers write disjoint `counts`; one serial loop (tracked CLAUDE.md:152-162) | TRUE | lib.rs:2370 `out.try_reserve(batch.len())`, :2371-2372 `counts` reserved and resized, :2373-2385 `std::thread::scope` spawns that write `*count = column.support(mask)`, :2388-2394 serial `primitives::append(out, …)`. The per-pair `exhausted` → `cannot_grow(out, batch.len()+1)` (lib.rs:1975-1977, 2210-2212) reserves ahead. **The agent-context copy of CLAUDE.md ("lane-local `kept` … one `extend` per chunk") is stale versus the tracked file.** `grep -n kept CLAUDE.md` → only :157, which states that the old wording was replaced (D-1440) |
| Rule 7: look-ahead is unreachable because `Column::build` streams one bar at a time | TRUE | indicators/src/column.rs:549-583 `for (index, bar) in bars.iter().enumerate() { … evaluator.step_with_warmth(bar) …}`. The anchored path (column.rs:422-424) uses the same `build_from`. `AnchoredEvaluator::advance_before` (anchored.rs:484-509) installs only references with `reference.day < signal_day`. The exact-minute overlay (anchored.rs:823-863) now targets only `session_close(signal_day)`, never the slice (GAP12-7 fixed, D-0943) |

## Part B: hot-path table (HEAD line numbers; prior row ids in brackets)

Verdict key: O(1) means constant; bounded means capped by a compile-time constant but not flat;
expected-amortised means a hash or Vec guarantee.

| path | file:line | unit | cost | verdict | if not O(1): why / how / where documented | status vs prior |
|---|---|---|---|---|---|---|
| `ConditionMask::hits` [e-1] | vocab/src/mask.rs (`pub const fn hits`) | candidate×bar | 6 AND, 6 XOR, 5 OR, 1 cmp | O(1) | — | re-verified |
| `Column::support` [e-3] | engine/src/column.rs:106 | bar | one `hits`; Θ(bars) per candidate | O(1)/bar | Per candidate it is the measurement itself, 06-limits §1. Probe: per-bar cost rises 3.2× from 200k to 3.2M bars because of cache/DRAM (9.6 MB → 154 MB column), not algorithm | re-verified, measured |
| `support_fingerprinted` [e-4] | column.rs:147-176 | bar | one `hits` + 1 mix per 64 bars | O(1)/bar | No production caller (doc, D-0760) | re-verified |
| `set_positions` [e-5] | column.rs:39-57 | mask | Θ(popcount) ≤ 384 | bounded | Doc column.rs:30-31 still says "`every_subset_is_frequent` … pays 384 probes"; that function is now `every_non_parent_subset_is_frequent` and walks set bits | KNOWN: ET-masks-evaluation-sweep-3 (marked FIXED in out_v2; this sentence is not) |
| k=1 per offered position [e-6] | lib.rs:1508-1563 | position | 1 SipHash insert + `table::is_live` (index) + 1 support | expected O(1) + Θ(bars) | Probe: offer 13.9 → 17.8 ns/insert at n → 16n | re-verified, measured |
| `table::definition/is_live/name` [e-7] | vocab/src/table.rs:1578-1580, 1729-1745 | call | `TABLE.get(index)` | O(1) | — | re-verified |
| join per pair: budget, ceiling, union [e-8, e-13] | lib.rs:1918-2032 | pair | int compares + `try_reserve` capacity test + 6-word OR | expected-amortised O(1) | Budget is now checked per pair (D-1438) | re-verified; e-13 FIXED |
| subset prune `every_non_parent_subset_is_frequent` [e-9] | lib.rs:2425-2430 | candidate | k-2 MaskSet probes | NOT O(1): Θ(k), k ≤ 384 | Inherent to an Apriori full-subset check. DOCUMENTED 06-limits:12100-12109 (D-0924). Probe: 69 / 735 / 3,572 ns per pair at k = 4 / 42 / 202 (≈18 ns per probe) | DOCUMENTED |
| `parents_informative` [e-10] | lib.rs:2144-2153 | pair | 2 reverse six-word scans + const match | O(1) | — | re-verified |
| `JoinIndex::try_new` sort, `sort_canonically` [e-11, e-12] | lib.rs:2084, 2437-2439, 1565, 2057 | level | O(\|F\| log \|F\|) | NOT O(1) per level | Needed for determinism and prefix grouping. DOCUMENTED 06-limits:12111-12117 | DOCUMENTED (was QUEUED) |
| `MaskHasher` [e-14] | lib.rs:242, 316, 374 | probe | 7 u128 multiply-folds | O(1) expected | Unkeyed fixed seed | re-verified |
| drain [e-15, e-16] | lib.rs:2358-2397 | batch / candidate | reserve once; `lane_count` spawns + `counts` alloc per batch; serial push | amortised O(1) per candidate | Probe: append 3.7 ns/push at 100k; 31.9 ns/push at 1.6M (90 MB, first-touch page faults on reserved pages). DOCUMENTED BATCH_PER_LANE doc lib.rs:2269-2300 | re-verified, measured |
| checkpoint write [e-19] | resume.rs:356-364, 377-431; cli and_checkpoint.rs:302, 425 | level boundary | cli writes `write_prefix_to` + `write_current_to` only | O(\|F_k\|) per level | Full `write_to` is kept for the one-shot form | FIXED-since-prior |
| `keep::Best::offer` [e-20] | keep.rs:321, 395 | offered itemset | O(1) refuse, O(log cap) admit | NOT O(1) (heap) | DOCUMENTED keep.rs:310-320, 06-limits:12118-12122. No production caller | FIXED (doc) |
| `Expression::evaluate` [e-22] | vocab/src/expression.rs:243-303, scratch_slots :36 | bar | Θ(len), ≤ 1151; scratch 8/64/576 | NOT O(1) in program, O(1) in bars | Every instruction can change the answer, so Θ(len) is inherent. DOCUMENTED 06-limits:11347-11358. Probe: 8.7 / 39.5 / 322 / 2,173 / 2,959 ns/bar for 1 / 15 / 127 / 799 / 1151 instructions; 16n/n per bar 1.009-1.015 for ≥127 instr | FIXED (stack sized) + DOCUMENTED |
| `Cursor::advance` / `place` [e-23] | vocab/src/expression_search.rs:174-232, 238-258 | node | O(1) except a sibling slice compare ≤ length; 1151-instr copy per emitted candidate | bounded | DOCUMENTED 06-limits:11331-11345, 11101-11125 | FIXED-since-prior |
| `table::index_of` [e-24] | table.rs:1709-1722 | token (parse-time) | FNV-1a + ≤ 7 probes | bounded | Probe: 16.5 ns/lookup. DOCUMENTED 06-limits:11320-11329 | FIXED-since-prior |
| indicators `Column::build_from` [e-28] | indicators/src/column.rs:549-583 | bar | one `step_with_warmth` + pushes into reserved Vecs | O(1) amortised | Probe: 4,977 → 3,518 ns/bar at 3,750 → 60,000 bars (ratio 0.707, warm-up amortised). Each step copies an `Evaluator` (size_of = 1,728 B) and, on the anchored path, `*self` (≤ 2,048 B, anchored.rs:295, 426): fixed, but a ~2 KB memcpy per bar | re-verified, measured |
| `AnchoredEvaluator::step_with_warmth` / `advance_before` [e-38] | anchored.rs:389-430, 484-509 | signal bar | refusal decided first; cursor walk only on accepted bars | amortised O(1) | D-0942 | FIXED-since-prior |
| exact-minute overlay [e-39] | anchored.rs:780-893 | minute + signal | monotone cursor, no HashMap; O(minutes + signals) | amortised O(1) | Doc anchored.rs:707 "UNVERIFIED performance" | re-verified |
| `replace_exact_positions` [e-40] | column.rs:743-765, `overlay_exact` :1101-1119 | row | family mask built once; 6-word ops per row | O(1) | — | FIXED-since-prior |
| `Calendar::is_non_regular` | indicators/src/evaluator.rs:294-296 | session rollover | `days.iter().take(len).any(..)`, len ≤ 9 | bounded | Fixed array of 9 | NEW row, fine |
| GapFib step | indicators/src/gap.rs:283-318 | bar | fixed fields, 11-rung `bits` | O(1) | — | changed since prior, verified |
| `isqrt_i128` in `Vwap::sigma` [e-36] | vwap.rs:179-218, 358-394 | bar (equity runs) | Newton seeded at v: 1..130 iterations of i128 division | bounded, not flat | DOCUMENTED §51 (vwap.rs doc now says equities execute it). Seeding at `1 << (bits/2+1)` would converge in ~7 steps. Probe: 1 / 13 / 35 / 55 / 69 iterations and 12 / 134 / 374 / 948 / 1,352 ns at v = 1 / 1e6 / 1e18 / 1e30 / i128::MAX | DOCUMENTED; seed unchanged (KNOWN W3-indicators2-0/1) |
| costs `DatedTable::value_on` [e-42] | costs/src/dated.rs:136-170 | trade | walk fixed `later` (≤ 5) | bounded | — | re-verified |
| costs `next_weekly_on` (changed) | costs/src/expiry.rs:435-458 | trade | ≤ MAX_LATER_ROWS+1 passes × `find` over ≤ 5 rows | bounded | `from` only moves to a strictly later row start, so it terminates. Doc module :8-14 | NEW row, fine |
| costs `swept_slot`, `charge_stack`, `lot_size_on` [e-43..46] | venue.rs:148; trip.rs:1031; lot.rs:233 | trade | fixed arrays / arithmetic | O(1) | — | re-verified |
| greeks `implied_volatility` [e-48] | greeks/src/solver.rs:156-176, 260, 414-436 | solve | ≤ 2+8+75+1 = 86 evaluations; bisection always exactly 75 | bounded | DOCUMENTED 06-limits:1970-1990 (was 64 → 75, D-0921) | changed, DOCUMENTED |
| runner `walk_core` [r-1, r-2] | runner/src/trade.rs:894-919ff | row | one `fires` per row from `first_row` (D-1186) | O(1)/row; Θ(rows) per candidate | DOCUMENTED 06-limits:6607-6612 (D-1204) | o1runner-10 FIXED |
| `SessionBounds` day map [r-5] | outcome.rs:179-206 | slice | `HashMap::with_capacity(candidates)` | expected O(1) | D-1177 | o1runner-8 FIXED |
| `prefix_median_steps_over` (new, D-1410) | outcome.rs:979-1034 | bar | two-heap running median, O(log g) | NOT O(1): O(log g)/bar | Needs an unbounded step alphabet. DOCUMENTED 06-limits:11176-11188 | DOCUMENTED |
| `WindowExtremes::over` [r-17] | outcome.rs:623-679 | forward query | amortised O(1) **if** both ends are monotone; Θ(window) on a backwards query | amortised O(1) only under that premise | See o1eng2-1: the premise fails on a flipping cadence | NEW (doc premise) |
| `OverlapWindow::observe` (NW) [r-19] | outcome.rs:1774-1855 | measured hit | amortised O(1) | expected-amortised O(1) | Same monotone-exit premise (:1810-1813) | W3-runner3-0 FIXED; premise is o1eng2-1 |
| `Ladder::from_excursions` [r-13] | excursion.rs:213-249 | candidate | ≤ 65 nested `select_nth_unstable`, O(n) | linear per ladder, O(1) per trade | doc :200-212 | FIXED-since-prior |
| `money_envelope_fits` (new, D-1147) | grid.rs:2382-2403 | candidate | Θ(Σ span over paths) | NOT O(1) per candidate | Same order as `crossings_checked`, doc :2371-2373 | NEW row, documented in code |
| `require_exact_execution_subslice` [r-28] | exit_grid_policy.rs:521-554 | candidate side | pointer offset fast path, O(1); linear fallback only for a copied slice | O(1) on shipped callers | D-1196 | o1runner-3 FIXED |
| attest once per resolution [r-30] | cli boolean_candidate_v1.rs:907-935 | program×side | cached `AttestedTrainingV1` | O(1) after first | W2-cli2-3 | o1runner-1 FIXED |
| OOS later slice hoisted [r-31] | cli boolean_oos_v1.rs:431-434 | program×side | `LaterExpressionSliceV1::new` and `LaterFoldMapV1` once | O(1) per program for invariant work | D-1188 | o1runner-2 FIXED |
| validate V4 pending replay [r-33] | validate.rs:2776-2800 | pending candidate | digests and `oos_facts` hoisted (D-1143, D-1184) | O(1) invariant work per candidate | — | FIXED-since-prior |
| validate OOS / training facts [r-39, r-40] | validate.rs:4963, 4682-4701 | fold | one `SliceFacts`, `forward_over` | — | Pinned by tests :7586-7757 | o1runner-4/5 FIXED |
| legacy Romano–Wolf [r-48] | bootstrap.rs:1454-1530 | family | one resample matrix, one suffix pass + select per rank | Θ(B·S·N + S·B) | D-0973 | o1runner-7 FIXED |
| ratio bitmap [r-11] | grid.rs:2133-2152 | candidate | removed (`let _ = ratios;`) | — | D-1140 | o1runner-6 FIXED (code gone) |
| audit report top-keep [r-49] | audit.rs:987-990 | report | `select_nth_unstable_by_key` + sort of head | Θ(cells + keep log keep) | — | o1runner-11 FIXED |
| portfolio per-minute arbitration [r-44] | portfolio.rs:42 | minute | ≤ 200 intents, bounded | bounded | 06-limits §128 (:7122) still says "at most 25 intents"; code `MAX_INTENTS_PER_MINUTE = SUPPORTED_RUNGS_MINUTES.len() * MAX_PRIORITY_PER_RUNG` (= 8×25) | NEW (o1eng2-3) |
| resample fold (changed) | runner/src/resample.rs:278-330 | bar | one accumulator, fixed ops | O(1) | doc :271-274 | re-verified |

Not re-verified row by row at HEAD: prior o1runner table rows 6, 7, 9, 10, 12, 14-16, 18, 20-27, 29, 32,
34, 35, 37, 41-43, 45-47, 50. Their files changed (grid.rs +1,219, exit_grid_policy.rs +929). A
pattern scan of the non-test added lines in those files for `position/find/contains/sort/clone/format!/
BTreeMap/collect/while/for` found no new per-candidate scan beyond the rows above. That scan is a
heuristic, not a proof.

## Part C: findings

| id | sev | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| o1eng2-1 | low | runner | runner/src/outcome.rs:599-621 (`WindowExtremes` doc: "`forward` never issues one"); :1810-1813 (`OverlapWindow::observe`: "exits only advance with the entry … so … the drain is complete") | Both structures assume each priced bar's exit is never earlier than the previous priced bar's. Since D-1410, though, `forward_over` (outcome.rs:805) sets `deadline = start + facts.step_at(i) * H`, and `step_at` is a prefix running median that can go DOWN. On a slice whose median flips, the exit index moves backwards. Cost: each backwards query clears and rebuilds the deques, Θ(window) instead of amortised O(1). The doc claims this never happens. Correctness (UNVERIFIED): the Newey–West drain pops only from the front, so a queued hit whose exit is earlier than the front's stays queued, and its pairs could be counted as overlapping. The effect on `edge`'s t-statistic was not measured | Probe (`runner/tests/zz_audit_o1eng2_3.rs`, deleted). 20 synthetic sessions with every third minute dropped (`(i % 375) % 3 != 2`), through the public `SliceFacts::step_at` and `Forward::exit_at`. Output verbatim: `drop 2: bars 5000 accepted 5000 exits_some 4739 step flips 248 first steps(s) [60, 60, 120, 60, 120, 60, 120, 60, 120, 60, 120, 60] H=9: priced 227, exit went BACKWARDS 111 times` and `… H=36: priced 200, exit went BACKWARDS 93 times`. Timing showed no measurable cost at this size (`alternating … H=9: 279.3 ns/bar … H=144: 260.7 ns/bar, ratio 0.93`), because few bars are priced and the flips stop after day 1. Fix: state the premise as "monotone while the cadence is constant" and name the rebuild cost in 06-limits, or make the drain independent of exit order (e.g., drain by `old_exit <= source` over an exit-ordered structure) | NEW (known W3-runner3-4 fixed the sparse table; nothing names non-monotone exits) |
| o1eng2-2 | low | engine | engine/src/column.rs:30-31 | The `set_positions` doc still says "`every_subset_is_frequent` says as much and pays 384 probes for the lack of one". That function was renamed to `every_non_parent_subset_is_frequent` and walks set bits (lib.rs:2425-2430). The doc misstates the cost | `/// \`ConditionMask\` exposes no bit iterator -- \`every_subset_is_frequent\` says as much` / `/// and pays 384 probes for the lack of one.` `grep -n every_subset_is_frequent crates/engine/src/*.rs` → only column.rs:30 | KNOWN: ET-masks-evaluation-sweep-3 (known/fix-queue_lane2.md:31), reported FIXED in known/resume_audit-20261003_out_v2.md:13. This sentence was missed |
| o1eng2-3 | low | docs | docs/06-limits.md:7122 (§128) | The doc says a minute owns at most 25 intents and a 25-slot result. The code allows 8 rungs × 25 = 200, so the "square of that compiled ceiling" bound is 40,000 compares, not 625 | 06-limits:7122 "A minute owns at most 25 intents and a fixed 25-slot result". runner/src/portfolio.rs:42 `pub const MAX_INTENTS_PER_MINUTE: usize = SUPPORTED_RUNGS_MINUTES.len() * MAX_PRIORITY_PER_RUNG;` | NEW (noted in the prior o1runner row 44 but never filed; not in known/) |
| o1eng2-4 | info | engine | engine/src/lib.rs:1692-1695, 742-743 | Two doc passages describe the deleted `seen` set in the present tense: `exhausted`'s doc "`seen` is built fresh inside [`Self::next_level`]", and `DEFAULT_CEILING`'s "A level holds each distinct candidate twice: once in `seen`". The second is partly corrected further down (:782-785) but its first sentence still reads as current | Quoted text at those lines | KNOWN: ET-masks-evaluation-sweep-3 ("Engine comments still describe the removed `seen` set"), marked FIXED in out_v2 but these remain |

## Measurements (probe output verbatim)

These were run in a shared 4-core container while other workers' cargo jobs ran. Absolute ns are noisy.
The `test` profile is `[optimized + debuginfo]`. Every probe file was deleted, and
`git status --porcelain | grep -c o1eng2` returned `0`.

engine (`crates/engine/tests/zz_audit_o1eng2_1.rs`):
```
Column::support k=1: n=200000 2.074 ns/bar, 16n=3200000 6.547 ns/bar, ratio(16n/n per bar) 3.157
Column::support k=300: n=200000 2.036 ns/bar, 16n=3200000 6.549 ns/bar, ratio(16n/n per bar) 3.216
Column::support at 16n: k=300 / k=1 total time ratio 0.998
(first run, n=20000: k=1 1.492 vs 6.300 ns/bar ratio 4.224; k=300 ratio 4.132; k=300/k=1 1.271)
primitives::offer n=10000: 13.921 ns/insert
primitives::offer n=160000: 17.770 ns/insert
primitives::append (reserved) n=100000: 3.725 ns/push
primitives::append (reserved) n=1600000: 31.866 ns/push
JoinProbe k=4: main block 1 pairs (1 survive), 69.0 ns/pair
JoinProbe k=42: main block 1 pairs (1 survive), 735.3 ns/pair
JoinProbe k=202: main block 1 pairs (1 survive), 3572.2 ns/pair
walk n=5000: depth 3 frequent 2312 in 168.1 ms; 16n: depth 3 frequent 2312 in 2335.2 ms; time ratio 13.89 (16.0 = linear in bars)
```
Reading: support is width-flat. Its per-bar rise with n tracks the column leaving cache (48 B/bar), not
extra operations. The walk is linear in bars. The prune is Θ(k), matching 06-limits. The append rise at
1.6M is first-touch paging of a 90 MB reservation.

vocab (`crates/vocab/tests/zz_audit_o1eng2_2.rs`):
```
Expression::evaluate leaves=1 (instructions 1): 8.7 ns/bar at n=20000, 17.4 ns/bar at 16n, ratio 2.009
Expression::evaluate leaves=8 (instructions 15): 39.5 ns/bar at n=20000, 44.4 ns/bar at 16n, ratio 1.124
Expression::evaluate leaves=64 (instructions 127): 322.2 ns/bar at n=20000, 324.9 ns/bar at 16n, ratio 1.009
Expression::evaluate leaves=400 (instructions 799): 2173.2 ns/bar at n=20000, 2065.5 ns/bar at 16n, ratio 0.950
Expression::evaluate leaves=576 (instructions 1151): 2959.1 ns/bar at n=20000, 3003.0 ns/bar at 16n, ratio 1.015
table::index_of: 16.5 ns/lookup over 370 names
```

runner / indicators (`crates/runner/tests/zz_audit_o1eng2_3.rs`):
```
indicators Column::build: n=3750 4976.6 ns/bar, 16n=60000 3517.6 ns/bar, ratio 0.707
size_of Evaluator = 1728 bytes
forward uniform 1-min: bars 15000 H=9: 206.6 ns/bar, priced 14160, ratio to H=9 1.00
forward uniform 1-min: bars 15000 H=36: 203.4 ns/bar, priced 14160, ratio to H=9 0.98
forward uniform 1-min: bars 15000 H=144: 175.8 ns/bar, priced 14160, ratio to H=9 0.85
forward alternating 1/2-min: bars 10000 H=9: 279.3 ns/bar, priced 227, ratio to H=9 1.00
forward alternating 1/2-min: bars 10000 H=36: 276.4 ns/bar, priced 200, ratio to H=9 0.99
forward alternating 1/2-min: bars 10000 H=144: 260.7 ns/bar, priced 92, ratio to H=9 0.93
forward H=15: n=3750 113.2 ns/bar, 16n=60000 437.2 ns/bar, per-bar ratio 3.862
isqrt_i128(1e0) = 1: 1 iterations, 12.2 ns
isqrt_i128(1e6) = 1000: 13 iterations, 133.6 ns
isqrt_i128(1e18) = 1000000000: 35 iterations, 374.4 ns
isqrt_i128(1e30) = 1000000000000000: 55 iterations, 948.4 ns
isqrt_i128(1.7014118346046923e38) = 13043817825332782212: 69 iterations, 1352.2 ns
drop 1: bars 5000 accepted 5000 exits_some 4720 step flips 0 first steps(s) [120, 120, ...] H=9: priced 0, exit went BACKWARDS 0 times
drop 2: bars 5000 accepted 5000 exits_some 4739 step flips 248 first steps(s) [60, 60, 120, 60, 120, 60, 120, 60, 120, 60, 120, 60] H=9: priced 227, exit went BACKWARDS 111 times
drop 2: bars 5000 accepted 5000 exits_some 4739 step flips 248 first steps(s) [60, 60, 120, 60, 120, 60, 120, 60, 120, 60, 120, 60] H=36: priced 200, exit went BACKWARDS 93 times
```
Reading: `forward` is flat in H on uniform cadence, which supports the sliding window.

**UNEXPLAINED, UNVERIFIED cause:** `forward` per-bar cost rose 3.86× from 3,750 to 60,000 bars on
uniform cadence. Two candidates exist and neither was isolated: `SliceFacts::of`'s `HashMap`, which
grows out of cache, and the prefix-median heaps, which are O(log g) per bar and documented. The
prefix-median cost is documented, so this is not filed as a finding. A follow-up should time
`SliceFacts::of` and `forward_over` separately at n and 16n.
