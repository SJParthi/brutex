STATUS: HELD

# CLOUD-CHG1-OPT: cloud build report

Part: the options remainder of CHG-1 (one dated IPFT authority, the FA73061 boundary pinned at the
stack, the golden option test's record). Spec: `reports/db-specs-20261024/CLOUD-CHG1-OPT.md` on
`origin/cloud/db-base-20261024`. Branch `cloud/db-CLOUD-CHG1-OPT-20261024`, cut from
`origin/feat/db-fix` at `e851da1ca`; `origin/feat/db-fix` at `1f84b447` (Merge R4-27) merged in
before the final gates. Prerequisites CHG-0 (`baa74072d`) and CLOUD-CHG1 (`7fe3b6cc8`) are merged.

## Held (second time)

`HOLD` reappeared at 12:56 IST (parts without a `GO-<NAME>` file hold; expected lift at the latest
after 17:30 IST). Every code, test and ledger step is committed and pushed; fmt and workspace
clippy are green. The full workspace test run and the base tree's shell gates were already running
and are left to finish (they use no usage budget); their results are read after the lift.

NEXT: when `HOLD` is gone (or `GO-CLOUD-CHG1-OPT` exists), read the workspace test log and the
base gate results; run the language-purity shell gates on this branch and compare with the base;
`cargo deny check`; `cargo llvm-cov -p costs --branch`; `cargo mutants --in-diff` on the costs
diff; write the final report and push `STATUS: DONE` if all green.

## What was built (code at `f1e2d15c`)

Where the spec states a fact the head contradicts, the head is the fact (spec section 4):
item 1 was settled at the head and item 2 mostly done; both are recorded, not redone.

- **Item 2, one dated IPFT authority (D-6740).**
  - `crates/costs/src/regime.rs`: `MAR_2026` is `pub(crate)` (was private), plus one doc line. No
    rate, row or table changed. This is smaller than the one named function the spec's SEQUENCED
    EDIT allows.
  - `crates/costs/src/schedule.rs`: `FA73061_EFFECTIVE` is now `crate::regime::MAR_2026`, one
    definition of the boundary. It is kept, not removed, because `costs` is a shared crate
    (`docs/10-shared-core.md`) and it is `pub`. Its stale doc comment is corrected.
  - New tests: `the_dated_option_card_reads_the_ipft_from_the_regime_only` (a source-shape check
    over `dated_option_card` via `include_str!`, refusing the `e6d05d708` and `0dc8361c3` card
    texts kept verbatim as test strings, plus five mutations of the head's text);
    `the_dated_option_cards_ipft_is_the_regimes_on_2023_04_01_2026_02_27_and_2026_03_02` (on
    2023-04-01 the card refuses with exactly the exchange charge's refusal while the IPFT table
    answers 50; 2023-03-31 added, where both refuse); and
    `fa73061_effective_is_the_one_day_the_nse_option_rows_and_the_cards_source_turn_on` (every
    representable day, 1990-01-01 to 2100-12-31, both venues).
- **Item 3, the boundary at the stack (D-6741).**
  - `dated_option_card` reads the regime's boundary once, only to name the row: before 2026-03-01
    the unchanged `REGIME_EXCHANGE_NSE` (FA64232 and the FA56129 IPFT); from it a new
    `REGIME_EXCHANGE_NSE_FA73061` (FA73061 alone). No figure, status, version or fingerprint moves.
    `REGIME_EXCHANGE_NSE` and its pin at `schedule.rs`'s value test are not edited.
  - New `schedule.rs` test: `the_dated_option_cards_source_names_its_own_row_on_each_side_of_fa73061`.
  - New `stack.rs` tests:
    `a_dated_option_exchange_line_is_fa64232_plus_ipft_on_2026_02_27_and_fa73061_alone_on_2026_03_02`
    (the spec's test), `a_position_carried_across_fa73061_prices_each_day_at_its_own_row`,
    `the_smallest_premium_and_a_crore_price_alike_on_both_sides_of_fa73061`, and
    `fourteen_threads_open_the_dated_option_card_around_fa73061_and_get_one_card`.
- **Item 1, the golden test (D-6742).** The head's file passed unchanged at `e851da1ca`
  (`test result: ok. 12 passed`). CHG-0's verify follow-up is done:
  - the stale doc comment (`dhan_calculator.rs:885-892`) now describes the folded 3,553 row;
  - the test asserts the regime's split, (3,553, 0), and the dated line's FA73061 source on every
    date;
  - 2026-03-31 and 2026-04-01 are added (STT differs, then all lines agree).
- **Item 4, kept refusals: checked, unchanged.** `PHYSICAL_DELIVERY_REFUSAL`, B-06 and S-06 not
  priced, BSE futures and BSE F&O calculator refusals: no line touched, and their tests are in the
  green `costs` run.
- **Ledgers.**
  - `docs/05-decisions.md`: D-6740, D-6741, D-6742.
  - `docs/04-invariants.md`: CHG1OPT-01 to CHG1OPT-07, beside CHG0-09.
  - `docs/06-limits.md`, section CHG-1: a new subsection; nothing old is edited.
  - `docs/00-charter.md`: one paragraph saying where the dated card names the FA73061 row; no rate.

## Tests failing before, passing after (all measured here)

- **The head's behaviour:** this branch with only the two source-choice lines put back as at the
  head. 9 fail, each on a source line or the missing boundary read, none on a figure:
  - schedule: `the_dated_option_card_reads_the_ipft_from_the_regime_only`
    (`left: Err("0 reads of the regime's boundary naming the row, not one: ...")`);
  - schedule: `the_dated_option_cards_ipft_is_the_regimes_on_2023_04_01_2026_02_27_and_2026_03_02`
    (source);
  - schedule: `the_dated_option_cards_source_names_its_own_row_on_each_side_of_fa73061` (source);
  - schedule: `fa73061_effective_is_the_one_day_the_nse_option_rows_and_the_cards_source_turn_on`
    (the source does not move on 2026-03-01);
  - stack: all four new tests (source);
  - `the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases` (source,
    `dhan_calculator.rs:939`).

  Lib `test result: FAILED. 292 passed; 8 failed; 1 ignored`; dhan_calculator
  `FAILED. 11 passed; 1 failed`.
- **At `0dc8361c3`** (CHG-0's merge first parent), with only the shape test and the split pin added
  (the split pin adapted: no `regime::ipft_rate` at that commit):
  - shape test: `left: Err("0 copies of the binding of the regime's IPFT `let ipft_bps = crate::regime::ipft_rate(exchange, day)?.get();`, not one")`;
  - split pin: `assertion failed: FA73061's folded row alone`, `left: (3503, 50)`,
    `right: (3553, 0)` (the spec predicted this; its "3503 + 0" was true only before D-5131).
- **Split mutation ("M6"-like):** the 2026-03-01 regime rows were set to 3,503 and 50, keeping the
  total.
  - The head's golden test: `1 passed` (blind to the split, as CHG-0's verify said).
  - This branch's: `the rows in force on 2026-10-15`, `left: (Ok(3503), Ok(50))`,
    `right: (Ok(3553), Ok(0))`.
- **After, on this branch:**
  - `cargo test -p costs --locked --lib -- schedule::tests stack::tests`: all pass;
  - `--test dhan_calculator`: `ok. 12 passed`.

## Gates

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | exit 0 (after the R4-27 merge and the clippy fix) |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 (`Finished`, 4m05s) at `f1e2d15c`; the first run was red on my own test (`type_complexity`, then `items_after_statements`), fixed in `f1e2d15c` |
| test | `cargo test --workspace --locked --no-fail-fast` | running at the hold; result after the lift |
| deny | `cargo deny check` | after the lift |
| CI shell gates (language-purity job via `checks_all`'s own commands) | base `1f84b447` running; branch after the lift | pending |
| coverage | `cargo llvm-cov -p costs --branch --locked` | after the lift |
| mutants | `cargo mutants --in-diff <costs diff> -p costs` | after the lift |
| bench | `cargo bench -p costs --locked --bench stack` | done, below |

## Measurements (O(1) at p99)

The change adds one date comparison to `RateCard::resolve` under `RateSelector::Dated` for an
option: no loop, no lookup. Method: `cargo bench -p costs --locked --bench stack`, 20,000 samples of
16 calls each; each sample is a batch's mean per call. Run on the cloud container (4 cores) on
24 Oct 2026, head `e851da1ca` and this branch.

- **`open a dated options day`, pinned** (`taskset -c 2`), ten interleaved runs of each:
  - head: p50 49.5-80.3 ns (median 56.0), p99 70.2-115.5 ns (median 96.9), max 3,926-28,617 ns;
  - branch: p50 49.8-65.8 ns (median 49.9), p99 57.6-105.8 ns (median 80.7), max 2,061-9,196 ns.
- **Unpinned**, fifteen runs of each:
  - branch p50 was 92.5-98.1 ns in 4 runs and 49.8-63.0 ns in 11;
  - head p50 was 49.5-64.6 ns in all 15.

  Not reproduced pinned, so recorded as scheduling noise; NOT PROVED beyond the pinned runs.
- **Gated ratios C-CHG-01..10**, three full runs of each tree: 48/48 ok on each (ceiling 3.0x).
  Branch 0.851x-1.659x, head 0.785x-1.833x.

Spec's comparison with the CLOUD-CHG1 figures in `docs/06-limits.md`: those are other machines'
absolute figures (16 Oct cloud container, 19 Oct Mac). Only ratios are comparable, and they are in
the same band.

## Extreme cases (each a test; every one fails before, see above)

1. The boundary day itself, 2026-03-01, a Sunday with no session: priced as the regime prices it,
   naming FA73061 (special session x boundary: a combination).
2. The last session before (2026-02-27) and the Saturday after it (2026-02-28): FA64232 + IPFT.
3. 2023-04-01: the IPFT table answers 50, but the card refuses with the exchange charge's exact
   refusal.
4. 2023-03-31: both tables refuse with different refusals, and the card carries the exchange's
   (two refusals at once: a combination).
5. Every representable day, 1990-01-01 to 2100-12-31, both venues (maximum input): the rows and the
   source move only on their own days.
6. TradeDay::MAX and the leap day 2028-02-29.
7. The month and year boundary 2026-03-31 / 2026-04-01: the STT row moves while the exchange line
   and source hold, in the golden test (month boundary x tax-regime boundary: a combination).
8. A position carried across the date: bought 2026-02-27 and sold 2026-03-02, each day at its own
   row (a combination).
9. Minimum and maximum premium on both sides under both rate sets: one paisa gives ₹0 and one crore
   gives exactly ₹3,553.
10. BSE on both sides of NSE's boundary: unchanged, flagged.
11. Out of the box:
    - the source-shape check run against the repository's own two historical card texts and five
      mutations of the head, comments ignored;
    - fourteen threads opening the card on the ten days around the date at once.

## Files outside ownership

- `reports/cloud-db-CLOUD-CHG1-OPT.md` (this report, required by the README).
- `crates/costs/src/regime.rs`: only the allowed SEQUENCED EDIT (`MAR_2026` made `pub(crate)`, one
  doc line).
- In `schedule.rs`, beside the owned `dated_option_card`, one new constant,
  `REGIME_EXCHANGE_NSE_FA73061`, which only `dated_option_card` reads. `REGIME_EXCHANGE_NSE` and the
  `:1552` pin are untouched.
- The new test helpers sit inside the owned test modules.

## Saved, audited, logged

Pricing is a pure function. This part changes no state, writes no store, emits no event and adds no
route, so there is no state change to audit. Every refusal it touches stays a named `ChargeError`,
as before.

## D-numbers

D-6740, D-6741, D-6742, from the assigned block D-6740..D-6759. They were checked unused on
`origin/feat/db-fix` before use.

## NEEDS THE MAC

Final list after the remaining gates; the spec's section 7 sequence applies.

## Log

- 09:18 IST: item 2 in regime.rs and schedule.rs; schedule tests `ok. 20 passed`; HOLD; pushed
  `ab009dc7`.
- 12:37 IST: HOLD lifted.
- 12:47-13:00 IST:
  - items 3 and 1 pushed as `8bcca30f`;
  - red runs taken (head behaviour, `0dc8361c3`, split mutation);
  - bench runs;
  - ledgers pushed as `47d31962`;
  - merged `origin/feat/db-fix` at `1f84b447` into `c5c0e693`, resolving a `docs/05-decisions.md`
    append conflict as both, in merge order;
  - clippy fix pushed as `f1e2d15c`.
- 13:06 IST: HOLD again (no GO-CLOUD-CHG1-OPT). NEXT: as in "Held" above.
