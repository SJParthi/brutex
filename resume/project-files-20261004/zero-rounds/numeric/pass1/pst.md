# SLICE pst: Population Statistics V2/V3, Finalization V2/V3, Observations V1 (numeric pass 1)

Commit 331b05c (origin/final/all-fixes). Audit only; nothing under /home/claude/brutex was modified.

## Verdict

None of these files has a precision, overflow, NaN or rounding-for-storage defect. All money sums are checked `i64`
paisa. Statistics are stored as exact rational counts plus `f64::to_bits`, and the bits are recomputed and compared
on reopen. I found two new non-O(1)-where-it-could-be items, both on the production Step-3 path. I also found one
statistical boundary deviation in the PBO rank event and one cosmetic float artifact.

Probes: `$SCRATCH/probes/pst/` (verbatim copies of the private functions) and `$SCRATCH/probes/pst/quad/` (same-shape
loops). `SCRATCH=/tmp/claude-0/-home-claude-brutex/e43249a0-a963-5273-8703-91766f1116e2/scratchpad`.

## Findings

### pst-1 (medium): Statistics V2 preparation is quadratic in candidates through a per-candidate full filter
`crates/cli/src/population_statistics_v2.rs:3673-3682` (`build_raw_candidates`, reached from the production path
`step3_orchestrator.rs:2430` -> `prepare_population_statistics_v2_from_observations` -> `PreparedPopulationStatisticsV2::new`)
```rust
let candidate_periods: Vec<_> = periods
    .iter()
    .copied()
    .filter(|period| period.candidate_sequence == sequence_u64)
    .collect();
let candidate_splits: Vec<_> = splits
    .iter()
    .copied()
    .filter(|split| split.candidate_sequence == sequence_u64)
    .collect();
```
`periods` has C*P rows in period-major order and `splits` has C*S rows in split-major order. Both are built in the
same function's callers (`build_raw_periods` and `build_raw_splits`). For each of the C candidates, the code scans all
C*(P+S) rows only to keep 1/C of them, so the total is Theta(C^2 * (P+S)) element visits plus copies of rows of about
140 bytes. The candidate's rows are at fixed strides, `period_sequence*C + candidate` and `split_sequence*C + candidate`,
so a strided walk gives exactly the same ordered digest input in O(P+S) per candidate. The reopen-side
`validate_period_records` and `validate_split_records` already do this with per-candidate hashers in one pass. The
sibling `prepare_single_family_statistics_v3` documents O(C*(P+S)) and is linear. This V2 door has no cost statement,
and `docs/06-limits.md` does not list it.

Repro (probe ran, same shape and record field sets, `--release`):
```
C=500 P=250 S=35: element visits=71250000 kept=142500 elapsed=289.217485ms
C=1000 P=250 S=35: element visits=285000000 kept=285000 elapsed=1.078159251s
C=2000 P=250 S=35: element visits=1140000000 kept=570000 elapsed=4.325021939s
```
Expected (linear): visits = C*(P+S) = 570,000 at C=2000. Actual: 1.14e9, and time grows 4x each time C doubles. At
C=2000 this already matches the order of the bootstrap work (draws*C*P = 5e8 at 1000 draws) that the function exists
to do.

### pst-2 (low): every CSCV split re-walks every period, which costs O(C*S*P) when O(C*(P + S*segments)) is available
`crates/cli/src/population_observations_v1.rs:1366-1393` (`append_candidate_split_scores`, called at :361 and :693/:703)
```rust
for (split_index, (train_mask, test_mask)) in masks.iter().copied().enumerate() {
    ...
    for (period_index, period) in candidate.periods.iter().enumerate() {
        let segment = period_index.checked_div(block_width)...
        if train_mask & bit != 0 { train_score = train_score.checked_add(period.return_paisa)...
```
The segments are contiguous blocks of equal width, so each split score is a sum of at most 16 per-segment sums. At the
layout maximum (16 segments, C(15,8) = 6435 splits) and P = 1600 sessions, the code makes 10.3M period visits per
candidate where about 1600 + 6435*16 = 104,560 would do. The module header disclaims any O(1) claim. This is a
could-be-cheaper item, not a documented-limit contradiction. A fix would need checked segment sums so that the
overflow refusal is kept: an overflow in a partial sum must still refuse, not be skipped.

Repro (probe ran, same loop shape against a per-segment pre-sum, results compared equal):
```
C=20 P=1600 segs=16 splits=6435: per-period walk 390.989345ms, segment pre-sum 4.709262ms, same=true
```
That is about 20 ms per candidate, roughly 33 minutes at 100k candidates, against about 0.25 ms per candidate with the
pre-sum.

