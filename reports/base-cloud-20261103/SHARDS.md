# SHARDS.md: who edits what (5 Nov 2026, BASE-COV-API repair r3)

Source: the per-file table of `base-cov-api-gaps-r2.txt.md` (repair r2's Route B whole:api on 8091e72f7d85, whose `.rs` files START shares): 182 files read, GAP 4,186 lines and 1,730 sides. Made by `state/cov-api-shards-20261105/work/group-join.sh` on the Mac (shell and awk): the method of the COV-PULL groups (`shards-pull-20261103.md`; the same script with no join gives those three groups stem for stem on their table): a stem is a file with its `_tests`/`_cov_tests`/`_cover_tests` files (`x/mod.rs` reads as `x`); stems sorted by GAP lines + sides, ties by name, each to the lightest group. **One addition** for verify r3's blocking items (2) and (3), so each named file and the test modules its production file declares with `#[path]` sit in one shard: `audit` takes `audit_lock_wait_measure.rs` and `audit_lock_wait_tests.rs` (both declared in `audit.rs`), `pullrun` takes `pullrun_restart_tests.rs`, `sources` takes `sources_day_truth_tests.rs`, `disk_route` takes `disk_route_seal_tests.rs`. The figures size the groups; the work also holds items (2) and (3) below, and GAP rows marked `ignored` (inside `#[ignore]`d real-data tests) are owed to the Mac.

Rules: a shard edits only its group's files and NEW test files for them (a new file belongs to the group of the production file that declares it); never `crates/api/src/lib.rs`, `Cargo.toml`, `Cargo.lock`, `docs/` or `.github/` (ASA's), never `crates/api/benches/**` (no route reads it, L-BCA-2: the operator's); an existing file of `crates/api` not listed below belongs to G1 (AS1).

## Totals

| group | session | GAP lines | GAP sides | lines + sides | stems | files | lines: production / test / ignored | sides: production / test / ignored |
|---|---|---|---|---|---|---|---|---|
| G1 | BASE-COV-API-AS1 | 1358 | 614 | 1972 | 65 | 76 | 1155 / 166 / 37 | 491 / 123 / 0 |
| G2 | BASE-COV-API-AS2 | 1433 | 539 | 1972 | 43 | 58 | 1192 / 131 / 110 | 412 / 115 / 12 |
| G3 | BASE-COV-API-AS3 | 1395 | 577 | 1972 | 44 | 48 | 1167 / 142 / 86 | 450 / 127 / 0 |

## The files verify r3 names (blocking items 2 and 3), each with its shard

Item (3), the operator's standing rule: at least 8 extreme cases for every file the part tests, at least 2 of them combinations of extremes at once and at least 1 out of the box, each asserted and named on the test's `Extremes:` doc line. The counts are verify r3's (singles / combined / out of the box). New tests go beside the file: in its own test module or a new test file declared in it.

| file | item | what is short | group |
|---|---|---|---|
| `crates/api/src/sources_tests.rs` | (2) and (3) | (2): line 2113 `println!("SOURCES-CROWD 429 on three doors");` in the crowd child of `a_sources_question_it_cannot_read_or_a_full_pool_is_said_by_name` is a print site gate 23 refuses (the tip lists `> api/src/sources_tests.rs:println!:1`, f01d7a0e4 does not): the child proves itself by a proof file its parent reads, as `served_routes_tests.rs`' child-ran file does, so no print site is left; a declared site in gate 23 is ASA's and needs its own decision. (3): that test 5 / 1 / 0 | G2 (AS2) |
| `crates/api/src/detail.rs` | (3) | `a_pool_made_at_run_time_is_the_pool_a_static_is` 3 / 0 / 0 (and its doc comment, lines 1983-1985, heads the wrong test: a verify r3 follow-up) | G1 (AS1) |
| `crates/api/src/store_route.rs` | (3) | `a_quarantined_months_row_names_it_and_every_other_row_says_none` 3 / 0 / 0 | G1 (AS1) |
| `crates/api/src/facets_tests.rs` | (3) | `an_underlying_held_on_two_exchanges_is_refused_until_one_is_named` 4 / 1 / 1 | G1 (AS1) |
| `crates/api/src/emitted.rs` | (3) | BCA-19, the specific-matcher block at line 1569: no extremes declared | G1 (AS1) |
| `crates/api/src/audit.rs` | (3) | the small-size timing wrapper `the_append_timing_runs_at_a_small_size`: none declared | G1 (AS1) |
| `crates/api/src/audit_lock_wait_measure.rs` | (3) | the small-size timing wrapper `the_lock_wait_measurement_runs_at_a_small_size`: none declared | G1 (AS1) |
| `crates/api/src/import_journal.rs` | (3) | the small-size timing wrappers `the_end_line_timing_runs_at_a_small_size`, `the_field_count_timing_runs_at_a_small_size`: none declared | G1 (AS1) |
| `crates/api/src/pullrun_restart_tests.rs` | (3) | the small-size timing wrappers `the_open_runs_timing_runs_at_a_small_size`, `the_walks_worst_case_timing_runs_at_a_small_size`: none declared | G1 (AS1) |
| `crates/api/src/research_doors_tests.rs` | (3) | 8 / 1 / 2: one combination short | G2 (AS2) |
| `crates/api/src/dayflags_rules_tests.rs` | (3) | the small-size timing wrappers `the_day_band_timing_runs_at_a_small_size`, `the_cold_share_page_timing_runs_at_a_small_size`, `the_profilers_pages_run_at_a_small_size`: none declared | G2 (AS2) |
| `crates/api/src/disk_route.rs` | (3) | the small-size timing wrapper `the_disk_timing_runs_at_a_small_size`: none declared | G2 (AS2) |
| `crates/api/src/server.rs` | (3) | the small-size timing wrapper `the_pull_runtime_measurement_runs_at_a_small_size`: none declared | G2 (AS2) |
| `crates/api/src/bars_series.rs` | (3) | `a_summary_that_does_not_describe_its_file_is_refused_field_by_field` 9 / 1 / 0 | G3 (AS3) |
| `crates/api/src/feeds_route.rs` | (3) | the small-size timing wrapper `the_held_span_bench_runs_at_a_small_size`: none declared | G3 (AS3) |
| `crates/api/src/frontierjson.rs` | met | `every_edge_of_a_page_answers_or_refuses_by_name` 10 / 2 / 2 (verify r3: holds); keep it so | G3 (AS3) |
| `crates/api/src/mastersrun.rs` | met | `a_refresh_with_no_credential_lands_the_public_masters_and_says_it_is_not_whole`, 9 configurations / 2 / 1 (verify r3: holds); keep it so | G3 (AS3) |
| `crates/api/src/sweeprun_accepted_tests.rs` | met | the press tests (attempt edges, seven 400s, three doors at once in flight, the field twice; verify r3: holds); keep them so | G1 (AS1) |

## G1 (session BASE-COV-API-AS1): 1358 lines + 614 sides, 65 stems

- recovery (382 / 175): `crates/api/src/recovery.rs`
- census (104 / 68): `crates/api/src/census.rs`
- booleansearchjson (117 / 26): `crates/api/src/booleansearchjson.rs`, `crates/api/src/booleansearchjson_tests.rs`
- facets (78 / 28): `crates/api/src/facets.rs`, `crates/api/src/facets_tests.rs`
- index_consistency_projection (89 / 10): `crates/api/src/index_consistency_projection.rs`
- feed_checks_files (61 / 29): `crates/api/src/feed_checks_files.rs`
- chain (61 / 23): `crates/api/src/chain.rs`
- detail (60 / 9): `crates/api/src/detail.rs`
- settings_route (47 / 15): `crates/api/src/settings_route.rs`, `crates/api/src/settings_route_tests.rs`
- audit_json (46 / 11): `crates/api/src/audit_json.rs`
- feed_checks (33 / 17): `crates/api/src/feed_checks.rs`, `crates/api/src/feed_checks_tests.rs`
- bars (22 / 24): `crates/api/src/bars.rs`
- pullrun (25 / 18): `crates/api/src/pullrun.rs`, `crates/api/src/pullrun_restart_tests.rs`
- state_stream_emits (21 / 19): `crates/api/tests/state_stream_emits.rs`
- candidatejson (21 / 13): `crates/api/src/candidatejson.rs`
- store_route (24 / 9): `crates/api/src/store_route.rs`
- audit (26 / 4): `crates/api/src/audit.rs`, `crates/api/src/audit_lock_wait_measure.rs`, `crates/api/src/audit_lock_wait_tests.rs`
- pool_load (11 / 18): `crates/api/tests/pool_load.rs`
- repair_route (15 / 9): `crates/api/src/repair_route.rs`
- census_warm (4 / 19): `crates/api/src/census_warm_tests.rs`
- recovery_journal (16 / 6): `crates/api/src/recovery_journal.rs`
- folder (15 / 2): `crates/api/src/folder.rs`
- booleanoosjson (9 / 6): `crates/api/src/booleanoosjson.rs`, `crates/api/src/booleanoosjson_tests.rs`
- credential_law (10 / 5): `crates/api/src/credential_law.rs`, `crates/api/src/credential_law_tests.rs`
- indexmap (8 / 6): `crates/api/src/indexmap.rs`
- assets (5 / 6): `crates/api/src/assets.rs`
- calendar_build (4 / 7): `crates/api/src/calendar_build.rs`
- stop (6 / 5): `crates/api/src/stop_tests.rs`
- import_journal (7 / 3): `crates/api/src/import_journal.rs`
- server_cloud_mutant_tests_a (5 / 5): `crates/api/src/server_cloud_mutant_tests_a.rs`
- merge (3 / 5): `crates/api/src/merge.rs`
- main (6 / 1): `crates/api/src/main.rs`
- gap_peer_route (5 / 1): `crates/api/src/gap_peer_route_tests.rs`
- backtest (1 / 4): `crates/api/src/backtest.rs`
- bars_page (3 / 1): `crates/api/src/bars_page_tests.rs`
- saved_response_boundary (3 / 1): `crates/api/src/saved_response_boundary_tests.rs`
- aliases (1 / 1): `crates/api/src/aliases.rs`
- bars_change (1 / 1): `crates/api/src/bars_change.rs`
- census_epoch_link (1 / 1): `crates/api/src/census_epoch_link_tests.rs`
- page_report_route (1 / 1): `crates/api/src/page_report_route.rs`, `crates/api/src/page_report_route_tests.rs`
- autopilot_stall (0 / 1): `crates/api/src/autopilot_stall_tests.rs`
- isolated (0 / 1): `crates/api/src/isolated.rs`
- optiongreeks_route (1 / 0): `crates/api/src/optiongreeks_route_tests.rs`
- audit_json_commit (0 / 0): `crates/api/src/audit_json_commit_tests.rs`
- bars_sealed (0 / 0): `crates/api/src/bars_sealed_tests.rs`
- basis_route (0 / 0): `crates/api/src/basis_route.rs`, `crates/api/src/basis_route_tests.rs`
- binary (0 / 0): `crates/api/tests/binary.rs`
- calendar (0 / 0): `crates/api/src/calendar.rs`
- catalog (0 / 0): `crates/api/src/catalog.rs`
- chain_route (0 / 0): `crates/api/src/chain_route_tests.rs`
- cost_limits (0 / 0): `crates/api/src/cost_limits_tests.rs`
- coverage (0 / 0): `crates/api/src/coverage.rs`
- dayflags_v2 (0 / 0): `crates/api/src/dayflags_v2_tests.rs`
- emitted (0 / 0): `crates/api/src/emitted.rs`
- hand_stop (0 / 0): `crates/api/src/hand_stop_tests.rs`
- imports_filter (0 / 0): `crates/api/src/imports_filter_tests.rs`
- indexstop_projection (0 / 0): `crates/api/src/indexstop_projection_tests.rs`
- indexstopvix_projection (0 / 0): `crates/api/src/indexstopvix_projection_tests.rs`
- scratch (0 / 0): `crates/api/src/scratch.rs`
- served_routes (0 / 0): `crates/api/src/served_routes_tests.rs`
- spot (0 / 0): `crates/api/src/spot.rs`
- spot_route (0 / 0): `crates/api/src/spot_route_tests.rs`
- sweeprun_accepted (0 / 0): `crates/api/src/sweeprun_accepted_tests.rs`
- sweeprun_admission (0 / 0): `crates/api/src/sweeprun_admission_tests.rs`
- verification_route (0 / 0): `crates/api/src/verification_route_tests.rs`

## G2 (session BASE-COV-API-AS2): 1433 lines + 539 sides, 43 stems

- server (373 / 95): `crates/api/src/server.rs`
- indexstoprankingjson (194 / 21): `crates/api/src/indexstoprankingjson.rs`
- sources (88 / 68): `crates/api/src/sources.rs`, `crates/api/src/sources_day_truth_tests.rs`, `crates/api/src/sources_tests.rs`
- provenance (84 / 30): `crates/api/src/provenance.rs`, `crates/api/src/provenance_tests.rs`
- booleanqualification_projection (99 / 6): `crates/api/src/booleanqualification_projection.rs`, `crates/api/src/booleanqualification_projection_tests.rs`
- bars_sort (52 / 46): `crates/api/src/bars_sort.rs`, `crates/api/src/bars_sort_tests.rs`
- census_cache (64 / 24): `crates/api/src/census_cache.rs`
- calendar_of (53 / 22): `crates/api/src/calendar_of.rs`
- calendar_route (47 / 18): `crates/api/src/calendar_route.rs`, `crates/api/src/calendar_route_tests.rs`
- booleanlaunch (35 / 19): `crates/api/src/booleanlaunch.rs`, `crates/api/src/booleanlaunch_tests.rs`
- expressionsearchjson (33 / 18): `crates/api/src/expressionsearchjson.rs`
- disk_route (26 / 20): `crates/api/src/disk_route.rs`, `crates/api/src/disk_route_seal_tests.rs`
- operation_audit (35 / 7): `crates/api/src/operation_audit.rs`, `crates/api/src/operation_audit_tests.rs`
- speed (32 / 10): `crates/api/src/speed.rs`, `crates/api/src/speed_tests.rs`
- dayflags_rules (20 / 13): `crates/api/src/dayflags_rules.rs`, `crates/api/src/dayflags_rules_tests.rs`
- trades (24 / 9): `crates/api/src/trades.rs`
- booleanjson (24 / 6): `crates/api/src/booleanjson.rs`, `crates/api/src/booleanjson_tests.rs`
- calendar_record (17 / 11): `crates/api/src/calendar_record.rs`, `crates/api/src/calendar_record_tests.rs`
- scrub_route (11 / 16): `crates/api/src/scrub_route.rs`
- topjson (14 / 9): `crates/api/src/topjson.rs`
- quarantine_route (14 / 4): `crates/api/src/quarantine_route.rs`, `crates/api/src/quarantine_route_tests.rs`
- booleancampaignjson (9 / 8): `crates/api/src/booleancampaignjson.rs`, `crates/api/src/booleancampaignjson_tests.rs`
- recovery_control (9 / 8): `crates/api/src/recovery_control.rs`
- indexstopvixjson (11 / 4): `crates/api/src/indexstopvixjson.rs`
- head_deadline_body (9 / 4): `crates/api/src/head_deadline_body_tests.rs`
- render (6 / 6): `crates/api/src/render.rs`
- logs_r3_24 (8 / 3): `crates/api/tests/logs_r3_24.rs`
- verify (9 / 2): `crates/api/src/verify.rs`
- master (7 / 3): `crates/api/src/master.rs`
- store_wire (6 / 3): `crates/api/src/store_wire.rs`
- instruments_route (5 / 3): `crates/api/src/instruments_route.rs`
- fault_log (2 / 5): `crates/api/src/fault_log.rs`
- server_cloud_mutant_tests_e (2 / 5): `crates/api/src/server_cloud_mutant_tests_e.rs`
- cloud_mutants_bars (0 / 5): `crates/api/tests/cloud_mutants_bars.rs`
- constituents (0 / 4): `crates/api/src/constituents.rs`
- boolean_observation_budget (1 / 2): `crates/api/src/boolean_observation_budget.rs`
- fno_boundary (3 / 0): `crates/api/src/fno_boundary_tests.rs`
- bars_unknown_series (1 / 1): `crates/api/src/bars_unknown_series_tests.rs`
- census_tail (2 / 0): `crates/api/src/census_tail_tests.rs`
- store_step_forms (2 / 0): `crates/api/tests/store_step_forms.rs`
- boolean_search_budget (0 / 1): `crates/api/src/boolean_search_budget.rs`
- livejson (1 / 0): `crates/api/src/livejson.rs`
- research_doors (1 / 0): `crates/api/src/research_doors_tests.rs`

## G3 (session BASE-COV-API-AS3): 1395 lines + 577 sides, 44 stems

- dayflags (214 / 101): `crates/api/src/dayflags.rs`
- indexstopqualificationjson (194 / 22): `crates/api/src/indexstopqualificationjson.rs`
- booleanevidencejson (186 / 20): `crates/api/src/booleanevidencejson.rs`, `crates/api/src/booleanevidencejson_tests.rs`
- logs (62 / 59): `crates/api/src/logs.rs`
- indexstopcandlesjson (94 / 19): `crates/api/src/indexstopcandlesjson.rs`
- bars_export (68 / 33): `crates/api/src/bars_export.rs`
- sweeprun (62 / 35): `crates/api/src/sweeprun.rs`
- indexstoplaunch (41 / 36): `crates/api/src/indexstoplaunch.rs`, `crates/api/src/indexstoplaunch_tests.rs`
- feeds_route (41 / 30): `crates/api/src/feeds_route.rs`
- mastersrun (52 / 16): `crates/api/src/mastersrun.rs`
- imports (30 / 24): `crates/api/src/imports.rs`
- optiongreeks (38 / 11): `crates/api/src/optiongreeks.rs`
- autopilot (33 / 13): `crates/api/src/autopilot.rs`
- frontierjson (39 / 7): `crates/api/src/frontierjson.rs`
- ingest (27 / 10): `crates/api/src/ingest.rs`
- census_rollup (31 / 4): `crates/api/src/census_rollup.rs`
- bars_route (15 / 17): `crates/api/src/bars_route.rs`
- bars_series (17 / 15): `crates/api/src/bars_series.rs`
- pool_table (19 / 8): `crates/api/src/pool_table_tests.rs`
- indexstoplaunch_metadata (16 / 9): `crates/api/src/indexstoplaunch_metadata.rs`
- server_cloud_mutant_tests_b (12 / 11): `crates/api/src/server_cloud_mutant_tests_b.rs`
- strict_sweep (10 / 10): `crates/api/src/strict_sweep_tests.rs`
- census_request (10 / 7): `crates/api/src/census_request_tests.rs`
- server_cloud_mutant_tests_c (11 / 6): `crates/api/src/server_cloud_mutant_tests_c.rs`
- sweepevidence (10 / 5): `crates/api/src/sweepevidence.rs`
- indexstopjson (7 / 5): `crates/api/src/indexstopjson.rs`
- ops_safety (5 / 7): `crates/api/src/ops_safety_tests.rs`
- bars_window_route (5 / 6): `crates/api/src/bars_window_route_tests.rs`
- selectionv6json (9 / 2): `crates/api/src/selectionv6json.rs`, `crates/api/src/selectionv6json_tests.rs`
- gaps_route (5 / 5): `crates/api/src/gaps_route.rs`
- serve_edge (8 / 2): `crates/api/src/serve_edge_tests.rs`
- ladder (2 / 6): `crates/api/src/ladder.rs`
- ingest_refusals (5 / 2): `crates/api/src/ingest_refusals.rs`
- speed_route (7 / 0): `crates/api/src/speed_route.rs`
- http_admission (3 / 2): `crates/api/src/http_admission_tests.rs`
- nse_identity (0 / 4): `crates/api/src/nse_identity.rs`
- dayflags_filter (0 / 3): `crates/api/src/dayflags_filter_tests.rs`
- owed (1 / 2): `crates/api/src/owed_tests.rs`
- booleanadmission_projection (1 / 1): `crates/api/src/booleanadmission_projection.rs`, `crates/api/src/booleanadmission_projection_tests.rs`
- month_change (1 / 1): `crates/api/src/month_change.rs`
- sweeprun_status_cache (1 / 1): `crates/api/src/sweeprun_status_cache_tests.rs`
- booleanlaunch_work (1 / 0): `crates/api/src/booleanlaunch_work.rs`
- logs_units (1 / 0): `crates/api/src/logs_units_tests.rs`
- server_cloud_mutant_tests_d (1 / 0): `crates/api/src/server_cloud_mutant_tests_d.rs`

## In no group

- `crates/api/src/lib.rs` (NOT PROVED: mod items only; no object holds it): ASA's, for a module a shard must declare (the shard writes `STATUS: BLOCKED OUTSIDE` with the exact line).
- `crates/api/benches/ratio.rs` and the 21 files under `crates/api/benches/ratio/` (NOT PROVED: `test = false`, L-BCA-2): the operator's decision; no session edits them.
