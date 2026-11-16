# Mutation testing: shard 8 of 30 (job MUT-8)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. Code only: no market
  data, and none of the 3 parameter tables.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`,
  cargo-mutants 27.1.0 (the version shards 0 to 3 used).
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 8/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

- **Wall (MEASURED):** 2026-11-16 11:39:10Z to 12:21:16Z, so 42 min 6 s (2,526 s).
- **cargo-mutants' own unmutated baseline (MEASURED):** 13 s build + 5 s test. It auto-set the test timeout to 26 s.
- **Exit code 2:** missed mutants were found. There were no timeouts.

## Result (EXACT)

Shard 8 is mutants 4,177 to 4,698 (1-based) of the workspace's 15,639 under the default `slice` sharding, 522 per shard.
Every one of them is in `crates/spikefade`.

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| report.rs | 42 | 285 | 0 | 2 | 329 |
| stress.rs | 5 | 103 | 0 | 0 | 108 |
| sw.rs | 12 | 69 | 0 | 4 | 85 |
| **all 3 files** | **59** | **457** | **0** | **6** | **522** |

**Missed, by function (EXACT):**

| file | function | missed |
|---|---|---:|
| report.rs | `cmd_run` | 126 |
| report.rs | `pick_detail` | 97 |
| report.rs | `bench_windows` | 23 |
| report.rs | `peak_margin` | 11 |
| report.rs | `r2` | 9 |
| report.rs | `rep_of` | 6 |
| report.rs | `prev_closes` | 4 |
| report.rs | `cfg_words`, `exit_words`, `now_ist` | 2 each |
| report.rs | `level_samples`, `measure_refs`, `settings_words` | 1 each |
| stress.rs | `dist` | 83 |
| stress.rs | `adverse_bp` | 20 |
| sw.rs | `from_args` | 53 |
| sw.rs | `words`, `words32`, `words_ra` | 3 each |
| sw.rs | `any`, `any_ra`, `ngrid_words` | 2 each |
| sw.rs | `any32` | 1 |

- `stress.rs:76` `dist` returns a 4-tuple of `i64`. cargo-mutants generates all 81 tuples with each element in {0, 1, -1}.
  Only `(0, 0, 0, 0)` and `(0, -1, 0, 0)` were caught; the other 79 are missed.
- Every mutant is listed by outcome in `caught.txt`, `missed.txt`, `timeout.txt` (empty) and `unviable.txt`.

## Read this before using the MISSED count

- **Which tests ran.** cargo-mutants 27.1.0 tests each mutant only against its own package's tests, here `spikefade`'s.
  Of its 168 tests, 158 ran, 1 is ignored (README-CLOUD.md section 5 calls it a measurement test) and 9 are excluded.
  So **457 missed means "not caught by those 158 tests"**.
- **The 9 excluded tests could not run here,** so whether they would catch any of the 457 is NOT measured.
  - No direct path was found. None of the 9 test bodies, and none of the helpers they call (`tmp`, `all_off`,
    `synth_roots`), names `sw::`, `stress::` or `report::`.
  - Neither do the modules they call into: `menu`, `slice`, `cube`, `kit` and `synth`.
  - `tests34`'s `all_off` helper calls `kit::set`, not `sw::set`.
  - A transitive path is not ruled out. Modules one level further down (`trade`, `kern`, `report3`, `acct`, `split18`,
    `ckpt`, `wf`) do name `sw::` or `stress::`, and those paths were not traced.

## Baseline

### As instructed: `cargo test --locked --workspace` (top of `baseline.log`)

- Exit 101 after 2.67 s (MEASURED), and 0 tests ran.
- Cargo stops at the first targets that fail to compile:
  - `core` lib test: `include_str!` of `docs/05-decisions.md`, `docs/06-limits.md`, `docs/00-charter.md` and
    `CLAUDE.md`;
  - `core` test `findings`: `include_str!` of `docs/11-findings.md`.
- None of those files is on this branch.

### Every target that compiles, with `--no-fail-fast` (labelled sections A, B0, B1 and B2 in `baseline.log`, MEASURED)

| package | passed | failed | ignored | targets that fail to compile |
|---|---:|---:|---:|---|
| core | 13 | 5 | 0 | 6: the lib unit tests, banned_lists, claim_reader, decision_citations, findings, graph |
| costs | 428 | 0 | 2 | none |
| greeks | 59 | 0 | 0 | none |
| spikefade | 158 | 9 | 1 | none |

- The core row covers `bound_probe`, `cost_invariants`, `lint`, `zerodha_index` and 4 doc tests.
- Section A, every package but core, took 20.6 s including the build.
- The whole baseline driver ran from 11:37:45Z to 11:38:16Z.

### With the exclusions (section C in `baseline.log`, MEASURED)

`cargo test --locked --package spikefade -- --exact --skip <the 14>` is the test command cargo-mutants runs for this shard.
- **Result:** 158 passed, 0 failed, 1 ignored, 9 filtered out.
- **Time:** 5.24 s of tests, 5.3 s wall.

## Exclusions

These are the same 14 names as shards 0, 2 and 3. Each one was re-checked on this box: it fails only because a file this
branch does not hold is missing.

| test | package | why it fails here (panic message on this box) |
|---|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | spikefade | `menu_head: missing`: `files()` reads `data/EXITS-DYN-MENU-v1.tsv` and `-SETS-v1.tsv` with `unwrap_or_default()` |
| `menu::tests::sf3_menu_fail_closed` | spikefade | the same |
| `menu::tests::sf3_distance_floor_counted` | spikefade | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | spikefade | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | spikefade | the same |
| `tests33s::v33_search1d_sets_load_and_count` | spikefade | `menu .../crates/spikefade/data/EXITS-DYN-MENU-v1.tsv: No such file or directory` |
| `tests3::sf3_ttp_replaces_tp_close` | spikefade | `NotFound` reading `data/EXITS-DYN-MENU-v1.tsv` (`tests3.rs:121`) |
| `tests34::root_digest_same_for_every_form` | spikefade | `zstd (/opt/homebrew/bin/zstd) ...: No such file or directory` (no zstd on this box, `SF_ZSTD` unset) |
| `tests34::sfz_roundtrip_all_rows` | spikefade | the same |
| `every_declared_cost_invariant_reaches_a_bench` | core (`cost_invariants`) | `docs/04-invariants.md must be readable` |
| `every_measured_cost_invariant_is_declared` | core | the same |
| `the_document_declares_the_five_operations_rule_4_names` | core | the same |
| `the_reconciliation_is_reading_something` | core | the same |
| `every_crate_carries_a_ratio_bench` | core | `crates/spikefade` has no `benches/ratio.rs` (shard 0's judgement call, kept) |

- **The 5 core names change nothing for this shard,** which tests `spikefade` only. They are passed so that every shard
  uses the same exclusion list.
- **Shard 0's `--cargo-arg=--test=...` target selection is not used.** It names `core` test targets, so it would make
  `cargo test --package=spikefade` refuse with "no test target named ...".
  - It is not needed here, because a `spikefade`-only run never builds `core`'s test targets.
  - Shards 2 and 3, which tested `costs` and `greeks`, ran without it as well.
