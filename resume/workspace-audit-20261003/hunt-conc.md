# hunt-conc: concurrency and reproducibility audit at 1087e54

## Verdict

The compute path is reproducible in the cases I could measure. Every rayon site that feeds a result (`runner::rank::offer_part` par_chunks, `runner::validate` three par_iter sites, `runner::bootstrap_family_pass::count`, `cli::batch`, `cli::pool`, `cli::sweep_rungs`, the boolean family pools, and `index_stop_search_progress::schedule`) gathers results with an indexed collect, or with slot assembly by index. It then folds or hashes them in one serial loop. No f64 reduction runs in parallel. The bootstrap tallies are integers and are absorbed in task order. `engine::drain` writes support counts into fixed disjoint slices and commits them serially in candidate order. Rank ordering is a strict total order: `total_cmp` with a mask tiebreak.

One probe ran `cli::audit_run_within(12, 1_400, 50_000)` five ways: 1, 2 and 4 CPUs through `taskset`, and `RAYON_NUM_THREADS` set to 1 and to 7. It also ran twice in one process. All seven outputs were byte-identical: 791,066 bytes, sha256 `a9889219…21bd7a`.

The defects that remain are of four kinds:
- **Known:** durable ledger and evidence writes happen inside rayon workers (GAP13-13). This is not fixed at HEAD. The same pattern also exists at two boolean sites the known finding does not list.
- **Error text:** refusal messages that pick the first bad record by walking a randomly seeded `HashMap`. The same bad store can therefore give different refusal text from one process to the next.
- **Machine-dependent reports:** report lines that print the machine's worker count.
- **False doc claim:** `rungs_within_cell_budget` says a larger machine gets more cells. The arithmetic cancels the core count, so every machine gets the same budget.

I found no deadlocking lock order, no Relaxed atomic used to publish data, and no production `.lock().unwrap()`. Poisoned mutexes are mostly recovered with `into_inner` or refused by name. The one exception is the `knobs` store, which ignores a poisoned lock silently (hunt-conc-6). I did not complete an exhaustive analysis of cross-process `flock` ordering (UNVERIFIED, see below).

## Probe (Rust, deleted afterwards)

The probe was `crates/cli/tests/zz_audit_huntconc_1.rs`. It calls `cli::audit_run_within(12, 1_400, 50_000)` twice, compares the two results, and writes the text to the scratchpad. I built it with `cargo test -p cli --test zz_audit_huntconc_1 -- --nocapture`, then ran the built binary under different environments.

```
PROBE cores=4 rayon=default len=791066 same_in_process=true
PROBE cores=4 rayon=1       len=791066 same_in_process=true
PROBE cores=1 (taskset -c 0)   len=791066 same_in_process=true
PROBE cores=2 (taskset -c 0,1) len=791066 same_in_process=true
PROBE cores=4 rayon=7       len=791066 same_in_process=true
a9889219da0e06e779acbe4a232bd4bcde729a7156f083d48cded3d35a21bd7a  audit_1_t1.txt
a9889219da0e06e779acbe4a232bd4bcde729a7156f083d48cded3d35a21bd7a  audit_2_t2.txt
a9889219da0e06e779acbe4a232bd4bcde729a7156f083d48cded3d35a21bd7a  audit_4_.txt
a9889219da0e06e779acbe4a232bd4bcde729a7156f083d48cded3d35a21bd7a  audit_4_r1.txt
a9889219da0e06e779acbe4a232bd4bcde729a7156f083d48cded3d35a21bd7a  audit_4_r7.txt
```

This exercises the engine support lanes, which take their count from `available_parallelism`. It also exercises the rank chunk width, which comes from `rayon::current_num_threads`, and the validate and screen par_iters. One limit applies: this synthetic audit prints `identity NOT RECORDED`, so the probe does not exercise the run-identity terms that depend on the machine (the ceiling). The probe file was deleted, and `git status --porcelain | grep -i huntconc` printed nothing.

## Findings

