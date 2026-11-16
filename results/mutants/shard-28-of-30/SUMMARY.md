# Mutation shard 28/30: results (job MUT-28)

Code only. No market data was read, fetched or written. Labels follow README-CLOUD.md: **EXACT** = a count, **MEASURED** = a
number taken on this box. No figure here is a trading result.

## Source, box and tools

- Branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`, a single root commit.
- The box: a Linux x86_64 cloud container with 4 vCPU and 15.7 GiB RAM (MemTotal 16,473,776 kB).
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
  - All 523 build logs (522 mutants plus the baseline) and all 517 test logs ran
    `/root/.rustup/toolchains/1.97.1-x86_64-unknown-linux-gnu/bin/cargo` (EXACT).
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked`. This is the same version shards 0 to 3 used.

## What shard 28/30 covers

- `cargo mutants --workspace --list` gives **15,639** mutants (EXACT): core 642, costs 1,312, greeks 345, spikefade 13,340.
- With the default `slice` sharding, `--shard 28/30` is list positions **14616..15137** (0-based): **522 mutants, all in
  `crates/spikefade`** (EXACT). The slice equals those lines of the full `--list`, checked with `diff`.
  - `ckmerge.rs`: 64 mutants, from `cmd_ckpt_merge` at line 209 to the end of the file.
  - `kit.rs`: 378 mutants, the whole file.
  - `slice.rs`: 80 mutants, up to `get_var` at line 174. The boundary splits `slice.rs:174:14`.
- cargo-mutants 27.1.0 tests each mutant with its own package's tests:
  `cargo test --package=spikefade@0.1.0 --locked -- <the skips>`.
  - So the 6 `core` test targets that cannot compile on this branch are never built in this shard.

## The run (canonical: `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`)

```sh
cargo mutants --workspace --shard 28/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
  --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed \
  --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted \
  --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count \
  --skip tests3::sf3_ttp_replaces_tp_close --skip tests34::root_digest_same_for_every_form \
  --skip tests34::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench \
  --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared \
  --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something
```

- **Wall (MEASURED):** 2026-11-16 11:41:37Z to 12:34:00Z, 52 min 23 s. cargo-mutants reports "522 mutants tested in 52m".
- **cargo-mutants' own unmutated baseline:** ok, 15 s build plus 5 s test. It set the test timeout to 29 s.
- **Exit code 3:** at least one mutant timed out. In 27.1.0 a timeout outranks missed mutants when the exit code is chosen.

### Result (EXACT, from `outcomes.json`)

**522 tested: 275 caught, 226 missed, 15 timeout, 6 unviable.**

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| ckmerge.rs (line 209 to the end) | 3 | 59 | 0 | 2 | 64 |
| kit.rs (all of it) | 250 | 109 | 15 | 4 | 378 |
| slice.rs (up to line 174) | 22 | 58 | 0 | 0 | 80 |
| **all 3 files** | **275** | **226** | **15** | **6** | **522** |

## Two things the canonical counts get wrong on this box, both measured

### 1. 23 "caught" rest on a wall-clock gate that fails under `--jobs 2`

- `split18::tests::v32rb_split_o1_gates` (`split18.rs:2120-2154`) times `close` and `account` with `Instant::now()` in a debug
  build. It asserts p99 < 20,000 ns and p99 < 2,000,000 ns respectively.
- **On the unmutated binary (MEASURED):**
  - one run at a time: 8 of 8 runs pass;
  - two runs at a time, which is what `--jobs 2` does: 4 of 16 runs fail, on that test alone.
  - `timing-gate-check.log` holds the second check: 1 of 8 failed, with `97 months: account p99 4052738 ns`. The first check
    (3 of 8) was not saved.
- **In the main run:**
  - the gate's FAILED line appears in 59 of the 517 test logs: 56 of the 275 caught runs and 3 of the 15 timeouts;
  - for **23 of the 275 catches it was the only failing test**, and every other test passed. `caught-only-by-timing-gate.txt`
    lists them. They are load noise, not catches. The 3 `ckmerge.rs` catches are all among them.
