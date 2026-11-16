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

## The run

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

## Result (EXACT, from `outcomes.json`)

**522 tested: 275 caught, 226 missed, 15 timeout, 6 unviable.**

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| ckmerge.rs (line 209 to the end) | 3 | 59 | 0 | 2 | 64 |
| kit.rs (all of it) | 250 | 109 | 15 | 4 | 378 |
| slice.rs (up to line 174) | 22 | 58 | 0 | 0 | 80 |
| **all 3 files** | **275** | **226** | **15** | **6** | **522** |

### Missed mutants by function

`missed.txt` lists all 226 of them.

| file | function | caught | missed | timeout | unviable |
|---|---|---:|---:|---:|---:|
| ckmerge.rs | `cmd_ckpt_merge` | 1 | **32** | 0 | 0 |
| ckmerge.rs | `canon` | 0 | **11** | 0 | 2 |
| ckmerge.rs | `cmd_ckpt_canon` | 2 | **4** | 0 | 0 |
| ckmerge.rs | `cmd_bck_check` | 0 | **4** | 0 | 0 |
| ckmerge.rs | `ident` | 0 | **8** | 0 | 0 |
| kit.rs | `any` | 1 | **1** | 0 | 0 |
| kit.rs | `words` | 0 | **2** | 0 | 0 |
| kit.rs | `args_digest` | 7 | **4** | 3 | 0 |
| kit.rs | `from_args` | 12 | **20** | 0 | 1 |
| kit.rs | `shard_walks` | 7 | **1** | 0 | 0 |
| kit.rs | `parse_set` | 6 | **2** | 0 | 0 |
| kit.rs | `Sha256::block` | 42 | **2** | 0 | 0 |
| kit.rs | `Sha256::update` | 3 | **1** | 8 | 0 |
| kit.rs | `tool` | 7 | **3** | 0 | 1 |
| kit.rs | `utc_now` | 0 | **2** | 0 | 0 |
| kit.rs | `now_ist` | 0 | **3** | 0 | 0 |
| kit.rs | `rss_kb` | 1 | **2** | 0 | 1 |
| kit.rs | `proc_cpu_s` | 2 | **5** | 0 | 0 |
| kit.rs | `strict_fail` | 5 | **2** | 0 | 0 |
| kit.rs | `strict_check` | 0 | **5** | 0 | 0 |
| kit.rs | `probe_tools` | 1 | **9** | 0 | 0 |
| kit.rs | `Plan::load` | 0 | **1** | 0 | 0 |
| kit.rs | `Plan::parse` | 20 | **2** | 0 | 0 |
| kit.rs | `Plan::check` | 19 | **2** | 0 | 0 |
| kit.rs | `fp_part` | 0 | **4** | 0 | 0 |
| kit.rs | `par_digest` | 7 | **1** | 1 | 0 |
| kit.rs | `list_digest` | 4 | **1** | 0 | 1 |
| kit.rs | `table_lines` | 3 | **8** | 0 | 0 |
| kit.rs | `inputs_manifest` | 0 | **2** | 0 | 0 |
| kit.rs | `check_inputs` | 0 | **4** | 0 | 0 |
| kit.rs | `Bck::parse` | 33 | **11** | 0 | 0 |
| kit.rs | `exe_sha` | 1 | **1** | 0 | 0 |
| kit.rs | `host` | 1 | **2** | 0 | 0 |
| kit.rs | `source_sha` | 0 | **2** | 0 | 0 |
| kit.rs | `provenance` | 1 | **4** | 0 | 0 |
| slice.rs | `magic_of` | 0 | **3** | 0 | 0 |
| slice.rs | `slice_bytes` | 0 | **18** | 0 | 0 |
| slice.rs | `root_day_digest` | 1 | **9** | 0 | 0 |
| slice.rs | `zstd_pipe` | 0 | **4** | 0 | 0 |
| slice.rs | `zstd_c` | 1 | **2** | 0 | 0 |
| slice.rs | `zstd_d` | 0 | **3** | 0 | 0 |
| slice.rs | `put_var` | 1 | **6** | 0 | 0 |
| slice.rs | `get_var` | 0 | **13** | 0 | 0 |

## Read this before using the MISSED count

**The 58 `slice.rs` misses are confounded with an exclusion.** Two of the 14 skipped tests can only run where
`/opt/homebrew/bin/zstd` exists: `tests34::sfz_roundtrip_all_rows` and `tests34::root_digest_same_for_every_form`.

- Both tests call `all_off()`, which turns PORTABLE off. `kit::tool("zstd")` then returns the Mac path and ignores `SF_ZSTD`
  and PATH (`kit.rs:461-470`).
- The source shows that all 8 functions holding the 58 misses are on these two tests' call paths:
  - `sfz_encode` calls `put_var` and `zstd_c`, which calls `zstd_pipe`;
  - `sfz_decode` calls `magic_of`, `zstd_d` (which calls `zstd_pipe`) and `get_var`;
  - `root_day_digest` calls `slice_bytes`, which calls `magic_of`.
- So on this box a `slice.rs` miss cannot be told apart from a real gap. On the Mac, where those two tests run, some of these
  58 may be caught.
- The same goes for `kit.rs` `tool`, which the two tests call.

**The other 7 skipped spikefade tests** read the 3 parameter tables. They are `menu.rs` and `tests3`/`tests33s` tests. Their
bodies make no direct call into `ckmerge`, `kit` or `slice`. Indirect reach was not traced.

**The 15 timeouts** all look like real hangs, not box noise. Each one hit the 29 s limit while the baseline test phase took
5 to 8 s:

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
- cargo-mutants counts both **caught**, because the test process died.
- No other process was killed. The other job's mutants at that time were the `Ok((0, 1))` and `Ok((1, 1))` siblings, both
  TIMEOUT.

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
| `tests34::root_digest_same_for_every_form` | spikefade | runs `/opt/homebrew/bin/zstd`, the Mac tool path. It is not on this box, and the test turns PORTABLE off |
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

- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt` and `unviable.txt`: cargo-mutants 27.1.0 output, unchanged.
- `baseline.log`: a header, then sections 1 to 5 above. Each section gives its command, rc and wall time.
