# BASE-COV-LIBS progress (append-only)

## 2026-10-27 18:32:40 IST attempt 1 start
DRAIN CHECK printed OPEN. No earlier progress, fix or verify file; no P-BASE-COV-LIBS worktree. Created wt/P-BASE-COV-LIBS on fix/db-BASE-COV-LIBS at f01d7a0e430e8f9b01e424a299b0bed20738a0ef (feat/db-fix tip). Rules audit has no BASE-COV-LIBS section (grep '^## .*BASE-COV-LIBS' empty). Set file (SET OVERRIDE, whole crates): /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-LIBS-f01d7a0e430e-set.txt (313 files).
NEXT: measure Route B over the ten crates in scratch/deep-BASE-COV-LIBS.

## 2026-10-27 18:35:43 IST D3 BASE-COV-LIBS tip f01d7a0e430e8f9b01e424a299b0bed20738a0ef set state/fix/round2/deep-BASE-COV-LIBS-f01d7a0e430e-set.txt
Lock scratch/deep-BASE-COV-LIBS.lock taken; tree scratch/deep-BASE-COV-LIBS made at the tip. tgt/deep-1.lock was untouched since 17:40:28 and still at 18:35:28 (150 s rule): moved to deep-1.lock.dead-1793106332 and taken. Route B started: PID 46227, log scratch/deep-BASE-COV-LIBS-logs/f01d7a0e430e-routeb-a1.log, profile scratch/deep-BASE-COV-LIBS-prof/f01d7a0e430e-1-B, show logs scratch/deep-BASE-COV-LIBS-logs/f01d7a0e430e-cov.
NEXT: wait for EXIT, then build the gaps table base-cov-libs-gaps-a1.txt.

