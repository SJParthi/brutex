# Mutation testing: shard 29 of 30 (job MUT-29)

Labels follow README-CLOUD.md: **MEASURED** is a number from a named run on this box, **EXACT** is a count. **INFERRED** marks a
figure read from this run's per-mutant logs rather than from a separate run. No figure here is a trading result.

## Read this first

- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt` and `unviable.txt` are cargo-mutants' own output, unedited:
  **215 caught, 256 missed, 0 timeout, 30 unviable**, of 501 (EXACT).
- **11 of those 215 catches are false.**
  - In each one, the only failing test was `split18::tests::v32rb_split_o1_gates`.
  - That test is a wall-clock gate: account p99 under 2 ms and close p99 under 20 µs (`split18.rs:2120-2154`). It calls none of
    the mutated code.
  - Under `--jobs 2` on 4 vCPU it failed on time alone. The failing run in `slice.rs:347:5`'s log measured "97 months: account
    p99 4042330 ns".
  - **Re-tested one at a time with `--jobs 1`, all 11 are MISSED** (see "Follow-up").
- **Corrected result: 204 caught, 267 missed, 0 timeout, 30 unviable** (EXACT, raw counts plus that follow-up).
- **This shard ran 2 tests that shards 0 to 3 skipped, and 106 of the 215 catches came from them alone** (see "Exclusions").

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`.
  - Code only: no market data was read, fetched or written.
  - None of the 3 parameter tables was present.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GiB RAM.
  - `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, both pinned by `rust-toolchain.toml`.
- **cargo-mutants 27.1.0**, installed with `cargo install cargo-mutants --locked`.
  - It was the newest release on crates.io at install time.
  - It is also the version shards 0 to 3 report. Slices are ranges of one list, so every shard needs the same version.
- **One box change, outside the checkout:** zstd 1.5.5 from apt, plus the symlink `/opt/homebrew/bin/zstd -> /usr/bin/zstd`.
  "Exclusions" says why. Nothing was added to the checkout.
- **Command,** run from the checkout root under `nohup`:

  ```sh
  cargo mutants --workspace --shard 29/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 12 tests listed under "Exclusions">
  ```

- **What cargo-mutants ran for each mutant** (from `outcomes.json`):
  `cargo test --package=spikefade@0.1.0 --locked -- --exact <the 12 skips>`.
  - Its baseline passed in 13 s build + 4 s test: 160 passed, 1 ignored, 7 filtered out.
  - It auto-set the test timeout to 21 s. There is no build timeout (the default).
- **Wall (MEASURED):** 2026-11-16 11:42:54Z to 12:16:31Z per `outcomes.json`, so 33 min 37 s.
  - cargo-mutants' last line: "501 mutants tested in 34m: 256 missed, 215 caught, 30 unviable".

## Result (EXACT)

Shard 29 is the last slice under the default `slice` sharding. It holds list positions 15,139 to 15,639 (1-based) of the
workspace's 15,639 mutants: **501 mutants, all in `crates/spikefade`**.

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| slice.rs, line 174 to the end | 166 | 61 | 0 | 29 | 256 |
| synth.rs | 49 | 195 | 0 | 1 | 245 |
| **raw, as in `outcomes.json`** | **215** | **256** | **0** | **30** | **501** |
| **corrected by the follow-up** (11 false catches: 2 in slice.rs, 9 in synth.rs) | **204** | **267** | **0** | **30** | **501** |

## Exclusions: 12 tests, not shards 0 to 3's 14

The full record is in `baseline.log`. It holds the literal `cargo test --locked --workspace` (rc 101, compile failure), every
compilable target with no exclusions, and the run with the 12 skips: 660 passed, 0 failed, 3 ignored, 12 filtered out.

### Skipped by exact name: each needs a file this repository does not hold

| test | package | missing file |
|---|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | spikefade | `crates/spikefade/data/EXITS-DYN-*.tsv` |
| `menu::tests::sf3_menu_fail_closed` | spikefade | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | spikefade | the same |
| `menu::tests::sf3_distance_floor_counted` | spikefade | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | spikefade | the same |
| `tests33s::v33_search1d_sets_load_and_count` | spikefade | the same |
| `tests3::sf3_ttp_replaces_tp_close` | spikefade | the same |
| `every_declared_cost_invariant_reaches_a_bench` | core, `tests/cost_invariants.rs` | `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | core, the same | the same |
| `every_measured_cost_invariant_is_declared` | core, the same | the same |
| `the_reconciliation_is_reading_something` | core, the same | the same |
| `every_crate_carries_a_ratio_bench` | core, the same | `crates/spikefade/benches/ratio.rs` |

