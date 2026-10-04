# Pass 7: tests and house rules (final/all-fixes-zero @ 1f4de71)

**Verdict at 1f4de71:** the head is the same commit as in pass 6, so every finding still open from pass 6 is still open. The production code of the ten crates in scope (runner, engine, indicators, vocab, store, lake, costs, core, greeks, telemetry) breaks none of the CLAUDE.md §4 bans or the §7 money rule. Every Result that is turned into a default or a skip was traced, and either the failure is named somewhere else or the fallback is decided and tested.

Workspace-wide, I found no `#[test]` that asserts nothing outside the classes already reported. Two new tests fall short of their stated claim. Both are low severity.

**New findings: 2 (both low).** Known ids (P1-*, P4-*, P5-*, P6-*, CE-*, p6num-*) are not re-reported.

**Method (ran; cargo was not used, because nothing reached medium or high).**
- I wrote a Python scanner in my scratchpad, not tracked. It blanks out strings and comments, then removes every `#[cfg(test)]` item by brace matching. It drops `tests/`, `benches/`, `*_tests.rs` and `tests.rs` files, which are the `#[path]` and `mod tests;` modules. The result was 92,372 production lines from 145 `src` files in the ten crates.
- I grepped those production lines for these patterns and triaged every hit:

  | Pattern | Hits |
  |---|---|
  | `unwrap_or(` | 240 |
  | `unwrap_or_default` | 17 |
  | `unwrap_or_else(\|_` | 1 |
  | `.ok()?` | 39 |
  | `Err(_) =>` | 8 |
  | `let _ =` | 111 |
  | bare `.ok();` | 5 |
  | `map_err(\|_\|` | about 40 |
  | `if let Ok` / `is_ok()` | about 10 |
- Tests: I parsed all 610 tracked `.rs` files, which hold 6,941 `#[test]` and `#[tokio::test]` functions. Each body was searched for a strong assertion: `assert*!`, `panic!`, `unreachable!`, `expect_err`, `unwrap_err` or `catch_unwind`. Where a body had none, I followed the helpers it calls, first with crate scope and then with file scope, one level deep plus one more.
- I ran a second scan for every `return;` or `return Ok(())` inside a test body, recording its guard (58 sites). I also searched for `assert!(true)`, `assert_eq!(x, x)`, and `let _ =` in tests (774 sites, grouped by callee).

---

## Theme 1: tests that assert nothing (CLAUDE.md §4)

### Every test with no strong assertion after helpers are followed

| Test | Verdict |
|---|---|
| cli step3_orchestrator.rs:5277 `stored_public_projection_and_private_authority_keep_distinct_return_types` | **Holds.** It is a type-level assertion: the two `fn`-pointer coercions fail to compile if the signatures drift. |
| web/sweep-readiness/deployment-preflight-tests.rs:772 | **Out of scope.** It sits under `web/`, and `gate(&plan).unwrap()` is its success assertion. |
| store tests/durability.rs (4), store tests/catalog.rs (2), store catalog_tests.rs (2), store tests/write.rs:697, pull tests/folder.rs:609, pull tests/pipeline.rs (2), api folder.rs:500, telemetry sink.rs (4) | **Hold.** Each delegates to `where_permission_binds`. That helper re-executes the single test as uid 65534 when running as root, and asserts `success && stdout.contains("1 passed")` (store tests/support/mod.rs:62-65, and the same in pull, api and telemetry). So the "skip when root" shape does not occur, and a child that ran zero tests fails. |
| api census_request_tests.rs:2127, pull emit_sites.rs:1480 | **Hold.** Both use the same child-process helper pattern, with an assertion inside the helper. |
| cli index_stop_tests.rs:424 `catalog_attempt_throughput_measurement` | **Holds.** It is `#[ignore]` and documented as a measurement. Minor: it parses `BRUTEX_MEASURE_GROUP` into `_group` and never uses it. |
| cli ledger_v6.rs:1097, 1135; population_admission_writer.rs:2165; step3_comparison.rs:2072 | **Hold.** Each checks through `expect(..)` on a validator, or through `columns::assert_under`. |

### Early returns in tests (58 sites)

