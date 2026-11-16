# Mutation testing: shard 27 of 30 (job MUT-27)

Code only. No market data was read, fetched or written. MEASURED = a number from this run on this box. EXACT = a count.
No figure here is a trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. The README-CLOUD.md
  section 4 manifest check passed: 146 files, sha256 `c3722928015fb27164c88ad687f64a7529c4c003dc8bc14825c82477984fbf8e`.
- **Box:** Linux x86_64 cloud container, 4 vCPU, 15 GB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`,
  `cargo 1.97.1 (c980f4866 2026-06-30)`.
- **Tool:** cargo-mutants 27.1.0, installed with `cargo install cargo-mutants --locked --version 27.1.0`. It is pinned to
  the version shards 0 to 3 report, so that every shard slices the same list.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 27/30 --jobs 2 --cargo-arg=--locked -- -- --exact \
    --skip <each of the 14 tests under "Exclusions">
  ```

  These are the same flags and the same 14 skips as shards 2 and 3. No `--test=` target selection is needed, because no
  mutant in this slice is in `core`.
- **Wall (MEASURED, outcomes.json):** 11:41:39Z to 12:16:34Z on 2026-11-16, so 34 min 55 s.
- **Test timeout:** cargo-mutants set it automatically to 20 s, its minimum. The baseline test phase took 3.9 s.
- **Exit code:** not captured, because the run was started with `nohup` in the background. The run completed:
  `522 mutants tested in 35m: 121 missed, 386 caught, 5 unviable, 10 timeouts`.

## What shard 27/30 covers (EXACT)

- `cargo mutants --workspace --list` gives 15,639 mutants: core 642, costs 1,312, greeks 345 and spikefade 13,340.
- Sharding is the default `slice`, and it is 0-indexed. So `--shard 27/30` is list positions 14,094 to 14,615, which is
  522 mutants. All of them are in `crates/spikefade/src/`.
  - `split18.rs`: the last 3 of its 1,031 mutants, all in `picks_tsv`. The slice boundary splits `split18.rs:1627:5`.
  - `sw33.rs`, `fix33.rs` and `fixg.rs`: every mutant (20, 216 and 203).
  - `ckmerge.rs`: 80 of its 144 mutants. The next slice starts at `ckmerge.rs:209:28` in `cmd_ckpt_merge`.

## Result (EXACT, from outcomes.json)

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| split18.rs | 0 | 3 | 0 | 0 | 3 |
| sw33.rs | 16 | 4 | 0 | 0 | 20 |
| fix33.rs | 151 | 53 | 10 | 2 | 216 |
| fixg.rs | 165 | 36 | 0 | 2 | 203 |
| ckmerge.rs | 54 | 25 | 0 | 1 | 80 |
| **all 5 files** | **386** | **121** | **10** | **5** | **522** |

Each mutant was tested against spikefade's own tests (`--package=spikefade@0.1.0`). On the unmutated tree that command
gives 158 passed, 1 ignored and 9 filtered out.

### Where the 121 missed mutants are

- **Argument parsers, 29:** `from_args` in `fix33.rs` (14), `fixg.rs` (13) and `sw33.rs` (2).
  - No test calls `fix33::from_args` or `fixg::from_args` directly. Their only call sites are `report3.rs:543` and `:545`.
- **The checkpoint merge, 25:** `ckmerge.rs` `merge` (15), `cmd_ckpt_merge` (7), `read_all` (2) and `bck_files` (1).
- **Text and flag helpers, 31:**
  - `fix33.rs` (20): `any`, `words`, `fp_part`, `level_words`, `vendor_of`, `vendor_of_text`, `vendor_words` and
    `cap_line`.
  - `fixg.rs` (9): `any`, `words`, `fp_part` and `win_line`.
  - `sw33.rs` (2): `any` and `words`.
  - 26 of the 31 replace the whole function body.
- **Other logic, 33:**
  - `fix33.rs` (19): `cap_minutes` (12), `StepSampler::want` (4), `StepSampler::push`, `StepAcc::merge` and
    `day_levels` (1 each).
  - `fixg.rs` (14): `IdxElm::parse` (5), `quanta`, `lot_mark` and `lot_line` (2 each), `IdxElm::at`, `IdxElm::load` and
    `idx_cover` (1 each).
- **`split18.rs` `picks_tsv`, 3.**
- The full list is in `missed.txt`.

### The 10 timeouts

7 are in `fix33.rs` `kth_union` (lines 170 to 176). The others are `cap_minutes` (354:15), a `<<` at 437:31 and
`StepAcc::merge` (449:33). Each mutant made the test run pass the 20 s limit.

### The 5 unviable

These mutants did not compile: `fix33.rs` 356:89 and 363:61 in `cap_minutes`, `fixg.rs` 278:9 (`IdxElm::parse`) and
318:9 (`IdxElm::at`), and `ckmerge.rs` 165:21 in `read_all`.

## Baseline (`baseline.log`)

1. **The literal `cargo test --locked --workspace`** stopped with exit 101 after 6.4 s (MEASURED), and 0 tests ran.
   - `core`'s lib test target `include_str!`s `CLAUDE.md`, `docs/00-charter.md`, `docs/05-decisions.md` and
     `docs/06-limits.md`. Those files are in the app repository, not on this branch.
2. **Step 2b:** `cargo build -p core --tests --keep-going` lists every `core` test target that cannot compile. They are
   the lib unit tests, `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`, the same 6 as
   shard 0. Step 2 itself only recorded a cargo usage error, because `cargo test` does not take `--keep-going`.
3. **Every target that compiles, with no exclusions** (steps 3a to 3e, `--no-fail-fast`):
   - costs: 428 passed, 2 ignored.
   - greeks: 59 passed.
   - spikefade: 158 passed, **9 failed**, 1 ignored.
   - core (`bound_probe`, `cost_invariants`, `lint`, `zerodha_index`): 9 passed, **5 failed**. Its 4 doc tests pass.
4. **With the 14 skips** (steps 4a to 4d): 0 failed in every package.
5. **Step 5:** the exact per-mutant test command of this shard passed on the unmutated tree: 158 passed, 0 failed,
   1 ignored, 9 filtered out. cargo-mutants' own baseline ran that command and passed (build 13.5 s, test 3.9 s).

## Exclusions (the 14 skips, the same list as shards 0 to 3)

| test | why it fails here |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | reads `crates/spikefade/data/EXITS-DYN-*.tsv`, which is not on this branch (README-CLOUD.md section 5) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs `/opt/homebrew/bin/zstd`, the Mac default (`kit.rs:470`). There is no zstd on this box, and `SF_ZSTD` is unset. |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_crate_carries_a_ratio_bench` (core `cost_invariants`) | `crates/spikefade/benches/ratio.rs` does not exist |
| `every_declared_cost_invariant_reaches_a_bench` | reads `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |

- The 5 `core` names match no spikefade test. They are passed so that every shard uses one list.
- Each failure above was checked against its panic message in `baseline.log`.
