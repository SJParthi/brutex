# Lane 1-b fix plan: 64 findings in 41 units

Base: origin/main `bc53131` (fetched 2 Oct 2026). Every line number below was re-read at that commit with `git archive origin/main`. Many cited lines had moved, so the current lines are given. Decision numbers run from D-0960 upward. Main's ledger currently ends at D-0801.

**Lane-3 branches:** `git ls-remote origin 'refs/heads/fix/cloud-*'` returns **nothing**. No `fix/cloud-*` branch exists on the remote, so no lane-3 branch currently touches `crates/indicators/**` or `crates/vocab/src/table.rs`. Check again before opening U29, U30 or U39.

**Not reproduced:** none. All 64 findings still apply at origin/main. U2 and U20 were confirmed by reading the code path, not by running it.

**Conflicts every unit shares:**
- Each unit appends a D-entry to the end of `docs/05-decisions.md`.
- Most cost units add or edit a section in `docs/06-limits.md`.

Both edits conflict at end of file and are trivial to resolve. Rebase on main before merging, and take the next free `§` number rather than the one guessed here.

Kinds: code-fix | cost-fix (made O(1) or output-proportional) | cost-doc (honest bound in docs/06-limits.md) | doc-false | test-gap.

---

## U1 fix/cloud-W2-cli3-0
- **findings:** W2-cli3-0 (high cost), W2-cli3-1 (med cost), W2-cli3-2 (med cost), W2-cli3-6 (low cost). Merged because all four sit on the per-(closed mask, side) path. The path is `expand_population_side`, then `append_validated_grid_rows`, plus its replay twin `replay_execution_side`. Fixing one changes the others' signatures.
- **files:**
  - `crates/cli/src/candidate_universe.rs`:
    - 2739-2830 `produce_candidate_universe_v1` (`let mut rows = Vec::new()` at 2810)
    - 4276 and 4352-4383 Execution V3 replay loop
    - 4510-4600 `replay_execution_side` (`new_with_daily_reference` at 4548)
    - 4735-4846 `expand_population_side` (`try_reserve_exact` at 4769, `new_with_daily_reference` at 4785, `evaluate_training_grid_attested` at 4800)
    - 4848-4910 `append_validated_grid_rows` (`materialize_cell` at 4890)
    - rustdoc 2710-2727
  - `crates/runner/src/grid.rs`: 2696 `per_trade`, 2768 `materialize_cell`, 3465 `levelled` (rebuilds `SliceFacts::of`), 3507 `levelled_over`, `levelled_timed`
  - `crates/runner/src/exit_grid_policy.rs`: 303-318 `new_with_daily_reference` (recomputes `data_digest_with_daily_reference`), 2057-2066 `evaluate_training_grid_attested` (calls `attest_training` every time)
  - `docs/06-limits.md` §147 (7554) and §150 (7621)
- **kind:** cost-fix plus cost-doc for the walk remainder.
- **dnum:** D-0960
- **verified:** yes.
  - 4861 loops over every cell and calls `materialize_cell`, which goes through `per_trade` and then `levelled`. `levelled` rebuilds `SliceFacts::of(bars, column)`, which is Θ(E).
  - 4785 and 4548 build `ExecutionRunV1::new_with_daily_reference` for every mask and side, re-hashing S+M+D.
  - `evaluate_training_grid_attested` re-attests the series and column each time.
  - 4769 is `rows.try_reserve_exact(additional)`. Exact growth makes the buffer reallocate on every (mask, side).
- **goal:**
  1. In runner, add `pub fn materialize_cell_over(.., facts: &SliceFacts)` and a per-(mask, side) walk door, `levelled_timed` over one hoisted `timed` walk. Then a grid's cells replay over one walk and cost O(trades × holding) per cell, not Θ(E).
  2. Build `SliceFacts` once per execution series in `produce_candidate_universe_v1` and in the V3 replay authority, and pass it down.
  3. Add a runner constructor that takes a precomputed, sealed daily-reference data digest. One option is an opaque `DailyReferenceDigestV1` token minted once by `data_digest_with_daily_reference`. Call `attest_training` once per resolved side, then `evaluate_with_attested` per mask.
  4. Replace `try_reserve_exact` with `try_reserve`, keeping the `bounds.max_rows` check, so growth is geometric again.
  5. **Cost after the fix:** per cell, O(trades × holding). Per (mask, side), one Θ(E) `walk_core` remains, because the mask is tested on every row. That walk is not O(1) and gets an honest bound in §147.
  - **Failing-first tests:**
    - A counting seam (cfg(test) static counter) in `SliceFacts::of` and `data_digest_with_daily_reference`. For a fixture with N closed masks × 2 sides, assert the counts are 1 and 1 after the fix. Today they are cells×2N and 2N.
    - A capacity test proving `rows.capacity()` grows geometrically: realloc count ≤ ⌈log2⌉ over 64 appends.
    - Byte-identity of the produced universe digest before and after (no identity change).
  - **Edge cases:** an empty grid, a zero-trade cell, one bar, `max_rows` exactly hit, foreign execution series refused with the new token, and a token minted from other streams refused.
- **conflicts:**
  - U15: same file, regions 3220-3830 and 6206, plus §150.
  - U14: depends on `materialize_cell_over`. Land U1 first.
  - U2 and U33: same file, other functions.

## U2 fix/cloud-W2-cli3-7
- **findings:** W2-cli3-7 (high bug)
- **files:**
  - `crates/cli/src/candidate_universe.rs` 1545-1590: `CandidateSearchColumnBuilderV1::build`. `final_close_minute` is at 1560-1566 and the exact-close refusal at 1578-1587.
  - Calendar authority: `pull::calendar` (`LAST_MINUTE` 15:29) or `pull::session`.
- **kind:** code-fix
- **dnum:** D-0961
- **verified:** yes, by reading the code; not run.
  - `final_close_minute = ts + signal_length − 60s` has no session clamp.
  - LEDGER_RUNGS (`ledger_all.rs:105`) includes 2, 10, 30 and 60min. Those rungs have a short final bar (60min: 15:15 holds 15:15-15:29, per `pull/src/fold.rs:175-179`).
  - So the demanded close is 16:14, the minute context ends at 15:29, and the build refuses "lacks exact closing minute".
- **goal:**
  - Clamp `final_close_minute` to `min(ts + len − 60s, session_last_minute(ist_day(ts)))`. Take the session's last minute from the same calendar authority the store fold uses (`pull::calendar::kind_of` window), never a literal.
  - If the day has no session window, refuse with that reason.
  - **Failing-first test:** a 60min (and a 10min) prefix ending on the 15:15 short bar, with complete minutes through 15:29. Expect `build` to give Ok and bind minute_end at 15:29. Today it gives Err.
  - **Edge cases:** a special or half session whose close differs, muhurat, a 1min rung (unchanged), a 375-divisible rung (3, 5 and 15min unchanged), a missing 15:29 still refused, and the last minute of the context.
- **conflicts:**
  - U20 needs the same "session last minute per venue/day" helper. If both land, share one helper; the first to merge owns it.
  - U1 and U33: same file, other regions.

## U3 fix/cloud-GAP13-13
- **findings:** GAP13-13 (med bug), R9-cli-o1-0 (med bug), W2-cli1-5 (low bug/doc).
  - R9 merges because the pool pass-1 `par_iter` lines (pool.rs 293-300) are the ones GAP13-13 restructures.
  - W2-cli1-5's false comments sit inside batch `one` (batch.rs 604, 708), which GAP13-13 restructures.
- **files:**
  - `crates/cli/src/batch.rs` 340-370 (`SharedBy::these` at 355, `par_iter` at 357), 540-730 `one` (digest at 556, `BATCH_CEILING` at 596, "same 64 hex" comments at 604 and 708, `record_swept_run` at 722)
  - `crates/cli/src/pool.rs` 80-96 header, 269-300 pass 1
  - `crates/cli/src/lib.rs` 13034-13370 `one_rung`, 14776-14800 `sweep_rungs`, 15899 `SWEEPS_SHARING_THIS_MACHINE`, 16202 `shared_out`
  - `crates/cli/src/sweep_evidence.rs` 498 `begin_many`
- **kind:** code-fix (plus doc-false for W2-cli1-5)
- **dnum:** D-0962
- **verified:** yes.
  - Workers inside `par_iter` (batch.rs:357, pool.rs:294, lib.rs:14786) call `one`/`one_rung`, and those record ledger and evidence rows in thread-completion order.
  - Pool pass 1 never calls `SharedBy::these`, so each concurrent sweep gets the whole-machine ceiling and all cores.
  - batch uses `stored_anchored_digest` plus `BATCH_CEILING`, while sweep-stored uses `stored_executed_digest` (lib.rs:3373). The identities can never be equal, so the "same 64 hex characters" comment is false.
