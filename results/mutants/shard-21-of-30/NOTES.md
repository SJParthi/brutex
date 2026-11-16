# Mutation testing: shard 21 of 30 (job MUT-21)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is
a trading result. Code only: no market data was read, fetched or written. None of the 3 parameter tables was present.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`.
  - The README-CLOUD.md section 4 source manifest gives 146 files and sha256
    `c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e`, as expected.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM.
  - Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`.
  - cargo-mutants 27.1.0, installed with `cargo install cargo-mutants --locked`. That is the latest release, and the one
    shards 0 to 3 used.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 21/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

- **Wall (MEASURED):** 2026-11-16 11:41:18Z to 12:12:35Z, so 31 min 17 s.
  - cargo-mutants' own unmutated baseline passed in 13 s build + 3 s test.
  - It auto-set the test timeout to 20 s.
- **Exit code 2:** some mutants were missed. None timed out.

## What shard 21/30 covers (EXACT)

- `cargo mutants --workspace --list` gives 15,639 mutants.
- Under 27.1.0's default `slice` sharding, `--shard 21/30` is list positions 10,963 to 11,484 (1-based). That is 522 mutants,
  all in `crates/spikefade/src/`.
- So each mutant was tested with spikefade's own tests:
  `cargo test --package=spikefade@0.1.0 --locked -- --exact <the 14 skips>`.

## Result (EXACT, from `outcomes.json`)

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| ban.rs | 15 | 8 | 0 | 1 | 24 |
| bhav.rs | 37 | 80 | 0 | 1 | 118 |
| expiry.rs | 83 | 141 | 0 | 1 | 225 |
| stage.rs | 1 | 2 | 0 | 60 | 63 |
| wf.rs | 78 | 13 | 0 | 1 | 92 |
| **all 5 files** | **214** | **244** | **0** | **64** | **522** |

- Every listed mutant has exactly one outcome. The list and the four `.txt` files agree, with no duplicates.
- 60 of the 64 unviable mutants are in `stage.rs`. They replace the return value of `trade3_once` (6) or `trade3_once_s`
  (54), and none of them compiles.

### Where the 244 missed mutants are

**No mutant caught** in these functions. The tests either never call them or never check what they return.

| file | function | missed |
|---|---|---:|
| expiry.rs | `is_session` | 14 |
| expiry.rs | `busiest` | 9 |
| expiry.rs | `dte_pass` | 5 |
| expiry.rs | `sessions_to` | 3 |
| expiry.rs | `not_staged_words` | 2 |
| bhav.rs | `expiries` | 10 |
| bhav.rs | `from_args` | 6 |
| bhav.rs | `Bhav::sessions` | 4 |
| ban.rs | `from_args` | 6 |
| stage.rs | `absent_word` | 2 |

**Almost untested:**

| file | function | missed | caught |
|---|---|---:|---:|
| expiry.rs | `facts` | 86 | 1 |
| bhav.rs | `prev_close` | 52 | 3 |
| expiry.rs | `staged` | 7 | 1 |

**Partly tested:**

| file | function | missed | caught |
|---|---|---:|---:|
| wf.rs | `learn_key` | 7 | 32 |
| wf.rs | `compose` | 6 | 20 |
| expiry.rs | `regular_and_moved` | 9 | 20 |
| expiry.rs | `bucket` | 2 | 4 |
| expiry.rs | `sessions_between` | 2 | 14 |
| expiry.rs | `dial_allowed` | 1 | 18 |
| expiry.rs | `weekday` | 1 | 3 |
| bhav.rs | `parse` | 5 | 6 |
| bhav.rs | `ymd_of` | 1 | 5 |
| bhav.rs | `Bhav::add_dir` | 1 | 2 |
| bhav.rs | `Bhav::day` | 1 | 1 |
| ban.rs | `Bans::add_dir` | 1 | 4 |
| ban.rs | `counts_line` | 1 | 1 |

### How to read the missed count

- **Unlike shards 0 and 1, this count measures the package's own test suite.** spikefade has 168 tests. For every mutant, the
  run used all of them except the 9 skipped below and 1 that the code marks `#[ignore]`.
- **None of the 9 skipped tests names a function of these 5 files directly.** This was checked with grep. Whether one of them
  reaches these files indirectly was not traced.

## Baseline (`baseline.log`)

The file holds the literal run first. After it comes one delimited section for each diagnostic run.

| section | command | result (EXACT) |
|---|---|---|
| (A) | `cargo test --locked --workspace`, as instructed | rc 101 after 2 s, and 0 tests ran: `core` test targets fail to compile |
| (B) | `cargo test --locked --workspace --exclude core --no-fail-fast` | costs: 428 passed, 2 ignored. greeks: 59 passed. spikefade: 158 passed, 9 failed, 1 ignored |
| (C) and (C2) | `cargo test --locked -p core --no-fail-fast`, then `cargo build --locked -p core --tests --keep-going` | 6 targets fail to compile: the lib unit tests, `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph` |
| (C3) and (C4) | core's 4 test targets that compile, then core's doc tests | 9 passed and 5 failed (`cost_invariants`); doc tests: 4 passed |
| (D) | `cargo test --locked --package=spikefade -- --exact <the 14 skips>`, the shape each mutant ran | 158 passed, 0 failed, 1 ignored, 9 filtered out |
| (E) | (B) with the 14 skips | 0 failed |
| (F) | core's 4 test targets that compile, with the 14 skips | 9 passed, 0 failed, 5 filtered out |

The 6 targets that fail to compile `include_str!` files of the app repository that this branch does not hold:
`CLAUDE.md`, `docs/*.md`, `.github/workflows/ci.yml`, `.github/gates_bounds.rs`, `deny.toml`, and other crates' files.

## Exclusions: 14 tests skipped by exact name, the same list as shards 0 to 3

| test | why |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | needs `crates/spikefade/data/EXITS-DYN-*.tsv` (README-CLOUD.md section 5) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs zstd at the Mac default `/opt/homebrew/bin/zstd`. This box has no zstd, and `SF_ZSTD` is unset |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core) | needs `docs/04-invariants.md` |
| `every_measured_cost_invariant_is_declared` (core) | the same |
| `the_document_declares_the_five_operations_rule_4_names` (core) | the same |
| `the_reconciliation_is_reading_something` (core) | the same |
| `every_crate_carries_a_ratio_bench` (core) | needs `crates/spikefade/benches/ratio.rs`, which this branch does not have |

- **The 5 core names match no spikefade test.** They are passed so that every shard uses the same list.
- **No target-level exclusion was needed.** This shard's test command builds only spikefade's tests, so `core`'s 6 targets
  that fail to compile are never built.
- **Judgement call, the same one shards 0 to 3 made:** the 2 zstd tests need a file that is missing from the box, the zstd
  binary, rather than one missing from the repository.
