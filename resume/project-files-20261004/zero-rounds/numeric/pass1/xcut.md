# xcut: numeric pass 1, cross-cutting sweep plus three cli ledgers

Commit 331b05c. Audit only; nothing under the repo was changed.

## Verdict

One new finding, rated medium. The three cli files in scope (`execution_disposition_v2.rs`, `pre_admission_data.rs`, `execution_capability.rs`) are integrity ledgers. Their non-test code contains no float, no price arithmetic and no bar-indexed decision, and I found nothing wrong in them. The workspace-wide sweep turned up one new defect in `/live.json`: its `clears_bar` verdict skips the two refusals `runner::report` applies before saying "clears". One related rounding mismatch on the same comparison was already reported as run1-3 and is not repeated here.

## Findings

### xcut-1 (medium): `/live.json` `clears_bar` ignores the n >= 30 and MISPAIRED refusals the report enforces

- `crates/api/src/livejson.rs:229`
  ```rust
  row.t_milli.saturating_abs() >= bar_milli,
  ```
- `crates/runner/src/report.rs:604-605` and `:631-636`
  ```rust
  let judgeable = s.edge.n >= MIN_OBSERVATIONS;   // 30
  let clears = judgeable && s.edge.t.abs() >= bar;
  ...
  if s.edge.mismatched > 0 { "MISPAIRED — ... this row's mean and t mean nothing" }
  else if !judgeable { "TOO FEW OBSERVATIONS to judge -- a normal bar cannot rule on a t" }
  ```
- **Why it is wrong.** The text report refuses a verdict when n < 30. Its reason: a normal bar applied to a Student-t is "understated by 4.89 at n = 13". It also refuses any row with `mismatched > 0`, whose comment says "it is now impossible to print one". `/live.json` serves `clears_bar` for the same `by_evidence` rows (`cli/src/lib.rs:17866-17873`, `Row::of`), but it checks only the `|t|` against the bar. `frontier::Row` carries `n` and does not carry `mismatched`, and neither is checked. The web page counts `r.clears_bar` as "proved" (`web/src/routes/backtest/+page.svelte:7247`). So, during a run, a 5-observation row or a mispaired row is shown as having cleared the Bonferroni bar, while the end-of-run report calls the same row TOO FEW or MISPAIRED. The ranker orders by |t| with no n floor (`runner/src/rank.rs:120-131`), and `min_hits` comes from the operator and can be 1 (`cli/src/lib.rs:1093`). That makes small-n rows with large |t| exactly the ones that reach the top rows. That last point is INFERRED from the code; no live sweep was run.
- **Repro (probe ran).** Crate `$SCRATCH/probes/xcut`, path dependency on `crates/runner`. It uses the real `bonferroni_t` and `Edge::t_milli`, the `bar_milli` formula copied verbatim from `lib.rs:17894`, and the comparison copied verbatim from `livejson.rs:229`.
  ```
  n=5 t=9.0 bar_milli=4055 t_milli=9000 livejson_clears_bar=true report_judgeable(n>=30)=false
  ```
  Expected: `clears_bar` is false, or absent, for a row the report will not judge. Actual: true.
- **Fix direction.** Carry the report's verdict onto the row (`judgeable && mismatched == 0 && |t| >= bar`, computed in `runner` at full precision). Do not recompute it from two milli integers in `api`. This also removes run1-3's truncate-versus-round window.

### Already reported, re-confirmed

- run1-3 (truncated `bar_milli` against rounded `t_milli`). I independently reproduced it with the real `Edge::t_milli`:
  ```
  trials=1000 bar=4.0556270 bar_milli=4055 t=4.0547270 t_milli=4055 float_clears=false clears_bar=true
  trials=316 bar=3.7777874 bar_milli=3777 t=3.7768874 t_milli=3777 float_clears=false clears_bar=true
  ```
  Nothing new to add.

## Checked and clean

- **`pre_admission_data.rs` (non-test, about lines 1-3915)**
  - No float of any kind.
  - Every count, offset and index sum uses `checked_add`/`checked_mul`, and each failure is a named refusal. The only `saturating_sub`s are `ineligible_count` (bounded by `validate`) and the page `count` (a min).
  - `execution_start_index + execution.count <= minute_context.count` is re-checked on reopen (`:684-695`).
  - The execution subslice is byte-compared against the minute context (`:1470`).
  - `position()` to find the execution start runs once per production, alongside an O(n) digest of the same bars. It is not on a hot path.
  - A page computes `physical = sequence*2` in O(1). Its staleness check is metadata only (`require_unchanged`).
  - Eligibility bytes must be 0 or 1 exactly. The excluded-day list is compared exactly and not sorted.
- **`execution_capability.rs` (non-test, about lines 1-3505)**
  - No float.
  - Percentiles are stored as a reduced `u32` numerator and denominator, re-canonicalised through `RationalPercentileV1::new` on validate.
  - Re-resolution compares the training digest, bar count and both endpoints. No lossy casts. The only `/` is `body / stride`, after a divisibility check.
- **`execution_disposition_v2.rs` (non-test, about lines 1-3402)**
  - No float.
  - The matrix and counters use `checked_add`. A page is bounded at 256 rows, with indexed `HashMap` lookups per row.
  - Preparation walks Population V4 and admission in bounded pages, with one `HashSet::insert` per row.
  - All `wrapping_*` and `saturating_mul` sites are inside `#[cfg(test)]` fixtures.
- **Workspace lints.** `float_arithmetic`, `cast_possible_truncation` and `cast_sign_loss` are denied. Overflow checks are on in release, so unchecked i64 money arithmetic panics rather than wrapping. Every float site carries an expect or allow, and I enumerated them all.
- **`f32`.** Absent from non-test code; the only hits are test strings in `pull/http.rs`.
- **`partial_cmp(..).unwrap()`.** Absent. Ranking uses `total_cmp` with finite-first ordering (`rank.rs:120-131`). BH rejects NaN and values outside [0,1] before `total_cmp` (`significance.rs:360-367`).
- **Float-to-int casts.**
  - `core::price::from_rupees_half_up` (floor plus fraction compare, range-checked, not finite refused).
  - `outcome::milli`, `worst_reward_risk_bp`, `payoff_bp`: NaN and inf guarded, clamped below 2^63.
  - `grid` Wilson bp: clamped. The five Wilson ppm projections (`population_statistics_v2/v3`, `population_admission_v3`, `runner::admission`, `institutional_evidence`) are finite-and-range checked or clamped, then floored. They are comparison-only, and the full-precision bits are retained.
  - `institutional_evidence:2148` bootstrap fraction: the `round` is verified by an exact round trip.
  - `live::expected_rewrites`: display-only count. With `keep == 0` it returns NaN cast to 0, which is harmless.
- **`{:.N}` formatting into a hash, digest or record.** None found.
- **Integer midpoints.** `store/file.rs` uses `low + (high-low)/2`. No `(a+b)/2` on paisa anywhere.
- **`Scored`.** Derives `PartialEq` but has a manual `Ord` and `impl Eq`, which is inconsistent in principle. No `==`, `dedup` or `contains` is used on `Scored`, so there is no observable effect. Dropped.
