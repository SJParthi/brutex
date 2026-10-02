# Cloud fix lane 1, second set

Handoff only. Branch `fix-queue` is never merged. Source: the batch-2 audit queue, 2 Oct 2026. Base every fix on origin/main. Do this file after lane1.md. Decision numbers for this lane come from the D-0910 to D-0959 range shared by the cloud lanes.

**Done means:** a test that fails before the fix and passes after; `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` and the CI gates green; for a cost finding, the operation is O(1) or the honest bound is named in docs/06-limits.md; extreme cases (empty, one, max, overflow, rerun, corrupt input) covered by tests. One branch and one PR per item, named `fix/cloud-<id>`.

## W2-cli3-0 · high cost · cli

**Where:** `crates/cli/src/candidate_universe.rs:4890`

**Finding:** append_validated_grid_rows (per exit cell -> runner::grid::materialize_cell) (crates/cli/src/candidate_universe.rs:4890): per one exit-grid cell of one (closed mask, side), the cost is Theta(E) per cell: a HashMap of E entries plus two E+1 prefix vectors (SliceFacts) and a full E walk, rebuilt for every cell; it grows with E = evaluated one-minute execution bars (plus trades x holding). Auditor verdict: undocumented-scan. Documented: Partly. The rustdoc at candidate_universe.rs:2717-2724 says 'Every grid cell is then replayed over its exact execution series by materialize_cell ... Neither the per-cell replay/folds nor the whole operation is O(1)'.

**Evidence:**
  - candidate_universe.rs:4861 is `for ordinal in 0..evaluated.grid().cells.len()`. runner grid.rs:2768 `materialize_cell` calls `per_trade` (2696), which calls `levelled` (3465). `SliceFacts::of` (trade.rs:407-455) does all of this for each call: `walk_over` goes to `walk_core` (trade.rs:675), which iterates `column.bits().iter().zip(column.sources())`, so O(E).
  - The loop is `candidate_universe.rs:4861` `for ordinal in 0..evaluated.grid().cells.len()`. `grid.rs:2768` `materialize_cell` calls `per_trade` (2778).

**Expected fix and test:** None

## W2-cli3-7 · high bug · cli

**Where:** `crates/cli/src/candidate_universe.rs:1578`

**Finding:** CandidateSearchColumnBuilderV1::build (crates/cli/src/candidate_universe.rs:1578): IST session boundary: a Search V4 fold prefix ending on the stored short final bar of a session (rungs 2/10/30/60 min, since 375 session minutes do not divide by 2, 10, 30 or 60). The 60-minute bar stamped 15:15 holds only 15:15-15:29. Code path: Exact code path. The store emits the short final bucket: stored.rs:1245-1246 expects buckets bucket_index_v2(window.from)..=bucket_index_v2(window.to) anchored at 555, with pull calendar.rs:133 `LAST_MINUTE: u16 = 15 * 60 + 29`, so (929-555)/60 = 6 gives a 15:15 bucket; pull fold.rs:175-179 says 'The last bar covers 15:15-15:30, is stamped correctly'.

**Evidence:**
  - *The code.** In crates/cli/src/candidate_universe.rs:1560-1566 the builder computes `final_close_minute = bar.ts_micros.saturating_add(signal_length_micros).saturating_sub(60_000_000)` with no clamp. The loop at 1568-1577 advances only over minutes `<= final_close_minute`.
  - *The refusal.** In crates/cli/src/candidate_universe.rs, lines 1560-1566 compute `final_close_minute = bar.ts_micros.saturating_add(signal_length_micros).saturating_sub(60_000_000)`. Lines 1578-1587 then require the last visited minute of `reference_minute_context` to equal that value, or return Err("candidate Search V4 lacks exact closing minute ...").

**Expected fix and test:** None

## GAP13-13 · medium bug · 

**Where:** `crates/cli/src/batch.rs:321-352, 568-572, 658-676; also pool.rs:90-96, 262-269; lib.rs:14713-14744, 7041-7061`

**Finding:** Parallel stored commands (sweep-all, pool pass 1, range-all) write durable ledger/evidence rows from inside rayon workers, so row order and attempt-token assignment follow thread completion; `cli results` differs between identical runs 

**Evidence:**
  - R
  - o
  - w

**Expected fix and test:** Fix:
1. Allocate every attempt in input order before the parallel map: `sweep_evidence::begin_many` already exists (sweep_evidence.rs:498) and allocates a group's tokens in one journal append.
2. Keep the pure computation in the workers.
3. After the indexed collect, commit sequentially in input order: record_swept_run, the frontier, trade and detail-receipt appends, and attempt.finish. For range-all and pool, return the prepared commit from `one_rung` instead of performing it inside the worker.
4. Correct the four comments quoted in the description.

Alternative if the commit must stay in the workers: make the readers order-independent. `results_at` sorts by (feed, underlying, timeframe, from, to, identity), and `best_complete_newest_first` breaks ties by identity, not append order. Then 

## GAP4-46 · medium law · 

**Where:** `crates/cli/src/lib.rs:18024 (knobs_checked); 5133-5140 grid_rungs; 1444-1451 reference_price; 4917-4921 max_stop_points; 19017/19267/19293 both_shapes; runner/src/validate.rs:4631-4637`

**Finding:** Walk-forward training exit grid is sized from the whole span, test windows included (fold rung count look-ahead) 

**Evidence:**
  - C
  - o
  - d

**Expected fix and test:** Fix:
- Resolve the fold rung count from each fold's own training slice. Replace `rungs: usize` on walk_forward_core and the `*_with_rungs` doors with `FoldRungs::{Fixed(usize), PerTraining(&(dyn Fn(&[Candle]) -> usize + Sync))}`.
- cli passes `Fixed(n)` when BRUTEX_GRID_RUNGS is set, else `PerTraining(grid_rungs)`, which runner calls on `train.0`'s signal slice before `Levels::derived`.
- Record the per-fold count on FoldResult.
- Append a new identity term naming the fold-rung policy rather than reinterpreting term 17, because identity is positional and append-only.
- Correct validate.rs:4736.

Test (fails before, passes after):
- Promote the probe. Two spans identical up to the last fold's test window. Assert that every earlier fold's recorded training rung count and whole FoldResult are

## GAP13-15 · medium bug · 

**Where:** `crates/cli/src/pool.rs:599-673 (647-648, 668), 818`

**Finding:** pool pass 2 prices on the coarse signal bars with a 15-bar horizon, unlike pass 1; at 60min it reports "no candidate fired" while pass 1 shows 83-94 trades per instrument 

