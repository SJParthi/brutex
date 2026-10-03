Verdict: 1 finding (0 high, 1 medium, 0 low). In-process concurrency in crates/telemetry is sound. The one real defect is that two PROCESSES can share one sink directory and nothing stops them.

Commit audited: 331b05c. Slice: crates/telemetry/src (sink.rs, tail.rs, lib.rs; record/encode/clock read where the slice calls them). Method: source reading only. No cargo was run, so no measurement is claimed.

## telemetry-1 (medium): two `cli` processes append to and rotate the same `<store>/logs/cli` set with no lock. Sequence numbers fork, `reserve_run_id` hands out ids already used, `Tail::missing` hides real loss, and `ms` order stops being file order

**Where**
- crates/cli/src/lib.rs:2996: `.or_else(|| store.map(|s| s.join("logs").join("cli")))`. Every cli process resolves the same directory. `cli/src/main.rs:69` calls `cli::install_log()` for EVERY verb, before dispatch.
- crates/cli/src/lib.rs:2972-2976 (doc) claims *"each writer owning one means each owns its own `events.ndjson` ... No lock, no coordination, nothing to get wrong under concurrency"*. That holds for api versus cli. It does not hold for cli versus cli.
- crates/cli/src/lib.rs:2101: only `is_sweep_command` verbs take `execution_lease::Lease::acquire`. Non-sweep verbs (report, verify, pool, ...) run alongside a sweep and alongside each other, each with its own sink on the same files.
- crates/telemetry/src/lib.rs (install doc): *"two sinks on one path would each keep their own byte count, so each would roll the other's file out from under it"*. The guard is a per-process `OnceLock` only. `Sink::open` (sink.rs:732) takes no file lock and does not detect another live writer.
- sink.rs:746 `let (seq, last_at) = resume_point(&config.dir, config.keep_files);`, sink.rs:788 `reserved_run: AtomicU64::new(seq)`, sink.rs:1128-1129 (`stamp` and `seq += 1` against per-process state only).
- tail.rs:361 `let span = newest.checked_sub(oldest)?.checked_add(1)?;` and tail.rs:625 `.is_some_and(|floor| record.at_unix_millis < floor)` (`since` ends the walk on the first older record).

**Why it is wrong.** Each process keeps its own `seq`, `last_at` (ms clamp) and `bytes`, and all of them append through O_APPEND to one `events.ndjson`. The crate's invariants are "seq is consecutive in file order", "ms is non-decreasing in file order" and "a reserved run id is above every id in the log". All three are enforced only inside one process's mutex.

**Repro 1: run-id reuse across commands that never overlap (concrete, needs no tight timing).**
1. P_A = `cli sweep-all ...` (long). It opens the sink when the last line is seq S, so A's counter is at S.
2. While A is quiet, P_B1 = `cli report ...` opens. It resumes at S and reserves run id S+1 (sink.rs:1032; cli/lib.rs:2137). It writes "command started"/"command finished" with seq S+1, S+2 and run=S+1, then exits.
3. P_B2 = `cli verify ...` opens, resumes at S+2, reserves run id **S+3**, writes seq S+3, S+4 with run=S+3, and exits.
4. A emits its next event with its own counter: seq **S+1**. The newest line in the file now carries S+1. A emits again: seq S+2.
5. P_E = any non-sweep verb opens. `resume_point` reads the last decodable line (S+2), so `reserved_run` = S+2 and E reserves run id **S+3**, the id B2 already used.

Result: `/logs?run=S+3` (and `Query::from_run`) returns B2's and E's lifecycle events as one story. This breaks the guarantee written at sink.rs:620-629 (*"makes the next process start strictly above every id still present in the log"*).

**Repro 2: `Tail::missing` masks a real drop.** With A and B interleaved as above, the file holds seq S+1(B), S+2(B), S+1(A), S+2(A), S+3(A), and so on. An unfiltered `tail` gets more records than `newest - oldest + 1`, so `span.saturating_sub(len)` returns 0. A genuine hole, for example a `Dropped` event that burned a seq in either writer, is then reported as `Some(0)`. When the newest line comes from the lagging writer (A's S+2 after B's S+4), `checked_sub` underflows and `missing` is `None`, which callers cannot tell apart from "a filter was applied". The field the crate calls "the one number that reports a loss by the WRITER" is therefore wrong whenever two cli processes have overlapped.