- **goal:**
  1. Allocate attempts in input order with `sweep_evidence::begin_many` before the map.
  2. Workers return a prepared commit (computation only).
  3. Commit sequentially in input order after the indexed collect: `record_swept_run`, frontier, trades, receipt, `attempt.finish`. `one_rung` grows a "prepare" half plus a "commit" half.
  4. Pool pass 1 takes `SharedBy::these(surface.len().min(rayon::current_num_threads()).max(1))`.
  5. Reword the batch comments: the batch identity is a different run (anchored digest, batch ceiling) and is not joinable with sweep-stored by identity. Name what does compare: feed, underlying, month and mask.
  - **Failing-first tests:**
    - Run sweep-all/range-all twice over a 3-instrument fixture with a forced worker delay seam that reverses completion order. Assert the bytes of runs.bin, attempts.bin and `cli results` are identical. Today they differ.
    - A pool-pass-1 test asserting `SWEEPS_SHARING_THIS_MACHINE` reads ≥ min(n, threads) inside `one_rung` (seam).
  - **Edge cases:** one instrument, all refused, a mid-commit refusal (later rows not committed, named), and a rerun reusing identities.
- **conflicts:**
  - U9 changes `latest_for`'s call inside `one_rung` (lib.rs:13358). Land U9 first.
  - U5 and U37: pool.rs header lines 80-96 are adjacent.
  - U41: batch.rs:613 `#[expect]`.
  - U31: pool `opening`.

## U4 fix/cloud-GAP4-46
- **findings:** GAP4-46 (med law)
- **files:**
  - `crates/cli/src/lib.rs`: 5123 `grid_rungs`, 18072-18085 `knobs_checked` (`grid_rungs(bars)` over the whole span at 18080), 19289-19420 `both_shapes` (`walk_forward_splits` and the four `*_with_rungs` calls)
  - `crates/runner/src/validate.rs`: `rungs: usize` at 1782, 1918, 1967, 2011, 2055, 2110 and 4273; `walk_forward_projected*_with_rungs` 1958-2060; the claim near 4631-4736
  - Run-identity term list
- **kind:** code-fix
- **dnum:** D-0963
- **verified:** yes. `knobs_checked` resolves `grid_rungs(bars)` from the full span, test windows included, and `both_shapes` passes it to every fold. The training exit grid therefore depends on test-window bars (look-ahead, §3 rule 7).
- **goal:**
  - Add `FoldRungs::{Fixed(usize), PerTraining(&(dyn Fn(&[Candle]) -> usize + Sync))}` in runner.
  - cli passes `Fixed(n)` when `BRUTEX_GRID_RUNGS` is set, else `PerTraining(grid_rungs)`, which runner evaluates on each fold's training signal slice.
  - Record the per-fold rung count on `FoldResult`.
  - Append a new identity term naming the policy. Do not reinterpret an existing term: identity is positional and append-only.
  - Correct the validate.rs comment.
  - **Failing-first test:** two spans identical except the last fold's test window. Assert every earlier fold's rung count and its whole `FoldResult` are equal. Today they differ when the extra bars move `max_stop_points`.
  - **Edge cases:** a training slice too short to derive a rung count (refuse by name), the env override, one fold, and identity change pinned by a test.
- **conflicts:**
  - U27: lib.rs 5455-5470, nearby but a different function.
  - U26: lib.rs 19210, adjacent to `both_shapes`.

## U5 fix/cloud-GAP13-15
- **findings:** GAP13-15 (med bug), W2-cli9-4 (med test-gap), R9-cli-o1-1 (low cost-doc). All three concern pool pass 2: its preparation, the test that claims to pin it, and its stated cost.
- **files:**
  - `crates/cli/src/pool.rs`: 80-88 header cost text; 767-848 `price_all` (`bars = span.bars` signal bars at 815, `horizon_for(bars, ..)` at 816, `evaluate_over` at 836); tests 2456-2500 `the_pool_prepares_a_span_exactly_as_the_screen_does`
  - `crates/cli/src/lib.rs` `audit_bars_work` / StoredReplay projection, as the reference
  - `docs/06-limits.md` §171 (8314)
- **kind:** code-fix, test-gap and cost-doc
- **dnum:** D-0964
- **verified:** yes.
  - `price_all` evaluates the grid on the coarse signal bars, with the horizon in signal bars. The screen executes on the 1min series.
  - The test's `find` has no end bound. `stored_anchored_column(` is absent from `screen_range_inner`/kernel and matches inside `audit_bars_work`.
  - Each `evaluate_over` is Θ(B) through `walk_core`, which §171 and pool.rs:81 omit.
- **goal:**
  - Prepare pass 2 exactly as audit/screen do. Build the anchored column, project it onto the 1min execution series (`Sourced::Fill`), and call `grid::evaluate_over` over the execution bars with the horizon in execution minutes and one hoisted `SliceFacts`.
  - If that is not done, refuse coarse rungs in pass 2 by name.
  - Replace the source-order test with a behavioural one.
  - Restate the cost as Θ(I × U × (B_exec + cells × T)) in §171 and pool.rs:80-88. U grows with I × top.
  - **Failing-first test:** a generated 60min pool fixture. For every pass-1 Ok instrument, price its recorded (mask, side) through `price_all` and assert `trades > 0` and the cell equals the pass-1 frontier cell. Today 0 of 97 fire.
  - **Edge cases:** the 1min rung (no projection), an instrument with withheld days, an empty union, and an instrument refused in pass 2.
- **conflicts:**
  - U3: pool.rs 80-96 and 269-300.
  - U28: render functions at 656 and 909.
  - U37: `union_of` at 706.
  - U31: `opening` at 620.

## U6 fix/cloud-GAP11-0
- **findings:** GAP11-0 (med bug), W2-cli13-5 (med bug). Both are in `Journal::publish_inner`.
- **files:**
  - `crates/cli/src/search_checkpoint.rs`: 14 `DIRECTORY_LIMIT`, 213-286 `publish_inner` (marker `create_new` then write at 262-266), 432 discover limit, 459-473 discover marker, 487-490 width refusal
  - Consumers, unchanged: and_checkpoint, expression_search, boolean_*, index_stop_search
- **kind:** code-fix
- **dnum:** D-0965
- **verified:** yes.
  - The `complete` marker is created empty, then written. A kill in between leaves a 0-byte marker that `discover` treats as acknowledged, and `read` then refuses "marker width mismatch" forever.
  - `publish_inner` never compares `sequence` against `DIRECTORY_LIMIT`, while `discover_through` refuses at `index == DIRECTORY_LIMIT`.
- **goal:**
  - Write `complete.tmp` with `create_new`, then `write_all(seal)`, `sync_all`, `fs::rename` to `complete`, and sync the directory. Remove the temp file on error.
  - Refuse `publish` before `create_dir` when the entry count (reservations + owner.lock) would reach `DIRECTORY_LIMIT`, with a named refusal. Ok must never be returned for an unreopenable checkpoint.
  - **Failing-first tests:**
    - Reservation N with a synced payload plus an empty `complete` (old) or `complete.tmp` (new). Expect `Journal::open` to give `interrupted()==1`, `latest()==N-1`, and a republish to succeed as N+1.
    - With the limit lowered via cfg(test) const, publishing at the limit refuses and a reopen still succeeds.
  - **Edge cases:** a stray `complete.tmp` with a full seal, ENOSPC at rename, limit−1, and sequence overflow.
- **conflicts:** U21 edits search_checkpoint.rs:334 (`O_NOFOLLOW` flag). That is a separate hunk.

## U7 fix/cloud-GAP11-1
- **findings:** GAP11-1 (med bug)
- **files:**
  - `crates/cli/src/sweep_evidence.rs`: 601-631 `Attempt::ranked` (per-row `write_all` at 629), 1062, 1188 shape header, 1282-1298 `append_row`, 1364-1397 `append_events`
  - Model: `frontier.rs` `append_locked_with` (D-0426 wording)
