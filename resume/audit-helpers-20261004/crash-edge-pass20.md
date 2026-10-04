# Crash/edge pass 20 (ce20): every lint exception in production code

Head audited: `/home/claude/wt/zero3` at 1f4de71 (detached, clean). Audit only: nothing was edited, and cargo was not run.

## Method

- The workspace lints (Cargo.toml `[workspace.lints.clippy]`) deny `unwrap_used`, `expect_used`, `panic`, `indexing_slicing`, `float_arithmetic`, `cast_possible_truncation`, `cast_sign_loss`, `todo` and `unimplemented`. The release profile sets `overflow-checks = true` and `panic = "abort"`.
- **The dev profile matters as well** (Cargo.toml:130-152). The operator's IntelliJ Run button builds `target/debug/api`, so the repository made `dev` opt-level 3. That keeps `debug_assertions` ON in the binary the operator actually runs.
- A scanner (scratchpad `ce20/scan.py`) listed every `#[allow(..)]`, `#[expect(..)]`, `#![allow(..)]` and `#![expect(..)]` under `crates/**/src`. It removed anything inside a `#[cfg(test)]` item and anything in a file reachable only through a `#[cfg(test)]` `mod` (including the `#[path]` test files and `#![cfg(test)]` files).
- **The scan found 333 production exceptions: 105 allow and 228 expect.**
  - 19 are module-level. No crate root (`lib.rs`/`main.rs`) carries any allow.
  - 31 give no `reason =`, but nearly all of those have a comment above them.
- Every exception that touches a denied crash or precision lint was read by hand. There are 133 of them: truncation 46, precision 32, indexing 31, float 26, sign 8 and expect 2, with some exceptions naming more than one.
- Every decision ID cited in a reason, or in the comment directly above one, was checked against `docs/05-decisions.md` headings. 28 sites cite one, and **all are real and on topic**. Test names cited in reasons were checked to exist; the only misses are historical references or field names.
- The other checks:
  - `unreachable!`, `unimplemented!`, `todo!`, `get_unchecked`, `unsafe` and `std::process::exit` / `abort` have **no** production occurrences. Every crate root has `#![forbid(unsafe_code)]`, and both `main`s return `ExitCode`, so Drop runs.
  - Runtime `assert!`: all are in `const` blocks except `cli::candidate_universe::put_bytes` (candidate_universe.rs:6017). Every caller passes a constant offset, so it holds.
  - `debug_assert!`: 23 sites, reviewed below.

## Findings (new; not CE-74 or anything else from pass 13)

### CE-95 (low): five module-wide `#![expect(dead_code)]` exceptions give reasons that stopped being true. Because they cover whole modules, dead production code added to these modules goes unnoticed.
- crates/cli/src/execution_v3.rs:38 `reason = "Execution V3 remains crate-private until Selection V5 consumes its source-retaining production capability"`. selection_v5.rs:54 already has `use crate::execution_v3::{..}`.
- crates/cli/src/population_v5.rs:28 `"... remain crate-private until the all-rung coordinator consumes their authenticated authorities"`. ledger_all.rs:87 and all_rung_population_v5.rs consume them.
- crates/cli/src/global_replay_v3.rs:40 `"... until the Step-3 orchestrator moves ..."`. step3_orchestrator.rs already imports `global_replay_v3`.
- crates/cli/src/population_finalization_v4.rs:25 `"... awaits its non-test all-rung caller"`. ledger_v6.rs:59 and population_v6.rs:47 are that caller.
- crates/cli/src/population_finalization_v3.rs:34 `"... still blocked on Population V4"`. population_v5.rs:56 and step3_orchestrator.rs:63 import it.

**Why it is wrong.** Compare population_v6.rs:20 and execution_v4.rs:41, which were updated to "the production caller exists; some items are still reached only from tests". These five still say "not wired yet". A reader is therefore told the whole module is unreachable, when it is on the production path. And because the exception covers the whole module, a production function that stops being called (for example a verify step that was dropped) raises no warning in these five modules.

**Repro:** not run. The evidence is the source greps above.

**Fix:** reword the five reasons as population_v6's is. Better, narrow each to `#[expect(dead_code)]` on the specific test-only items.

### CE-96 (low): `runner::outcome`'s module-wide float exception says only the mean and t are floating. The module carries money as f64.
- crates/runner/src/outcome.rs:48 `#![allow(clippy::float_arithmetic, reason = "... Returns are paisa i64; only the mean and t-statistic are floating.")]`
- But `Edge` (outcome.rs:1216-1267) has `pub mean_paisa: f64`, `win_sum: f64`, `adverse_sum: f64`, `favourable_sum: f64` and `loss_sum: f64`.
- `fn wide(paisa: i128) -> f64` (outcome.rs:1829) exists to put paisa into them, and its own doc says "`Edge`'s money fields stay `f64`" (D-1173).
- The payoff ratio at outcome.rs:1407 divides two f64 paisa magnitudes.
- grid.rs:449-452 repeats the claim to justify scoping its own exception narrowly: "`significance`, `bootstrap` and `outcome` ... none of them ever sees a price".

