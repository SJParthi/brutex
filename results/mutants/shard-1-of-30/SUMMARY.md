# Mutation shard 1/30: results (job MUT-1)

Code only. No market data was read, fetched or written. **EXACT** = a count. **MEASURED** = a wall time taken on this box.

## Source and tools

- Branch `cloud/spikefade-shard-20261116`, commit `15fd4f4`, a single root commit.
- `rustc 1.97.1 (8bab26f4f 2026-07-14)` and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`. Every cargo call in both runs below used the 1.97.1 toolchain; the per-mutant logs show its path.
- `cargo-mutants 27.1.0`, installed with `cargo install cargo-mutants --locked`.
- The box: 4 vCPU, 15 GiB RAM, Linux x86_64.

## What "shard 1/30" covers

- `cargo mutants --workspace --list` gives **15,639** mutants (EXACT): core 642, costs 1,312, greeks 345, spikefade 13,340.
- In cargo-mutants 27.1.0, shards are **0-indexed** and **sliced** by default. `--shard k/n` takes one contiguous range of the list, and `k` runs from 0 to n-1.
  - So `--shard 1/30` is the **second** slice: list positions 522 to 1043, **522 mutants**.
  - `--shard 30/30` is refused: "shard k must be less than n". **If the 30 boxes are numbered 1 to 30, slice 0/30 runs on no box.** That slice is the first 522 mutants, all of them in `core`.
- This slice holds two groups:
  - 120 mutants in `crates/core/src/vendor.rs`, the last 120 of its 156. The slice boundary splits `vendor.rs:455:20`.
  - 402 mutants in `crates/costs/src/`: `charge_error`, `dated`, `day`, `dhan_calc`, `equity_clock`, and `equity_dated` up to line 1118.

## Baseline and exclusions (see `baseline.log`)

1. **The literal `cargo test --locked --workspace` does not compile** (rc 101, 3 s). `core`'s test targets `include_str!` files that exist only in the app repository: `CLAUDE.md`, `docs/*.md`, `.github/*`, `deny.toml`, and ten other crates' `Cargo.toml`. README-CLOUD.md section 5 says the same: "The other crates' own tests need files of the app repository, so they are not run here".
   - **Excluded as whole targets**, because a test inside a binary that cannot compile cannot be skipped by name:
     - `core`'s lib unit tests;
     - `core`'s integration tests `banned_lists`, `claim_reader`, `decision_citations`, `findings` and `graph`.
2. **Every other target, with no exclusions** (`--no-fail-fast`): **658 passed, 14 failed, 3 ignored** (EXACT). Each failure is a file this repository does not hold:
   - `core` `tests/cost_invariants.rs`, all 5 of its tests:
     - 4 read `docs/04-invariants.md`;
     - `every_crate_carries_a_ratio_bench` fails because `crates/spikefade/benches/ratio.rs` does not exist. This branch adds `spikefade` to a crate set the test was written for.
   - `spikefade`, 7 tests that read `crates/spikefade/data/EXITS-DYN-*.tsv`, the three parameter tables README-CLOUD.md section 5 names:
     - `menu::tests::sf3_distance_floor_counted`, `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts`, `menu::tests::sf3_menu_fail_closed`;
     - `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry`, `menu::tests::sf3_tp_theta_level_and_absence`;
     - `tests33s::v33_search1d_sets_load_and_count`, `tests3::sf3_ttp_replaces_tp_close`.
   - `spikefade`, 2 tests that run the Mac default tool path `/opt/homebrew/bin/zstd` (`kit.rs:470`): `tests34::root_digest_same_for_every_form` and `tests34::sfz_roundtrip_all_rows`. zstd is not installed here, and `SF_ZSTD` is unset.
3. **With exactly those 14 skipped** (`-- --exact --skip <name>` x14): **658 passed, 0 failed, 3 ignored, 14 filtered out** (EXACT).
   - costs: 428 passed, 2 ignored. greeks: 59 passed. spikefade: 158 passed, 1 ignored, 9 excluded. core: 13 passed, 5 excluded.

## The run: `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`

```
 --workspace --shard 1/30 --jobs 2 --test-package costs --baseline=skip --timeout 60 --cargo-arg=--locked -- -- --exact --skip every_crate_carries_a_ratio_bench --skip every_declared_cost_invariant_reaches_a_bench --skip every_measured_cost_invariant_is_declared --skip the_document_declares_the_five_operations_rule_4_names --skip the_reconciliation_is_reading_something --skip menu::tests::sf3_distance_floor_counted --skip menu::tests::sf3_menu_and_sets_load_with_the_signed_counts --skip menu::tests::sf3_menu_fail_closed --skip menu::tests::sf3_tp_floor_not_armed_at_or_above_entry --skip menu::tests::sf3_tp_theta_level_and_absence --skip tests33s::v33_search1d_sets_load_and_count --skip tests3::sf3_ttp_replaces_tp_close --skip tests34::root_digest_same_for_every_form --skip tests34::sfz_roundtrip_all_rows
```

**Why `--test-package costs` and `--baseline=skip`.** By default each mutant is tested by its own package's tests, but `core`'s tests cannot compile here.

- **cargo-mutants' own baseline cannot pass on this slice.** In 27.1.0 it always builds and tests the mutated packages (`--package=core --package=costs`, in `lab.rs`), whatever `--test-package` says.
- **No single cargo invocation can split targets per package.** `--lib` cannot select costs' lib tests without also selecting core's, which do not compile.
- So **every mutant in this slice is tested by the `costs` package's tests** (all of its targets, doctests included):
  - For the 402 costs mutants, that is exactly the default.
  - For the 120 core mutants it is the only workable choice, and `costs` does not use `core::vendor` (see below).
- The baseline was skipped **only** because of the core problem above. It was run by hand instead: the exact per-mutant command, `cargo test --verbose --package=costs@0.1.0 --locked -- <the 14 skips>`, passed twice on the unmutated tree (428 passed, 2 ignored, 0 filtered, about 1.5 s; steps 3d and 3e in `baseline.log`).
  - The 14 skips match no costs test. They are passed so that every run carries the same exclusions.
- **Timeouts.** The test timeout is 60 s, set explicitly because the baseline was skipped. That is 40x the measured 1.5 s test phase, and 3x cargo-mutants' 20 s default floor. There is no build timeout, which is cargo-mutants' default.

**Result (EXACT, from `outcomes.json`): 522 tested: 267 caught, 123 missed, 7 timeout, 125 unviable.** Wall: 711 s (MEASURED, 10:30:30 to 10:42:21 UTC, 2026-11-16).

| file | caught | missed | timeout | unviable |
|---|---:|---:|---:|---:|
| `crates/core/src/vendor.rs` | 0 | 112 | 0 | 8 |
| `crates/costs/src/charge_error.rs` | 1 | 0 | 0 | 2 |
| `crates/costs/src/dated.rs` | 9 | 0 | 0 | 13 |
| `crates/costs/src/day.rs` | 131 | 0 | 7 | 54 |
| `crates/costs/src/dhan_calc.rs` | 88 | 0 | 0 | 13 |
| `crates/costs/src/equity_clock.rs` | 18 | 11 | 0 | 18 |
| `crates/costs/src/equity_dated.rs` | 20 | 0 | 0 | 17 |

### How to read the 123 missed

- **11 in `costs` are real gaps.** All are in `equity_clock.rs`'s *Measurement* section: `per_unit`, `synthetic_trade` and `measure_price_trade`. These are the timing helpers and the synthetic input behind `benches/ratio.rs` (C-FNO-COSTS-01..03); only that bench calls them, and no test pins their arithmetic.
  - Everything else this slice covers in `costs` has **no missed mutant**: `day.rs`, `dhan_calc.rs`, `dated.rs`, `charge_error.rs`, and `equity_dated.rs` up to line 1118.
- **112 in `core/src/vendor.rs` say nothing about core's test quality.**
  - `costs` does not import `core::vendor`, and its tests caught none of these 120. The tests that exercise `vendor.rs` are core's own: 60 unit tests inside it, which cannot compile here, and `tests/zerodha_index.rs`.
  - The follow-up below re-tests the same 120 mutants against core's own compilable targets.
- **The 7 timeouts** are all in `day.rs` date arithmetic (`days_from_civil`, `civil_from_days`, `TradeDay::before`). The mutant makes a test hang until the 60 s limit.

### The 11 missed in costs

```
crates/costs/src/equity_clock.rs:356:5: replace per_unit -> u64 with 0
crates/costs/src/equity_clock.rs:356:5: replace per_unit -> u64 with 1
crates/costs/src/equity_clock.rs:358:33: replace / with % in per_unit
crates/costs/src/equity_clock.rs:358:33: replace / with * in per_unit
crates/costs/src/equity_clock.rs:358:25: replace * with + in per_unit
crates/costs/src/equity_clock.rs:358:25: replace * with / in per_unit
crates/costs/src/equity_clock.rs:393:48: replace + with - in synthetic_trade
crates/costs/src/equity_clock.rs:393:55: replace % with / in synthetic_trade
crates/costs/src/equity_clock.rs:393:55: replace % with + in synthetic_trade
crates/costs/src/equity_clock.rs:435:40: replace + with - in measure_price_trade
crates/costs/src/equity_clock.rs:435:40: replace + with * in measure_price_trade
```

### The 7 timeouts

```
crates/costs/src/day.rs:292:22: replace < with <= in TradeDay::before
crates/costs/src/day.rs:504:35: replace - with / in days_from_civil
crates/costs/src/day.rs:504:25: replace + with - in days_from_civil
crates/costs/src/day.rs:537:19: replace - with / in civil_from_days
crates/costs/src/day.rs:539:40: replace + with * in civil_from_days
crates/costs/src/day.rs:539:19: replace - with / in civil_from_days
crates/costs/src/day.rs:540:49: replace - with / in civil_from_days
```

## Follow-up: the same 120 `vendor.rs` mutants against core's own compilable tests

These figures are **not** in `outcomes.json`. They come from a second run whose output was kept apart from it.

- Mutant selection: `-f crates/core/src/vendor.rs`, plus `-E` with the 36 shard-0 `vendor.rs` names, escaped and anchored.
  - Its `--list` was diffed against this slice's `vendor.rs` mutants: **identical, 120 of 120, in the same order**.
- Tests: `core`'s compilable targets `bound_probe`, `cost_invariants` (its 5 tests skipped), `lint` and `zerodha_index`, with the same 14 skips.
  - Left out: core's 4 doctests. None is in `vendor.rs`, and doctests cannot share one cargo call with `--test` selectors.
- cargo-mutants' own baseline ran and passed, and it auto-set the test timeout to 20 s.

```
cargo mutants --workspace -f crates/core/src/vendor.rs -E '<the 36 shard-0 names>' --jobs 2 --cargo-arg=--locked \
  --cargo-arg=--test=bound_probe --cargo-arg=--test=cost_invariants --cargo-arg=--test=lint --cargo-arg=--test=zerodha_index \
  -o <outside the checkout> -- -- --exact --skip <the same 14>
```

**Result (EXACT): 120 tested: 29 caught, 82 missed, 1 timeout, 8 unviable.** Wall: 53 s (MEASURED).

- Its 8 unviable mutants are the same 8 as in the main run.
- Of the main run's 112 `vendor.rs` misses:
  - 29 are caught by `zerodha_index`, and 1 times out;
  - **82 stay missed** even here.
- Even these 82 are only a lower bound on what core catches, because the 60 `vendor.rs` unit tests could not run. **Judge `vendor.rs` in the app repository**, where all of core's tests compile.

### Still missed by core's compilable tests (82)

```
crates/core/src/vendor.rs:455:20: replace > with >= in MasterRow<'_>::over_wide
crates/core/src/vendor.rs:797:9: replace Skip::reason -> &'static str with ""
crates/core/src/vendor.rs:797:9: replace Skip::reason -> &'static str with "xyzzy"
crates/core/src/vendor.rs:819:9: replace Skip::is_routine -> bool with true
crates/core/src/vendor.rs:819:9: replace Skip::is_routine -> bool with false
crates/core/src/vendor.rs:819:9: delete ! in Skip::is_routine
crates/core/src/vendor.rs:835:9: replace Skip::unread_code -> Option<&'a str> with None
crates/core/src/vendor.rs:835:9: replace Skip::unread_code -> Option<&'a str> with Some("")
crates/core/src/vendor.rs:835:9: replace Skip::unread_code -> Option<&'a str> with Some("xyzzy")
crates/core/src/vendor.rs:860:9: replace Skip::judges_the_paper -> bool with true
crates/core/src/vendor.rs:860:9: replace Skip::judges_the_paper -> bool with false
crates/core/src/vendor.rs:1276:17: delete match arm "IDX" | "EQ" | "FUT" | "CE" | "PE" in Vendor::type_of
crates/core/src/vendor.rs:1277:21: delete match arm "IDX" in Vendor::type_of
crates/core/src/vendor.rs:1278:21: delete match arm "EQ" in Vendor::type_of
crates/core/src/vendor.rs:1279:21: delete match arm "FUT" in Vendor::type_of
crates/core/src/vendor.rs:1280:21: delete match arm "CE" in Vendor::type_of
crates/core/src/vendor.rs:1286:17: delete match arm "INDEX" in Vendor::type_of
crates/core/src/vendor.rs:1287:17: delete match arm "EQUITY" in Vendor::type_of
crates/core/src/vendor.rs:1288:17: delete match arm "FUTSTK" | "FUTIDX" in Vendor::type_of
crates/core/src/vendor.rs:1289:17: delete match arm "OPTSTK" | "OPTIDX" in Vendor::type_of
crates/core/src/vendor.rs:1295:17: delete match arm "FUTCUR" | "OPTCUR" in Vendor::type_of
crates/core/src/vendor.rs:1290:21: delete match arm "CE" in Vendor::type_of
crates/core/src/vendor.rs:1291:21: delete match arm "PE" in Vendor::type_of
crates/core/src/vendor.rs:1307:17: delete match arm "FUT" in Vendor::type_of
crates/core/src/vendor.rs:1309:17: delete match arm "CE" in Vendor::type_of
crates/core/src/vendor.rs:1310:17: delete match arm "PE" in Vendor::type_of
crates/core/src/vendor.rs:1411:9: replace Vendor::is_known_foreign_exchange -> bool with true
crates/core/src/vendor.rs:1411:9: replace Vendor::is_known_foreign_exchange -> bool with false
crates/core/src/vendor.rs:1415:46: replace == with != in Vendor::is_known_foreign_exchange
crates/core/src/vendor.rs:1541:5: replace unread_master_code -> Option<(&'static str, &'a str)> with None
crates/core/src/vendor.rs:1541:5: replace unread_master_code -> Option<(&'static str, &'a str)> with Some(("", ""))
crates/core/src/vendor.rs:1541:5: replace unread_master_code -> Option<(&'static str, &'a str)> with Some(("", "xyzzy"))
crates/core/src/vendor.rs:1541:5: replace unread_master_code -> Option<(&'static str, &'a str)> with Some(("xyzzy", ""))
crates/core/src/vendor.rs:1541:5: replace unread_master_code -> Option<(&'static str, &'a str)> with Some(("xyzzy", "xyzzy"))
crates/core/src/vendor.rs:1541:34: replace || with && in unread_master_code
crates/core/src/vendor.rs:1541:67: replace != with == in unread_master_code
crates/core/src/vendor.rs:1610:19: replace match guard vendor.is_known_foreign_exchange(row.exchange) with true in decode_master_row
crates/core/src/vendor.rs:1610:19: replace match guard vendor.is_known_foreign_exchange(row.exchange) with false in decode_master_row
crates/core/src/vendor.rs:1619:45: replace || with && in decode_master_row
crates/core/src/vendor.rs:1807:9: delete match arm "EQ" in decode_master_row
crates/core/src/vendor.rs:1813:9: delete match arm "FUT" in decode_master_row
crates/core/src/vendor.rs:1806:17: replace match guard vendor.master_columns().listing_class.is_empty() with true in decode_master_row
crates/core/src/vendor.rs:1806:17: replace match guard vendor.master_columns().listing_class.is_empty() with false in decode_master_row
crates/core/src/vendor.rs:1865:25: replace match guard !vendor.master_columns().isin.is_empty() with true in decode_master_row
crates/core/src/vendor.rs:1865:25: delete ! in decode_master_row
crates/core/src/vendor.rs:1865:25: replace match guard !vendor.master_columns().isin.is_empty() with false in decode_master_row
crates/core/src/vendor.rs:1903:5: replace unsuffixed_key -> Result<Option<InstrumentKey>, InstrumentError> with Ok(None)
crates/core/src/vendor.rs:1904:33: replace || with && in unsuffixed_key
crates/core/src/vendor.rs:1904:17: replace != with == in unsuffixed_key
crates/core/src/vendor.rs:1914:9: delete field underlying from struct InstrumentKey expression in unsuffixed_key
crates/core/src/vendor.rs:1934:37: replace || with && in parse_expiry
crates/core/src/vendor.rs:1934:21: replace || with && in parse_expiry
crates/core/src/vendor.rs:1934:16: replace != with == in parse_expiry
crates/core/src/vendor.rs:1934:32: replace != with == in parse_expiry
crates/core/src/vendor.rs:1934:48: replace != with == in parse_expiry
crates/core/src/vendor.rs:1942:8: delete ! in parse_expiry
crates/core/src/vendor.rs:1942:49: replace || with && in parse_expiry
crates/core/src/vendor.rs:1942:54: replace == with != in parse_expiry
crates/core/src/vendor.rs:1954:39: replace + with - in parse_expiry
crates/core/src/vendor.rs:1954:39: replace + with * in parse_expiry
crates/core/src/vendor.rs:1954:34: replace * with + in parse_expiry
crates/core/src/vendor.rs:1954:34: replace * with / in parse_expiry
crates/core/src/vendor.rs:1954:53: replace - with + in parse_expiry
crates/core/src/vendor.rs:1954:53: replace - with / in parse_expiry
crates/core/src/vendor.rs:1955:55: replace + with - in parse_expiry
crates/core/src/vendor.rs:1955:55: replace + with * in parse_expiry
crates/core/src/vendor.rs:1955:50: replace * with + in parse_expiry
crates/core/src/vendor.rs:1955:50: replace * with / in parse_expiry
crates/core/src/vendor.rs:1955:60: replace - with + in parse_expiry
crates/core/src/vendor.rs:1955:60: replace - with / in parse_expiry
crates/core/src/vendor.rs:1956:53: replace + with - in parse_expiry
crates/core/src/vendor.rs:1956:53: replace + with * in parse_expiry
crates/core/src/vendor.rs:1956:48: replace * with + in parse_expiry
crates/core/src/vendor.rs:1956:48: replace * with / in parse_expiry
crates/core/src/vendor.rs:1956:58: replace - with + in parse_expiry
crates/core/src/vendor.rs:1956:58: replace - with / in parse_expiry
crates/core/src/vendor.rs:1978:5: replace parse_strike -> Result<Paisa, InstrumentError> with Ok(Default::default())
crates/core/src/vendor.rs:1979:23: replace match guard strike > Paisa::ZERO with true in parse_strike
crates/core/src/vendor.rs:1979:23: replace match guard strike > Paisa::ZERO with false in parse_strike
crates/core/src/vendor.rs:1979:30: replace > with == in parse_strike
crates/core/src/vendor.rs:1979:30: replace > with < in parse_strike
crates/core/src/vendor.rs:1979:30: replace > with >= in parse_strike
```

### Caught by core's compilable tests (29) and timeout (1)

```
crates/core/src/vendor.rs:455:20: replace > with < in MasterRow<'_>::over_wide
crates/core/src/vendor.rs:458:15: replace += with -= in MasterRow<'_>::over_wide
crates/core/src/vendor.rs:944:9: replace Decoded::skip -> Option<Skip> with None
crates/core/src/vendor.rs:977:14: replace == with != in collapse_spaces
crates/core/src/vendor.rs:987:13: replace += with -= in collapse_spaces
crates/core/src/vendor.rs:987:13: replace += with *= in collapse_spaces
crates/core/src/vendor.rs:1005:9: replace Collapsed::as_str -> &str with ""
crates/core/src/vendor.rs:1005:9: replace Collapsed::as_str -> &str with "xyzzy"
crates/core/src/vendor.rs:1274:9: replace Vendor::type_of -> Result<Option<&'static str>, InstrumentError> with Ok(None)
crates/core/src/vendor.rs:1274:9: replace Vendor::type_of -> Result<Option<&'static str>, InstrumentError> with Ok(Some(""))
crates/core/src/vendor.rs:1274:9: replace Vendor::type_of -> Result<Option<&'static str>, InstrumentError> with Ok(Some("xyzzy"))
crates/core/src/vendor.rs:1306:17: delete match arm "EQ" in Vendor::type_of
crates/core/src/vendor.rs:1364:9: replace Vendor::index_segment_word -> Option<&'static str> with None
crates/core/src/vendor.rs:1364:9: replace Vendor::index_segment_word -> Option<&'static str> with Some("")
crates/core/src/vendor.rs:1364:9: replace Vendor::index_segment_word -> Option<&'static str> with Some("xyzzy")
crates/core/src/vendor.rs:1481:9: replace Vendor::index_alias -> Option<&'static str> with None
crates/core/src/vendor.rs:1481:9: replace Vendor::index_alias -> Option<&'static str> with Some("")
crates/core/src/vendor.rs:1481:9: replace Vendor::index_alias -> Option<&'static str> with Some("xyzzy")
crates/core/src/vendor.rs:1483:17: delete match arm "NIFTY50" in Vendor::index_alias
crates/core/src/vendor.rs:1484:17: delete match arm "NIFTYBANK" in Vendor::index_alias
crates/core/src/vendor.rs:1507:9: replace Vendor::index_alias_source -> Option<&'static str> with None
crates/core/src/vendor.rs:1507:9: replace Vendor::index_alias_source -> Option<&'static str> with Some("")
crates/core/src/vendor.rs:1507:9: replace Vendor::index_alias_source -> Option<&'static str> with Some("xyzzy")
crates/core/src/vendor.rs:1509:17: delete match arm "NIFTY" in Vendor::index_alias_source
crates/core/src/vendor.rs:1510:17: delete match arm "BANKNIFTY" in Vendor::index_alias_source
crates/core/src/vendor.rs:1667:45: replace == with != in decode_master_row
crates/core/src/vendor.rs:1668:15: replace != with == in decode_master_row
crates/core/src/vendor.rs:1759:28: replace == with != in decode_master_row
crates/core/src/vendor.rs:1775:9: delete match arm "IDX" in decode_master_row
TIMEOUT crates/core/src/vendor.rs:458:15: replace += with *= in MasterRow<'_>::over_wide
```

## Files in this folder

| file | what it holds |
|---|---|
| `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt` | the main run, copied verbatim from `mutants.out/` |
| `baseline.log` | a summary header, then the raw output of the 3 baseline steps |
| `SUMMARY.md` | this file |