| Site | Verdict |
|---|---|
| core tests/findings.rs:588/687/769 `history_is_at` | **Holds now. P1-14-04 is FIXED** (findings.rs:458-480). It skips only when `git` cannot run or `.git` is absent. A failed `rev-parse` fails the test. |
| store tests/cited_commits.rs:232/675/763/922 | **Holds.** It skips only when `git --version` cannot run. |
| lake tests/real_lake.rs:170/202 | **Holds.** Both are `#[ignore = "needs ~/.brutex/lake…"]`. |
| lake tests/refusals.rs:1141/1158 | **Holds.** Only the extra real-lake census is skipped, and only after the injectivity and round-trip assertions have run. The comment says so. |
| store tests/docs.rs:225 | **Holds.** It is conditional on the superseded sentence still standing, which is the stated design. |
| store tests/fifo.rs:213 `/dev/null` absent | **Holds.** `/dev/null` always exists on CI runners. |
| runner exit_grid_policy.rs:5101-7448 (13 × `let Some(x) = x.ok() else { return }`) | **Hold.** Each is directly preceded by `assert!(x.is_ok())`. |
| api/cli `if let Some(root) = var_os(CHILD) { …; return }` (12) | **Hold.** These are child-process entry points. |
| **lake page.rs:708** | **P7-01** |
| **runner bootstrap.rs:2948** | **P7-02** |

### Tautology and `let _ =` scan

- `assert_eq!(f(), f())` sites:
  - runner synthetic.rs:117/359 are test-fixture generators.
  - indicators session.rs:1071 carries D-0373's non-constant guard.
  - engine column.rs:717 is guarded by its sibling `the_fold_is_ordered_so_moving_bars_moves_the_identity`.
  - runner tests/research_family_readiness.rs:220 is preceded by an independent-oracle `assert_eq!`.

  None of these is new.
- **P1-13-01 is FIXED.** evaluator.rs:2173-2178 now asserts `once.len() == bars.len()` and a non-constant guard, and column.rs:1759-1761 adds the same.
- `let _ =` in tests: 68 are `ok(..)` warm-up helpers that panic on refusal. The rest are cleanup, `write!`, `black_box`, or no-panic overflow probes (indicators lib.rs:1224-1226, under debug overflow checks). None discards the Result under test.

---

## Theme 2: fallbacks that hide a failure, and the other §4 bans (production code)

