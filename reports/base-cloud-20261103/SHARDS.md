# SHARDS.md: who edits what (3 Nov 2026 BASE cloud wave)

## COV-LIBS attempt 5: crate shards (PLAN 4.1)

A shard edits only files under its crates' `src/`, `tests/` and `benches/`; a new test module is declared in the production file it tests. `docs/`, `.github/`, `Cargo.toml`, `Cargo.lock` and `crates/*/src/lib.rs` are LSA's: put drafts in your report, and a need there is `STATUS: BLOCKED OUTSIDE`. Line and side counts are the 28 Oct figures (`fix-BASE-COV-LIBS-progress.md`), stale by attempts 2-4.

| shard | crates | uncovered lines / sides (stale) |
|---|---|---|
| LS1 | store | 1,820 / 673 |
| LS2 | runner | 1,675 / 1,032 |
| LS3 | lake, indicators, vocab | 784 / 526 |
| LS4 | core, costs, engine, greeks, telemetry | 325 / 208 |

## MSUM-BACKFILL: MS1's files

The nine files the branch changed over f01d7a0e4 (`git diff --stat f01d7a0e4 4d7f446f7a81`), plus new test files declared in one of them: `crates/store/src/month_summary_writer.rs`, `crates/store/src/sort_index_writer.rs`, `crates/store/src/month_summary_tests.rs`, `crates/store/src/file_fault_tests.rs`, `crates/pull/src/upkeep.rs`, `crates/pull/src/upkeep_tests.rs`, `crates/cli/src/upkeep_command.rs`, `crates/cli/src/upkeep_command_tests.rs`, `crates/cli/src/msum_builder.rs`.

## COV-PULL: file groups

PS1's step (1) may also edit the 34 files repair 2 changed (its brief). The groups:

### COV-PULL file groups for the D6 shards (3 Nov 2026, repair 1 of the cloud-wave PLAN)

Source: the per-file table of `state/fix/round2/deep-BASE-COV-PULL-c9d653d5f914-gaps.txt` (check r2's Route B on c9d653d5f914): 196 files, 6949 uncovered lines, 2694 uncovered sides. Made by node (scratchpad groups2.mjs): a stem is a file with its `_tests`/`_cov_tests`/`_cover_tests` files; stems sorted by lines + sides, each to the lightest group. Figures are before repair 2's uncompiled tests (f98a899398f6), so they size the groups, not the work left.

Rules: a shard edits only its group's files and NEW test files for them (a new file belongs to the group of the file it tests); never `crates/pull/src/lib.rs`, `Cargo.toml`, `Cargo.lock`, `docs/` or `.github/` (PSA's); a file not listed below belongs to G1 (PS1).

## G1 (session BASE-COV-PULL-PS1): 2258 lines + 957 sides, 46 stems