**Why it is wrong.** CLAUDE.md §7 bans floats for money. The module-wide allow turns `float_arithmetic` off for the whole 2,400-line file, on a premise that file contradicts. As a result, a new float price computation there would not be linted. (The f64 sums themselves are D-1173's deliberate, append-only choice; the defect is the exception's scope and stated reason.)

**Repro:** not run (source read).

**Fix:** correct both comments. Scope the allow to the functions that compute statistics: `Edge` accumulation, `t` and `milli`.

### CE-97 (low): `pull::tenor::years` claims to be THE ONE FLOAT BOUNDARY in `pull`. `pricing.rs` takes a second one.
- crates/pull/src/tenor.rs:298 `reason = "THE ONE FLOAT BOUNDARY IN THIS CRATE, and the lint is right to make it argue for itself ..."`
- crates/pull/src/pricing.rs:251-259 `#[expect(clippy::float_arithmetic, reason = "negating a bound ...")] let band = -MAX_PLAUSIBLE_RATE..=MAX_PLAUSIBLE_RATE;`

**Why it is wrong.** A reviewer who audits `pull`'s float surface by trusting the tenor reason misses the pricing exception. The pricing exception is harmless: a unary negation of a constant.

**Repro:** not run. `grep -rn float_arithmetic crates/pull/src` shows two `#[expect]` sites.

**Fix:** write `-MAX_PLAUSIBLE_RATE` as a negative literal constant, as runner/report.rs:456 does for `LOWER`, and delete the exception. Or reword the tenor reason.

### CE-98 (low): three `debug_assert!`s in `api` assert that a log write succeeded. In the operator's dev-profile binary, a failed log append panics the pull before the failure is recorded.
```
crates/api/src/server.rs:7194  debug_assert!(noted.is_written() || telemetry::global().is_none(), ...);
                     7199      site.autopilot.fail(&failure.instrument, month, &failure.why);   // after the assert
crates/api/src/server.rs:9093  debug_assert!(noted.is_written() || ..., "the vendor's reason is the one field ...");
crates/api/src/server.rs:10182 debug_assert!(emitted.is_written() || ..., ...);
```

**Why it is wrong.**
- `Emitted::Dropped` is an environmental outcome, not an invariant. The sink returns it whenever the append fails (telemetry sink.rs:1255-1273) and counts it in `Sink::health().dropped`.
- Release builds ignore it, which is correct. But Cargo.toml:131-152 records that the operator runs `target/debug/api` (the dev profile, which has debug assertions on).
- In that build, a log volume that is full or erroring makes `note_member_failure` panic before `site.autopilot.fail` runs. That function's caller (server.rs:7640-7645) exists "so only naming the first would [not] hide the disk".
- The same panic also aborts the 9093 path before the vendor's refusal text is formatted into the run's answer, and aborts the broker loop at 10182.
- The most likely moment for the log append to fail is a full disk, which is also the moment the event exists for. In this build the full disk produces a panicked task instead of a named failure. CLAUDE.md §4 says "degrade loudly and name the reason"; here an observable failure is replaced by a crash.

**Repro:** not run. Expected repro: run the dev-profile api with the log directory on a full tmpfs and pull a month whose store write fails. The task panics at server.rs:7194.

**Fix:** delete the three `debug_assert!`s, or replace them with a non-panicking path, since the drop is already counted in `Sink::health`. For example, append "(log write dropped)" to the autopilot note.

## debug_assert review (23 sites)

| site | verdict |
|---|---|
| api/detail.rs:84 `previous > 0` in Permit::drop | holds: Drop runs once per constructed permit |
| api/server.rs:7194, 9093, 10182 | **CE-98**: asserts an environmental condition |
| cli/population_admission_v4.rs:1791, cli/population_base_evidence_v2.rs:1430 `require_remaining_zero` | holds: fresh zeroed buffers |
| cli/results.rs:429, cli/trades.rs:236 `at == PAYLOAD_BYTES` | holds: fixed field list. An overrun would panic on indexing anyway |
| indicators/evaluator.rs:565 `encoded.next().is_none()` | holds: `Calendar.days` is `[i64; 9]`, so the length is fixed at 163 |
| pull/ingest.rs:3323 durable_through | holds: `sync_data` flushes the whole file regardless |
| runner/admission.rs:2276, 2281, 2301, 2362, 2495, 5961 | holds: every `put_slice` is a fixed array into a const-length buffer |
| runner/admission.rs:3254, 3576 | holds: literal arrays |
| runner/exit_grid_policy.rs:1805, 3850 | holds: both callers construct the matching tag and bits |
| runner/lib.rs:506, 557 `considered == sweep.streamed` | holds: engine lib.rs:997-1000 calls `on_retire` and bumps `streamed` in the same function |
| telemetry/encode.rs:171 | holds: `Utf8Sink::write_str` is infallible |

