# Mutation testing: shard 19 of 30 (job MUT-19)

Labels follow README-CLOUD.md: MEASURED is a number from a named run on this box, and EXACT is a count. No figure here is a
trading result. Code only: no market data was read, fetched or written, and none of the 3 parameter tables was present.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`, its single root commit.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GiB RAM.
  - `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, both pinned by `rust-toolchain.toml`.
  - cargo-mutants 27.1.0, from `cargo install cargo-mutants --locked`. That was the newest on crates.io and is the version
    shards 0-3 used.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 19/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

- **cargo-mutants' own baseline:** ok, 14 s build and 5 s test (MEASURED). It auto-set the test timeout to 27 s.
- **Wall (MEASURED):** 2026-11-16 11:42:47Z to 12:26:40Z, so 2,633 s (43 min 53 s).
- **Exit code 2:** missed mutants were found.

## What shard 19 covers (EXACT)

- `cargo mutants --workspace --list` gives **15,639** mutants, the same total shards 0 and 1 report.
- In 27.1.0, shards are 0-indexed contiguous slices (default `--sharding slice`). `--shard 19/30` is list positions 9,918 to
  10,439 (0-based), **522 mutants**.
- All 522 are in `crates/spikefade/src/report3.rs` lines 4594 to 4646, in four functions:

| function (`report3.rs`) | defined at | mutants |
|---|---:|---:|
| `ma_tf` | 4593 | 3 |
| `set_tfdep` | 4602 | 1 |
| `dedup_ratio` | 4608 | 13 |
| `speed_probe` | 4645 | 505, all replacements of its 9-tuple return value; the other 36 of its 541 mutants fall in shard 20 |

## Result (EXACT, from `outcomes.json`, unedited)

| function | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| `ma_tf` | 0 | 3 | 0 | 0 | 3 |
| `set_tfdep` | 1 | 0 | 0 | 0 | 1 |
| `dedup_ratio` | 2 | 11 | 0 | 0 | 13 |
| `speed_probe` | 43 | 462 | 0 | 0 | 505 |
| **all** | **46** | **476** | **0** | **0** | **522** |

## Read this before using the CAUGHT count: the 46 are timing flakes

**Every one of the 46 was caught by the same single test, and that test is a wall-clock gate:**
`split18::tests::v32rb_split_o1_gates` (`split18.rs:2120`).
- Each of the 46 logs reads `157 passed; 1 failed; 1 ignored; 9 filtered out`. The one failure is that test, at
  `split18.rs:2150:13`.
- `split18.rs:2150` asserts that one account's split figures cost under 2 ms at p99. The 46 failures measured a p99 of 3.92
  to 4.10 ms: 20 at "25 months" and 26 at "97 months".
- The values cluster just above 4 ms. That looks like the timed thread losing one scheduler time slice while the other job
  builds or tests, with 2 jobs on 4 vCPU. This is an interpretation, not a measurement.
- The test passed in every run outside the 2-job phase: the three spikefade runs in `baseline.log`, and cargo-mutants' two
  baselines.

**None of the 4 mutated functions runs under any test.**
- `speed_probe` runs only under `run3 --speed-probe K` (`report3.rs:1007-1010`).
- `dedup_ratio` runs only under `--dedup-ratio` (`report3.rs:1021-1022`).
- `set_tfdep` runs only under `--hand` or `--hand-only` (`report3.rs:1057-1059`). So does `ma_tf`, through `hand_sample`
  (`report3.rs:4514`).
- No test in `crates/spikefade/src` passes any of these flags.
- **Independent check:** branch `cloud-results/coverage-engine` holds `lcov.info`. There, `report3.rs` lines 4590 to 4700
  have 81 instrumented lines and **0 executed**, and the function records of `speed_probe`, `dedup_ratio` and `hand_sample`
  all read 0.

**Follow-up (MEASURED):** the same 46 mutants were re-tested with `--jobs 1`, everything else equal.
- **Result: 46 of 46 MISSED, 0 caught.**
- Its baseline was ok: 16 s build and 5 s test, with the test timeout auto-set to 28 s.
- Wall: 12:28:48Z to 12:34:55Z, so 367 s.
- Its `outcomes.json` and `.txt` files are in `followup-caught-46-jobs1/`. They are **not** in the top-level `outcomes.json`,
  which is the main run unedited.

**So the true result of shard 19 is 522 of 522 MISSED. No test executes these four functions.**
- `missed.txt` lists 476 of them.
- The other 46 are `caught.txt`, which is identical to `followup-caught-46-jobs1/missed.txt`.

### A finding for every other spikefade shard