**Evidence:**
  - `
  - p
  - o

**Expected fix and test:** Fix: prepare pass 2 exactly as the screen and audit path do. Build the anchored column, project it onto the 1-minute execution series (Sourced::Fill, as audit_bars_work/StoredReplay does), and call `grid::evaluate_over` over the execution bars with the horizon counted in execution minutes. If that cannot be done yet, refuse coarse rungs in pass 2 by name instead of printing "no candidate fired".

Test (fails today, passes after): an end-to-end pool over a generated fixture at 60min. For every instrument whose pass-1 outcome is Ok, price that run's recorded (mask_words, side) through `price_all` on the same instrument and assert `trades > 0`. Today 0 of 97 candidates fire on any instrument. Also replace `the_pool_prepares_a_span_exactly_as_the_screen_does` with a behavioural assertion (see 

## GAP11-0 · medium bug · 

**Where:** `crates/cli/src/search_checkpoint.rs:262-266 (marker created, then written); 459-465 (discover counts any marker file as acknowledged latest); 488-490 (read refuses width); consumers: and_checkpoint.rs:70, expression_search.rs:259, boolean_campaign.rs:419, boolean_grammar_campaign.rs:162, boolean_qualified_journal.rs:119, boolean_search_command.rs:225, index_stop_search.rs:401`

**Finding:** A kill (or ENOSPC) between creating a checkpoint's `complete` marker and writing its seal blocks every rerun of that search for good 

**Evidence:**
  - C
  - o
  - m

**Expected fix and test:** Fix: publish the marker atomically. Use `File::create_new(dir.join("complete.tmp"))`, write_all(seal), sync_all, then `fs::rename(tmp, dir.join("complete"))`, then sync the reservation directory and remove the temp file on error. discover_through only looks up `<reservation>/complete`, so a stray complete.tmp counts as an interrupted reservation (:469-473) and the previous checkpoint stays the resume point.
Test that fails before and passes after: build a reservation N holding the fully synced payload plus the state a kill leaves (today an empty `complete`; after the fix an empty `complete.tmp`). Assert that Journal::open gives interrupted()==1, latest() returns checkpoint N-1, and a republish succeeds as N+1. Today it refuses with "checkpoint marker width mismatch". Add a source-order tes

## GAP11-1 · medium bug · 

**Where:** `crates/cli/src/sweep_evidence.rs:1364-1397 (append_events, write at 1383-1386); 1282-1298 (append_row); 626-631 (Attempt::ranked writes rows one at a time); 1188-1190 (shape header); callers: lib.rs:3371 (sweep-stored), lib.rs:18431 (audit), expression.rs:80, expression_search.rs:253, batch.rs:568, boolean_*_v1.rs, index_stop*.rs`

**Finding:** sweep_evidence never rolls back its own partial write: one short write in attempts.bin blocks every later attempt in the store 

**Evidence:**
  - R
  - e
  - p

**Expected fix and test:** Fix: in append_events, append_row, shape's header write and Attempt::ranked, measure `end = file.seek(SeekFrom::End(0))` under the held lock. On any write_all error, call `file.set_len(end)` and return a refusal saying whether the rollback held (the D-0426 wording, as in frontier.rs:1086-1093). In `ranked`, encode every row into one buffer and write it once.
Test that fails before and passes after: add a write seam (for example `append_events_with(path, encode, write)`, mirroring frontier's `append_locked_with` and its test `a_partial_frontier_write_rolls_back_to_the_last_whole_row`). The injected writer writes 16 bytes and then returns Err. Assert attempts.bin keeps its pre-call length and the next `sweep_evidence::begin` of another identity returns Ok; today it refuses with "torn or shor

## W2-cli2-6 · medium cost · cli

**Where:** `crates/cli/src/candidate_trades.rs:739`

**Finding:** pinned (via candidates_page, tier, TradeReader::open) (crates/cli/src/candidate_trades.rs:739): per one candidate-manifest page / tier read / trade-reader open, the cost is O(C) per page, run twice (re-read and hash the whole catalog, then rebuild the full index); it grows with total captured candidates C across all tiers (catalog.bin is 32 bytes per candidate side). Auditor verdict: undocumented-scan. Documented: no. Module header line 7 says 'Pages are bounded; cold verification and all retained history are not O(1)', which reads as a bound on page cost.

**Evidence:**
  - *What each call costs.** `fn pinned` (candidate_trades.rs:739) calls `read_model`, then compares `held.digest != summary.digest`. `read_model` at :652 does `let (payload, digest) = read_sealed(&path, model.catalog(), max_bytes)?`, and `read_sealed` (:1217-1251) allocates `payload_len`, calls `read_exact`, then `hash.update(&payload)`.
  - `fn pinned` (crates/cli/src/candidate_trades.rs:739) calls `read_model(...)`, which does `read_sealed(&path, model.catalog(), max_bytes)?` (652). read_sealed (1217-1250) reads the whole payload into a Vec, blake3-hashes all of it and re-checks the file generation. `TradeReader::page` uses only FileGeneration checks (987-1003), so it is O(rows).

**Expected fix and test:** None

## W2-cli2-9 · medium cost · cli

**Where:** `crates/cli/src/candidate_trades.rs:375`

**Finding:** Capture::record_inner (crates/cli/src/candidate_trades.rs:375): per one evaluated screen candidate x direction, the cost is O(cells + materialize + trades) plus about 4 fsyncs per candidate side; it grows with grid cells (re-selection and linear contains), bars/signals (materialize) and trades; plus 2 durable files with fsync, directory fsync and full read-back. Auditor verdict: needs-measurement. Documented: module header 'UNVERIFIED performance' (line 1). No limits entry and no measurement.

**Evidence:**
  - candidate_trades.rs:375 is `if evaluated.selected != crate::shown_cell(evaluated.grid, tier.rules)`. `shown_cell` (lib.rs:11475-11484) scans the cells up to three times: `best_within` twice and `best` once. :398 is `if !evaluated.grid.cells.contains(&cell)`, a linear scan of the cells. :401 is `self.materialize(...)`, which replays one cell through `per_trade`. :429-434 call `write_exact` twice.
  - `shown_cell` (lib.rs:11475-11484) runs up to three `best_within` scans over the grid's cells (grid.rs:1012). The caller already computed the same value at lib.rs:11690 (`let shown = shown_cell(&g, rules);`), so this is a deliberate second, independent check.

**Expected fix and test:** None

## W2-cli3-1 · medium cost · cli

**Where:** `crates/cli/src/candidate_universe.rs:4785`

**Finding:** expand_population_side (per closed mask x side) (crates/cli/src/candidate_universe.rs:4785): per one (closed mask, direction) grid evaluation, the cost is Theta(S+M+D+E) re-hashing plus about five Theta(E) attestation passes per candidate side, on top of the grid's own work; it grows with S signal + M minute-context + D daily + E execution bars (identity and attestation passes that are the same for every candidate). Auditor verdict: undocumented-scan. Documented: No. The module header (lines 30-39) and §147 describe the cost as 'proportional to the naturally extinct candidate population'. Neither says every candidate re-hashes all streams and re-attests the slice.

**Evidence:**
  - candidate_universe.rs:2812-2813 calls expand_population_member from inside `sweeper.run_prepared_population_by_reporting(signal_column, on_level, |member| ...)`, once per retired population member. For every Closed member, :4653-4732 calls expand_population_side twice, once Long and once Short.
  - candidate_universe.rs:4785 calls ExecutionRunV1::new_with_daily_reference once per closed mask per side, and :4800 calls evaluate_training_grid_attested the same way. The constructor (exit_grid_policy.rs:310-316) runs data_digest_with_daily_reference.

**Expected fix and test:** None

## W2-cli3-2 · medium cost · cli

**Where:** `crates/cli/src/candidate_universe.rs:4548`

**Finding:** replay_execution_side (Execution V3 replay, per mask-direction group) (crates/cli/src/candidate_universe.rs:4548): per one authenticated (mask, direction) group during Execution V3 replay, the cost is Theta(S+M+D+E) per group, repeated for every closed mask x side; it grows with S+M+D+E bars (candidate-invariant hashing and attestation). Auditor verdict: undocumented-scan. Documented: No. build_execution_v3_replay_authority's rustdoc (4226-4231) describes re-evaluating each grid, not re-hashing every stream for each group.

**Evidence:**
  - In `build_execution_v3_replay_authority`, the loop at candidate_universe.rs:4352-4383 calls `replay_execution_side` twice per closed mask, once for Long and once for Short. Work in `new_with_daily_reference` (exit_grid_policy.rs:303-318, identity.rs:527-622):
  - The loop at candidate_universe.rs:4352-4383 calls replay_execution_side once for Long and once for Short in every mask group. ExecutionRunV1::new_with_daily_reference (candidate_universe.rs:4548) recomputes data_digest_with_daily_reference over S+M+D (exit_grid_policy.rs:310; identity.rs:527-575).

**Expected fix and test:** None

## W2-cli3-4 · medium cost · cli

**Where:** `crates/cli/src/candidate_universe.rs:3801`

**Finding:** append_produced_candidate_universe_v1 (production append door) (crates/cli/src/candidate_universe.rs:3801): per one receipt-last Candidate universe append, the cost is 2 x Theta(R_total + C) full re-validation (seek+read, seal hash, semantic hash and payload re-encode per historical row) plus O(new rows) per append; it grows with R_total rows and C completions already in the ledger (all history), not just the new block. Auditor verdict: undocumented-scan. Documented: Contradicted. Module header 14-15 says 'the sealed internal append is O(new rows)', and docs/06-limits.md §150 (line ~7583-7584) says 'Append, hashing, canonical-order validation and durability are proportional to the ne ...

**Evidence:**
  - `read_row`, an unbuffered `seek` plus `read_exact` of 480 bytes (6010-6030) ledger_v6.rs:330 passes `root: source_root.as_path()` for every rung and family (8 x 2 = 16 appends per run). all_rung_population_v5.rs:586 and 610 pass `roots.source.path()` for NIFTY and BANKNIFTY at every rung.
  - crates/cli/src/candidate_universe.rs:3801 `let mut ledger = CandidateUniverseLedgerV1::open(root, bounds)?;` is followed by `append_complete` and then, at 3806, `CandidateUniverseLedgerV1::open_read(root, bounds)?.reopen_audit(...)`. `scan` (3339-3402) runs `for index in 0..receipt_count { ... `fn open` is private (3220), and `append_and_reopen` (2667-2676) is the only caller of this door.

**Expected fix and test:** None

## W2-cli7-2 · medium cost · cli

**Where:** `crates/cli/src/ledger_v6.rs:667`

**Finding:** replay_route -> run_route (crates/cli/src/ledger_v6.rs:667): per one ledger-v6-replay invocation (and every ledger-v6 rerun), the cost is O(full Step-4 route) per invocation even when every authority is already committed; it grows with span bars x candidates: all 8 rungs x 2 families re-load strict inputs and re-run Search V4 before reuse is decided. Auditor verdict: undocumented-scan. Documented: no - docs/06-limits.md mentions Global Replay V4 record sizes (lines 8427-8430) but not that replay recomputes the route

**Evidence:**
  - `crates/cli/src/ledger_v6.rs:667`: `replay_route` calls `let selected: [_; 8] = run_route(request, out)?`. calls `strict::size_sweeper` at :309, which does a full strict `load` of NIFTY (`strict_v6_inputs.rs:63`); then, for each of the 2 `ROUTE_FAMILIES`, calls `commit_strict_candidate_pre_admission_authority_v1` at :328.
  - ledger_v6.rs:667 `let selected: [_; 8] = run_route(request, out)?`. run_route (ledger_v6.rs:289-414) walks all 8 `LEDGER_RUNGS` x 2 `ROUTE_FAMILIES`. For each one it calls `commit_strict_candidate_pre_admission_authority_v1` (:328), then `commit_stored_population_v6_route` and `render_selection`.

**Expected fix and test:** None

## W2-cli7-3 · medium cost · cli

**Where:** `crates/cli/src/ledger_v6.rs:309`

**Finding:** run_route -> strict_v6_inputs::size_sweeper (crates/cli/src/ledger_v6.rs:309): per sizing one rung's support threshold, the cost is O(M + B + source bytes) per rung, duplicated; it grows with M months x B bars of the NIFTY span: a full cold checksum-audited load only to read signal.bars.len(), then the same span is loaded again by the NIFTY family commit; guards re-checked 4x per rung at O(M). Auditor verdict: undocumented-scan. Documented: partly - §151 gives load cost O(M+B) and the strict-admission section gives O(source bytes); the duplicate load is not stated

**Evidence:**
  - ledger_v6.rs:309 calls `size_sweeper` once per rung. At strict_v6_inputs.rs:63-77 it runs `let context = load(StoredContextLoadSpecV1 { underlying: "NIFTY", ... The loop at ledger_v6.rs:326-328 then calls `commit_strict_candidate_pre_admission_authority_v1` for "NIFTY" and "BANKNIFTY".
  - strict_v6_inputs.rs:63-76 calls `load(StoredContextLoadSpecV1 { vendor, underlying: "NIFTY", rung_name: rung, from, to, signal_bound: bounds.signal_records, ... That goes through RangeInputs::load_bounded (strict_v6_inputs.rs:105). For every month, and for the month before the span, it runs checksum_receipts::audit_month (audited_range.rs:252).

**Expected fix and test:** None

## W2-cli8-0 · medium cost · cli

**Where:** `crates/cli/src/lib.rs:11266`

**Finding:** screen_cascade (crates/cli/src/lib.rs:11266): per one tier of the generated tier ladder (after the operator's rules admit nothing and validate=true), the cost is O(M + C*2*grid(V,trades)) per tier; O(T*(M + C*grid)) per cascade once tiers are actually walked; it grows with priced candidates C (<= screen_cap <= SCREEN_CAP_CEILING 10,000,000) x 2 sides x exit-grid variants x trades on the 1-min execution series M, plus O(M) per-tier setup. Auditor verdict: false-o1-claim. Documented: Contradicted in code: lib.rs:10998-11006 says "Why the grid is built once and the tiers only re-filter ... eight tiers cost eight passes over cells already computed rather than eight sweeps".

**Evidence:**
  - crates/cli/src/lib.rs:11266 is `for (rank, tier) in ladder.iter().enumerate() { let rules = tier.rules(top, reference); let body = screen(bars, column, by_evidence, horizon, rules, pricing)?;`. runner grid.rs:2204-2208 merges that value into the stop ladder. The doc comment at 10998-11006 is therefore false: "eight tiers cost eight passes over cells already computed rather than eight sweeps ...
  - (1) lib.rs:11266-11268: `for (rank, tier) in ladder.iter().enumerate() { let rules = tier.rules(top, reference); let body = screen(bars, column, by_evidence, horizon, rules, pricing)?;`.

**Expected fix and test:** None

## W2-cli8-7 · medium cost · cli

**Where:** `crates/cli/src/lib.rs:12310`

**Finding:** measure_top (crates/cli/src/lib.rs:12310): per one measured screen row (calendar-consistency measurement), the cost is O(grid + trades*7) per row, over up to R <= 10,000,000 rows single-threaded; it grows with per row: a full exit-grid rebuild (`grid::evaluate_over`) + per_trade + 7-grain bucketing, single-threaded. Row count = min(8*BRUTEX_TOP, R), and BRUTEX_TOP has no upper bound. Auditor verdict: unbounded-per-request. Documented: Only in a code comment (lib.rs:12286-12309) that records a 3-hour single-threaded stall and says the loop is "BOUNDED BY WHAT CAN BE PRINTED". docs/06-limits.md has no hit for measure_top or measured_band.

**Evidence:**
  - lib.rs:12207-12211: `const fn measured_band(top)` gives `top.saturating_mul(8)`, with a floor of 32. lib.rs:12310: `for row in rows.iter_mut().take(measured_band(rules.top))` is a plain sequential for loop, not rayon. lib.rs:8771 sets `top: usize::try_from(at("BRUTEX_TOP", 25)).unwrap_or(25)`. strict_range_knobs.rs:64-66 only checks `count > 0`.
  - Loop: lib.rs:12310 `for row in rows.iter_mut().take(measured_band(rules.top))`. Each row rebuilds a full grid: lib.rs:12328 `grid::evaluate_over(... &facts_again)`, then lib.rs:12343 calls `consistency_of`. `consistency_of` calls `grid::per_trade` (lib.rs:12412), then runs `stability::at` once per grain for 7 grains. Each pass is O(trades), with no sort and no map (stability.rs:258-265).

**Expected fix and test:** None

## W2-cli8-8 · medium bug · cli

**Where:** `crates/cli/src/lib.rs:11271`

**Finding:** screen_cascade (crates/cli/src/lib.rs:11271): Tier ladder reports the STRICTEST tier as MET when nothing was admitted. The mildest-tier probe can never prove 'no tier admits'. Code path: `screen` returns `selected = final_selection(&rows, rules)` (11999). final_selection falls back to any row that traded: `.find(|row| row.admitted && row.cell.trades > 0).or_else(|| rows.iter().find(|row| row.cell.trades > 0))` (10358-10360). The cascade still treats `selected` as admission: mildest probe `if widest.selected.is_none()` (11245) and tier loop `if body.selected.is_none() { ... UNMET` (11271).

**Evidence:**
  - `final_selection` (crates/cli/src/lib.rs:10326) no longer requires admission. Lines 10358-10360 read `.find(|row| row.admitted && row.cell.trades > 0).or_else(|| rows.iter().find(|row| row.cell.trades > 0))`. `screen` sets `selected = final_selection(&rows, rules)` (lib.rs:11999) and computes `admitted_any` separately (12002).
  - The filter at 11724-11725 is `.filter(|&(_, _, cell, _)| cell.trades > 0)`. `shown_cell` (11475-11483) always falls back to `g.best()`, so a row is not dropped for failing the rules. `final_selection` (10358-10360) is `.find(|row| row.admitted && row.cell.trades > 0).or_else(|| rows.iter().find(|row| row.cell.trades > 0))`. It prints `"  {:<8} MET -- {}"` with `Tier::label(0)` = "S++++++" (9591-9593).

**Expected fix and test:** None

## W2-cli8-9 · medium bug · cli

**Where:** `crates/cli/src/lib.rs:14983`

**Finding:** latest_for (via one_rung) (crates/cli/src/lib.rs:14983): Idempotent rerun: a range-all/descend row shows ANOTHER run's figures (identity not tied to output). Code path: A rerun of identity A takes `Committed::Reused` (16572) and appends nothing. The report says 'RESULT ALREADY RECORDED AND VERIFIED' (16726-16733), which is neither a refusal nor NOT_RECORDED, so one_rung calls `latest_for(vendor_word, underlying, rung, from, to, min_hits)` (13190). That returns the newest row matching the key (14985-14993), not the identity.

**Evidence:**
  - `ensure_run_record` (crates/cli/src/lib.rs:16552-16599) returns `Committed::Reused(index)` when `store.holds(&record.identity)` is true and the deterministic fields match. For that case `record_run` renders "RESULT ALREADY RECORDED AND VERIFIED" (16726-16733). `refusal_reason` (13810-13846) does not match that text.
  - That reads the row at `store.index_of_identity(...)`, compares it with `same_run_answer`, and returns `Ok(Committed::Reused(index))` (crates/cli/src/lib.rs:16553-16578). The report says "RESULT ALREADY RECORDED AND VERIFIED" (16726-16733). Existing tests show an exact retry reaches this state (audited_range_tests.rs:524, audited_stored_tests.rs:262).

**Expected fix and test:** None

## AC-whp-law-0 · medium bug · cli

**Where:** `crates/cli/src/lib.rs:3307`

**Finding:** The stored doors turn the typed word into the canonical key (`Symbol::new` upper-cases it), and the RunId is built from `loaded.key`. The ledger row, however, records the raw typed word. `stored_month_kernel` takes `underlying` straight from the request and passes it into `Recording { underlying, .. }`, and `record_run` stores `results::field(into.underlying)`. The ledger deduplicates by identity and accepts a repeat only when every field except `finished_micros` matches (`same_run_answer`).

So `cli sweep-stored zerodha nifty 1min 2026 1` followed by `cli sweep-stored zerodha NIFTY 1min 2026 1` computes one RunId over the same bars, with the same commit and the same ladder. The second run reaches `ensure_run_record` -> `verify` and is refused with "run … is already in the ledger, but its deterministic fields differ from this exact rerun". Because `finish_stored_month` propagates the error, the whole command prints `refused: RESULT NOT RECORDED: …`. That breaks §3 rule 5 ("reruns are safe") and the promise of the store's own test `a_lower_case_instrument_reads_the_same_month_as_its_canonical_name`: "one file, one answer, whatever case the operator happened to type".

There is a sec

**Evidence:**
  - fix/c2-final:crates/cli/src/stored.rs:2167 has `let key = swept_index(underlying)?;`. That path goes through `InstrumentKey::index/cash`, which calls `Symbol::new` (core/src/instrument.rs:273-293). `stored_month_kernel` then builds the RunId from `instrument: &loaded.key` (lib.rs:3352).
  - The chain on fix/c2-final, for sweep-stored: - lib.rs:2058 passes the raw `under` word through. `sweep_stored_inner` (lib.rs:3120) and then `stored_month_kernel` destructure it unchanged (lib.rs:3307-3316). stored.rs:2180 says so, and stored.rs:3534-3550 is a test that supports lower case: "nifty" loads the same bars and `key.underlying` comes back as "NIFTY".

**Expected fix and test:** None

## W2-cli9-3 · medium bug · cli

**Where:** `crates/cli/src/minute_gaps.rs:195`

**Finding:** days_with_interior_gaps (as used by pool::price_all pool.rs:631-646 and screen_range_inner lib.rs:15228-15244) (crates/cli/src/minute_gaps.rs:195): A hole at the start or end of a session (for example 15:25-15:29 missing on day D while other days hold them), or a day with no minutes at all but with signal bars, is never withheld. The exact-minute overlay then refuses the whole instrument span, which is the failure this module exists to prevent. Code path: the gap test is `same_day && step > MINUTE_MICROS` between adjacent bars (minute_gaps.rs:192-195). The last minute of D (15:24) is followed by D+1 09:15, so `same_day` is false and D is never pushed. In indicators/src/anchored.rs:815-851, for the 5-min signal bar opening 15:25 on D, `demanded` = 15:29. `latest_session_micros` is 15:29, taken from other days, so `expected = demanded`.

**Evidence:**
  - minute_gaps.rs:192-195 is `let same_day = indicators::ist_day(before.ts_micros) == day; ... When day D stops early, the step from D's last minute to the next day's 09:15 fails `same_day`, so D is never withheld. Coarse rungs are folded from the minutes at ingest (pull ingest.rs:2352 `crate::fold::fold(bars, bucket)`). So if D ends at 15:26, a 5-min bar opening 15:25 still exists.
  - minute_gaps.rs:192-195 flags a day only when `same_day && step > MINUTE_MICROS`. If 15:29 is missing on day D, the bar after D 15:28 is D+1 09:15, so `same_day` is false and D is never flagged.

**Expected fix and test:** None

## W2-cli9-5 · medium bug · cli

**Where:** `crates/cli/src/operation_audit.rs:621`

**Finding:** read / page (crates/cli/src/operation_audit.rs:621): A crash after `create_new` of the per-invocation journal but before its first synced record leaves a 0-byte journal. `read` then errors, although a missing journal returns the unconfirmed start, and every page that covers that ID refuses permanently. Code path: The journal is created at operation_audit.rs:561-566 `options().read(true).append(true).create_new(true).open(own(&base, id))`, and the start record is written at 567 `write_synced(&mut file, &image)?`. On read, the NotFound arm returns `Ok(Some(started))` (615), but `if bytes == 0 || at(&mut file, 0)? != started { return Err("invocation journal lost its indexed start") }` (620-623).

**Evidence:**
  - begin writes and syncs the index row, then releases the lock (`write_synced(&mut *index, &image)?; index.release()...`, lines 559-560). It then creates the journal with `.create_new(true).open(own(&base, id))` (561-566). write_synced is `write_all` followed by `sync_all` (348-351). `length()` (307-315) accepts 0 bytes, because it refuses only `!is_file() || len % STRIDE != 0`.
  - How the code produces it: `begin` (crates/cli/src/operation_audit.rs:548-566) syncs the Started record into index.bin with `write_synced(&mut *index, &image)?` and releases the lock. != started { return Err(error("invocation journal lost its indexed start")); }` (lines 620-623). `page` (lines 663-668) collects the `read` results into a `Result`, so one Err refuses the whole page.

**Expected fix and test:** None

## W2-cli9-4 · medium bug · cli

**Where:** `crates/cli/src/pool.rs:1436`

**Finding:** tests::the_pool_prepares_a_span_exactly_as_the_screen_does (crates/cli/src/pool.rs:1436): The sequence-pinning test passes on text outside the function it claims to pin. It does not prove that pool pass 2 prepares a span exactly as the screen does. Code path: The search is `lib[last_screen..].find(step)` with no end bound. `stored_anchored_column(` occurs 0 times inside screen_range_inner (lib.rs:15101-15321, counted with awk). The test matches it at lib.rs:18389, a different function, and then matches `horizon_for(` at lib.rs:20394. The screen actually builds its column inside audit_bars through StoredReplay, while pool.rs:640 calls stored_anchored_column directly.

**Evidence:**
  - The test is at crates/cli/src/pool.rs:1415-1449. At this commit screen_range_inner (lib.rs:15101-15133) is a thin wrapper around screen_range_kernel (lib.rs:15150-15321). `stored_anchored_column(` never appears in 15101-15321. The search matches it at lib.rs:18389, inside `fn audit_bars_work(` (18367): `(None, Some(replay)) => match stored_anchored_column(`.
  - Both searches have no end bound: `lib[last_screen..].find(step)` (pool.rs:1437) and `pool[last_pool..].find(step)` (pool.rs:1436). *Screen side.** `screen_range_inner` (lib.rs:15101-15131) now only hands off to `screen_range_kernel` (lib.rs:15150-15321). Steps 1-8 do land inside the kernel, at lib.rs:15170-15244. `stored_anchored_column(` appears nowhere in 15101-15321.

**Expected fix and test:** None

## R9-cli-o1-0 · medium bug · cli

**Where:** `crates/cli/src/pool.rs:289`

**Finding:** The memory bound that SWEEPS_SHARING_THIS_MACHINE exists to enforce does not hold for pool pass 1. The pass runs `crate::one_rung` for every surface instrument under `surface.par_iter()`, so up to rayon::current_num_threads() sweeps run at once (the rayon pool has no global limit set anywhere in crates/). Nothing on this path raises the sharing counter: `SharedBy::these` is called only at batch.rs:348 (sweep-all) and lib.rs:14628 (sweep_rungs / range-all). The counter therefore reads 1 inside every concurrent pool sweep, and each one gets:
(a) the whole-machine candidate ceiling. one_rung -> audit_range_for_attempt -> audit_range_kernel -> `ladder_for(min_hits)` (lib.rs:6611) -> ladder_within -> `ceiling_from_env()` -> `shared_out(ceiling_asked()?)`. shared_out divides by `SWEEPS_SHARING_THIS_MACHINE`, which is 1 here, so the result is whole_machine_ceiling (lib.rs:15939-15944), about 19.6 GB by the engine's own table on the 14-core reference machine.
(b) every core as support lanes: `.with_support_lanes(shared_support_lanes())` (lib.rs:16087-16094) = cores / 1.
(c) on the derived-support path, an affordability probe of `whole_machine_ceiling().checked_div(PROBE_SHARE)` (lib.rs:128

**Evidence:**
  - On fix/c2f-r-cli, pool.rs:288-296, `run_under` pass 1 is `surface.par_iter().map(|symbol| Screened { .. The only production writers of SWEEPS_SHARING_THIS_MACHINE are two `SharedBy::these` calls: batch.rs:348 (sweep-all) and lib.rs:14628 (sweep_rungs / range-all). The static starts at 1 (lib.rs:15742) and Drop resets it to 1.
  - Nothing limits it: - `git grep "SharedBy::these(\|SWEEPS_SHARING_THIS_MACHINE.store"` finds production call sites only at batch.rs:348 (sweep-all: `SharedBy::these(wanted.len().min(rayon::current_num_threads()))`) and lib.rs:14628 (`sweep_rungs`: `SharedBy::these(rungs.len())`). The other matches are the guard itself (lib.rs:15778/15785) and tests (lib.rs:21802+). `LEDGER` is taken only in `record_all_attempt` (lib.rs:19472), and `knobs::serially` is cfg(test).

**Expected fix and test:** None

## W2-cli12-0 · medium cost · cli

**Where:** `crates/cli/src/population_statistics_v2.rs:3675`

**Finding:** build_raw_candidates (crates/cli/src/population_statistics_v2.rs:3675): per one candidate summary during Statistics V2 preparation, the cost is O(C*(P+S)) per candidate; O(C^2*(P+S)) per preparation; it grows with whole-family period rows C*P plus split rows C*S, for every one of C candidates. Auditor verdict: undocumented-scan. Documented: No. docs/06-limits.md §154 (7672-7685) states only that OPENING validates O(C*(P+S)) rows. 'quadratic' appears only in §47 and §158 (Admission V2). PreparedPopulationStatisticsV2::new (3751) has no cost doc.

**Evidence:**
  - Inside that loop, lines 3675-3684 walk both complete vectors: `periods.iter().copied().filter(|period| period.candidate_sequence == sequence_u64).collect()`, and the same for `splits`. The reopen path already does this in one pass with per-candidate hashers: `validate_period_records` (2813; `period_hashers` at 2824-2835) and `validate_split_records` (2917). ledger_all.rs:1036 `commit_population`
  - `periods` holds C*P entries: `build_raw_periods` reserves `candidate_count.checked_mul(period_count)` and pushes period-major (3568-3599). `splits` holds C*S entries, also split-major (`build_raw_splits`, 3602-3638). `commit_stored_observation_statistics_v2` (step3_orchestrator.rs:2378) `commit_all_rung_with_verified_build_v5` (all_rung_population_v5.rs:651), once per rung over 8 rungs

**Expected fix and test:** None

## W2-cli12-2 · medium cost · cli

**Where:** `crates/cli/src/population_statistics_v2.rs:2226`

**Finding:** PopulationStatisticsV2Ledger::open_inner (crates/cli/src/population_statistics_v2.rs:2226): per one ledger open (two per append, plus every reader open), the cost is O(max_audits) allocation and control-byte initialisation per open; it grows with the configured ceiling bounds.audits, not the audits actually stored (production 1<<24). Auditor verdict: undocumented-scan. Documented: No. In 06-limits the reserve/try_reserve hits are lines 123-127 (another crate) and 3532-3534 only.

**Evidence:**
  - CODE: crates/cli/src/population_statistics_v2.rs:2224-2227 runs `let mut audits = HashMap::new(); audits.try_reserve(usize_of(bounds.audits, "audit bound")?)` on every open_inner call, before scan() has counted any records. In production the first argument of `PopulationStatisticsV2Bounds::new` is `max_audits` (lines 1060-1087).
  - crates/cli/src/population_statistics_v2.rs:2224-2227 does `let mut audits = HashMap::new(); audits.try_reserve(usize_of(bounds.audits, "audit bound")?)` on every open_inner, and scan() fills that map afterwards. Production passes `PopulationStatisticsV2Bounds::new(CEILING_RECORDS, ...)` (ledger_all.rs:1077-1078), with `pub(crate) const CEILING_RECORDS: u64 = 1 << 24;` (ledger_all.rs:1230).

**Expected fix and test:** None

## W2-cli13-4 · medium bug · cli

**Where:** `crates/cli/src/results.rs:708`

**Finding:** open_result_file (Results::open_read / open_read_bounded); also result_set.rs:280/298 Receipts::open_read(_bounded), pre_admission_data.rs:3601 open_file (crates/cli/src/results.rs:708): A FIFO planted at results/runs.bin or results/detail-sets.bin, or at a pre-admission lock or data path, makes every read-only open block forever instead of refusing. This includes the HTTP path api/detail.rs:463-471 -> CommittedParents::open_read_bounded. Code path: results.rs:708-713 `OpenOptions::new().read(true).write(writable).create(writable).truncate(false).open(path)` has no O_NONBLOCK and no pre-open type check. result_set.rs:280 is plain `File::open(&path)`. I compiled and ran a probe using the identical OpenOptions on a mkfifo path (scratchpad o1audit-cli13/fifo.rs): 'STILL BLOCKED after 2.00s'.

**Evidence:**
  - (1) crates/cli/src/results.rs:707-713 `open_result_file` opens with `OpenOptions::new().read(true).write(writable).create(writable).truncate(false).open(path)`. The only file-type check, `file.metadata()...is_file()` at 724-727, runs after open() has already returned. (2) crates/cli/src/result_set.rs:280 and 298 call a plain `File::open(&path)` for results/detail-sets.bin.
  - crates/cli/src/results.rs:707-713. `is_file()` is checked only after `open` returns (:728-731), and a non-regular file just goes without a lock (`else { None }`, :748). `Results::open_read` and `open_read_bounded` (:814, :828) reach it through `open_with` (:855-862). crates/cli/src/result_set.rs:280 and :298. crates/cli/src/pre_admission_data.rs:3601.

**Expected fix and test:** None

## W2-cli13-5 · medium bug · cli

**Where:** `crates/cli/src/search_checkpoint.rs:213`

**Finding:** Journal::publish_inner / discover_through (crates/cli/src/search_checkpoint.rs:213): Publishing past DIRECTORY_LIMIT: publish acknowledges checkpoint number 1,000,000 (Ok is returned), but every later Journal::open or Snapshot::open refuses the namespace, so the search is permanently unresumable. Code path: publish_inner (:213-286) creates `format!("{sequence:016x}")` with no comparison against DIRECTORY_LIMIT. discover_through :432 `if index == DIRECTORY_LIMIT { return Err("checkpoint directory admission limit exceeded") }` counts owner.lock plus every reservation. expression_search.rs:551 publishes once per candidate, bounded only by caller budgets. Grepping it for next_sequence or DIRECTORY found no guard;

**Evidence:**
  - crates/cli/src/search_checkpoint.rs:14 sets `pub(crate) const DIRECTORY_LIMIT: usize = 1_000_000;`. `publish_inner` (:213-286) checks only three things: the acknowledgment counter, the byte admission (`length.checked_add(96).is_none_or(|n| n > max_bytes)`) and sequence overflow (`self.next.checked_add(1)`). format!("{sequence:016x}"))` at :228-229.
  - Journal::publish_inner (crates/cli/src/search_checkpoint.rs:213-286) takes `let sequence = self.next;` and increments it with checked_add (:223-227). It then runs `fs::create_dir(&directory)` on `format!("{sequence:016x}")` (:228-229) and returns `Ok((sequence, seal))` (:285). Its only checks are the acknowledgment-counter overflow (:214-217), the owner file generation (:218) and the byte admission (:220).

**Expected fix and test:** None

## GAP15-21 · low doc-false · 

**Where:** `crates/cli/src/ledger_v6.rs:245, 613; also ledger_all.rs:826, boolean_catalog_prepared.rs:49`

**Finding:** STORED_PROVENANCE's single-instrument, single-month and run-identity promise heads the ledger-v6, ledger-v6-replay, ledger-all and Boolean research pages too, not only pool (extends R9-cli-law-3) 

**Evidence:**
  - l
  - i
  - b

**Expected fix and test:** Fix: add a multi-instrument stored banner that still says REAL MARKET DATA, so it is not the generated banner, but that promises no single instrument or month. Either name the per-family Candidate universe identities as the run-identity carriers, or print the nine-term run identities per family and rung. Use it on the ledger-v6, ledger-v6-replay, ledger-all and Boolean research pages. Test: extend `the_generated_and_stored_banners_make_opposite_claims` with the new banner. Add a page test asserting that ledger_v6 and ledger_v6_replay pages do not contain 'describes that instrument and that month' unless they print a run identity for every instrument they report.

## GAP11-3 · low bug · 

**Where:** `crates/cli/src/lib.rs:18386-18399 (record_unadmitted); compare record_all_attempt 19562-19572 (LEDGER mutex + ResultSetLock) and 19632-19639 (confirm_result_directory before record_run)`

**Finding:** record_unadmitted publishes the ledger row without the result-set lock and without the directory barrier the commit protocol requires 

**Evidence:**
  - l
  - i
  - b

**Expected fix and test:** Fix: run record_unadmitted under the same LEDGER.lock() plus ResultSetLock::acquire as record_all_attempt, and call confirm_result_directory(into.root) after ensure_detail_receipt and before record_swept_run.
Test that fails before and passes after: extend the existing source-order test at lib.rs:23849-23870. It pins `if let Err(why) = confirm_result_directory(into.root)` before `record_run(` in record_all_attempt; require that record_unadmitted's body names ResultSetLock::acquire and confirm_result_directory before `record_swept_run(`.

## GAP13-14 · low doc-false · 

**Where:** `crates/cli/src/lib.rs:16426-16428, 16571-16573, 16605-16607`

**Finding:** An idempotent rerun of a completed audit says its frontier, trades and detail receipt came "from an interrupted attempt" 

**Evidence:**
  - a
  - u
  - d

**Expected fix and test:** Fix: word the Reused branches neutrally, e.g. "already present for this exact identity; byte-verified and reused". Or pass in whether the results ledger already holds the identity (`Results::open_read(root)?.holds(&identity)`) and say "from the recorded run" versus "from an interrupted attempt" accordingly.

Test:
- In the exact-retry test at audited_range_tests.rs:524 (or audited_stored_tests.rs:262), assert that the second report contains "RESULT ALREADY RECORDED AND VERIFIED" and does not contain "interrupted attempt".
- Add a case where the frontier and trade blocks were written but the ledger append was not (simulated interruption), and assert the interrupted wording appears there.

## GAP16-25 · low bug · 

**Where:** `crates/cli/src/lib.rs:7784-7785 (origin/main 7538-7539)`

**Finding:** cli top-combinations report truncates the stored mean (milli-paisa to paisa) and t (milli to hundredths) toward zero: the bias D-0287 names, and it disagrees with the other two renderings 

**Evidence:**
  - C
  - o
  - d

**Expected fix and test:** Fix: add one integer helper `div_round_half_away(n: i64, d: i64) -> i64`, or the §7 half-up variant, and use it for both reductions. Or render three decimals of the stored thousandths instead of reducing them. Keep the sign for values in (-1, 0).

Test in cli lib.rs tests: render_top_record with a frontier::Row of mean_milli_paisa 47_600, t_milli 2_999 must contain "₹0.48" and "3.00". A row of mean_milli_paisa -600 must contain "-₹0.01". It fails today ("₹0.47", "2.99", "₹0.00") and passes after.

## GAP13-16 · low bug · 

**Where:** `crates/cli/src/pool.rs:771, 786`

**Finding:** The pooled table prints `net` and `dd>=` with no separator, so a 5-digit drawdown runs into the net and reads as one number 

**Evidence:**
  - F
  - r
  - o

**Expected fix and test:** Fix: add a separator and widen the column, e.g. `{:>12} {:>9}` in both the header (pool.rs:771) and the row (pool.rs:786).

Test: call render_pooled with one Pooled row where net = -16462 and dd_bound = 123456. Assert the rendered row contains "-16462" and "123456" as separate whitespace-delimited tokens. Fails today ("-16462123456").

## GAP12-6 · low bug · 

**Where:** `crates/cli/src/stored.rs:2961-2987 (prior_accepted_session), 3130-3160 (terminal geometry check)`

**Finding:** pull::calendar::kind_of, the cli's sole session authority, is venue-blind, so a correct post-CAS equity session (ends 15:14) is refused as 'Early or truncated' 

**Evidence:**
  - c
  - l
  - i

**Expected fix and test:** Fix: give the cli session authority the venue. For NseCash keys on days where cash_auction_eligibility_required holds, derive the window from pull::fold::minute_session with the dated cash schedule, the same authority derive uses. Where no schedule is available, refuse with a CAS-specific named reason ('cash session close UNVERIFIED: dated CAS eligibility required'), never 'truncated'. Test: the probe above becomes `a_cas_equity_prior_session_ending_1514_seeds_gapfib`, which asserts Ok, or, with no schedule, an Err whose text names CAS. It fails today with the 'Early or truncated' message.

## GAP4-48 · low doc-false · 

**Where:** `crates/cli/src/stored.rs:2867-2870`

**Finding:** stored.rs comment claims same-day and future daily records are omitted rather than relying on the evaluator; for every day but the last they are offered and the evaluator excludes them 

**Evidence:**
  - s
  - t
  - o

**Expected fix and test:** Fix: reword the comment. The filter bounds the offered set to records some signal day can consume, so remaining() is 0 and the identity claims no unread bytes. Per-row causality is advance_before (anchored.rs:440-446), pinned by anchored.rs:1198.

Test: add a cli test that daily_context_from_span over a multi-day span yields references whose census.remaining() is 0 after the full build, and whose rows equal a prefix build. No behavioural change is expected.

## W2-cli1-5 · low bug · cli

**Where:** `crates/cli/src/batch.rs:540`

**Finding:** one (run identity) (crates/cli/src/batch.rs:540): Identity parity: the comment promises that sweep-all and sweep-stored give one month 'the same 64 hex characters' and so can be compared, but they can never be equal, so their ledger rows cannot be joined by identity. Code path: batch.rs:540-544 says 'Same construction as `sweep_stored`, so sweeping a month here and sweeping it alone produce the same 64 hex characters, which is the only thing that makes the two reports comparable' (repeated at :641-648). batch uses `crate::stored_anchored_digest(&loaded.bars, &exact_minute, &daily)` (:496) and `.with_ceiling(BATCH_CEILING)` = 1<<20 (:534-535).

**Evidence:**
  - batch.rs:496 calls `crate::stored_anchored_digest(&loaded.bars, &exact_minute, &daily)` and uses the result directly. The ordinary sweep-stored path does something else: stored_month_kernel at lib.rs:3321 calls `stored_executed_digest(&loaded.bars, exact_minute, daily, execution_slice)`. That returns `bind_execution_digest(stored_anchored_digest(..), runner::identity::data_digest(execution))` (lib.rs:2478-2487).
  - Batch uses `crate::stored_anchored_digest(&loaded.bars, &exact_minute, &daily)` (batch.rs:496). sweep-stored's `stored_month_kernel` uses `stored_executed_digest(&loaded.bars, exact_minute, daily, execution_slice)` (lib.rs:3271). That calls `bind_execution_digest`, which hashes `b"brutex-stored-executed-inputs-v1\0"`, then the anchored digest, then `runner::identity::data_digest(execution)` (lib.rs:2478-2496).

**Expected fix and test:** None

## W2-cli3-3 · low cost · cli

**Where:** `crates/cli/src/candidate_universe.rs:1356`

**Finding:** CandidateGlobalReplayOosSourceV1::new + mint_witness_recorded (per Global Replay / stored OOS witness) (crates/cli/src/candidate_universe.rs:1356): per one minted OOS witness (one selected winner), the cost is Theta(S+Q+D+E) per witness: a full anchored column build and overlay, alignment, calendar receipts and stream digests at construction, then again in require_integrity and the run seal; it grows with S signal + Q minute context + D daily + E OOS execution bars. Auditor verdict: undocumented-scan. Documented: Contradicted. docs/06-limits.md §169 (line ~8193-8198) says constructing one StoredPostTrainingOosCohortV1 is O(M+S+D+Q) and that 'Minting every witness is proportional to the authenticated Runner replay over its OOS bar ...

**Evidence:**
  - population_v6.rs:3202-3223 caches one StoredPostTrainingOosCohortV1 per family slot (`*cohort = Some(held.stored_post_training_oos_cohort(request)?)`). Lines 3224-3227 then call `mint_witness_recorded` once for each strategy. The cohort struct (stored_post_training_oos.rs:195-211) has no Column field, and `from_retained` builds none.
  - The production path is ledger_v6.rs:677 → global_replay_v4.rs:231 `selection.replay(...)` → all_rung_selection_v6.rs:55, once per rung for 8 rungs → population_v6.rs:3181 `for strategy in strategies`, capped at 25 → population_v6.rs:3225-3227 `.mint_witness_recorded(row.disposition(), observer)`. The cohort is cached per family slot (3199-3223), but the source is not.

**Expected fix and test:** None

## W2-cli3-6 · low cost · cli

**Where:** `crates/cli/src/candidate_universe.rs:4769`

**Finding:** expand_population_side (result append into the retained Candidate row Vec) (crates/cli/src/candidate_universe.rs:4769): per appending one directional grid's rows (cell_count pushes) to the retained row vector, the cost is one reallocation of the whole buffer per (closed mask, side). The amortised-O(1) Vec guarantee is lost, and per-append cost is allocator-dependent; it grows with rows already retained (capacity grows linearly by one grid width instead of geometrically). Auditor verdict: needs-measurement. Documented: No. CLAUDE.md §3 rule 4 requires result append to be amortised O(1). Nothing documents that this buffer grows by exact increments.

**Evidence:**
  - The production row cap is `CEILING_RECORDS = 1 << 24` (`ledger_all.rs:1230`, used at 1268). The default five-step exit ladder gives about 1089 cells per side (comment near `EXIT_CELL_CEILING = 16_384`, `ledger_all.rs:220`). Each row also runs `materialize_cell`, which replays trades over the execution bars (`runner/src/grid.rs:2768` into `per_trade`), plus digest and observation work.
  - In produce_candidate_universe_v1, candidate_universe.rs:2810 creates the row buffer with `let mut rows = Vec::new();` and never reserves it ahead of time. step3_orchestrator.rs:3977 calls it with max_rows = CEILING_RECORDS = 1<<24 (ledger_all.rs:1230). docs/06-limits.md §147 and §150 and the producer's doc comment (2710-2727) describe construction as linear or input-proportional.

**Expected fix and test:** None

## W2-cli3-8 · low bug · cli

**Where:** `crates/cli/src/candidate_universe.rs:6206`

**Finding:** file_generation / require_unchanged (documented guarantee) (crates/cli/src/candidate_universe.rs:6206): Concurrent or corrupt file detection: docs claim content hashing that the code does not perform Code path: docs/06-limits.md §150 (~7587-7589): 'On Unix, cached-generation refusal binds ... device/inode, length and nanosecond modification/change times and also hashes the data files.' The code compares metadata only: 6206-6230 file_generation (fstat vs stat), 6233-6243 generation_of (len, dev, ino, mtime, ctime), 3756-3764 require_unchanged. No hash appears on that path.

**Evidence:**
  - *What the limits document says.** docs/06-limits.md:7587-7589 (§150) says: "On Unix, cached-generation refusal binds the held lock, row and receipt paths by device/inode, length and nanosecond modification/change times and also hashes the data files."
  - docs/06-limits.md:7587-7589 reads "cached-generation refusal binds the held lock, row and receipt paths by device/inode, length and nanosecond modification/change times and also hashes the data files." `FileGenerationV1` (candidate_universe.rs:3174-3190) has only these fields: `len`, `device`, `inode`, `modified_seconds/nanoseconds`, `changed_seconds/nanoseconds`, and on non-Unix `modified`.

**Expected fix and test:** None

## W2-cli4-2 · low bug · cli

**Where:** `crates/cli/src/execution_lease.rs:61`

**Finding:** options (also O_NOFOLLOW_FLAG at execution_v3.rs:131 and execution_v4.rs:135) (crates/cli/src/execution_lease.rs:61): The no-follow flag is hard-coded for every target_os="linux". On aarch64 Linux, 0x20000 is not O_NOFOLLOW, so opening the lock follows symbolic links Code path: `#[cfg(target_os = "linux")] options.custom_flags(0x20000 | 0x800);` has no target_arch gate. In libc-0.2.189, O_NOFOLLOW is 0x8000 for aarch64 (src/unix/linux_like/linux/gnu/b64/aarch64/mod.rs:453) and 0x20000 only for x86_64 (x86_64/mod.rs:527).

**Evidence:**
  - crates/cli/src/execution_lease.rs:58-61 has `#[cfg(target_os = "macos")] options.custom_flags(0x100 | 0x4); #[cfg(target_os = "linux")] options.custom_flags(0x20000 | 0x800);`. linux/gnu/b64/aarch64/mod.rs:453 has `O_NOFOLLOW: c_int = 0x8000`. musl/b64/aarch64/mod.rs:119 has `O_LARGEFILE: c_int = 0x20000`. x86_64/mod.rs:527 has `O_NOFOLLOW = 0x20000`.
  - At crates/cli/src/execution_lease.rs:60-61 the flags are `#[cfg(target_os = "linux")] options.custom_flags(0x20000 | 0x800);`, with no target_arch gate. Lease::acquire (104-116) opens the lock with `.read(true).write(true).create(true).truncate(false)`. On glibc aarch64, libc 0.2.189 defines `O_NOFOLLOW: c_int = 0x8000` (src/unix/linux_like/linux/gnu/b64/aarch64/mod.rs:453).

**Expected fix and test:** None

## ET-strategies-trades-ranking-costs-2 · low bug · cli

**Where:** `crates/cli/src/lib.rs:5193`

**Finding:** The SAMPLE block of the audit report states a fold count the run did not use, and derives its 'each fold tests on roughly N day(s)' figure from that wrong count. The WALK-FORWARD section of the same report shows the real count.

**Evidence:**
  - `crates/cli/src/lib.rs:5193` prints `walk-forward folds {WALK_FORWARD_SPLITS}`. `lib.rs:5201` computes the day figure as `sessions / WALK_FORWARD_SPLITS.max(1)`, and `lib.rs:5476` sets `const WALK_FORWARD_SPLITS: usize = 5`. `both_shapes` at `lib.rs:18867` runs `let splits = walk_forward_splits(bars.len());`.
  - crates/cli/src/lib.rs:5193 prints `walk-forward folds {WALK_FORWARD_SPLITS}`, and lib.rs:5201 computes the per-fold day figure as `sessions / WALK_FORWARD_SPLITS.max(1)`. The constant is `const WALK_FORWARD_SPLITS: usize = 5` at lib.rs:5476. both_shapes calls `let splits = walk_forward_splits(bars.len())` at lib.rs:18867 and passes `splits` to all four walk_forward_projected*_with_rungs calls (lib.rs:18892-18939).

**Expected fix and test:** None

## ET-strategies-trades-ranking-costs-3 · low bug · cli

**Where:** `crates/cli/src/lib.rs:18789`

**Finding:** The live audit OVERFITTING block uses the legacy pbo::place adapter, which rounds an exact half-rank toward the better half. A winner that is strictly in the bottom half out of sample is therefore counted as not overfit, which understates the 'winner below median' count. The block is labelled 'legacy' and 'NON-AUTHORITATIVE', and the exact place_v1 exists at the same cost.

**Evidence:**
  - `crates/cli/src/lib.rs:18773-18793`: `overfitting_of` calls `runner::pbo::place` (line 18789), then `runner::pbo::probability_of_overfitting` (line 18793). `audit_bars` passes the result to `audit::render_selected`, and that dispatches to `audit::overfitting` (`crates/runner/src/audit.rs:303-306`). `crates/runner/src/pbo.rs:398-409`: `place` wraps `place_v1` and sets `winner_rank: exact.winner_rank_twice() / 2`.
  - The live audit calls `overfitting_of` (crates/cli/src/lib.rs:18675/18677). `overfitting_of` (lib.rs:18773-18795) calls `runner::pbo::place` at lib.rs:18789, then `probability_of_overfitting`. `place` (crates/runner/src/pbo.rs:398-409) runs `place_v1` itself (pbo.rs:399), then sets `winner_rank: exact.winner_rank_twice() / 2` (pbo.rs:406).

**Expected fix and test:** None

## W2-cli8-1 · low cost · cli

**Where:** `crates/cli/src/lib.rs:9413`

**Finding:** tiers / stop_rungs_in_points (crates/cli/src/lib.rs:9413): per one generated tier, and one stop rung of the tier ladder, the cost is O(rungs*N) for the stop rungs + O(T*n_needed) + O(T log T) sort, with up to ~710k tiers materialized; it grows with per stop rung: bars N (the full bar slice is scanned and allocated). Per tier: trades_needed_for up to TRADES_SEARCH_CEILING=5000 Wilson evaluations, recomputed for every (stop, ratio) pair. Tier count T = |stops| (<= 64) x 28 ratios x |rates| (<= 396). Auditor verdict: undocumented-scan. Documented: not in docs/06-limits.md (no hits for tiers/stop_rungs_in_points). The ci.yml:5421-5423 gate-11 rationale for this file's sorts says "The ladder is a compile-time list of eight".

**Evidence:**
  - lib.rs:9318 is `.filter(|&pt| pt > 0 && pt <= max_stop_points(bars))`, so max_stop_points is called again for every rung with pt > 0. max_stop_points (4848) calls range_percentile_points(bars, 9, 10), which calls range_percentile (4592-4617). The default derives about 7 rungs (the 5140-5143 comment table).
  - (1) crates/cli/src/lib.rs:9318 is `.filter(|&pt| pt > 0 && pt <= max_stop_points(bars))`. Each call goes to `range_percentile_points` and then `range_percentile` (4592-4617), which collects every bar's range into a `Vec<i64>` of length N and calls `select_nth_unstable`. (2) lib.rs:9457-9461 calls `runner::grid::trades_needed_for(10_000, min_win_rate_bp, TRADES_SEARCH_CEILING)` inside the stop x ratio x rate loop.

**Expected fix and test:** None

## W2-cli8-3 · low cost · cli

**Where:** `crates/cli/src/lib.rs:16374`

**Finding:** record_frontier (crates/cli/src/lib.rs:16374): per one retained ranked combination per result commit, the cost is O(K log K) key evaluations per commit to write `top` rows; it grows with retained combinations K (<= audit_keep() = max(screen_cap, 250) <= 10,000,000). The key is recomputed on every comparison: a HashMap probe plus `rules.admits` (Wilson sqrt) plus two ratio calls. Auditor verdict: false-o1-claim. Documented: docs/06-limits.md: no hit for record_frontier. The ci.yml:5421-5423 gate-11 rationale claims the rows are "capped before it is ordered", but here the cap `take(top)` comes after the full sort.

**Evidence:**
  - *What the code does.** `record_frontier` is at crates/cli/src/lib.rs:16323. At 16373-16374 it runs `let mut ordered: Vec<&&runner::rank::Scored> = by_evidence.iter().collect(); ordered.sort_by_key(|scored| { priced.get(&scored.mask.words()).map_or(...` over the whole `by_evidence` slice.
  - crates/cli/src/lib.rs:16373-16399 collects every retained row with `let mut ordered: Vec<&&runner::rank::Scored> = by_evidence.iter().collect(); ordered.sort_by_key(|scored| { priced.get(&scored.mask.words()).map_or(..., |(cell, _)| (false, core::cmp::Reverse((rules.admits(cell), ranked(cell.return_over_drawdown()), ranked(cell.reward_to_risk_bp()), cell.pessimistic))))})`.

**Expected fix and test:** None

## W2-cli8-4 · low cost · cli

**Where:** `crates/cli/src/lib.rs:14983`

**Finding:** latest_for (crates/cli/src/lib.rs:14983): per one rung's read-back of its own ledger row (range-all/range-rung/descend, per rung and per step), the cost is O(rows) backward scan + O(runs) open per rung; it grows with ledger rows appended since this identity's original row (unbounded on an idempotent rerun), plus the O(runs) index scan of `Results::open` on every call. Auditor verdict: false-o1-claim. Documented: The code doc lib.rs:14958-14966 claims "That is `O(1)` in the ordinary case and `O(rows)` only if the run was somehow not recorded". docs/06-limits.md: no hit for latest_for.

**Evidence:**
  - Every call pays an O(runs) open.** lib.rs:14976 calls `Results::open(&root)` on a fresh handle each time. It does not use the cached `with_shared_writer` handle (results.rs:771). `open_with` walks the whole file, results.rs:982-995: `while at + stride <= len { let (raw, sealed) = read_at(&mut file, at, version)?; if sealed { seen.insert(...) } ...
  - `crates/cli/src/lib.rs:14976` calls `crate::results::Results::open(&root)?` fresh every time. `Results::open` goes to `open_with` (`results.rs:855`). Then `lib.rs:14983-14995` scans backward, `for back in 1..=count { let record = store.read(count.saturating_sub(back))?; if record.feed == feed && ... `sweep_rungs` (`lib.rs:14621`) runs the range rungs in parallel with `par_iter`.

**Expected fix and test:** None

## W2-cli8-5 · low cost · cli

**Where:** `crates/cli/src/lib.rs:7772`

**Finding:** newest_complete / results_at (crates/cli/src/lib.rs:7772): per one `cli top` or `cli results` request, the cost is O(ledger rows) reads (one seek+read syscall each) + O(runs) open index + O(matching) memory per request; it grows with total ledger rows (every recorded run ever); results_at also holds every matching Record in memory while printing <= LIST_ROWS=40. Auditor verdict: undocumented-scan. Documented: The code doc lib.rs:7815-7825 says "O(rows) — the size of the answer ... UNVERIFIED as a measurement", but the printed answer is at most 40 rows plus one best row.

**Evidence:**
  - newest_complete (crates/cli/src/lib.rs:7772-7792) runs `for index in 0..count { let record = store.read(index)?; ... results_at (lib.rs:7872-7885) pushes every matching Record into `let mut rows: Vec<crate::results::Record> = Vec::new();`, prints only `rows.iter().take(LIST_ROWS)` (7910, LIST_ROWS=40 at 7528), then runs `best_complete_newest_first(&rows)` (7945).
  - crates/cli/src/lib.rs:7772-7792 (`newest_complete`) reads every row in the ledger, one at a time (`for index in 0..count { let record = store.read(index)?; ... lib.rs:7872-7885 (`results_at`) collects every matching Record into a `Vec`, then prints at most `LIST_ROWS` = 40 of them (7910) and picks the best with `best_complete_newest_first(&rows)` (7945).

**Expected fix and test:** None

## W2-cli8-6 · low cost · cli

**Where:** `crates/cli/src/lib.rs:933`

**Finding:** column_withholding_at_build / exact_minute_withholding_unsourceable_days (crates/cli/src/lib.rs:933): per one withheld IST day with an unsourceable closing minute, the cost is O(N*vocab + M) per withheld day; O(64*(N*vocab+M)) worst case per rung; it grows with signal bars N + exact-minute context M + daily context. Each withheld day reloads both contexts from disk, re-digests, writes a durable preparation attempt and rebuilds the whole anchored column (up to ATTEMPTS=64). Auditor verdict: undocumented-scan. Documented: not in docs/06-limits.md (no hits for unsourceable, ATTEMPTS in this context, or column_withholding). 06-limits.md:9058 covers only the single `minute_gaps` pass.

**Evidence:**
  - crates/cli/src/lib.rs:926 declares `const ATTEMPTS: usize = 64;`. Lines 933-940 are the retry loop: `for _ in 0..ATTEMPTS { let daily = stored::load_daily_context(...)?; let exact = stored::load_exact_minute_context(...)?; let digest = stored_anchored_digest(bars, &exact, &daily)?; let attempt = preparation_attempt_with_commit(...)?; let folded = stored_anchored_column(...);`
  - crates/cli/src/lib.rs:933-944 runs `for _ in 0..ATTEMPTS {` with ATTEMPTS = 64 (lib.rs:926). On a refusal it calls `crate::minute_gaps::withhold(bars, &[day])` (lib.rs:970), which walks every bar (minute_gaps.rs:240-246), and loops again. stored.rs:3197-3199 reads `let warm_from = previous_month(from)?; let minute = load_span(root, vendor, underlying, "1min", warm_from, to)?;`.

**Expected fix and test:** None

## W2-cli8-10 · low bug · cli

**Where:** `crates/cli/src/lib.rs:13982`

**Finding:** elite_arm / elite_descend_in_points_inner / screen_range_in_points (crates/cli/src/lib.rs:13982): Zero input: MAX_POINTS=0 is documented as 'no ceiling beyond the derived ladder' but is always refused. Code path: USAGE (454-459) and elite_arm (1174-1201) accept `pts == 0`, then call elite_descend_in_points, whose inner function refuses `if max_points <= 0 { return "refused: the stop ceiling must be a whole number of index points, 1 or more...` (13982-13987).

**Evidence:**
  - USAGE, crates/cli/src/lib.rs:454-459, promises "pass ZERO for no ceiling beyond the ladder the bars themselves derive". dispatch, lib.rs:2096-2097, forwards `pts` unchanged to `elite_arm`. `elite_arm` refuses only `if pts < 0` (lib.rs:1193), and its own error text (1197-1199) says "or 0 for no ceiling". It then calls `elite_descend_in_points(..., pts, n)` (lib.rs:1225).
  - The USAGE text at crates/cli/src/lib.rs:454-459 says "MAX_POINTS is yours when you mean it -- or pass ZERO for no ceiling beyond the ladder the bars themselves derive". The comment in elite_arm at lib.rs:1175-1190 says "ZERO IS \"NO CEILING BEYOND THE SWEPT LADDER\", NOT A MISTAKE" and that the old refusal "was the only thing preventing an operator asking for it".

**Expected fix and test:** None

## W2-cli8-11 · low bug · cli

**Where:** `crates/cli/src/lib.rs:2317`

**Finding:** parse_support_ppm vs support_from_knob (crates/cli/src/lib.rs:2317): Off-by-one at a boundary: 1,000,000 ppm is accepted from argv but refused from the knob. A comment claims both doors refuse it. Code path: parse_support_ppm refuses only `ppm > 1_000_000` (2317), so 1,000,000 passes, while support_from_knob requires `ppm > 0 && ppm < 1_000_000` (7989). one_rung's comment (12979-12982) says "Refused at zero and at a million ... Both are the same refusal `screen` already makes", which is false for argv.

**Evidence:**
  - `parse_support_ppm` (crates/cli/src/lib.rs:2313-2323) refuses only `Ok(0)` and `Ok(ppm) if ppm > 1_000_000`, then returns `Ok(ppm) => Ok(ppm)`. `parse_support_choice` (2347-2352) calls it. `support_from_knob` (7986-7995) accepts only `Ok(ppm) if ppm > 0 && ppm < 1_000_000`. Lines 12979-12982 say "Refused at zero and at a million ... The test at crates/cli/src/audited_stored_tests.rs:588 runs `screen ...
  - crates/cli/src/lib.rs:2313-2322 `parse_support_ppm` refuses only `Ok(0)` and `Ok(ppm) if ppm > 1_000_000`. `parse_support_choice` (2347-2352) delegates to it, which puts range-all (1538/1683/1712), screen_arm (1264), descend_arm (1593) and ledger-all/v6/replay (1798, 1867) behind the same bound. crates/cli/src/lib.rs:7989 `support_from_knob` has `Ok(ppm) if ppm > 0 && ppm < 1_000_000 => Some(ppm)`.

**Expected fix and test:** None

## R9-cli-law-1 · low bug · cli

**Where:** `crates/cli/src/lib.rs:3287`

**Finding:** The `Run` literal in `stored_month_kernel` says `vocab` is not among `cli`'s dependencies and that naming `ConditionMask` would add an arrow CLAUDE.md §5 does not draw. This range changed the `params:` line of that same literal. Its `#[expect(clippy::default_trait_access, reason = ...)]` repeats the claim, and so do the same reason at lib.rs:5906, 6616, 15265, 20823 and batch.rs:550-561 (`one`, which this range also changed).

The claim is false on every count:
- cli/Cargo.toml declares `vocab`.
- CLAUDE.md §5 draws `... telemetry vocab <-- cli` (D-0683).
- Production cli code already names `vocab::ConditionMask` in a `Run` literal (audited_range_command.rs:211) and in pool's `price_all`.
- This range's own new tests do the same (lib.rs test `the_run_identity_still_folds_the_divided_ceiling_the_ladder_was_given`, audited_stored_tests.rs:1028).

A comment that misstates the crate-graph law is the kind of drift §5 warns about, and it keeps a lint suppression in place for a reason that does not hold.

**Evidence:**
  - Both fix/c2f-r-cli and fix/c2-final declare `vocab` in crates/cli/Cargo.toml:47 and draw `... telemetry vocab <-- cli` in CLAUDE.md:228. Production cli code also already names the type, as in audited_range_command.rs:210 `mask: vocab::ConditionMask::default(),` and pool.rs `vocab::ConditionMask::from_words`.
  - Production cli code already names `vocab::ConditionMask::default()`, for example at audited_range_command.rs:210 on origin/main. `git grep` on origin/main finds the same text at 7 sites: batch.rs:517/522, and lib.rs:3154/3159, 5728, 6431, 14980, 20304/20307.

**Expected fix and test:** None

## AC-whp-o1-2 · low cost · cli

**Where:** `crates/cli/src/lib.rs:17379`

**Finding:** bootstrap_family builds the bootstrap family on the execution series. For each of up to BOOTSTRAP_CANDIDATES=16 candidates it calls runner::trade::walk. walk calls SliceFacts::of(bars, column) on every call. Each call rebuilds, over the whole execution slice B (e.g. ~618k one-minute bars on the audit path):
- a HashMap<i64,usize> sized B;
- two B+1 prefix Vec<u64>;
- a B-entry forced-exit table (SessionBounds).

None of that depends on the candidate. trade.rs:315-317 states the rule outright: 'That is the right cost for a one-off; a loop over candidates must hoist the facts and use [`walk_over`]'. This file already records the same defect fixed at four other sites ('This is the FOURTH site of one defect', lib.rs:12364), and bootstrap_family is a fifth.

The cost is bounded (16×O(B) allocation and hash builds instead of 1×), so this is not an O(1)-law breach on a per-bar or per-candidate primitive. It is repeated per-slice work in a candidate loop, and backlog W3-runner4-0 covers only the grid call sites.

**Evidence:**
  - **The code.** fix/c2-final:crates/cli/src/lib.rs:17377-17396, in bootstrap_family, is `by_evidence.iter().take(BOOTSTRAP_CANDIDATES).map(|scored| { let walked = trade::walk(bars, column, &scored.mask, horizon, direction_of(side_of_evidence(scored))); session_returns(&index, days.len(), bars, &walked) })`. BOOTSTRAP_CANDIDATES is 16 (lib.rs:5566). **What walk costs.** runner/src/trade.rs:321-336 `walk` returns `walk_over(..., &SliceFacts::of(bars, column))`.
  - fix/c2-final:crates/cli/src/lib.rs:17378-17396: `by_evidence.iter().take(BOOTSTRAP_CANDIDATES)` (BOOTSTRAP_CANDIDATES = 16, lib.rs:5566), and inside the `.map` it calls `trade::walk(bars, column, &scored.mask, horizon, direction_of(side_of_evidence(scored)))`. runner/src/trade.rs:321-336 shows `walk` is `walk_over(..., &SliceFacts::of(bars, column))`. `SliceFacts::of` (trade.rs:407-463) depends only on (bars, column).

**Expected fix and test:** None

## W2-cli9-0 · low cost · cli

**Where:** `crates/cli/src/pool.rs:560`

**Finding:** union_of (crates/cli/src/pool.rs:560): per one screened instrument's frontier lookup while building the pass-2 union, the cost is O(I*(L+R)) per pool invocation; O(L+R) per instrument; it grows with instruments I x (total results-ledger records L + total detail receipts R), plus that run's frontier rows. Auditor verdict: false-o1-claim. Documented: No. pool.rs:84-88 says the union is 'one `HashSet` insert per frontier row — O(1) expected' and that 'Nothing here scans the store per candidate'.

**Evidence:**
  - crates/cli/src/pool.rs:545 opens one handle with `let mut frontier = match Frontier::open_read(root)`. open_read goes through open_read_bounded, which sets `require_parent: true` (frontier.rs:842). pool.rs:556-560 then runs `for s in screened { ... frontier.rs:1358-1360: every call does `if self.require_parent { crate::result_set::committed_receipt(&self.root, identity)?
  - pool.rs:545 opens one read-only frontier handle, `Frontier::open_read(root)`. frontier.rs:842 sets `require_parent: true` on it. pool.rs:552-560 then calls `frontier.of_run(&record.identity)` once for each screened instrument. frontier.rs:1359-1360 shows what each of those calls does first: `let receipt = if self.require_parent { crate::result_set::committed_receipt(&self.root, identity)?

**Expected fix and test:** None

## W2-cli9-7 · low bug · cli

**Where:** `crates/cli/src/pool.rs:508`

**Finding:** render_per_symbol (crates/cli/src/pool.rs:508): Refused instruments render FIRST in the pass-1 table, although the comment says 'Refusals sort last'. Code path: The key is `Ok(r) => (false, ...)` and `Err(_) => (true, i64::MIN, i64::MIN, i64::MIN)` (499-507). An ascending sort puts Err rows after Ok rows, then `rows.reverse();` (508) moves them to the front. Test search: `render_per_symbol` is not called from any test in pool.rs or crates/cli/src/*_tests.rs.

**Evidence:**
  - In crates/cli/src/pool.rs:498-508, `rows.sort_by_key` maps `Ok(r) => (false, r.max_drawdown.saturating_neg(), r.worst_trade, r.pessimistic)` and `Err(_) => (true, i64::MIN, i64::MIN, i64::MIN)`, and then calls `rows.reverse();`. The comment at pool.rs:496-497 says "Refusals sort last and are named, never dropped", and docs/05-decisions.md:32606-32607 says the pass-1 table is sorted "with refusals named last".
  - The comment at 496-497 says "Refusals sort last and are named, never dropped." The sort key at 499-507 is `Ok(r) => (false, r.max_drawdown.saturating_neg(), r.worst_trade, r.pessimistic)` and `Err(_) => (true, i64::MIN, i64::MIN, i64::MIN)`. No test covers this: render_per_symbol has one caller, pool.rs:276, and none of the test functions in pool.rs's tests module (from line 856) calls it.

**Expected fix and test:** None

## R9-cli-law-3 · low bug · cli

**Where:** `crates/cli/src/pool.rs:525`

**Finding:** Every pool page is headed by `opening`, which begins with `STORED_PROVENANCE`. This range made `head_under` bind the page to that head. The banner tells the reader that "the run identity beneath names the exact column they came from" and that "A figure here describes that instrument and that month."

The pool page prints no run identity anywhere. The pass-1 table has no identity column, and pooled rows print only a mask. Its pooled figures describe many instruments over a span of months.

The banner separates real from generated data and is the §5-named guard against a failure passing as a success, so a false sentence in it matters. The page's own figures cannot be traced to the nine-term identities pass 1 recorded (§3 rule 3) without leaving the page. `sweep-all`, the other multi-instrument stored page, prints `identity {hex}` per row (batch.rs render), so this is specific to pool.

**Evidence:**
  - I tried to refute the finding and could not. The pool page starts with the stored-data banner and never prints a run identity. That same page has the same shape on origin/main (96194c11). I checked this by reading the code; I did not build it or run it. What I found on fix/c2f-r-cli: - `crates/cli/src/lib.rs` lines 2622-2627 define STORED_PROVENANCE. It says "the run identity beneath names the exact column they came from" and "A figure here describes that instrument and that month." - ...
  - `STORED_PROVENANCE` (fix/c2f-r-cli lib.rs:2622-2627) says "the run identity beneath names the exact column they came from. A figure here describes that instrument and that month." On the pool page, `opening` (pool.rs:525) starts with that banner, but neither table prints an identity. The codebase has already called the same thing a lie for `sweep-all`: batch.rs:130-135 at origin/main reads "its absence was the report's own banner telling a lie ...

**Expected fix and test:** None

## R9-cli-o1-1 · low cost · cli

**Where:** `crates/cli/src/pool.rs:80`

**Finding:** The limits entry and the pool doc understate pass 2's cost. docs/06-limits.md §171 says 'pass 2 is I × U grid evaluations, each O(cells × T) for that instrument and candidate', and pool.rs:81-82 says each is 'the cost of one exit grid over that instrument's trades for that mask'. That is not what price_all pays. For every (instrument, candidate) it calls `grid::evaluate_over`, which calls `crate::trade::walk_over` (grid.rs:2049). walk_over calls walk_core, whose loop visits every row of the column (trade.rs:691), testing the mask on each bar. Every evaluation is therefore Θ(B) in the instrument's signal bars. The crossing walks (grid.rs:1904/1921 state '2 × trades × span per evaluate call') and the cells x T pricing come on top. For the objective the pool exists for, a setup that fires a handful of times (T small) over a multi-year span (B large), the omitted B term dominates. U is the union of every instrument's frontier rows, and record_frontier keeps at most `rules.top` per run (lib.rs:16334ff, `let kept = by_evidence.len().min(top)`), so U grows with I. Pass 2 is then Θ(I x U x B), up to Θ(I^2 x top x B), not I x U x cells x T. The same section also omits a second cost. price_a

**Evidence:**
  - Neither is the real cost: - `price_all` (pool.rs:664-739) calls `grid::evaluate_over` once for every union candidate. - `evaluate_over` (grid.rs:1983) goes through `evaluate_families_over`, which calls `crate::trade::walk_over` (grid.rs:2049). - `walk_over` (trade.rs:549) calls `walk_core` with `|bits, _| bits.hits(mask)`.
  - On origin/main, that calls `evaluate_families_over`, which calls `crate::trade::walk_over` (grid.rs:2049). `walk_core`'s loop is `for (index, (bits, &signal)) in column.bits().iter().zip(column.sources()).enumerate() { if !fires(bits, index) { continue; } ...` (trade.rs:680 on main). The two sentences leave out that B term and describe only the pricing part: - pool.rs:81-82 says each evaluation is "the cost of one exit grid over that instrument's trades for that mask".

**Expected fix and test:** None

## W2-cli11-2 · low cost · cli

**Where:** `crates/cli/src/population_finalization_v2.rs:1453`

**Finding:** PopulationFinalizationV2Ledger::reopen_structural_receipt (and the append path via require_unchanged) (crates/cli/src/population_finalization_v2.rs:1453): per one identity lookup; one block append, the cost is O(F) per lookup; O(F), with at least 8 whole-file hash passes, per append; it grows with ledger file bytes. Auditor verdict: undocumented-scan. Documented: Only in module rustdoc (1174-1182: 'the complete lookup operation is O(file bytes), not universally O(1)'; 1431-1438).

**Evidence:**
  - **Lookup re-hashes the whole file.** `reopen_structural_receipt` (population_finalization_v2.rs:1444-1466) calls `self.require_unchanged().map(|()| self.receipts.get(finalization_id).copied())`. `require_unchanged` (1852-1860) calls `require_file_generation` for the data file with `self.bounds.max_file_bytes`.
  - reopen_structural_receipt (1444) calls `self.require_unchanged().map(|()| self.receipts.get(finalization_id).copied())` (1452-1454). append, open_write and persist carry `expect(dead_code)` outside tests (1531-1536, 1562-1567, 1943-1948). The rustdoc states the limit honestly and labels it UNVERIFIED (1174-1182, 1431-1438).

**Expected fix and test:** None

## W2-cli11-3 · low cost · cli

**Where:** `crates/cli/src/population_observations_v1.rs:2213`

**Finding:** ObservationAuthorityLedgerV1::reopen_audit / ObservationAuthorityLedgerV2::reopen_audit (3394) (crates/cli/src/population_observations_v1.rs:2213): per one authority lookup by id, the cost is O(B) per lookup (whole file read into a Vec, then blake3); append also does it twice; it grows with authority file bytes B (64 + 1024*A for V1, 64 + 2048*A for V2, for A authorities). Auditor verdict: undocumented-scan. Documented: docs/06-limits.md §157 (lines 7826-7828: 'Opening the ledger scans and hashes its bounded bytes; hash-index lookup is average O(1)') and §161 (7945-7946: 'one identity-map lookup is average O(1)').

**Evidence:**
  - `pub fn reopen_audit(&mut self, authority_id) { self.require_unchanged()?; Ok(self.audits.get(authority_id).copied()) }` is at population_observations_v1.rs:2209-2215 (V1) and 3390-3396 (V2). `require_unchanged` (2314-2325) runs the two metadata generation checks, then `let bytes = read_bounded_authority_file(&mut self.file, self.bounds)?; if digest_authority_file(&bytes) != self.snapshot_digest`.
  - V1 `reopen_audit` (population_observations_v1.rs:2209-2215) is `self.require_unchanged()?; Ok(self.audits.get(authority_id).copied())`. `require_unchanged` (2314-2325) first runs two metadata-generation checks, then `let bytes = read_bounded_authority_file(&mut self.file, self.bounds)?; if digest_authority_file(&bytes) != self.snapshot_digest`.

**Expected fix and test:** None

## W2-cli12-1 · low cost · cli

**Where:** `crates/cli/src/population_statistics_v2.rs:4703`

**Finding:** append_population_statistics_v2 (crates/cli/src/population_statistics_v2.rs:4703): per appending one audit (one rung's result block), the cost is O(sum over all stored audits of (C*(P+S) + bootstrap)) per append, done twice; cumulative O(A^2); it grows with number of prior audits A in the root, times each audit's C*(P+S) rows plus its B-draw bootstrap. Auditor verdict: undocumented-scan. Documented: No. §154 gives one block's open cost. The cumulative-quadratic statement in §158 covers Admission V2 only. Nothing says that every V2 append runs two full-ledger opens, each re-running every historical bootstrap.

**Evidence:**
  - (1) append_population_statistics_v2 at population_statistics_v2.rs:4703 calls `PopulationStatisticsV2Ledger::open_writer(root, bounds)?`. That call re-runs white_reality_check_receipt_v1, spa_receipt_v1 and romano_wolf_adjusted_p_values_v1 (3048-3058). In production the procedure is BOOTSTRAP_DRAWS=1_000 (ledger_all.rs:1248).
  - At crates/cli/src/population_statistics_v2.rs:4703 the append runs `let mut writer = PopulationStatisticsV2Ledger::open_writer(root, bounds)?;`. That open runs `ledger.scan()?` at :2240, and the scan loops `while cursor < records` and calls `validate_complete_block(&mut self.data_file, cursor, &manifest)?` at :2302 for every complete stored block.

**Expected fix and test:** None

## W2-cli12-5 · low bug · cli

**Where:** `crates/cli/src/population_statistics_v2.rs:4607`

**Finding:** resume_orphan (crates/cli/src/population_statistics_v2.rs:4607): Retrying an interrupted append (receipt-less orphan) with a byte ceiling smaller than the orphan's full planned block Code path: The new-write path checks `if desired > self.bounds.file_bytes` (4567), but resume_orphan (4607-4660) appends planned[present..] and the Completion and syncs both (4631-4648) with no byte check. Only afterwards does `self.data_generation = file_generation(&self.data_file, &self.data_path, self.bounds.file_bytes)?` (4657-4658) refuse, via `if measured_len > max_bytes` (5573).

**Evidence:**
  - V2 new-write path (crates/cli/src/population_statistics_v2.rs:4567) checks the size before writing: `if desired > self.bounds.file_bytes { return Err(...) }`. V2 resume path: append_locked sends a receipt-less orphan straight to resume_orphan (4543-4544: `if let Some(orphan) = self.orphan { return self.resume_orphan(prepared, &orphan); }`). resume_orphan (4607-4660) never compares bytes against bounds.file_bytes.
  - `append_locked` hands an orphan to `resume_orphan` (4541-4542) before that check and before `validate_manifest`. `resume_orphan` (4607-4660) appends `planned[present..completion]`, calls `sync_all`, appends the Completion and calls `sync_all` again (4631-4648).

**Expected fix and test:** None

## W2-cli13-0 · low cost · cli

**Where:** `crates/cli/src/pre_admission_data.rs:1222`

**Finding:** PreAdmissionDataLedgerV1::page (crates/cli/src/pre_admission_data.rs:1222): per one page read (<=256 rows), the cost is O(F + P) per page; paging all R rows is O(R/256 * F) = O(R^2); it grows with F = total bytes of pre-admission-data-v1.bin (all rows ever appended), plus P returned rows. Auditor verdict: false-o1-claim. Documented: docs/06-limits.md:7651 says the opposite: 'A page costs O(P) for P returned rows.' Module doc :30-31 says 'A page is proportional to the returned records after that scan.'

**Evidence:**
  - `PreAdmissionDataLedgerV1::page` (:1222) takes the shared lock. It then runs `self.require_unchanged()?` (:1241), and later `let mut file = open_file(&self.data_path, false, false)?; require_generation(self.data_generation, &file, &self.data_path)?;` (:1248-1249). `require_unchanged` (:1428-1431) calls `require_generation` on the lock file and on the data file.
  - *What one page does.** `PreAdmissionDataLedgerV1::page` (crates/cli/src/pre_admission_data.rs:1222) does three things before it reads any row: It calls `self.require_unchanged()?` (:1241). That function (:1428-1431) calls `require_generation` twice: first on the lock file, then on `self.data_file`. *Why each check reads the whole file.** `require_generation` (:3701) calls `file_generation` (:3611).

**Expected fix and test:** None

## AC-whp-law-2 · low bug · cli

**Where:** `crates/cli/src/results.rs:300`

**Finding:** `results::Record::bars` is documented as "Signal bars swept". It is an append-only field that readers compare across doors. `sweep-stored` records `loaded.bars.len()`, which is every retained signal bar including the warm-up bars the column did not sweep, and so do the stored audit (lib.rs:6005) and other doors. `sweep-all` records `outcome.census.swept`.

The evaluator's warm-up (`prev5.filled() >= 5 && … && trend.every_position_can_answer()`) holds back real bars at the start of every stored month. For the same instrument-month, the two doors therefore write different `bars` values under one field name, and only `sweep-all`'s matches the doc. The sweep-stored report also says "The sweep uses the remaining {} signal bars" of the same `loaded.bars.len()`, which includes warming bars that were never swept. A support fraction computed as `min_hits / bars` from a ledger row therefore depends on which door wrote it.

**Evidence:**
  - **The doc.** `Record::bars` is documented as "/// Signal bars swept." at fix/c2-final:crates/cli/src/results.rs:300. The same sentence is at origin/main:crates/cli/src/results.rs:298 and at crates/api/src/backtest.rs:428 on both refs. **The value goes straight into the ledger.** `record_swept_run` hands `bars` to `record_run` (fix/c2-final:crates/cli/src/lib.rs:16749).
  - fix/c2-final:crates/cli/src/results.rs:300-301 reads `/// Signal bars swept.` over `pub bars: u64,`. It is the same text at origin/main results.rs:298-299, and `git diff origin/main fix/c2-final -- crates/cli/src/results.rs` shows no change to it. fix/c2-final lib.rs:3496-3510 (stored_month_kernel) passes `u64::try_from(loaded.bars.len()).unwrap_or(u64::MAX)` as the bars argument to record_swept_run.

**Expected fix and test:** None

## R9-csr-cx-4 · low bug · indicators

**Where:** `crates/indicators/src/lib.rs:124`

**Finding:** The D-0694 commits (770c6505) rewrote the weekend comment in evaluator.rs. The old one said 'NSE does not trade Saturday or Sunday, so such a bar is a store defect'. The new one says 'NSE does trade on weekends'. The same commits updated weekday_bit's doc in lib.rs to 'Six weekend sessions, 1,710 real trading bars ... none of them a defect'. But weekday_bit's own doc still begins 'NSE trades Monday to Friday', and its match arm still says '2 and 3 are Saturday and Sunday: NSE does not trade them.' Both contradict the paragraph between them and docs/00-charter.md §3 ('Six, totalling 1,710 one-minute bars'). vocab/src/table.rs:1145 repeats the claim at the day-of-week bits: 'NSE trades Monday to Friday, so five bits and no more.' These lines predate the range, but this change corrected the same fact one file away and in the same doc block.

**Evidence:**
  - - indicators/src/lib.rs:168 reads "2 and 3 are Saturday and Sunday: NSE does not trade them." - The test comment at lib.rs:2011-2013 reads "NSE DOES NOT TRADE THESE ... - lib.rs:124 and lib.rs:83 say NSE "trades Monday to Friday".
  - A weekend bar in an equity series is a store defect" - line 83: "an exchange that trades Monday to Friday" crates/vocab/src/table.rs:1145 also still says "NSE trades Monday to Friday, so five bits and no more".

**Expected fix and test:** None

## W3-indicators2-0 · low cost · indicators

**Where:** `crates/indicators/src/vwap.rs:203`

**Finding:** isqrt_i128_counted (reached per bar via Vwap::sigma from Vwap::bits at vwap.rs:457 and from Vwap::known at vwap.rs:513) (crates/indicators/src/vwap.rs:203): per one VWAP sigma per bar on an Availability::Present (cash-equity) run. On the Column::build path it runs twice per bar on the same post-fold state: evaluator.rs:903 `self.vwap.step(..)` -> bits -> sigma -> isqrt, and eval ..., the cost is bounded by a named constant (130 iterations of a 128-bit division) and dependent on the operand. The true worst case is 129 iterations at k^2-1, not the documented 69 at i128::MAX; it grows with the variance operand's value, not bars, candidates or session length. 1 iteration at v=1, 13-16 at realistic equity variances, and 128-129 at EVERY v = k^2-1 whatever its size. Capped at ITERATION_CEILING = 130. Auditor verdict: false-o1-claim. Documented: Partly, and the documented figures are wrong. docs/06-limits.md section 51 (lines 3435-3486) says: "a small input finishes in one iteration and a 127-bit one needs sixty-nine.

**Evidence:**
  - The loop is at vwap.rs:203-207: `while i < NEWTON_STEPS && guess != previous { previous = guess; guess = guess.midpoint(v / guess); ... It runs to NEWTON_STEPS = 128, and the step-down at 213-216 then corrects the root. Measurement: I compiled a verbatim sed copy of vwap.rs:195-218 with rustc 1.97.1 -O as a standalone file under the scratchpad (sk1-isqrt/main.rs).
  - *Why the loop oscillates.** The loop at vwap.rs:203-207 is `while i < NEWTON_STEPS && guess != previous { previous = guess; guess = guess.midpoint(v / guess); ... *The counts, measured.** I copied lines 195-225 verbatim into a standalone program and ran it on pinned rustc 1.97.1. *Sigma is computed twice per bar, but that is a constant 2x.** column.rs:185 calls step_known.

**Expected fix and test:** None

## W3-indicators2-1 · low bug · indicators

**Where:** `crates/indicators/src/vwap.rs:203`

**Finding:** isqrt_i128_counted (crates/indicators/src/vwap.rs:203): Inputs of the form v = k^2-1 (3, 8, 143, 975^2-1, (10^15)^2-1, isqrt(i128::MAX)^2-1). Newton oscillates between k-1 and k and uses the whole 128-step cap, which contradicts every documented iteration figure Code path: A run of the real function (release, under load, average 9-11) gave 128-129 iterations and 1.0-2.1 us per call for every such input, against the documented range of 1 to 69; 999 of v in 1..=10^6 hit the cap. The output root stays exact.

**Evidence:**
  - *Code.** vwap.rs:203 is `while i < NEWTON_STEPS && guess != previous {`, and the loop body is `guess = guess.midpoint(v / guess)`. docs/06-limits.md:3456: "a small input finishes in one iteration and a 127-bit one needs sixty-nine. docs/06-limits.md:3478: "Dropping the convergence exit would run all 130 iterations every call".
  - The exit condition at crates/indicators/src/vwap.rs:204-208 is `while i < NEWTON_STEPS && guess != previous { previous = guess; guess = guess.midpoint(v / guess); ... When the loop ends on k, the bounded step-down at lines 213-216 corrects it, so the root stays exact. sigma(), which calls isqrt_i128 at vwap.rs:343, is itself called on the per-bar evaluation path at vwap.rs:457 and :513.

**Expected fix and test:** None

## AC-whp-tb-9 · low bug · indicators

**Where:** `crates/indicators/tests/invariants.rs:109`

**Finding:** - **V-03:** the test doc says the suffix is replaced with 'i64::MAX / i64::MIN prices, a far-future timestamp'. The code writes ±i64::MAX/4, open 0 and volume i64::MAX, and never touches `ts_micros`.
- **V-04:** the row says 'Time-of-day and VWAP bits are cleared on a daily timeframe'. `daily_mask_clears` checks only the VWAP positions under `Availability::Absent` on a one-minute session. The `Evaluator` has no timeframe input and marks the clock positions known on every bar (evaluator.rs:727-732). Nothing clears time-of-day bits anywhere, and the claim is moot because 1day is not a swept rung (cli lib.rs:22939-22949).
- **V-05:** the row says 'The fast evaluator agrees with a naive reference on random input'. The test checks five hand-picked (h, l, c) triples of `DailyLevels::from_previous_session` only. It does not test the Evaluator and does not use random input.

All three rows also name modules that do not exist (`indicators::proptest::`, `indicators::unit::`). Gate 10 ignores the module segment, so they pass it.

**Evidence:**
  - docs/04-invariants.md:104-106 is the same on origin/main, fix/c2-final, fix/c2f-r-api and fix/c2f-r-cli. (1) V-03 (fix/c2-final:crates/indicators/tests/invariants.rs:112-115 against 122-128).
  - What I verified: - **V-03 test doc** (invariants.rs:112-115) promises `i64::MAX` / `i64::MIN` prices and a far-future timestamp. - **V-04 row** (docs/04-invariants.md:105) says 'Time-of-day and VWAP bits are cleared on a daily timefram

**Expected fix and test:** None