- **This affects every spikefade shard,** because cargo-mutants runs spikefade's tests for a spikefade mutant. A CAUGHT whose
  log shows only `split18::tests::v32rb_split_o1_gates ... FAILED` is not a catch. Shards 0 to 3 (core, costs, greeks) do not
  run spikefade's tests, so they are not affected.

### 2. 52 of the 226 "missed" are caught once the two zstd tests can run

- `tests34::sfz_roundtrip_all_rows` and `tests34::root_digest_same_for_every_form` are in the shared skip list because they
  exec `/opt/homebrew/bin/zstd`.
  - Both call `all_off()`, which turns PORTABLE off, so `kit::tool("zstd")` returns the Mac path and ignores `SF_ZSTD` and
    PATH (`kit.rs:461-470`).
- They reach the `slice.rs` code this shard mutates:
  - `sfz_encode` calls `put_var` and `zstd_c`, which calls `zstd_pipe`;
  - `sfz_decode` calls `magic_of`, `zstd_d` and `get_var`;
  - `root_day_digest` calls `slice_bytes`, which calls `magic_of`.
- Run with them (F1 below), **52 of the 226 are caught, all in `slice.rs`, every one by one or both of these two tests**.
  `missed-caught-by-zstd-tests.txt` lists them.

### Corrected result (EXACT, from the main run plus F1 to F3)

This counts catches by functional tests only, with the two zstd tests able to run (as on the Mac) and the timing gate not
counted.

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| ckmerge.rs | 0 | 62 | 0 | 2 | 64 |
| kit.rs | 233 | 126 | 15 | 4 | 378 |
| slice.rs | 73 | 6 | 1 | 0 | 80 |
| **all 3 files** | **306** | **194** | **16** | **6** | **522** |

**How the 522 move:**

| main-run group | count | after the follow-ups |
|---|---:|---|
| caught, with at least one functional test failing | 252 | caught |
| caught by the timing gate alone (F2) | 23 | 20 missed, 2 caught by the zstd tests, 1 timeout |
| missed (F1) | 226 | 52 caught by the zstd tests; 155 missed; 19 caught by the gate alone, which F3 measured as missed |
| timeout | 15 | timeout |
| unviable | 6 | unviable |

- `missed-after-followups.txt` lists the 194 corrected misses, in list order.
- `timeout-after-followups.txt` lists the 16 timeouts. The new one is `slice.rs:157:13: replace >= with < in put_var`: the
  varint loop never ends once the zstd tests run.

### Missed mutants by function

| file | function | missed, main run | missed, corrected |
|---|---|---:|---:|
| ckmerge.rs | `cmd_ckpt_merge` | 32 | **33** |
| ckmerge.rs | `canon` | 11 | **11** |
| ckmerge.rs | `cmd_ckpt_canon` | 4 | **6** |
| ckmerge.rs | `cmd_bck_check` | 4 | **4** |
| ckmerge.rs | `ident` | 8 | **8** |
| kit.rs | `any` | 1 | **1** |
| kit.rs | `words` | 2 | **2** |
| kit.rs | `args_digest` | 4 | **4** |
| kit.rs | `from_args` | 20 | **21** |
| kit.rs | `shard_walks` | 1 | **1** |
| kit.rs | `parse_set` | 2 | **2** |
| kit.rs | `Sha256::block` | 2 | **3** |
| kit.rs | `Sha256::update` | 1 | **1** |
| kit.rs | `tool` | 3 | **3** |
| kit.rs | `utc_now` | 2 | **2** |
| kit.rs | `now_ist` | 3 | **3** |
| kit.rs | `rss_kb` | 2 | **3** |
| kit.rs | `proc_cpu_s` | 5 | **7** |
| kit.rs | `strict_fail` | 2 | **2** |
| kit.rs | `strict_check` | 5 | **5** |
| kit.rs | `probe_tools` | 9 | **10** |
| kit.rs | `Plan::load` | 1 | **1** |
| kit.rs | `Plan::parse` | 2 | **3** |
| kit.rs | `Plan::check` | 2 | **4** |
| kit.rs | `fp_part` | 4 | **4** |
| kit.rs | `par_digest` | 1 | **2** |
| kit.rs | `list_digest` | 1 | **1** |
| kit.rs | `table_lines` | 8 | **11** |
| kit.rs | `inputs_manifest` | 2 | **2** |
| kit.rs | `check_inputs` | 4 | **4** |
| kit.rs | `Bck::parse` | 11 | **12** |
| kit.rs | `exe_sha` | 1 | **2** |
| kit.rs | `host` | 2 | **3** |
| kit.rs | `source_sha` | 2 | **2** |
| kit.rs | `provenance` | 4 | **5** |
| slice.rs | `magic_of` | 3 | **0** |
| slice.rs | `slice_bytes` | 18 | **2** |
| slice.rs | `root_day_digest` | 9 | **1** |
| slice.rs | `zstd_pipe` | 4 | **0** |
| slice.rs | `zstd_c` | 2 | **0** |
| slice.rs | `zstd_d` | 3 | **0** |
| slice.rs | `put_var` | 6 | **1** |
| slice.rs | `get_var` | 13 | **2** |
| **total** | | **226** | **194** |

