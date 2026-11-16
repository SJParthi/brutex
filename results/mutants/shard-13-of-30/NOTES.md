# Mutation testing: shard 13 of 30 (job MUT-13)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. Code only: no market
  data, none of the 3 parameter tables, nothing fetched.
- **Box:** a Linux x86_64 cloud container, 4 vCPU (Intel Xeon @ 2.10GHz), 15.7 GiB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`
  and `cargo 1.97.1 (c980f4866 2026-06-30)`, pinned by `rust-toolchain.toml`. cargo-mutants 27.1.0, installed with
  `cargo install cargo-mutants --locked`.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 13/30 --jobs 2 --cargo-arg=--locked \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

- **Each mutant** ran `cargo test --no-run --verbose --package=spikefade@0.1.0 --locked`, then
  `cargo test --verbose --package=spikefade@0.1.0 --locked -- --exact <the 14 skips>` (read from `mutants.out/log`).
- **cargo-mutants' own baseline** passed: 15 s build + 4 s test, 158 passed, 1 ignored, 9 filtered out. It auto-set the test
  timeout to 25 s. There is no build timeout, which is the 27.1.0 default.
- **Wall (MEASURED):** 2026-11-16 11:38:45Z to 12:19:08Z, so 40 min 23 s.
- **Exit code 3:** one mutant timed out. In 27.1.0 a timeout outranks missed mutants when the exit code is chosen.

## What shard 13/30 covers (EXACT)

- `cargo mutants --workspace --list` gives 15,639 mutants. Default `slice` sharding gives 522 per shard, so shard 13 is list
  positions 6,786 to 7,307 (0-based).
- **All 522 are in `crates/spikefade`.** So every mutant here is tested by `spikefade`'s own tests, and `core`'s test targets
  that cannot compile (see "Exclusions") are never built.
  - `src/v2.rs`: 178 mutants, the last 178 of its 296, from `270:5` (`raw_1m`) to `547:5` (`write_new`). The boundary with
    shard 12 splits the `raw_1m` return-value mutants at `270:5`.
  - `src/kern.rs`: 344 mutants, the first 344 of its 634, from `147:5` (`maps`) to `653:51` (`step3g`). The boundary with
    shard 14 splits `653:51`.

## Result (EXACT, from `outcomes.json`)

| file (`crates/spikefade/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| v2.rs | 10 | 168 | 0 | 0 | 178 |
| kern.rs | 257 | 73 | 1 | 13 | 344 |
| **both** | **267** | **241** | **1** | **13** | **522** |

## Read this before using the CAUGHT count: 15 catches come from a timing assert

- **15 of the 267 caught mutants failed one test only,** `split18::tests::v32rb_split_o1_gates`, and only on its wall-clock
  asserts: per-close p99 under 20 us and per-account p99 under 2 ms, timed with `Instant::now()` in a debug build while the
  other job compiled. A typical failure: `97 months: account p99 4049740 ns`.
  - **10 are in `v2.rs`.** `split18.rs` names nothing in `v2`; the only file that names `v2::` is `report.rs`. So the timed
    code cannot reach these 10 mutants, and these failures are not detections.
  - **5 are in `kern.rs`:** `159:59` twice (`month_ok`), `166:22` (`close_month`), `246:12` (`Pieces::group_end`) and `313:23`
    (`flush`). `split18.rs` does call `kern::month_ok`, so for the two `month_ok` mutants the code alone cannot rule out that
    the mutant changed the timed path.
- **Follow-up (MEASURED, kept apart from `outcomes.json`, in `followup-skip-o1-gate/`):** the same 15 mutants were re-tested
  with that one test skipped as well. Result: **15 tested, 15 missed.** Wall: 12:21:17Z to 12:22:56Z, so 1 min 39 s.
  - They were selected with `-f crates/spikefade/src/v2.rs -f crates/spikefade/src/kern.rs` and one anchored, escaped
    `--re` per name. Its `--list` was identical to the 15.
  - Everything else matched the main run: the same 14 skips plus `--skip split18::tests::v32rb_split_o1_gates`,
    `--jobs 2 --cargo-arg=--locked`. cargo-mutants' own baseline passed and auto-set the timeout to 23 s.
- **Counted by functional tests only: 252 caught, 256 missed, 1 timeout, 13 unviable.**
- **No other catch rests on a timing assert alone.** All 267 logs were scanned for their panic messages. Every other caught
  mutant has at least one failure whose message is not a p99 timing message.
  - Two other tests that failed in this run also time code: `tests32r::v32r_o1_decision_at_n100_and_sampled_steps` and
    `tests33f::v33f_day_major_step_sampled`. Every failure they recorded is a functional assert, an overflow panic or a bounds
    panic, never a timing assert.
  - The timed code of `v32rb_split_o1_gates` cannot reach `v2.rs` through `report.rs` either: `split18.rs` names no
    `report` item.