- **kind:** code-fix
- **dnum:** D-0966
- **verified:** yes. There is no `set_len` anywhere in sweep_evidence.rs. A short write leaves a torn tail, and every later `begin` refuses.
- **goal:**
  - Under the held lock, measure `end = seek(End(0))`. On any write error, `set_len(end)` and return a refusal stating whether the rollback held.
  - `ranked` encodes all rows into one buffer and writes once.
  - Add write seams such as `append_events_with`.
  - **Failing-first test:** an injected writer writes 16 bytes and then errors. Assert attempts.bin keeps its length and the next `begin` of another identity is Ok. Today it fails with "torn or short".
  - **Edge cases:** rollback itself fails (named, loud), zero rows, max rows, the header write on a new file, and a rerun.
- **conflicts:** U3 calls `begin_many` but does not edit these functions.

## U8 fix/cloud-W2-cli8-8
- **findings:** W2-cli8-8 (med bug), W2-cli8-0 (med cost). Both are in `screen_cascade`.
- **files:**
  - `crates/cli/src/lib.rs`: 11170-11195 false comment ("eight tiers cost eight passes over cells already computed"); 11196-11480 `screen_cascade` (mildest probe `widest.selected.is_none()` at 11413, tier loop `body.selected.is_none()` at 11439); 10493-10533 `final_selection` (falls back to a non-admitted traded row); 11668 `screen`
  - `runner/src/grid.rs` 2199-2208 (tier `forced` stop merged into the ladder)
- **kind:** code-fix and cost-doc
- **dnum:** D-0967
- **verified:** yes.
  - `selected` is Some for any traded row, admitted or not, so the cascade prints the strictest tier as "MET" when nothing was admitted.
  - Each tier re-runs `screen`, and the grid differs per tier because `max_mae_ppm` is merged into the stop ladder. The comment is false.
- **goal:**
  - The cascade tests `admitted_any`, not `selected.is_some()`, in both the probe and the loop.
  - Correct the comment.
  - Add a 06-limits entry: O(T × (M + C × 2 × grid)) per cascade. O(1) per tier is not achievable without one tier-union ladder, which would change the grid and the identity. Record that rejected option in the D-entry.
  - **Failing-first test:** a fixture where no row passes any tier but some trade. Assert the output has no "MET", reads "every tier UNMET", and `admitted_any == false`. Today it prints `S++++++ MET`.
  - **Edge cases:** the mildest tier admits only (walk continues), the first tier admits, zero traded rows.
- **conflicts:** U34 (lib.rs 9456-9620 `tiers`), a different function.

## U9 fix/cloud-W2-cli8-9
- **findings:** W2-cli8-9 (med bug), W2-cli8-4 (low cost). Both are in `latest_for`.
- **files:**
  - `crates/cli/src/lib.rs`: 15122-15165 `latest_for` (doc claims O(1) at 15122-15134), call at 13358 inside `one_rung`, `ensure_run_record` 16721+, "RESULT ALREADY RECORDED AND VERIFIED" 16895
  - `results.rs` `index_of_identity`, `open_with` 855
- **kind:** code-fix and cost-fix
- **dnum:** D-0968
- **verified:** yes. `latest_for` returns the newest row matching (feed, underlying, tf, span, min_hits), not the identity. On a Reused rerun it shows another run's figures. Each call also opens `Results::open` (O(runs)) and scans backward.
- **goal:**
  - Pass the run's `RunId` into `latest_for` and read `store.index_of_identity(identity)`. That is an O(1) expected probe after the open.
  - Reuse one opened handle across the rungs of a command, so it is not reopened per rung.
  - State the remaining O(runs) open in docs/06-limits.
  - **Failing-first test:** record run A, then a different run B with the same key, then rerun A (Reused). Assert the rendered row carries A's identity and figures. Today it shows B's.
  - **Edge cases:** identity absent (refuse, named), Reused vs Written, and an empty ledger.
- **conflicts:** U3 (`one_rung`). Merge U9 first.

## U10 fix/cloud-AC-whp-law-0
- **findings:** AC-whp-law-0 (med bug), AC-whp-law-2 (low bug). Both are wrong ledger fields written at the same `Recording`/`record_swept_run` call sites.
- **files:**
  - `crates/cli/src/lib.rs`: 3340 `stored_month_kernel` (`Recording { underlying, .. }` at 3535-3548 with `loaded.bars.len()` at 3548); audit door 6043 and 6099-6110; `Recording` at 6807 and 15470; 13684
  - `crates/cli/src/results.rs` 300 `/// Signal bars swept.`
  - `crates/api/src/backtest.rs` 428 (same doc)
  - `batch.rs` 736 and 755 (`census.swept`)
- **kind:** code-fix (plus doc-false)
- **dnum:** D-0969
- **verified:** yes.
  - `Recording.underlying` is the raw typed word, while the RunId uses `loaded.key`. Running `nifty` then `NIFTY` gives one identity with different fields, so the rerun is refused.
  - sweep-stored and audit record `loaded.bars.len()` (warm-up included). sweep-all records `census.swept`.
