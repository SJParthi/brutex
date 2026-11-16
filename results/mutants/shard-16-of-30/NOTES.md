# Mutation testing: shard 16 of 30 (job MUT-16)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is
a trading result. Code only: no market data was read, fetched or written, and none of the 3 parameter tables is present.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`.
- **Box:** Linux x86_64 cloud container (`Linux 6.18.44-fc-v470`), 4 vCPU (Intel Xeon @ 2.10GHz), 15 GB RAM.
  `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`, cargo-mutants 27.1.0 (installed with
  `cargo install cargo-mutants --locked`).
- **Command,** run from the checkout root (the same 14 skips as shards 0-3, nothing else added):

  ```sh
  cargo mutants --workspace --shard 16/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
    --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed \
    --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry \
    --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count \
    --skip tests3::sf3_ttp_replaces_tp_close --skip tests34::root_digest_same_for_every_form \
    --skip tests34::sfz_roundtrip_all_rows --skip every_crate_carries_a_ratio_bench \
    --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names \
    --skip every_measured_cost_invariant_is_declared --skip the_reconciliation_is_reading_something
  ```

- **Wall (MEASURED):** 2026-11-16 11:44:40Z to 12:26:10Z, so 41 min 29 s. Exit code 2 (missed mutants, no timeout).
- **cargo-mutants' own baseline passed:** build 15.4 s, test 4.7 s (MEASURED). It set the test timeout to 24 s.

## What shard 16 covers (EXACT)

- `cargo mutants --workspace --list` gives 15,639 mutants, the same total as shards 0-3. Sharding is 0-indexed and sliced,
  so shard 16 is list positions 8352 to 8873 (0-based): 522 mutants.
- **All 522 are in `crates/spikefade/src/report3.rs`, lines 252 to 2010.** Each one was tested by `spikefade`'s own tests
  only: `cargo test --package=spikefade@0.1.0 --locked -- --exact --skip <the 14>`. That runs 158 tests, with 1 ignored
  (a measurement test) and 9 filtered out by the skips.

## Result

| classification | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| cargo-mutants, raw (`outcomes.json`, `caught.txt`, `missed.txt`, ...) | 158 | 349 | 0 | 15 | 522 |
| **after the timing-gate re-test (`adjusted/`)** | **125** | **382** | **0** | **15** | **522** |

The raw files are cargo-mutants' own output and are unedited. The adjusted lists move 33 mutants from caught to missed,
for the reason below.

## Read this before using the raw CAUGHT count: a wall-clock gate failed under load

`split18::tests::v32rb_split_o1_gates` (`crates/spikefade/src/split18.rs:2120`) times 300 accounts in the unoptimised
test build and asserts a close p99 under 20 us and an account p99 under 2 ms. Its result depends on CPU load:

- **Before the run (MEASURED, 20 runs of the per-mutant test command):** 0 of 8 failed idle, 0 of 6 with 2 `yes` CPU
  burners, 2 of 6 with 4 burners (account p99 4,040,299 ns and 4,058,427 ns). The baseline's section C also failed it
  once (4,089,301 ns), with nothing else running.
- **It cannot catch a `report3.rs` mutant by what it computes.** `split18.rs` names `report3` once, in a doc comment
  (line 1054), and calls nothing in it.
- **In the run (EXACT, from the per-mutant logs; see `caught-tests.txt`):** the gate failed in 45 of the 158 caught
  mutants. In 12 of them other tests failed too, so those are real catches. In the other **33 it was the only failing
  test**: these 33 were the suspects.
- **The operator's decision,** asked while the run was going: leave the run unchanged, flag the suspects, and re-test them
  at `--jobs 1`.
- **Re-test 1 (`retest-1/`, MEASURED 12:26:40Z to 12:39:50Z):** all **33 suspects were MISSED** (`gate-suspects.txt`).
  Every gate-only catch was the gate failing under load. No suspect stayed gate-only, so no second re-test was needed.
- **The gate failed once in the re-test too, at `--jobs 1`.** It caught `report3.rs:1837:124: delete field run from struct
  WinRes expression in stage_b_whole` alone, a mutant the main run had MISSED. That is the same false catch the other way
  round, so the main run's MISSED stands for it.
- **No other catching test is load-sensitive.** None of the other 15 tests that caught mutants reads a clock in its own
  body (`Instant`, `elapsed`, `Duration`; checked in each test body, helpers not traced). The one whose name mentions RSS, `tests33f::sf31_daymajor_streamed_equals_inmemory_peak_rss_printed`, asserts streamed days equal
  in-memory days, the load and release counts, and only `rss_peak_kb > 0`. Its 11 catches are in `Store::ensure`,
  `Store::release`, `Store::sample_rss` and `slice_ok`, which it calls.

**Effect:** without the re-test, 7 functions would look partly tested when no functional test catches any of their
mutants. `cmd_run3` had 13 raw catches, `stage_b_whole` 9, `prepare_segment_c2` 3, `stage_a_events_c` and `prepare_day`
2 each, and `wtie` and `prepare_segment` 1 each. All of them were gate-only.

## What MISSED means here

MISSED means "not caught by the 158 `spikefade` tests that run on this box". Of the 9 skipped `spikefade` tests, none
names a shard-16 function. That was checked by name, and indirect calls were not traced. A run with the 3 tables and zstd
present might still catch more.

## By function (EXACT; adjusted except the "raw caught" column)

| function in `report3.rs` | raw caught | caught | missed | unviable | total |
|---|---:|---:|---:|---:|---:|
| `cmd_run3` | 13 | 0 | 107 | 4 | 111 |
| `stage_b_whole` | 9 | 0 | 83 | 6 | 89 |
| `prepare_segment` | 1 | 0 | 32 | 0 | 32 |
| `prepare_segment_c2` | 3 | 0 | 32 | 0 | 32 |
| `wtie` | 1 | 0 | 27 | 0 | 27 |
| `stage_a_events_c` | 2 | 0 | 23 | 1 | 24 |
| `prepare_day` | 2 | 0 | 21 | 1 | 22 |
| `compare_skip` | 0 | 0 | 16 | 0 | 16 |
| `offer_windows` | 9 | 8 | 11 | 0 | 19 |
| `spike_of` | 0 | 0 | 5 | 0 | 5 |
| `Store::put` | 0 | 0 | 4 | 1 | 5 |
| `groups_of` | 0 | 0 | 3 | 0 | 3 |
| `Store::cube_only` | 0 | 0 | 2 | 0 | 2 |
| `Store::n` | 0 | 0 | 2 | 0 | 2 |
| `build_lists` | 9 | 9 | 2 | 0 | 11 |
| `build_lists_derived` | 11 | 11 | 2 | 0 | 13 |
| `compile` | 0 | 0 | 2 | 0 | 2 |
| `pf_src33` | 6 | 5 | 2 | 0 | 7 |
| `slice_ok` | 1 | 1 | 2 | 0 | 3 |
| `Store::release` | 5 | 5 | 1 | 0 | 6 |
| `stage_a_chunks33` | 15 | 15 | 1 | 0 | 16 |
| `stage_a_events` | 0 | 0 | 1 | 0 | 1 |
| `stage_a_whole` | 0 | 0 | 1 | 0 | 1 |
| `Layout33::idx` | 10 | 10 | 0 | 0 | 10 |
| `Layout33::per_list` | 6 | 6 | 0 | 0 | 6 |
| `Layout33::with_mask` | 5 | 5 | 0 | 1 | 6 |
| `Store::ensure` | 4 | 4 | 0 | 0 | 4 |
| `Store::sample_rss` | 1 | 1 | 0 | 1 | 2 |
| `Store::src_of` | 3 | 3 | 0 | 0 | 3 |
| `append_cfg_lists` | 3 | 3 | 0 | 0 | 3 |
| `day_classes33` | 5 | 5 | 0 | 0 | 5 |
| `day_items33` | 6 | 6 | 0 | 0 | 6 |
| `dd_grouped_pub` | 3 | 3 | 0 | 0 | 3 |
| `fp_v33` | 2 | 2 | 0 | 0 | 2 |
| `headline_gate` | 2 | 2 | 0 | 0 | 2 |
| `headline_gate_liq` | 2 | 2 | 0 | 0 | 2 |
| `headline_ok` | 19 | 19 | 0 | 0 | 19 |
| **all 37 functions** | **158** | **125** | **382** | **15** | **522** |

## Baseline (`baseline.log`)

| section | command | result (MEASURED) |
|---|---|---|
| 0 | `cargo test --locked --workspace` (the job's literal command) | rc 101 after 3 s, 0 tests ran: `core` test `claim_reader` cannot compile (`include_str!` of `.github/workflows/ci.yml` and `.github/gates_bounds.rs`, absent) |
| A | every compilable target outside `core`, no skips, `--no-fail-fast` | rc 101: costs 428 passed, 2 ignored; greeks 59 passed; spikefade 158 passed, **9 failed**, 1 ignored |
| B | `core`'s 4 compilable test targets, no skips | rc 101: `cost_invariants` **5 failed**; `bound_probe` 2, `lint` 2 and `zerodha_index` 5 passed |
| C | as A, with the 14 skips | rc 101: everything passed except **`split18::tests::v32rb_split_o1_gates`** (account p99 4,089,301 ns), the timing gate above |
| D | as B, with the 14 skips | rc 0 |
| E1, E2 | the exact per-mutant command for a `spikefade` mutant | rc 0 both times: 158 passed, 1 ignored, 9 filtered out |

`core`'s lib unit tests and its integration targets `banned_lists`, `claim_reader`, `decision_citations`, `findings` and
`graph` cannot compile here (`include_str!` of files of the app repository). They are never built for this shard, because
cargo-mutants 27.1.0 tests a mutant only with its own package, and every mutant here is in `spikefade`.

## Exclusions (the same 14 as shards 0-3, each one a file this branch or box does not hold)

| skipped test | package | why it fails here |
|---|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | spikefade | reads `crates/spikefade/data/EXITS-DYN-*.tsv` (absent, README-CLOUD.md section 5) |
| `menu::tests::sf3_menu_fail_closed` | spikefade | the same |
| `menu::tests::sf3_distance_floor_counted` | spikefade | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | spikefade | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | spikefade | the same |
| `tests33s::v33_search1d_sets_load_and_count` | spikefade | the same |
| `tests3::sf3_ttp_replaces_tp_close` | spikefade | the same |
| `tests34::root_digest_same_for_every_form` | spikefade | runs `/opt/homebrew/bin/zstd`, the Mac default (`kit.rs:470`); no zstd on this box, `SF_ZSTD` unset |
| `tests34::sfz_roundtrip_all_rows` | spikefade | the same |
| `every_declared_cost_invariant_reaches_a_bench` | core (`cost_invariants`) | reads `docs/04-invariants.md` (absent) |
| `the_document_declares_the_five_operations_rule_4_names` | core | the same |
| `every_measured_cost_invariant_is_declared` | core | the same |
| `the_reconciliation_is_reading_something` | core | the same |
| `every_crate_carries_a_ratio_bench` | core | needs `crates/spikefade/benches/ratio.rs` (absent) |

The 5 `core` skips match no `spikefade` test. They are passed so that every shard uses the same exclusion list.
**`split18::tests::v32rb_split_o1_gates` was NOT skipped**: it fails only under load, not because a file is missing.

## The re-test (`retest-1/`)

```sh
cargo mutants --workspace -f crates/spikefade/src/report3.rs -F "$(cat retest-1/selection-regex.txt)" \
  --jobs 1 --cargo-arg=--locked -o <dir> -- -- --exact <the same 14 skips>