### pst-3 (low): the PBO rank event excludes the exact OOS median, so PBO is biased low against Bailey et al.'s closed interval
`crates/cli/src/population_statistics_v2.rs:4973-4981` (`cscv_placement`)
```rust
let doubled_rank = u128::try_from(rank_twice)?.checked_mul(2)?;
let doubled_last = u128::try_from(last_twice)?;
Ok((doubled_rank > doubled_last, true))
```
The test is `2*midrank_from_top > (N-1)`, which is strictly below the median. When the in-sample winner lands exactly
on the OOS median (odd N, or an even-N tied block whose midrank equals (N-1)/2), the split is counted as not
overfit. The cited source (charter line 588, Bailey/Borwein/López de Prado/Zhu, Algorithm 2.3) uses
`omega = rank/(N+1)`, `lambda = logit(omega)`, and estimates PBO as the share of splits with lambda in (-inf, 0],
which includes lambda = 0. **UNVERIFIED**: the closed-interval reading is from memory of the paper and is not quoted
in `docs/00-charter.md`. The charter row says only "ranks below the out-of-sample median" and leaves tie policy as an
implementation decision. The exact-median, no-tie case is not a tie, and no decision entry records it.

Repro (probe ran, verbatim copy of `cscv_placement` against the lambda <= 0 estimator):
```
train=[10, 30, 20] test=[300, 200, 100] cscv_placement=(false, true) bailey_lambda_le_0=true
train=[10, 50, 20, 30, 40] test=[500, 300, 100, 200, 400] cscv_placement=(false, true) bailey_lambda_le_0=true
train=[10, 30] test=[7, 7] cscv_placement=(false, true) bailey_lambda_le_0=true
```
At N=3, a winner ranked 2nd of 3 OOS is not counted as overfit. Under lambda <= 0 it is. With few candidates per
family this moves PBO against the 20% limit in the permissive direction. The fix is either a decision entry naming the
strict boundary or `>=`.

### pst-4 (info): the Wilson lower bound with zero wins is not always exactly 0.0
`crates/cli/src/population_statistics_v2.rs:4996-5005` (`wilson_lower`; `population_statistics_v3.rs:3225` is the same)
```rust
let centre = p + z2 / (2.0 * n);
let margin = Z * (p * (1.0 - p) / n + z2 / (4.0 * n * n)).sqrt();
((centre - margin) / denominator).clamp(0.0, 1.0)
```
With p = 0, centre and margin are equal mathematically, but rounding can leave a positive residue. Probe output:
`wilson_lower(wins=0, trades=1000) = 2.1601063851133708e-19 bits=0x3c0fe0a6b4a64a2e` (trades = 1, 2, 3, 7, 10, 50,
100, 12345 and 1e6 give exactly 0). The result is deterministic, since Rust does not contract to FMA and sqrt and
division are correctly rounded. It floors to 0 ppm at `admission_wilson_ppm_v3`, so no decision changes. It is only a
stored statistic that claims a nonzero lower bound for a 0-win record. Returning 0.0 when wins == 0 would make it
exact.

## Checked and clean
- `PopulationStatisticsFractionV2`: exact numerator/denominator. `validate` refuses a zero denominator and num > den.
  `bits()` matches runner's `ExactFamilyTestPValueV1::value()` (the same u64->f64 division), so the reopen bit-equality
  check is sound.
- No statistic is rounded for storage (§7). White, SPA, Romano-Wolf and PBO are stored as exact counts plus
  `to_bits`. The floor-ppm in `admission_wilson_ppm_v3` (v2:2759, v3:978) is a comparison-only projection after an
  `is_finite` and `[0,1]` check, and is already allowed in 06-limits.
- NaN/inf: `require_finite` covers every stored float bit pattern, and the Wilson range is refused when not finite.
  `wilson_lower_bits(_, 0)` returns 0.0 instead of dividing by zero.
- Empty populations: `validate_raw_shape` refuses an empty family and period_count < 2. `pbo_from_counts` refuses
  contributing == 0. `cscv_placement` refuses empty or misaligned vectors and marks N < 2 unrankable. V3 sets PBO to
  `None` below 2 candidates. `derive_layout` refuses when no even divisor exists, so nothing is dropped or padded.
- Overflow: all period, total and split-score `i64` sums and all trade/win `u64` sums use `checked_add`. `choose_u64`
  uses an exact u128 multiply-then-divide, and the running product is always divisible. Gosper `next_train_mask`
  bounds the sequence. The `cscv_placement` doubled-rank arithmetic is checked and widened to u128.
- Wins/trades: `wins.min(trades)` in `wilson_lower` never hides anything, because every caller validates
  wins <= trades first (v2:856, :939, :3535; observations :417).
- Look-ahead: observations attribute each trade to its exit IST day and refuse cross-day trades. Split scores are
  sums over complete contiguous blocks. Nothing at bar level reads the future here, since these are whole-history
  out-of-sample statistics by design.
- Reopen validation (`validate_period_records`, `validate_split_records`, `verify_bootstrap`) and the V3
  `validate_rank_permutation` are linear, with direct indexing into `candidates`. `seen_ranks.contains(&false)` is
  already declared in 06-limits.
- Finalization V2/V3: byte contracts and digests only. The only scans are zero-padding checks over fixed-size
  records. No floats.
- Known and not re-reported: the half-enumeration C(S-1,S/2) deviation (charter row 588, D-1448) and the truncated
  `Z = 1.959_964` (06-limits §7 allowances).