## 2026-10-27 19:28:09 IST interim
Route B a1 interim (six crates measured, runner/store/telemetry/vocab still queued: the deep-1 and pool-5 targets get only the queue's 3 low slots, measured waits 20-40 min per cargo command): engine lines 5154/5195 branches 508/572; indicators 16275/16357, 1894/2178; costs 11148/11366, 481/514; lake 4795/5452, 252/342; core 7302/7375, 582/658; greeks 1918/1921, 93/96. NOT PROVED by the route: every benches/ file (Cargo.toml test = false, so cargo test --no-run builds none: 13 files, 7,928 lines) and lib.rs files with no executable line (lake/src/lib.rs: only mod declarations).
Edits so far (WIP c61f98d92 + uncommitted): costs lot.rs (D-7321: from_rows and compiled_rows rebuilt without their unreachable let-else guards), lot_table_tests.rs (extremes test), ledger.rs (witness returns the type name + test), regime.rs, dated.rs, moneyness.rs (dead tie clause in a test oracle), trip.rs (pinned the measured Overflow), stack.rs (to_rupee extremes: measured that it never leaves i64, so its narrowing refusal is (U)), tests/adversarial.rs (refusal column was dead: every cell must price); telemetry clock.rs (D-7322: now_millis routes through millis_since_epoch, both sides tested); engine lib.rs (manifest string-grammar edges); indicators tests/exact_minute_orb_gap_readiness.rs (pre-read refusals + short signal).
First costs run: 1 FAILED (my to_rupee assertion was wrong: i64::MAX ends in 07 and rounds down) - corrected. Iteration 2 (costs+telemetry) PID 14547 log scratch/deep-BASE-COV-LIBS-logs/iter2.log.
NEXT: read iter2; wait Route B EXIT; write base-cov-libs-gaps-a1.txt; continue store (most changed) then telemetry gate 20.

## 2026-10-27 20:34:03 IST runner measured
Route B a1: runner tests 797 passed, 0 failed (99 min incl. queue); runner lines 44019/45694 (uncovered 1675), branches 3156/4188 (uncovered 1032). With the six crates above, uncovered lines already exceed 2,000 (about 2,750 before runner + 1,675 runner) with store, telemetry and vocab still to read: OVER the 2,000-line mark the spec names; said in notes.
WIP commits: c61f98d92, 3bc843ebf, 55b107d12, c1761a617, 667137e99 (D-7320..D-7323 entries, gate 20 clock.rs 1->0). Iterations: iter2 (costs+telemetry) all ok; iter3b (core+engine+indicators) only decision_citations failed (D-732x cited before their entries; entries now written); iter4 (core+indicators) running.
NEXT: iter4 result; store/telemetry/vocab figures when Route B ends; gaps file a1; store most-changed files.

## 2026-10-27 20:59:28 IST iterations green
iter4 (core+indicators): 1 failure in my new column assertion (acceptance length is per OFFERED bar, not per kept row) - fixed; iter5 (indicators+lake): 29 result lines, all ok, EXIT 0 (scratch/deep-BASE-COV-LIBS-logs/iter5.log). WIP 1838dddcc.
NEXT: Route B store/telemetry/vocab; gaps a1 + fu files; final merge + gates; targeted re-measure of touched crates.

## 2026-10-27 21:04:20 IST D3 BASE-COV-LIBS tip f01d7a0e430e8f9b01e424a299b0bed20738a0ef set state/fix/round2/deep-BASE-COV-LIBS-f01d7a0e430e-set.txt: MEASURE FIRST done
Route B PID 46227 EXIT 3 (ROUTE-B DONE 21:03:46 WITH NOT PROVED lines: 13 benches/ files, test = false; 6 lib.rs with no executable line). Wall 18:35:37-21:03:46 (148 min; runner 99 min incl. queue, store 35 min).
TOTALS (before): ALL lines 155497/160279 (uncovered 4782), branches 11912/14382 (uncovered 2470), 294 files read. Per crate: core 73/76, costs 218/33, engine 41/64, greeks 3/3, indicators 82/284, lake 657/90, runner 1675/1032, store 1861/676, telemetry 67/50, vocab 105/162 (uncovered lines/branches). OVER 2,000 uncovered lines (4,782): said in notes (D6 undecided).
First-pass classification: 1,572 (F) candidates (failing assert/panic arms inside test code), 5,680 GAP rows (lines + branch sides). Files: state/fix/round2/base-cov-libs-gaps-a1.txt (7,566 lines), state/fix/round2/base-cov-libs-fu.txt.
NEXT: docs/05 D-7320 figures + docs/06 L-BCL rows; commit; merge feat/db-fix; gates on touched crates; targeted Route B re-measure of touched crates; mutants list of own diff.

## 2026-10-27 21:07:52 IST red checks
Hand changes (then restored with git checkout of my own committed files): LotStanding::parse 'sourced'->'source'; millis_since_epoch pre-epoch arm true->false; parse_month numbering 1_u8.. -> 0_u8..; QuarantineError::Full sentence changed. cargo test -p costs -p telemetry -p lake -p store --lib -- <the four new tests>: all 4 FAILED (log scratch/deep-BASE-COV-LIBS-logs/red2.log); restored, tree clean. iter6 (store quarantine filter 34 ok; telemetry keep_nothing 1 ok).
NEXT: final gates (fmt, clippy touched, tests touched) under pool-5 lock; targeted Route B re-measure; mutants list.

## 2026-10-27 21:08:16 IST D3 BASE-COV-LIBS tip 6d03e350ec45 (branch head, feat/db-fix unchanged at f01d7a0e4) set state/fix/round2/deep-BASE-COV-LIBS-6d03e350ec45-set.txt
Final gates PID 38419 log scratch/deep-BASE-COV-LIBS-logs/gates-final.log (fmt, clippy -p core costs engine indicators lake store telemetry --all-targets -D warnings, test same --locked --no-fail-fast).
Targeted Route B after-measure PID 38676 log scratch/deep-BASE-COV-LIBS-logs/6d03e350ec45-routeb-after.log, profile scratch/deep-BASE-COV-LIBS-prof/6d03e350ec45-1-B, packages core costs engine indicators lake store telemetry (greeks, runner, vocab untouched: their before figures stand).
NEXT: wait both; mutants list of own diff; report.

## 2026-10-27 21:10:18 IST D3 mutants tip 6d03e350ec45
D3 MUTSET: selected 37 of 296 listed mutants (from_rows, compiled_rows, witness, now_millis, millis_since_epoch, parse_month); D3 MUTANTS LIST OK 21:09:18 (scratch/deep-BASE-COV-LIBS-mut/6d03e350ec45-1). D3 SHARD PLAN OK: 2 shards (19, 18). Lane tree scratch/deep-BASE-COV-LIBS-m2 at the tip (S itself is busy with the after-measure). Lane 1 of 1 PID 41485 log scratch/deep-BASE-COV-LIBS-logs/mut-lane1.log, target tgt/deep-1/mut-1.
NEXT: wait gates, after-measure, mutants; write report.

## 2026-10-27 21:18:43 IST D3 BASE-COV-LIBS tip 5ad554601835 set state/fix/round2/deep-BASE-COV-LIBS-5ad554601835-set.txt
Mutants at 6d03e350ec45 (lane 1/1, 2 shards): shard 0: caught 17, unviable 1, MISSED 1 (lot.rs:907:23 > -> >= in from_rows), baseline build 5.6 s test 2.8 s, per-mutant p50 0.5 s; shard 1: caught 16, unviable 1, MISSED 1 (lot.rs:1070:42 == -> != in compiled_rows: the sha choice). Both killed by lot::table_tests::each_sourced_row_carries_its_own_circulars_digest_and_the_row_cap_is_exact (WIP 5ad554601).
The first after-measure at 6d03e350 was cut by my own kill of the gate run (same package list matched): build rc=143; not counted. Gate runs 1-3 found fmt and clippy items in the new tests (fixed).
Now: gates PID 50142 (gates-final4.log); Route B after-measure PID 50327 (5ad554601835-routeb-after.log, P scratch/deep-BASE-COV-LIBS-prof/5ad554601835-1-B); mutants MUTSET 37 of 296, LIST OK, PLAN OK 2 shards, lane PID 51747 (mut-lane1-5ad554601835.log) in scratch/deep-BASE-COV-LIBS-m2, target tgt/deep-1/mut-1.
NEXT: wait all three; report.

## 2026-10-27 21:27:38 IST D3 mutants tip 5ad554601835 DONE
shard 0/2 lane 1/1: caught 18, unviable 1, missed 0, timeout 0; per-mutant test p50 0.5 s, p99 0.6 s, max 0.6 s, n 18. shard 1/2 lane 1/1: caught 17, unviable 1, missed 0, timeout 0; p50 0.5 s, p99 8.2 s, max 8.2 s, n 17. LANE DONE 21:27:15. Folder scratch/deep-BASE-COV-LIBS-mut/5ad554601835-1.
NEXT: gates test result; Route B after-measure.

## 2026-10-27 21:27:48 IST gates green at 5ad554601835
gates-final4.log: cargo fmt --all --check exit 0; cargo clippy -p core -p costs -p engine -p indicators -p lake -p store -p telemetry --all-targets --locked -- -D warnings exit 0; cargo test -p (same) --locked --no-fail-fast exit 0: passed 2599, failed 0, ignored 17 over 97 result lines (21:17:29-21:22:20). pool-5 lock released.
NEXT: Route B after-measure (PID 50327); then release deep-1 and scratch/deep-BASE-COV-LIBS locks; report.

## 2026-10-27 21:51:38 IST attempt 1 ends (report written)
After-measure Route B PID 50327 EXIT 3 (NOT PROVED = benches + no-code lib.rs only), tip 5ad554601835, 21:08-21:50: core 56/75, costs 175/22, engine 40/56, indicators 42/271, lake 637/93, store 1820/673, telemetry 51/52 uncovered lines/branch sides; untouched greeks 3/3, runner 1675/1032, vocab 105/162. ALL ten: 4604 / 2439 (from 4782 / 2470). After file: state/fix/round2/base-cov-libs-gaps-a1-after.txt (1184 F?, 2879 GAP rows for the seven crates).
Final commit c8abf1668 (normal; docs-only after the measured tip 5ad554601835, so the measured .rs content is the head's). Locks released: pool-5, deep-1, scratch/deep-BASE-COV-LIBS. Lane tree scratch/deep-BASE-COV-LIBS-m2 left in place (detached at 5ad554601835).
NOT DONE: the bar. Shadow-server check (step 5) not run: no route's code path changed beyond behaviour-preserving restructures proven by unit tests and mutants; NOT PROVED on a server.
NEXT (attempt 2): read base-cov-libs-gaps-a1-after.txt; close GAP rows file by file (store quarantine.rs/file.rs/settings.rs/repair.rs first, then runner admission/exit_grid_policy/grid/validate/report, lake page/reader and its ignored real-lake tests per L-BCL-3); review every (F) row by hand; re-measure with Route B; operator: D6 split, L-BCL-1 benches, L-BCL-3 ignored tests.

## 2026-10-28 08:15:00 IST attempt 2 start
DRAIN CHECK printed OPEN. Worktree reused at c8abf1668, clean (web/node_modules link only). feat/db-fix still at f01d7a0e4 (no new merges, no fold B). No verify report exists. Continuing from attempt 1 NEXT: close GAP rows from base-cov-libs-gaps-a1-after.txt.
NEXT: pick files (store quarantine/file/settings/repair, runner, lake), write tests.

## 2026-10-28 08:35:20 IST quarantine
D-7324 (store quarantine.rs): write_at, one bounded write for every slot/header/image field (encode_slot, empty_image, seal_header, put, set_described); field() uses checked_add; note_damaged/note_cut take their figures (the if-let arms no caller could miss are gone). New crates/store/src/quarantine_cover_tests.rs, 16 tests. cargo test -p store --lib -- quarantine:: : test result: ok. 51 passed; 0 failed (log scratch/deep-BASE-COV-LIBS-logs/a2-q2.log). WIP 1ef870bfd. pool-5 lock held by me since 08:17.
NEXT: store file.rs, sort_index.rs, settings.rs, repair.rs gaps.

## 2026-10-28 09:06:32 IST store batch 2 (uncompiled yet)
Written: sort_index_cover_tests.rs (D-7325 zip loops, bounded take/put), settings_cover_tests.rs, repair_cover_tests.rs, month_summary_cover_tests.rs, generation_cover_tests.rs (put_field), tests/base_cov_words.rs; file.rs write_sealed/write_sealed_refused take their fields; runner tests/base_cov_admission_words.rs. sort_index+file tests passed before the later files (a2-s1.log: ok. 151 passed). WIP 42737ba18. Build queue: pool-5 gets 3 low slots, waits 20+ min (load 19-28).
NEXT: a2-s2.log result (cover_tests + sort_index); fix failures; then catalog/file_sealed/flock/provenance; runner.

## 2026-10-29 09:39:57 IST attempt 3 start
DRAIN CHECK printed OPEN. feat/db-fix still at f01d7a0e4 (no new merge, no fold). Found attempt 2's uncommitted edits (catalog slice pattern, pbo relative D-7326 cited without entry, significance and catalog cover tests, month_summary test fix for the a2-s2 failure, runner words): reviewed, D-7326 entry written, committed WIP 036570d80.
NEXT: compile+run the new store/runner tests; then continue gaps (store, runner, lake) from base-cov-libs-gaps-a1-after.txt.

## 2026-10-29 09:47:18 IST runner admission readers and statistics
a3-t1.log: store cover_tests+catalog ok 58 passed; runner cover_tests ok 21, base_cov_admission_words ok 4 (EXIT 0). New: crates/runner/src/admission_reader_cover_tests.rs (V1/V2/V3 canonical readers: every take/tag/observed/reserved/header arm by direct call; verdict decode and reconciliation; rate overflow; exact probability), admission_statistics_cover_tests.rs (Statistics V2/V3 verify_detached every refusal + accessors; walk-forward V2/V3 from_canonical_fields every rule). file.rs Display: allow too_many_lines (attempt 2's arms grew it to 116). a3-t3: reader tests ok 17 passed; clippy fixes made. WIP 844fccb09, a05196d04, 2271ce812.
NEXT: a3-t4 result (runner cover tests + clippy runner store); then remaining admission gaps 4100-4700, 5000-7200; lake reader.rs; then final merge, gates, Route B re-measure of runner+store, mutants on own diff.

## 2026-10-29 09:50:30 IST evidence joins, exit-grid constructors
a3-t5.log: runner cover tests + evidence join test ok 25 passed; clippy -p runner -p store --all-targets -D warnings RC 0. Added evidence_joins_refuse_a_prefilled_authority_slot_and_a_foreign_trade_count (inside admission mod tests, uses its fixtures), exit_grid_policy_cover_tests.rs (RungPlanV1/RatioLimitsV1/ExitGridPolicyV1 refusals + accessors). WIP 360bffe84, c945062d7. a3-t6 running.
NEXT: a3-t6; then lake reader.rs gaps; then final merge feat/db-fix + gates + Route B re-measure (runner store) + mutants of own diff.

## 2026-10-29 10:24:54 IST red check queued; lake, grid
a3-t6: policy_cover_tests ok 4 passed, clippy -p runner RC 0. RED CHECK in progress (a3-red1.log, queued 25+ min, load 43): hand changes UNCOMMITTED in admission.rs (read_bool 1=>false; V2 bootstrap_periods <2 -> <1; V3 walk decided_folds >0 -> >1) and exit_grid_policy.rs (ZeroLimit max_ratio_pairs -> max_pairs); restore = git checkout -- crates/runner/src/admission.rs crates/runner/src/exit_grid_policy.rs (both committed at c945062d7). New WIP 9c9790fcf: lake reader_cover_tests.rs (read_bounded Io, /dev/zero TooLarge, exact cap), runner grid_cover_tests.rs (written, mod line not yet added to grid.rs).
NEXT: read red result, restore the two files, wire grid_cover_tests into grid.rs, run lake+runner tests; then final gates and Route B (runner store lake) + mutants with set from scratchpad mkset.mjs.

## 2026-10-29 11:10:50 IST red check 1 read
a3-red1.log (queued 09:52-11:08, load 18-43): with the four hand changes, 3 of 4 tests FAILED as they must (exit_grid_policy policy_cover_tests ratio_limits..., admission statistics_cover_tests a_v2..., reader_cover_tests a_v1_bool...); walk_v3 canonical_fields... PASSED under decided_folds > 1: a real test gap: no case had exactly one decided fold. Restored both files (git checkout), added case (1, 1, 0), WIP a64a6a140; grid_cover_tests wired. a3-red2.sh running: the same mutation re-applied UNCOMMITTED, cargo test -p runner -p lake --lib -- walk_v3:: grid::cover_tests reader_cover_tests, then the script itself restores admission.rs.
NEXT: read a3-red2 (walk_v3 must FAIL, grid and lake cover tests pass); then gates; then Route B runner+store+lake at the tip + mutants (set from scratchpad mkset.mjs).

## 2026-10-29 11:29:50 IST D3 BASE-COV-LIBS tip 26018ed261b7e66019503f0ed4dadf10ada2ed00 set state/fix/round2/deep-BASE-COV-LIBS-26018ed261b7-set.txt
a3-red2.log: walk_v3 canonical_fields... FAILED under decided_folds > 1 (as it must), grid cover tests + lake reader_cover_tests ok; script restored admission.rs (0 dirty). cargo fmt --all applied (WIP 26018ed26). Gates a3-gates.log running (fmt check of that run was RC 1 BEFORE the fmt commit; clippy and test of the same run see the formatted tree). Set file written by scratchpad mkset.mjs (coverage 328 files whole; mutants 224 items over 51 files). Tree scratch/deep-BASE-COV-LIBS lock taken, checked out at the tip. D3 MUTSET: selected 173 of 7268 listed; D3 MUTANTS LIST OK 11:29:30 (M scratch/deep-BASE-COV-LIBS-mut/26018ed261b7-1): lot 27, ledger 2, contract 2, pbo 9, catalog 4, file 3, generation 3, quarantine 41, sort_index 76, clock 6. Waiting for tgt/deep-1 or deep-2 (both held, fresh) in a blocking loop (bg task).
NEXT: on a deep dir: Route B -p lake -p runner -p store (P scratch/deep-BASE-COV-LIBS-prof/26018ed261b7-1-B); mutants sample shard n0=9; fmt check re-run; gates result.

## 2026-10-29 13:53:57 IST attempt 3 ends (report written)
a3-gates.log (pool-5 lock 11:17-13:53; clippy queued 8585 s): cargo clippy -p core -p costs -p engine -p indicators -p lake -p store -p telemetry -p runner --all-targets --locked -- -D warnings RC 0; cargo test -p (same) --locked --no-fail-fast RC 0: 3483 passed, 0 failed, 17 ignored over 113 result lines. That run's fmt check RC 1 was before the fmt commit 26018ed26; a3-fmt.log: cargo fmt --all --check RC 0 (13:53). D-7327 entry WIP fd584a9f0; final normal (empty) commit ee65c9f6fcbdf2cb40f0f0ead7c941954a2b4bce. Route B of lake/runner/store at the tip and the 173-mutant D2 set (LIST OK, M scratch/deep-BASE-COV-LIBS-mut/26018ed261b7-1) NOT RUN: tgt/deep-1 and deep-2 held by other steps 11:17-13:53 (my waiter stopped 13:42, it never held either); tree lock scratch/deep-BASE-COV-LIBS.lock released (tree left at 26018ed26). pool-5 lock: touched twice by hand (12:41, 13:42) while my queued clippy waited, never by a loop.
NEXT (attempt 4): take a deep dir; Route B: d3-route-b.sh scratch/deep-BASE-COV-LIBS tgt/deep-<k> scratch/deep-BASE-COV-LIBS-prof/<tip12>-1-B scratch/deep-BASE-COV-LIBS-logs/<tip12>-cov <set> lake runner store; mutants: d3-mutants.sh plan/lane on M (173 mutants; sample n0=9 first); regenerate gaps with classify.mjs; continue GAP rows: runner exit_grid_policy replay refusals, validate.rs, report.rs, signal_candle_stop.rs; store file.rs, quarantine.rs; lake reader.rs crafted-footer refusals; operator: D6 split, L-BCL-1 benches, L-BCL-3 ignored real-data tests (~1,400 lines).

## 2026-11-02 19:02:57 IST attempt 4 start
DRAIN CHECK printed OPEN. Worktree reused at ee65c9f6f, clean (web/node_modules link only). feat/db-fix at f01d7a0e4, 27 commits on this branch not on it, 0 behind. No verify report. Continuing attempt 3's NEXT plus the 29 Oct assurance item (sink.rs crash-recovery lines).

## 2026-11-02 19:03:58 IST D3 BASE-COV-LIBS tip ee65c9f6fcbdf2cb40f0f0ead7c941954a2b4bce set /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fix/round2/deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt
Lock scratch/deep-BASE-COV-LIBS.lock taken, tree at the tip. Route B over the ten crates started in the background: wrapper /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-LIBS-logs/routeb-a4.sh (waits for any deep dir, releases it), PID 86638, log /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-LIBS-logs/ee65c9f6fcbd-routeb-a4.log, profile /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-LIBS-prof/ee65c9f6fcbd-1-B. Set: 339-ish coverage files whole + the mutants lines of 26018ed261b7 (regenerated at the final tip).
NEXT: sink.rs recovery tests (assurance 29 Oct) in a new test file; then GAP rows while the measure runs.

## 2026-11-02 19:26:30 IST sink recovery, engine refusals (uncompiled yet)
WIP 0c190f24f: crates/telemetry/src/sink_recovery_tests.rs (moved-aside refusal in four forms via where_permission_binds, seed one byte past PATH_MAX, FIFO-parked mark_coverage with the directory taken away, post with no parent, counters direct), dead arms removed: seed_state take_left, count_code, seed_aside sync, clock push_padded, tail open+fstat, keep take_root; crates/engine/src/refusal_cover_tests.rs with a cfg(test) seam in reserved (L-BCL-4); ledger witness a plain fn; secret_shape tests. a4-t1.log (telemetry) still queued.
NEXT: read a4-t1; run telemetry+engine+core+costs tests; D-7328/D-7329 entries; gate 20 counts from the fresh Route B.

## 2026-11-02 20:58:09 IST queue
a4-t1 (telemetry) queued 1 h 50 min with no slot (lottery of 5 low slots, 27 callers, this part is not on the priority file); replaced by a4-t2 (superset: --lib of telemetry engine brutex-core costs runner) PID 61310, log /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/deep-BASE-COV-LIBS-logs/a4-t2.full. WIP head 7051c2e63. pool-5 lock held since 19:07 (touched by hand). Route B wrapper still waits for a deep dir.
NEXT: read a4-t2; fix; docs; report.

## 2026-11-03 00:10:19 IST attempt 4 ends (report written)
Head bd04fb299d23dbbbcd5974085235a84e7afaa069, branch clean, feat/db-fix unchanged at f01d7a0e4 (BASE-REDS-1023 has not landed). NOTHING of this attempt was compiled or run: a4-t1 queued 19:07-20:58 and a4-t2 20:58-00:09, no slot in 5 h (5 low slots, 24-27 callers, some waiting over 4 h 40; this part is not on state/cargo-priority.txt); the Route B wrapper (PID 86638) never got a deep dir. All stopped by me (EXIT 143), locks pool-5 and scratch/deep-BASE-COV-LIBS released. cargo fmt --all --check RC 0 (parses every new file). Route B wrapper log scratch/deep-BASE-COV-LIBS-logs/ee65c9f6fcbd-routeb-a4.log; its set file round2/deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt stands for that tip only.
NEXT (attempt 5): take pool-5; cargo test -p telemetry -p engine -p brutex-core -p costs -p runner --lib --locked --no-fail-fast, fix what fails (compile errors first; likely fixture guesses: report_cover_tests is_complete/top non-empty, portfolio empty-minute last_minute_micros, Trade words count); clippy; red checks for the new tests; then Route B on the ten crates; gate 20 counts in ci.yml and docs/06 section 54 from that figure (sink.rs, tail.rs, lib.rs, record.rs all drop); (F)/(U) list; then runner validate.rs and expression_execution, store file.rs/quarantine/sort_index remainder, lake reader, indicators guards. Never write tests from the stale gaps files for store or runner admission: attempts 2 and 3 already covered many rows (this attempt found store flock/format words already tested in tests/base_cov_words.rs).