```

- **cargo-mutants 27.1.0 does not apply `-F`/`-E` to "delete field ... from struct ..." mutants:** every one is selected
  whatever the regex says. `-f report3.rs` keeps those extras inside this file. The selection was checked before the run:
  every suspect was selected, and every extra was a `report3.rs` field deletion.
- So `retest-1/outcomes.json` holds **102 mutants**: the 33 suspects plus 69 field deletions.
  - 47 of the 69 are in shard 16. 46 were MISSED in both runs. The 47th is `1837:124`, described above.
  - 22 are outside shard 16 (`report3.rs` lines 2064 and up, the territory of later shards). One was caught
    (`3298:16 ... in twin10`, by `tests32::v32_wanted_picks_x_and_twins`) and 21 were missed. They are not counted here.
- cargo-mutants' baseline for the re-test passed: build 16.1 s, test 5.0 s. It set the test timeout to 25 s.

## Files in this folder

- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`: cargo-mutants' output for the run, unedited.
- `baseline.log`: sections 0 and A to E above.
- `caught-tests.txt`: each of the 158 raw catches, its class (`others`, `gate+others` or `gate-only`) and the tests that
  failed.
- `gate-suspects.txt`: the 33 gate-only catches and their re-test result.
- `retest-1/`: cargo-mutants' output for the re-test, unedited, plus the selection regex.
- `adjusted/`: shard 16's 522 mutants after the re-test, in source order: `caught.txt` 125, `missed.txt` 382,
  `timeout.txt` 0, `unviable.txt` 15.
