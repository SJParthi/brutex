# Mutation shard 14/30: results (job MUT-14)

Code only. No market data was read, fetched or written. **EXACT** = a count. **MEASURED** = a wall time taken on this box.
No figure here is a trading result.

## Source and tools

- Branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`.
  - README-CLOUD.md section 4's source manifest was checked before any build: 146 files, sha256 `c3722928...` (matches).
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked`: the version shards 0-3 used.
- The box: a Linux x86_64 cloud container, 4 vCPU, 15 GiB RAM. Nothing else was installed (no zstd).

## What shard 14/30 covers

- `cargo mutants --workspace --list` gives **15,639** mutants (EXACT), the same total as shards 0-3.
- `--shard 14/30` (default `slice` sharding, 0-indexed) is list positions 7,309 to 7,830: **522 mutants, all in `crates/spikefade`**.
  - `kern.rs`: 290, from `kern.rs:653:51` (`step3g`) to the end of the file.
  - `menu.rs`: 232, from the start of the file to `menu.rs:625:33` (`rule_at`).
- The set tested (caught + missed + timeout + unviable) equals the pre-run `--list --shard 14/30` exactly.

## Baseline and exclusions (raw output: `baseline.log`)

1. **The literal `cargo test --locked --workspace` does not compile** (rc 101 after 3 s).
   - 6 `core` test targets `include_str!` app-repository files that this branch does not hold: the lib unit tests,
     `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`.
   - None of them is built in this shard, which tests `spikefade` only.
2. **Every other target, no exclusions:** 658 passed, 14 failed, 3 ignored (EXACT). Each failure's panic names a file that
   is absent from the repository or the box:
   - 7 `spikefade` tests read `crates/spikefade/data/EXITS-DYN-*.tsv` (README-CLOUD.md section 5):
     - `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts`, `menu::tests::sf3_menu_fail_closed`,
       `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry`, `menu::tests::sf3_distance_floor_counted`,
       `menu::tests::sf3_tp_theta_level_and_absence`;
     - `tests33s::v33_search1d_sets_load_and_count` and `tests3::sf3_ttp_replaces_tp_close`.
   - 2 `spikefade` tests exec `/opt/homebrew/bin/zstd`: `tests34::root_digest_same_for_every_form` and
     `tests34::sfz_roundtrip_all_rows`.
   - 4 `core` `cost_invariants` tests read `docs/04-invariants.md`, and `every_crate_carries_a_ratio_bench` needs
     `crates/spikefade/benches/ratio.rs`.
3. **With exactly those 14 skipped by exact name:** 658 passed, 0 failed, 3 ignored, 14 filtered out (EXACT). They are the
   same 14 that shards 0-3 skipped.
4. **cargo-mutants' own baseline** (`cargo test --package=spikefade` with the skips) passed: 16 s build + 7 s test. It
   auto-set the test timeout to 39 s.

## The run

```sh
cargo mutants --workspace --shard 14/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
  --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed \
  --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted \
  --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count \
  --skip tests3::sf3_ttp_replaces_tp_close --skip tests34::root_digest_same_for_every_form \
  --skip tests34::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench \
  --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared \
  --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something
```

**Result (EXACT, from `outcomes.json`): 522 tested: 240 caught, 254 missed, 12 timeout, 16 unviable.**

- Exit code 3. In 27.1.0 a timeout outranks missed mutants when the exit code is chosen.
- Wall: 50 min 11 s (MEASURED, 11:39:44 to 12:29:55 UTC, 16 Nov 2026).

| file | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| `kern.rs` | 215 | 53 | 12 | 10 | 290 |
| `menu.rs` | 25 | 201 | 0 | 6 | 232 |
| **both** | **240** | **254** | **12** | **16** | **522** |

## Read this before using the counts

### 1. 36 of the 240 catches are false: a wall-clock test failing under load

- `split18::tests::v32rb_split_o1_gates` asserts wall-clock budgets: p99 of one `account` call under 2 ms, p99 of a close
  under 20 us, and a growth ratio.
- In 36 mutants' logs it is the **only** test that failed. Each of the 36 failed on the same line (`split18.rs:2150`,
  `account p99`) with a measured p99 between 4.00 and 4.10 ms, the same for `kern.rs` and `menu.rs` mutants.
  - About 4 ms is one scheduler tick at 250 Hz. The test thread was preempted while `--jobs 2` ran a second build and
    test suite on the same 4 vCPU. A `menu.rs` loader mutation cannot slow `split18`'s `account`.
- **Follow-up (MEASURED, kept out of `outcomes.json`):** the 36 were re-tested with `--jobs 1`, the same 14 skips and
  output outside the checkout. **All 36 passed every test, `v32rb_split_o1_gates` included: all 36 are MISSED.** See the
  last section.