| Class / site | Failure still named loudly? |
|---|---|
| runner lib.rs:917 `auto_prepared(..).unwrap_or_else(\|_\| auto_memory_refusal)` | **Yes.** With the always-`Ok` reporter, the only `Err` is the support-column allocation (lib.rs:953). It becomes `Breach::Memory`, carried in `outcome.halted`. |
| engine lib.rs:1264/1394/1467/1761 `Err(_) =>` | **Yes.** All four are `TryReserveError` and become `halt(.., Breach::Memory)`. |
| indicators `set_exact/set_near(..).unwrap_or(mask)` (evaluator, pattern, fib, orb, vwap, daily, session, trend, lib: about 30 sites) | **Yes, by decision.** A refusal leaves the bit clear (docs/03 §4). Per-module position tests pin every index as live and of the right kind, and `Widths` (private fields, D-1546/D-1553) makes a WrongBand pair unbuildable outside the module. `a_mismatched_width_is_withheld_as_unknown_and_never_answered` pins the consequence. Stale comment: fib.rs:222-224 still lists the refusal causes without `WrongBand`. |
| indicators evaluator.rs:1278 `DailyLevels::from_previous_session(..).ok()` | **Partly.** The absence is visible through `has_yesterday`/`warmed_up`, but the reason is not. The code comment at :1268-1272 states this limit itself (§3 rule 6). It is only reachable where a session span leaves `i64`. |
| indicators trend.rs:439/465 `i64::try_from(stop).ok()` | **Yes.** It is documented: an absent stop reseeds on the next candle. |
| indicators fib.rs:168, vwap.rs:614 | **N/A.** These are exact arithmetic, not fallbacks. |
| runner outcome.rs:238 `square_off ... .ok()` | **Yes.** `None` refuses the forced exit and never saturates (comment at :230-232). |
| runner grid.rs:1783-1790, trade.rs:1344-1353 `.ok()?` | **Yes.** Already triaged in crash-edge pass 6. |
| runner grid.rs:1681/2151/2153/2164 `Ladder::from_excursions(..).unwrap_or_default()` | **N/A.** It returns `Option`, and `None` is the documented "nothing observed, no ladder". |
| runner validate.rs:3146/4351/4361, exit_grid_policy.rs:3860 `unwrap_or_default` inside identity hashes | **Yes.** Each is preceded by a presence tag byte (`u8::from(x.is_some())`) or a `debug_assert!` binding the tag, so `None` and `Some(0)` do not collide. |
| `try_from(len).unwrap_or(MAX)` (about 120 sites) | **N/A.** These are infallible on 64-bit targets. |
| runner grid.rs:660 `guaranteed_floor` and audit.rs:227: i128 money saturating to `i64::MAX/MIN` | **Documented** ("saturates rather than wrapping"). Unreachable from paisa series. |
| store header.rs:672 `best_candidate(..).ok()` | **Yes.** The newest slot's refusal is kept and returned (:662-674). |
| store file.rs:3311 / telemetry sink.rs:1485-1489 | **Known** (crash-edge pass 6, CE-51). |
| lake page.rs:396/405 `.ok()?` | **Yes.** The caller counts the page as unread. |
| lake reader.rs:104 `Err(_) =>` | **Yes.** It raises the event level to `Error` with "refused". |
| lake footer.rs:122 `checked_shl(shift).unwrap_or(0)` | **Low risk.** The 10th varint byte's high bits are dropped silently. This is a pre-parse guard, and `parquet` decodes and refuses on its own. Not a finding. |
| lake footer.rs:242 `i32::try_from(signed).unwrap_or(i32::MIN)` | **Documented.** An over-wide field id cannot match the schema id and is skipped. |
| costs day.rs:258 `Err(_) => None` | **N/A.** A `const` constructor over literals. |
| core vendor.rs:1470 `Err(_) => declined(UnrecognisedExchange)` | **Yes.** A distinct, named decline. |
| `let _ = write!/writeln!` (core instrument.rs, store catalog.rs, runner audit.rs ×56, report.rs ×45, identity.rs) | **N/A.** These write into a `String`, which is infallible. |
| `let _dropped_when_filtered = telemetry::emit(..)` | **Yes.** Sink write failures are counted in `health()` and reported once on stderr (sink.rs:1370-1390). |
| §4 query planner / ORM / dynamic schema | **None.** The lock has no diesel, sqlx, sea-orm, rusqlite, sled, polars, datafusion or arrow. |
| §4 writable memory mapping | **None.** `memmap2` is declared but unused, which is already noted in pass 6. |
| §4 depth parameter | **None.** The only `depth` arguments are parser recursion bounds (vocab expression.rs:588-610, telemetry json.rs:364) and `pattern::known_at`. |

## Theme 3: money (§7)

- The ten crates in scope have no `f32`. `f64` appears only in these places:
  - greeks: model math, under a crate-level float exception (greeks lib.rs:34-37).
  - runner statistics (outcome, bootstrap, significance, admission Wilson bound, grid Wilson bound): statistical values, which §7 allows.
  - store format.rs:537-549 and lake bar.rs:53-67: greeks columns, not prices.
  - core price.rs:113: `from_rupees_half_up`, the one designated conversion.
  - lake bar.rs:161 `paisa_from_lake`: the documented read boundary for the f64 lake. It delegates to `from_rupees_half_up` and refuses NaN, infinities and out-of-range values.
- runner outcome.rs:1224-1303 holds per-trade extremes as `f64` paisa (`min_win_paisa`, `max_loss_paisa`). These are converted from exact `i64` returns, exact below 2^53, and used for ranking only. This is the statistical carve-out, not money storage.
- Rounding: outside price.rs, the only `.round()` calls are statistic-to-display conversions (outcome.rs:1713 `milli`, report.rs:457 `paisa(mean)`). No price is re-snapped outside the write boundary.
- Nit, not a finding: the report.rs:438-442 doc says the conversion "saturates and says so", but it only saturates.

---

## New findings

### P7-01 low: `a_refused_page_writes_its_reason_to_the_log` passes without asserting whenever `telemetry::install` fails, and the reason its comment gives for that is impossible in its own binary
- **Where:** crates/lake/src/page.rs:702-709
  ```rust
  let Ok(sink) = telemetry::install(
      &telemetry::Config::new(&dir).with_min_level(telemetry::Level::Trace),
  ) else {
      // Another binary in this process installed first. ...
      return;
  };
  ```
