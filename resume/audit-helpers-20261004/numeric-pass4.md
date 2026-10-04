# Numeric / look-ahead / O(1) audit, pass 4

**Verdict (head 1f4de71, final/all-fixes-zero, which merges zero/numeric fd45f9d):** the zero/numeric fixes are correctly merged and keep every min-gated twin on its floor. But 24 of the 41 prior items are still NOT FIXED, 1 is PARTIAL, and the diff sweep found 2 new low findings. The worse of the two is a side effect of D-1990.

Audit only. No repository was edited. No cargo was run in this pass: both new findings are low, so the rules allowed reading the source only. The checkout read was /home/claude/wt/zero2 @ 1f4de71. The task started at 6104a4b, and every state below was re-taken at 1f4de71. The diff swept for task B was 5140aca..1f4de71.

## Task A: review of zero/numeric (now merged as 99fba50)

- **The merge matches the branch.** `git diff origin/zero/numeric 1f4de71` over the 12 touched source files is empty.
- **Min-gated twins are untouched.** `win_rate_ppm` still uses the floor `measured_rate`/`rate` in all four builders. Tests check this: population_base_evidence_v2 asserts 666_666, institutional_evidence asserts 333_333, boolean_admission_tests asserts 333_333, and index_stop_qualification_tests compares against `down(...)`. `losing_trade_rate_ppm` stays the canonical floor. The runner's gate reads `losing_rate_ceil` (runner/src/admission.rs:1025, :2889), which is the floor plus one exactly when there is a remainder. The validator has already proved that floor.
- **Probabilities.** All three V1 cli builders take `ceiling_ppm` (boolean_admission_v1.rs:297-328, boolean_admission_reader.rs:224-247, index_stop_qualification_numeric.rs:608). The V2/V3 doors keep the floor and refuse a hidden-floor value (`floor_hidden_ceiling_v2`, admission.rs:4174-4205), and that path is unchanged. The `.ppm()` calls in institutional_evidence.rs:745/749 are on a different type that already rounds up (its test at :3486 expects 333_334).
- **Decision entries.** D-1990 is at docs/05-decisions.md:57320 and D-1991 at :57328. Invariant rows ZN-01..ZN-05 are present.
- **Boundary pins.** 400,001/2,000,000 (200,000.5 ppm) is pinned as stored 200_001 (`max_gated_rates_round_up_and_min_gated_rates_keep_their_floor`). An exact value stays exact: 1/2 gives 500_000 (`the_ceiling_projection_rounds_up_only_a_remainder`). Only the losing rate has an end-to-end policy test (`a_losing_rate_whose_floor_sits_on_the_cap_fails_when_the_exact_rate_is_above`). The four share and rate fields are pinned at the stored value only, so no test evaluates 200_001 against a 200_000 maximum. That is a small test gap, not a defect, because the gate is a strict `>`.
- **D-0742 block refusal.** It is correct as a statistic. It has one bad rendering consequence, recorded as p4num-1.

## Verification table (state at 1f4de71)