- **The gate's flake rate here:** `v32rb_split_o1_gates` failed for 46 of the 522 mutants (8.8%). Each time it failed on
  its p99 timing assertion alone, and no other test failed.
- **Any spikefade shard is exposed:** a shard run at `--jobs 2` on a 4-vCPU box can report false catches the same way.
- **What to do:** treat a CAUGHT whose only failing test is this one as suspect, and re-test it at `--jobs 1`.

### How the 46 were selected (a 27.1.0 quirk)

- **Regexes alone over-selected.** One anchored, escaped `-F` regex per line of `caught.txt` also selected 206 unrelated
  `delete field ... from struct ... expression in ...` mutants. In 27.1.0 neither `-F` nor `-E` filters that mutant kind.
- **Adding `--in-diff` fixed it.** It took a diff that marks `report3.rs` lines 4603, 4618, 4625 and 4646 as changed:
  - the diff appends ` // rerun-select` to those 4 lines on the old side, so the new side is the tree as it stands;
  - with it, `--list` gave exactly the 46, a set identical to `caught.txt`.
- **Command:**

  ```sh
  cargo mutants --workspace --in-diff select.diff \
    -F '^crates/spikefade/src/report3\.rs:4603:5: replace set_tfdep with \(\)$' -F '<one per line of caught.txt, 46 in all>' \
    --jobs 1 --cargo-arg=--locked -o <outside the checkout> -- -- --exact --skip <the same 14>
  ```

## Baseline (`baseline.log`)

### As instructed: `cargo test --locked --workspace`

- **Exit 101 after 8 s (MEASURED), and 0 tests ran.** `core`'s lib test target cannot compile.
- **Why:** it `include_str!`s files of the app repository that this branch does not hold: `docs/00-charter.md`,
  `docs/05-decisions.md` (twice), `docs/06-limits.md` and `CLAUDE.md`.

### Every target that compiles, `--no-fail-fast`, no exclusions (MEASURED)

| package | passed | failed | ignored |
|---|---:|---:|---:|
| core: `bound_probe`, `cost_invariants`, `lint`, `zerodha_index` and 4 doc tests | 13 | 5 | 0 |
| costs | 428 | 0 | 2 |
| greeks | 59 | 0 | 0 |
| spikefade | 158 | 9 | 1 |

### With the exclusions: 0 failed everywhere (MEASURED)

- **The same targets:** 658 passed, 0 failed, 3 ignored, 14 filtered out.
- **spikefade:** 158 passed, 1 ignored, 9 filtered out, in about 6 s.
- **The exact per-mutant command,** `cargo test --package=spikefade@0.1.0 --locked -- --exact --skip <x14>`, gave the same:
  158 passed, 1 ignored, 9 filtered out, in 5.4 s.

## Exclusions: the same 14 test names as shards 0-3

- **Only the 9 spikefade names act in this shard.** cargo-mutants tests every mutant here with `spikefade`'s own tests,
  which is its default.
- **The 5 core names match no spikefade test.** They are passed so that every shard carries the same list.
- **No `--test` target selection was needed,** because `core` is not tested in this shard.
- **What core cannot compile here:** its lib unit tests, `banned_lists`, `claim_reader`, `decision_citations`, `findings` and
  `graph`. Shard 0's NOTES.md lists the files each one lacks.

| test | why it fails here |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | needs `crates/spikefade/data/EXITS-DYN-*.tsv`, absent from the repo (README-CLOUD.md section 5) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs zstd at the Mac default `/opt/homebrew/bin/zstd` (see below) |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core `cost_invariants`) | needs `docs/04-invariants.md`, absent from the repo |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | needs `crates/spikefade/benches/ratio.rs`, absent from the repo |

**The two `tests34` exclusions are not missing repo files.**
- **Cause:** both tests call `all_off()`, which turns PORTABLE off, so `kit::tool("zstd")` returns the Mac path
  `/opt/homebrew/bin/zstd` (`kit.rs:470`). Neither `SF_ZSTD` nor PATH is consulted.
- **Status:** they fail on any Linux box, so this is a portability finding.
- **Why they are excluded anyway:** to match shards 0-3.
- **Why that changes nothing here:** neither test reaches any shard-19 function. They call slice, cube and kit code only.

## Files

| file | what it holds |
|---|---|
| `outcomes.json` and `caught.txt` (46), `missed.txt` (476), `timeout.txt` (0), `unviable.txt` (0) | the main run, unedited |
| `baseline.log` | the literal step-2 command, then every compilable target without and with the exclusions, then the exact per-mutant command |
| `followup-caught-46-jobs1/` | the `--jobs 1` re-test of the 46: its `outcomes.json` and `.txt` files |
| `NOTES.md` | this file |
