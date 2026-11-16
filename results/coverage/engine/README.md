# COV-1: coverage of the spike-selling engine workspace (cloud, code only)

Labels follow README-CLOUD.md: MEASURED = a number from the run named here; EXACT = a count. No figure here is a trading
result.

## 1. What was measured

- Branch `cloud/spikefade-shard-20261116`, commit `15fd4f4737ea11bcef398efa75db240d38128d10`.
- Source manifest as README-CLOUD.md section 4 builds it: 146 files, sha256
  `c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e`, equal to the expected value.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)`, cargo 1.97.1, cargo-llvm-cov 0.9.1. Dev profile (no `--release`).
  `SF_SOURCE_SHA256` and `SF_RUSTC` unset (no test reads them).
- Cloud box: Linux x86_64, 4 vCPU, 15 GiB RAM, 16 Nov 2026.
- Code only: no market data, no parameter tables, nothing fetched.

## 2. Result (MEASURED)

| scope | regions | functions | lines |
|---|---|---|---|
| **all 88 reported files** | **60.61%** (62,375 / 102,912) | **59.76%** (3,012 / 5,040) | **70.45%** (29,978 / 42,550) |
| `core` (11 files) | 98.23% (9,548 / 9,720) | 97.46% (499 / 512) | 97.93% (5,583 / 5,701) |
| `costs` (24 files) | 99.20% (20,581 / 20,747) | 99.49% (979 / 984) | 99.49% (11,872 / 11,933) |
| `greeks` (5 files) | 99.86% (2,161 / 2,164) | 100.00% (85 / 85) | 99.86% (1,436 / 1,438) |
| `spikefade` (48 files) | 42.81% (30,085 / 70,281) | 41.89% (1,449 / 3,459) | 47.22% (11,087 / 23,478) |
| all, without `spikefade/src/tests*.rs` (78 files) | 57.30% | 57.21% | 68.57% (27,080 / 39,491) |

- **Branches: NOT MEASURED.** On the pinned stable toolchain, `cargo llvm-cov --branch` stops with
  `error: the option 'Z' is only accepted on the nightly compiler`. The Branches columns of `cov-summary.txt` read 0 and
  `-` for that reason.
- cargo-llvm-cov counts test code that lives under `src/` by default: the ten `spikefade/src/tests*.rs` modules and every
  in-file `#[cfg(test)]` module. The last row leaves out the ten files. In-file test modules cannot be split out by file.
- **Not in the report:**
  - `pinned/common-fill-v24`, the engine's path dependency. It is not a workspace member, and the report holds no record
    of it.
  - Doctests. cargo-llvm-cov runs them only on nightly. All 67 pass in the baseline.
- `spikefade/src/report3.rs` (4,224 missed lines) and `spikefade/src/report.rs` (2,203) hold 51% of the 12,572 missed lines.

## 3. Baseline (`cargo test`)

- **Attempt 1:** `cargo test --locked --workspace --no-fail-fast`, the specified command plus `--no-fail-fast`. It stopped
  with rc 101 after 2.5 s (MEASURED), and no test ran. `core`'s unit-test target does not compile: five `include_str!`
  calls in `crates/core/src/universe.rs` name app-repository files that this branch does not hold (`docs/05-decisions.md`
  twice, `docs/06-limits.md`, `docs/00-charter.md`, `CLAUDE.md`).
- **Attempt 2 (the baseline used):** the same suite, with the build-level exclusions and placeholders of section 4.
  - Wall time: 31 s (MEASURED): 24.8 s for the workspace without `core`, 6.5 s for `core`, 0.4 s for `core`'s doctests.
  - Counts (EXACT): **861 passed, 16 failed, 3 ignored**, plus 38 tests not built.
  - The 3 ignored tests are the suite's own `#[ignore]` measurement tests:
    - `lot::table_tests::a_lookup_is_flat_in_the_table` (costs);
    - `the_dated_lookups_cost_the_same_on_every_row_measured` (costs, `fa73061`);
    - `split18::tests::v32b_split_cost_97_months` (spikefade).
  - All 16 failures need a file this branch does not hold (section 4). Nothing else failed.

## 4. Exclusions (every one needs a file this branch does not hold)

**Not built (38 tests).** Each of these `core` integration targets is a test crate built around `include_str!` constants
naming app-repository files, so it cannot compile here:

