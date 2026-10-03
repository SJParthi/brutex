# Pass 2 — slice p2inst

Commit 5140aca (origin/final/all-fixes-zero). Audit only. Nothing under /home/claude/brutex was edited. The probe ran against a scratch copy of the tree; the only change in that copy is a `#[doc(hidden)] pub fn probe_reconcile` wrapper that calls the real `pub(crate) reconcile_trade_rows` without modifying it.

## Verdict

One new confirmed finding (low): p2inst-1. It is the suspected floor-then-compare-to-a-maximum defect in `institutional_evidence::rate_ppm`, now **confirmed through the real `reconcile_trade_rows` and the real `runner::admission::AdmissionPolicyV1::evaluate_research_projection`**. p2bool-1 called this path INFERRED; it is now probed. No other new findings in the readers, codecs, wire files or web number paths.

## Findings

### p2inst-1 — low — crates/cli/src/institutional_evidence.rs:2009-2018 (callers :1719-1736 in `reconcile_trade_rows`; :1437-1438 via `measured_rate` :2001)

```rust
fn rate_ppm(part: u64, total: u64) -> Result<u64, String> {
    ...
    let projected = u128::from(part)
        .checked_mul(u128::from(PPM))
        .and_then(|scaled| scaled.checked_div(u128::from(total)))
```
`rate_ppm` floors. Its results are used for four fields, and the admission policy gates every one of them against a **maximum** (`value > ceiling` fails):
- `largest_trade_profit_share_ppm` (:1722)
- `session_concentration_ppm` (:1731)
- `ambiguous_fill_rate_ppm` and `gap_affected_rate_ppm`, through `measured_rate` (:1437-1438)

An exact rate up to ceiling + 1 ppm − ε therefore floors onto the ceiling and passes.

Two production paths reach this code:
- the institutional builder;
- the Boolean admission path, `boolean_admission_v1::base_values` (:193 → `reconcile_trade_rows`), which copies `largest_trade_profit_share_ppm` and `session_concentration_ppm` straight into the evaluated values.

The same file states the opposite rule in its own doc comment for `BootstrapFraction` (:2079-2089, D-0602/D-0930): *"rounding AWAY from significance … a floor makes a finding look stronger against a maximum"*. This is not recorded in 06-limits or 11-findings. Pass-1 run3-1 and p2bool-1 are the same class in runner/admission.rs and population_base_evidence_v2.rs. This is the institutional_evidence and Boolean copy.

**Probe (ran)**: `$SCRATCH/probes/p2inst/repo/crates/cli/examples/p2inst.rs`. Real `TradeRow`s (one win per IST day) and a matching `Cell` go through the real `reconcile_trade_rows`. Its two outputs then go into the real `evaluate_research_projection`, with every other gate set wide open. Bit 65536 = LargestTradeProfitShare, 32768 = SessionConcentration. Verbatim output:
```
A share-at-boundary: rows=5 best=400001 gross=2000000 -> session_concentration_ppm=Measured(200000) (ceiling 1000000, exact above=false) largest_share_ppm=Measured(200000) (ceiling 200000, exact above=true) failed=ReasonBits(0)
A' share-exact: rows=5 best=400000 gross=2000000 -> session_concentration_ppm=Measured(200000) (ceiling 1000000, exact above=false) largest_share_ppm=Measured(200000) (ceiling 200000, exact above=false) failed=ReasonBits(0)
A'' share+2: rows=5 best=400002 gross=2000000 -> session_concentration_ppm=Measured(200000) (ceiling 1000000, exact above=false) largest_share_ppm=Measured(200001) (ceiling 200000, exact above=true) failed=ReasonBits(65536)
B sess-1/3: rows=3 best=100 gross=300 -> session_concentration_ppm=Measured(333333) (ceiling 333333, exact above=true) largest_share_ppm=Measured(333333) (ceiling 1000000, exact above=false) failed=ReasonBits(0)
B' sess-1/3 ceiling 333332: rows=3 best=100 gross=300 -> session_concentration_ppm=Measured(333333) (ceiling 333332, exact above=true) failed=ReasonBits(32768)
```
(The last line is trimmed only of the unchanged largest_share column.)