- **Main run with the follow-up's verdicts applied (derived, not in `outcomes.json`):**

  | file | caught | missed | timeout | unviable |
  |---|---:|---:|---:|---:|
  | `kern.rs` | 204 | 64 | 12 | 10 |
  | `menu.rs` | 0 | 226 | 0 | 6 |
  | **both** | **204** | **290** | **12** | **16** |

- Checked and NOT false:
  - the 11 catches where `v32rb_split_o1_gates` failed together with other tests;
  - the 6 catches resting only on `tests32r::v32r_o1_decision_at_n100_and_sampled_steps`. That test also has a 50 us
    timing assert, but all 6 failed on its functional asserts (`tests32r.rs:447`, `:450`, `:452`) or on a divide by zero.
- Every other catch rests on deterministic equality tests.
- **For the other spikefade shards:** the same flake will hit any shard run with `--jobs 2` on 4 vCPU. Either re-test
  every catch whose only failing test is `v32rb_split_o1_gates`, as here, or run with `--jobs 1`.

### 2. `menu.rs`: 226 missed, not a measurement of menu.rs's own tests

- All 5 of `menu.rs`'s own unit tests load the 3 absent parameter tables and were skipped. So were
  `tests3::sf3_ttp_replaces_tp_close` and `tests33s::v33_search1d_sets_load_and_count`, which load them too.
- With those out, **no test that ran catches any `menu.rs` mutant.** Even `Menu::parse -> Ok(Default::default())` and
  `Sets::parse -> Ok(Default::default())` survive.
- Judge `menu.rs` on a box that holds the 3 tables in `crates/spikefade/data/`, with the 7 table tests un-skipped.

| `menu.rs` function | caught | missed | unviable | missed after follow-up |
|---|---:|---:|---:|---:|
| `Kind::allowed` | 0 | 16 | 0 | 16 |
| `Kind::families` | 0 | 3 | 0 | 3 |
| `Kind::parse` | 2 | 17 | 0 | 19 |
| `Kind::sl`, `Kind::trail`, `Kind::word` | 0 | 6 | 0 | 6 |
| `Menu::is_empty`, `Menu::len` | 0 | 4 | 0 | 4 |
| `Menu::load` | 1 | 1 | 0 | 2 |
| `Menu::of_ident` | 0 | 3 | 0 | 3 |
| `Menu::parse` | 3 | 19 | 0 | 22 |
| `Sets::load` | 0 | 3 | 0 | 3 |
| `Sets::parse` | 4 | 23 | 0 | 27 |
| `Sets::select` | 0 | 4 | 0 | 4 |
| `fam_rank` | 0 | 4 | 0 | 4 |
| `parse_setting` | 7 | 45 | 4 | 52 |
| `rule_at` (to line 625) | 0 | 10 | 2 | 10 |
| `sha_of` | 0 | 2 | 0 | 2 |
| `slot_rule` | 8 | 41 | 0 | 49 |

### 3. `kern.rs`: 64 missed, the candidate real gaps

- The only spikefade tests that did not run are the 9 skips.
- None of the 14 skipped tests names a `kern.rs` function that has a miss. This was checked by a static grep, not by
  running them. The one `tests3` skip calls only `Menu::parse` and `rule_at`.

### 4. The 12 timeouts

- All are in `kern.rs`, on two accumulator lines in each of `walk_account` (777, 779), `walk_day` (863, 865) and
  `walk_day_core` (904, 906): `+=` to `*=`, and `+` to `-` or `*`.
- The test phase ran past the 39 s limit, against a 7 s baseline. They were not re-tested.

## Missed in `kern.rs` (64)

The 53 missed in the main run:

