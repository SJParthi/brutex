# lane4-items.tsv (copied verbatim from /mnt/project-files/brutex-lanes/lane4-items.tsv)

```
rederive	C3	claimed by another lane	pull::ingest derive_all/reconcile_derived and committed_cash_days incremental: O(new minutes + one bucket) per ingest, derived files byte-identical…
resample	C3	in PR #35 (merge pending push)	runner::resample anchored at the 09:15 open like pull::fold; the int64-edge saturation made loud; stale "IST midnight" statements corrected.
poolcols	C3	worker running	Every fixed-width table renderer in crates/cli and api text renderers separates every column, sized for i64 paisa extremes, header aligned; a test…
resume	C3	lane 2 PR #23	engine resume accepts a different support-lane count (scheduling only) without weakening any check that guards the answer; byte-identical survivors…
store-02	C4	dup of PR #26	2 findings · bug · store/src/checksum_audit.rs, store/src/header.rs
gate8	C3	lane 1	Gate 8 (crates/*/benches/ratio.rs) times the real production functions, scaled by what their cost can grow with, and FAILS under a deliberate O(n) or…
indicators	C3	lane 3	Exact integer VWAP sigma (no floats), refuse an all-zero previous-day anchor loudly, fold the gap family by 3 clock minutes, through one append-only…
purity	C3	lane 1	The Rust-only guards (Gates 1, 1f, 1g, 2, 13, 15, workspace_is_rust.rs, .github/one_language.rs) turn CI red on every violation spelling; D-0706…
cli-14	C4	in PR #35	4 findings · cost · cli/src/population_finalization_v3.rs, cli/src/population_finalization_v4.rs, cli/src/population_v5.rs
lookahead	C3	in PR #35	Every quantity a trade at bar N uses depends only on bars 0..N (prefix-only cadence; excursion look-ahead fixed the same way); appending future bars…
api-01	C4	in PR #35	4 findings · cost, bug, test-gap · api/src/audit_json.rs, api/src/bars.rs, api/src/booleanqualification_projection.rs
core-01	C4	in PR #35	3 findings · bug · core/src/vendor.rs, core/src/price.rs
costs-01	C4	in PR #35	4 findings · bug · costs/src/expiry.rs, costs/src/fill.rs
docs-web-01	C4	in PR #35	4 findings · bug, cost · docs/02-store-format.md, docs/03-vocabulary.md
engine-01	C4	in PR #35	4 findings · bug · engine/src/column.rs, engine/src/keep.rs
indicators-01	C4	in PR #35	3 findings · bug · indicators/src/column.rs, indicators/src/trend.rs
pull-01	C4	in PR #35	4 findings · cost, bug · pull/src/archive.rs, pull/src/csv.rs, pull/src/fno.rs
runner-01	C4	in PR #35	4 findings · cost, bug · runner/src/admission.rs, runner/src/bootstrap.rs, runner/src/exit_grid_policy.rs
store-01	C4	in PR #35	4 findings · bug · store/src/catalog.rs
telemetry-01	C4	in PR #35	4 findings · bug, doc-false, cost · telemetry/src/tail.rs, telemetry/src/event.rs, telemetry/src/lib.rs
vocab-01	C4	in PR #35	4 findings · cost, bug · vocab/src/expression_search.rs, vocab/src/expression.rs
cli-01	C4	in PR #35	4 findings · test-gap, cost · cli/build_provenance.rs, cli/src/and_checkpoint.rs, cli/src/boolean_campaign.rs
lake-01	C4	in PR #35	4 findings · bug, test-gap · lake/src/contract.rs, lake/src/error.rs, lake/src/reader.rs
cli-02	C4	in PR #35	4 findings · cost, bug · cli/src/boolean_grammar_campaign.rs, cli/src/boolean_oos_v1.rs, cli/src/boolean_search_command.rs
cli-06	C4	in PR #35	4 findings · bug, cost · cli/src/population_v6.rs, cli/src/selection_v6_source.rs, cli/src/trades.rs
cli-10	C4	in PR #35	4 findings · bug, cost · cli/src/fold_audit.rs, cli/src/global_replay.rs, cli/src/global_replay_v3.rs
cli-18	C4	in PR #35	2 findings · cost · cli/src/strict_v6_inputs.rs, cli/src/trades.rs
pull-05	C4	in PR #35	4 findings · test-gap, cost, bug · pull/src/gaps.rs, pull/src/http.rs
pull-09	C4	in PR #35	4 findings · doc-false, bug, law · pull/src/session.rs, pull/src/ssm.rs, pull/src/totp.rs
api-04	C4	in PR #35	4 findings · cost, test-gap, bug · api/src/indexstoprankingjson.rs, api/src/indexstopvixjson.rs, api/src/ingest.rs
runner-03	C4	in PR #35	4 findings · bug, cost · runner/src/rank.rs, runner/src/resolved_grid_view.rs, runner/Cargo.toml
vocab-02	C4	in PR #35	4 findings · bug, cost · vocab/src/expression_search.rs, vocab/src/implication.rs, vocab/src/lib.rs
cli-03	C4	in PR #35	4 findings · cost, bug · cli/src/boolean_statistics_v1.rs, cli/src/execution_v3.rs, cli/src/fold_audit.rs
cli-07	C4	in PR #35	4 findings · bug, cost · cli/src/admission_store.rs, cli/src/anchored_search_lineage_v2.rs, cli/src/anchored_search_lineage_v3.rs
cli-11	C4	in PR #35	4 findings · bug, cost · cli/src/global_replay_v4.rs, cli/src/index_stop_store.rs, cli/src/institutional_evidence.rs
cli-15	C4	in PR #35	4 findings · cost, bug · cli/src/population_v6.rs, cli/src/selection.rs, cli/src/selection_v5.rs
pull-02	C4	in PR #35	4 findings · bug, cost · pull/src/http.rs, pull/src/resolve.rs
pull-06	C4	in PR #35	4 findings · bug, doc-false, cost · pull/src/http.rs, pull/src/manifest.rs
pull-10	C4	in PR #35	3 findings · cost, bug, test-gap · pull/src/universe.rs, pull/src/work.rs, pull/tests/unit.rs
api-05	C4	in PR #35	4 findings · cost · api/src/pullrun.rs, api/src/recovery.rs
runner-04	C4	in PR #35	4 findings · cost, bug · runner/src/bootstrap.rs
vocab-03	C4	in PR #35	1 finding · cost · vocab/src/expression_search.rs
costs-02	C4	in PR #35	1 finding · bug · costs/src/scope.rs
cli-04	C4	in PR #35	4 findings · bug, cost · cli/src/frontier.rs, cli/src/index_stop_search_checkpoint.rs, cli/src/institutional_statistics.rs
cli-08	C4	in PR #35	4 findings · cost, bug · cli/src/anchored_search_lineage_v4.rs, cli/src/audited_range.rs, cli/src/boolean_candidate_grid.rs
cli-12	C4	in PR #35	4 findings · bug, cost · cli/src/institutional_evidence.rs, cli/src/institutional_statistics.rs, cli/src/knobs.rs
cli-16	C4	in PR #35	4 findings · cost, doc-false · cli/src/selection_v5.rs, cli/src/selection_v6.rs, cli/src/step3_orchestrator.rs
pull-03	C4	in PR #35	4 findings · bug, cost · pull/src/rolling.rs, pull/src/archive.rs, pull/src/calendar.rs
pull-07	C4	in PR #35	4 findings · bug, cost · pull/src/nse.rs, pull/src/nseindex.rs, pull/src/rate.rs
api-02	C4	in PR #35	4 findings · bug, cost · api/src/ingest.rs, api/src/pullrun.rs, api/src/recovery.rs
api-06	C4	in PR #35	2 findings · cost · api/src/recovery.rs
runner-05	C4	in PR #35	4 findings · bug, cost · runner/src/closed.rs, runner/src/exit_grid_policy.rs
telemetry-02	C4	in PR #35	3 findings · bug · telemetry/src/sink.rs, telemetry/src/tail.rs
docs-web-02	C4	in PR #35	1 finding · bug · web/src/routes/backtest/+page.svelte
cli-05	C4	in PR #35	4 findings · bug · cli/src/population_admission_v3.rs, cli/src/population_admission_v4.rs, cli/src/population_finalization_v3.rs
cli-09	C4	in PR #35	4 findings · cost, test-gap · cli/src/boolean_search_projection.rs, cli/src/boolean_statistics_v1.rs, cli/src/execution_capability.rs
cli-13	C4	in PR #35	4 findings · cost, bug · cli/src/population_admission_writer.rs, cli/src/population_base_evidence_ledger_v2.rs, cli/src/population_base_evidence_v2.rs
cli-17	C4	in PR #35	4 findings · test-gap, cost, bug · cli/src/step3_orchestrator.rs, cli/src/stored_data_completeness.rs, cli/src/stored_post_training_oos.rs
pull-04	C4	in PR #35	4 findings · bug, doc-false · pull/src/calendar.rs, pull/src/chain.rs, pull/src/config.rs
pull-08	C4	in PR #35	4 findings · cost, bug · pull/src/refusal.rs, pull/src/request_minutes.rs, pull/src/rolling.rs
api-03	C4	in PR #35	4 findings · bug, cost · api/src/verify.rs, api/src/booleanqualification_projection.rs, api/src/indexmap.rs
runner-02	C4	in PR #35	4 findings · cost · runner/src/exit_grid_policy.rs, runner/src/expression_execution.rs, runner/src/expression_oos.rs
runner-06	C4	in PR #35	2 findings · bug · runner/src/rank.rs, runner/src/significance.rs
engine-02	C4	in PR #35	2 findings · cost, bug · engine/src/keep.rs
```
