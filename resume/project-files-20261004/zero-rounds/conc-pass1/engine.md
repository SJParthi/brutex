Verdict: the compute path is deterministic across thread counts and holds no shared mutable state. There is 1 finding (0 high, 1 medium, 0 low): a transient OS resource refusal gets frozen into a durable checkpoint.

Slice: crates/engine, runner, indicators, vocab, costs, greeks, core. Commit 331b05c. I read the source only. I did not run cargo.

## engine-1 (medium): a transient `Breach::Workers` / `Breach::Memory` halt is checkpointed as terminal, and every rerun of the same identity replays it

**Where**
- crates/engine/src/lib.rs:2373-2383 (`drain`):
  ```rust
  std::thread::Builder::new()
      .spawn_scoped(scope, move || { ... })
      .map_err(|_| Breach::Workers)?;
  ```
- crates/engine/src/lib.rs:2210-2212 (`cannot_grow` → `Breach::Memory`): `out.try_reserve(by).is_err()`
- crates/engine/src/lib.rs, `continue_walk` (around lines 1660-1675):
  ```rust
  progress.halted = halt;
  sink.report(&current, progress.admitted, progress.pairs);
  sink.checkpoint(&current, &progress)?;
  if halt.is_some() { break; }
  ```
  The checkpoint is taken for every breach kind. It does not distinguish the budget breaches (Candidates, Pairs), which follow from the inputs, from the environmental ones (Workers, Memory).
- crates/engine/src/resume.rs:268-280 (`validate_halt`) accepts `Breach::Workers` and `Breach::Memory`. resume.rs:631-635 decodes tags 3 and 4.
- crates/engine/src/resume.rs:509-548 (`resume_checkpointed`) says *"Extinct and resource-halted checkpoints return their original outcome; a partial frontier never seeds another level."* `continue_walk` is entered with `progress.halted = Some(..)`. Its `while ... && progress.halted.is_none()` loop therefore never runs, so the saved halt is returned as the answer.
- Consumer: crates/cli/src/and_checkpoint.rs:145-215. `Journal::open(root, NAMESPACE, attempt.identity())` is keyed by the run identity, not by the attempt token. `recover()` takes the newest acknowledged boundary, and `ladder.resume_checkpointed(...)` replays it.

**Why it is wrong**

`Breach::Workers` is documented (lib.rs:677-679) as "The operating system refused to create a support-counting worker". That is a property of the moment, not of the inputs. Typical causes are EAGAIN from RLIMIT_NPROC or a cgroup `pids.max` while the api server's and rayon's threads are live, because `drain` spawns `lane_count` new OS threads on every batch. `Breach::Memory` is by its own doc "what the allocator genuinely refuses" at that instant, which includes pressure from another process.

Neither cause is a term of the run identity (CLAUDE.md §3 rule 3). Yet the halted level is published as a sealed, acknowledged boundary under that identity, and resume treats it as terminal. Rerunning the same inputs on the same machine, with the pressure gone, returns the same `WORKERS` / `MEMORY` halt and spawns no thread at all. This breaks "Reruns are safe" (§3 rule 5). A one-off resource failure becomes the permanent recorded outcome for that identity. The only escape is deleting the journal by hand, and nothing tells the operator to do that.

**Repro (exact sequence)**
1. Run `cli sweep-stored` (AND checkpoint path) on instrument X. The column is wide enough that k=3 needs more than one batch (`batch_cap = lanes × BATCH_PER_LANE`).
2. During k=3, a cgroup `pids.max` is reached because the api server is busy. `spawn_scoped` returns EAGAIN. `drain` returns `Err(Breach::Workers)`. `next_level` returns `halt = Some(Halt{k:3, breach: Workers, ..})`.
3. `continue_walk` sets `progress.halted` and calls `sink.checkpoint(..)`. `walk_within`'s callback publishes the k=3 boundary to the journal with `journal.publish`, and it is acknowledged. The run ends halted with WORKERS.
4. The load goes away. The operator reruns the identical command. The identity is the same, so the journal is the same. `recover()` returns the k=3 boundary, and `validate_for` passes because lanes are deliberately not compared. `resume_checkpointed` → `continue_walk`: the loop condition `progress.halted.is_none()` is false, so the walk returns the WORKERS halt again with no spawn attempted. Every later rerun does the same.

**Minimal fix**

Do not make an environmental halt durable. Pick one of these:
- (a) In `continue_walk`, skip `sink.checkpoint` when `halt` is `Some` with `breach ∈ {Memory, Workers}`. The last durable boundary is then the previous complete level, and a rerun resumes from k-1 and retries.
- (b) Have `Checkpoint::validate` / `resume_checkpointed` refuse, or rewind past, a checkpoint whose halt is Memory or Workers. Then `walk_within` retries from the last complete boundary.

Option (a) is the smaller change. Either way, keep the halt loud in the live run's report.

## What I checked and found sound (not findings)
- **engine `drain`** (lib.rs:2361-2400). Scoped threads write disjoint `chunks_mut` slices, and results are committed serially in candidate order only after every spawn succeeded. On a spawn failure the scope joins the threads that did start and nothing is committed; `generated` and `emitted` are rolled back by `batch.len()` (lib.rs:2044-2051). A worker panic propagates through the scope rather than being lost. The ceiling, pair-budget and prune checks run serially before batching, so the lane count cannot move a halt. `Memory`'s `grow_by = batch.len()+1` depends on the lane count, but only at the allocator's own non-deterministic boundary.
- **Lane count in identity and checkpoint.** `support_lanes` is written into the checkpoint header (resume.rs:387) but deliberately not compared on resume (resume.rs:162-186, D-1439), so a resume on a different core count works. It is not a run-identity term.
- **runner rayon sites** (rank.rs:705 `par_chunks` with an indexed collect and a serial `admit`; validate.rs:2661, 4710, 4987; bootstrap_family_pass.rs:396). All gather results by index and fold serially. The rank comparator ends in `total_cmp` and then the mask. This matches the earlier hunt-conc table; nothing has changed at this commit.
- **Shared statics.** The only non-test runtime static is `runner::research_family::membership_snapshot_digest_v1`'s `OnceLock<[u8;32]>` (research_family.rs:205). It is a pure function of const tables, so a race to initialise it is harmless. Every other `static` in the slice is const-built and immutable: core vendor.rs and universe.rs `MemberIndex`, vocab table.rs `NAME_INDEX`. Every `thread_local!` (runner bootstrap, trade, identity, validate, exit_grid_policy; greeks bsm; indicators anchored) is `#[cfg(test)]` and counts calls only. There are no `static mut`, atomics, Mutex or RwLock in production code in the slice.
- **HashMap and HashSet use** (engine resume.rs and lib.rs, runner closed.rs and rank.rs). These are used only for membership checks. Output is always sorted canonically (`sort_canonically` by mask words; closed.rs sorts before output). I found no case where hash iteration order reaches output bytes or a digest.
- **indicators, costs, greeks, core, vocab.** None of these crates has threading or shared mutable state.
- **runner auto_probe** (lib.rs:1142). It uses a `RefCell` failure slot inside a `&dyn Fn` reporter. The engine calls the reporter only from the serial level boundary, never from a lane thread.
