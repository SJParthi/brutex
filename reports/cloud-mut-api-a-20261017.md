STATUS: IN PROGRESS

# cloud-mut-api-a-20261017 — the api half-A mutation and coverage pass, carried onto the newest `feat/db-fix` and finished

Branch `cloud/mut-api-a-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584`. It carries the 16 Oct pass (`origin/cloud/mut-api-a-20261016`,
report `reports/cloud-mut-api-a-20261016.md` on that branch) and works its
NOT DONE list. **This report holds no authority** (`CLAUDE.md` §10): it is
evidence of what was measured on this date, on this machine. Every number
below was measured in this session; none is extrapolated.

**Scope**, unchanged from 16 Oct: `crates/api/src/` `bars.rs`,
`bars_change.rs`, `bars_series.rs`, `bars_sort.rs`, `gaps_route.rs`,
`spot.rs`, `render.rs`, `ladder.rs`. Not touched: `bars_route.rs`,
`facets.rs`, `livejson.rs` (the Mac merge-back CLOUD-ATTACKO1 owns them) and
half B's seven files. Outside `crates/api/src`, this pass edits only its new
`tests/cloud_mutants_bars.rs` tests, `coverage.rs`'s timing test,
`docs/04-invariants.md`, `docs/05-decisions.md` and this report.

## Done so far

1. **Carried the 16 Oct pass onto `feat/db-fix` `f9cdb584`.**
   `git diff c9e3472e5 origin/cloud/mut-api-a-20261016 -- . ':(exclude)reports' | git apply -3`:
   six files applied cleanly; one conflict, at the end of `bars.rs`, where
   `feat/db-fix` added `mod unknown_series_tests` (AG-03) and the pass added
   `mod cloud_mutant_tests`. Both are kept. No `feat/db-fix` line was dropped.
2. **Adapted the carried tests to what `feat/db-fix` changed under them**,
   each keeping its assertion:
   - `gaps_route.rs`: `second_audit`, `trade_days` and `one_second_members`
     now read a `MonthSummary` (R3-06) and `run_word` an `ImportReach`
     (R3-05). The carried tests build the summary with the writer's own
     `MonthSummary::fold` and pass a reach that says nothing.
   - `tests/cloud_mutants_bars.rs`: AG-03 refuses a key that names no held
     series. `bars_exactly_max_series_series_are_read` now plants one bar in
     each of the 1,024 series and counts 1,024 rows;
     `bars_only_a_gdfl_nse_option_is_decoded_for_its_successor` plants one
     bar under each feed and exchange it asks.
3. **The 18 NOT DONE survivors of 16 Oct have tests** (D-5422), through three
   `#[cfg(test)]` seams on the existing `READ_HELD` precedent, each compiled
   out of every non-test build:
   - `bars::cancel_trip`: a `Cancel::pair` flag that reads "cancelled" from
     a look the test chose, on the test's thread (`Cancel::never` untouched).
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

4. **Eighteen 16 Oct "equivalent" survivors are not equivalent**: each changes
   something the function's own contract states, and now has a test.

   | 16 Oct survivor | Why it is observable | New test |
   |---|---|---|
   | `bars_sort.rs:132`, `:137` budget constants (8) | What the fence and cut memos hold between requests | `bars::sort::cloud_mutant_tests::the_index_memos_hold_exactly_their_budgets` (child process) |
   | `bars_change.rs:473:49` memo guard → `false` | Whether the lineage ledger is read again under an unmoved stamp | `bars::change::cloud_mutant_tests::the_lineage_ledger_is_read_once_per_stamp` |
   | `bars_series.rs:1258:9` `clear_unread` → `()` | The ledger a page asked again starts from | `series::cloud_mutant_tests::clearing_the_unread_ledger_forgets_what_a_read_left_unread` |
   | `bars_series.rs:978:42` `+` → `*` in `rank` | The run and row `rank` answers past a run's end | `series::cloud_mutant_tests::an_instant_past_a_days_window_ranks_at_the_next_days_first_row` |
   | `bars_series.rs:1115:60` `+` → `*` in `slot_of` | A month after every slot: the search never ends | `series::cloud_mutant_tests::a_month_outside_the_slots_has_no_slot` |
   | `bars_sort.rs:406:44` `&&` → `\|\|` in `view` | An index set aside under a stamp it no longer has | `bars::sort::cloud_mutant_tests::an_index_is_set_aside_only_at_its_own_path_and_stamp` |
   | `bars_sort.rs:1595:14`, `:1598:56` in `run_of_slot` | A slot past every run: the search never ends | `bars::sort::cloud_mutant_tests::a_slot_past_every_run_has_no_run` |
   | `bars_sort.rs:384:21` guard → `true` in `view` | An index the host cannot stat (the store changed in the request) | `bars::sort::cloud_mutant_tests::an_index_the_host_cannot_stat_is_named_with_its_path` |
   | `bars.rs:2185:43`, `:2185:64` in `scan` | The scan's open-interest null rule, asked directly | `bars::cloud_mutant_tests::the_scan_orders_open_interest_with_its_nulls_last` |

   The budget-constant and lineage tests were each checked by hand against
   the mutants they name (the mutated tree fails them; the restored tree
   passes).