| id | state | evidence |
|---|---|---|
| pst-1 | NOT FIXED | cli/src/population_statistics_v2.rs:3687-3696: each candidate still filters the whole `periods` and `splits` vectors, which is O(C²). Not declared in docs/06-limits.md (`build_raw_candidates` does not appear in docs). |
| pst-2 | NOT FIXED | cli/src/population_observations_v1.rs:1367-1395: every split still re-walks every period. |
| pst-3 | NOT FIXED (held; still real) | population_statistics_v2.rs:5033 still uses strict `doubled_rank > doubled_last`, so the exact-median winner is not counted as overfit. Charter :588 leaves tie policy to a versioned decision, and no decision entry names this boundary. The closed-interval reading of Bailey is still UNVERIFIED. |
| pst-4 | NOT FIXED (info) | population_statistics_v2.rs:5049-5058 still has no `wins == 0` short-cut. |
| grk-1 | NOT FIXED | api/src/server.rs:13575-13576 stores the vendor IV unchecked. pull/src/pricing.rs:760-764 takes it as is. There is no percent screen. |
| grk-2 / apis-1 | NOT FIXED | server.rs:13552 calls `Tenor::between(ts, ..)` with the bar's OPEN stamp. pull/src/tenor.rs:252-255 is unchanged. |
| run1-1 | NOT FIXED | runner/src/grid.rs:652-665 `guaranteed_floor`: `losses = trades - wins` still charges flats as worst losses. |
| run1-2 | NOT FIXED | grid.rs:557-563 `avg_loss` still divides by losers plus flats. The defect has now spread to the admission field (p4num-2). |
| run1-3 | FIXED | cli/src/lib.rs:18307-18313 takes the bar as `ceil`, and api/src/livejson.rs:245 compares with strict `>` (CE-7, D-1769). |
| run2-1 | NOT FIXED | grid.rs:5288-5291 `paisa_of` still floors and excursion.rs:1484-1490 `ppm_of` still floors, so a bar printing exactly at the stop does not stop out. |
| run2-2 | NOT FIXED | runner/src/exit_grid_policy.rs:2460-2466: `OperatorRule` has no `wins > 0` guard, and `admits` (:2425-2428) has none either. |
| run3-1 | FIXED (merged D-1990) | admission.rs:1025 and :2889 `losing_rate_ceil`. |
| xcut-1 | FIXED (merged D-1991) | livejson.rs:244-245 requires `n >= MIN_JUDGEABLE_OBSERVATIONS` (30). lib.rs:18271 and :18354 refuse the live view when a row is MISPAIRED. The web reads `clears_bar` only. |
| STO-1 | NOT FIXED | store/src/format.rs:1046-1048: `Overlay::is_sane` still returns `true`. |
| STO-2 | PARTIAL | An unreadable IV cell is now refused (pull/src/rolling.rs:669-672 → `micros_of` :893-904, CE-16). A negative IV is still accepted, and `shift_six` still rounds a negative value away from zero. |
| pul-1 | NOT FIXED | pull/src/fold.rs:238-242 still uses a single 1970 anchor. The :740-741 "all are correct" text is unchanged. |
| core-1 | NOT FIXED | core/src/instrument.rs:393-395 still says the names sort by strike, but the strike is unpadded text. |
| apir-1 | NOT FIXED | api/src/render.rs:2387-2399: `page_peak` takes the maximum over every vendor. |
| apir-2 | NOT FIXED | render.rs:1378-1391: the last reason absorbs the remainder even when its count is 0. |
| apir-3 | NOT FIXED (info, latent) | render.rs:521 still uses `rupees_trunc()` with `paisa_part().abs()`. |
| apis-2 | NOT FIXED (info) | server.rs:13548 still has the 18-space gap. |
| rep-1 | NOT FIXED (info) | cli/src/global_replay.rs:750-765 and global_replay_v2.rs:3493-3508 still scan the offered streams linearly. |
| clib-1 | NOT FIXED (held; still real) | lib.rs:11612 floors ppm, then lib.rs:8569-8571 floors hits, so `min_hits = needed-1`. No decision entry. |
| clib-2 | NOT FIXED (held; still real) | lib.rs:14117-14125 still divides weeks by 100 before multiplying by `n`. No decision entry. |
| ind1-1 | NOT FIXED | indicators/src/vwap.rs:613-616 `band_levels` is still floor(vwap) ± m·floor(σ). |
| ind1-2 | NOT FIXED | trend.rs:241 floored level, vwap.rs (floored VWAP) and session.rs:476-482 all still compare with strict `<` on a floored level. The new SMA seed (trend.rs:215-221) does not change the comparison. |
| p2bool-1 | FIXED (merged D-1990) | population_base_evidence_v2.rs:575-596 `measured_ceiling_rate`/`ceiling_rate_ppm`. |
| p2bool-2 | NOT FIXED (info, declared) | cli/src/boolean_statistics_numeric_v1.rs:190-214. |
| p2inst-1 | FIXED (merged D-1990) | institutional_evidence.rs:1437-1438, :1719-1735, :2006-2027. |
| p2idx-1 | FIXED (merged D-1990) | index_stop_qualification_metrics.rs:178-189. The win rate stays floored. |
| p2run-1 | NOT FIXED | runner/src/report.rs:406-414 prints threshold "-" at n<2 while :560 judges against `bonferroni_t(n)`. |
| p2misc-1 | NOT FIXED | cli/src/pool.rs:38-41 still calls `dd≥` "a lower bound on the pooled figure". The 1f4de71 diff to pool.rs changed only the lanes (D-1556). |
| p2misc-2 | NOT FIXED | cli/src/minute_gaps.rs:206-224 still checks interior steps only, and `MINUTE_GAP_POLICY` is still 1. |
| D-0742 block length | FIXED (merged D-1990) | bootstrap.rs `aligned_for` (:1616) is used by every entry point, and bootstrap_family_pass.rs:437 also refuses. The rendering consequence is p4num-1. |
| D-0743 PBO floor (cli V1) | FIXED (merged D-1990) | `ceiling_ppm` in the three builders. A source-binding test checks this (`no_cli_v1_builder_floors_a_max_gated_probability`). |
| p3floor-1 | FIXED (D-1769), but see p4num-2 | `div_ceil` in boolean_admission_v1.rs:252-256, population_base_evidence_v2.rs:614-617, index_stop_qualification_metrics.rs:215-217, and grid.rs:572-578 `avg_loss_magnitude_ceil`. The denominator still counts flats. |
| p3floor-2 | FIXED (D-1769) | grid.rs:4510 takes `adverse_ppm_ceil_at` (excursion.rs:762-775, `ppm_ceil_of` :1493). Crossing and the rows keep the floored `ppm_of`. The stored candidate cells re-verify loudly (candidate_universe.rs:4721 "measured cell differs"), never silently. |
| hunt-costs-1 | FIXED | D-1535: costs/src/trip.rs prices each leg at its own day's regime (test at :1697). |
| hunt-runner-1 | FIXED | D-1541: grid.rs `ExitChoices::of(.., on_time_exit_bar)` gives `deadline_target` → Time. |
| errpaths-3 | FIXED | D-1545: grid.rs:2074 and `levels_refusal` :2443-2454. One residual: `Ladder::stepped` (excursion.rs:167-175) silently drops a rung whose `i·step` overflows i64. That needs a step above 4.6e18 ppm, so it is not counted as a finding. |
| hunt-pull-1 | FIXED | D-1529: pull/src/gaps.rs uses the venue-dated session (test at :662-666). |
| hunt-store-1 | FIXED | D-1523: the bar table is `Layout::V2` only. |
| gaps-7 | NOT FIXED, undeclared | core/src/universe.rs:914 is the single present-day `FNO_UNDERLYINGS` list. No survivorship or membership-history statement exists anywhere in docs/. |

