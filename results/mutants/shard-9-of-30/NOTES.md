# Mutation testing: shard 9 of 30 (job MUT-9)

Labels follow README-CLOUD.md: **MEASURED** means a number from a named run on this box, and **EXACT** means a count. No
figure here is a trading result. This was a code-only run: no market data was read, fetched or written.

## Read this first: 35 of the 74 catches are false

- **The gate.** `split18::tests::v32rb_split_o1_gates` checks wall-clock time (`crates/spikefade/src/split18.rs:2149-2150`):
  - the p99 of one close must stay under 20 µs;
  - the p99 of one account finish must stay under 2 ms.
- **How load breaks it.** With `--jobs 2`, this box runs one mutant's tests while it compiles the next one, and the gate
  then fails from load alone. It failed in **42 of the 521** test phases of the main run (EXACT).
- **The false catches.** For **35** caught mutants, the gate was the **only** failing test: 34 in `report.rs` and 1 in
  `screen.rs`. They are listed in `timing-only-catches.txt`. `split18.rs` names none of the 10 `report.rs` functions
  involved.
- **The recheck.** Those 35 were re-tested one at a time (`--jobs 1`, with nothing else running), using the same
  toolchain, flags and 14 skips.
  - **All 35 were MISSED.**
  - The gate failed in 0 of the 48 recheck runs (EXACT).
- **Corrected result:** **39 caught and 482 missed**, listed in `missed-after-recheck.txt`.
  - cargo-mutants recorded 74 caught and 447 missed.
  - `outcomes.json` and the `.txt` files at this folder's top level are left exactly as cargo-mutants wrote them.
- **This affects other shards.** Every `spikefade` shard run with `--jobs 2` is exposed to the same false catches. Check
  each caught mutant's failing tests in `mutants.out/log/` before trusting the caught count.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM.
- **Tools:**
  - `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`;
  - cargo-mutants 27.1.0, installed with `cargo install cargo-mutants --locked`. It was the newest on crates.io, and it
    is the version shards 0 to 3 report.
- **Command,** run from the checkout root. This is the form shard 3 used:

  ```sh
  cargo mutants --workspace --shard 9/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests under "Exclusions">
  ```

- **Per mutant:** cargo-mutants' default per-package testing, which here is
  `cargo test --package=spikefade@0.1.0 --locked -- --exact <the 14 skips>`.
- **cargo-mutants' own baseline passed:**
  - 15 s build and 5 s test;
  - 158 passed, 0 failed, 1 ignored, 9 filtered out;
  - it auto-set the test timeout to 29 s.
- **Wall time (MEASURED):**
  - main run: 2026-11-16 11:39:17Z to 12:26:23Z, so 47 min 5 s;
  - recheck: 12:28:00Z to 12:34:35Z, so 6 min 35 s.
- **Exit code:** the nohup wrapper did not record the main run's. The recheck exited 2.

## What shard 9 covers (EXACT)

- `cargo mutants --workspace --list` gives 15,639 mutants: core 642, costs 1,312, greeks 345 and spikefade 13,340. These
  are the same totals shards 0 to 3 report.
- Under the default `slice` sharding, shard 9/30 is list positions 4,698 to 5,219 (0-indexed). That is 522 mutants, all in
  `crates/spikefade/src/`:
  - `report.rs`: 467, from `peak_margin` (line 856) to the end of the file;
  - `rules.rs`: 43;
  - `screen.rs`: 12, all on line 37, `pub const NCFG: usize = 10 * 4 * 3 * 4 * 4 * NTHR;`.

## Result (EXACT)

| file | recorded caught | recorded missed | caught after recheck | missed after recheck | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|---:|---:|
| `report.rs` | 34 | 432 | 0 | 466 | 0 | 1 | 467 |
| `rules.rs` | 35 | 8 | 35 | 8 | 0 | 0 | 43 |
| `screen.rs` | 5 | 7 | 4 | 8 | 0 | 0 | 12 |
| **all** | **74** | **447** | **39** | **482** | **0** | **1** | **522** |

- **`report.rs`:** after the recheck, 0 of its 466 viable mutants are caught.
  - The one unviable mutant does not compile: `report.rs:1189:25: replace && with || in write_results`.
- **The 39 real catches:**
  - all 35 in `rules.rs` fail `rules::tests::weekdays_and_holiday_shifted_expiries`;
  - all 4 in `screen.rs` fail `tests32::v32_topk_by_volume_5_10_20`;
  - in 7 of the 39, the timing gate also failed, alongside a functional test.

### Missed in `rules.rs` and `screen.rs` after the recheck (16)

```
crates/spikefade/src/rules.rs:36:9: replace Rules::name -> &'static str with ""
crates/spikefade/src/rules.rs:36:9: replace Rules::name -> &'static str with "xyzzy"
crates/spikefade/src/rules.rs:83:9: replace Calendar::load -> Result<Self, String> with Ok(Default::default())
crates/spikefade/src/rules.rs:86:35: replace || with && in Calendar::load
crates/spikefade/src/rules.rs:134:21: replace + with - in Calendar::expiry_facts
crates/spikefade/src/rules.rs:134:21: replace + with * in Calendar::expiry_facts
crates/spikefade/src/rules.rs:147:9: replace Calendar::expiry_words -> String with String::new()
crates/spikefade/src/rules.rs:147:9: replace Calendar::expiry_words -> String with "xyzzy".into()
crates/spikefade/src/screen.rs:37:44: replace * with +
crates/spikefade/src/screen.rs:37:44: replace * with /
crates/spikefade/src/screen.rs:37:40: replace * with +      <- recorded CAUGHT (timing gate only)
crates/spikefade/src/screen.rs:37:40: replace * with /
crates/spikefade/src/screen.rs:37:36: replace * with +
crates/spikefade/src/screen.rs:37:36: replace * with /
crates/spikefade/src/screen.rs:37:32: replace * with +
crates/spikefade/src/screen.rs:37:32: replace * with /
```

