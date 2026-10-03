# Pass 3 slice p3rest: cli files and api files no earlier pass covered

Target: origin/final/all-fixes-zero @ 5140aca. Read only. No probes were run because no candidate finding survived reading.
I skipped the floor-then-max-gated ppm class, because the other pass-3 agent is sweeping it.

## Verdict: NO NEW FINDINGS

The cli files in this slice are almost all codecs, journals and orchestration: binary record readers and writers, checkpoint chains, page joins. They do no price or statistic arithmetic. Every size, count and offset step I read uses `checked_*` with a named refusal, or a `saturating_*` on a display or telemetry counter that cannot realistically reach its limit. The api files in this slice turn already-computed values into JSON strings and page through them. They do not compute any statistic again.

## Checked and clean

### cli
- `anchored_search_lineage_v2/v3/v4.rs`: record counts come from `checked_mul`. The `count = len / width` helper refuses a ragged file first (`is_multiple_of`) and then checks the explicit maximum.
- `admission_join.rs` `join_page` (:355-420): expected = `total.saturating_sub(offset).min(limit)`. The row sequence uses `checked_add`. Totals and lengths are cross-checked.
- `admission_store.rs` `page` (:808-860): reads O(limit) rows by direct block address with a fixed row ceiling. `require_generations_unchanged` compares file generations; it does not rehash the file. Record counts check for a ragged tail (:1275, :1360, :1632).
- `expression.rs` `read`: the body is `checked_sub(HEADER+FOOTER)` and must be a multiple of 56. Rows are verified twice, and counts are sealed.
- `expression_search.rs`: the run loop's budget subtractions only see values that grow. Counters use `checked_add`. `verify_transition` limits the work delta to 4096 or less. `verify_history` is resume-only and bounded by `HISTORY_LIMIT`.
- `boolean_search_projection.rs` `upper_ppm` (:102-110): uses `checked_mul` then `div_ceil`, so the probability rounds up into a max-gated ceiling. That direction is correct.
- `boolean_candidate_grid.rs`: the codec checks that levels are positive and strictly increasing, that the grid has cells, that the horizon is above zero, and that first ≤ last.
- `boolean_candidate_v1.rs` `Sessions::new/observe` (:948-1078): day offsets use `checked_sub`. A trade must enter and exit on the same IST day. Per-session sums use `checked_add` and must reconcile exactly with the cell's (pessimistic, trades, wins). No look-ahead: periods are attributed by the trade's own exit day.
- `boolean_grammar_batch.rs`: capacity is `checked_mul/add`. The node budget uses `checked_sub`. The file length must equal HEADER + n·ENCODED_LEN exactly.
- `boolean_rung_scope.rs`, `boolean_observation.rs`, `index_stop_search_progress.rs`: no arithmetic beyond `count_ones` and scheduling.
- `index_stop_search_checkpoint.rs`: the node allowance and ordinals use `checked_sub`. The chain length must equal the journal's acknowledged count. Replaying the whole chain on cold resume is a bounded verification step, not a per-operation cost.
- `ledger_v6.rs`, `strict_v6_inputs.rs`: the support floor is computed only from the training span's bar count (`min_hits_for`, the known clib-1 class). The OOS span is a separate request that must start strictly after the training months (`ledger_v6.rs:676`), so the in-sample floor cannot see OOS bars.
- `batch.rs`: the `Tally` counters saturate, but they are report totals only. Every month row has its own identity, and the ladder uses `with_min_hits(min_hits)` plus a ceiling.

### api (excluding server/sweeprun/autopilot/render/ingest/livejson and *_tests)
- `bars.rs` `rupees` (:61-86): uses `unsigned_abs`, so `i64::MIN` cannot panic. The sign is kept for sub-rupee values (-5 gives "-0.05"). `ist_clock` uses `div_euclid` and `rem_euclid`.
- `backtest.rs` ledger read (:1135-1160): `total = body/stride`. A ragged tail is reported, not served. The read is O(take), walking backward.
- `detail.rs` `Page`/`seek_window` (:245-475): `page*limit` is checked with `checked_mul`. Addressability is refused with a `div_ceil` suggestion. Window end is `saturating_add(...).min(total)`.
- `frontierjson.rs`: `total_admitted` walks the run's rows once per request. Those rows are already capped at `MAX_RESULT_ROWS` and already produced by `of_run_against_receipt` in the same request, so this adds no extra order of cost.
- `indexstoprankingjson.rs`, `indexstopqualificationjson.rs`, `index_consistency_projection.rs`: page windows are `saturating_add().min(total)` after an offset ≤ total check. Byte admissions use `checked_mul`. `later_days = sessions.len().checked_sub(training_days)` and refuses on underflow.
- `boolean_observation_budget.rs` (u128 aggregate), `boolean_search_budget.rs`, `booleanlaunch_work.rs`, `candidatejson.rs` (`checked_add`), `trades.rs` (`exit_bar.saturating_sub(entry_bar)` is display only; entry ≤ exit is enforced upstream), `census.rs`, `ladder.rs`, `coverage.rs`, `calendar.rs`, `calendar_of.rs`, `recovery.rs` (micros/1e6 after an exact-minute `rem_euclid` check), `merge.rs`, `catalog.rs`, `master.rs`, `mastersrun.rs`, `constituents.rs`, `audit.rs`, and the remaining *json.rs serializers: only telemetry `as u64` widenings, vendor index casts on small enums, and string formatting of values that are already integers.