## New findings

### p4num-1 (low): a refused Romano-Wolf stepdown is printed as "names 0"

D-1990 widened when the stepdown is refused, and the audit report prints that refusal as a measured zero.

**Where**
- cli/src/lib.rs:18530-18537 (`bootstrap_family`)
- runner/src/bootstrap.rs:1198 (`aligned_for`, :1616)
- runner/src/audit.rs:1398-1407

**Code**
```rust
let named = runner::bootstrap::romano_wolf(&family, draws, BOOTSTRAP_SEED,
    runner::bootstrap::DEFAULT_BLOCK, BOOTSTRAP_ALPHA_PPM).len();   // lib.rs
let Some(periods) = aligned_for(returns, block) else { return Vec::new(); };  // bootstrap.rs
"Romano-Wolf (named)", "-", "-", named   ...  "it names {named}."  // audit.rs
```

**Why it is wrong.** `DEFAULT_BLOCK` is 10 (bootstrap.rs:389). Since D-1990, a family with 1 to 9 session periods is refused by every entry point:
- `reality_check` and `spa` return `None`, which audit.rs renders as REFUSED.
- `romano_wolf` returns an empty `Vec` for the same refusal. `bootstrap_family` takes `.len()`, so the report prints `Romano-Wolf (named) 0` and "Only Romano-Wolf says WHICH, and it names 0".