The other 466 are in `report.rs`. Every one is listed in `missed-after-recheck.txt`.

## Baseline (`baseline.log`)

1. **Step 2 as instructed, `cargo test --locked --workspace`:** rc 101 after 4 s (MEASURED), and 0 tests ran.
   - `core`'s lib unit tests and its integration test `banned_lists` do not compile.
   - They `include_str!` files of the app repository that this branch does not hold.
2. **Section 2, `cargo build --locked --workspace --tests --keep-going`:** exactly 6 test targets fail to compile. All 6
   are in `core`, and all fail on `include_str!` of an absent file:

   | `core` target | missing file(s) |
   |---|---|
   | lib unit tests | `docs/00-charter.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `CLAUDE.md` |
   | `banned_lists` | `.github/workflows/ci.yml`, `deny.toml`, `crates/vocab/tests/workspace_is_rust.rs` |
   | `claim_reader` | `.github/workflows/ci.yml`, `.github/gates_bounds.rs` |
   | `decision_citations` | `docs/05-decisions.md` |
   | `findings` | `docs/11-findings.md` |
   | `graph` | `docs/01-architecture.md`, and the `Cargo.toml` of `api`, `cli`, `engine`, `indicators`, `lake`, `pull`, `runner`, `store`, `telemetry` and `vocab` |

3. **Every compilable target with no exclusions** (sections 3A-quiet, 3B and 3C; MEASURED):

   | package | passed | failed | ignored |
   |---|---:|---:|---:|
   | core (`bound_probe`, `cost_invariants`, `lint`, `zerodha_index`, doc tests) | 13 | 5 | 0 |
   | costs | 428 | 0 | 2 |
   | greeks | 59 | 0 | 0 |
   | spikefade | 158 | 9 | 1 |

   Every one of the 14 failures needs something this box does not have (see "Exclusions").
4. **Sections 4 and 4b, the shard-9 scope with the 14 exclusions:**
   `cargo test --locked -p spikefade -- --exact <the 14 skips>` gave 158 passed, 0 failed, 1 ignored and 9 filtered out,
   twice.
5. **One extra failure, caused by this job and not by the tree:**
   - In section 3A, `split18::tests::v32rb_split_o1_gates` also failed: "97 months: account p99 4068593 ns", against a
     limit of 2,000,000 ns.
   - Section 3A ran while this job's own `cargo install cargo-mutants` was compiling on the same 4 cores.
   - It passed when re-run on a quiet box (section 3A-quiet), in sections 4 and 4b, and in both of cargo-mutants'
     baselines.
   - It is **not** excluded. The first section of this file covers what it did to the run.

## Exclusions: the same 14 as shards 0, 1 and 3

| test | why it cannot pass on this box |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | reads `crates/spikefade/data/EXITS-DYN-*.tsv`, which this branch leaves out on purpose (README-CLOUD.md section 2) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs zstd at the Mac default `/opt/homebrew/bin/zstd` (`kit.rs:470`): "No such file or directory" |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core, `cost_invariants`) | reads `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | needs `crates/spikefade/benches/ratio.rs`, which this branch does not have |

- The first 7 are the 7 that README-CLOUD.md section 5 predicts.
- The 5 `core` tests never run in this shard, because cargo-mutants tests spikefade's mutants with spikefade's tests only.
  They are passed so that every shard carries one list.
- **3 of the 14 are not missing repository files:**
  - the 2 zstd tests need a tool at a Mac path;
  - `every_crate_carries_a_ratio_bench` needs a file that this branch never had.

  Shards 0, 1 and 3 made the same three calls.

## Files

- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`: the main run, exactly as cargo-mutants
  wrote them.
- `baseline.log`: step 2's literal command, then sections 2 to 4b and 3A-quiet. Each section records its own start, end, rc
  and wall time.
- `timing-only-catches.txt`: the 35 mutants caught only by the timing gate.
- `missed-after-recheck.txt`: the 482 missed after the recheck, which is `missed.txt` plus those 35, in list order.
- `recheck/`: the recheck's own `outcomes.json` and `.txt` files.
  - cargo-mutants 27.1.0 applies neither `-F` nor `-E` to "delete field ... from struct ... expression" mutants. It
    applies only `-f`.
  - So the recheck selected `-f crates/spikefade/src/report.rs -f crates/spikefade/src/screen.rs` plus the 35 names as
    anchored, escaped `-F` patterns.
  - It therefore tested 48 mutants: the 35, and 13 other field-deletion mutants of those two files.
  - Those 13 are not used here. 12 were missed. 1 was caught by functional tests: `screen.rs:503:30`, which is outside
    shard 9.