## Other observations

**Timeouts.** The 15 timeouts of the main run all look like real hangs, not box noise. Each one hit the 29 s limit while the
baseline test phase took 5 to 8 s:

- `args_digest` and `Sha256::update`: their loop steps are changed (`+=` to `-=`/`*=`, a flipped bound or condition), or
  `update`/`update_pad` is replaced with `()`.
- `Plan::batch_at` returning `Ok((0, 1))` or `Ok((1, 1))`.
- `par_digest`'s `>=` turned into `<` (`kit.rs:831`). This is the worker loop's exit test, so some workers stop at once and
  the rest never stop.

**Two out-of-memory kills, both of a mutant's own test process.**

- The kernel's cgroup OOM killer killed the spikefade test binary twice. Each had about 9 GB of anonymous RSS, allocated
  from a thread named `tests34::batch_*`.
- The two mutants were `kit.rs:799:9: replace Plan::batch_at -> Result<(usize, usize), String> with Ok((0, 0))` and
  `with Ok((1, 0))`. They finished at 12:14:26Z and 12:14:57Z.
- `tests34::plan_roundtrip_and_refusals` failed in both runs before the kill, so both are real catches.
- No other process was killed. The other job's mutants at that time were the `Ok((0, 1))` and `Ok((1, 1))` siblings, both
  TIMEOUT. No OOM kill happened in F1 to F3.

**The other 7 skipped spikefade tests** read the 3 parameter tables. They are `menu.rs` and `tests3`/`tests33s` tests. Their
bodies make no direct call into `ckmerge`, `kit` or `slice`. Indirect reach was not traced.

**A cargo-mutants 27.1.0 quirk** met in the follow-ups: `-F/--re` and `-E/--exclude-re` do not filter
`delete field ... from struct ... expression` mutants.