- The 7 spikefade tests are the 7 that README-CLOUD.md section 5 predicts.
- The 5 core tests match no spikefade test, so they change nothing here. They are passed so that the list matches the baseline.
- **Never built in this shard:** core's 6 test targets that cannot compile, each on `include_str!` of an app-repository file.
  They are the lib unit tests, `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`. cargo-mutants
  builds only the mutated package, and every mutant here is in spikefade.

### Not skipped: the 2 zstd tests

On the box as delivered, `tests34::root_digest_same_for_every_form` and `tests34::sfz_roundtrip_all_rows` fail because they
exec `/opt/homebrew/bin/zstd`.

- **That is a missing tool on the box, not a file missing from the repository,** so they do not meet the exclusion rule.
- **Installing zstd alone cannot fix them.**
  - Both call `all_off()`, so PORTABLE is off.
  - `kit::tool("zstd")` (`kit.rs:450-478`) then returns the Mac path unconditionally. `SF_ZSTD` and PATH are read only under
    PORTABLE.
  - That is why shard 3 still skipped them after installing zstd.
- **With zstd 1.5.5 linked at the Mac path, both pass** (`baseline.log` section 4).
- **Why it matters for this shard and for no earlier one.**
  - These 2 tests are the main tests of `slice.rs`'s transport-file codec: `sfz_encode`, `sfz_decode`, `sec_split`, `sections`,
    and `zz` / `unzz`. They also cover `cmd_cube_slice`.
  - Shards 0 to 3 held no spikefade mutant, so their 14 skips cost them nothing.
- **106 of the 215 catches came from these 2 tests alone** (INFERRED: in each mutant's log, the libtest failure list names only
  these 2 tests).
  - 105 are in slice.rs and 1 in synth.rs. They are listed at the end of this file.
  - With shards 0 to 3's 14 skips, those 106 would have been reported MISSED.
- No caught mutant's log lacks a failure list, so there was no harness crash.

## Follow-up: the 11 catches made by the timing gate alone

These figures are **not** in `outcomes.json`. They come from a second run whose output was kept outside the checkout and is
not committed.

- **Selection:** the 11 names, each escaped and anchored as `-F '^<name>$'`, plus `-f` for `synth.rs` and `slice.rs`.
  - In 27.1.0, `-F` and `-E` do not filter the "delete field ... from struct ... expression" mutants: `--list` with `-F` alone
    gave all 206 of them as well.
  - The file globs limit that to the 2 such mutants in `synth.rs`, both already in this slice and both MISSED in the main run.
  - The `--list` output was 13 mutants: the 11 plus those 2.
- **Command:**

  ```sh
  cargo mutants --workspace -f crates/spikefade/src/synth.rs -f crates/spikefade/src/slice.rs \
    -F '^<escaped name>$' (x 11) --jobs 1 --cargo-arg=--locked -o <outside the checkout> \
    -- -- --exact --skip <the same 12>
  ```

- **Wall (MEASURED):** 2026-11-16 12:20:12Z to 12:21:47Z. Its baseline passed in 14.7 s build + 4.0 s test.
- **Result (EXACT): 13 tested, 13 MISSED.**

```
MISSED   crates/spikefade/src/slice.rs:347:5: replace sfz_bin -> Result<Vec<u8>, String> with Ok(vec![1]) in 1s build + 4s test
MISSED   crates/spikefade/src/slice.rs:484:36: replace || with && in check_slice_equal in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:47:20: replace + with - in next_weekday in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:52:5: replace month_last_thursday -> i64 with 0 in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:65:86: replace + with * in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:66:24: delete field ymd from struct Cube expression in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:66:29: delete field day from struct Cube expression in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:133:40: replace * with / in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:137:25: replace * with / in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:163:31: replace * with + in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:168:19: replace += with *= in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:189:24: replace | with ^ in day in 1s build + 4s test
MISSED   crates/spikefade/src/synth.rs:216:79: replace + with - in cmd_synth in 1s build + 4s test
```

**For the other spikefade shards:** any spikefade shard run with `--jobs 2` can get the same false catches. A catch whose only
failing test is `split18::tests::v32rb_split_o1_gates` should be re-tested alone before it counts. That test fired in 24 of the
215 catch logs here: 11 times alone, and 13 times beside tests that caught the mutant on their own.

## How to read the 267 missed (corrected counts)

### synth.rs: 204 of 245 missed

