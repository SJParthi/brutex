# Mutation testing: shard 26 of 30 (job MUT-26)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. Code only: no market
  data, none of the 3 parameter tables, and nothing added to the checkout. No tool was installed on the box except
  cargo-mutants. zstd is not installed.
- **Box:** a Linux x86_64 cloud container with 4 vCPU (Intel Xeon @ 2.10GHz) and 15.7 GiB RAM.
  `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
- **cargo-mutants 27.1.0,** from `cargo install cargo-mutants --locked`. It was crates.io's latest at 11:40Z, and shards 0
  to 3 used the same version.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 26/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
    --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts \
    --skip menu::tests::sf3_menu_fail_closed \
    --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry \
    --skip menu::tests::sf3_distance_floor_counted \
    --skip menu::tests::sf3_tp_theta_level_and_absence \
    --skip tests33s::v33_search1d_sets_load_and_count \
    --skip tests3::sf3_ttp_replaces_tp_close \
    --skip tests34::root_digest_same_for_every_form \
    --skip tests34::sfz_roundtrip_all_rows \
    --skip every_declared_cost_invariant_reaches_a_bench \
    --skip the_document_declares_the_five_operations_rule_4_names \
    --skip every_measured_cost_invariant_is_declared \
    --skip every_crate_carries_a_ratio_bench \
    --skip the_reconciliation_is_reading_something
  ```

  - This is shard 3's command with `26/30` in place of `3/30`.
  - Shard 0's `--cargo-arg=--test=...` selectors name `core` targets. They are left out: this slice holds no `core` mutant,
    and `cargo test -p spikefade --test bound_probe` is refused.
- **Wall (MEASURED):** 2026-11-16 11:43:35Z to 12:05:45Z, so 1,330 s (22 min 10 s).
  - cargo-mutants' own `start_time` and `end_time` in `outcomes.json` are 11:43:38.8Z and 12:05:45.9Z.
- **cargo-mutants' baseline:** it ran on the unmutated tree and passed in 16 s build plus 5.8 s test, with 158 passed,
  1 ignored and 9 filtered out. It then set the test timeout to 30 s itself.
- **Exit code 2:** at least one mutant was missed, and none timed out.

## What slice 26 is (EXACT)

- `cargo mutants --workspace --list` gives 15,639 mutants: core 642, costs 1,312, greeks 345 and spikefade 13,340. These are
  the same counts shards 0 to 3 report.
- **The slice.** With the default `slice` sharding the chunk is 522, so shard 26 is list positions 13,572 to 14,093
  (0-based). `--list --shard 26/30` was diffed against those 522 lines of the full list and is identical.
- All 522 mutants are in `crates/spikefade/src/split18.rs`, lines 1131 (`pnum`) to 1627 (`picks_tsv`).
- **How each mutant was tested.** The argv is recorded in `outcomes.json`:
  - built with `cargo test --no-run --package=spikefade@0.1.0 --locked`;
  - tested with `cargo test --package=spikefade@0.1.0 --locked -- --exact --skip <the 14>`.
  - That is `spikefade`'s own 158 runnable tests. `core`'s non-compiling targets are never built in this shard.

## Result (EXACT, from `outcomes.json`)