- So every follow-up selection also held the two such mutants of these files: `ckmerge.rs:41:26` (shard 27's) and
  `kit.rs:707:29` (caught in the main run).
- Both are left out of every figure above.
- The main run used positional sharding with no name filter, so it is not affected.

## Follow-up runs F1 to F3 (box-specific; not in the canonical `outcomes.json`)

**Box change for all three:** `apt-get install -y zstd` (zstd 1.5.5), then `/opt/homebrew/bin/zstd` made a symlink to
`/usr/bin/zstd`. Both are outside the checkout, and nothing in the tree changed.

- On the unmutated tree, the two zstd tests then pass.
- spikefade with the other 12 skips gives 160 passed, 0 failed, 1 ignored, 7 filtered out.

Every run used `--workspace -f crates/spikefade/src/{ckmerge,kit,slice}.rs -F '^(?:<the names, regex-escaped>)$' --jobs 2
--cargo-arg=--locked`. Each run's `--list` was diffed against the intended names first.

| run | what | skips | baseline | wall (MEASURED) | result for the intended names (EXACT) | folder |
|---|---|---|---|---|---|---|
| F1 | the 226 main-run misses | the 12 that are not zstd tests | ok, 17 s + 6 s, timeout 31 s | 12:39:17Z to 13:02:49Z, 23 min 32 s | 52 caught by the zstd tests, 19 caught by the timing gate alone, 155 missed | `followup-1-missed-with-zstd/` |
| F2 | the 23 main-run gate-only catches | the same 12, plus `split18::tests::v32rb_split_o1_gates` | ok, 17 s + 6 s, timeout 31 s | 13:05:51Z to 13:09:29Z, 3 min 38 s | 2 caught (`slice.rs:103:5`, `slice.rs:143:5`, by the zstd tests), 1 timeout (`slice.rs:157:13`), 20 missed | `followup-2-gate-only-main/` |
| F3 | F1's 19 gate-only catches | the same as F2 | ok, 16 s + 5 s, timeout 30 s | 13:09:58Z to 13:12:38Z, 2 min 40 s | 19 missed | `followup-3-gate-only-f1/` |

## Baseline and exclusions (details in `baseline.log`)

1. **The literal `cargo test --locked --workspace` does not compile:** rc 101 after 3 s, and 0 tests ran.
2. **`cargo build --locked --workspace --tests --keep-going`:** exactly 6 test targets fail to compile, all in `core`.
   - The targets: the lib unit tests and the integration tests `banned_lists`, `claim_reader`, `decision_citations`,
     `findings` and `graph`.
   - Each fails on `include_str!` of an app-repository file this branch does not hold: `docs/*.md`, `CLAUDE.md`,
     `.github/*`, `deny.toml`, or ten other crates' `Cargo.toml`.
   - They are excluded as whole targets. In this shard they are never built anyway.
3. **Every compilable target, no exclusions, `--no-fail-fast`:** 658 passed, 14 failed, 3 ignored (EXACT). Every failure
   needs a file that is not here. These 14 are skipped by exact name:

| test | package | why it fails here |
|---|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | spikefade | reads `crates/spikefade/data/EXITS-DYN-*.tsv` (absent by design, README-CLOUD.md section 5) |
| `menu::tests::sf3_menu_fail_closed` | spikefade | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | spikefade | the same |
| `menu::tests::sf3_distance_floor_counted` | spikefade | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | spikefade | the same |
| `tests33s::v33_search1d_sets_load_and_count` | spikefade | the same |
| `tests3::sf3_ttp_replaces_tp_close` | spikefade | the same |
| `tests34::root_digest_same_for_every_form` | spikefade | runs `/opt/homebrew/bin/zstd`, the Mac tool path. It was not on this box for the main run, and the test turns PORTABLE off |
| `tests34::sfz_roundtrip_all_rows` | spikefade | the same |
| `every_declared_cost_invariant_reaches_a_bench` | core (`cost_invariants`) | reads `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | core (`cost_invariants`) | the same |
| `every_measured_cost_invariant_is_declared` | core (`cost_invariants`) | the same |
| `the_reconciliation_is_reading_something` | core (`cost_invariants`) | the same |
| `every_crate_carries_a_ratio_bench` | core (`cost_invariants`) | wants `crates/spikefade/benches/ratio.rs`, which this branch does not hold |

4. **The same, with the 14 skips:** 658 passed, 0 failed, 3 ignored, 14 filtered out (EXACT).
5. **The scope this shard tests,** `cargo test --locked -p spikefade` with the skips: 158 passed, 0 failed, 1 ignored,
   9 filtered out (EXACT), in a 6 s wall (MEASURED).

The list is the same 14 names shards 0 to 3 used. The 5 `core` names match no test that runs in this shard. They are passed
so that every shard carries one list.

## Files

- **Canonical cargo-mutants 27.1.0 output, unchanged:** `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt` and
  `unviable.txt`.
- **`baseline.log`:** a header, then sections 1 to 5 above. Each section gives its command, rc and wall time.
- **Derived from the main run and F1 to F3**, each one name per line, in list order:
  - `caught-only-by-timing-gate.txt` (23);
  - `missed-caught-by-zstd-tests.txt` (52);
  - `missed-after-followups.txt` (194);
  - `timeout-after-followups.txt` (16).
- **`timing-gate-check.log`:** the unmutated gate check (A: alone; B: two at a time).
- **`followup-1-missed-with-zstd/`, `followup-2-gate-only-main/`, `followup-3-gate-only-f1/`:** each run's `outcomes.json`
  and its `.txt` lists. Each run also holds the two `delete field` extras named above.
