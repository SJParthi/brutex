# p2stat: runner bootstrap / bootstrap_family_pass / bootstrap_zero_v2 / pbo / significance / rank / split / family_allocation_v1 / search_allocation_v1 @ 5140aca

## Verdict

**No new findings.** Every candidate defect I found was already recorded in the known-items sources (docs/06-limits.md, docs/05-decisions.md, hunt-runner.md, gaps.md), so I dropped it. I ran one probe and quote it below because it confirms two known limits.

## Known items re-confirmed (the code matches their documented status, so not new)

- **Block length vs series length, and restart-ppm quantization** (D-0742; docs/06-limits.md:10926-10936). `stationary_indices` (bootstrap.rs:1663-1688) has two documented gaps. It accepts any block up to 1,000,000 whatever the period count. Its restart probability is `floor(1e6/block)` ppm, so every block from 500,001 to 1,000,000 resamples under the same law. The probe is `$SCRATCH/probes/p2stat`, which depends on `runner` and was run with `--release`. Verbatim output for pure-noise 3-strategy families, 400 periods, 999 draws and 40 trials:
  ```
  periods=400 block=     10: pure-noise families with p<0.05 out of 40: white 2 spa 2 rw 2
  periods=400 block=    100: pure-noise families with p<0.05 out of 40: white 4 spa 4 rw 4
  periods=400 block=    400: pure-noise families with p<0.05 out of 40: white 5 spa 7 rw 6
  periods=400 block=   4000: pure-noise families with p<0.05 out of 40: white 28 spa 28 rw 29
  periods=400 block=  40000: pure-noise families with p<0.05 out of 40: white 36 spa 36 rw 36
  periods=400 block= 400000: pure-noise families with p<0.05 out of 40: white 36 spa 36 rw 36
  periods=400 block=1000000: pure-noise families with p<0.05 out of 40: white 36 spa 36 rw 36
  ```
  An earlier run showed that blocks 500,001 and 1,000,000 (and 600,000 and 999,999) give bit-identical White p-values and RW numerators: `same_law=true`. The cli callers (population_statistics_v2.rs:312/690, v3:498, finalization_v2:307) refuse only `block_length == 0`. This is known, but the measured false-positive rate is worth raising when the fix thread prioritises: at block >= 10x periods, 90% of pure-noise families clear 5%. A `block <= periods` refusal would close it.
- **RW strict `>` vs White/SPA `>=` on ties**: charter row (00-charter.md:590) and D-0465. Ties are reachable with integer returns (a resampled mean of exactly 2x the mean gives an exact tie), but the rule is chosen by the charter.
- **Legacy `pbo::place`/`Placement::overfit`** rounds half-ranks toward the better half: n=3 with rank2=3 is bottom-half under the exact rule but not overfit under the legacy one. cli lib.rs:19815-19835 still uses the legacy path. D-0525-era note at docs/05-decisions.md:30524-30530 and docs/06-limits.md:4575.
- hunt-runner-2/3/5 (5% boundary mismatch, p_value tail, i.i.d. SE in SPA), gaps-3 (BH/pbo/V3 no production caller), and gaps-12 (per-search allocation) are unchanged.

## Checked and clean

- **+1 correction**: White/SPA receipts use `(matched_or_exceeded+1)/(draws+1)`. The legacy versions do the same, with p forced to 1 for draws==0, periods<2, or an all-constant family. RW adjusted uses `(strict+1)/(B+1)` with a cumulative max in stepdown order. The fused pass (bootstrap_family_pass.rs) walks `ranked.rev()`, then reverses back in `receipt`, which gives the same suffix maxima as bootstrap.rs:1305-1330. Legacy RW `admissible = floor((B+1)α)` and bar `= m_(B-a)` are equivalent to `#{>x}+1 <= a`, checked by hand, including α=1 (bar=-inf).
- **`rejects_at_ppm`, `compare` (both allocations)**: exact u128 cross-multiplication with `<=`. Neither floors to ppm before comparing. `minimum_draws = ceil(d/n)-1` is correct. `reduced(0,d)` gives 0/1.
- **search_allocation weight** `(b+1)(b+2)·8` telescopes to a total share of α. family_allocation `families = batches·rungs` is checked and needs no remainder handling, because both use exact fractions.
- **RNG determinism**: SplitMix64 is seeded only by the caller. The family pass draws its indices serially and tallies them in parallel with `checked_add` absorbed in order.
- **Variance with n<2**: `summarise` returns SE 0 and `studentized` maps that to 0. The two-pass variance has no sum-of-squares cancellation. Receipts refuse SE<=0 or non-finite values. zero_v2 refuses a non-zero constant and assigns p=1 to a zero constant.
- **SPA gate**: guarded by `n > 3`, so there is no ln ln of a value <= 1. Observed is floored at 0, so an all-negative family gives p=1.
- **NaN in sorts**: rank `Scored` demotes non-finite |t| below finite values before `total_cmp`. RW order uses `total_cmp` then index. BH refuses values outside [0,1], and -0.0 is admitted and harmless.
- **significance**: every quantile is taken from its tail probability. `upper_tail_quantile(0)` returns 0, but FWER/(2n) >= 1.35e-21 for any u64 n, so that branch is unreachable. Acklam coefficients and signs checked.
- **pbo exact**: `bottom_half` compares `rank2 > (2(n-1))/2`, which is exact because `last` is even. `at_or_above_half` is `bottom >= ceil(folds/2)`. The ppm projections are display-only.
- **split**: anchored and rolling purge h bars, which covers outcome labels over `[i+1, i+h]`. For trade labels, where entry is N+1 and exit is at entry+H, validate's strict `before(test_open)` prefix closes the extra bar (run3). `purged_folds` has no production caller.
- **O(1)**: the bootstrap and rank surfaces are per-run boundaries with documented costs. rank `admit` is O(log keep) per candidate. Per-candidate `edge` is a support walk that is already documented. There are no per-candidate rescans in these files.