- gdfl_nfo (802 / 156): `crates/pull/src/gdfl_nfo.rs`, `crates/pull/src/gdfl_nfo_tests.rs`
- master_snapshots (209 / 54): `crates/pull/src/master_snapshots.rs`, `crates/pull/src/master_snapshots_cov_tests.rs`, `crates/pull/src/master_snapshots_tests.rs`
- cash_session_cache (190 / 40): `crates/pull/src/cash_session_cache.rs`
- fno_membership (134 / 64): `crates/pull/src/fno_membership.rs`, `crates/pull/src/fno_membership_tests.rs`
- gdfl_import (61 / 105): `crates/pull/src/gdfl_import.rs`, `crates/pull/src/gdfl_import_cov_tests.rs`, `crates/pull/src/gdfl_import_tests.rs`
- reference_data (110 / 48): `crates/pull/src/reference_data.rs`, `crates/pull/src/reference_data_cov_tests.rs`, `crates/pull/src/reference_data_tests.rs`
- ssm (84 / 43): `crates/pull/src/ssm.rs`, `crates/pull/src/ssm_cov_tests.rs`
- gdfl_import_day (97 / 28): `crates/pull/src/gdfl_import_day.rs`
- gdfl_run_locks (81 / 22): `crates/pull/src/gdfl_run_locks.rs`, `crates/pull/src/gdfl_run_locks_cov_tests.rs`
- vendor (35 / 51): `crates/pull/src/vendor.rs`, `crates/pull/src/vendor_cov_tests.rs`
- cash_auction (76 / 3): `crates/pull/src/cash_auction.rs`
- master_snapshots_first_listed (42 / 36): `crates/pull/src/master_snapshots_first_listed.rs`, `crates/pull/src/master_snapshots_first_listed_tests.rs`
- manifest (43 / 17): `crates/pull/src/manifest.rs`
- unit (28 / 32): `crates/pull/tests/unit.rs`
- request_minutes (26 / 27): `crates/pull/src/request_minutes.rs`
- upkeep (12 / 27): `crates/pull/src/upkeep.rs`, `crates/pull/src/upkeep_tests.rs`
- pricing (16 / 19): `crates/pull/src/pricing.rs`
- calendar_tradable (19 / 15): `crates/pull/src/calendar_tradable_tests.rs`
- gdfl_r3_attack (15 / 15): `crates/pull/src/gdfl_r3_attack_tests.rs`
- nse (12 / 16): `crates/pull/src/nse.rs`
- census (12 / 14): `crates/pull/tests/census.rs`
- expiry_alias (8 / 18): `crates/pull/src/expiry_alias.rs`
- fetch (19 / 6): `crates/pull/src/fetch.rs`
- gdfl_r4_attack (9 / 14): `crates/pull/src/gdfl_r4_attack_tests.rs`
- gdfl_run_stop (16 / 6): `crates/pull/src/gdfl_run_stop.rs`, `crates/pull/src/gdfl_run_stop_cov_tests.rs`
- folder (9 / 11): `crates/pull/src/folder.rs`, `crates/pull/tests/folder.rs`
- gdfl_cm_auction (16 / 3): `crates/pull/src/gdfl_cm_auction.rs`
- capture (12 / 6): `crates/pull/src/capture.rs`
- refusal (12 / 5): `crates/pull/src/refusal.rs`, `crates/pull/tests/refusal.rs`
- rehearsal (7 / 7): `crates/pull/src/rehearsal.rs`, `crates/pull/src/rehearsal_tests.rs`
- token_identity (3 / 10): `crates/pull/tests/token_identity.rs`
- sealed_month (4 / 7): `crates/pull/tests/sealed_month.rs`
- gdfl_cm_auction_v2 (5 / 5): `crates/pull/src/gdfl_cm_auction_v2_tests.rs`
- allocation (6 / 3): `crates/pull/tests/allocation.rs`
- gdfl_import_causes (9 / 0): `crates/pull/src/gdfl_import_causes.rs`
- scrub (5 / 3): `crates/pull/src/scrub.rs`
- request_gap_events (1 / 6): `crates/pull/tests/request_gap_events.rs`
- census_day_pace (1 / 4): `crates/pull/tests/census_day_pace.rs`
- listing (3 / 2): `crates/pull/src/listing.rs`, `crates/pull/src/listing_tests.rs`
- series_identity (1 / 4): `crates/pull/src/series_identity.rs`
- verdict_words (1 / 3): `crates/pull/src/verdict_words.rs`
- identity_screen (2 / 1): `crates/pull/src/identity_screen.rs`
- universe (3 / 0): `crates/pull/src/universe.rs`
- alerts_seams (1 / 0): `crates/pull/tests/alerts_seams.rs`
- gdfl_fixtures (1 / 0): `crates/pull/src/gdfl_fixtures.rs`
- support (0 / 1): `crates/pull/tests/support/mod.rs`

## G2 (session BASE-COV-PULL-PS2): 2391 lines + 823 sides, 62 stems