### The timeout

`crates/spikefade/src/kern.rs:355:18: replace -= with /= in flush`: its test phase ran past 25 s.

### How to read the missed

- **`v2.rs`: no spikefade test detects any of its 178 mutants here** (168 missed plus the 10 timing catches above).
  - `v2.rs` holds no `#[test]`.
  - Missed by function: `blocks_md` 47, `evidence_md` 57, `raw_1m` 12, `remark_peak` 39, `rows_in` 7, `usable_words` 5,
    `write_new` 1.
- **`kern.rs`: 73 missed in the run, 78 counting the 5 timing catches.**
  - Missed by function: `step3g` 18, `unrealised` 11, `Pieces::realize` 7, `decide` 7, `flush` 7, `Pieces::group_end` 6,
    `close_month` 5, `close_day_p` 3, `wf19_on` 3, `close_day` 2, `Pieces::on` 1, `finish_pieces` 1, `month_ok` 1,
    `verdict_ids` 1.
  - The full list is `missed.txt`.

## Baseline (`baseline.log`)

The log holds the literal command first (step 2a), then labelled steps 2b to 2e.

- **2a, as instructed: `cargo test --locked --workspace`.** It exited 101 after about 6 s (MEASURED, 11:35:08Z to 11:35:14Z),
  and 0 tests ran. Cargo stops at the first target that fails to compile, here `core` (lib test): its unit tests
  `include_str!` `docs/05-decisions.md`, `docs/06-limits.md`, `docs/00-charter.md` and `CLAUDE.md`, which this branch does not
  hold.
- **2b: which `core` targets cannot compile.** Step `2b` itself is void: `cargo test` rejects `--keep-going`. Steps `2b-redo`
  and `2b-target` list the six targets in the table under "Exclusions".
- **2c: every target that compiles, no exclusions, `--no-fail-fast` (MEASURED):**

  | package | passed | failed | ignored |
  |---|---:|---:|---:|
  | core (`bound_probe`, `cost_invariants`, `lint`, `zerodha_index` and 4 doc tests) | 13 | 5 | 0 |
  | costs | 428 | 0 | 2 |
  | greeks | 59 | 0 | 0 |
  | spikefade | 158 | 9 | 1 |

- **2d: the same targets with the 14 exclusions (MEASURED): 0 failed everywhere.** core: 13 passed, 5 filtered out. costs:
  428 passed, 2 ignored. greeks: 59 passed. spikefade: 158 passed, 1 ignored, 9 filtered out.
- **2e: the exact per-mutant shape, run twice:** `cargo test --locked --package=spikefade -- --exact <the 14 skips>`. Both runs
  gave 158 passed, 0 failed, 1 ignored and 9 filtered out, in 4.8 s and 4.6 s wall (MEASURED).

## Exclusions

**The same 14 tests as shards 0 to 3, unchanged, so every shard carries one list.** Only the 9 `spikefade` ones can match
anything in this shard. The 5 `core` ones match no `spikefade` test.

### Tests that fail at run time (skipped by exact name)

| test | why |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | needs `crates/spikefade/data/EXITS-DYN-*.tsv` (README-CLOUD.md section 5) |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs zstd at the Mac default `/opt/homebrew/bin/zstd` (`kit.rs:470`). This box has no zstd and `SF_ZSTD` is unset |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core `cost_invariants`) | needs `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | needs `crates/spikefade/benches/ratio.rs` |

**Judgement call, carried over from shards 0 to 3:** the two `tests34` tests need a tool file absent from this box, not a
file of the repository.

### `core` test targets that cannot compile here (never built in this shard)

Every one fails on `include_str!` of a file of the app repository that this branch does not hold (steps 2a, 2b-redo,
2b-target).

| `core` target | missing file(s) |
|---|---|
| lib unit tests (`src/**`) | `docs/00-charter.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `CLAUDE.md` |
| `tests/banned_lists.rs` | `.github/workflows/ci.yml`, `deny.toml`, `crates/vocab/tests/workspace_is_rust.rs` |
| `tests/claim_reader.rs` | `.github/workflows/ci.yml`, `.github/gates_bounds.rs` |
| `tests/decision_citations.rs` | `docs/05-decisions.md` |
| `tests/findings.rs` | `docs/11-findings.md` |
| `tests/graph.rs` | `docs/01-architecture.md`, and `crates/<api,cli,engine,indicators,lake,pull,runner,store,telemetry,vocab>/Cargo.toml` |

## Files

- `outcomes.json`, `caught.txt`, `missed.txt`, `timeout.txt`, `unviable.txt`: cargo-mutants' own output from the main run,
  unedited.
- `baseline.log`: steps 2a to 2e above.
- `followup-skip-o1-gate/`: the same five files from the 15-mutant follow-up.
