# Mutation testing: shard 0 of 30 (job MUT-0)

Labels follow README-CLOUD.md. MEASURED means a number from a named run on this box. EXACT means a count. No figure here is a
trading result.

## Run

- **Source:** branch `cloud/spikefade-shard-20261116` at `15fd4f4737ea11bcef398efa75db240d38128d10`. Code only: no market
  data, and none of the 3 parameter tables.
- **Box:** a Linux x86_64 cloud container with 4 vCPU and 15 GB RAM. `rustc 1.97.1 (8bab26f4f 2026-07-14)`, cargo-mutants 27.1.0.
- **Command,** run from the checkout root:

  ```sh
  cargo mutants --workspace --shard 0/30 --jobs 2 --cargo-arg=--locked \
    --cargo-arg=--test=bound_probe --cargo-arg=--test=cost_invariants \
    --cargo-arg=--test=lint --cargo-arg=--test=zerodha_index \
    -- -- --exact --skip <each of the 14 tests listed under "Exclusions">
  ```

- **Wall (MEASURED):** 2026-11-16 10:29:52Z to 10:34:13Z, so 4 min 21 s.
- **Exit code 3:** at least one mutant timed out. In 27.1.0 a timeout outranks missed mutants when the exit code is chosen.

## Result (EXACT)

Shard 0 is the first 522 of the workspace's 15,639 mutants under the default `slice` sharding, and every one of them is in
`crates/core`. The workspace total by package is core 642, costs 1,312, greeks 345 and spikefade 13,340.

| file (`crates/core/src/`) | caught | missed | timeout | unviable | total |
|---|---:|---:|---:|---:|---:|
| alias.rs | 0 | 59 | 0 | 40 | 99 |
| blake3.rs | 0 | 96 | 0 | 2 | 98 |
| error.rs | 0 | 3 | 0 | 0 | 3 |
| instrument.rs | 1 | 63 | 0 | 7 | 71 |
| isin.rs | 0 | 35 | 0 | 1 | 36 |
| knob.rs | 0 | 7 | 0 | 0 | 7 |
| price.rs | 0 | 42 | 0 | 3 | 45 |
| secret_shape.rs | 0 | 49 | 0 | 1 | 50 |
| symbol.rs | 2 | 5 | 0 | 0 | 7 |
| universe.rs | 12 | 37 | 1 | 20 | 70 |
| vendor.rs | 7 | 28 | 0 | 1 | 36 |
| **all 11 files** | **22** | **424** | **1** | **75** | **522** |

## Read this before using the MISSED count

cargo-mutants 27.1.0 tests each mutant only against its own package's tests, which here means `core`'s. On this branch only
9 of `core`'s tests can run: `bound_probe` (2), `lint` (2) and `zerodha_index` (5).

`core`'s 205 unit tests and 38 of its integration tests cannot compile here, because they `include_str!` files of the app
repository that this branch does not hold (see "Exclusions"). So **424 missed means "not caught by those 9 tests"**. It is
not a measurement of `core`'s own test suite, which never ran.

The 22 caught and 1 timeout are real catches by those 9 tests.

## Baseline

### As instructed: `cargo test --locked --workspace` (`baseline.log`)

- Exit 101 after 2.2 s (MEASURED), and 0 tests ran.
- Cargo stops at the first target that fails to compile: `core` test `claim_reader`, which cannot read
  `.github/workflows/ci.yml` and `.github/gates_bounds.rs`.

### Every target that compiles, run with `--no-fail-fast` (MEASURED)

| package | passed | failed | ignored | targets that fail to compile |
|---|---:|---:|---:|---|
| core | 13 | 5 | 0 | 6: the lib unit tests, banned_lists, claim_reader, decision_citations, findings, graph |
| costs | 428 | 0 | 2 | none |
| greeks | 59 | 0 | 0 | none |
| spikefade | 158 | 9 | 1 | none |

The core row covers `bound_probe`, `cost_invariants`, `lint`, `zerodha_index` and 4 doc tests.

### With the exclusions (MEASURED, 0 failed everywhere)

| run | passed | failed | ignored | skipped by name | wall |
|---|---:|---:|---:|---:|---:|
| core, shard 0's exact command | 9 | 0 | 0 | 5 | 0.16 s |
| costs | 428 | 0 | 2 | 0 | 1.8 s |
| greeks | 59 | 0 | 0 | 0 | 0.7 s |
| spikefade | 158 | 0 | 1 | 9 | 5.0 s |

cargo-mutants' own baseline ran shard 0's exact command and passed.

## Exclusions

### Test targets that cannot compile here (excluded through `--test` target selection)

Every one fails on `include_str!` of a file of the app repository that this branch does not hold.

| `core` target | `#[test]` count | missing file(s) |
|---|---:|---|
| lib unit tests (`src/**`) | 205 | `docs/00-charter.md`, `docs/05-decisions.md`, `docs/06-limits.md`, `CLAUDE.md` |
| `tests/banned_lists.rs` | 2 | `.github/workflows/ci.yml`, `deny.toml`, `crates/vocab/tests/workspace_is_rust.rs` |
| `tests/claim_reader.rs` | 3 | `.github/workflows/ci.yml`, `.github/gates_bounds.rs` |
| `tests/decision_citations.rs` | 4 | `docs/05-decisions.md` |
| `tests/findings.rs` | 14 | `docs/11-findings.md` |
| `tests/graph.rs` | 15 | `docs/01-architecture.md`, and `crates/<api,cli,engine,indicators,lake,pull,runner,store,telemetry,vocab>/Cargo.toml` |

**Side effect:** `core`'s 4 doc tests do not run either. They pass, but cargo cannot combine `--doc` with `--test`.

### Tests that fail at run time (skipped by exact name)

| test | why |
|---|---|
| `menu::tests::sf3_menu_and_sets_load_with_the_signed_counts` | needs `crates/spikefade/data/EXITS-DYN-*.tsv` |
| `menu::tests::sf3_menu_fail_closed` | the same |
| `menu::tests::sf3_distance_floor_counted` | the same |
| `menu::tests::sf3_tp_floor_not_armed_at_or_above_entry` | the same |
| `menu::tests::sf3_tp_theta_level_and_absence` | the same |
| `tests33s::v33_search1d_sets_load_and_count` | the same |
| `tests3::sf3_ttp_replaces_tp_close` | the same |
| `tests34::root_digest_same_for_every_form` | runs zstd at the Mac default `/opt/homebrew/bin/zstd` (`kit.rs:470`); no zstd on this box, `SF_ZSTD` unset |
| `tests34::sfz_roundtrip_all_rows` | the same |
| `every_declared_cost_invariant_reaches_a_bench` (core `cost_invariants`) | needs `docs/04-invariants.md` |
| `the_document_declares_the_five_operations_rule_4_names` | the same |
| `every_measured_cost_invariant_is_declared` | the same |
| `the_reconciliation_is_reading_something` | the same |
| `every_crate_carries_a_ratio_bench` | needs `crates/spikefade/benches/ratio.rs` (see the judgement call below) |

The first 7 are the 7 that README-CLOUD.md section 5 predicts. The spikefade skips change nothing for shard 0, which tests
`core` only. They are passed so that every run uses the same exclusion list.

**Judgement call:** `every_crate_carries_a_ratio_bench` is the app repository's rule that every crate under `crates/` has
`benches/ratio.rs`. It fails because `crates/spikefade` was copied in without one. That is a fact about how this branch is
put together, not about the environment.
