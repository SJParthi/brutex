# Mutation testing: shard 23 of 30 (job MUT-23)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result. Code only: no market data, none of the 3 parameter tables, and no credential was read.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`.
  - The README-CLOUD.md section 4 manifest check passed: 146 files, sha256
    `c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e`.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM.
  - `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, from `rust-toolchain.toml`.
  - cargo-mutants 27.1.0, installed with `cargo install cargo-mutants --locked`.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 23/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

  - No `--test-package`, `--baseline` or `--timeout` override was needed. This slice holds only `spikefade` mutants, and
    `spikefade`'s tests compile here.
- **cargo-mutants' own baseline** passed: 13.3 s build plus 3.8 s test (MEASURED). It auto-set the test timeout to 20 s.
  - Its test command was `cargo test --verbose --package=spikefade@0.1.0 --locked -- --exact --skip <the 14>`.
- **Wall (MEASURED):** 2026-11-16 11:41:28Z to 12:15:13Z, so 33 min 45 s.
- **Exit code 2:** missed mutants were found. No mutant timed out.

## Result (EXACT)

Shard 23 is list positions 12,006 to 12,527 (0-based) of the workspace's 15,639 mutants under the default `slice` sharding.

- That is **522 mutants, every one in `crates/spikefade/src/v32.rs`**, at lines 125 to 511.
- The tested set equals `cargo mutants --workspace --shard 23/30 --list`, 522 of 522.
- cargo-mutants 27.1.0 tests each mutant against its own package's tests, which here means `spikefade`'s 158 runnable tests.

**522 tested: 316 caught, 202 missed, 0 timeout, 4 unviable.**

| function (`v32.rs`) | span starts at line | caught | missed | timeout | unviable |
|---|---:|---:|---:|---:|---:|
| `iv_band` | 120 | 0 | 12 | 0 | 0 |
| `gain_words` | 133 | 10 | 1 | 0 | 0 |
| `bp_round` | 143 | 19 | 7 | 0 | 0 |
| `gain_words_r` | 159 | 6 | 1 | 0 | 0 |
| `level_words` | 169 | 2 | 0 | 0 | 0 |
| `pct_bp` | 192 | 0 | 7 | 0 | 0 |
| `week_of` | 200 | 5 | 0 | 0 | 0 |
| `month_id` | 206 | 9 | 0 | 0 | 0 |
| `month_name` | 213 | 0 | 2 | 0 | 0 |
| `utilization` | 219 | 5 | 115 | 0 | 2 |
| `margin_mtm_peak` | 252 | 180 | 15 | 0 | 2 |
| `mtm_peak_words` | 286 | 0 | 3 | 0 | 0 |
| `portfolio_mtm` | 296 | 2 | 2 | 0 | 0 |
| `port_walk` | 350 | 36 | 25 | 0 | 0 |
| `v19_of` | 451 | 24 | 4 | 0 | 0 |
| `a19_passed` | 482 | 13 | 7 | 0 | 0 |
| `v19_words` | 493 | 2 | 0 | 0 | 0 |
| `v19_miss_words` | 505 | 3 | 1 | 0 | 0 |
| **all 18 functions** | | **316** | **202** | **0** | **4** |

The 4 unviable mutants are `replace * with /` at `v32.rs:232:60`, `233:88`, `265:60` and `266:88`. They do not compile.

## How to read the 202 missed

- **`v32.rs` has no test module of its own.**
- **Four functions account for 159 of the 202:** `utilization` (115), `port_walk` (25), `iv_band` (12) and `pct_bp` (7).
  - No test in the crate calls any of the four by name.
  - They are reached only through report text: `report3.rs` (lines 5037 and 5259), `side17.rs` (lines 217, 421 and 432),
    and `v32.rs`'s own report writers (lines 305, 561, 592 and 819).
  - The tests that walk those paths do not pin these numbers. `iv_band` and `pct_bp` have no catching test at all.
- **The other 43** are spread over 10 functions. The largest are `margin_mtm_peak` (15 missed of 197), and `bp_round` and
  `a19_passed` (7 each).
  - These functions are named in `tests32r.rs` or `tests33g.rs`, and most of their mutants are caught.
- **The exclusions are unlikely to explain any of the 202.**
  - None of the 9 excluded `spikefade` tests names a `v32.rs` function, `report3`, `side17` or `run3` in its body.
  - This was checked by direct reference only. Indirect paths were not traced.
  - The 5 excluded `core` tests are not in `spikefade`'s test run at all.

## Baseline (`baseline.log`)

Every step is in `baseline.log` with its command, rc and wall time.

| step | command | result (EXACT) |
|---|---|---|
| 2a | `cargo test --locked --workspace`, as instructed | rc 101: does not compile; 0 tests ran |
| 2a-x | each of `core`'s 6 non-compiling targets alone (`--no-run`) | rc 101 each |
| 2b | every compilable target, `--no-fail-fast`, no exclusions | 658 passed, **14 failed**, 3 ignored |
| 2c | as 2b with the 14 skips | 658 passed, 0 failed, 3 ignored, 14 filtered out |
| 2d | `cargo test --package=spikefade@0.1.0 --locked -- <the 14 skips>` | 158 passed, 0 failed, 1 ignored, 9 filtered out |

- 2b and 2c cover `costs` (428 passed, 2 ignored) and `greeks` (59 passed).
- They also cover `spikefade` (158 passed, 1 ignored, and 9 failed in 2b).
- For `core` they cover `bound_probe`, `cost_invariants`, `lint`, `zerodha_index` and 4 doctests: 13 passed, and 5 failed in 2b.

## Exclusions

### `core` targets that cannot compile here

Every one fails on `include_str!` of a file of the app repository that this branch does not hold. These targets do not
affect this shard, because `spikefade`'s test run never builds them:

- the lib unit tests;
- `tests/banned_lists.rs`, `tests/claim_reader.rs`, `tests/decision_citations.rs`, `tests/findings.rs` and `tests/graph.rs`.

### Tests that fail at run time, skipped by exact name

These are the same 14 that shards 0 to 3 skipped. Each failure was re-derived on this box in step 2b, with the cause read
from its panic message.

| test | cause on this box |
|---|---|
| `menu::tests::sf3_distance_floor_counted` | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` and `-SETS-v1.tsv` are absent, so the parse fails with `menu_head: missing` |
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | the same |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | `EXITS-DYN-MENU-v1.tsv`: No such file or directory |
| `tests3::sf3_ttp_replaces_tp_close` | the same file: NotFound |
| `tests34::root_digest_same_for_every_form` | runs `/opt/homebrew/bin/zstd`, the Mac default (`kit.rs:470`). Without PORTABLE, `SF_ZSTD` is not consulted, and this box has no zstd |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (`core` `cost_invariants`) | `docs/04-invariants.md` is absent |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` (`core` `cost_invariants`) | `crates/spikefade/benches/ratio.rs` is absent |

The first 7 are the 7 that README-CLOUD.md section 5 predicts.

**Judgement call:** the 2 `tests34` tests need a file outside the repository, the Mac's Homebrew `zstd`. They are skipped as
shards 0 to 3 skipped them. Neither touches `v32.rs`.

## Files

- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt` (empty) and `unviable.txt`: copied unchanged from `mutants.out/`.
- `baseline.log`: steps 2a to 2d above.