- gdfl_rekey (336 / 129): `crates/pull/src/gdfl_rekey.rs`, `crates/pull/src/gdfl_rekey_tests.rs`
- census_store (222 / 112): `crates/pull/src/census_store.rs`, `crates/pull/src/census_store_tests.rs`
- store_evidence (308 / 22): `crates/pull/tests/store_evidence.rs`
- gdfl_run (211 / 28): `crates/pull/src/gdfl_run.rs`, `crates/pull/src/gdfl_run_cov_tests.rs`, `crates/pull/src/gdfl_run_tests.rs`
- gdfl_ia3 (209 / 20): `crates/pull/src/gdfl_ia3_tests.rs`
- landing_run (164 / 33): `crates/pull/src/landing_run.rs`, `crates/pull/src/landing_run_tests.rs`
- gdfl_v3 (131 / 31): `crates/pull/src/gdfl_v3.rs`, `crates/pull/src/gdfl_v3_tests.rs`
- alerts (100 / 53): `crates/pull/src/alerts.rs`, `crates/pull/src/alerts_tests.rs`
- response_keep (78 / 39): `crates/pull/src/response_keep.rs`, `crates/pull/src/response_keep_tests.rs`
- calendar (59 / 40): `crates/pull/src/calendar.rs`, `crates/pull/src/calendar_cov_tests.rs`
- census_store_kept (81 / 11): `crates/pull/src/census_store_kept_tests.rs`
- disk_projection (70 / 8): `crates/pull/src/disk_projection.rs`, `crates/pull/src/disk_projection_tests.rs`
- fnowork_futures (56 / 17): `crates/pull/src/fnowork_futures_tests.rs`
- zerodha_spool (37 / 29): `crates/pull/src/zerodha_spool.rs`, `crates/pull/src/zerodha_spool_cov_tests.rs`
- gdfl_import_filing (41 / 14): `crates/pull/src/gdfl_import_filing.rs`
- gdfl_dayzips (49 / 4): `crates/pull/src/gdfl_dayzips.rs`, `crates/pull/src/gdfl_dayzips_tests.rs`
- repair_plan (27 / 12): `crates/pull/src/repair_plan.rs`
- gdfl_run_ledger (26 / 11): `crates/pull/src/gdfl_run_ledger.rs`, `crates/pull/src/gdfl_run_ledger_cov_tests.rs`
- cloud_mutants_import (18 / 14): `crates/pull/tests/cloud_mutants_import.rs`
- gdfl_r2_attack (11 / 19): `crates/pull/src/gdfl_r2_attack_tests.rs`
- fno (17 / 12): `crates/pull/src/fno.rs`
- zerodha_gaps (6 / 22): `crates/pull/tests/zerodha_gaps.rs`
- chain (14 / 11): `crates/pull/src/chain.rs`
- gdfl_scope (8 / 16): `crates/pull/src/gdfl_scope_tests.rs`
- fold (13 / 10): `crates/pull/src/fold.rs`
- zerodha_identity (14 / 9): `crates/pull/tests/zerodha_identity.rs`
- emit_sites (14 / 5): `crates/pull/src/emit_sites.rs`
- gdfl_cm (9 / 10): `crates/pull/src/gdfl_cm.rs`
- broker (10 / 8): `crates/pull/tests/broker.rs`
- gdfl_seconds_attack (2 / 15): `crates/pull/src/gdfl_seconds_attack_tests.rs`
- padded_refold (14 / 0): `crates/pull/tests/padded_refold.rs`
- gdfl_import_quality (5 / 8): `crates/pull/src/gdfl_import_quality.rs`
- gdfl_backfill (6 / 6): `crates/pull/src/gdfl_backfill_tests.rs`
- riskfree (6 / 4): `crates/pull/src/riskfree.rs`
- census_resume (4 / 5): `crates/pull/tests/census_resume.rs`
- gdfl_sealed (2 / 7): `crates/pull/src/gdfl_sealed_tests.rs`
- census_store_link (1 / 6): `crates/pull/src/census_store_link_tests.rs`
- gdfl_format_change (1 / 6): `crates/pull/src/gdfl_format_change_tests.rs`
- csv (3 / 3): `crates/pull/src/csv.rs`
- padded (2 / 3): `crates/pull/src/padded.rs`, `crates/pull/src/padded_tests.rs`
- daycheck (0 / 4): `crates/pull/src/daycheck.rs`
- gdfl_import_scope (0 / 4): `crates/pull/src/gdfl_import_scope.rs`
- zerodha_repair (3 / 1): `crates/pull/tests/zerodha_repair.rs`
- base_cov_words (1 / 1): `crates/pull/src/base_cov_words_tests.rs`
- gdfl_import_census (1 / 1): `crates/pull/src/gdfl_import_census.rs`
- gdfl_format (1 / 0): `crates/pull/src/gdfl_format.rs`
- audit_ledger (0 / 0): `crates/pull/src/audit_ledger.rs`, `crates/pull/src/audit_ledger_tests.rs`
- base_cov_codes (0 / 0): `crates/pull/src/base_cov_codes_tests.rs`
- concurrency (0 / 0): `crates/pull/tests/concurrency.rs`
- config (0 / 0): `crates/pull/src/config.rs`
- dhan (0 / 0): `crates/pull/src/dhan.rs`
- futures_refusal_events (0 / 0): `crates/pull/tests/futures_refusal_events.rs`
- gaps_not_owed (0 / 0): `crates/pull/src/gaps_not_owed_tests.rs`
- gdfl_wp11c (0 / 0): `crates/pull/src/gdfl_wp11c_tests.rs`
- groww (0 / 0): `crates/pull/src/groww.rs`
- lib (0 / 0): `crates/pull/src/lib.rs` (listed for the counts only: 0 uncovered lines and sides at c9d653d5f914; no shard edits it, rule above; PSA does if a module must be declared)
- nseindex (0 / 0): `crates/pull/src/nseindex.rs`
- rate (0 / 0): `crates/pull/src/rate.rs`
- scrub_blocks (0 / 0): `crates/pull/src/scrub_blocks_tests.rs`
- secret (0 / 0): `crates/pull/src/secret.rs`
- tls (0 / 0): `crates/pull/tests/tls.rs`
- work (0 / 0): `crates/pull/src/work.rs`