**Repro 3: `since` truncation.** A runs `stamp(now_millis())` and gets t under A's mutex. B runs `stamp` and gets t+1 under B's mutex, and B's `write` lands first. File order is now B(t+1) then A(t). `Query::since(t+1)` walks backwards, meets A(t) first, `take_line` returns `true` at tail.rs:625, and the walk ends without B's record. The comment at tail.rs:604-612 says this early exit is sound only because one writer's clamp orders the file. Two writers break that.

**Repro 4: rotation (needs 8 MiB of cli events, rarer).** B's `roll` renames `events.ndjson` to `.1` (sink.rs:1284) and reopens. A's descriptor still points at the renamed inode, so A keeps appending NEWER events into `.1`. The tail reader assumes `.1` is entirely older than the current file, so the `since` early exit and newest-first order both break across the boundary. A later roll by either process shifts the other's live file further down the set. After `keep_files - 1` rolls by the busy writer, `remove_if_present(oldest)` unlinks the inode the quiet writer is still appending to. The quiet writer's `emit` then returns `Written` and counts `written += 1` for bytes that no path will ever show.

A side effect at open: if B opens while A's 41 KB line is half-visible, `ends_mid_line` (sink.rs ~1423) reports a torn tail that is not torn. B then spends its one stderr notice and sets `last_error` to "a record torn by a kill or a power cut" for a healthy file.

**Minimal fix.** Make the sink directory single-writer, in the place the crate already says the rule belongs:
- In `Sink::open`, open `dir/events.lock` and call `File::try_lock()` (std, stable since 1.89; the toolchain is 1.97.1). Hold the handle in `Sink` for its whole life. On `WouldBlock`, return `Err("<dir> is already being written by another process")`. The kernel releases the lock on exit or crash, so no stale lock file can block forever.
- In `cli::install_log`, when that refusal comes back, fall back LOUDLY to a per-process child `logs/cli/<pid>` (or refuse and say so), and have `api::logs` walk the children.

Either way `seq`, `last_at`, `bytes` and `reserved_run` each go back to having exactly one owner per set.

## Checked and found sound (no finding)

- **Level floors.** One packed `AtomicU16` (sink.rs `floors`). A relaxed single-word store and load cannot be observed crossed, and nothing else is published through it.
- **`claim_run`/`release_run`.** AcqRel CAS: two claimants cannot both win, and a loser cannot clear someone else's key. `set_run` has no production caller (already noted in hunt-conc.md).
- **`reserve_run_id`.** `fetch_update` is correct within one process. The cross-process failure is telemetry-1.
- **Poisoned `inner`/`last_error` mutexes.** Recovered with `into_inner`. A panic inside `line()` leaves only a burned seq (visible as a hole) and a buffer that is cleared on the next emit.
- **No lock held across stderr.** `report` runs after `drop(guard)` on every path, including the roll-failure path. `Debug` takes no lock. `health` takes the two locks one after the other, never nested, so there is no lock-order cycle.
- **`ms` monotonicity within one process.** The clock is read inside the lock and clamped. The clamp is resumed from disk at open (D-1325).
- **Rotation crash points.** A kill after unlinking the oldest file, after any middle rename, or after `current -> .1` but before `reopen` all restart cleanly. Open recreates `events.ndjson`, and `resume_point` takes the first non-empty file newest-first. No fsync of file or directory is intended (stated in the module doc), so a power cut losing a rename loses only stream history.
- **The failed-roll storm.** Stopped by `rotation_broken`, which is set and read under the inner lock.
- **A partial `write_all` on ENOSPC.** Terminated with `\n` before the lock is released, so the next event cannot fuse onto it.
- **Install race inside one process.** `OnceLock::get_or_init` plus the path comparison. The losing `Sink` is dropped. At worst its open wrote one extra torn-tail `\n`, which the reader skips as a blank line.
- **Tail reader against a roll mid-walk.** One `open` plus one `fstat` per file (no path-then-open TOCTOU). Dedup by `(dev, ino)`. A roll between files makes `.1` the already-read inode, which is skipped, and `.2` is then correct. A reader descriptor survives unlink. A writer caught mid-line is handled by `partial_tail`. Inode reuse and out-of-order records would need two 8 MiB rolls during one millisecond-scale walk, which is not a concrete failure path.
- **`last_record`/`ends_mid_line` at open.** Read-only, run once, and bounded at 64 KiB. They become wrong only with another live writer (telemetry-1).
