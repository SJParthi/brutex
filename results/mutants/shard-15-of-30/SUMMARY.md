# Mutation shard 15/30: results (job MUT-15)

Code only. No market data was read, fetched or written. **EXACT** = a count. **MEASURED** = a wall time taken on this box.

## Source and tools

- Branch `cloud/spikefade-shard-20261116`, commit `15fd4f4737ea11bcef398efa75db240d38128d10`.
  - The README-CLOUD.md section 4 source manifest gives 146 files and sha256 `c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e`, as expected.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked`. It was the latest on crates.io at install time, and it is the version shards 0-3 used.
- The box: Linux x86_64, 4 vCPU, 15 GiB RAM. Nothing else was installed on it (no zstd), and nothing was added to the checkout.

## What shard 15/30 covers

- `cargo mutants --workspace --list` gives **15,639** mutants (EXACT), the same total shards 0-3 report.
- Sharding is 0-indexed and sliced (27.1.0's default), so `--shard 15/30` covers list positions **7830 to 8351**: **522 mutants**, every one in `crates/spikefade`.
  - The slice starts at `menu.rs:626:36` in `rule_at`; shard 14 ends one mutant earlier at `menu.rs:625:33`.
  - It ends inside `report3.rs`'s `wtie` at line 252; shard 16 starts with the next `wtie` replacement.
  - `--shard 15/30 --list` was diffed against those positions of the full list: identical.
- cargo-mutants tests each mutant with its own package's tests, so every mutant here was tested by **spikefade's tests only**. `core`'s six test targets that cannot compile on this branch are never built in this shard.

## Baseline and exclusions (details in `baseline.log`)

The exclusion list is the same 14 tests that shards 0-3 use, and each was derived again here from this box's own baseline.

1. **The literal `cargo test --locked --workspace` does not compile** (rc 101, 2.5 s, MEASURED). Six `core` test targets `include_str!` files of the app repository that this branch does not hold:
   - the lib unit tests;
   - `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`.
2. **Every compilable target, no exclusions** (`--no-fail-fast`): **658 passed, 14 failed, 3 ignored** (EXACT). Every failure is a file that is not on this box:
   - **7 spikefade tests read `crates/spikefade/data/EXITS-DYN-*.tsv`**, the parameter tables README-CLOUD.md section 5 leaves out on purpose:
     - `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts`, `menu::tests::sf3_menu_fail_closed`, `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry`, `menu::tests::sf3_distance_floor_counted`, `menu::tests::sf3_tp_theta_level_and_absence`;
     - `tests33s::v33_search1d_sets_load_and_count`, `tests3::sf3_ttp_replaces_tp_close`.
   - **2 spikefade tests exec the Mac-only path `/opt/homebrew/bin/zstd`**: `tests34::root_digest_same_for_every_form` and `tests34::sfz_roundtrip_all_rows`.
     - They switch PORTABLE off, and `kit::tool` reads `SF_ZSTD` only under PORTABLE (`kit.rs:461-470`). So installing zstd would not make them pass.
   - **4 core `cost_invariants` tests read `docs/04-invariants.md`**: `every_declared_cost_invariant_reaches_a_bench`, `the_document_declares_the_five_operations_rule_4_names`, `every_measured_cost_invariant_is_declared` and `the_reconciliation_is_reading_something`.
   - **1 core `cost_invariants` test requires `crates/spikefade/benches/ratio.rs`**: `every_crate_carries_a_ratio_bench`.
3. **With exactly those 14 skipped**, from a clean target dir: **658 passed, 0 failed, 3 ignored, 14 filtered** (EXACT).
   - Shard 15's scope, `cargo test -p spikefade` with the skips, gives **158 passed, 0 failed, 1 ignored, 9 filtered** (6 s).
4. **cargo-mutants' own baseline passed**: 17 s build and 6 s test. It auto-set the test timeout to **32 s**.

## The run

```
cargo mutants --workspace --shard 15/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
  --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed \
  --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_distance_floor_counted \
  --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count \
  --skip tests3::sf3_ttp_replaces_tp_close --skip tests34::root_digest_same_for_every_form \
  --skip tests34::sfz_roundtrip_all_rows --skip every_declared_cost_invariant_reaches_a_bench \
  --skip the_document_declares_the_five_operations_rule_4_names --skip every_measured_cost_invariant_is_declared \
  --skip every_crate_carries_a_ratio_bench --skip the_reconciliation_is_reading_something