```
crates/spikefade/src/kern.rs:672:37: delete - in step3g
crates/spikefade/src/kern.rs:688:23: replace += with -= in step3g
crates/spikefade/src/kern.rs:688:23: replace += with *= in step3g
crates/spikefade/src/kern.rs:690:26: replace += with -= in step3g
crates/spikefade/src/kern.rs:695:15: replace & with | in step3g
crates/spikefade/src/kern.rs:718:5: replace out_of_decision -> Option<Out> with None
crates/spikefade/src/kern.rs:718:5: replace out_of_decision -> Option<Out> with Some(Default::default())
crates/spikefade/src/kern.rs:745:5: replace seg_of -> u8 with 0
crates/spikefade/src/kern.rs:761:15: replace += with -= in walk_account
crates/spikefade/src/kern.rs:765:15: replace += with -= in walk_account
crates/spikefade/src/kern.rs:761:15: replace += with *= in walk_account
crates/spikefade/src/kern.rs:796:568: replace == with != in sum_of
crates/spikefade/src/kern.rs:849:15: replace += with -= in walk_day
crates/spikefade/src/kern.rs:849:15: replace += with *= in walk_day
crates/spikefade/src/kern.rs:854:23: replace == with != in walk_day
crates/spikefade/src/kern.rs:854:64: replace || with && in walk_day
crates/spikefade/src/kern.rs:855:15: replace += with -= in walk_day
crates/spikefade/src/kern.rs:855:15: replace += with *= in walk_day
crates/spikefade/src/kern.rs:892:15: replace += with -= in walk_day_core
crates/spikefade/src/kern.rs:892:15: replace += with *= in walk_day_core
crates/spikefade/src/kern.rs:895:23: replace == with != in walk_day_core
crates/spikefade/src/kern.rs:895:64: replace || with && in walk_day_core
crates/spikefade/src/kern.rs:896:15: replace += with -= in walk_day_core
crates/spikefade/src/kern.rs:896:15: replace += with *= in walk_day_core
crates/spikefade/src/kern.rs:961:19: replace += with -= in replay_day
crates/spikefade/src/kern.rs:971:31: replace + with * in replay_day
crates/spikefade/src/kern.rs:975:31: replace + with - in replay_day
crates/spikefade/src/kern.rs:977:31: replace + with - in replay_day
crates/spikefade/src/kern.rs:995:5: replace finish_p -> Sum with Default::default()
crates/spikefade/src/kern.rs:1005:78: replace || with && in decision_code
crates/spikefade/src/kern.rs:1005:33: replace || with && in decision_code
crates/spikefade/src/kern.rs:1006:25: replace - with / in decision_code
crates/spikefade/src/kern.rs:1012:37: replace - with / in decision_code
crates/spikefade/src/kern.rs:1018:61: replace - with / in decision_code
crates/spikefade/src/kern.rs:1029:201: replace - with / in dedup_groups
crates/spikefade/src/kern.rs:1057:5: replace fnv -> u64 with 0
crates/spikefade/src/kern.rs:1057:5: replace fnv -> u64 with 1
crates/spikefade/src/kern.rs:1059:11: replace ^= with |= in fnv
crates/spikefade/src/kern.rs:1059:11: replace ^= with &= in fnv
crates/spikefade/src/kern.rs:1115:54: replace && with || in walk_accs
crates/spikefade/src/kern.rs:1115:39: replace && with || in walk_accs
crates/spikefade/src/kern.rs:1115:49: replace == with != in walk_accs
crates/spikefade/src/kern.rs:1115:44: replace % with / in walk_accs
crates/spikefade/src/kern.rs:1124:27: replace == with != in walk_accs
crates/spikefade/src/kern.rs:1124:47: replace || with && in walk_accs
crates/spikefade/src/kern.rs:1135:56: replace != with == in walk_accs
crates/spikefade/src/kern.rs:1143:22: replace > with >= in walk_accs
crates/spikefade/src/kern.rs:1145:88: replace - with + in walk_accs
crates/spikefade/src/kern.rs:1176:27: replace * with + in step_weighted
crates/spikefade/src/kern.rs:1176:27: replace * with / in step_weighted
crates/spikefade/src/kern.rs:1180:18: replace >= with < in step_weighted
crates/spikefade/src/kern.rs:1226:53: replace % with / in walk_dedup
crates/spikefade/src/kern.rs:1226:53: replace % with + in walk_dedup
```

The 11 false catches that the follow-up found MISSED:

```
crates/spikefade/src/kern.rs:654:17: replace < with <= in step3g
crates/spikefade/src/kern.rs:690:26: replace += with *= in step3g
crates/spikefade/src/kern.rs:765:15: replace += with *= in walk_account
crates/spikefade/src/kern.rs:796:479: replace == with != in sum_of
crates/spikefade/src/kern.rs:961:19: replace += with *= in replay_day
crates/spikefade/src/kern.rs:971:31: replace + with - in replay_day
crates/spikefade/src/kern.rs:975:31: replace + with * in replay_day
crates/spikefade/src/kern.rs:977:31: replace + with * in replay_day
crates/spikefade/src/kern.rs:1169:110: replace / with * in step_weighted
crates/spikefade/src/kern.rs:1169:63: replace > with >= in step_weighted
crates/spikefade/src/kern.rs:1179:15: replace += with *= in step_weighted
```

## Timeouts (12)

```
crates/spikefade/src/kern.rs:777:15: replace += with *= in walk_account
crates/spikefade/src/kern.rs:777:20: replace + with - in walk_account
crates/spikefade/src/kern.rs:777:20: replace + with * in walk_account
crates/spikefade/src/kern.rs:779:15: replace += with *= in walk_account
crates/spikefade/src/kern.rs:863:15: replace += with *= in walk_day
crates/spikefade/src/kern.rs:863:20: replace + with - in walk_day
crates/spikefade/src/kern.rs:863:20: replace + with * in walk_day
crates/spikefade/src/kern.rs:865:15: replace += with *= in walk_day
crates/spikefade/src/kern.rs:904:15: replace += with *= in walk_day_core
crates/spikefade/src/kern.rs:904:20: replace + with - in walk_day_core
crates/spikefade/src/kern.rs:904:20: replace + with * in walk_day_core
crates/spikefade/src/kern.rs:906:15: replace += with *= in walk_day_core
```

