# Mutation shard 10/30: results (job MUT-10)

Code only. No market data was read, fetched or written. **EXACT** = a count. **MEASURED** = a wall time or rate taken on this box.

## Headline

| reading | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| as cargo-mutants reports it (`outcomes.json`, EXACT) | 211 | 304 | 0 | 7 | 522 |
| **functional tests only** (the 39 catches by a wall-clock gate alone moved to missed; see "Read this before using the CAUGHT count") | **172** | **343** | 0 | 7 | 522 |

- **Wall (MEASURED):** 39 min 1 s for the main run, from `start_time` 11:37:27Z to `end_time` 12:16:28Z in `outcomes.json`.
- The process exit code was not captured, because the run was detached with nohup.

## Source and tools

- Branch `cloud/spikefade-shard-20261116`, commit `15fd4f4`, a single root commit.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked`. It is the same version shards 0-3 report.
- The box: 4 vCPU, 15 GiB RAM, Linux x86_64. Nothing else was installed.

## What shard 10/30 covers

- `cargo mutants --workspace --list` gives **15,639** mutants (EXACT).
- `--shard 10/30` uses the default `slice` sharding, 0-indexed. It covers list positions **5221 to 5742** (1-based): **522 mutants**.
- **Every one is in `crates/spikefade/src/screen.rs`**, lines 37 to 849.
  - `screen.rs` holds 562 mutants. Its first 12 are in shard 9 and its last 28 in shard 11.
- cargo-mutants tests each mutant with its own package's tests. Here that is `spikefade`: the binary's unit tests, with 9 skipped by name and 1 ignored.
  - `spikefade` is a binary crate, and no other workspace crate depends on it.
  - So, unlike shards 0 and 1, **this shard's missed count measures `spikefade`'s own test suite**, less the 9 tests that cannot run here.

## Command (run from the checkout root)

```sh
cargo mutants --workspace --shard 10/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
  --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed \
  --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted \
  --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count \
  --skip tests3::sf3_ttp_replaces_tp_close --skip tests34::root_digest_same_for_every_form --skip tests34::sfz_roundtrip_all_rows \
  --skip every_declared_cost_invariant_reaches_a_bench --skip the_document_declares_the_five_operations_rule_4_names \
  --skip every_measured_cost_invariant_is_declared --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something