## Verification tally
No prior IDs were assigned to this pass for verification: FIXED 0, PARTIAL 0, NOT FIXED 0, WRONG FIX 0.

## Every production lint exception, with its verdict (333 rows)
In the verdict column, STYLE means a pedantic or structure lint (`too_many_lines`, `too_many_arguments`, `struct_field_names`, and so on) that cannot panic, truncate or wrap. "reason given = no" means no `reason =` string; most of those sites carry a comment instead.

| # | site | kind | lints | reason given | verdict |
|---|---|---|---|---|---|
| 1 | crates/api/src/backtest.rs:526 | #[expect] | indexing_slicing | yes | HOLDS; reason overstates: FIELD_SUM is a hand list, not tied to the `take` sequence (tests hold it) |
| 2 | crates/api/src/backtest.rs:1065 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 3 | crates/api/src/bars.rs:192 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 4 | crates/api/src/bars.rs:310 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 5 | crates/api/src/bars.rs:1023 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 6 | crates/api/src/calendar.rs:403 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 7 | crates/api/src/calendar_of.rs:2078 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 8 | crates/api/src/census.rs:580 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 9 | crates/api/src/frontierjson.rs:105 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 10 | crates/api/src/frontierjson.rs:459 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 11 | crates/api/src/recovery.rs:34 | #[expect] | needless_pass_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 12 | crates/api/src/server.rs:4443 | #[allow] | expect_used | yes | HOLDS: note_alphabet emits only 0x20..=0x7E |
| 13 | crates/api/src/server.rs:11873 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 14 | crates/api/src/server.rs:12731 | #[allow] | too_many_arguments | no (comment/none) | STYLE: cannot panic, truncate or wrap |
| 15 | crates/api/src/server.rs:12732 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 16 | crates/api/src/server.rs:13262 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 17 | crates/api/src/server.rs:13611 | #[expect] | float_arithmetic,cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 18 | crates/api/src/server.rs:13929 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 19 | crates/api/src/server.rs:14092 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 20 | crates/api/src/server.rs:14207 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 21 | crates/api/src/server.rs:16503 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 22 | crates/api/src/server.rs:19040 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 23 | crates/api/src/server.rs:34623 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 24 | crates/api/src/sweeprun.rs:3301 | #[expect] | large_enum_variant | yes | STYLE: cannot panic, truncate or wrap |
| 25 | crates/api/src/sweeprun.rs:3310 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 26 | crates/api/src/trades.rs:84 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 27 | crates/api/src/verify.rs:230 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 28 | crates/cli/src/admission_join.rs:352 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 29 | crates/cli/src/admission_store.rs:698 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 30 | crates/cli/src/all_rung_population_v5.rs:31 | #![expect] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 31 | crates/cli/src/all_rung_population_v5.rs:559 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 32 | crates/cli/src/all_rung_population_v5.rs:729 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 33 | crates/cli/src/all_rung_population_v5.rs:1599 | #[expect] | unnecessary_wraps | yes | STYLE: cannot panic, truncate or wrap |
| 34 | crates/cli/src/all_rung_selection_v5.rs:34 | #![expect] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 35 | crates/cli/src/all_rung_selection_v5.rs:431 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 36 | crates/cli/src/batch.rs:492 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 37 | crates/cli/src/batch.rs:677 | #[expect] | default_trait_access | yes | STYLE: cannot panic, truncate or wrap |
| 38 | crates/cli/src/batch.rs:865 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 39 | crates/cli/src/boolean_candidate_v1.rs:710 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 40 | crates/cli/src/boolean_oos_v1.rs:565 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 41 | crates/cli/src/boolean_qualification_v1.rs:364 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 42 | crates/cli/src/boolean_qualification_v1.rs:555 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 43 | crates/cli/src/boolean_qualification_wire.rs:467 | #[expect] | cast_precision_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 44 | crates/cli/src/boolean_qualified_command.rs:110 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 45 | crates/cli/src/boolean_search_command.rs:161 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 46 | crates/cli/src/boolean_statistics_reader.rs:669 | #[expect] | cast_precision_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 47 | crates/cli/src/candidate_universe.rs:562 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 48 | crates/cli/src/candidate_universe.rs:823 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 49 | crates/cli/src/candidate_universe.rs:827 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 50 | crates/cli/src/candidate_universe.rs:959 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 51 | crates/cli/src/candidate_universe.rs:1152 | #[expect] | large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 52 | crates/cli/src/candidate_universe.rs:1195 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 53 | crates/cli/src/candidate_universe.rs:2743 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 54 | crates/cli/src/candidate_universe.rs:4355 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 55 | crates/cli/src/candidate_universe.rs:4359 | #[expect] | large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 56 | crates/cli/src/candidate_universe.rs:4640 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 57 | crates/cli/src/candidate_universe.rs:4877 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 58 | crates/cli/src/candidate_universe.rs:4951 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 59 | crates/cli/src/candidate_universe.rs:5053 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 60 | crates/cli/src/execution_disposition_v2.rs:1269 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 61 | crates/cli/src/execution_disposition_v2.rs:2119 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 62 | crates/cli/src/execution_disposition_v2.rs:2620 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 63 | crates/cli/src/execution_v3.rs:38 | #![expect] | dead_code | yes | STALE REASON + module-wide (CE-95): Selection V5 already consumes it |
| 64 | crates/cli/src/execution_v3.rs:1950 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 65 | crates/cli/src/execution_v3.rs:2066 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 66 | crates/cli/src/execution_v4.rs:41 | #![expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 67 | crates/cli/src/execution_v4.rs:1612 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 68 | crates/cli/src/execution_v4.rs:1733 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 69 | crates/cli/src/execution_v4.rs:1887 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 70 | crates/cli/src/execution_v4.rs:2448 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 71 | crates/cli/src/execution_v4.rs:2593 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 72 | crates/cli/src/expression_search.rs:514 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 73 | crates/cli/src/fold_audit.rs:407 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 74 | crates/cli/src/frontier.rs:394 | #[allow] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 75 | crates/cli/src/frontier.rs:525 | #[allow] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 76 | crates/cli/src/frontier.rs:3283 | #[expect] | struct_excessive_bools | yes | STYLE: cannot panic, truncate or wrap |
| 77 | crates/cli/src/global_replay.rs:365 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 78 | crates/cli/src/global_replay.rs:2475 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 79 | crates/cli/src/global_replay_v2.rs:339 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 80 | crates/cli/src/global_replay_v2.rs:3141 | #[allow] | too_many_arguments,too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 81 | crates/cli/src/global_replay_v2.rs:3302 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 82 | crates/cli/src/global_replay_v3.rs:40 | #![expect] | dead_code | yes | STALE REASON + module-wide (CE-95): step3_orchestrator already imports it |
| 83 | crates/cli/src/global_replay_v3.rs:124 | #[expect] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 84 | crates/cli/src/global_replay_v3.rs:571 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 85 | crates/cli/src/global_replay_v3.rs:911 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 86 | crates/cli/src/global_replay_v3.rs:3116 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 87 | crates/cli/src/index_stop_qualification.rs:756 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 88 | crates/cli/src/index_stop_qualification_codec.rs:136 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 89 | crates/cli/src/index_stop_search_checkpoint.rs:366 | #[expect] | too_many_arguments,too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 90 | crates/cli/src/index_stop_search_progress.rs:42 | #[expect] | large_enum_variant | yes | STYLE: cannot panic, truncate or wrap |
| 91 | crates/cli/src/index_stop_search_reader.rs:381 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 92 | crates/cli/src/index_stop_source_context_codec.rs:240 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 93 | crates/cli/src/index_stop_store.rs:318 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 94 | crates/cli/src/index_stop_store.rs:496 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 95 | crates/cli/src/institutional_evidence.rs:496 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 96 | crates/cli/src/institutional_evidence.rs:781 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 97 | crates/cli/src/institutional_evidence.rs:1554 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 98 | crates/cli/src/institutional_evidence.rs:2066 | #[expect] | float_arithmetic,cast_precision_loss,cast_possible_truncation,cast_sign_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 99 | crates/cli/src/institutional_evidence.rs:2139 | #[expect] | float_arithmetic,cast_precision_loss,cast_possible_truncation,cast_sign_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 100 | crates/cli/src/institutional_statistics.rs:90 | #[expect] | cast_precision_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 101 | crates/cli/src/institutional_statistics.rs:363 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 102 | crates/cli/src/lib.rs:1095 | #[expect] | default_trait_access | yes | STYLE: cannot panic, truncate or wrap |
| 103 | crates/cli/src/lib.rs:1678 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 104 | crates/cli/src/lib.rs:2182 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 105 | crates/cli/src/lib.rs:2739 | #[allow] | needless_pass_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 106 | crates/cli/src/lib.rs:2755 | #[allow] | large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 107 | crates/cli/src/lib.rs:3548 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 108 | crates/cli/src/lib.rs:3596 | #[expect] | default_trait_access | yes | STYLE: cannot panic, truncate or wrap |
| 109 | crates/cli/src/lib.rs:3902 | #[allow] | needless_pass_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 110 | crates/cli/src/lib.rs:3911 | #[allow] | large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 111 | crates/cli/src/lib.rs:4160 | #[expect] | default_trait_access | yes | STYLE: cannot panic, truncate or wrap |
| 112 | crates/cli/src/lib.rs:6117 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 113 | crates/cli/src/lib.rs:6231 | #[expect] | default_trait_access | yes | STYLE: cannot panic, truncate or wrap |
| 114 | crates/cli/src/lib.rs:7009 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 115 | crates/cli/src/lib.rs:7108 | #[expect] | default_trait_access | yes | STYLE: cannot panic, truncate or wrap |
| 116 | crates/cli/src/lib.rs:7308 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 117 | crates/cli/src/lib.rs:11680 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 118 | crates/cli/src/lib.rs:11684 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 119 | crates/cli/src/lib.rs:12061 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 120 | crates/cli/src/lib.rs:12160 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 121 | crates/cli/src/lib.rs:13665 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 122 | crates/cli/src/lib.rs:16042 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 123 | crates/cli/src/lib.rs:16101 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 124 | crates/cli/src/lib.rs:16323 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 125 | crates/cli/src/lib.rs:16390 | #[expect] | default_trait_access | yes | STYLE: cannot panic, truncate or wrap |
| 126 | crates/cli/src/lib.rs:16533 | #[allow] | needless_pass_by_value,large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 127 | crates/cli/src/lib.rs:18290 | #[expect] | cast_possible_truncation,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 128 | crates/cli/src/lib.rs:19294 | #[allow] | large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 129 | crates/cli/src/lib.rs:19596 | #[allow] | needless_pass_by_value,large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 130 | crates/cli/src/lib.rs:19603 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 131 | crates/cli/src/lib.rs:20334 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 132 | crates/cli/src/live.rs:1087 | #[expect] | cast_precision_loss,cast_possible_truncation,cast_sign_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 133 | crates/cli/src/population.rs:2272 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 134 | crates/cli/src/population.rs:2764 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 135 | crates/cli/src/population_admission_v3.rs:1659 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 136 | crates/cli/src/population_admission_v4.rs:721 | #[expect] | too_many_arguments,too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 137 | crates/cli/src/population_admission_v4.rs:1969 | #[expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 138 | crates/cli/src/population_admission_v4.rs:2363 | #[expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 139 | crates/cli/src/population_admission_v4.rs:2694 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 140 | crates/cli/src/population_admission_writer.rs:355 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 141 | crates/cli/src/population_admission_writer.rs:385 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 142 | crates/cli/src/population_admission_writer.rs:457 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 143 | crates/cli/src/population_admission_writer.rs:505 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 144 | crates/cli/src/population_admission_writer.rs:1051 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 145 | crates/cli/src/population_base_evidence_v2.rs:988 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 146 | crates/cli/src/population_finalization_v3.rs:34 | #![expect] | dead_code | yes | STALE REASON + module-wide (CE-95): Population V4 is no longer the blocker; V5/step3 import it |
| 147 | crates/cli/src/population_finalization_v3.rs:820 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 148 | crates/cli/src/population_finalization_v4.rs:25 | #![expect] | dead_code | yes | STALE REASON + module-wide (CE-95): ledger_v6/population_v6 are the non-test all-rung caller |
| 149 | crates/cli/src/population_finalization_v4.rs:451 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 150 | crates/cli/src/population_finalization_v4.rs:2329 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 151 | crates/cli/src/population_observations_v1.rs:847 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 152 | crates/cli/src/population_observations_v1.rs:1138 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 153 | crates/cli/src/population_observations_v1.rs:1443 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 154 | crates/cli/src/population_statistics_v2.rs:462 | #[expect] | cast_precision_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 155 | crates/cli/src/population_statistics_v2.rs:2767 | #[expect] | cast_precision_loss,cast_possible_truncation,cast_sign_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 156 | crates/cli/src/population_statistics_v2.rs:3965 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 157 | crates/cli/src/population_statistics_v2.rs:5044 | #[expect] | cast_precision_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 158 | crates/cli/src/population_statistics_v3.rs:22 | #![expect] | large_types_passed_by_value | yes | STYLE: cannot panic, truncate or wrap |
| 159 | crates/cli/src/population_statistics_v3.rs:147 | #[expect] | cast_precision_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 160 | crates/cli/src/population_statistics_v3.rs:847 | #[expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 161 | crates/cli/src/population_statistics_v3.rs:978 | #[expect] | cast_precision_loss,cast_possible_truncation,cast_sign_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 162 | crates/cli/src/population_statistics_v3.rs:2379 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 163 | crates/cli/src/population_statistics_v3.rs:2572 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 164 | crates/cli/src/population_statistics_v3.rs:3282 | #[expect] | cast_precision_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 165 | crates/cli/src/population_v5.rs:28 | #![expect] | dead_code | yes | STALE REASON + module-wide (CE-95): ledger_all/all_rung_population_v5 consume it |
| 166 | crates/cli/src/population_v6.rs:20 | #![expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 167 | crates/cli/src/population_v6.rs:79 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 168 | crates/cli/src/pre_admission_data.rs:2686 | #[expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 169 | crates/cli/src/research_policy.rs:317 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 170 | crates/cli/src/results.rs:382 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 171 | crates/cli/src/results.rs:474 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 172 | crates/cli/src/results.rs:869 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 173 | crates/cli/src/selection.rs:108 | #[allow] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 174 | crates/cli/src/selection.rs:243 | #[allow] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 175 | crates/cli/src/selection_v3.rs:114 | #[allow] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 176 | crates/cli/src/selection_v5.rs:413 | #[expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 177 | crates/cli/src/selection_v5.rs:1303 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 178 | crates/cli/src/selection_v5.rs:2731 | #[allow] | too_many_arguments | no (comment/none) | STYLE: cannot panic, truncate or wrap |
| 179 | crates/cli/src/step3_comparison.rs:217 | #[expect] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 180 | crates/cli/src/step3_orchestrator.rs:470 | #[expect] | unnecessary_wraps | yes | STYLE: cannot panic, truncate or wrap |
| 181 | crates/cli/src/step3_orchestrator.rs:800 | #[allow] | dead_code | yes | HOLDS: only test callers today (allow, not expect) |
| 182 | crates/cli/src/step3_orchestrator.rs:1421 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 183 | crates/cli/src/step3_orchestrator.rs:2681 | #[allow] | dead_code | yes | HOLDS: reached only via the test-only caller above |
| 184 | crates/cli/src/step3_orchestrator.rs:3495 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 185 | crates/cli/src/step3_orchestrator.rs:3835 | #[expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 186 | crates/cli/src/step3_orchestrator.rs:4019 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 187 | crates/cli/src/stored.rs:2311 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 188 | crates/cli/src/stored_post_training_oos.rs:23 | #![allow] | dead_code | yes | HOLDS (module-wide allow, not expect: will not flag when stale) |
| 189 | crates/cli/src/stored_post_training_oos.rs:213 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 190 | crates/cli/src/trades.rs:208 | #[allow] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 191 | crates/cli/src/trades.rs:259 | #[allow] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 192 | crates/cli/src/vix_reference.rs:108 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 193 | crates/core/src/blake3.rs:399 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 194 | crates/core/src/price.rs:112 | #[allow] | float_arithmetic | no (comment/none) | HOLDS: statistic/count, float->int casts saturate; no panic |
| 195 | crates/core/src/price.rs:168 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 196 | crates/core/src/symbol.rs:81 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 197 | crates/core/src/universe.rs:4190 | #[expect] | indexing_slicing | yes | HOLDS: both arrays typed [&str; 750] |
| 198 | crates/core/src/universe.rs:4308 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 199 | crates/core/src/universe.rs:4417 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 200 | crates/core/src/universe.rs:4501 | #[expect] | cast_possible_truncation | yes | HOLDS (n is a const power of two); doc says n=0 "wraps" -- with overflow-checks it would panic; unreachable |
| 201 | crates/core/src/universe.rs:4520 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 202 | crates/core/src/vendor.rs:311 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 203 | crates/core/src/vendor.rs:453 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 204 | crates/core/src/vendor.rs:1334 | #[allow] | match_same_arms | yes | STYLE: cannot panic, truncate or wrap |
| 205 | crates/costs/src/day.rs:428 | #[allow] | cast_lossless | no (comment/none) | STYLE: cannot panic, truncate or wrap |
| 206 | crates/costs/src/day.rs:494 | #[allow] | cast_lossless | no (comment/none) | STYLE: cannot panic, truncate or wrap |
| 207 | crates/costs/src/day.rs:530 | #[allow] | cast_possible_truncation,cast_sign_loss | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 208 | crates/costs/src/expiry.rs:295 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 209 | crates/costs/src/expiry.rs:308 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 210 | crates/costs/src/expiry.rs:351 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 211 | crates/costs/src/expiry.rs:407 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 212 | crates/costs/src/lot.rs:92 | #[allow] | cast_lossless | no (comment/none) | STYLE: cannot panic, truncate or wrap |
| 213 | crates/costs/src/lot.rs:232 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 214 | crates/costs/src/scope.rs:104 | #[allow] | match_same_arms | yes | STYLE: cannot panic, truncate or wrap |
| 215 | crates/costs/src/strike.rs:221 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 216 | crates/costs/src/venue.rs:124 | #[allow] | indexing_slicing | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 217 | crates/engine/src/lib.rs:397 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 218 | crates/greeks/src/bsm.rs:56 | #![allow] | float_arithmetic | no (comment/none) | HOLDS: statistic/count, float->int casts saturate; no panic |
| 219 | crates/greeks/src/moneyness.rs:28 | #![allow] | float_arithmetic | no (comment/none) | HOLDS: statistic/count, float->int casts saturate; no panic |
| 220 | crates/greeks/src/moneyness.rs:125 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 221 | crates/greeks/src/normal.rs:42 | #![allow] | float_arithmetic | no (comment/none) | HOLDS: statistic/count, float->int casts saturate; no panic |
| 222 | crates/greeks/src/solver.rs:130 | #![allow] | float_arithmetic | no (comment/none) | HOLDS: statistic/count, float->int casts saturate; no panic |
| 223 | crates/indicators/src/anchored.rs:776 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 224 | crates/indicators/src/gap.rs:119 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 225 | crates/indicators/src/lib.rs:575 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 226 | crates/indicators/src/pattern.rs:410 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 227 | crates/indicators/src/session.rs:309 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 228 | crates/lake/src/batch.rs:144 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 229 | crates/pull/src/archive.rs:682 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 230 | crates/pull/src/capture.rs:196 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 231 | crates/pull/src/capture.rs:309 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 232 | crates/pull/src/capture.rs:364 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 233 | crates/pull/src/capture.rs:417 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 234 | crates/pull/src/dhan.rs:143 | #[allow] | match_same_arms | yes | STYLE: cannot panic, truncate or wrap |
| 235 | crates/pull/src/groww.rs:61 | #[allow] | match_same_arms | yes | STYLE: cannot panic, truncate or wrap |
| 236 | crates/pull/src/http.rs:409 | #[allow] | missing_fields_in_debug | no (comment/none) | STYLE: cannot panic, truncate or wrap |
| 237 | crates/pull/src/ingest.rs:2094 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 238 | crates/pull/src/ingest.rs:2818 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 239 | crates/pull/src/pricing.rs:251 | #[expect] | float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 240 | crates/pull/src/pricing.rs:567 | #[expect] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 241 | crates/pull/src/pricing.rs:614 | #[expect] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 242 | crates/pull/src/pricing.rs:767 | #[expect] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 243 | crates/pull/src/pricing.rs:785 | #[expect] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 244 | crates/pull/src/session.rs:323 | #[expect] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 245 | crates/pull/src/session.rs:616 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 246 | crates/pull/src/session.rs:744 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 247 | crates/pull/src/session.rs:752 | #[expect] | cast_sign_loss | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 248 | crates/pull/src/session.rs:775 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 249 | crates/pull/src/ssm.rs:212 | #[allow] | missing_fields_in_debug | no (comment/none) | STYLE: cannot panic, truncate or wrap |
| 250 | crates/pull/src/ssm.rs:484 | #[allow] | expect_used | yes | HOLDS: HMAC accepts any key length |
| 251 | crates/pull/src/ssm.rs:696 | #[allow] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 252 | crates/pull/src/tenor.rs:298 | #[expect] | float_arithmetic | yes | STALE REASON (CE-97): claims the ONE float boundary in `pull`; pricing.rs:251 is a second |
| 253 | crates/pull/src/tenor.rs:313 | #[expect] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 254 | crates/pull/src/vendor.rs:447 | #[allow] | match_same_arms | yes | STYLE: cannot panic, truncate or wrap |
| 255 | crates/pull/src/vendor.rs:1417 | #[expect] | dead_code | yes | HOLDS (expect still fulfilled) |
| 256 | crates/pull/src/vendor.rs:3565 | #[allow] | large_enum_variant | yes | STYLE: cannot panic, truncate or wrap |
| 257 | crates/pull/src/vendor.rs:3684 | #[allow] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 258 | crates/runner/src/admission.rs:593 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 259 | crates/runner/src/admission.rs:833 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 260 | crates/runner/src/admission.rs:3398 | #[expect] | cast_precision_loss,cast_possible_truncation,cast_sign_loss,float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 261 | crates/runner/src/audit.rs:414 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 262 | crates/runner/src/bootstrap.rs:58 | #![allow] | float_arithmetic | yes | HOLDS (module-wide; statistic inputs) |
| 263 | crates/runner/src/bootstrap.rs:67 | #![allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 264 | crates/runner/src/bootstrap.rs:841 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 265 | crates/runner/src/bound.rs:187 | #[expect] | struct_field_names | yes | STYLE: cannot panic, truncate or wrap |
| 266 | crates/runner/src/exit_grid_policy.rs:932 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 267 | crates/runner/src/exit_grid_policy.rs:3864 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 268 | crates/runner/src/grid.rs:456 | #[expect] | float_arithmetic | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 269 | crates/runner/src/grid.rs:474 | #[expect] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 270 | crates/runner/src/grid.rs:480 | #[expect] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 271 | crates/runner/src/grid.rs:503 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 272 | crates/runner/src/grid.rs:1861 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 273 | crates/runner/src/grid.rs:1960 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 274 | crates/runner/src/grid.rs:2034 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 275 | crates/runner/src/grid.rs:2293 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 276 | crates/runner/src/grid.rs:2739 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 277 | crates/runner/src/grid.rs:2852 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 278 | crates/runner/src/grid.rs:2992 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 279 | crates/runner/src/grid.rs:3285 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 280 | crates/runner/src/grid.rs:3440 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 281 | crates/runner/src/grid.rs:3505 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 282 | crates/runner/src/grid.rs:3619 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 283 | crates/runner/src/grid.rs:3642 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 284 | crates/runner/src/grid.rs:3684 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 285 | crates/runner/src/grid.rs:3709 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 286 | crates/runner/src/grid.rs:4768 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 287 | crates/runner/src/outcome.rs:48 | #![allow] | float_arithmetic | yes | STALE REASON + module-wide (CE-96): `Edge` carries paisa sums as f64 |
| 288 | crates/runner/src/outcome.rs:1414 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 289 | crates/runner/src/outcome.rs:1536 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 290 | crates/runner/src/outcome.rs:1554 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 291 | crates/runner/src/outcome.rs:1634 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 292 | crates/runner/src/outcome.rs:1708 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 293 | crates/runner/src/outcome.rs:1830 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 294 | crates/runner/src/outcome.rs:2281 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 295 | crates/runner/src/outcome.rs:2288 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 296 | crates/runner/src/outcome.rs:2466 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 297 | crates/runner/src/portfolio.rs:406 | #[expect] | large_stack_arrays | yes | STYLE: cannot panic, truncate or wrap |
| 298 | crates/runner/src/portfolio.rs:546 | #[expect] | large_stack_arrays | yes | STYLE: cannot panic, truncate or wrap |
| 299 | crates/runner/src/replay_mask.rs:163 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 300 | crates/runner/src/report.rs:443 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 301 | crates/runner/src/research_family.rs:134 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 302 | crates/runner/src/significance.rs:46 | #![allow] | float_arithmetic | yes | HOLDS (module-wide; counts and t only) |
| 303 | crates/runner/src/significance.rs:190 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 304 | crates/runner/src/significance.rs:264 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 305 | crates/runner/src/significance.rs:296 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 306 | crates/runner/src/significance.rs:372 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 307 | crates/runner/src/significance.rs:381 | #[allow] | cast_precision_loss | yes | HOLDS: statistic/count, float->int casts saturate; no panic |
| 308 | crates/runner/src/trade.rs:893 | #[allow] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 309 | crates/runner/src/trade.rs:898 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 310 | crates/runner/src/validate.rs:1941 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 311 | crates/runner/src/validate.rs:1994 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 312 | crates/runner/src/validate.rs:2044 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 313 | crates/runner/src/validate.rs:2090 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 314 | crates/runner/src/validate.rs:2135 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 315 | crates/runner/src/validate.rs:2253 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 316 | crates/runner/src/validate.rs:2475 | #[expect] | too_many_arguments,too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 317 | crates/runner/src/validate.rs:4371 | #[expect] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 318 | crates/runner/src/validate.rs:4376 | #[expect] | too_many_lines | yes | STYLE: cannot panic, truncate or wrap |
| 319 | crates/store/src/crc.rs:98 | #[allow] | indexing_slicing,cast_possible_truncation | no (comment/none) | HOLDS: constant offsets / loop- or type-bounded index |
| 320 | crates/store/src/file.rs:1664 | #[allow] | too_many_arguments | yes | STYLE: cannot panic, truncate or wrap |
| 321 | crates/store/src/file.rs:2153 | #[expect] | used_underscore_binding | yes | STYLE: cannot panic, truncate or wrap |
| 322 | crates/store/src/format.rs:467 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 323 | crates/store/src/format.rs:614 | #[allow] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 324 | crates/store/src/format.rs:1064 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 325 | crates/store/src/header.rs:261 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS for every shipped Layout; no reason; public `Layout::declare` admits stride > u16::MAX (no production caller) |
| 326 | crates/vocab/src/expression_search.rs:35 | #[expect] | large_enum_variant | yes | STYLE: cannot panic, truncate or wrap |
| 327 | crates/vocab/src/mask.rs:50 | #[allow] | cast_possible_truncation | no (comment/none) | HOLDS: value bounded before cast (or hash truncation by design) |
| 328 | crates/vocab/src/table.rs:252 | #[allow] | large_const_arrays | yes | STYLE: cannot panic, truncate or wrap |
| 329 | crates/vocab/src/table.rs:1593 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 330 | crates/vocab/src/table.rs:1615 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 331 | crates/vocab/src/table.rs:1633 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
| 332 | crates/vocab/src/table.rs:1653 | #[expect] | indexing_slicing | yes | HOLDS: constant offsets / loop- or type-bounded index |
| 333 | crates/vocab/src/table.rs:1667 | #[expect] | cast_possible_truncation | yes | HOLDS: value bounded before cast (or hash truncation by design) |