| function | caught | missed | unviable | total | false catches removed |
|---|---:|---:|---:|---:|---:|
| `day` | 33 | 163 | 1 | 197 | 6 |
| `month_last_thursday` | 0 | 16 | 0 | 16 | 1 |
| `Rng::next` | 2 | 9 | 0 | 11 | 0 |
| `next_weekday` | 0 | 9 | 0 | 9 | 1 |
| `cmd_synth` | 0 | 7 | 0 | 7 | 1 |
| `Rng::below` | 5 | 0 | 0 | 5 | 0 |

- `synth.rs` is the synthetic-day generator behind `spikefade synth`, the C2 canary in README-CLOUD.md section 6.
- Tests reach it only from `tests34.rs`: `synth_roots` calls `synth::day`, and other tests draw from `synth::Rng`.
- **Nothing pins the generated bytes.**
  - The tests check reproducibility: one seed gives the same bytes, another seed gives other bytes.
  - They check structure: index first, both segments, seconds rows, at least one spike.
  - They check round trips and digest equality across root forms.
  - Most changes to the generator's arithmetic keep all of those true, and survive.
- **The one pin is not in `cargo test`.** `cmd_synth` is reached only from `main.rs`. The one pin on the generator's output is
  section 6's expected digest, `SYNTHETIC SET sha256 7712d2a2...`, which `cargo test` does not check.

### slice.rs: 63 of 256 missed

| function | caught | missed | unviable | total | false catches removed |
|---|---:|---:|---:|---:|---:|
| `check_slice_equal` | 54 | 28 | 0 | 82 | 1 |
| `decode_line` | 0 | 10 | 0 | 10 | 0 |
| `cmd_cube_slice` | 10 | 9 | 0 | 19 | 0 |
| `sfz_head` | 6 | 4 | 0 | 10 | 0 |
| `read_sfz` | 0 | 3 | 0 | 3 | 0 |
| `sfz_sec` | 0 | 3 | 0 | 3 | 0 |
| `sfz_bin` | 0 | 3 | 0 | 3 | 1 |
| `sections` | 20 | 1 | 29 | 50 | 0 |
| `sec_split` | 17 | 1 | 0 | 18 | 0 |
| `section` | 5 | 1 | 0 | 6 | 0 |
| `sfz_encode` | 23 | 0 | 0 | 23 | 0 |
| `sfz_decode` | 10 | 0 | 0 | 10 | 0 |
| `unzz` | 9 | 0 | 0 | 9 | 0 |
| `zz` | 6 | 0 | 0 | 6 | 0 |
| `get_var` | 3 | 0 | 0 | 3 | 0 |
| `write_new` | 1 | 0 | 0 | 1 | 0 |

- **`check_slice_equal`** is only ever called on equal inputs.
  - Its callers are the self-check inside `cmd_cube_slice` (`slice.rs:431`) and `tests34.rs:619`, which calls `.unwrap()`.
  - No test hands it a slice that differs, so a mutant that weakens a comparison passes.
- **`decode_line`** builds the MEASURED decode-timing line (p50 and p99 per section decode, bytes read). No test reads it.
- **`read_sfz`, `sfz_sec` and `sfz_bin`** are the E9 loader hooks.
  - They are reached from `cube.rs:1008` and `cube.rs:1183`, and only when `kit::TRANSPORT` is on (`--cube-transport`).
  - No test turns TRANSPORT on.
- **The rest are partial gaps:** `cmd_cube_slice`, `sfz_head`, `sections`, `sec_split` and `section`. See `missed.txt`.

## The 11 false catches (CaughtMutant in `outcomes.json`, MISSED in the follow-up)

```
crates/spikefade/src/slice.rs:347:5: replace sfz_bin -> Result<Vec<u8>, String> with Ok(vec![1])
crates/spikefade/src/slice.rs:484:36: replace || with && in check_slice_equal
crates/spikefade/src/synth.rs:47:20: replace + with - in next_weekday
crates/spikefade/src/synth.rs:52:5: replace month_last_thursday -> i64 with 0
crates/spikefade/src/synth.rs:65:86: replace + with * in day
crates/spikefade/src/synth.rs:133:40: replace * with / in day
crates/spikefade/src/synth.rs:137:25: replace * with / in day
crates/spikefade/src/synth.rs:163:31: replace * with + in day
crates/spikefade/src/synth.rs:168:19: replace += with *= in day
crates/spikefade/src/synth.rs:189:24: replace | with ^ in day
crates/spikefade/src/synth.rs:216:79: replace + with - in cmd_synth
```

## The 106 catches made only by the 2 zstd tests