- **goal:**
  - Every stored door records `loaded.key.underlying` (canonical).
  - Locked choice for `bars`: record the swept count (`outcome.census.swept`, or the column's swept rows) everywhere and keep the doc. The alternative is to rename the doc to "signal bars offered" and make batch match.
  - Rows already in ledgers hold the old value. A rerun of an old identity would differ on `bars`, so the D-entry must state how `same_run_answer` treats legacy rows: refuse loudly, or name the field. Never silently accept.
  - **Failing-first tests:**
    - `sweep-stored zerodha nifty` then `NIFTY` on a fixture. The second gives "RESULT ALREADY RECORDED AND VERIFIED". Today it is refused.
    - sweep-stored and sweep-all over one month write equal `bars`.
  - **Edge cases:** mixed-case equities, a month entirely warm-up (0 swept), and a legacy row.
- **conflicts:**
  - U41: same Run literal block, lib.rs 3378-3390.
  - U3: batch.rs `one` record call at 722-755.

## U11 fix/cloud-W2-cli9-3
- **findings:** W2-cli9-3 (med bug), W2-cli8-6 (low cost). Same root cause: the gap census misses holes at a session's edge, so the column build discovers them one refusal at a time in a 64-attempt reload loop.
- **files:**
  - `crates/cli/src/minute_gaps.rs` 198-225 `days_with_interior_gaps` (same-day-only test at 210-214), 252 `withhold`
  - `crates/cli/src/lib.rs` 791-915 `exact_minute_withholding_unsourceable_days` (`ATTEMPTS` 799), 916-990 `column_withholding_at_build` (`ATTEMPTS` 926, reload loop 933-980)
  - Callers: `pool::price_all` (pool.rs:797), screen kernel
- **kind:** code-fix and cost-fix
- **dnum:** D-0970
- **verified:** yes.
  - A missing 15:25-15:29 (or a day with signal bars and no minutes) is never flagged, because the step to the next 09:15 fails `same_day`.
  - Each retry reloads both contexts, re-digests, writes an attempt and rebuilds the column.
- **goal:**
  - Make the census expectation-based. For each IST day that has signal bars, compare the first and last stored minute against that day's calendar session window (`pull::calendar`), and flag interior gaps as before. This gives one O(M + days) pass.
  - Withhold every flagged day up front, so the retry loop becomes a defence that should run at most once. Keep it bounded and loud.
  - Document the remaining bound.
  - **Failing-first tests:**
    - Day D missing 15:25-15:29 while other days hold them gets flagged, and pool/screen sweep the span instead of refusing.
    - A day with signal bars and zero minutes gets flagged.
    - A counter seam shows `load_exact_minute_context` is called once for a span with 3 edge-holed days. Today it is called 4 times.
  - **Edge cases:** a short session (special day), the first day of the span, the last day, the whole span holed (refuse), muhurat.
- **conflicts:**
  - U2 and U20 use the same session-window authority (share the helper).
  - U5 (`price_all` calls the census).

## U12 fix/cloud-W2-cli9-5
- **findings:** W2-cli9-5 (med bug)
- **files:** `crates/cli/src/operation_audit.rs`: 548-567 `begin` (journal `create_new` at 561-566, first `write_synced` at 567), 600-630 `read` (`bytes == 0` refusal at 621), 663-668 `page`, 307-315 `length`
- **kind:** code-fix
- **dnum:** D-0971
- **verified:** yes. A crash between `create_new` and the first synced record leaves a 0-byte journal. `read` errors on it, so every page covering that id refuses permanently.
- **goal:**
  - Treat a 0-byte journal exactly like NotFound: return the unconfirmed start (`Ok(Some(started))`).
  - Alternatively, create the journal atomically (write the temp, sync, rename).
  - A partial record (0 < len < STRIDE) still refuses loudly.
  - **Failing-first test:** `begin`, then truncate the journal to 0. Expect `read` to give `Ok(Some(started))` and `page` to succeed. Today both fail.
  - **Edge cases:** len == STRIDE−1, a journal whose first record mismatches, a NotFound journal.
- **conflicts:** U21 edits operation_audit.rs:303 flags, a separate hunk.

## U13 fix/cloud-W2-cli13-4
- **findings:** W2-cli13-4 (med bug)
- **files:**
  - `crates/cli/src/results.rs` 707-750 `open_result_file` (the type check runs after open, at 724-731)
  - `crates/cli/src/result_set.rs` 280 and 298 (`File::open`)
  - `crates/cli/src/pre_admission_data.rs` 3601-3609 `open_file`
  - Existing helper `crates/cli/src/readonly_file.rs` (FLAGS = NOFOLLOW|NONBLOCK)
  - HTTP path `crates/api/src/detail.rs` 463-471
- **kind:** code-fix
- **dnum:** D-0972
- **verified:** yes. None of the three opens sets O_NONBLOCK or checks the type before opening, so a FIFO blocks a read-only open forever.
- **goal:**
  - Route the read-only opens through one helper. It `symlink_metadata`-checks `is_file()` before open, opens with O_NONBLOCK|O_NOFOLLOW (`readonly_file`), and re-checks `fstat` `is_file()` after.
  - Writable opens refuse non-regular files the same way.
  - **Failing-first test (unix):** `mkfifo` at `results/runs.bin`, `detail-sets.bin` and a pre-admission data path. Each `open_read` returns Err within 1s (run in a thread with a timeout). Today it hangs.
  - **Edge cases:** a socket, a directory, a symlink to a regular file (refused), NotFound unchanged.
- **conflicts:**
  - U21 changes the `readonly_file.rs` flag constant per arch. Land U21 first or rebase.
  - U38 edits pre_admission_data.rs `file_generation` 3611-3700, adjacent to `open_file`.

## U14 fix/cloud-W2-cli2-6
- **findings:** W2-cli2-6 (med cost), W2-cli2-9 (med cost). Merged because both must rewrite the same module header (candidate_trades.rs lines 1 and 7: "UNVERIFIED performance" and "Pages are bounded").
- **files:**
  - `crates/cli/src/candidate_trades.rs`:
    - 1-7 header
    - 362-440 `record_inner` (`shown_cell` recheck at 375, `cells.contains` at 398, `materialize` at 401, `write_exact` ×2 at 429-434)
    - `materialize` around 446-470 (calls `grid::materialize_cell` at 466)
    - 640-660 `read_model`
    - 739-757 `pinned` (callers 764, 798, 821, 908, 912)
    - 1217-1251 `read_sealed`
  - `lib.rs` 11643 `shown_cell`
- **kind:** cost-fix and cost-doc
- **dnum:** D-0973
- **verified:** yes.
  - `pinned` re-reads and blake3-hashes the whole catalog (O(C)) on every page, tier read or reader open.
  - `record_inner` rescans the cells (`shown_cell` up to 3 scans plus a linear `contains`), replays a cell through `per_trade` (Θ(B) `SliceFacts`), and does about 4 fsyncs per candidate side.
- **goal:**
  - `pinned`: hash the catalog once when the `Summary` is opened, and check only the `FileGeneration` (as `TradeReader::page` does) on later pages. A page becomes O(page). The cold open stays O(C) and is documented.
  - `record_inner`:
    - Replace `contains` with a check of the selected ordinal's cell.
    - Keep the independent `shown_cell` recheck, but note it is O(cells).
    - Use `materialize_cell_over` with facts hoisted once per Capture (needs U1).
    - The fsyncs per candidate are inherent. Give an honest 06-limits entry with a measured count per candidate.
  - **Failing-first tests:** counting seams. (a) `read_sealed` is called once across 10 pages of one summary; today it is 10. (b) `SliceFacts::of` is called once per capture of N candidates.
  - **Edge cases:** the catalog changes between pages (must still refuse via generation), an empty catalog, a zero-trade selected cell.
- **conflicts:** U1 provides the runner API, so land U1 first.

## U15 fix/cloud-W2-cli3-4
- **findings:** W2-cli3-4 (med cost), W2-cli3-8 (low doc-false). Both falsify docs/06-limits §150 and the module header of `CandidateUniverseLedgerV1`.
- **files:**
  - `crates/cli/src/candidate_universe.rs`: 10-16 header ("sealed internal append is O(new rows)"); 3220 `open`, 3233 `open_read`, 3339-3402 `scan`; 3756-3764 `require_unchanged` (metadata only); 3795-3830 `append_produced_candidate_universe_v1` (open, then append, then a fresh `open_read` and `reopen_audit`); 6206-6250 `file_generation`/`generation_of`
  - `docs/06-limits.md` §150, lines 7621-7647: "proportional to the new block" at 7628 and "also hashes the data files" at 7632-7633
  - Callers: `ledger_v6.rs:330`, `all_rung_population_v5.rs:586,610`
- **kind:** cost-doc and doc-false (cost-fix optional)
- **dnum:** D-0974
- **verified:** yes.
  - Every production append does 2 full O(R_total + C) opens (scan, seal and semantic re-hash per row). That contradicts "O(new rows)".
  - The generation check is metadata only (len, dev, ino, mtime, ctime) and hashes no data, which contradicts §150.
- **goal:**
  - Correct the header and §150. An append is O(R_total + C) twice plus O(new rows).
  - Remove "also hashes the data files", or add content hashing; the decision chooses. The recommendation is to correct the doc, because the metadata generation is the intended cheap check.
  - Optional cost-fix: run the second `reopen_audit` against the still-held handle's index after a generation re-check, plus a re-read of only the appended block. That makes the append O(new rows) after one open per process. Pass one opened ledger through the 16 appends per run.
  - **Failing-first test:** a doc-pinning test that greps §150 for "hashes the data files" and fails while the code has no hash. Plus, if the cost-fix is taken, a counting seam: `scan` runs ≤1 per append, today 2.
  - **Edge cases:** an append onto an empty ledger, a Reused append, and a foreign modification between append and reopen (must refuse).
- **conflicts:** U1 (same file and §150 neighbour §147). Coordinate the §150 hunk.

## U16 fix/cloud-W2-cli7-2
- **findings:** W2-cli7-2 (med cost), W2-cli7-3 (med cost). Both are in `run_route`.
- **files:**
  - `crates/cli/src/ledger_v6.rs` 289-414 `run_route` (`size_sweeper` at 309, NIFTY and BANKNIFTY commits at 326-345), 647-670 `replay_route` (`run_route` at 667)
  - `crates/cli/src/strict_v6_inputs.rs` 54-92 `size_sweeper` (full strict `load` of NIFTY only to read `signal.bars.len()`)
  - `docs/06-limits.md` §151 (7649) / new
- **kind:** cost-fix (duplicate load) and cost-doc (replay recompute)
- **dnum:** D-0975
- **verified:** yes. `size_sweeper` does a cold checksum-audited NIFTY load per rung, and the NIFTY family commit loads the same span again. `ledger-v6-replay` re-runs the whole 8×2 route before reuse is decided.
- **goal:**
  - `size_sweeper` returns the loaded NIFTY context, and the NIFTY family commit consumes it (one load per rung per family).
  - Replay: whether reuse applies needs the data digest, which needs the load, so O(1) is not achievable without a request-keyed committed-route index. Document the route cost honestly in 06-limits.
  - **Failing-first test:** a counter seam on strict `load`. One `run_route` over the fixture does 8×2 loads; today it does 8×3.
  - **Edge cases:** the source changes between sizing and commit (`require_current` must still refuse), and a rung with no data.
- **conflicts:** U31 (ledger_v6.rs 245 and 613 banner lines), separate hunks.

## U17 fix/cloud-W2-cli8-7
- **findings:** W2-cli8-7 (med cost)
- **files:**
  - `crates/cli/src/lib.rs` 12375-12380 `measured_band`, 12382-12560 `measure_top` (sequential loop at 12478, `evaluate_over` per row, `consistency_of`), 8920 `BRUTEX_TOP`
  - `crates/cli/src/strict_range_knobs.rs` 8, 64-66 and 193 (`BRUTEX_TOP` only checked > 0)
- **kind:** cost-doc and cost-fix (cap plus parallel)
- **dnum:** D-0976
- **verified:** yes. The loop measures `min(8 × top, rows)` rows sequentially, each a full grid rebuild plus `per_trade` plus 7 grains. `BRUTEX_TOP` has no ceiling.
- **goal:**
  - Give `BRUTEX_TOP` a named ceiling, refused by name in `strict_range_knobs`.
  - Run the band with an indexed `par_iter` and a sequential write-back, so the result is deterministic.
  - Add a 06-limits entry: O(band × (grid + 7 × trades)). Per-row grid rebuild is inherent (memory trade-off per the comment).
  - **Failing-first tests:** `BRUTEX_TOP` above the ceiling is refused, and `measure_top` output is byte-identical between the sequential and parallel runs.
  - **Edge cases:** top = 1 (floor 32), rows fewer than the band, the ceiling exactly.
- **conflicts:** none significant (U8 is at 11170-11480).

## U18 fix/cloud-W2-cli12-0
- **findings:** W2-cli12-0 (med cost), W2-cli12-1 (low cost), W2-cli12-2 (low cost). All three are Statistics V2 open, prepare and append cost, and all three touch §154.
- **files:**
  - `crates/cli/src/population_statistics_v2.rs`: 2170-2240 `open_inner` (`audits.try_reserve(bounds.audits)` at 2224-2227); scan around 2240-2302; 3660-3690 `build_raw_candidates` (whole-vector filter per candidate at 3678 and 3683); 3751 `PreparedPopulationStatisticsV2::new`; 4697-4730 `append_population_statistics_v2` (`open_writer` plus a full `open_read`)
  - `ledger_all.rs` 1077 and 1230 (`CEILING_RECORDS = 1<<24`)
  - `docs/06-limits.md` §154 (7717)
- **kind:** cost-fix (12-0, 12-2) and cost-doc (12-1)
- **dnum:** D-0977
- **verified:** yes.
  - Each candidate filters all C×P periods and C×S splits, which is O(C² × (P+S)).
  - Every open reserves `bounds.audits` (1<<24) map slots before counting.
  - Each append runs two full opens, and each re-runs every stored bootstrap (1,000 draws).
- **goal:**
  - Group periods and splits in one pass, period-major then split-major, using index arithmetic (`sequence × P + p`), so each candidate is O(P+S).
  - Reserve `min(bounds.audits, records counted by scan)`, or grow on demand.
  - Append: document the cumulative O(A × block) cost in §154. The optional fix is the same held-handle reopen as U15.
  - **Failing-first tests:**
    - Equivalence: the prepared bytes are identical to the old implementation over a 50-candidate fixture.
    - A counting seam: period visits equal C×P, not C²×P.
    - Opening with `bounds.audits = 1<<40` on an empty ledger succeeds with small capacity. Today it fails on the reserve, or allocates huge.
  - **Edge cases:** zero candidates, P = 0, S = 0, unsorted input (refuse), `audits` bound exceeded by the scan.
- **conflicts:**
  - U22: same file, 4540-4660, adjacent to 4697.
  - U21: line 110 const.

## U19 fix/cloud-GAP11-3
- **findings:** GAP11-3 (low bug)
- **files:**
  - `crates/cli/src/lib.rs` 18459-18472 `record_unadmitted`
  - Reference `record_all_attempt` 19630+ (LEDGER lock, `ResultSetLock`, `confirm_result_directory` before `record_run`)
  - Source-order test around 23958
- **kind:** code-fix
- **dnum:** D-0978
- **verified:** yes. `record_unadmitted` calls `record_frontier`, then `ensure_detail_receipt`, then `record_swept_run` with no LEDGER mutex, no `ResultSetLock` and no `confirm_result_directory`.
- **goal:**
  - Take `LEDGER.lock()` plus `ResultSetLock::acquire`, and call `confirm_result_directory(into.root)` after `ensure_detail_receipt` and before `record_swept_run`.
  - **Failing-first test:** extend the source-order test so `record_unadmitted` names `ResultSetLock::acquire` and `confirm_result_directory` before `record_swept_run(`.
  - Plus a behavioural test: a concurrent writer seam that is blocked while the lock is held.
  - **Edge cases:** a poisoned mutex (named refusal), a directory confirm failure (no ledger row).
- **conflicts:** U32 changes `record_frontier`, which this function calls. Hunks are separate unless the signature changes.

## U20 fix/cloud-GAP12-6
- **findings:** GAP12-6 (low bug)
- **files:**
  - `crates/cli/src/stored.rs` 3022 `prior_accepted_session`, 3172-3240 terminal geometry check (refusal text at 3235), test at 4762
  - `pull::calendar::kind_of`, `pull::fold::minute_session`, `pull::vendor::cash_auction_eligibility_required`, `pull/src/cash_session_cache.rs`
- **kind:** code-fix
- **dnum:** D-0979
- **verified:** yes, by reading the code. `prior_accepted_session` uses the venue-blind `kind_of`. For an NSE cash key on a CAS-eligible day, the correct close at 15:14 is refused as "Early or truncated".
- **goal:**
  - For NseCash keys on days where `cash_auction_eligibility_required` holds, derive the window from `pull::fold::minute_session` with the dated cash schedule.
  - Where no schedule is available, refuse naming CAS ("cash session close UNVERIFIED: dated CAS eligibility required"). Never say "truncated".
  - **Failing-first test:** `a_cas_equity_prior_session_ending_1514_seeds_gapfib` expects Ok, or, with no schedule, an Err naming CAS. Today it fails with "Early or truncated".
  - **Edge cases:** an index key unchanged, a pre-CAS date, a CAS-eligible day ending at 15:29 (refuse as truncated? the decision states which).
- **conflicts:** U2 and U11 (shared session helper); U40 (stored.rs 2934, separate hunk).

## U21 fix/cloud-W2-cli4-2
- **findings:** W2-cli4-2 (low bug)
- **files (every hard-coded Linux 0x20000 O_NOFOLLOW):**
  - `crates/cli/src/`: execution_lease.rs:61, readonly_file.rs:17, operation_audit.rs:303, search_checkpoint.rs:334, checksum_receipts.rs:512, selection_v6.rs:234, global_replay_v4_store.rs:185
  - `O_NOFOLLOW_FLAG` consts in `crates/cli/src/`: execution_v3.rs:131, execution_v4.rs:135, anchored_search_lineage_v2/v3/v4.rs:70/71/69, population_finalization_v2/v3/v4.rs:119/89/88, population_statistics_v2/v3.rs:110/89, population_admission_v2/v3/v4.rs:80/112/98, population_v5/v6.rs:92/109, selection_v5.rs:100
  - `crates/store/src/checksum_audit.rs:42`
- **kind:** code-fix
- **dnum:** D-0980
- **verified:** yes. 0x20000 is O_NOFOLLOW only on x86_64. On aarch64/arm Linux it is O_LARGEFILE, and O_NOFOLLOW is 0x8000, so these opens follow symlinks there. 0x800 (O_NONBLOCK) is the same on both.
- **goal:**
  - Add one `pub(crate) const O_NOFOLLOW` per (target_os, target_arch) in one module: x86_64 0x20000, aarch64 0x8000, macOS 0x100. Add `compile_error!` for any other Linux arch, with no silent default.
  - No libc dependency; that would need a deny check.
  - Replace all sites. `store` needs its own copy, or `core` hosts it if the graph allows (core depends on nothing, so that is fine).
  - **Failing-first test:** a cfg(target_arch="aarch64") test that the constant equals 0x8000. Plus a portable test: open a symlink through each helper and get ELOOP. That runs on x86_64 CI today and fails if a site is missed.
  - Add a source gate test that greps `crates/` for the literal `0x20_000`/`0x20000` outside the one module.
  - **Edge cases:** musl vs gnu (same values), macOS unchanged.
- **conflicts:** single-line hunks in files owned by U6, U12, U13, U18 and U22. Trivial to rebase; land early.

## U22 fix/cloud-W2-cli12-5
- **findings:** W2-cli12-5 (low bug)
- **files:** `crates/cli/src/population_statistics_v2.rs` 4535-4570 `append_locked` (orphan dispatch at 4541-4544; byte check at 4567 on the new-write path only), 4607-4660 `resume_orphan` (writes and syncs at 4631-4648; the refusal comes only after, at 4657-4658)
- **kind:** code-fix
- **dnum:** D-0981
- **verified:** yes. `resume_orphan` appends the rest of the planned block plus the Completion and syncs, with no `bounds.file_bytes` check, then refuses afterwards.
- **goal:**
  - Compute `desired = current_len + remaining planned bytes + completion` before any write in `resume_orphan`, and refuse when it exceeds `bounds.file_bytes`, leaving the file untouched.
  - **Failing-first test:** create an orphan, then retry with a ceiling below the full block. Assert Err and an unchanged file length. Today the bytes are written, then refused.
  - **Edge cases:** the ceiling exactly equal (Ok), orphan already complete but missing its receipt, overflow in `desired`.
- **conflicts:** U18 (same file, adjacent), U21 (const at 110).

## U23 fix/cloud-GAP16-25
- **findings:** GAP16-25 (low bug)
- **files:** `crates/cli/src/lib.rs` 7781-7830 `render_top_record` (truncating `mean_milli_paisa / 1_000` at 7822 and `t_milli / 10` at 7823)
- **kind:** code-fix
- **dnum:** D-0982 (locked rounding rule: half away from zero, or §7 half-up)
- **verified:** yes.
- **goal:**
  - Add one integer helper `div_round_half_away(n, d)` and use it for both reductions. Keep the sign for values in (−1, 0).
  - **Failing-first test:** `render_top_record` with `mean_milli_paisa` 47_600 and `t_milli` 2_999 contains "₹0.48" and "3.00". −600 renders "-₹0.01". Today: "₹0.47", "2.99", "₹0.00".
  - **Edge cases:** i64::MIN and MAX (no overflow), exact halves, zero.
- **conflicts:** U35 (lib.rs 7867-8000, adjacent function).

## U24 fix/cloud-W2-cli8-10
- **findings:** W2-cli8-10 (low bug)
- **files:**
  - `crates/cli/src/lib.rs`: USAGE 454-459; `elite_arm` 1170-1230 (accepts 0 at 1194); dispatch at 2096
  - 14140-14160 `elite_descend_in_points_inner` (`max_points <= 0` refusal at 14150)
- **kind:** code-fix
- **dnum:** D-0983
- **verified:** yes. 0 is documented and accepted by `elite_arm` as "no ceiling", then always refused by the inner function.
- **goal:**
  - In the inner function, map 0 to "no ceiling beyond the derived ladder": use the derived ladder's own maximum, or `None`. Refuse only negative values.
  - Correct the refusal text.
  - **Failing-first test:** `elite ... 0 N` over a fixture does not contain "refused: the stop ceiling" and sweeps the derived ladder. Today it is refused.
  - **Edge cases:** −1 refused, i64::MAX, the api `sweeprun.rs` caller.
- **conflicts:** none.

## U25 fix/cloud-W2-cli8-11
- **findings:** W2-cli8-11 (low bug)
- **files:** `crates/cli/src/lib.rs` 2326-2345 `parse_support_ppm` (`> 1_000_000` at 2336), 2347-2352 `parse_support_choice`, 8130-8140 `support_from_knob` (`< 1_000_000`), comment at 13147
- **kind:** code-fix
- **dnum:** D-0984
- **verified:** yes. argv accepts 1,000,000 and the knob refuses it. The comment says both refuse.
- **goal:**
  - One shared validator refusing 0 and ≥ 1,000,000 (the decision fixes the bound), used by both doors.
  - Fix the comment.
  - **Failing-first test:** `parse_support_ppm("1000000")` is Err, and the knob and argv agree on {0, 1, 999_999, 1_000_000, 1_000_001}.
  - **Edge cases:** non-numeric input, leading +.
- **conflicts:** U3 (`one_rung` comment at 13147 is inside `one_rung`; tiny hunk).

## U26 fix/cloud-ET-strategies-trades-ranking-costs-3
- **findings:** ET-strategies-trades-ranking-costs-3 (low bug)
- **files:**
  - `crates/cli/src/lib.rs` 19210-19232 `overfitting_of` (`runner::pbo::place` at 19226)
  - `crates/runner/src/pbo.rs` 160 `anchored_walk_forward_bottom_half_rate_v1`, 190 `place_v1`, 340 `probability_of_overfitting`, 398-409 `place` (`winner_rank_twice() / 2`)
  - `runner/src/audit.rs` 1185 `overfitting`
- **kind:** code-fix
- **dnum:** D-0985
- **verified:** yes. The live audit uses the legacy `place`, which rounds an exact half-rank toward the better half.
- **goal:**
  - Use `place_v1` and the exact aggregate (`anchored_walk_forward_bottom_half_rate_v1`, or a v1 PBO over `PlacementV1`). Render through an audit path that takes the exact type.
  - **Failing-first test:** folds where the winner's out-of-sample rank is exactly at the half boundary in the bottom half. The audit counts it as below median. Today it does not.
  - **Edge cases:** one candidate, ties, an even and an odd count.
- **conflicts:** U4 (adjacent `both_shapes` region).

## U27 fix/cloud-ET-strategies-trades-ranking-costs-2
- **findings:** ET-strategies-trades-ranking-costs-2 (low bug)
- **files:** `crates/cli/src/lib.rs` 5455-5470 SAMPLE block (prints `WALK_FORWARD_SPLITS` and divides by it), 5742 const, 5772 `walk_forward_splits`
- **kind:** code-fix
- **dnum:** none (presentation of an existing value)
- **verified:** yes.
- **goal:**
  - Print `walk_forward_splits(bars.len())`, the value `both_shapes` uses, and divide by it.
  - **Failing-first test:** a span where `walk_forward_splits(n) != 5`. The SAMPLE line equals the WALK-FORWARD section's fold count.
  - **Edge cases:** 0 or 1 folds (no division by zero).
- **conflicts:** U4 (both read `walk_forward_splits`).

## U28 fix/cloud-W2-cli9-7
- **findings:** W2-cli9-7 (low bug), GAP13-16 (low bug). Both are pool table renderers.
- **files:** `crates/cli/src/pool.rs` 656-700 `render_per_symbol` (sort, then `rows.reverse()` at 676, so refusals render first), 909-1000 `render_pooled` (`{:>12}{:>5}` net and dd>= with no separator, header at 939-940 and row around 954)
- **kind:** code-fix
- **dnum:** none (makes the code match the documented "refusals last")
- **verified:** yes.
- **goal:**
  - Sort Ok rows descending, then append Err rows, as an explicit two-part order. No `reverse` over the mixed key.
  - Widen and separate the columns as `{:>12} {:>9}` in the header and the row.
  - **Failing-first tests:**
    - `render_per_symbol` with one Err and two Ok rows: the REFUSED line comes last.
    - `render_pooled` with net −16462 and dd_bound 123456: the two print as separate tokens. Today they print as "-16462123456".
  - **Edge cases:** all refused, all Ok, ties.
- **conflicts:** U5 and U31 (adjacent functions in pool.rs).

## U29 fix/cloud-W3-indicators2-1
- **findings:** W3-indicators2-1 (low bug), W3-indicators2-0 (low cost). Both are in `isqrt_i128_counted`.
- **files:**
  - `crates/indicators/src/vwap.rs` 60-75 and 130-195 docs, 196-218 `isqrt_i128_counted` (oscillating loop at 203-207), `NEWTON_STEPS` and `STEP_DOWN_STEPS`, `ITERATION_CEILING`
  - Callers `sigma` (343), 457 and 513
  - `docs/06-limits.md` §51 (3465-3500: "sixty-nine", "130 iterations")
- **kind:** cost-fix (smaller constant) and doc-false
- **dnum:** D-0986
- **verified:** yes. Traced by hand: v=3 oscillates 2↔1 until the 128-step cap. Every v = k²−1 takes 128-129 iterations, not the documented 1-69. The root stays exact.
- **goal:**
  - Use decreasing Newton with the textbook exit: stop when `next >= guess`. Seed with `1 << ((bits(v)+1)/2)`, which is ≥ √v. That gives ≤ ~7 iterations for 127-bit inputs and no oscillation.
  - Keep a hard ceiling constant.
  - Re-measure and correct the §51 and vwap.rs figures (label measured vs proved).
  - **Failing-first test:** `isqrt_i128_counted(k²−1)` for k in {2, 3, 12, 975, 10^15, isqrt(i128::MAX)} returns k−1 with count ≤ a small bound such as 10. Today the count is 128+.
  - Also add an exhaustive check over 1..=10^6 and the i128 extremes.
  - **Edge cases:** v ≤ 0, v=1, perfect squares, i128::MAX.
- **conflicts:** no lane-3 `fix/cloud-*` branch exists. Indicators lane-3 work may arrive later, so re-check with `ls-remote` before pushing. Also touches the docs/06-limits early section §51.

## U30 fix/cloud-AC-whp-tb-9
- **findings:** AC-whp-tb-9 (low bug: invariant rows misstate their tests)
- **files:**
  - `crates/indicators/tests/invariants.rs` 108-140 (V-03 doc vs code), plus the V-04 `daily_mask_clears` and V-05 tests
  - `docs/04-invariants.md` 145-147 and 982-984
  - Gate 10 in `.github/workflows/ci.yml` (ignores the module segment)
- **kind:** doc-false and test-gap
- **dnum:** D-0987 (rewrite of the V-04 and V-05 claims)
- **verified:** yes.
  - V-03's doc promises i64::MIN prices and a far-future ts. The code writes ±MAX/4 and never touches ts.
  - V-04 claims daily-timeframe clearing that no code does.
  - V-05 is 5 hand triples of `DailyLevels`, not a random differential of the Evaluator.
  - The module paths `indicators::proptest::`/`unit::` do not exist.
- **goal:**
  - V-03: make the test do what the row says (also mutate `ts_micros` far future and use i64::MIN/MAX where the fold permits), or reword. Prefer strengthening.
  - V-04: restate it as "VWAP bits cleared under `Availability::Absent`", which the test proves. Drop the time-of-day/daily claim.
  - V-05: either write a seeded-random differential test of the `Evaluator` vs a naive reference, or rename the row to what it tests.
  - Fix the module paths. Optionally make gate 10 check the module segment.
  - **Failing-first:** the strengthened V-03, and a gate-10 path check that fails on the current rows.
  - **Edge cases:** cut=1, cut=len−1.
- **conflicts:** none in indicators (no `fix/cloud-*` branches). ci.yml is shared, so a gate-10 change conflicts with any other gate edit.

## U31 fix/cloud-GAP15-21
- **findings:** GAP15-21 (low doc-false), R9-cli-law-3 (low bug). Both are false single-instrument, single-month STORED_PROVENANCE claims on multi-instrument pages.
- **files:**
  - `crates/cli/src/lib.rs` 2641-2675 `STORED_PROVENANCE` and `stored_provenance*`, test at 21619 `the_generated_and_stored_banners_make_opposite_claims`
  - `crates/cli/src/pool.rs` 620-655 `opening`
  - `crates/cli/src/ledger_v6.rs` 245 and 613
  - `crates/cli/src/ledger_all.rs` 826
  - `crates/cli/src/boolean_catalog_prepared.rs` 49
- **kind:** doc-false (report text)
- **dnum:** D-0988
- **verified:** yes. These pages print STORED_PROVENANCE, which promises that "the run identity beneath names the exact column" and that "A figure here describes that instrument and that month". The pages print no per-run identity, and the figures span many instruments and months.
- **goal:**
  - Add `STORED_POOLED_PROVENANCE`: REAL MARKET DATA, promising no single instrument or month, and naming where identities live.
  - Better, print the nine-term identity per instrument row in the pool pass-1 table, and per family×rung on the ledger pages.
  - Use it on pool, ledger-v6, ledger-v6-replay, ledger-all and the Boolean pages.
  - **Failing-first tests:**
    - Extend the opposite-claims test with the new banner (it must not equal either).
    - A page test: the pool and ledger_v6 pages do not contain "describes that instrument and that month" unless every reported instrument has a printed identity.
  - **Edge cases:** refusal pages (they currently assert the banner is absent; keep that).
- **conflicts:** U5 and U28 (pool.rs adjacent), U16 (ledger_v6.rs other lines), U3 (pool pass-1 table, if identities are added there).

## U32 fix/cloud-W2-cli8-3
- **findings:** W2-cli8-3 (low cost), GAP13-14 (low doc-false). Both are in the frontier/trade/receipt commit helpers; 16628 is inside `record_frontier`.
- **files:** `crates/cli/src/lib.rs` 16405-16490 `ensure_trade_rows`/`record_trades` ("interrupted attempt" at 16483), 16491-16640 `record_frontier` (full `sort_by_key` with a recomputed key at 16541-16568, "interrupted attempt" at 16628), 16641-16700 `ensure_detail_receipt` (at 16662)
- **kind:** cost-fix and doc-false
- **dnum:** D-0989
- **verified:** yes.
  - The sort is O(K log K) key evaluations over all K retained rows (HashMap probe plus Wilson sqrt per comparison), followed by `take(top)`.
  - An idempotent rerun says "from an interrupted attempt".
- **goal:**
  - Compute keys once (`sort_by_cached_key`, or a Vec<(key, &row)>). Use `select_nth_unstable_by` to cut to `top`, then sort only the top. That is O(K + top log top) with K key evaluations, and the tie order stays deterministic: include the mask words as the final key.
  - Word the Reused branches neutrally, or pass `holds(identity)` to say "from the recorded run" vs "from an interrupted attempt".
  - **Failing-first tests:**
    - Equivalence: the written frontier bytes are identical to the old full sort over random fixtures with ties.
    - A key-evaluation counter ≤ K.
    - The exact-retry test (`audited_range_tests.rs:524`): the second report does not contain "interrupted attempt".
    - A simulated interruption keeps that wording.
  - **Edge cases:** K=0, top > K, all-equal keys.
- **conflicts:** U19 (`record_unadmitted` calls `record_frontier`).

## U33 fix/cloud-W2-cli3-3
- **findings:** W2-cli3-3 (low cost)
- **files:**
  - `crates/cli/src/stored_post_training_oos.rs` 334-400 `mint_witness_recorded`/`mint_witness_inner` (builds `CandidateGlobalReplayOosSourceV1::new` per witness at about 384)
  - `crates/cli/src/candidate_universe.rs` 1300-1400 (`require_integrity` 1305, `mint_witness_recorded` 1356)
  - `crates/cli/src/population_v6.rs` 3195-3230
  - `docs/06-limits.md` §169 (8232-8246)
- **kind:** cost-fix
- **dnum:** D-0990
- **verified:** yes. The cohort is cached per family, but every witness rebuilds the source (full anchored column, overlay, alignment and digests), then `require_integrity` re-walks it. §169 says minting is proportional to the replay.
- **goal:**
  - Build the OOS source once per cohort, lazily (it depends only on cohort fields: widths, availability, thresholds, streams).
  - Each witness then pays `require_integrity` (O(E), or cache a sealed check) plus the replay.
  - Update §169 with the per-witness term that remains.
  - **Failing-first test:** a counter seam. `CandidateGlobalReplayOosSourceV1::new` is called once for 3 witnesses of one family; today it is called 3 times. Witness bytes are unchanged.
  - **Edge cases:** the cohort's source becomes stale (integrity must still refuse), a family with 0 winners.
- **conflicts:** U2 and U1 (same file, other regions).

## U34 fix/cloud-W2-cli8-1
- **findings:** W2-cli8-1 (low cost)
- **files:**
  - `crates/cli/src/lib.rs` 9456-9480 `stop_rungs_in_points` (`max_stop_points(bars)` re-evaluated per rung inside `filter`, at 9468), 9563-9620 `tiers` (`trades_needed_for` inside the stop × ratio × rate loop, at 9607)
  - `max_stop_points`, `range_percentile`
- **kind:** cost-fix
- **dnum:** none (pure hoist and memoization)
- **verified:** yes.
- **goal:**
  - Hoist `max_stop_points(bars)` out of the filter: O(N) once.
  - Memoize `trades_needed_for` per (rate, assurance), since it is independent of stop and ratio.
  - Add a 06-limits line for the remaining O(T log T) sort over T ≤ 64×28×396 generated tiers. Correct the ci.yml gate-11 rationale ("a compile-time list of eight") if it refers to this file.
  - **Failing-first test:** a counter seam. `range_percentile` is called once per `stop_rungs_in_points` call; today it is called once per rung. Tier output is byte-identical.
  - **Edge cases:** empty bars, all rungs filtered.
- **conflicts:** U8 (cascade consumes `tiers`).

## U35 fix/cloud-W2-cli8-5
- **findings:** W2-cli8-5 (low cost)
- **files:** `crates/cli/src/lib.rs` 7635 `LIST_ROWS`, 7867-7950 `newest_complete` (forward scan of every row), 7952-8060 `results_at` (collects every matching Record, prints 40), 7079 `best_complete_newest_first`
- **kind:** cost-fix and cost-doc
- **dnum:** D-0991
- **verified:** yes. The O(rows) reads are documented as "O(rows) — the size of the answer", but the printed answer is ≤ 40 + 1 rows.
- **goal:**
  - `results_at`: keep a bounded window of the first `LIST_ROWS` matches plus a running best (O(1) memory beyond 41 records), still one O(rows) pass.
  - `newest_complete`: scan backward and stop at the first complete match.
  - Correct the rustdoc to "O(ledger rows) reads, O(1) retained", and add a 06-limits line. Per-request O(1) is not achievable without a secondary index.
  - **Failing-first test:** byte-identical output vs today on a fixture of 500 rows, plus an allocation or capacity assertion that `rows.len() ≤ LIST_ROWS + 1`.
  - **Edge cases:** an empty ledger, all rows incomplete, ties for best.
- **conflicts:** U23 (adjacent `render_top_record`). GAP13-13's alternative fix would touch `results_at`; U3 does not take that alternative.

## U36 fix/cloud-AC-whp-o1-2
- **findings:** AC-whp-o1-2 (low cost)
- **files:** `crates/cli/src/lib.rs` 17390-17460 `bootstrap_family` (`trade::walk` per candidate at about 17443); `runner/src/trade.rs` 321 `walk`, 549 `walk_over`
- **kind:** cost-fix
- **dnum:** none (the existing hoist pattern; the fifth site of a recorded defect)
- **verified:** yes. Up to 16 calls of `trade::walk` each rebuild `SliceFacts::of(bars, column)` over the execution slice.
- **goal:**
  - Build `SliceFacts::of` once before the map, and call `trade::walk_over(.., &facts)`.
  - **Failing-first test:** a counter seam on `SliceFacts::of`, or a source-shape test in the style of the existing "FOURTH site" guard. Assert a single construction in `bootstrap_family`. Bootstrap output is byte-identical.
  - **Edge cases:** an empty `by_evidence`, fewer than 16 candidates.
- **conflicts:** none (U1 adds runner API but this uses existing `walk_over`).

## U37 fix/cloud-W2-cli9-0
- **findings:** W2-cli9-0 (low cost)
- **files:**
  - `crates/cli/src/pool.rs` 706-760 `union_of` (`of_run` per instrument at 728), header 84-88 ("Nothing here scans the store per candidate")
  - `crates/cli/src/frontier.rs` 842 (`require_parent`), 1358-1365 `of_run` (calls `committed_receipt` each time), 1380 `of_run_against_receipt`
  - `crates/cli/src/result_set.rs` 728 `committed_receipt`, 738-760 `CommittedParents`
- **kind:** cost-fix
- **dnum:** D-0992
- **verified:** yes. Every `of_run` call reopens and scans the results ledger plus the receipts (O(L+R)), once per instrument.
- **goal:**
  - Open `CommittedParents::open_read_bounded` once, look up each instrument's receipt from its index (O(1) expected), and call `of_run_against_receipt`.
  - Cost becomes O(L+R) once plus O(rows) per instrument. Fix the header text.
  - **Failing-first test:** a counter seam on `committed_receipt_with_limit` (or on ledger opens). One call per `union_of` over 5 instruments; today there are 5. The union is unchanged.
  - **Edge cases:** a receipt missing for one instrument (named in `unread`), the ledger changing mid-walk (`CommittedParents` refresh refuses).
- **conflicts:** U3 and U5 (pool.rs header lines 80-96).

## U38 fix/cloud-W2-cli13-0
- **findings:** W2-cli13-0 (low cost, contradicted doc), W2-cli11-2 (low cost), W2-cli11-3 (low cost). The same defect class: a lookup or page re-hashes the whole ledger file through its generation check, while the docs say O(1) or O(P).
- **files:**
  - `crates/cli/src/pre_admission_data.rs` 1222-1260 `page` (`require_unchanged` at 1241, `require_generation` at 1248-1249), 1428-1431 `require_unchanged`, 3611-3700 `file_generation` (`hash_file` of the whole file), 3701 `require_generation`
  - `crates/cli/src/population_finalization_v2.rs` 1444-1466 `reopen_structural_receipt`, 1852-1860 `require_unchanged`, rustdoc 1174-1182 and 1431-1438
  - `crates/cli/src/population_observations_v1.rs` 2209-2215 and 3390-3396 `reopen_audit` V1/V2, 2314-2325 `require_unchanged` (reads and blake3s the whole file)
  - `docs/06-limits.md`: "A page costs O(P)" at 7452 and 7696, §157 at 7848 (lines about 7826-7828 per the finding), §161 at 7978
- **kind:** cost-doc (doc-false corrections). A cost-fix is possible but is a design choice.
- **dnum:** D-0993
- **verified:** yes. All three generation checks content-hash the complete file per page or lookup, and two docs claim O(P) or average-O(1) lookup.
- **goal:**
  - Decision: either (a) keep the content hash (stronger ABA detection) and correct every doc to O(file bytes) per page or lookup, or (b) use metadata generation per page with a content hash only at open, which makes it O(P) or O(1).
  - Recommend (a) for finalization and observations, which are audit-only and seldom read. For pre-admission paging, recommend (b) with a documented ABA limit, because paging R rows is O(R²/256) today.
  - **Failing-first test:**
    - (b): a counter seam on `hash_file` shows 1 call across 10 pages; today it is 20.
    - (a): a doc-pinning test that the §157 and §161 text names O(file bytes).
  - **Edge cases:** a file replaced with the same length and mtime between pages (state what is and is not detected).
- **conflicts:** U13 (pre_admission_data.rs `open_file` at 3601, adjacent).

## U39 fix/cloud-R9-csr-cx-4
- **findings:** R9-csr-cx-4 (low bug, stale doc)
- **files:**
  - `crates/indicators/src/lib.rs` 83 and 124 ("trades Monday to Friday"), 168 ("NSE does not trade them"), test comment around 2011
  - `crates/vocab/src/table.rs` 1145
- **kind:** doc-false
- **dnum:** none (aligns comments with D-0694 and charter §3)
- **verified:** yes. Every line is still present and contradicts the same doc block and charter §3 (six weekend sessions, 1,710 bars).
- **goal:**
  - Reword to: NSE ordinarily trades Monday to Friday; six charter-recorded weekend sessions exist; weekday bits are five and a weekend bar sets none of them by design (cite D-0694).
  - Keep the behaviour.
  - **Failing-first test:** a doc-consistency test grepping indicators/src/lib.rs and vocab/src/table.rs for "does not trade"/"NSE trades Monday to Friday" (fails today).
  - **Edge cases:** none.
- **conflicts:** no lane-3 `fix/cloud-*` branch exists. Watch `vocab/src/table.rs`: a bit-table edit elsewhere would collide.

## U40 fix/cloud-GAP4-48
- **findings:** GAP4-48 (low doc-false)
- **files:** `crates/cli/src/stored.rs` 2934-2942 (comment above `if day >= last_signal_day`); `indicators/src/anchored.rs` 440-446 `advance_before`
- **kind:** doc-false (plus test-gap)
- **dnum:** none
- **verified:** yes. The filter drops only days ≥ the last signal day. Earlier same-day records are offered, and the evaluator's `advance_before` excludes them.
- **goal:**
  - Reword: the filter bounds the offered set to records some signal day can consume, so `remaining()` is 0. Per-row causality is `advance_before`.
  - **Test:** `daily_context_from_span` over a multi-day span yields `census.remaining() == 0` after the full build, and its rows equal a prefix build.
- **conflicts:** U20 (stored.rs 3022+, separate hunk).

## U41 fix/cloud-R9-cli-law-1
- **findings:** R9-cli-law-1 (low bug, false crate-graph claim)
- **files:**
  - `crates/cli/src/lib.rs` 1008, 3378-3390, 3947, 6002, 6712, 15421, and tests 20985, 21038, 22521
  - `crates/cli/src/batch.rs` 608-618
  - All are `#[expect(clippy::default_trait_access, reason = "...arrow §5 does not draw")]` plus comment.
- **kind:** doc-false (lint suppression removal)
- **dnum:** none (CLAUDE.md §5 and D-0683 already draw `vocab <-- cli`)
- **verified:** yes. `crates/cli/Cargo.toml` declares `vocab`, and production code already names `vocab::ConditionMask::default()`.
- **goal:**
  - Replace `Default::default()` with `vocab::ConditionMask::default()`, and delete the false comments and the `#[expect]`s.
  - **Test:** clippy `-D warnings` stays green with the expects removed (that is the proof), plus a source test that no `reason = "the named path would add a dependency arrow` remains in crates/cli.
- **conflicts:** U10 (lib.rs 3378 vs 3535, same function), U3 (batch.rs `one`). Small hunks; land early.
