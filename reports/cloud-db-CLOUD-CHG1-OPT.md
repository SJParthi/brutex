STATUS: BLOCKED

# CLOUD-CHG1-OPT: cloud build report

Part: the options remainder of CHG-1 (one dated IPFT authority, the FA73061 boundary pinned at the
stack, the golden option test's record). Spec: `reports/db-specs-20261024/CLOUD-CHG1-OPT.md` on
`origin/cloud/db-base-20261024`. Branch `cloud/db-CLOUD-CHG1-OPT-20261024`, cut from
`origin/feat/db-fix` at `e851da1ca`. The newest `origin/feat/db-fix` was merged in five times before
the final gates: `1f84b447` (Merge R4-27), `7b397bbe` (Merge CLOUD-MUTCLI), `666ddf7f` (Merge
DATA-1w), `bf9b1c8f` (Merge R4-18, as `a3e04fc2`), then `cc0a3a0d` (Merge R4-28, as `b60b0f46`, the
`docs/05-decisions.md` append conflict resolved with `feat/db-fix`'s file as the exact prefix),
still its tip at the end. Prerequisites CHG-0 (`baa74072d`) and CLOUD-CHG1 (`7fe3b6cc8`) are merged.

## Status: code complete; every remaining red is the base's or this container's

BLOCKED only on items that are not this part's, each measured on the unmodified base `cc0a3a0d`
(and before it on `bf9b1c8f`):

1. **`cargo deny check`: RUSTSEC-2026-0386** (tokio 1.53.1). Base item; BASE-DENY-1024 bumps tokio to
   1.53.3 on the Mac. `Cargo.lock`, `deny.toml` and every manifest are untouched here, and identical
   on `feat/db-fix` from `666ddf7f` (where deny was last run, red on this advisory alone) to
   `cc0a3a0d`.
2. **`cargo test --workspace`: 29 tests red, the same 29 on the base** (run by exact name on
   `cc0a3a0d`: `api` 8, `cli` 3, `pull` 9, `store` 5, `telemetry` 4, all `FAILED` there too). They
   are read-only-directory and refused-write tests (this container runs as root, which ignores the
   modes they set) and five slow-reader socket tests in `api`. None is in `costs`; `costs` is all
   green.
3. **Two `api` tests hang, on the base too:**
   `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
   (lib) and `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
   (`tests/cloud_mutants_bars.rs`). Each was run ALONE on the base `cc0a3a0d` (the built test
   binary, `timeout 300`) and killed with exit 124, "has been running for over 60 seconds"; on
   `bf9b1c8f` the same on both trees.
4. **CI shell gates (language-purity, all but 1e): the same 10 red on the base `cc0a3a0d`** (1c, 1d,
   10, 11, 12, 19, 23, 27b, 29, 30), with the same refusals; see the Gates table. Gate 1e was red on
   `bf9b1c8f` and the branch with the same 29 `api`-side names as item 2; not re-run on `cc0a3a0d`.

Nothing is left to build. `crates/costs` is byte-identical from `f854fd09` (where coverage, mutants
and the bench were taken) to the final head, and `feat/db-fix` did not touch `crates/costs` between
`bf9b1c8f` and `cc0a3a0d`, so those results stand for it.

## What was built (code at `f854fd09`)

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
| fmt | `cargo fmt --all --check` | exit 0 on the merged tree `b60b0f46` (`cc0a3a0d` merged, 26 Oct, before 01:41 IST); earlier exit 0 on `a3e04fc2`; `28fe5ecc` changes one doc line only |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 (`Finished`, 2m09s) on the merged tree `b60b0f46`; earlier exit 0 on `a3e04fc2`. Earlier: the first run was red on my own test (`type_complexity`, then `items_after_statements`), fixed in `f1e2d15c` |
| test | `cargo test --workspace --locked --no-fail-fast -j 2 -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit` | on `24ad7e5f` (`cc0a3a0d` merged), both hangs also skipped by name: 254 suites, 5 targets red (`api` lib 8, `cli` lib 3, `pull` lib 9, `store` lib 5, `telemetry` lib 4), all 29 `FAILED` on the base `cc0a3a0d` by exact name; both hangs reproduce alone on `cc0a3a0d` (exit 124). Earlier, on `28fe5ecc`: 251 suites; 6 targets red (`api` lib 8, `api` `cloud_mutants_bars` (one test hung, killed), `cli` lib 3, `pull` lib 9, `store` lib 5, `telemetry` lib 4), all 29 failing tests `FAILED` on the base `bf9b1c8f` too, run there by exact name; both hangs reproduce ALONE on the base (`timeout 300`, exit 124). Every `costs` suite ok (lib 300 passed 1 ignored; adversarial 4; dhan_calculator 12; fa73061 file 11 passed 1 ignored). BASE / ENVIRONMENT (root) |
| costs | `cargo test -p costs` (under both llvm-cov runs, on `f854fd09`'s code) | on `b60b0f46` and under both llvm-cov runs: lib `ok. 300 passed; 1 ignored`, adversarial `ok. 4 passed`, dhan_calculator `ok. 12 passed`, the CHG-0 file `ok. 11 passed; 1 ignored` |
| deny | `cargo deny check` | **red, and red on the unmodified base too**: `advisories FAILED, bans ok, licenses ok, sources ok`, the single advisory RUSTSEC-2026-0386 (tokio 1.53.1; fix tokio >=1.53.3), measured on `7b397bbe` and `666ddf7f` and this branch; lock, `deny.toml` and every manifest identical from `666ddf7f` to `cc0a3a0d` and the final head. Not re-run on 25 Oct after `bf9b1c8f`: no `cargo-deny` in this fresh container, and `Cargo.lock`, `Cargo.toml`, `deny.toml` and every crate manifest are identical from `666ddf7f` to `bf9b1c8f` to this branch. BASE item (BASE-DENY-1024): NEEDS THE MAC |
| CI shell gates (language-purity job, `checks_all`'s own commands, all 34) | 26 Oct, before 01:41 IST, branch `b60b0f46` and base `cc0a3a0d`, all but 1e: 23 green and 10 red on each, the same 10 (1c, 1d, 10, 11, 12, 19, 23, 27b, 29, 30); sorted refusals identical except counts (1754 vs 1751 decision headings), timings and gate 10's line number for the base's own R427-11 (8793 vs 8777). Earlier, branch `a3e04fc2` and base `bf9b1c8f`: 23 green and 11 red on each, the same 11 (1c, 1d, 1e, 10, 11, 12, 19, 23, 27b, 29, 30). Each red's refusals compared sorted: identical except counts (lines in scope; 1747 vs 1744 decision headings, the three D-674x) and gate 10, which on the branch stopped at MY row CHG1OPT-01; fixed in `28fe5ecc`, after which gate 10 stops on the base's own R427-11 as the base does. Gate 1e (workspace test with `web/` deleted): `FAILED. 2537 passed; 29 failed; 16 ignored` in `api` on both, the same 29 names and an identical tail. No red names a file this part touches. BASE |
| coverage, lines | `cargo llvm-cov -p costs --locked` (stable, fresh dir) | `regime.rs` 100%; `schedule.rs` 99.49% (5 missed: 1244, 1311, 1319, 1665, 1674); `stack.rs` 99.83% (3 missed: 589, 720, 907). Every missed line predates this part and lies outside its hunks; **every added line is covered** |
| coverage, branches | `cargo +nightly llvm-cov -p costs --branch --locked` (fresh dir; `--branch` needs nightly, installed here as a tool) | `regime.rs` 25/26, `schedule.rs` 32/34, `stack.rs` 86/88. Every untaken branch (regime 274; schedule 1275, 1664; stack 2878) predates this part; **every added branch is taken**. Nightly also put five untaken short-circuit branches and one unreachable `panic!` line in my new tests; they were fixed in `f854fd09` without weakening an assertion |
| mutants | `cargo mutants --in-diff <costs diff> -p costs` | `3 mutants tested in 50s: 2 caught, 1 unviable`, none survived. Caught: the boundary guard forced `true`, and forced `false`. Unviable: the card replaced by `Ok(Default::default())`; `RateCard` has no `Default` |
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

This part changes no route, page, cli verb, import or store file, so there is **no real-data,
shadow-server, store-write or `:8080` check** (spec section 7). At merge-back, in a worktree of this
branch merged onto the newest `feat/db-fix`, with the shared-pool lock and the `PATH` /
`CARGO_TARGET_DIR` of spec section 7, as the operator (not root):

```text
cargo fmt --all --check                                   # expect exit 0
cargo clippy --workspace --all-targets -- -D warnings     # expect exit 0
cargo test -p costs --locked                              # expect lib ok. 300 passed; 1 ignored, adversarial 4, dhan_calculator 12, fa73061 11 passed 1 ignored
cargo test -p costs --locked --test dhan_calculator -- the_dated_regime_and_todays_table_agree_on_the_live_october_2026_option_cases --exact
                                                          # expect 1 passed
cargo test -p costs --locked --lib -- schedule::tests::the_dated_option_card stack::tests::a_dated_option
                                                          # expect all pass
cargo test -p costs --locked --lib -- fa73061 stack::tests::a_position_carried_across_fa73061 stack::tests::the_smallest_premium stack::tests::fourteen_threads
                                                          # expect all pass (the other new tests, by name)
cargo test -p costs --locked --test fa73061               # expect 11 passed, 1 ignored, unchanged
cargo test --workspace --locked -- --skip the_pool_verb_prints_its_whole_page_on_a_generated_store
                                                          # expect green as on feat/db-fix. Here, as root, the same 29 tests failed on the base and the
                                                          # branch, and the two api tests above hung on both: if either hangs on the Mac, it is feat/db-fix's
cargo deny check                                          # expect green after BASE-DENY-1024 (tokio 1.53.3); red only on RUSTSEC-2026-0386 before it
cargo bench -p costs --locked --bench stack               # expect C-CHG-01..10 within the 3.0x ceiling (here 0.851x-1.659x); record date, host, load
cargo llvm-cov -p costs --branch --locked                 # expect every added line and branch covered (here: lines regime 100%, schedule 99.49%, stack 99.83%,
                                                          # the misses all pre-existing: schedule 1244 1311 1319 1665 1674, stack 589 720 907)
cargo mutants -p costs --file crates/costs/src/schedule.rs --file crates/costs/src/stack.rs --file crates/costs/src/regime.rs --file crates/costs/tests/dhan_calculator.rs
                                                          # expect no surviving mutant on the touched code (here --in-diff: 2 caught, 1 unviable)
checks_all script | bash -s -- <out dir>                  # expect the language-purity gates no redder than feat/db-fix's own (here: the same 10 as cc0a3a0d, 1e not re-run; 11 with 1e on bf9b1c8f)
```

Expected answers (spec section 7): NSE dated card 3553 on 2026-02-27 (3503 + 50) and on 2026-03-02
(3553 + 0); options NSE Dated, buy 65 at Rs 100 and sell 65 at Rs 120: exchange line Rs 5.08 on both
days, STT Rs 8 (0.10%) on both days, 0.15% only from 2026-04-01;
`RateCard::resolve(Options, Nse, 2023-04-01, Dated)` refuses while `ipft_rate(Nse, 2023-04-01)` is 50.
The red runs were taken here (section "Tests failing before, passing after").

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
- 17:34 IST: HOLD lifted. The worker had restarted, and the workspace test run was lost with it.
- 18:05-18:55 IST:
  - merged `origin/feat/db-fix` at `7b397bbe` into `6893521f` (the `docs/05-decisions.md` append
    conflict resolved as both, in merge order);
  - clippy exit 0;
  - deny red on the base's own RUSTSEC-2026-0386;
  - coverage taken in fresh dirs (the first nightly run had mixed its profiles into the stable
    dir; those figures are discarded);
  - test-only coverage fixes, mutants `2 caught, 1 unviable`;
  - pushed `f854fd09`.
- 18:55 IST: HOLD (set 18:16) seen at this save point. NEXT: as in "Held" above.
- 25 Oct, new cloud session: checked out the branch at `12d4faab`. `HOLD` (set 25 Oct 13:0x IST,
  base `40f1cdb2`) is present and there is no `GO-CLOUD-CHG1-OPT`, so no step was started. The two
  runs the previous session left going were lost with its container; NEXT item 1 now means a fresh
  workspace test run. Waiting as the README says. NEXT: unchanged.
- 25 Oct, 13:12 IST: HOLD lifted (base `1dcc33e9`). Merged `origin/feat/db-fix` at `666ddf7f` into
  `bbdd1738` (the `docs/05-decisions.md` append conflict resolved as both, in merge order: D-7300
  first, then D-6740..D-6742). fmt exit 0, clippy exit 0, deny red on the base's RUSTSEC-2026-0386
  only. The workspace test run hung again on
  `a_full_calendar_pool_refuses_the_census_and_the_audit`; the same test run ALONE on the
  unmodified base `666ddf7f` hung the same way (no output for 8 min at 0% CPU, killed). Final gates
  in progress.
- 25 Oct, new cloud session (13:3x-19:0x IST): checked out the branch at `e8890491`. Merged
  `origin/feat/db-fix` at `bf9b1c8f` (Merge R4-18) as `a3e04fc2`, no conflict. fmt and clippy exit 0.
  Shell gates on both trees; gate 10's red on my own row fixed in `28fe5ecc`. Workspace test run
  OOM-killed (two workspaces building at once); deny not re-run (no `cargo-deny` here; no manifest
  or lock change since its last run). 18:52 IST HOLD (base `62267ba5`) seen; report written HELD.
- 25 Oct, 19:23 IST: `GO-CLOUD-CHG1-OPT` released. Base gate 1e: the same 29 `api` reds as the
  branch. Workspace test run alone (`-j 2`): 29 reds and one more hang, every one reproduced on the
  base `bf9b1c8f` (by exact name; the hang alone under `timeout 300`). 20:40 IST HOLD (base
  `17d214c2`, the per-part GO withdrawn) arrived during that attribution run; the step was finished
  and the final report written. STATUS: BLOCKED on base items only; code complete.
- 25 Oct, new cloud session (23:0x IST, `GO` present, no `HOLD`): checked out the branch at
  `d749b4b0`. `origin/feat/db-fix` had moved to `cc0a3a0d` (Merge R4-28), so it was merged as
  `b60b0f46` (the `docs/05-decisions.md` append conflict resolved with `feat/db-fix`'s file kept as
  the exact prefix, D-7185..D-7194 first, then D-6740..D-6742). fmt exit 0; clippy exit 0; every
  `costs` suite green (lib 300 passed 1 ignored, adversarial 4, dhan_calculator 12, fa73061 11
  passed 1 ignored). Language-purity gates (all but 1e) on the branch and the base `cc0a3a0d`: 23
  green and the same 10 red on each (1c, 1d, 10, 11, 12, 19, 23, 27b, 29, 30), refusals identical
  sorted except counts, timings and gate 10's line number (R427-11, 8793 vs 8777). Workspace test
  run in progress.
  Workspace test (`-j 2`, the two known hangs skipped by name): 254 suites, 5 targets red, `api` lib
  8, `cli` lib 3, `pull` lib 9, `store` lib 5, `telemetry` lib 4 (29); the same 29 names run by
  exact name on the unmodified base `cc0a3a0d` all `FAILED` there too (`0 passed; 8/3/9/5/4
  failed`). Every `costs` suite ok. The two hangs are being re-run alone on the base.
- 26 Oct, 01:41 IST: both hangs run alone on the base `cc0a3a0d` (built test binaries, `timeout
  300`): exit 124 each. Final report written; STATUS: BLOCKED on base items only, code complete.
  `GO` present and no `HOLD` throughout this session.
