STATUS: IN PROGRESS

# BASE-COV-API-AS3 (shard G3, repair r3, 5 Nov 2026): working report

This file is rewritten as the session goes; line 1 is the state of the branch now.

- Branch `cloud/cov-api-as3-20261105`, cut from the kit commit `f4111a0132473d8771d8624a9538b1362926f70b`;
  START `6ea46e59c07488eeccff388407976041852aae2c`.
- Session 1 (token `BASE-COV-API-AS3-20261105-1`) pushed up to `7700e2f0`; the main session paused the shard at
  `edd54703` (token `PAUSED-BY-MAIN-20261105-1420`). This continuation holds token `BASE-COV-API-AS3-20261105-3`
  (`a5f009a8`) and resumed from `edd54703`.
- Host: 4 cores, 15 GB, root, Linux x86_64. Pinned nightly installed as `nightly-2026-08-08`:
  `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, `LLVM version 23.1.0-rust-1.99.0-nightly` (llvm-cov and
  llvm-profdata). Dev loop: the api lib tests built with Route B's flags (`-C instrument-coverage -Z
  coverage-options=branch`, opt-level 0), the tests of the files at hand run, and each kit GAP row checked against
  that run's uncovered rows (dev figures; the measurement of record is Route B, still to run). Three helper
  sessions work file groups in worktrees of their own; their commits are cherry-picked here after a combined run.

## Kit GAP rows closed so far by the dev check (closed / kit rows; open rows with their reason)

| file | closed | open, why |
|---|---|---|
| `indexstoplaunch.rs` | 71 / 71 | (345, the `Ok(report)` arm, run by the saved-qualification probe) |
| `indexstoplaunch_metadata.rs` | 24 / 25 | 52:20 False (U) |
| `indexstoplaunch_tests.rs` | 6 / 6 | |
| `indexstopqualificationjson.rs` | 205 / 216 | (U) 152:9 F, 154:16 F, 317; GAP: the 8 MiB page (114:16 F, 117, 119), the ranking budget (210:8 T, 215: needs a second process whose observation budget lies between the admitted bytes and 16 bytes a row more), the other verdict statuses (235-237: the generated months give only unmeasured verdicts) |
| `indexstopcandlesjson.rs` | 104 / 113 | (U) 117:16 T, 118, 120 (a page is under 1.2 MiB), 161:9 F (root from the environment), 201 (the catalog decodes only long or short); race: 188:25 T, 189-191 (a file changed between the projection and its recheck) |
| `indexstopjson.rs` | 9 / 12 | (U) 99:16 F, 102, 104 (a page is under 5.2 MiB) |
| `frontierjson.rs` | 24 / 46 | (U) 193-196 and 265-274; 300:25 F and 310-316 only by a concurrent change of the frontier path |
| `gaps_route.rs` | 3 / 10 | the 7 production rows are (U) |
| `dayflags.rs` | 213 / 232 | 15 (U), 3 race-only (3220:12 F, 3322:16 F, 3323:16 F), 1 now an (F) line |
| `dayflags_filter_tests.rs`, `month_change.rs` | 3 / 3, 2 / 2 | |
| `ladder.rs` | 5 / 8 | 3 production rows (U) |
| `census_rollup.rs`, `ingest.rs`, `ingest_refusals.rs`, `imports.rs` | 34/35, 31/37, 7/7, 49/54 | helper c's report pending |
| `owed_tests.rs`, `nse_identity.rs`, `booleanevidencejson_tests.rs`, `http_admission_tests.rs`, `pool_table_tests.rs`, `census_request_tests.rs` | 3/3, 4/4, 9/9, 5/5, 27/27, 17/17 | |

## L-BCA-5, reached through the production path

`indexstopqualificationjson_saved_tests.rs` (declared at the end of `indexstopqualificationjson.rs`) writes three
generated NIFTY months with the store's own writer, then in a child process with bounded settings runs the real
`/backtest/command` launch (`sweeprun::command_with`) of the native single-stop search: strict source load, search,
qualification and daily assessments all saved by cli's production path in about 12 s at opt-level 0. It then reads
every page back: settings, a kept reader, folds, index days and weeks, the refusals, a saved body changed under the
kept reader (stale, then readmitted when restored), the single-stop source pages and the candle reader. On a build
with no commit stamp (a dirty tree) the launch is refused and the test asserts that refusal: the measured build is
stamped (cli's build.rs stamps a clean tree), so that one arm is (U) for the measured object. A second probe
(`booleanevidencejson_saved_tests.rs`) launches the qualified Boolean search over the same months: in progress.

## Each new test of the main session, red under a hand change

One temporary local commit held every hand change at once (so cli's build.rs stamped the build), the named tests ran
on it, and the commit was reset away (never pushed); runs `red3` (13 tests) and `red4` (the probe alone, because in
`red3` it failed first on the preview door's change). Each failure below is its own hand change's:

| test | hand change | failure |
|---|---|---|
| `indexstoplaunch::tests::every_incoherent_observation_is_refused_by_name_and_its_coherent_edges_pass` | the refused-source restart refusal reworded | left "single-stop refused source restarted" |
| `indexstoplaunch::tests::a_foreign_command_and_a_count_of_no_or_too_many_digits_are_refused_before_any_read` | "requires its exact native command" -> "its native command" | left the changed text |
| `indexstoplaunch::tests::the_configuration_preview_door_answers_json_when_nothing_is_configured` | the door answers 202 for 200 | left 202, right 200 |
| `indexstoplaunch::tests::a_preview_past_its_observation_records_or_store_bounds_names_each_one` | "not an existing absolute directory" -> "not a directory" | left the changed bound list |
| `indexstoplaunch::tests::a_launch_whose_slot_moved_and_a_preview_that_cannot_run_are_refused_by_name` | the replay admission sentence shortened | left the shortened sentence |
| `frontierjson::tests::every_unasked_frontier_answer_is_given_or_refused_by_name` | the unreadable results file answers 502 for 503 | left 502, right 503 |
| `booleanevidencejson::tests::the_scanner_reads_unclosed_and_look_alike_tokens_as_the_rule_needs` | `code_only` writes `r''` for a raw string | left "x = r''" |
| `server::pool_table_tests::the_table_readers_read_every_literal_shape_and_the_row_builder_runs` | `head_text` keeps 601 characters | left 601, right 600 |
| `server::http_admission_tests::a_journaled_read_asked_while_every_slot_is_held_waits_for_one` | `ask_journaled` asks once | the 429 "bounded request audit capacity is full" answer |
| `indexstopqualificationjson::tests::pins_scope_period_and_bounded_paging_are_required_before_any_read` | the period refusal reworded | left "unknown native qualification period" |
| `indexstopqualificationjson::saved_tests::a_saved_qualification_is_served_by_every_page_that_reads_it` (`red4`) | the settings page says "kept" for "saved" | left "kept", right "saved" |
| `indexstopjson::cover_tests::a_second_root_reopens_and_a_page_kind_no_query_names_is_refused_by_name` | the cache key drops the root | the removed first root's `owner.lock` cannot be measured |
| `indexstopcandlesjson::tests::original_source_requests_are_pinned_exact_and_bounded_before_file_reads` (case `&kind=source&after=0`) | the source page stops checking `after` | its `is_err` assertion, line 277 |

## Also fixed

- `gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit` held
  `MAX_CALENDAR_CONCURRENT` (8) slots, but the pool has one slot a core up to 8: on this 4-core host the child hung on
  its barrier (gdb: four holders inside their slots). It now holds `Named::Calendar.pool().max()` slots and waits until
  each is inside its slot before it asks the pool (D-8664 draft).
- `feeds_route.rs` test code from session 1: two clippy refusals (`too_many_lines`, `useless_format`).

## Not yet

- Helpers finishing by 16:30 UTC: a on `optiongreeks.rs`, `mastersrun.rs`, `autopilot.rs`; b on `selectionv6json.rs`
  (a saved Selection V6 through the production ledger verb); c on the saved Boolean evidence probe
  (`booleanevidencejson.rs`). Their earlier batches (dayflags group; logs, sweeprun and sweepevidence; bars, imports,
  ingest) are cherry-picked here.
- Route B whole:api on the code tip, `gaps-AS3.txt.md`, `fu-AS3.txt.md`, the gates on the tip,
  `fix-BASE-COV-API-AS3.json.md`.
- Slips to record: twice this session a shell command named the Py-language's interpreter (once in an earlier
  context, once at about 16:00 UTC as a version check whose output was discarded and never used); helper b reported
  one such slip in its own shell. Nothing was written to any file, tracked or not.
