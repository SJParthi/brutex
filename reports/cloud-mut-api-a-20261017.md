STATUS: IN PROGRESS

# cloud-mut-api-a-20261017 — the api half-A mutation and coverage pass, carried onto the newest `feat/db-fix` and finished

Branch `cloud/mut-api-a-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584`. It carries the 16 Oct pass (`origin/cloud/mut-api-a-20261016`,
report `reports/cloud-mut-api-a-20261016.md` on that branch) and works its
NOT DONE list. **This report holds no authority** (`CLAUDE.md` §10): it is
evidence of what was measured on this date, on this machine. Every number
below was measured in this session; none is extrapolated.

Scope, unchanged from 16 Oct: `crates/api/src/` `bars.rs`, `bars_change.rs`,
`bars_series.rs`, `bars_sort.rs`, `gaps_route.rs`, `spot.rs`, `render.rs`,
`ladder.rs`. Not touched: `bars_route.rs`, `facets.rs`, `livejson.rs` (the
Mac merge-back CLOUD-ATTACKO1 owns them) and half B's seven files.

## Done

1. **Carried the 16 Oct pass onto `feat/db-fix` `f9cdb584`.**
   `git diff c9e3472e5 origin/cloud/mut-api-a-20261016 -- . ':(exclude)reports' | git apply -3`:
   six files applied cleanly; one conflict, at the end of `bars.rs`, where
   `feat/db-fix` added `mod unknown_series_tests` (AG-03) and the pass added
   `mod cloud_mutant_tests`. Both are kept. No `feat/db-fix` line was dropped.
2. **Adapted the carried tests to what `feat/db-fix` changed under them**,
   keeping each test's assertion:
   - `gaps_route.rs`: `second_audit`, `trade_days` and `one_second_members`
     now read a `MonthSummary` (R3-06) and `run_word` an `ImportReach`
     (R3-05). The carried tests build the summary with the writer's own
     `MonthSummary::fold` and pass a reach that says nothing.
   - `tests/cloud_mutants_bars.rs`: AG-03 refuses a key that names no held
     series. `bars_exactly_max_series_series_are_read` now plants one bar in
     each of the 1,024 series and counts 1,024 rows;
     `bars_only_a_gdfl_nse_option_is_decoded_for_its_successor` plants one
     bar under each feed and exchange it asks.
3. **The 18 NOT DONE survivors of 16 Oct now have tests** (D-5422), through
   three `#[cfg(test)]` seams on the existing `READ_HELD` precedent:
   - `bars::cancel_trip`: a `Cancel::pair` flag that reads "cancelled" from
     a look the test chose, on the test's thread (`Cancel::never` is
     untouched; nothing of it exists in a non-test build).
   - `bars::sort::SCAN_BUDGET` / `EXTREMES_BUDGET`: thread-local lowerings of
     `MAX_SCAN_BARS` and `MAX_EXTREMES_SCAN_BARS`, read through
     `scan_budget()` / `extremes_budget()`. Production values and sentences
     are unchanged.

   | 16 Oct survivor (old line) | New test |
   |---|---|
   | `spot.rs:535:19` `%` → `/` | `server::spot::cloud_mutant_tests::a_client_gone_mid_read_is_seen_at_the_next_look_of_the_stride` |
   | `bars.rs:1603:29`, `bars.rs:1674:29` guard → `false` | `bars::cloud_mutant_tests::a_client_gone_while_the_extremes_fold_ends_the_request_as_cancelled` |
   | `bars_series.rs:1222:20` `+=` → `*=`, `:1223:23` `>=` → `<` | `bars::cloud_mutant_tests::a_stretch_read_looks_at_its_client_once_a_stride` |
   | `bars_sort.rs:1996:18` `>` → `==`, `>=` | `bars::sort::cloud_mutant_tests::a_value_page_is_served_to_the_scan_budget_and_refused_past_it` |
   | `bars_sort.rs:2259:45`, `:2261:52` (2), `:2265:17` (2), `:1680:5` (2) | `bars::sort::cloud_mutant_tests::a_day_cut_value_page_owes_only_unfolded_cut_rows_to_the_extremes_budget` |
   | `bars_sort.rs:2523:39`, `:2526:48`, `:2531:13` (2), `:1680:5` (2) | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_cut_runs_owe_only_unfolded_rows_to_the_extremes_budget` |

   Whether each is now killed is measured by the mutation run below.
4. **NOT DONE item 5, the load-sensitive timing test**:
   `coverage::tests::the_lookup_does_not_grow_with_the_universe` now takes
   the fastest of seven interleaved samples a size (D-0800's method,
   D-5421). Ceiling and production code unchanged.
5. **NOT DONE item 7, the documents**: D-5420 (the 16 Oct `bucket_of` fix),
   D-5421, D-5422 in `docs/05-decisions.md`; MUT-A-01 to MUT-A-05 in
   `docs/04-invariants.md`.
6. **NOT DONE item 3, branch coverage**: it CAN be measured on the pinned
   stable `1.97.1`. `RUSTC_BOOTSTRAP=1 cargo llvm-cov --branch` instruments
   branches (a probe crate reported 4 branches, 2 missed). The switch is a
   measurement flag for this run only; `rust-toolchain.toml` is unchanged.

## Left

- The full mutation run of the eight files on this base (2,218 mutants),
  then survivors against the integration file and the whole library.
- Line and branch coverage of the eight files, then tests or reasons for
  every line and branch left.
- `cargo test --workspace --locked`, as root and the root-failing tests as
  uid 65534; `cargo deny check` against the base.

## Gates (so far)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass (exit 0), at `33dc17d1` |
| `cargo test -p api --locked --no-fail-fast`, as root | lib 2,179 passed, 12 failed, 8 ignored; `tests/cloud_mutants_bars.rs` 58 passed (after the AG-03 fix); `src/main.rs` 4, `tests/binary.rs` 4, doc-tests 1 |
| the 12 lib failures | every one is a permission test that `root` cannot fail: the ten 16 Oct named, plus `pullrun::restart_tests::the_journals_own_extremes` and `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted` (new on `feat/db-fix`). Both new ones pass as uid 65534. |

## NEEDS THE MAC

- **Decision numbers.** D-5420 to D-5422 were taken above every remote
  ledger head (D-5301) on 17 Oct. Local branches may hold more; amendment A6
  renumbers at landing.
- Nothing in `bars_route.rs`, `facets.rs` or `livejson.rs` yet.