- Case A: a best trade of 400001 paisa out of a gross win of 2000000 paisa is exactly 200000.5 ppm, which is above the 200000 ceiling. Expected: failed bit 65536. Actual: failed=0.
- Case B: one of 3 sessions is exactly 333333.33 ppm, which is above a 333333 ceiling. Expected: failed bit 32768. Actual: failed=0.

The share leaks at round ceilings because its denominator is a large paisa sum. The count-denominated rates (session, ambiguous, gap) leak at non-round ceilings, or at round ceilings once there are more than 1e6 trades. The ambiguous and gap fields run through the same function as case B. They were not driven separately: INFERRED from identical code.

Fix direction: use `div_ceil` for the max-gated projections, as `ceiling_ppm_of` already does for bootstrap p-values in this same file. This changes stored evidence and projection bytes, so it needs a decision entry. Fix it together with run3-1 and p2bool-1.

## Known, code matches documented status (not new)

- `boolean_admission_reader.rs:223-246` projects White, SPA, PBO, FWER and RW with `AdmissionExactProbabilityV2::ppm()` (floor) into max-gated fields. This is the same known D-0743 item that p2bool lists for `boolean_admission_v1::apply_statistics`.
- Losing-trade-rate floor (`institutional_evidence.rs:1441`, via `measured_rate`) is run3-1.

## Checked and clean

- institutional_evidence: `bootstrap_fraction` refuses non-finite or out-of-range values, a zero numerator and a denominator above 2^53, and proves the recovered k/(draws+1) bit-for-bit. `BootstrapFraction::ppm` is a ceiling, and the decision uses the exact inclusive integer rule. `ratio_observed` and `return_drawdown` are floored, which is conservative for min gates. Profit ≤ 0 gives 0, and dd = 0 gives u64::MAX, so there is no division by zero. `wilson_lower_ppm` gives the same formula and floor as runner `canonical_wilson_projection_v2` (only the clamp order differs, which is equivalent), and it is reconciled to the cell's bp bucket. `reconcile_trade_rows` uses checked i64 sums and refuses out-of-order, overlapping or non-increasing-session rows. `ist_day` uses `div_euclid`. `checked_weakest_period_return` uses checked adds.
- boolean_qualification_wire / boolean_qualification_reader / boolean_statistics_reader: f64 fields are carried as raw bits, `is_finite` is checked, the class sign is checked against the bits, and probability bits are recomputed from the exact n/d. Rank order uses `total_cmp`, which matches runner bootstrap.rs (:1307, :1489, :1528). Exceedance counts are checked with `checked_add`.
- boolean_candidate_reader / boolean_oos_reader / boolean_campaign_codec / boolean_search_reader: every `as u64` is usize→u64 (lossless on 64-bit). u64→usize goes through `try_from`. Counts are reconciled, and `saturating_*` is used only for capacity sizing, never for values.
- global_replay_v4_codec :169-172 and index_stop_vix_codec :93-94: an `Option` is encoded as a presence tag plus the value, and the decoder refuses tag 0 with a non-zero word (vix :346). None and Some(0) stay distinct.
- web/ (boolean and population pages): every u64/i64 arrives as a decimal string and is validated by regex and BigInt range (`signed`, `uint`, `sint`, `integer`). Paisa is rendered with BigInt `/100n` and `%100n`, with no float and no rounding. f64 statistics are shown as the server's Rust shortest round-trip `to_string` (api booleanevidencejson.rs:460), with the raw bits in the tooltip. ppm values are shown as integers and probabilities as numerator/denominator. No `toFixed`, `Math.round` or `parseFloat` appears on these pages. `catalogDay` formats an IST day index at UTC midnight, which is correct.
- Look-ahead / OOS: nothing in this slice reads later bars. The OOS readers check `exit_bar < bars` and later-window identity, and p2bool covered OOS leakage.
- Complexity: `reconcile_trade_rows` is a single O(rows) forward fold. `checked_weakest_period_return` is O(rows × grains). Both are linear, which is unavoidable because each row must be read.