| function (`split18.rs`) | first line | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|---:|
| `pnum` | 1131 | 0 | 0 | 0 | 1 | 1 |
| `fig_line`, `fig_of`, `c_line`, `c_of`, `sp_line`, `sp_of`, `s18b_line` | 1135 | 11 | 0 | 0 | 0 | 11 |
| `s18b_parse` | 1179 | 1 | 0 | 0 | 4 | 5 |
| `ckpt_lines_of` | 1193 | 3 | 0 | 0 | 0 | 3 |
| `better_pick` | 1207 | 0 | 2 | 0 | 0 | 2 |
| `Sel18::ckpt_lines` | 1214 | 3 | 0 | 0 | 0 | 3 |
| `Sel18::batch_start` | 1221 | 0 | 1 | 0 | 0 | 1 |
| `Sel18::absorb` | 1229 | 1 | 4 | 0 | 1 | 6 |
| `Sel18::batch_parts` | 1251 | 1 | 71 | 0 | 288 | 360 |
| `Sel18::ckpt_restore` | 1262 | 5 | 0 | 0 | 0 | 5 |
| `better` | 1290 | 6 | 5 | 0 | 0 | 11 |
| `fresh_ok` | 1296 | 20 | 0 | 0 | 0 | 20 |
| `account` | 1307 | 30 | 17 | 0 | 1 | 48 |
| `fig_words` | 1486 | 2 | 10 | 0 | 0 | 12 |
| `verdict`, `quant`, `section`, `combo_table`, `tsv`, `picks_tsv` | 1508 | 0 | 34 | 0 | 0 | 34 |
| **all of slice 26** | | **83** | **144** | **0** | **295** | **522** |

`Sel18::batch_parts` returns a 6-tuple. Its 360 return-value replacements are 69% of the slice: 288 do not compile, and of the
72 that do, 71 are missed.

## Read this before using the CAUGHT count: 11 catches come from a timing gate under load

`split18::tests::v32rb_split_o1_gates` asserts wall-clock bounds: `close` p99 under 20 µs, `account` p99 under 2 ms, and an
`account` p50 that grows at most 3x (+2 µs) from 25 to 97 months.

- **In 11 of the 83 caught mutants it was the only test that failed.** Every one of the 11 failed on the `account` p99 bound,
  with a p99 between 4,038,563 and 4,061,075 ns.
- **Re-check (MEASURED; not in `outcomes.json`; its files are in `recheck-jobs1/`):**
  - The same 11 were re-run with `--jobs 1` and the same skips and `--cargo-arg=--locked`, into an output directory outside
    the checkout, from 12:09:47Z to 12:12:12Z (145 s).
  - **All 11 were MISSED:** the gate passed in every one, and no other test failed.
  - Each was run once.
- **So these 11 are catches by a timing assertion under the two-job load, not by any test of the mutated behaviour.** Counted
  as missed, the slice is **72 caught and 155 missed**. The official files above are unchanged.
- The 11:

  ```
  crates/spikefade/src/split18.rs:1235:21: replace += with -= in Sel18::absorb
  crates/spikefade/src/split18.rs:1251:9: replace Sel18::batch_parts -> (u64, u64, u64, Vec<u32>, Vec<usize>, Vec<(usize, usize, Best)>) with (1, 1, 0, vec![0], vec![], vec![])
  crates/spikefade/src/split18.rs:1290:5: replace better -> bool with true
  crates/spikefade/src/split18.rs:1290:8: replace > with < in better
  crates/spikefade/src/split18.rs:1290:38: replace < with <= in better
  crates/spikefade/src/split18.rs:1327:23: replace + with * in account
  crates/spikefade/src/split18.rs:1335:14: replace += with *= in account
  crates/spikefade/src/split18.rs:1344:19: replace += with *= in account
  crates/spikefade/src/split18.rs:1348:26: replace += with *= in account
  crates/spikefade/src/split18.rs:1489:44: replace < with == in fig_words
  crates/spikefade/src/split18.rs:1489:44: replace < with <= in fig_words
  ```

- **A tool quirk met on the way (cargo-mutants 27.1.0).** The `-F` and `-E` name filters do not apply to
  `delete field ... from struct ... expression` mutants.
  - One anchored `-F` pattern lists all 206 of them in the workspace, and `-E 'delete field'` removes none.
  - The re-check therefore selected with `-f crates/spikefade/src/split18.rs` plus the 11 anchored `-F` patterns. That also
    took this file's 5 field-deletion mutants, at lines 91 and 1111, which lie outside slice 26.
  - They ran and are in `recheck-jobs1/outcomes.json`, but they are not part of this shard: `91:28` was caught and the four
    at `1111` were missed.
  - Any follow-up that selects mutants by name should diff its `--list` before it runs.