| target | tests | missing files |
|---|---|---|
| `banned_lists` | 2 | `.github/workflows/ci.yml`, `deny.toml`, `crates/vocab/tests/workspace_is_rust.rs` |
| `claim_reader` | 3 | `.github/workflows/ci.yml`, `.github/gates_bounds.rs` |
| `decision_citations` | 4 | `docs/05-decisions.md`; at run time also `docs/04-invariants.md` |
| `findings` | 14 | `docs/11-findings.md` |
| `graph` | 15 | `docs/01-architecture.md` and the manifests of ten crates that are not in this branch |

None of the five calls `core`'s library: `graph.rs` names `brutex_core` in a comment only. Dropping them removes no
production-code coverage.

**Skipped by exact name (16 tests).** Passed as `-- --exact --skip NAME`. Before the run, each name was listed against
every test binary and matches exactly one test in one binary.

| test | binary | missing file |
|---|---|---|
| `menu::tests::sf3_distance_floor_counted` | spikefade | the parameter tables `crates/spikefade/data/EXITS-DYN-*.tsv` |
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | spikefade | the same |
| `menu::tests::sf3_menu_fail_closed` | spikefade | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | spikefade | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | spikefade | the same |
| `tests3::sf3_ttp_replaces_tp_close` | spikefade | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` |
| `tests33s::v33_search1d_sets_load_and_count` | spikefade | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` |
| `tests34::root_digest_same_for_every_form` | spikefade | `/opt/homebrew/bin/zstd`, the Mac default tool path (the test data is synthetic) |
| `tests34::sfz_roundtrip_all_rows` | spikefade | `/opt/homebrew/bin/zstd`, the same |
| `universe::tests::d0506s_fno_probe_figures_are_ntm_indexs_and_the_limits_correct_them` | core (lib) | `docs/05-decisions.md`, `docs/06-limits.md` |
| `universe::tests::the_surface_counts_the_records_give_are_the_two_lists_counts` | core (lib) | `docs/00-charter.md`, `docs/05-decisions.md`, `CLAUDE.md` |
| `every_declared_cost_invariant_reaches_a_bench` | core `cost_invariants` | `docs/04-invariants.md` |
| `every_measured_cost_invariant_is_declared` | core `cost_invariants` | `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | core `cost_invariants` | `docs/04-invariants.md` |
| `the_reconciliation_is_reading_something` | core `cost_invariants` | `docs/04-invariants.md` |
| `every_crate_carries_a_ratio_bench` | core `cost_invariants` | `crates/spikefade/benches/ratio.rs` (see below) |

- **The 7 parameter-table tests** are exactly the 7 that README-CLOUD.md section 5 predicts.
- **`every_crate_carries_a_ratio_bench` is a judgment call.** It enforces the app repository's rule that every crate
  under `crates/` carries `benches/ratio.rs`. This branch adds `spikefade` without one. The test walks directories only,
  so skipping it changes no coverage figure.

**Placeholders.** Four EMPTY files existed only during the runs: `docs/00-charter.md`, `docs/05-decisions.md`,
`docs/06-limits.md` and `CLAUDE.md`. They were untracked, never committed, and have been removed.

- They let `core`'s unit-test target compile, which keeps its other 203 tests in the measurement.
- Their only readers are the two `universe::tests` skipped above. Both failed against the empty files in attempt 2, and
  no test that ran reads them.

## 5. Coverage commands (MEASURED wall 38 s)

```sh
cargo llvm-cov clean --workspace
cargo llvm-cov --no-report --locked --workspace --exclude core --no-fail-fast -- --exact --skip NAME ...   # the 16 names
cargo llvm-cov --no-report --locked -p core --lib --test bound_probe --test cost_invariants --test lint --test zerodha_index \
  --no-fail-fast -- --exact --skip NAME ...                                                                # the same 16
cargo llvm-cov report --lcov --output-path lcov.info
cargo llvm-cov report --summary-only > cov-summary.txt
```

- **Why two runs:** one `--workspace` run cannot build `core`'s five excluded targets. The work is split in two and merged
  by `report`, which is cargo-llvm-cov's documented merge.
- **The instrumented run (EXACT):** 794 passed, 0 failed, 3 ignored, 16 filtered out (spikefade 9, core lib 2,
  `cost_invariants` 5).

## 6. Files

| file | content |
|---|---|
| `lcov.info` | the LCOV report, 88 source files |
| `cov-summary.txt` | `cargo llvm-cov report --summary-only` |
| `baseline.log` | baseline attempt 1, then attempt 2, raw |
| `cov.log` | the instrumented run, raw |