| id | severity | crate | file:line | what is wrong | evidence | status |
|---|---|---|---|---|---|---|
| hunt-conc-1 | medium | cli | batch.rs:356-360 → batch.rs:631, 724; pool.rs:293-300; lib.rs:15153-15168 (`sweep_rungs`→`one_rung`) | Durable writes happen inside rayon workers: attempt-token allocation (`sweep_evidence::begin`) and run-ledger commits (`record_swept_run`). So the token each row gets, and the order of rows on disk, follow which thread finishes first. The comment at batch.rs:332-333 still says "Nothing is shared and nothing is written". | `.par_iter().map(\|held\| one(root, held, min_hits, commit))` (batch.rs:358-359). Inside `one`: `let attempt = match crate::sweep_evidence::begin(` (batch.rs:631) and `crate::record_swept_run(` (batch.rs:724). | KNOWN: fix-queue_lane1-b.md GAP13-13 (planned as U3, not landed at HEAD) |
| hunt-conc-2 | low | cli | boolean_catalog_prepared.rs:224-231; boolean_oos_command.rs:100-102; reaches boolean_candidate_v1.rs:304 | The same class as hunt-conc-1, at sites GAP13-13 does not list. Each family's `candidate::produce` runs inside a `par_iter` on a pool sized from `available_parallelism`, and it allocates its evidence attempt through the process-global attempt journal. Token assignment therefore follows the thread schedule. The rendered results are gathered in index order, so the report text itself is ordered; only the token mapping and the journal append order vary. | `self.scope.families().par_iter().map(\|family\| { ... self.produce(self.request(rung, programs, ...)) })` (boolean_catalog_prepared.rs:225-231). `crate::sweep_evidence::begin(request.output, identity, Operation::BooleanCandidates)?` (boolean_candidate_v1.rs:304). `begin_group` allocates tokens through `allocate(root, &mut starts)` (sweep_evidence.rs:534). | NEW (location); class KNOWN: GAP13-13 |
| hunt-conc-3 | low | cli | admission_store.rs:1297 (`reconcile_all`); population.rs:4644, 4670, 4706 (`reconcile_receipts`, `_v3`, `_v4`) | When the store holds more than one bad population, the refusal returned names whichever bad identity a randomly seeded `std::collections::HashMap` yields first. The same bytes on disk give a different refusal text in different processes. This conflicts with §3 rule 5, because refusal text is output. File-validation I/O also runs in that random order. | `for (population_id, receipt) in completions { ... return Err(format!("population {} has an admission receipt without its decision block", hex(population_id)))` (admission_store.rs:1297-1312). `completions` is a `HashMap::new()` (admission_store.rs:1271; `use std::collections::{HashMap, HashSet}` at :31). population.rs:4706+ follows the same pattern: `for (identity, receipt) in receipts { ... return Err(format!(... hex(identity)))`. | NEW |
| hunt-conc-4 | low | cli | rungs_within_cell_budget, lib.rs:5354-5357 (doc) vs 5407-5412 (code) | The doc is false. It says "A larger machine earns more; a smaller one is protected". The code multiplies by cores in `whole_machine_ceiling` (lib.rs:16627-16632) and then divides by `available_parallelism`, so the cores cancel and every machine gets the same budget. This happens to be good for reproducibility: the grid rung count does not depend on the core count. | `let budget = whole_machine_ceiling().checked_div(GRID_SHARE).and_then(\|share\| share.checked_div(threads))`. Computed for cores 1, 2, 3, 4, 7, 10, 13, 14, 16, 32, 64 and 128: `budget=9362` in every case. | NEW |
| hunt-conc-5 | info | cli | boolean_catalog_prepared.rs:213-221; index_stop_search.rs:212-220 | Report text prints the worker count, which comes from `available_parallelism`. The same command over the same inputs prints different bytes on machines with different core counts, or under a CPU quota. The run identity is not affected: `workers` is not folded into it (grep for workers in hash, update or identity code found nothing). | `"... {} parallel family workers. ..." , ... lanes` and `"Single-stop search {}: ... {} parallel timeframe workers."`, where `lanes` derives from `available_parallelism`. | NEW |
| hunt-conc-6 | info | cli | knobs.rs:291-293, 319-327, 359-366 | If the process-wide knob `RwLock` is ever poisoned, `set` and `clear_all` silently do nothing, and `var` silently falls back to the process environment. A request's knobs would then be ignored without a word, which is the hidden fallback §4 bans. Reaching it needs a panic while the write guard is held, and the guarded code only does `HashMap` insert, remove and clear. That makes it practically unreachable, so this is a law-shape issue rather than a live bug. | `store().read().ok()?.get(name).cloned()` and `if let Ok(mut held) = store().write() { ... }`. | NEW |
| hunt-conc-7 | info | runner | validate.rs:1671-1676 (`fold_rungs`), 1945 | The legacy `walk_forward_shaped` door reads `BRUTEX_GRID_RUNGS` straight from `std::env`, bypassing the request-local `cli::knobs` store. It would therefore disagree with a knob set over HTTP. No production caller reaches it: `cli` calls only the `_with_rungs` doors, and the remaining callers are runner tests. | `std::env::var_os("BRUTEX_GRID_RUNGS")...unwrap_or(DEFAULT_RUNGS)`. Its only non-test caller is validate.rs:1945, inside `walk_forward_shaped`. Grep found no `walk_forward(` or `walk_forward_shaped(` call from `cli`. | DOCUMENTED: the doc comment at validate.rs:1647-1652 names it a legacy door |
| hunt-conc-8 | info | cli | lib.rs:16446-16475 (`SharedBy`, `SWEEPS_SHARING_THIS_MACHINE`); pool.rs:293 | One process-global divisor that is set by a plain store and reset to 1 when the guard drops. Two overlapping guards in one process would make the first drop reset the divisor for the other. That changes the ceiling, and the ceiling is folded into the identity through `Params::ceiling`. The API admits only one engine run at a time (`ADMISSION` mutex plus the in-flight check, sweeprun.rs:1663-1700), so overlap is not reachable over HTTP. Pool pass 1 still takes no `SharedBy`. | `SWEEPS_SHARING_THIS_MACHINE.store(count.max(1), Relaxed)` / `Drop: store(1, Relaxed)`. `SharedBy::these(` appears only at batch.rs:356 and lib.rs:15153. | KNOWN (pool pass 1): fix-queue_lane1-b.md "SWEEPS_SHARING_THIS_MACHINE ... does not hold for pool pass 1"; the ceiling's dependence on cores is DOCUMENTED: docs/06-limits.md §93-94 and D-0685 |