## Baseline (`baseline.log`, MEASURED)

| section | command | result |
|---|---|---|
| 1 | `cargo test --locked --workspace` (as instructed) | rc 101 after 4.2 s, 0 tests ran: `core`'s test targets do not compile |
| 2 | `cargo build --locked --workspace --tests --keep-going` | rc 101: exactly 6 test targets fail to compile, all in `core` |
| 3 | every compilable target, no exclusions, `--no-fail-fast` | 658 passed, **14 failed**, 3 ignored |
| 4 | the same with the 14 skips | 658 passed, 0 failed, 3 ignored, 14 filtered out |
| 5 | the shard's scope, `cargo test --locked -p spikefade -- <14 skips>`, twice | 158 passed, 0 failed, 1 ignored, 9 filtered out (5.5 s, 5.9 s) |

Section 3 by target: `costs` 428 passed and 2 ignored, `greeks` 59 passed, `spikefade` 158 passed with 9 failed and
1 ignored, `core` 13 passed (`bound_probe` 2, `lint` 2, `zerodha_index` 5, doc tests 4) with 5 failed (`cost_invariants`).

## Exclusions

Every exclusion is a file this checkout or box does not hold. Each one was confirmed from the compiler error or the panic
message in `baseline.log`.

### Test targets that cannot compile (`include_str!` of app-repository files)

Never built in this shard: cargo-mutants builds and tests only `spikefade` here.

| `core` target | missing file(s) |
|---|---|
| lib unit tests (`src/**`) | `docs/00-charter.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `CLAUDE.md` |
| `tests/banned_lists.rs` | `.github/workflows/ci.yml`, `deny.toml`, `crates/vocab/tests/workspace_is_rust.rs` |
| `tests/claim_reader.rs` | `.github/workflows/ci.yml`, `.github/gates_bounds.rs` |
| `tests/decision_citations.rs` | `docs/05-decisions.md` |
| `tests/findings.rs` | `docs/11-findings.md` |
| `tests/graph.rs` | `docs/01-architecture.md`, `crates/<api,cli,engine,indicators,lake,pull,runner,store,telemetry,vocab>/Cargo.toml` |

### Tests skipped by exact name (they fail at run time)

| test | why (from the panic message) |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | reads `crates/spikefade/data/EXITS-DYN-*.tsv`: absent, so `menu_head: missing` |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv: No such file or directory` |
| `tests3::sf3_ttp_replaces_tp_close` | the same folder: `NotFound` |
| `tests34::root_digest_same_for_every_form` | runs `/opt/homebrew/bin/zstd`, the Mac default (`kit.rs:470`, PORTABLE off): `No such file or directory` |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (`core` `cost_invariants`) | `docs/04-invariants.md` must be readable: absent |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | `crates/spikefade/benches/ratio.rs` does not exist (the app repository's rule, applied to this kit's extra crate) |

- The first 7 are the 7 that README-CLOUD.md section 5 predicts. These are the same 14 names that shards 0, 1 and 3 used, in
  shard 3's order.
- In this shard only the 9 `spikefade` skips take effect. The 5 `core` names match no test that runs here; they are passed
  so that every shard uses one list.
- **Limit:** the 9 skipped `spikefade` tests ran against no mutant, so a mutant that only they would catch is counted here
  as missed.

## Files

- `outcomes.json` and `caught.txt`, `missed.txt`, `timeout.txt` (empty) and `unviable.txt`: cargo-mutants' output for slice
  26, unchanged.
- `baseline.log`: sections 1 to 5 above, each with its exact command, rc and wall time.
- `recheck-jobs1/`: the re-check's `outcomes.json` and lists (16 mutants: the 11 above plus the 5 out-of-slice field
  deletions).