## G3 (session BASE-COV-PULL-PS3): 2300 lines + 914 sides, 46 stems

- gdfl_dropbox (301 / 115): `crates/pull/src/gdfl_dropbox.rs`, `crates/pull/src/gdfl_dropbox_tests.rs`
- zerodha_verify (247 / 150): `crates/pull/src/zerodha_verify.rs`, `crates/pull/src/zerodha_verify_tests.rs`
- tickstore_write (256 / 45): `crates/pull/src/tickstore_write.rs`, `crates/pull/src/tickstore_write_tests.rs`
- http (162 / 94): `crates/pull/src/http.rs`
- ingest (168 / 55): `crates/pull/src/ingest.rs`, `crates/pull/src/ingest_cov_tests.rs`
- gdfl_run_ia2 (164 / 34): `crates/pull/src/gdfl_run_ia2_tests.rs`
- masters (126 / 39): `crates/pull/src/masters.rs`
- gdfl_lineage (122 / 23): `crates/pull/src/gdfl_lineage.rs`, `crates/pull/src/gdfl_lineage_tests.rs`
- gdfl_wp12e (117 / 5): `crates/pull/src/gdfl_wp12e_tests.rs`
- gdfl_import_ia2 (102 / 11): `crates/pull/src/gdfl_import_ia2_tests.rs`
- census_v4 (79 / 2): `crates/pull/tests/census_v4.rs`
- gdfl_archive (69 / 9): `crates/pull/src/gdfl_archive.rs`
- gdfl_nfo_attack (37 / 36): `crates/pull/src/gdfl_nfo_attack_tests.rs`
- gdfl_ia16 (16 / 49): `crates/pull/src/gdfl_ia16_tests.rs`
- gdfl_cm_admission (52 / 6): `crates/pull/src/gdfl_cm_admission_tests.rs`
- corporate_actions (15 / 35): `crates/pull/src/corporate_actions.rs`, `crates/pull/src/corporate_actions_cov_tests.rs`, `crates/pull/src/corporate_actions_tests.rs`
- rolling (24 / 16): `crates/pull/src/rolling.rs`
- gdfl_import_lineage (18 / 18): `crates/pull/src/gdfl_import_lineage.rs`
- resolve (29 / 6): `crates/pull/src/resolve.rs`
- derive (12 / 17): `crates/pull/tests/derive.rs`
- gdfl_import_journal (13 / 15): `crates/pull/src/gdfl_import_journal.rs`
- gdfl_tickstore (18 / 9): `crates/pull/src/gdfl_tickstore.rs`
- gdfl_run_start (24 / 2): `crates/pull/src/gdfl_run_start.rs`
- pipeline (14 / 10): `crates/pull/tests/pipeline.rs`
- gdfl_import_split (13 / 10): `crates/pull/src/gdfl_import_split_tests.rs`
- gap_backfill (10 / 12): `crates/pull/src/gap_backfill.rs`
- no_scheduler_delete (5 / 16): `crates/pull/tests/no_scheduler_delete.rs`
- session (11 / 8): `crates/pull/src/session.rs`
- cash_session_cache_lock (11 / 7): `crates/pull/src/cash_session_cache_lock_tests.rs`
- anchor (6 / 8): `crates/pull/tests/anchor.rs`
- gaps (6 / 8): `crates/pull/src/gaps.rs`
- gdfl_census_stop (5 / 8): `crates/pull/tests/gdfl_census_stop.rs`
- archive (8 / 4): `crates/pull/src/archive.rs`
- fnowork (10 / 0): `crates/pull/src/fnowork.rs`, `crates/pull/src/fnowork_cov_tests.rs`
- totp (5 / 5): `crates/pull/src/totp.rs`
- gdfl_bad_row_audit (6 / 3): `crates/pull/tests/gdfl_bad_row_audit.rs`
- day_universe (0 / 8): `crates/pull/src/day_universe.rs`
- gdfl_run_scope (6 / 1): `crates/pull/src/gdfl_run_scope.rs`
- tenor (4 / 2): `crates/pull/src/tenor.rs`
- repair_plan_not_owed (2 / 3): `crates/pull/src/repair_plan_not_owed_tests.rs`
- futures_events (1 / 3): `crates/pull/tests/futures_events.rs`
- ingest_failkind (1 / 3): `crates/pull/src/ingest_failkind.rs`, `crates/pull/src/ingest_failkind_tests.rs`
- gdfl_ia4 (1 / 2): `crates/pull/src/gdfl_ia4_tests.rs`
- kite (3 / 0): `crates/pull/src/kite.rs`
- gdfl_series_code (0 / 2): `crates/pull/src/gdfl_series_code_tests.rs`
- integration (1 / 0): `crates/pull/tests/integration.rs`

