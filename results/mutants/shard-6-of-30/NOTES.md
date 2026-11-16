# Mutation testing: shard 6 of 30 (job MUT-6)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. Code only: no market
  data, and none of the 3 parameter tables.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`. cargo-mutants
  27.1.0, from `cargo install cargo-mutants --locked`: the same version as shards 0-3.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 6/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

- **Wall (MEASURED):** 2026-11-16 11:36:01Z to 12:11:41Z, so 35 min 40.5 s.
- **cargo-mutants' own baseline (MEASURED):** 14.9 s build and 3.8 s test. The auto-set test timeout was 20 s.
- **Exit code 3:** at least one mutant timed out. In 27.1.0 a timeout outranks missed mutants when the exit code is chosen.

## Result (EXACT)

Shard 6 is mutants 3,133 to 3,654 (1-based, in `--list` order) of the workspace's 15,639, under the default `slice`
sharding. All 522 are in `crates/spikefade`.

As cargo-mutants reported them (`caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`, `outcomes.json`):

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| charges.rs | 131 | 105 | 0 | 0 | 236 |
| csvgz.rs | 110 | 18 | 7 | 1 | 136 |
| cube.rs | 54 | 95 | 0 | 1 | 150 |
| **all 3 files** | **295** | **218** | **7** | **2** | **522** |

After the re-check below, 17 caught verdicts turn out to be false, and they are missed:

| file | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| charges.rs | 124 | 112 | 0 | 0 | 236 |
| csvgz.rs | 109 | 19 | 7 | 1 | 136 |
| cube.rs | 45 | 104 | 0 | 1 | 150 |
| **all 3 files** | **278** | **235** | **7** | **2** | **522** |

## Read this before using the CAUGHT count: one timing test catches mutants on CPU load alone

- 17 of the 295 CAUGHT verdicts rest on one failing test alone: `split18::tests::v32rb_split_o1_gates`.
  - That test asserts wall-clock bounds: `close` p99 under 20,000 ns, `account` p99 under 2,000,000 ns, and the 97-month p50
    at most 3 times the 25-month p50 plus 2,000 ns.
  - With 2 jobs on 4 vCPU it failed whenever the box was busy. One log reads `97 months: account p99 4043094 ns`.
- 3 of the 17 replace the body of `charges::charged`, which rustc reports as never used (`charges.rs:131`). No test can catch
  them for a real reason.
- **Re-check (MEASURED, `recheck-jobs1/`):** those 17 and the 7 timeouts were run again with `--jobs 1`, with the same
  `--locked` and the same 14 exclusions.
  - Selection: one `--re` per exact mutant name, plus `--file` for the 3 files. `--list` showed exactly those 24.
  - Result: 0 caught, 17 missed, 7 timeout. Wall 4 min 45 s (12:15:54Z to 12:20:39Z).
  - The timing test failed in none of the 24 runs.
- The files cargo-mutants wrote for the shard are left exactly as written. The corrected count is this note plus
  `recheck-jobs1/`.
- 18 more CAUGHT verdicts had the timing test fail together with at least one other test. They stand on the other test.
- **For the other shards:** every shard whose mutants are in `spikefade` runs this test under `--jobs 2`. A caught mutant whose
  only failing test is `split18::tests::v32rb_split_o1_gates` should be re-checked the same way.

## Where the 235 missed mutants are (EXACT, after the re-check)

| function | caught | missed | timeout | unviable | note |
|---|---:|---:|---:|---:|---|
| `charges.rs` `charged` | 0 | 65 | 0 | 0 | dead code: rustc warns "function `charged` is never used" |
| `charges.rs` `charges_est` | 4 | 11 | 0 | 0 | its only non-test caller is `charged`; `tests33g.rs:114-115` calls it |
| `charges.rs` `charges_est_sell` | 59 | 32 | 0 | 0 | live: `trade.rs:583` |
| `charges.rs` `charged_sell` | 61 | 4 | 0 | 0 | live: `trade.rs:588`, `:597` |
| `csvgz.rs` `GzLines::next_line`, `GzLines::finish` | 0 | 6 | 0 | 0 | |
| `csvgz.rs` `ts` | 41 | 7 | 0 | 0 | |
| `csvgz.rs` `split` | 22 | 5 | 6 | 0 | |
| `csvgz.rs` `paisa` | 34 | 1 | 1 | 0 | |
| `cube.rs` `build`, `is_current`, `size_mtime`, `inputs`, `read_instruments`, `check_header` | 0 | 99 | 0 | 1 | no test calls `build` or `is_current`; their only caller is `main.rs:122`, `:126` |
| `cube.rs` `Cube::wval`, `Cube::allowed`, `Cube::guard`, `sha256` | 4 | 5 | 0 | 0 | |

Every other function in the shard has no missed mutant.

## Do the exclusions hide catches in this shard? (read from the code, not run)

- 7 of the 9 excluded `spikefade` tests parse the menu and sets tables.
- The other 2 (`tests34::sfz_roundtrip_all_rows`, `tests34::root_digest_same_for_every_form`) build synthetic cubes in memory
  (`synth::day`, `cube::encode_with`) and compress them through zstd.
- None of the 9 calls into `charges.rs` or `csvgz.rs`, nor `cube::build` or `cube::is_current`. So the exclusions are not
  expected to change this shard's counts.

## Baseline

### As instructed: `cargo test --locked --workspace` (`baseline.log`)

- Exit 101 after 2.13 s (MEASURED), and 0 tests ran.
- Cargo stops at the first targets that fail to compile: `core` test `graph` and `core`'s lib unit tests. Both `include_str!`
  files of the app repository that this branch does not hold.
- Adding `--no-fail-fast` does not get past a target that fails to compile: exit 101 after 13.8 s (MEASURED), stopped at
  `core` test `claim_reader`.

### Every target on its own, with `--no-fail-fast` (MEASURED, `baseline-per-target.log`)

| package | passed | failed | ignored | targets that fail to compile |
|---|---:|---:|---:|---|
| core | 13 | 5 | 0 | 6: the lib unit tests, banned_lists, claim_reader, decision_citations, findings, graph |
| costs | 428 | 0 | 2 | none |
| greeks | 59 | 0 | 0 | none |
| spikefade | 158 | 9 | 1 | none |

- The core row covers `bound_probe` 2, `lint` 2, `zerodha_index` 5 and 4 doc tests. All 5 failures are in `cost_invariants`.
- These are the same counts as shard 0's.

### With the exclusions (MEASURED)

- `cargo test --locked -p spikefade -- --exact --skip <the 14>`: 158 passed, 0 failed, 1 ignored, 9 filtered out.
  The tests took 3.97 s, 4.04 s wall.
- cargo-mutants' own baseline ran the shard command and passed.

## Exclusions

The same 14 names as shards 0-3, passed to the test binary as `--exact --skip`. Every reason is the failure message in
`baseline-per-target.log`.

| test | why it fails here |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | reads `crates/spikefade/data/EXITS-DYN-*.tsv`, absent (`menu_head: missing`) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | reads `data/EXITS-DYN-MENU-v1.tsv`, absent |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs zstd at the Mac default `/opt/homebrew/bin/zstd` (`kit.rs:470`); absent, `SF_ZSTD` unset |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core `cost_invariants`) | reads `docs/04-invariants.md`, absent |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | wants `crates/spikefade/benches/ratio.rs`, absent |

- The 5 `core` names cannot match a `spikefade` test under `--exact`, so they change nothing here. They are passed so that
  every shard uses the same list.
- **Not passed here: shard 0's `--cargo-arg=--test=...` selection of `core` test targets.**
  - Every shard 6 mutant is in `spikefade`, and cargo-mutants 27.1.0 tests only the mutated package. `core`'s test targets
    are never built.
  - Passing that selection fails every run (MEASURED): `error: no test target named 'bound_probe' in 'spikefade' package`.

## Also noticed

- In cargo-mutants 27.1.0, `--re` and `--exclude-re` do not filter the workspace's 206 `delete field ... from struct ...
  expression` mutants.
  - With any `--re`, all 206 are listed as well. None of them is in shard 6.
  - This is why the re-check also selects by `--file`.

## Files

- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`: the shard run, as cargo-mutants wrote them.
- `baseline.log`: the instructed `cargo test --locked --workspace`.
- `baseline-per-target.log`: each test target on its own.
- `recheck-jobs1/`: the 24-mutant re-check at `--jobs 1`.