## Verified clean (re-checked at HEAD)

| path | file:line | concern | verdict |
|---|---|---|---|
| rank: parallel scoring and merge | runner/src/rank.rs:694-711, 614-626, 120-131 | chunk width depends on `rayon::current_num_threads()` | Deterministic. Every comparator ends in `total_cmp` and then the mask, and masks are unique, so the per-chunk top-k merge is schedule-independent. Probe hash matched at rayon 1/4/7. |
| validate: fold assessment | runner/src/validate.rs:2659-2668, 4708-4767, 4986+ | par_iter, then a sequential scan | Indexed collect, then a serial hash or `>` scan. Ties go to the earliest candidate. |
| bootstrap family pass | runner/src/bootstrap_family_pass.rs:376-401 | parallel reduce | Draws come from one serial RNG stream. Tallies are integers with `checked_add`, absorbed in task order. |
| engine support lanes | engine/src/lib.rs:2358-2393 | scoped threads | Disjoint `chunks_mut` slices, committed serially in candidate order. The ceiling and pair checks run sequentially before batching, so the lane count cannot move a halt. |
| index-stop scheduler | cli/src/index_stop_search_progress.rs:162-205, 72-140 | `fetch_add` work counter (Relaxed) | Results are reassembled by index. The counter only hands out work. The bounded channel cannot deadlock: `for … in receiver` drops the receiver on `break`, so a blocked `send` returns Err. |
| telemetry run key | telemetry/src/sink.rs:944-1035 | Relaxed store of the run key | `set_run` has no production caller (grep). Overlapping runs use `claim_run`/`release_run` (AcqRel CAS). The level floors are a single packed word. |
| sweep evidence publication | cli/src/sweep_evidence.rs:577-647, 1116-1145 | AcqRel/Acquire counters | The depth digest lock is held across the append, so digest order equals file order. `finish` takes `self` by value. |
| capture slots | pull/src/capture.rs:315-319 | Relaxed `fetch_update` | Bounds a counter only; publishes no data. |
| recovery lock order | api/src/recovery.rs:553-575; recovery_control.rs:26-82; server.rs:10945-10956 | two mutexes | The order is always `site.run` then `recovery_active`. I found no path that takes them in reverse. |
| `recovery::resume` `max_by_key` over a HashMap | api/src/recovery.rs:695 | HashMap order tie | Activation sequence numbers are strictly increasing (`max+1`, recovery.rs:786-793). No tie, so the result is deterministic. |
| HashMap iteration to output | coverage.rs:442, census.rs:820-828/887-895, server.rs:7500, server.rs:32750, trades.rs:2395, master.rs:155 | order leak | Each is sorted before it is emitted. Other HashMap walks (merge.rs:325, constituents.rs:827, manifest.rs:2512) are order-independent folds. |
| screen budget calibration | cli/src/lib.rs:10055-10115, 10135 | wall-clock decides the cap | Every recording run refuses it (D-0685). |
| `finished_micros` in the run record | cli/src/lib.rs:17373 | wall-clock in the ledger | A timestamp field, not part of the identity. Byte-identity of ledger rows is not claimed. |
| serde_json maps | workspace | random key order | No `preserve_order` feature in any manifest, so `serde_json::Map` is a BTreeMap. |
| poisoned mutexes (production) | api/sweeprun.rs, telemetry/sink.rs, sweep_evidence.rs and others | `.lock().unwrap()` | Every `.lock().unwrap/expect` match sits inside a `#[cfg(test)]` module. Production code uses `into_inner` or a named refusal, except knobs (hunt-conc-6). |

## UNVERIFIED

- **Cross-process `flock` ordering.** There are about 118 blocking `lock()`/`lock_shared()` sites across cli and store. I spot-checked one: anchored_search_lineage_v4.rs:707-716 takes one lock file and then opens its members. I did not prove the full order is acyclic between processes (cli plus api).
- **hunt-conc-3 by probe.** It is proven from code plus the documented `RandomState` per-process seeding. I did not build a fixture store with two corrupt populations.
- **Ceiling and identity across machines.** `whole_machine_ceiling` scales with `available_parallelism` and is folded into the identity, so the same month gets a different RunId on a machine with a different core count. This is documented in 06-limits §93-94. I did not exercise it with a probe, because the synthetic audit records no identity.