## Follow-up: the 36 false catches, re-tested with `--jobs 1`

- **Selection:** `-f crates/spikefade/src/kern.rs -f crates/spikefade/src/menu.rs` plus one `-F '^<escaped name>$'` per
  mutant.
  - In 27.1.0, `-F` does not filter `delete field ... from struct ... expression` mutants. Without the two `-f`, it
    selected 241. With them it selected the 36 plus 2 such mutants, `menu.rs:475:19` and `menu.rs:475:35` in
    `Sets::load`, both MISSED in the main run.
- **Tests:** the same 14 skips. cargo-mutants' baseline passed (15 s build + 5 s test, timeout auto-set to 29 s).
- **Result (EXACT): 38 tested, 38 missed, 0 caught.** `v32rb_split_o1_gates` passed in all 38. Exit code 2. Wall:
  5 min 23 s (MEASURED, 12:34:17 to 12:39:40 UTC).

The 25 `menu.rs` mutants among the 36 (the 11 in `kern.rs` are listed above):

```
crates/spikefade/src/menu.rs:73:13: delete match arm "SLV" in Kind::parse
crates/spikefade/src/menu.rs:84:13: delete match arm "TIME" in Kind::parse
crates/spikefade/src/menu.rs:189:19: delete field sha from struct Self expression in Menu::load
crates/spikefade/src/menu.rs:226:17: delete match arm (true, "SEARCH") in Menu::parse
crates/spikefade/src/menu.rs:227:17: delete match arm (true, "COMPARISON") |(false, "-") in Menu::parse
crates/spikefade/src/menu.rs:237:16: delete ! in Menu::parse
crates/spikefade/src/menu.rs:327:5: replace slot_rule -> Result<Option<(usize, u8)>, String> with Ok(Some((0, 1)))
crates/spikefade/src/menu.rs:335:17: delete match arm "P" in slot_rule
crates/spikefade/src/menu.rs:336:17: delete match arm "A1" in slot_rule
crates/spikefade/src/menu.rs:338:17: delete match arm "H" in slot_rule
crates/spikefade/src/menu.rs:350:9: delete match arm "TT" in slot_rule
crates/spikefade/src/menu.rs:356:17: delete match arm "P" in slot_rule
crates/spikefade/src/menu.rs:367:17: delete match arm ["GR", j, g] in slot_rule
crates/spikefade/src/menu.rs:397:8: delete ! in slot_rule
crates/spikefade/src/menu.rs:408:5: replace parse_setting -> Result<(Vec<u16>, Vec<u8>, u16, String), String> with Ok((vec![0], vec![0], 1, "xyzzy".into()))
crates/spikefade/src/menu.rs:408:5: replace parse_setting -> Result<(Vec<u16>, Vec<u8>, u16, String), String> with Ok((vec![0], vec![], 0, "xyzzy".into()))
crates/spikefade/src/menu.rs:408:5: replace parse_setting -> Result<(Vec<u16>, Vec<u8>, u16, String), String> with Ok((vec![1], vec![1], 1, String::new()))
crates/spikefade/src/menu.rs:408:5: replace parse_setting -> Result<(Vec<u16>, Vec<u8>, u16, String), String> with Ok((vec![1], vec![], 1, "xyzzy".into()))
crates/spikefade/src/menu.rs:408:5: replace parse_setting -> Result<(Vec<u16>, Vec<u8>, u16, String), String> with Ok((vec![], vec![0], 0, "xyzzy".into()))
crates/spikefade/src/menu.rs:408:5: replace parse_setting -> Result<(Vec<u16>, Vec<u8>, u16, String), String> with Ok((vec![], vec![0], 1, "xyzzy".into()))
crates/spikefade/src/menu.rs:440:25: replace <= with > in parse_setting
crates/spikefade/src/menu.rs:489:35: replace || with && in Sets::parse
crates/spikefade/src/menu.rs:493:16: delete ! in Sets::parse
crates/spikefade/src/menu.rs:539:71: replace == with != in Sets::parse
crates/spikefade/src/menu.rs:558:12: delete ! in Sets::parse
```

The main run's 201 `menu.rs` misses are in `missed.txt`.

## Files in this folder

| file | what it holds |
|---|---|
| `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt` | the main run, copied verbatim from `mutants.out/`; the 36 false catches are still in `caught.txt` there |
| `baseline.log` | a summary header, then the raw output of the literal command (section 1) and of baseline sections 2-4 |
| `SUMMARY.md` | this file |
