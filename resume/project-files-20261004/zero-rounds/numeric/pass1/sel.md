# Numeric pass 1, slice `sel`

Files: `crates/cli/src/selection.rs`, `selection_v5.rs`, `population.rs`, `population_v5.rs`, `population_v6.rs`, `population_admission_v3.rs` (commit 331b05c)

## Verdict

**NO NEW FINDINGS.** The production code in this slice has no `f64` or `f32` at all. The only floats are in `#[cfg(test)]` code: `population_admission_v3.rs:5225-5233` sits inside `mod tests`, which starts at :4897. Ranking is delegated to `runner::topn`, which uses a total `Ord` on integers. Every counter that could overflow is checked. Every per-candidate scan is bounded by the constant Top-25. The one linear scan that is not bounded already documents itself.

No probe was built. With no float, no lossy cast and no unchecked arithmetic on a live path, I found nothing that needed a measurement to confirm.

## Findings

None.

## Checked and clean

- **Ranking comparator and NaN.** `runner::topn::RankedCandidate::cmp` (runner/src/topn.rs:530-560) compares, in order: `admitted`, the `u64` score, `pessimistic_profit` and `assurance_ppm` (integers), then the mask, the direction, and finally the full strategy digest. It is a total order with a deterministic last tie-breaker. No `partial_cmp().unwrap()` and no NaN can occur. Selection V5 `validate_strongest_first` (selection_v5.rs:1190) and selection.rs `validate_selection_reconciliation` (:2502) both check the full `Ord`.
- **Stored-receipt order checks in V1 and V2** (selection.rs:1154 and :1556) compare only `a.score < b.score`. Tie order among equal scores cannot be checked from `SelectedEntryV1`, which does not carry the tie-break fields. A receipt opened from disk is not recomputed. Only `verify_against_populations` recomputes it, and that compares every byte. Scores are bounded at SCORE_SCALE. Not reported: on an opened receipt the tie order is no less verified than any other field, and gaps.md says the module is unreachable.
- **Fixed-point scores and rates.** `SelectedEntryV1.score` is a `u64` with a domain check (`> SCORE_SCALE` is refused, at :785 and :817). `canonical_rate_ppm` (population.rs:5206) widens to `u128`, guards a zero total, and floors. It mirrors the runner's documented floor-ppm projection; it is a validator, not new rounding.
- **Overflow.** All proof and count reconciliation goes through `checked_add` / `checked_sum`: selection.rs:2429-2470, selection_v5.rs `matrix_marginals` and `increment`, population.rs `BlockFacts`. The `saturating_add` admission counters at population.rs:4207-4218 count rows, and the reconciliation with the receipts would catch any drift. Page offset and byte address use `checked_mul` / `checked_add` (population.rs:3856-3866). The `refused` count in the V5 proof is `checked_sub` (selection_v5.rs:950).
- **V5 terminal matrix.** The index is family*8 + admission*2 + terminal. I checked by hand that the admitted marginal {0,1,8,9}, the authorized marginal {even indices} and the eligible marginal {0,8} match that layout (selection_v5.rs:1709-1766).
- **Eligibility and proof.** `Candidate.admitted = Admitted && Authorized` matches `eligible_count`, so `proof.refused == row_count - eligible_count` holds.
- **Look-ahead and OOS.** No selection decision in these files reads timestamps or bar data. They join and re-rank already-computed population metrics over one `requested_span` identity. That span must be equal across NIFTY and BANKNIFTY (`require_same`). `latest_selection` is by file order and "never infers a current span from timestamps". The OOS replay in `population_v6::selected_stored_oos_witnesses` hands off to `stored_post_training_oos_cohort`, whose overlap refusal is tested in step3_orchestrator.rs:6298+ (other slice).
- **Survivorship.** The families in this slice are fixed to {Nifty, BankNifty} (`InstrumentFamilyV1`, `SelectionV5Family`, `AdmissionV4Family`). No membership comes from today's F&O list, so there is no survivorship path here.
- **Per-candidate cost.** `resolve_selected` and `resolve_selected_v2` (selection.rs:2512, :2562) run an O(25) inner loop for each streamed row: constant-bounded, O(C) total. The V5 `resolve_winners` uses one `HashMap` probe per winner. Duplicate checks use pre-reserved `HashSet`s. The pairwise loops at selection.rs:1142 and :1544 are over at most 25 entries. `population_v6.rs:3257` is a linear `find`, O(25*C), but it documents itself as "Not O(1) per strategy" and a doc test pins that text (W2-cli13-3).
- **Determinism.** The `HashMap` fields in these files serve lookups only. File order is held in a `Vec` (`order`). The `HashMap` iteration-order refusal in population.rs `reconcile_receipts*` is already hunt-conc-3 (known).
- **Search-outcome hierarchy** (population_admission_v3.rs:809-826). It mirrors runner/src/admission.rs:3696-3708. Upstream aggregation in runner/src/validate.rs:3348-3365 uses `checked_add`.