- **Why it is wrong:**
  - This is the only `telemetry::install` call in the lake library test binary. `grep -rn telemetry::install crates/lake` hits only page.rs:702 and tests/events.rs:250. The test's own doc (page.rs:682-694) says events.rs runs as a separate process with a separate `OnceLock`.
  - So the `else` arm is reachable only through `config.refusal()` or a `Sink::open` error (telemetry lib.rs:174-183). In other words, it fires only when telemetry is broken or the temp directory is unusable. In that case the test, which is the only proof that the lake.page refusal event reaches the file, passes green.
  - The sibling in store does it correctly: store emits.rs:658-660 uses `.expect("nothing else in this test binary installs a sink")`. In cli ledger_all.rs:1472 and api emitted.rs:87, the install-or-adopt helpers call `telemetry::global().expect(..)` and never return.
- **Repro (not run):** make `Config::refusal` return `Some(..)` for a `Trace` floor, or point `temp_dir` at a read-only directory (`TMPDIR=/proc`). Then `cargo test -p lake --lib a_refused_page_writes_its_reason_to_the_log` reports `ok`, and none of its four assertions executes.
- **Minimal fix:** replace the `let … else { return }` with `.expect("the only sink installer in lake's lib test binary")`, as store/emits.rs does. Alternatively, use install-or-adopt and `expect` on `telemetry::global()`.

### P7-02 low: `the_surviving_set_strictly_shrinks_every_round_that_rejects` cannot fail against the implementation it guards, and passes vacuously if nothing is rejected
- **Where:** crates/runner/src/bootstrap.rs:2930-2963 (test); romano_wolf loop at :1549-1581
  ```rust
  let rejected = romano_wolf(&set, 150, 5, DEFAULT_BLOCK, 50_000);
  if rejected.is_empty() {
      return;
  }
  ...
  for round in 0..=rounds { let n = …count(); assert!(n > 0, …) }
  ```
- **Why it is wrong:**
  - The doc says "a test that merely finished would not say so. This one asserts the count." But `round` is incremented only after a non-empty `rejected_now` has been pushed (:1565-1580: `let Some(rejected_now) = …filter(|now| !now.is_empty()) else { break }`, then `round = round.saturating_add(1)`). So every round number in `0..=last` has at least one row by construction, and `n > 0` cannot fail.
  - The defect the doc names, a survivor pushed on both branches, shows up as a non-terminating loop. The test then hangs rather than fails, which is exactly the "merely finished" signal the doc disclaims.
  - Separately, a regression that makes `romano_wolf` reject nothing for this fixture takes the early `return` and passes green. Neither the test name nor any decision states that skip.
- **Repro (not run, from reading the source):** the `n > 0` assertion is true for every output this loop can produce, because each round is numbered only after a non-empty push. Make `romano_wolf` return `Vec::new()`, and this test passes. Other tests (e.g. :3092, :3210) would catch that particular mutant, so the gap is confined to this test's claim.
- **Minimal fix:**
  - Assert `!rejected.is_empty()` for this fixture, which has means of 0.75 to 121.75 against an SE of about 0.7, instead of returning.
  - Assert the property on the survivor sets rather than on the output's own labels: for example, that `rejected.len()` strictly increases across rounds and that `rounds + 1 <= set.len()`.
  - Alternatively, drop the "asserts the count" claim and name the termination guarantee as structural (`start = end` with `end > start`).

---

## Earlier findings re-checked at 1f4de71

| ID | Status | Evidence |
|---|---|---|
| P1-14-04 | FIXED | core tests/findings.rs:458-480 `history_is_at` now skips only when git is not runnable or `.git` is absent, and fails with git's words otherwise (the same rule as store cited_commits.rs:220-240). |
| P1-13-01 | FIXED | indicators evaluator.rs:2173-2178 now has `once.len() == bars.len()` plus the `later != earlier` guard. column.rs:1755-1761 adds a non-vacuous guard. |
| P4-01, P5-01, P6-01, P6-02, P6-03, P6-04 | NOT FIXED | The head is unchanged since pass 6 (1f4de71), so the pass-6 evidence stands. |