5. **Branch coverage is measured**, for the first time on these files. The
   16 Oct pass could not (`cargo llvm-cov --branch` needs a nightly), and
   half B saw `0/0` branches. On the pinned stable `1.97.1`,
   `RUSTC_BOOTSTRAP=1 cargo llvm-cov --branch` instruments branches (a probe
   crate reported 4 branches, 2 missed). The switch is a measurement flag
   for that run only: `rust-toolchain.toml` and every build input are
   unchanged.
6. **Coverage tests for every reachable gap the line and branch run named**
   (first measurement, at `9d85bcd6`: 106 production lines and 118 branches
   unrun across the eight files). Tests were added in all eight files'
   cloud modules (a new one in `bars_series.rs`, for its private items);
   the re-measurement is below.
7. **NOT DONE item 5, the load-sensitive timing test**:
   `coverage::tests::the_lookup_does_not_grow_with_the_universe` takes the
   fastest of seven interleaved samples a size (D-0800's method, D-5421).
   Ceiling and production code unchanged. It passed in an instrumented run
   with two mutation workers loading the machine.
8. **NOT DONE item 7, the documents**: D-5420 to D-5423 in
   `docs/05-decisions.md`; MUT-A-01 to MUT-A-06 in `docs/04-invariants.md`.

## Real defects found

1. **Carried from 16 Oct, now documented (D-5420): an `at=` instant past the
   year 9999 ranked before every row** (`bars_series.rs`, `bucket_of`).
2. **New, latent (D-5423): the bars scan ordered a null open interest by the
   number a vendor stamped.** For an index or a share, open interest is not
   applicable and every row is a null (D-3322), but the scan's candidate
   kept the stamped number and the comparator ordered nulls by it before
   time: `[30, null, 10, null, 20]` came back `30, 20, 10, null, null`
   descending. `SortKey::of` says a null is never ordered as a real number.
   Fixed at the cause: a null candidate's value is `OI_NULL`. No route sends
   open interest to the scan today, so no served page changes; the test
   (`the_scan_orders_open_interest_with_its_nulls_last`) failed before the
   fix (checked by reverting it in the dev copy) and passes after it.

**Observed, not changed.** A month with an index and no time index whose
first bisected record is damaged names one line per page row the index put
on the page (each carries its row's instant, so `push_once` keeps them all):
37 lines for a page of 37. Bounded by the page limit; whether to coalesce
them is a design call for the route's owner. Pinned as it is by
`a_row_whose_instant_cannot_be_located_is_named_and_not_served`.

## Method

| Item | Value |
|---|---|
| toolchain | `1.97.1` (pinned by `rust-toolchain.toml`) |
| cargo-mutants | `27.1.0` |
| cargo-llvm-cov | `0.9.1` |
| cargo-deny | `0.20.2` |
| machine | 4 cores, 15 GiB, cloud container, runs as `root` |

**Mutation.** Two workers, each a full copy of the tree with its `.git`
moved aside (the reason is 16 Oct's: `cli`'s build script watches every
tracked file, so an in-place mutation rebuilt `cli` too), run `cargo mutants
--in-place --baseline skip --timeout 300 -p api --file <f> -C=--lib` with
the `api` crate at `opt-level = 0`, incremental, no debug info
(`--config profile.{dev,test}.package.api…`). Dependencies keep the checked-in
profile. Stage 1 runs each file's mutants against the library tests its
filter names (16 Oct's filters). Every stage-1 survivor is then run against
the final suite (the whole library and `tests/cloud_mutants_bars.rs`).
Gates and coverage use the checked-in profile unchanged.

**Coverage.** `RUSTC_BOOTSTRAP=1 cargo llvm-cov --no-report -p api --branch`
as `root`, then the twelve permission tests (below) run as uid 65534 from
the same instrumented binary into the same profile directory, then
`cargo llvm-cov report --branch --lcov`. Production lines and branches are
those outside every top-level `#[cfg(test)]` item of the file.

**Root.** Twelve `api` library tests fail as `root` and pass as uid 65534:
16 Oct's ten, plus `pullrun::restart_tests::the_journals_own_extremes` and
`server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`,
new on `feat/db-fix`. Each needs mode bits that bind the process.

## Gates (so far)

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | pass, at every commit |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | pass (exit 0) at `33dc17d1`; to be run again at the end |
| `cargo test -p api --locked --no-fail-fast`, as root, at `33dc17d1` | lib 2,179 passed, 12 failed (the twelve above), 8 ignored; `tests/cloud_mutants_bars.rs` 58 passed; `src/main.rs` 4; `tests/binary.rs` 4; doc-tests 1 |
| the twelve as uid 65534 | 12 passed |
| `cargo deny check` | **same as the base**: on `f9cdb584` and on this branch, `advisories FAILED` on exactly RUSTSEC-2026-0344 (`ruzstd`) and RUSTSEC-2026-0346 to 0349 (`rustls`), with the same two duplicate warnings (`getrandom`, `syn`); `bans`, `licenses`, `sources` ok. No manifest or lock file is touched. |

## Left

- The mutation run (2,218 mutants of the eight files) is still running;
  then every survivor against the final suite.
- The coverage re-measurement after the new tests, and the reason for
  every line and branch still unrun.
- `cargo test --workspace --locked`, and the final clippy.

## NEEDS THE MAC

- **Decision numbers.** D-5420 to D-5423 were taken above every remote
  ledger head (D-5301) on 17 Oct. Local branches may hold more; amendment
  A6 renumbers at landing.
- Nothing in `bars_route.rs`, `facets.rs` or `livejson.rs` yet.
