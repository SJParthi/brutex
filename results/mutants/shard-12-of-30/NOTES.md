# Mutation testing: shard 12 of 30 (job MUT-12)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. Code only: no market
  data, and none of the 3 parameter tables.
- **Box:** a Linux x86_64 cloud container with 4 vCPU (Intel Xeon @ 2.10GHz) and 15.7 GiB RAM.
  `rustc 1.97.1 (8bab26f4f 2026-07-14)`, cargo-mutants 27.1.0 (the crates.io latest, and the version shards 0-3 used).
  Nothing else was added to the box or to the checkout.
- **Command,** run from the checkout root with `nohup`:

  ```sh
  cargo mutants --workspace --shard 12/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
    --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed \
    --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry \
    --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count \
    --skip tests3::sf3_ttp_replaces_tp_close --skip tests34::root_digest_same_for_every_form \
    --skip tests34::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench \
    --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared \
    --skip the_reconciliation_is_reading_something --skip every_crate_carries_a_ratio_bench
  ```

- **Wall (MEASURED, cargo-mutants' own timestamps in `outcomes.json`):** 2026-11-16 11:38:16Z to 12:16:23Z, so 38 min 7 s.
- **cargo-mutants' baseline (MEASURED):** 14.1 s build + 4.2 s test, passed. Auto-set test timeout: 22 s.
- **Exit code:** not captured, because the run was detached with `nohup`. The counts below come from `outcomes.json`.

## Result (EXACT)

Shard 12 is mutants 6,265 to 6,786 (1-based) of the workspace's 15,639 under the default `slice` sharding (522 per shard).
Every one of them is in `crates/spikefade`. The workspace total matches shard 0's count: core 642, costs 1,312, greeks 345
and spikefade 13,340.

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| trade.rs | 171 | 225 | 0 | 8 | 404 |
| v2.rs | 9 | 107 | 0 | 2 | 118 |
| **both** | **180** | **332** | **0** | **10** | **522** |

Of the 512 viable mutants, 180 were caught (35.2%).

### By function

| file | function | first line | caught | missed | timeout | unviable | total |
|---|---|---:|---:|---:|---:|---:|---:|
| trade.rs | `charged_trip` | 584 | 1 | 0 | 0 | 0 | 1 |
| trade.rs | `charges_dearer_of` | 594 | 7 | 2 | 0 | 0 | 9 |
| trade.rs | `settle_charges` | 606 | 5 | 0 | 0 | 0 | 5 |
| trade.rs | `buy_orders` | 625 | 23 | 2 | 0 | 0 | 25 |
| trade.rs | `order_end` | 635 | 5 | 0 | 0 | 0 | 5 |
| trade.rs | `never_closed_value` | 643 | 19 | 4 | 0 | 0 | 23 |
| trade.rs | `record_base` | 685 | 1 | 1 | 0 | 2 | 4 |
| trade.rs | `buy_path` | 717 | 5 | 0 | 0 | 0 | 5 |
| trade.rs | `truncate` | 731 | 6 | 3 | 0 | 0 | 9 |
| trade.rs | `anchors` | 748 | 4 | 0 | 0 | 0 | 4 |
| trade.rs | `charges_of` | 766 | 5 | 49 | 0 | 0 | 54 |
| trade.rs | `charges_orders` | 775 | 19 | 41 | 0 | 0 | 60 |
| trade.rs | `trip` | 784 | 21 | 19 | 0 | 0 | 40 |
| trade.rs | `out_of` | 871 | 0 | 34 | 0 | 0 | 34 |
| trade.rs | `out_of31` | 902 | 0 | 5 | 0 | 0 | 5 |
| trade.rs | `margin` | 916 | 13 | 4 | 0 | 0 | 17 |
| trade.rs | `size_event` | 938 | 1 | 0 | 0 | 0 | 1 |
| trade.rs | `size_event_mx` | 946 | 26 | 18 | 0 | 2 | 46 |
| trade.rs | `price_event` | 1036 | 0 | 21 | 0 | 2 | 23 |
| trade.rs | `trade_once` | 1115 | 9 | 3 | 0 | 1 | 13 |
| trade.rs | `walk_rec` | 1157 | 1 | 12 | 0 | 0 | 13 |
| trade.rs | `evidence` | 1180 | 0 | 7 | 0 | 1 | 8 |
| v2.rs | `parse_settings` | 39 | 4 | 41 | 0 | 0 | 45 |
| v2.rs | `read_picks` | 89 | 0 | 2 | 0 | 0 | 2 |
| v2.rs | `eval_entries` | 108 | 1 | 0 | 0 | 0 | 1 |
| v2.rs | `eval_entries_full` | 125 | 0 | 1 | 0 | 1 | 2 |
| v2.rs | `summarize` | 155 | 0 | 8 | 0 | 0 | 8 |
| v2.rs | `interval_peak` | 175 | 0 | 12 | 0 | 0 | 12 |
| v2.rs | `r1_overlaps` | 193 | 2 | 15 | 0 | 0 | 17 |
| v2.rs | `why_words` | 223 | 1 | 12 | 0 | 1 | 14 |
| v2.rs | `expiry_class` | 242 | 0 | 10 | 0 | 0 | 10 |
| v2.rs | `raw_1m` | 270 | 1 | 6 | 0 | 0 | 7 |
| **both** | **32 functions** | | **180** | **332** | **0** | **10** | **522** |

In 8 functions not one mutant was caught: `out_of` (34 missed), `price_event` (21), `interval_peak` (12),
`expiry_class` (10), `summarize` (8), `evidence` (7), `out_of31` (5) and `read_picks` (2).

## Read this before using the MISSED count

- cargo-mutants 27.1.0 tests each mutant only against its own package's tests, which here means `spikefade`'s unit tests:
  158 run, 1 ignored, 9 skipped by name. They are the only tests behind the 180 caught and the 332 missed.
- **The 9 skipped `spikefade` tests do not appear to reach this shard's code.** This was read from the code, not run:
  - the 5 `menu::tests` call only `menu.rs` (`rule_at`, `parse`, `parse_setting`, `row`, `files`), and the only `trade.rs`
    names they reach are constants, which cargo-mutants does not mutate;
  - `tests3::sf3_ttp_replaces_tp_close` parses the menu and calls `menu::rule_at`;
  - `tests33s::v33_search1d_sets_load_and_count` loads and counts the setting sets;
  - the 2 `tests34` tests round-trip cube slices through zstd.
  The functions with the most missed mutants are called from `report.rs` (the v2 report path) and `stage.rs`. So the
  exclusions are not expected to inflate this shard's MISSED count. That is a judgement from reading, not a measurement.
- 0 timeouts: no mutant ran the suite past the 22 s limit.

## Baseline (full record in `baseline.log`)

| step | command | rc | result (EXACT) | wall (MEASURED) |
|---|---|---:|---|---:|
| 1 | `cargo test --locked --workspace` (the literal command) | 101 | compile failure in `core` test targets; 0 tests ran | 3.6 s |
| 2 | every target that compiles, `--no-fail-fast`, no exclusions | 101 | 658 passed, 14 failed, 3 ignored | 28 s |
| 3 | the same with the 14 skips | 0 | 658 passed, 0 failed, 3 ignored, 14 filtered out | 6.7 s |
| 3d, 3e | `cargo test --package=spikefade@0.1.0 --locked -- --exact --skip <the 14>` (the per-mutant command), twice | 0 | 158 passed, 0 failed, 1 ignored, 9 filtered out | 4.4 s, 4.2 s |

Every one of the 14 failures in step 2 was read from its panic message, and each needs a file that is not here. No other
test failed.

## Exclusions

These are the same 14 by-name skips shards 0-3 used, so every shard runs with one exclusion list.

### Tests that fail at run time (skipped by exact name)

| test | why |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | needs `crates/spikefade/data/EXITS-DYN-*.tsv`, left out by design (README-CLOUD.md section 5) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs zstd at the Mac default `/opt/homebrew/bin/zstd` (`kit.rs:470`). The tests switch PORTABLE off, so neither `SF_ZSTD` nor `PATH` is read, and that path does not exist on this box |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core `cost_invariants`) | needs `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | needs `crates/spikefade/benches/ratio.rs` (the app repository's every-crate-has-a-ratio-bench rule; this branch adds `spikefade` without one) |

The 5 `core` skips change nothing on this shard, which never tests `core`. They are passed so that every shard uses one list.

### Test targets that cannot compile here

`core`'s lib unit tests and its integration tests `banned_lists`, `claim_reader`, `decision_citations`, `findings` and
`graph` `include_str!` files of the app repository that this branch does not hold. Shard 0 excluded them through `--test`
target selection. **Shard 12 does not pass that selection:** every one of its mutants is in `spikefade`, cargo-mutants never
builds `core`'s tests for them, and `--test=<core target>` would make `cargo test -p spikefade` fail.

## Files

- `baseline.log`: a header stating each baseline step, then the full output of every baseline command in order.
- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt` (empty) and `unviable.txt`: cargo-mutants' own output,
  unchanged.