```

- It ran under `nohup` from the checkout root.
- **Wall time (MEASURED):** 46 min 17 s, from `outcomes.json`'s `start_time` 2026-11-16T11:39:51Z to its `end_time` 12:26:08Z. cargo-mutants' own last line: `522 mutants tested in 46m: 277 missed, 236 caught, 9 unviable`.
- **Per mutant:** typically 1-4 s of incremental build and 6-11 s of test.

## Result (EXACT, from `outcomes.json`)

**522 tested: 236 caught, 277 missed, 0 timeout, 9 unviable.** The four `.txt` files agree: 236 + 277 + 0 + 9 = 522.

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| `menu.rs` | 7 | 67 | 0 | 0 | 74 |
| `p3.rs` | 7 | 76 | 0 | 3 | 86 |
| `read.rs` | 187 | 88 | 0 | 0 | 275 |
| `report3.rs` | 35 | 46 | 0 | 6 | 87 |
| **all 4 files** | **236** | **277** | **0** | **9** | **522** |

## How to read the 277 missed

- **`menu.rs` (67 missed): `rule_at` 52, `isqrt` 13, `tf_dependent` 2. Read this group as an upper bound.**
  - `menu.rs` has exactly 5 tests, and all 5 load the exit-rule tables.
  - `rule_at`'s only direct test call is in `tests3::sf3_ttp_replaces_tp_close`, which loads the tables too. `isqrt` is called only from `rule_at`.
  - All of these tests are skipped here, so these 67 mean "not caught by the spikefade tests that can run without the tables".
  - **Judge them on a box that holds the 3 tables.**
- **`p3.rs` (76 missed): `cmd_p3` 46, `new_kind` 24, `to_v21` 6.**
  - `p3.rs` has no test module, and no test in the crate names any of these three functions (static grep). The skips are not the reason.
- **`read.rs` (88 missed):** `window_reading` 19, `liq_pass` 15, `Cal3::add_specials` 12, `Cal3::minutes` 10, `Prep::liq_at` 6, `base_reading` 6, `Prep::vol60_at` 4, `theta_at` 4, `Cal3::load` 2, `Prep::k_of` 2, `Prep::liq_plain` 2, `Cal3::dated_check` 1 and `wilder` 1.
  - 4 more are constant expressions at lines 26 and 28, outside any function.
  - The two zstd tests cannot reach `read.rs`: `slice.rs` and `cube.rs` never call `crate::read`.
- **`report3.rs` (46 missed):** `tie_key` 12, `wtie` 9, `Seg3::ul_day` 8, `pct` 7, `arg` 6, `now_ist` 2, `Seg3::lv_at` 1 and `Seg3::ul` 1.
- **Not measured:** for `p3.rs`, `read.rs` and `report3.rs`, no skipped test names a missed function directly. This box did not measure whether a skipped table test would reach one through a caller.

## Unviable (9): the mutant does not compile

```
crates/spikefade/src/p3.rs:28:5: replace to_v21 -> Option<Rule> with Some(Default::default())
crates/spikefade/src/p3.rs:166:49: replace && with || in cmd_p3
crates/spikefade/src/p3.rs:181:49: replace && with || in cmd_p3
crates/spikefade/src/report3.rs:180:9: replace Seg3::ul -> Option<std::borrow::Cow<'_, [u32]>> with Some(Cow::Borrowed(Vec::leak(Vec::new())))
crates/spikefade/src/report3.rs:180:9: replace Seg3::ul -> Option<std::borrow::Cow<'_, [u32]>> with Some(Cow::Owned(Vec::leak(Vec::new()).to_owned()))
crates/spikefade/src/report3.rs:180:9: replace Seg3::ul -> Option<std::borrow::Cow<'_, [u32]>> with Some(Cow::Borrowed(Vec::leak(vec![0])))
crates/spikefade/src/report3.rs:180:9: replace Seg3::ul -> Option<std::borrow::Cow<'_, [u32]>> with Some(Cow::Owned(Vec::leak(vec![0]).to_owned()))
crates/spikefade/src/report3.rs:180:9: replace Seg3::ul -> Option<std::borrow::Cow<'_, [u32]>> with Some(Cow::Borrowed(Vec::leak(vec![1])))
crates/spikefade/src/report3.rs:180:9: replace Seg3::ul -> Option<std::borrow::Cow<'_, [u32]>> with Some(Cow::Owned(Vec::leak(vec![1]).to_owned()))
```

## Every missed mutant (277), in list order

`missed.txt` holds the same 277 in completion order.

```
crates/spikefade/src/menu.rs:626:36: replace < with > in rule_at
crates/spikefade/src/menu.rs:626:36: replace < with <= in rule_at
crates/spikefade/src/menu.rs:627:78: replace * with + in rule_at
crates/spikefade/src/menu.rs:627:78: replace * with / in rule_at
crates/spikefade/src/menu.rs:627:148: replace - with + in rule_at
crates/spikefade/src/menu.rs:627:148: replace - with / in rule_at
crates/spikefade/src/menu.rs:627:142: replace + with - in rule_at
crates/spikefade/src/menu.rs:627:142: replace + with * in rule_at
crates/spikefade/src/menu.rs:628:84: replace * with / in rule_at
crates/spikefade/src/menu.rs:666:34: replace match guard cfg!(feature = "m_sf3_ttp_close") && FAMILIES.get(usize::from(r.fam)) == Some(&"TTP") with true in rule_at
crates/spikefade/src/menu.rs:666:34: replace match guard cfg!(feature = "m_sf3_ttp_close") && FAMILIES.get(usize::from(r.fam)) == Some(&"TTP") with false in rule_at
crates/spikefade/src/menu.rs:644:71: replace * with + in rule_at
crates/spikefade/src/menu.rs:646:40: replace * with + in rule_at
crates/spikefade/src/menu.rs:646:40: replace * with / in rule_at
crates/spikefade/src/menu.rs:647:48: replace + with - in rule_at
crates/spikefade/src/menu.rs:647:48: replace + with * in rule_at
crates/spikefade/src/menu.rs:653:42: replace / with % in rule_at
crates/spikefade/src/menu.rs:653:42: replace / with * in rule_at
crates/spikefade/src/menu.rs:654:36: replace * with + in rule_at
crates/spikefade/src/menu.rs:654:36: replace * with / in rule_at
crates/spikefade/src/menu.rs:654:31: replace * with + in rule_at
crates/spikefade/src/menu.rs:654:31: replace * with / in rule_at
crates/spikefade/src/menu.rs:654:26: replace * with + in rule_at
crates/spikefade/src/menu.rs:654:26: replace * with / in rule_at
crates/spikefade/src/menu.rs:655:26: replace * with + in rule_at
crates/spikefade/src/menu.rs:655:26: replace * with / in rule_at
crates/spikefade/src/menu.rs:656:35: replace / with % in rule_at
crates/spikefade/src/menu.rs:656:35: replace / with * in rule_at
crates/spikefade/src/menu.rs:657:31: replace < with == in rule_at
crates/spikefade/src/menu.rs:657:31: replace < with > in rule_at
crates/spikefade/src/menu.rs:657:31: replace < with <= in rule_at
crates/spikefade/src/menu.rs:657:25: replace * with + in rule_at
crates/spikefade/src/menu.rs:657:25: replace * with / in rule_at
crates/spikefade/src/menu.rs:657:21: replace * with + in rule_at
crates/spikefade/src/menu.rs:657:21: replace * with / in rule_at
crates/spikefade/src/menu.rs:658:19: replace += with -= in rule_at
crates/spikefade/src/menu.rs:658:19: replace += with *= in rule_at
crates/spikefade/src/menu.rs:664:68: delete - in rule_at
crates/spikefade/src/menu.rs:665:67: replace * with / in rule_at
crates/spikefade/src/menu.rs:666:104: replace == with != in rule_at
crates/spikefade/src/menu.rs:674:86: replace * with / in rule_at
crates/spikefade/src/menu.rs:691:27: replace && with || in rule_at
crates/spikefade/src/menu.rs:691:21: replace >= with < in rule_at
crates/spikefade/src/menu.rs:691:30: delete ! in rule_at
crates/spikefade/src/menu.rs:697:54: replace == with != in rule_at
crates/spikefade/src/menu.rs:698:71: replace / with % in rule_at
crates/spikefade/src/menu.rs:698:71: replace / with * in rule_at
crates/spikefade/src/menu.rs:698:52: replace * with + in rule_at
crates/spikefade/src/menu.rs:698:52: replace * with / in rule_at
crates/spikefade/src/menu.rs:700:18: replace >= with < in rule_at
crates/spikefade/src/menu.rs:706:67: replace * with + in rule_at
crates/spikefade/src/menu.rs:706:67: replace * with / in rule_at
crates/spikefade/src/menu.rs:713:5: replace isqrt -> u128 with 0
crates/spikefade/src/menu.rs:713:5: replace isqrt -> u128 with 1
crates/spikefade/src/menu.rs:713:10: replace < with == in isqrt
crates/spikefade/src/menu.rs:713:10: replace < with > in isqrt
crates/spikefade/src/menu.rs:713:10: replace < with <= in isqrt
crates/spikefade/src/menu.rs:718:13: replace < with == in isqrt
crates/spikefade/src/menu.rs:718:13: replace < with > in isqrt
crates/spikefade/src/menu.rs:718:13: replace < with <= in isqrt
crates/spikefade/src/menu.rs:720:25: replace / with % in isqrt
crates/spikefade/src/menu.rs:720:16: replace + with - in isqrt
crates/spikefade/src/menu.rs:720:16: replace + with * in isqrt
crates/spikefade/src/menu.rs:720:20: replace / with % in isqrt
crates/spikefade/src/menu.rs:720:20: replace / with * in isqrt
crates/spikefade/src/menu.rs:728:5: replace tf_dependent -> bool with true
crates/spikefade/src/menu.rs:728:5: replace tf_dependent -> bool with false
crates/spikefade/src/p3.rs:28:5: replace to_v21 -> Option<Rule> with None
crates/spikefade/src/p3.rs:29:9: delete match arm XRule::Stop{x} in to_v21
crates/spikefade/src/p3.rs:30:9: delete match arm XRule::QuickCut{x} in to_v21
crates/spikefade/src/p3.rs:31:9: delete match arm XRule::Target{x} in to_v21
crates/spikefade/src/p3.rs:32:9: delete match arm XRule::TimeExit{secs} in to_v21
crates/spikefade/src/p3.rs:34:9: delete match arm XRule::Trail{act, dist} in to_v21
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with None
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, 0, 0))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, 0, 1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, 0, -1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, 1, 0))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, 1, 1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, 1, -1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, -1, 0))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, -1, 1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((0, -1, -1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, 0, 0))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, 0, 1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, 0, -1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, 1, 0))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, 1, 1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, 1, -1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, -1, 0))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, -1, 1))
crates/spikefade/src/p3.rs:40:5: replace new_kind -> Option<(u8, i64, i64)> with Some((1, -1, -1))
crates/spikefade/src/p3.rs:41:9: delete match arm XRule::StopAt{level} in new_kind
crates/spikefade/src/p3.rs:42:9: delete match arm XRule::TargetAt{level} in new_kind
crates/spikefade/src/p3.rs:43:9: delete match arm XRule::TrailPct{act, bp} in new_kind
crates/spikefade/src/p3.rs:44:9: delete match arm XRule::ArmedStop{act, off} in new_kind
crates/spikefade/src/p3.rs:45:9: delete match arm XRule::TimedStop{after, off} in new_kind
crates/spikefade/src/p3.rs:57:5: replace cmd_p3 -> Result<String, String> with Ok(String::new())
crates/spikefade/src/p3.rs:57:5: replace cmd_p3 -> Result<String, String> with Ok("xyzzy".into())
crates/spikefade/src/p3.rs:57:49: replace == with != in cmd_p3
crates/spikefade/src/p3.rs:57:76: replace + with - in cmd_p3
crates/spikefade/src/p3.rs:60:71: replace && with || in cmd_p3
crates/spikefade/src/p3.rs:60:65: replace == with != in cmd_p3
crates/spikefade/src/p3.rs:65:42: replace == with != in cmd_p3
crates/spikefade/src/p3.rs:66:16: delete ! in cmd_p3
crates/spikefade/src/p3.rs:66:36: replace == with != in cmd_p3
crates/spikefade/src/p3.rs:84:106: replace == with != in cmd_p3
crates/spikefade/src/p3.rs:97:28: replace |= with &= in cmd_p3
crates/spikefade/src/p3.rs:104:22: replace != with == in cmd_p3
crates/spikefade/src/p3.rs:133:58: replace || with && in cmd_p3
crates/spikefade/src/p3.rs:133:42: replace != with == in cmd_p3
crates/spikefade/src/p3.rs:138:86: replace * with + in cmd_p3
crates/spikefade/src/p3.rs:138:86: replace * with / in cmd_p3
crates/spikefade/src/p3.rs:144:117: replace != with == in cmd_p3
crates/spikefade/src/p3.rs:144:105: replace & with | in cmd_p3
crates/spikefade/src/p3.rs:144:105: replace & with ^ in cmd_p3
crates/spikefade/src/p3.rs:144:110: replace << with >> in cmd_p3
crates/spikefade/src/p3.rs:155:76: replace * with + in cmd_p3
crates/spikefade/src/p3.rs:161:42: replace += with -= in cmd_p3
crates/spikefade/src/p3.rs:163:46: replace != with == in cmd_p3
crates/spikefade/src/p3.rs:164:50: replace += with -= in cmd_p3
crates/spikefade/src/p3.rs:164:50: replace += with *= in cmd_p3
crates/spikefade/src/p3.rs:166:61: replace < with == in cmd_p3
crates/spikefade/src/p3.rs:166:61: replace < with > in cmd_p3
crates/spikefade/src/p3.rs:166:61: replace < with <= in cmd_p3
crates/spikefade/src/p3.rs:176:42: replace += with -= in cmd_p3
crates/spikefade/src/p3.rs:176:42: replace += with *= in cmd_p3
crates/spikefade/src/p3.rs:178:57: replace != with == in cmd_p3
crates/spikefade/src/p3.rs:179:50: replace += with -= in cmd_p3
crates/spikefade/src/p3.rs:179:50: replace += with *= in cmd_p3
crates/spikefade/src/p3.rs:181:61: replace < with == in cmd_p3
crates/spikefade/src/p3.rs:181:61: replace < with > in cmd_p3
crates/spikefade/src/p3.rs:181:61: replace < with <= in cmd_p3
crates/spikefade/src/p3.rs:187:42: replace += with -= in cmd_p3
crates/spikefade/src/p3.rs:187:42: replace += with *= in cmd_p3
crates/spikefade/src/p3.rs:190:178: delete - in cmd_p3
crates/spikefade/src/p3.rs:191:46: replace += with -= in cmd_p3
crates/spikefade/src/p3.rs:191:46: replace += with *= in cmd_p3
crates/spikefade/src/p3.rs:192:94: replace < with == in cmd_p3
crates/spikefade/src/p3.rs:192:94: replace < with <= in cmd_p3
crates/spikefade/src/p3.rs:216:34: replace += with *= in cmd_p3
crates/spikefade/src/p3.rs:221:32: replace += with -= in cmd_p3
crates/spikefade/src/p3.rs:221:32: replace += with *= in cmd_p3
crates/spikefade/src/read.rs:26:51: replace + with -
crates/spikefade/src/read.rs:26:51: replace + with *
crates/spikefade/src/read.rs:28:49: replace / with *
crates/spikefade/src/read.rs:28:30: replace - with +
crates/spikefade/src/read.rs:75:50: replace + with * in wilder
crates/spikefade/src/read.rs:206:9: replace Prep::vol60_at -> u64 with 0
crates/spikefade/src/read.rs:206:9: replace Prep::vol60_at -> u64 with 1
crates/spikefade/src/read.rs:206:38: replace - with + in Prep::vol60_at
crates/spikefade/src/read.rs:206:38: replace - with / in Prep::vol60_at
crates/spikefade/src/read.rs:211:66: replace - with + in Prep::k_of
crates/spikefade/src/read.rs:211:66: replace - with / in Prep::k_of
crates/spikefade/src/read.rs:243:68: replace - with + in Prep::liq_at
crates/spikefade/src/read.rs:243:68: replace - with / in Prep::liq_at
crates/spikefade/src/read.rs:243:76: replace + with - in Prep::liq_at
crates/spikefade/src/read.rs:243:76: replace + with * in Prep::liq_at
crates/spikefade/src/read.rs:253:31: replace + with - in Prep::liq_at
crates/spikefade/src/read.rs:253:31: replace + with * in Prep::liq_at
crates/spikefade/src/read.rs:265:97: replace > with >= in Prep::liq_plain
crates/spikefade/src/read.rs:265:110: replace > with >= in Prep::liq_plain
crates/spikefade/src/read.rs:284:9: delete match arm 0 in liq_pass
crates/spikefade/src/read.rs:285:9: delete match arm 1 in liq_pass
crates/spikefade/src/read.rs:287:9: delete match arm 3 in liq_pass
crates/spikefade/src/read.rs:288:9: delete match arm 4 in liq_pass
crates/spikefade/src/read.rs:289:9: delete match arm 5 in liq_pass
crates/spikefade/src/read.rs:291:9: delete match arm 7 in liq_pass
crates/spikefade/src/read.rs:293:9: delete match arm 9 in liq_pass
crates/spikefade/src/read.rs:285:18: replace >= with < in liq_pass
crates/spikefade/src/read.rs:288:18: replace <= with > in liq_pass
crates/spikefade/src/read.rs:291:24: replace && with || in liq_pass
crates/spikefade/src/read.rs:291:18: replace >= with < in liq_pass
crates/spikefade/src/read.rs:291:31: replace <= with > in liq_pass
crates/spikefade/src/read.rs:293:24: replace && with || in liq_pass
crates/spikefade/src/read.rs:293:18: replace >= with < in liq_pass
crates/spikefade/src/read.rs:293:31: replace <= with > in liq_pass
crates/spikefade/src/read.rs:324:39: replace || with && in Cal3::load
crates/spikefade/src/read.rs:331:23: replace += with -= in Cal3::load
crates/spikefade/src/read.rs:346:9: replace Cal3::add_specials with ()
crates/spikefade/src/read.rs:346:23: replace && with || in Cal3::add_specials
crates/spikefade/src/read.rs:355:26: replace == with != in Cal3::add_specials
crates/spikefade/src/read.rs:355:22: replace & with | in Cal3::add_specials
crates/spikefade/src/read.rs:355:22: replace & with ^ in Cal3::add_specials
crates/spikefade/src/read.rs:356:31: replace % with / in Cal3::add_specials
crates/spikefade/src/read.rs:356:31: replace % with + in Cal3::add_specials
crates/spikefade/src/read.rs:361:18: replace <= with > in Cal3::add_specials
crates/spikefade/src/read.rs:365:72: replace - with + in Cal3::add_specials
crates/spikefade/src/read.rs:365:72: replace - with / in Cal3::add_specials
crates/spikefade/src/read.rs:365:68: replace + with - in Cal3::add_specials
crates/spikefade/src/read.rs:365:68: replace + with * in Cal3::add_specials
crates/spikefade/src/read.rs:394:24: replace || with && in Cal3::dated_check
crates/spikefade/src/read.rs:417:74: replace / with % in Cal3::minutes
crates/spikefade/src/read.rs:417:74: replace / with * in Cal3::minutes
crates/spikefade/src/read.rs:417:62: replace - with + in Cal3::minutes
crates/spikefade/src/read.rs:417:62: replace - with / in Cal3::minutes
crates/spikefade/src/read.rs:417:21: replace + with - in Cal3::minutes
crates/spikefade/src/read.rs:417:21: replace + with * in Cal3::minutes
crates/spikefade/src/read.rs:417:26: replace * with + in Cal3::minutes
crates/spikefade/src/read.rs:417:26: replace * with / in Cal3::minutes
crates/spikefade/src/read.rs:417:44: replace + with - in Cal3::minutes
crates/spikefade/src/read.rs:417:44: replace + with * in Cal3::minutes
crates/spikefade/src/read.rs:438:5: replace base_reading -> Rd with Default::default()
crates/spikefade/src/read.rs:438:29: replace - with + in base_reading
crates/spikefade/src/read.rs:438:29: replace - with / in base_reading
crates/spikefade/src/read.rs:440:42: replace / with % in base_reading
crates/spikefade/src/read.rs:440:42: replace / with * in base_reading
crates/spikefade/src/read.rs:440:32: replace - with + in base_reading
crates/spikefade/src/read.rs:448:55: replace - with + in theta_at
crates/spikefade/src/read.rs:448:55: replace - with / in theta_at
crates/spikefade/src/read.rs:449:15: replace || with && in theta_at
crates/spikefade/src/read.rs:452:80: replace - with / in theta_at
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (0, 0)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (0, 1)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (0, -1)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (1, 0)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (1, 1)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (1, -1)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (-1, 0)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (-1, 1)
crates/spikefade/src/read.rs:469:5: replace window_reading -> (i64, i64) with (-1, -1)
crates/spikefade/src/read.rs:470:42: replace / with % in window_reading
crates/spikefade/src/read.rs:470:42: replace / with * in window_reading
crates/spikefade/src/read.rs:470:32: replace - with + in window_reading
crates/spikefade/src/read.rs:470:32: replace - with / in window_reading
crates/spikefade/src/read.rs:471:19: replace == with != in window_reading
crates/spikefade/src/read.rs:475:50: replace == with != in window_reading
crates/spikefade/src/read.rs:475:46: replace & with | in window_reading
crates/spikefade/src/read.rs:479:54: replace == with != in window_reading
crates/spikefade/src/read.rs:479:90: replace - with + in window_reading
crates/spikefade/src/read.rs:479:90: replace - with / in window_reading
crates/spikefade/src/report3.rs:47:5: replace now_ist -> String with String::new()
crates/spikefade/src/report3.rs:47:5: replace now_ist -> String with "xyzzy".into()
crates/spikefade/src/report3.rs:54:5: replace arg -> Option<&'a str> with None
crates/spikefade/src/report3.rs:54:5: replace arg -> Option<&'a str> with Some("")
crates/spikefade/src/report3.rs:54:5: replace arg -> Option<&'a str> with Some("xyzzy")
crates/spikefade/src/report3.rs:54:29: replace == with != in arg
crates/spikefade/src/report3.rs:54:56: replace + with - in arg
crates/spikefade/src/report3.rs:54:56: replace + with * in arg
crates/spikefade/src/report3.rs:58:5: replace pct -> u64 with 0
crates/spikefade/src/report3.rs:58:5: replace pct -> u64 with 1
crates/spikefade/src/report3.rs:62:58: replace / with % in pct
crates/spikefade/src/report3.rs:62:54: replace * with + in pct
crates/spikefade/src/report3.rs:62:54: replace * with / in pct
crates/spikefade/src/report3.rs:62:36: replace - with + in pct
crates/spikefade/src/report3.rs:62:36: replace - with / in pct
crates/spikefade/src/report3.rs:155:9: replace Seg3::lv_at -> &Levels with Box::leak(Box::new(Default::default()))
crates/spikefade/src/report3.rs:184:98: replace + with - in Seg3::ul
crates/spikefade/src/report3.rs:194:9: replace Seg3::ul_day with ()
crates/spikefade/src/report3.rs:196:49: replace * with / in Seg3::ul_day
crates/spikefade/src/report3.rs:201:98: replace + with - in Seg3::ul_day
crates/spikefade/src/report3.rs:201:98: replace + with * in Seg3::ul_day
crates/spikefade/src/report3.rs:203:57: replace < with <= in Seg3::ul_day
crates/spikefade/src/report3.rs:203:94: replace < with <= in Seg3::ul_day
crates/spikefade/src/report3.rs:212:53: replace < with <= in Seg3::ul_day
crates/spikefade/src/report3.rs:212:86: replace < with <= in Seg3::ul_day
crates/spikefade/src/report3.rs:235:5: replace tie_key -> u32 with 0
crates/spikefade/src/report3.rs:235:5: replace tie_key -> u32 with 1
crates/spikefade/src/report3.rs:237:9: delete match arm Kind::SlP in tie_key
crates/spikefade/src/report3.rs:238:9: delete match arm Kind::SlA1 in tie_key
crates/spikefade/src/report3.rs:239:9: delete match arm Kind::SlA5 in tie_key
crates/spikefade/src/report3.rs:240:9: delete match arm Kind::SlH in tie_key
crates/spikefade/src/report3.rs:242:9: delete match arm Kind::SlV in tie_key
crates/spikefade/src/report3.rs:246:49: replace | with & in tie_key
crates/spikefade/src/report3.rs:246:49: replace | with ^ in tie_key
crates/spikefade/src/report3.rs:246:34: replace | with & in tie_key
crates/spikefade/src/report3.rs:246:34: replace | with ^ in tie_key
crates/spikefade/src/report3.rs:246:27: replace << with >> in tie_key
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 0, 0, 0)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 0, 0, 1)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 0, 1, 0)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 0, 1, 1)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 1, 0, 0)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 1, 0, 1)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 1, 1, 0)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 0, 1, 1, 1)
crates/spikefade/src/report3.rs:252:5: replace wtie -> (u32, u16, u32, u16, u8) with (0, 1, 0, 0, 0)
```