```

The 14 skips are the same list shards 0-3 use. The 5 `core` skips match no test that runs in this shard.

## Baseline and exclusions (details and raw output in `baseline.log`)

1. **The literal `cargo test --locked --workspace` does not compile** (rc 101, 5 s). Six `core` test targets `include_str!` files that exist only in the app repository:
   - the lib unit tests;
   - `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`.
2. **Every compilable target, with no exclusions** (`--no-fail-fast`): **658 passed, 14 failed, 3 ignored** (EXACT).
   - Each failure needs a file this box does not hold. The panic messages are in `baseline.log`.
   - 7 `spikefade` tests need `crates/spikefade/data/EXITS-DYN-*.tsv`.
   - 2 `spikefade` tests (`tests34::*`) exec `/opt/homebrew/bin/zstd`.
   - 4 `core` `cost_invariants` tests need `docs/04-invariants.md`.
   - 1 test, `every_crate_carries_a_ratio_bench`, needs `crates/spikefade/benches/ratio.rs`.
3. **With those 14 skipped:** **658 passed, 0 failed, 3 ignored, 14 filtered out** (EXACT).
4. **This shard's scope,** `cargo test --locked -p spikefade` with the 14 skips, run twice: **158 passed, 0 failed, 1 ignored, 9 filtered out** both times (EXACT).
5. **cargo-mutants' own baseline passed:** "Unmutated baseline in 15s build + 4s test". It auto-set the test timeout to **23 s**. There is no build timeout, which is cargo-mutants' default.

## Read this before using the CAUGHT count

**39 of the 211 catches come from a wall-clock gate alone, and that gate fails with no mutation at all.**

**The gate.** `split18::tests::v32rb_split_o1_gates` (`crates/spikefade/src/split18.rs:2120`) times its own calls with `Instant::now()`. It asserts:
- close p99 < 20 µs;
- account p99 < 2 ms;
- p50 at 97 months <= 3x p50 at 25 months + 2 µs.

**In the main run** it failed in 60 of the 522 mutant runs. In **39** of them it was the only failing test.

- 37 of the 39 failed on "account p99", at 2.07 to 5.34 ms against the 2 ms bound.
- The other 2 failed on the p50 ratio.

**The gate fails with no mutation** (MEASURED, `followup/gate-on-unmutated-tree.log`).
- The setup: two unmutated copies of the tree, run at the same time. Each repeated the per-mutant cycle 12 times: touch `screen.rs` (no content change), `cargo test --no-run`, then this shard's exact test command.
- The gate failed **2 of 24** runs: "25 months: account p99 4079415 ns" and "97 months: account p99 4046135 ns".
- No other test failed in any of the 24.
- That is about 8 %, against 60 of 522 (11 %) in the main run.

**The 39 with the gate skipped** (EXACT, `followup/`).
- The re-run is identical except for one more `--skip split18::tests::v32rb_split_o1_gates`. Its output was kept apart from `outcomes.json`.
- The result: **39 of 39 missed**.
- **Selection quirk:** `--re` named exactly these 39 (`followup/timing-only-39.txt`). cargo-mutants 27.1.0 also selected the 5 "delete field ... from struct DayEvents expression in generate" mutants at `screen.rs:503`, which `--re` and `--exclude-re` do not filter.
  - Their outcomes repeat the main run exactly: `503:30` caught, the other 4 missed.
  - So the follow-up tested 44: 43 missed and 1 caught.

**So, judged by the functional tests, this shard is 172 caught and 343 missed.** The 343 are the 304 in `missed.txt` plus the 39 in `followup/timing-only-39.txt`.

**This applies to every shard that holds `spikefade` mutants.** `spikefade` starts at list position 2300, so that is shards 4 to 29. Each runs the same gate, so a catch made by `v32rb_split_o1_gates` alone is suspect.

## Per function (`screen.rs`)

The columns:
- **caught** and **missed** are from `outcomes.json`.
- **gate only** is the part of "caught" that `v32rb_split_o1_gates` alone made.
- **func. caught** and **func. missed** are the counts with those moved to missed.

| function | first line | caught | gate only | func. caught | missed | func. missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `(no function: const item)` | 37 | 2 | 2 | 0 | 0 | 2 | 0 | 0 | 2 |
| `cfg_id` | 42 | 14 | 0 | 14 | 8 | 8 | 0 | 0 | 22 |
| `cfg_parts` | 48 | 8 | 8 | 0 | 76 | 84 | 0 | 0 | 84 |
| `thr_name` | 63 | 1 | 1 | 0 | 1 | 2 | 0 | 0 | 2 |
| `tf_name` | 69 | 0 | 0 | 0 | 2 | 2 | 0 | 0 | 2 |
| `lists` | 106 | 7 | 0 | 7 | 0 | 0 | 0 | 0 | 7 |
| `lists_all` | 112 | 20 | 3 | 17 | 25 | 28 | 0 | 1 | 46 |
| `stock_roll_facts` | 219 | 11 | 0 | 11 | 1 | 1 | 0 | 0 | 12 |
| `rank_key` | 236 | 5 | 1 | 4 | 2 | 3 | 0 | 0 | 7 |
| `window` | 242 | 31 | 0 | 31 | 2 | 2 | 0 | 0 | 33 |
| `member` | 250 | 29 | 0 | 29 | 4 | 4 | 0 | 0 | 33 |
| `prev_closes` | 288 | 1 | 1 | 0 | 2 | 3 | 0 | 5 | 8 |
| `member_prev` | 309 | 15 | 1 | 14 | 0 | 1 | 0 | 0 | 15 |
| `members` | 330 | 2 | 0 | 2 | 0 | 0 | 0 | 0 | 2 |
| `round2` | 350 | 2 | 2 | 0 | 12 | 14 | 0 | 0 | 14 |
| `quant` | 361 | 1 | 1 | 0 | 7 | 8 | 0 | 0 | 8 |
| `measure` | 372 | 0 | 0 | 0 | 2 | 2 | 0 | 0 | 2 |
| `splitmix` | 472 | 1 | 1 | 0 | 10 | 11 | 0 | 0 | 11 |
| `path_after` | 482 | 1 | 0 | 1 | 16 | 16 | 0 | 0 | 17 |
| `generate` | 502 | 55 | 13 | 42 | 95 | 108 | 0 | 1 | 151 |
| `rank_at` | 769 | 5 | 5 | 0 | 28 | 33 | 0 | 0 | 33 |
| `cmd_aggcheck` | 822 | 0 | 0 | 0 | 11 | 11 | 0 | 0 | 11 |
| **all** | | **211** | **39** | **172** | **304** | **343** | **0** | **7** | **522** |

### How to read the misses

**No functional catch at all:**
- the `NCFG` const expression (line 37);
- `cfg_parts` (84), `thr_name`, `tf_name`, `round2` (14), `quant` (8), `measure`, `splitmix` (11), `rank_at` (33) and `cmd_aggcheck` (11);
- `prev_closes`: 3 missed, and its other 5 mutants are unviable.

**Every "replace the whole body" mutant was missed** in `tf_name`, `measure`, `quant`, `splitmix`, `path_after` and `cmd_aggcheck`. No test that runs here tells their real result from a constant.

- `path_after`'s only catch is `483:16 + -> -`. Its tests panicked with "attempt to subtract with overflow".

**`cfg_parts`:** all 64 of its {0,1}^6 tuple replacements and all 20 of its `%` and `/` mutants survive the functional tests. `cfg_id` lost 8 of its 22. Pinning the round trip `cfg_parts(cfg_id(..))` would close most of these.

**`generate`** (108 missed of 151 by the functional count) and **`rank_at`** (33 of 33) hold most of the rest.

- `generate` covers lines 503-730: the event and counter arithmetic.
- `rank_at` covers lines 769-810: the rank lookup.

**`lists_all`** (28 of 46): its misses are boolean-guard flips (`&&`/`||`, `==`/`!=`, `delete !`) on lines 115-181.

**Open question:** the 9 skipped `spikefade` tests could not run here. Whether any of them would catch a mutant listed here is unknown.

- None of them is in a file that names `screen::`; only `tests32.rs`, `tests33.rs` and `tests33f.rs` do.
- Indirect reach was not checked.

### Unviable (7): the mutant does not compile

```
crates/spikefade/src/screen.rs:147:25: replace && with || in lists_all
crates/spikefade/src/screen.rs:288:5: replace prev_closes -> std::sync::Arc<Vec<i32>> with Arc::new(vec![])
crates/spikefade/src/screen.rs:288:5: replace prev_closes -> std::sync::Arc<Vec<i32>> with Arc::new(vec![0])
crates/spikefade/src/screen.rs:288:5: replace prev_closes -> std::sync::Arc<Vec<i32>> with Arc::new(vec![1])
crates/spikefade/src/screen.rs:288:5: replace prev_closes -> std::sync::Arc<Vec<i32>> with Arc::new(vec![-1])
crates/spikefade/src/screen.rs:290:9: replace && with || in prev_closes
crates/spikefade/src/screen.rs:640:57: replace && with || in generate
```

## Files in this folder

| file | what it holds |
|---|---|
| `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt` | the main run, copied verbatim from `mutants.out/` (the `.txt` lists are in completion order) |
| `baseline.log` | a summary header, then the raw output of every baseline section |
| `followup/timing-only-39.txt` | the 39 mutants that only `v32rb_split_o1_gates` caught |
| `followup/outcomes.json`, `followup/*.txt`, `followup/followup-console.log` | the re-run of those 39 with the gate skipped (44 tested: see the selection quirk above) |
| `followup/gate-on-unmutated-tree.log` | the gate on the unmutated tree under two emulated jobs: 2 failures in 24 runs |
| `SUMMARY.md` | this file |