Before 1f4de71 the stepdown actually ran for 2 to 9 periods. Now a refusal is shown as a computed "no strategy is real". That is the failure-as-success shape that CLAUDE.md §4 bans, printed directly under two REFUSED rows.

There is a second, smaller consequence. The short-sample table that `bootstrap` prints (the "37.1% at 3 periods" row, audit.rs:1458-1486, docs/06-limits §77) describes a 3-period p-value that the cli path can no longer produce at the default block.

**Repro.** Not run, read from source. Run `audit-stored` over a span of 5 IST sessions. Expected: the Romano-Wolf row says REFUSED, as the two rows above it do. Actual, inferred from the code path above: `named 0`.

**Fix.** Call `romano_wolf_receipt` (which returns `Option`) or check `aligned_for` first, pass `Option<usize>` to `audit::bootstrap`, and render `None` as REFUSED. Name the block/period refusal in the report.

### p4num-2 (low): the gated `average_loss_paisa` is diluted by flat trades

Rounding the mean up (p3floor-1) left its denominator unchanged, and that denominator counts flat trades as losers.

**Where**
- cli/src/boolean_admission_v1.rs:206-209 and :252-256
- cli/src/population_base_evidence_v2.rs:1103-1105 and :614-617
- cli/src/index_stop_qualification_metrics.rs:155-158 and :215-217
- cli/src/institutional_evidence.rs:1402 and :1409-1413 → runner/src/grid.rs:572-578

**Code**
```rust
let losses = cell.trades.checked_sub(cell.wins)...;          // non-wins, flats included
average_loss_paisa: (losses != 0).then(|| gross_loss.div_ceil(losses))
pub const fn avg_loss_magnitude_ceil(&self) -> u64 { let losers = self.trades.saturating_sub(self.wins); ... .div_ceil(losers) }
```

**Why it is wrong.** The field is gated by `<= max_average_loss_paisa` (runner admission.rs:1065), which the policy documents as the "average losing-trade magnitude". A flat trade (pessimistic P&L exactly 0 paisa) adds nothing to `gross_loss` but does add one to the denominator. Every flat therefore shrinks the stored mean loss, which is the permissive direction for a maximum.

This is run1-2's defect. It now reaches four admission builders, and p3floor-1's ceiling fixed only the half-paisa rounding.

**Repro.** Not run, arithmetic only:
- Inputs: 3 trades, 0 wins, one at −300 and two flat.
- Stored value: `div_ceil(300, 3)` = 100, which passes a 150 cap.
- True average losing trade: 300.

**Fix.** Count losers as trades with a negative P&L. `Cell` has no loser count, so carry one, or derive it from the trade rows, which the reconciling builders already have. Add a decision entry, because stored evidence values move.

## Clean in the 5140aca..1f4de71 sweep

- **runner/src/outcome.rs `WindowExtremes`.** A backward query, or `lo < popped_to`, goes to `BlockExtremes`. That costs a sparse-table lookup plus at most two partial 64-bar scans, so O(1) per query. The table is built once per `forward`, in O(n). The forward path keeps the deque invariant (`next = max(next, lo)`, stale fronts popped).
- **runner/src/outcome.rs `OverlapWindow` timing wheel.** It has min(H, bars+1) slots, and each hit dies within one turn (death ≤ source + H). The tick only advances, so cost is amortised O(1) per bar. D-1572 declares this in docs/06-limits.md:14222-14249.
- **Integer arithmetic.** `ppm_ceil_of` uses i128 with no overflow. The cli/src/pool.rs i128 pooled sums are exact, with `tail_bp` guarded at worst==0 (D-1852).
- **`Verdict::clears` p ≤ 0.05.** For p = k/(B+1), no rational other than exactly 1/20 maps to the same f64 at B+1 ≤ 1e6.
- **indicators trend.rs SMA seed (EMA and ATR).** It reads only bars already folded, so there is no look-ahead.
- **pattern.rs.** Every pattern reads only bar0..bar2 of the past window.
- **api.** `refuse_unread` is a fixed 26-field loop. The logs.rs failed-request ration is O(1).
