STATUS: HELD

# CLOUD-CHG1-OPT: cloud build report

Part: the options remainder of CHG-1 (one dated IPFT authority, the FA73061 boundary pinned at the
stack, the golden option test's record). Spec: `reports/db-specs-20261024/CLOUD-CHG1-OPT.md` on
`origin/cloud/db-base-20261024`. Branch `cloud/db-CLOUD-CHG1-OPT-20261024`, cut from
`origin/feat/db-fix` at `e851da1ca` (the head the spec was grepped at; it had not moved).
Prerequisites CHG-0 (`baa74072d`) and CLOUD-CHG1 (`7fe3b6cc8`) are both merged on `feat/db-fix`.

## Held

`HOLD` appeared on the base branch at 09:16 IST (the shared usage window, lift expected after
12:30 IST). The step in progress (item 2 in `schedule.rs` and its one `regime.rs` edit) was finished
and is committed here; no new step is started until `HOLD` is gone.

## Done so far

- **Item 2 (one dated IPFT authority), code and tests, green on this branch:**
  - `crates/costs/src/regime.rs`: `MAR_2026` is `pub(crate)` (was private), with one doc line saying
    `FA73061_EFFECTIVE` is this constant. No rate, row or table changed. This is smaller than the one
    named function the spec allows as the SEQUENCED EDIT.
  - `crates/costs/src/schedule.rs`: `FA73061_EFFECTIVE` is now `crate::regime::MAR_2026` (one copy of
    the boundary; kept rather than removed because `costs` is a shared crate,
    `docs/10-shared-core.md`), with its stale doc comment corrected. `dated_option_card` reads the
    boundary only to choose the NSE exchange line's source: `REGIME_EXCHANGE_NSE` before
    2026-03-01, a new `REGIME_EXCHANGE_NSE_FA73061` (names FA73061 alone) from it. No figure moves.
  - New tests in `schedule.rs`: `the_dated_option_card_reads_the_ipft_from_the_regime_only`
    (source shape, refuses the `e6d05d708` and `0dc8361c3` card texts kept verbatim as test strings),
    `the_dated_option_cards_ipft_is_the_regimes_on_2023_04_01_2026_02_27_and_2026_03_02`,
    `fa73061_effective_is_the_one_day_the_nse_option_rows_and_the_cards_source_turn_on` (every
    representable day, both venues), `the_dated_option_cards_source_names_its_own_row_on_each_side_of_fa73061`.
  - `cargo test -p costs --locked --lib -- schedule::tests`: `test result: ok. 20 passed; 0 failed`.

## Left (NEXT)

1. Item 3: the stack test in `stack.rs`'s test module, plus a carried-position combination test.
2. Item 1: the stale doc comment in `tests/dhan_calculator.rs`, make the golden test see the split,
   the optional 2026-03-31 / 2026-04-01 STT pin; record the head's green run.
3. Red runs: the new tests at the head without the source change, and at `0dc8361c3`.
4. Ledgers: `docs/04-invariants.md`, `docs/05-decisions.md` (D-6740, D-6741, D-6742),
   `docs/06-limits.md` section CHG-1, `docs/00-charter.md` citation.
5. Gates: fmt, clippy, workspace tests, deny, CI shell gates, llvm-cov, mutants, the `stack` bench.

## Log

- 2026-10-24 09:18 IST: item 2 code and tests in `crates/costs/src/regime.rs` and
  `crates/costs/src/schedule.rs`; `cargo test -p costs --locked --lib -- schedule::tests` ->
  `test result: ok. 20 passed; 0 failed; 0 ignored; 0 measured; 277 filtered out`. HOLD seen at this
  save point. NEXT: wait for HOLD to lift, then item 3 in `crates/costs/src/stack.rs`.

## D-numbers

Block `D-6740..D-6759` (assigned by the spec). Planned: D-6740 (one boundary, one IPFT authority),
D-6741 (date-specific NSE source line), D-6742 (the golden test sees the split). None written yet.

## NEEDS THE MAC

To be completed at DONE; see the spec's section 7 for the merge-back sequence.
