# Mutation testing: shard 5 of 30 (job MUT-5)

Labels follow README-CLOUD.md: MEASURED means a number from a named run on this box, and EXACT means a count. No figure here
is a trading result. Code only: no market data was read, fetched or written, and none of the 3 parameter tables was present.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. The README-CLOUD.md
  section 4 manifest check gives 146 files and sha256 `c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e`,
  as expected.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GiB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`,
  `cargo 1.97.1 (c980f4866 2026-06-30)`, and cargo-mutants 27.1.0 from `cargo install cargo-mutants --locked`.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 5/30 --jobs 2 --cargo-arg=--locked -- -- --exact --skip <each of the 14 tests under "Exclusions">
  ```

- **Wall (MEASURED):** 2026-11-16 11:36:32Z to 12:05:56Z, which is 29 min 24 s. cargo-mutants reports 29 min 22 s.
- **Exit code 2:** some mutants were missed.
- **Scope (EXACT):** 522 of the workspace's 15,639 mutants, list positions 2611 to 3132 (1-based) under the default
  `slice` sharding. All 522 are in `crates/spikefade/src/`:
  - `acct.rs`: its last 135 of 407;
  - `cal.rs`: all 237;
  - `charges.rs`: its first 150 of 386.
- **Tests per mutant:** `spikefade`'s own tests, exactly as cargo-mutants 27.1.0 runs them: 158 run, 1 ignored, 9 skipped.
  - cargo-mutants' own unmutated baseline passed in 11 s build + 3 s test.
  - It auto-set the test timeout to 20 s.

## Result (EXACT, from `outcomes.json`)

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| acct.rs | 16 | 119 | 0 | 0 | 135 |
| cal.rs | 212 | 25 | 0 | 0 | 237 |
| charges.rs | 51 | 97 | 0 | 2 | 150 |
| **all 3 files** | **279** | **241** | **0** | **2** | **522** |

## Read this before using the CAUGHT count: 4 catches are a timing flake

`split18::tests::v32rb_split_o1_gates` asserts wall-clock percentiles in a debug build: account p99 under 2 ms and close
p99 under 20 µs. With 2 jobs on 4 vCPU it failed at random. One log reads "97 months: account p99 4038115 ns".

It was the ONLY failing test for these 4 mutants:

```
crates/spikefade/src/acct.rs:570:9: replace Tops::export -> (usize, Vec<Vec<((i64, i64, i64), Sum)>>) with (1, vec![vec![((0, 0, 0), Default::default())]])
crates/spikefade/src/acct.rs:570:9: replace Tops::export -> (usize, Vec<Vec<((i64, i64, i64), Sum)>>) with (1, vec![vec![((-1, 1, -1), Default::default())]])
crates/spikefade/src/acct.rs:577:9: replace Tops::canonical -> (usize, Vec<Vec<((i64, i64, i64), Sum)>>) with (1, vec![vec![((-1, -1, 0), Default::default())]])
crates/spikefade/src/cal.rs:108:5: replace fmt_us -> String with String::new()
```

**Re-tested on this box (MEASURED).** Each mutant was run alone, with nothing else running, by applying cargo-mutants' own
recorded diff to a copy of the tree.

| run | result for each of the 4 |
|---|---|
| the run's exact per-mutant command, gate kept | passes, 158 passed |
| the same with `--skip split18::tests::v32rb_split_o1_gates` | passes, 157 passed |
| unmutated control | passes, 158 passed |

So no deterministic test catches these 4.
- **Effective counts:** 275 caught and 245 missed.
- **`outcomes.json` and the `.txt` lists are unchanged.** They still record these 4 as caught.

Why the follow-up did not use cargo-mutants itself: in 27.1.0 its 206 "delete field" mutants ignore both `-F` and `-E`.
So no regex selects exactly these 4.

## One out-of-memory kill, a real catch

`cal.rs:19:35: replace + with *` made a test grow to about 13 GiB (kernel log: anon-rss 13651200 kB). The kernel killed
that test binary (SIGKILL), and cargo-mutants scored the mutant caught.
- The kill hit only that binary.
- The mutant that ran beside it, `cal.rs:19:27: replace * with +`, failed on 20 assertion panics of its own, not on a kill.
- There were no other kills in the run.

## Baseline (`baseline.log`)

| run | passed | failed | ignored | filtered out | wall (MEASURED) |
|---|---:|---:|---:|---:|---:|
| the exact `cargo test --locked --workspace` | 0 | 0 | 0 | 0 | 2.2 s, rc 101 |
| every compilable target, no exclusions, `--no-fail-fast` | 658 | 14 | 3 | 0 | 5.7 s |
| the same with exactly the 14 exclusions, core's 4 doc tests not re-run | 654 | 0 | 3 | 14 | 5.5 s |
| this shard's scope: `spikefade` only, with the 14 exclusions | 158 | 0 | 1 | 9 | 3.6 s |

- **The doc tests in the third row:** core's 4 doc tests passed in the second row, and no exclusion names a doc test. So
  658 pass with the exclusions, but only 654 were run in that row.

- **Why the exact command ran 0 tests:** cargo stops at the first test target that fails to compile. Six `core` test
  targets cannot compile here: the lib unit tests and `banned_lists`, `claim_reader`, `decision_citations`, `findings` and
  `graph`.
  - Each one does `include_str!` of an app-repository file this branch does not hold: `docs/*.md`, `CLAUDE.md`,
    `.github/*`, `deny.toml`, or another crate's `Cargo.toml`.
  - None of the six is in this shard's scope, which is `spikefade` only.
- **How the walls add up:** "every compilable target" is 5.4 s for the other packages plus 0.1 s for core's 4 integration
  targets plus 0.3 s for core's doc tests.
- **The build is not in those walls.** It happened before them: in the exact command, then in
  `cargo build --locked --workspace --tests --keep-going` (10.9 s, rc 101, which lists the 6 targets above).

## Exclusions

These are the same 14 as shards 0 to 3. Each one was checked here against its own failure message.

| test | fails because it needs |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` and `-SETS-v1.tsv` (read with `unwrap_or_default`, so it fails as "menu_head: missing") |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | `crates/spikefade/data/EXITS-DYN-MENU-v1.tsv` |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | the Mac default `/opt/homebrew/bin/zstd` (`kit.rs:470`) |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core `cost_invariants`) | `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | `crates/spikefade/benches/ratio.rs` |

- The first 9 are `spikefade` tests, so they are the only exclusions that matter for this shard.
- The last 5 are `core` tests. They match no test in this shard's scope, and they are passed so that every shard uses one
  list.