```
crates/spikefade/src/slice.rs:174:14: replace & with ^ in get_var
crates/spikefade/src/slice.rs:177:12: replace += with *= in get_var
crates/spikefade/src/slice.rs:177:12: replace += with -= in get_var
crates/spikefade/src/slice.rs:182:5: replace zz -> u64 with 0
crates/spikefade/src/slice.rs:182:5: replace zz -> u64 with 1
crates/spikefade/src/slice.rs:182:9: replace << with >> in zz
crates/spikefade/src/slice.rs:182:15: replace ^ with & in zz
crates/spikefade/src/slice.rs:182:15: replace ^ with | in zz
crates/spikefade/src/slice.rs:182:20: replace >> with << in zz
crates/spikefade/src/slice.rs:186:5: replace unzz -> i64 with 0
crates/spikefade/src/slice.rs:186:5: replace unzz -> i64 with 1
crates/spikefade/src/slice.rs:186:9: replace >> with << in unzz
crates/spikefade/src/slice.rs:186:23: replace ^ with & in unzz
crates/spikefade/src/slice.rs:186:23: replace ^ with | in unzz
crates/spikefade/src/slice.rs:186:25: delete - in unzz
crates/spikefade/src/slice.rs:186:30: replace & with ^ in unzz
crates/spikefade/src/slice.rs:186:30: replace & with | in unzz
crates/spikefade/src/slice.rs:193:5: replace sec_split -> Result<(usize, usize), String> with Ok((0, 0))
crates/spikefade/src/slice.rs:193:5: replace sec_split -> Result<(usize, usize), String> with Ok((0, 1))
crates/spikefade/src/slice.rs:193:5: replace sec_split -> Result<(usize, usize), String> with Ok((1, 0))
crates/spikefade/src/slice.rs:193:20: replace != with == in sec_split
crates/spikefade/src/slice.rs:196:40: replace + with - in sec_split
crates/spikefade/src/slice.rs:199:19: replace + with * in sec_split
crates/spikefade/src/slice.rs:199:19: replace + with - in sec_split
crates/spikefade/src/slice.rs:199:25: replace + with * in sec_split
crates/spikefade/src/slice.rs:199:30: replace * with + in sec_split
crates/spikefade/src/slice.rs:199:30: replace * with / in sec_split
crates/spikefade/src/slice.rs:201:16: replace != with == in sec_split
crates/spikefade/src/slice.rs:201:24: replace + with - in sec_split
crates/spikefade/src/slice.rs:201:29: replace * with + in sec_split
crates/spikefade/src/slice.rs:201:29: replace * with / in sec_split
crates/spikefade/src/slice.rs:212:5: replace sfz_encode -> Result<Vec<u8>, String> with Ok(vec![0])
crates/spikefade/src/slice.rs:212:5: replace sfz_encode -> Result<Vec<u8>, String> with Ok(vec![1])
crates/spikefade/src/slice.rs:212:5: replace sfz_encode -> Result<Vec<u8>, String> with Ok(vec![])
crates/spikefade/src/slice.rs:216:22: replace + with * in sfz_encode
crates/spikefade/src/slice.rs:216:22: replace + with - in sfz_encode
crates/spikefade/src/slice.rs:216:26: replace * with + in sfz_encode
crates/spikefade/src/slice.rs:216:26: replace * with / in sfz_encode
crates/spikefade/src/slice.rs:217:38: replace + with * in sfz_encode
crates/spikefade/src/slice.rs:217:38: replace + with - in sfz_encode
crates/spikefade/src/slice.rs:217:42: replace * with + in sfz_encode
crates/spikefade/src/slice.rs:217:42: replace * with / in sfz_encode
crates/spikefade/src/slice.rs:217:49: replace + with * in sfz_encode
crates/spikefade/src/slice.rs:217:49: replace + with - in sfz_encode
crates/spikefade/src/slice.rs:217:53: replace * with + in sfz_encode
crates/spikefade/src/slice.rs:217:53: replace * with / in sfz_encode
crates/spikefade/src/slice.rs:217:57: replace + with * in sfz_encode
crates/spikefade/src/slice.rs:217:57: replace + with - in sfz_encode
crates/spikefade/src/slice.rs:220:28: replace - with + in sfz_encode
crates/spikefade/src/slice.rs:220:28: replace - with / in sfz_encode
crates/spikefade/src/slice.rs:221:29: replace - with + in sfz_encode
crates/spikefade/src/slice.rs:221:29: replace - with / in sfz_encode
crates/spikefade/src/slice.rs:222:29: replace - with + in sfz_encode
crates/spikefade/src/slice.rs:222:29: replace - with / in sfz_encode
crates/spikefade/src/slice.rs:252:5: replace sfz_head -> Result<(u32, u32), String> with Ok((1, 0))
crates/spikefade/src/slice.rs:252:5: replace sfz_head -> Result<(u32, u32), String> with Ok((1, 1))
crates/spikefade/src/slice.rs:255:35: replace + with - in sfz_head
crates/spikefade/src/slice.rs:256:14: replace != with == in sfz_head
crates/spikefade/src/slice.rs:256:29: replace != with == in sfz_head
crates/spikefade/src/slice.rs:264:5: replace sections -> Result<Vec<(usize, std::ops::Range<usize>, [u8; 32])>, String> with Ok(vec![])
crates/spikefade/src/slice.rs:265:21: replace + with * in sections
crates/spikefade/src/slice.rs:265:29: replace * with + in sections
crates/spikefade/src/slice.rs:265:29: replace * with / in sections
crates/spikefade/src/slice.rs:267:20: replace + with * in sections
crates/spikefade/src/slice.rs:267:20: replace + with - in sections
crates/spikefade/src/slice.rs:267:24: replace * with + in sections
crates/spikefade/src/slice.rs:267:24: replace * with / in sections
crates/spikefade/src/slice.rs:268:39: replace + with - in sections
crates/spikefade/src/slice.rs:269:91: replace + with * in sections
crates/spikefade/src/slice.rs:269:91: replace + with - in sections
crates/spikefade/src/slice.rs:271:36: replace + with * in sections
crates/spikefade/src/slice.rs:271:36: replace + with - in sections
crates/spikefade/src/slice.rs:271:44: replace + with * in sections
crates/spikefade/src/slice.rs:271:44: replace + with - in sections
crates/spikefade/src/slice.rs:272:28: replace + with * in sections
crates/spikefade/src/slice.rs:272:28: replace + with - in sections
crates/spikefade/src/slice.rs:273:12: replace += with *= in sections
crates/spikefade/src/slice.rs:273:12: replace += with -= in sections
crates/spikefade/src/slice.rs:275:11: replace != with == in sections
crates/spikefade/src/slice.rs:282:5: replace section -> Result<Vec<u8>, String> with Ok(vec![0])
crates/spikefade/src/slice.rs:282:5: replace section -> Result<Vec<u8>, String> with Ok(vec![1])
crates/spikefade/src/slice.rs:282:5: replace section -> Result<Vec<u8>, String> with Ok(vec![])
crates/spikefade/src/slice.rs:285:18: replace != with == in section
crates/spikefade/src/slice.rs:285:39: replace != with == in section
crates/spikefade/src/slice.rs:296:5: replace sfz_decode -> Result<Vec<u8>, String> with Ok(vec![0])
crates/spikefade/src/slice.rs:296:5: replace sfz_decode -> Result<Vec<u8>, String> with Ok(vec![1])
crates/spikefade/src/slice.rs:313:26: replace + with * in sfz_decode
crates/spikefade/src/slice.rs:313:26: replace + with - in sfz_decode
crates/spikefade/src/slice.rs:314:27: replace + with * in sfz_decode
crates/spikefade/src/slice.rs:314:27: replace + with - in sfz_decode
crates/spikefade/src/slice.rs:315:28: replace - with + in sfz_decode
crates/spikefade/src/slice.rs:315:28: replace - with / in sfz_decode
crates/spikefade/src/slice.rs:324:54: replace != with == in sfz_decode
crates/spikefade/src/slice.rs:382:5: replace cmd_cube_slice -> Result<String, String> with Ok("xyzzy".into())
crates/spikefade/src/slice.rs:382:5: replace cmd_cube_slice -> Result<String, String> with Ok(String::new())
crates/spikefade/src/slice.rs:389:66: replace == with != in cmd_cube_slice
crates/spikefade/src/slice.rs:396:12: delete ! in cmd_cube_slice
crates/spikefade/src/slice.rs:418:41: replace != with == in cmd_cube_slice
crates/spikefade/src/slice.rs:418:75: replace != with == in cmd_cube_slice
crates/spikefade/src/slice.rs:426:22: replace + with - in cmd_cube_slice
crates/spikefade/src/slice.rs:439:15: replace += with -= in cmd_cube_slice
crates/spikefade/src/slice.rs:439:32: replace + with - in cmd_cube_slice
crates/spikefade/src/slice.rs:440:15: replace += with -= in cmd_cube_slice
crates/spikefade/src/slice.rs:451:5: replace write_new -> Result<(), String> with Ok(())
crates/spikefade/src/slice.rs:482:59: replace < with <= in check_slice_equal
crates/spikefade/src/synth.rs:130:38: replace + with - in day
```
