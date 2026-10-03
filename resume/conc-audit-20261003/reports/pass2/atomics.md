Verdict: the workspace's production atomics are mostly sound. I found 1 new finding (0 high, 0 medium, 1 low), and the one pass-1 finding about atomic ordering (autopilot-1) is still present.

Slice: workspace-wide, non-test code only. That covers every `Atomic*` use and its `Ordering`, flag-then-data publication, counters used as generations or epochs, `compare_exchange` loops, `fetch_add` overflow, and Relaxed loads that gate correctness. Commit 331b05c. I read the source only and ran no cargo.

Method: I grepped every `Atomic*`, `fetch_*`, `compare_exchange`, `swap(` and `Ordering::{Relaxed,Acquire,Release,AcqRel,SeqCst}` in `crates/**/*.rs` and dropped `*_tests.rs`, `tests/` and `#[cfg(test)] mod` bodies. That left 414 raw hit lines, and I sorted each one into production or test. Most cli and store hits are temp-root counters inside test modules, and those are not reported. `api::isolated` is `#[cfg(test)]` (lib.rs:137), and `cli::stored::CALENDAR_POLICY_DIGEST_INITIALIZATIONS_V2` is `#[cfg(test)]`.

## atomics-1 (low): the pull-run telemetry key is claimed and released per leg, not per press, so the first leg to finish clears it while sibling feeds are still running

**Where**
- crates/api/src/server.rs:7811 (inside `broker_run`, which runs once per leg): `let claimed_run = note_run_started(asked, targets.len());`
- crates/api/src/server.rs:6951-6954 (`note_run_started`):
  ```rust
  let claimed = telemetry::global().and_then(|sink| {
      let id = telemetry::now_millis().unsigned_abs();
      sink.claim_run(id).then_some(id)
  });
  ```
- crates/api/src/server.rs:7058-7060 (`note_run_finished`, reached from server.rs:7947 at the end of the same leg):
  ```rust
  if let (Some(sink), Some(run)) = (telemetry::global(), claimed) {
      sink.release_run(run);
  }
  ```
- crates/telemetry/src/sink.rs:993-996 (`claim_run`: `compare_exchange(0, run, AcqRel, Acquire)`), 1015-1018 (`release_run`: `compare_exchange(run, 0, ..)`), and 1080 (`emit` stamps `self.run.load(Ordering::Relaxed)`).
- crates/api/src/pullrun.rs:851-875: `run_pass` does `tokio::spawn(run_chain(..))` once per feed, so the feeds run concurrently. Each chain runs its legs one after another through `request_leg` → `pull_spot` → `broker_answer` → `broker_run` (pullrun.rs:682-687, server.rs:6534).

**Why it is wrong**

The CAS itself is correct. The scope that holds the key is the problem. The design comments say the key belongs to the press:
- sink.rs:972: "The OUTERMOST scope that knows a run has begun takes the key, every concurrent leg beneath it inherits that one id."
- server.rs:7052-7055: "a leg that finished early can no longer clear the key out from under the ones still running -- which is what left the survivors emitting events with no run at all."

But the claim is made in `broker_run`, one level below the press. Whichever leg claims first also releases at the end of its own leg, while the other feeds' chains are still running. The CAS stops a loser from clearing the key, but nothing stops the winner from clearing it early. Once it is cleared, the next leg on any chain claims a new millisecond id, and the still-running sibling inherits that one. The defect the comments call fixed, a story that stops mid-sentence and a second story assembled from two feeds, can still happen whenever a press has two or more feeds.

**Repro (exact interleaving)**

Take a press of two feeds: Dhan with legs D-day and D-minute, and Groww with leg G-day.

1. t0: the chains spawn. D-day enters `broker_run` and claims X (`run = X`). G-day enters `broker_run`, its `claim_run` fails, and `claimed = None`. G-day's "pull.run started" event and its per-instrument events carry X.
2. t1: D-day finishes. `note_run_finished` → `release_run(X)` succeeds, so `run = 0`.
3. t2: G-day's next per-instrument events go through `telemetry::emit` → `emit_for_run(self.run.load())` = 0. The `run` key is omitted.
4. t3: the Dhan chain starts D-minute. `claim_run(Y)` succeeds because the key is free. G-day's remaining events, including its "pull.run finished" event, now carry Y.

Result: `/logs?run=X` shows G-day starting and never finishing. `/logs?run=Y` shows a G-day finish with no start, mixed in with D-minute. Some of G-day's events carry no run at all. This is telemetry correlation only. No stored bar or journal row is affected.

**Minimal fix**

Claim the key in the press scope. In `pullrun::conduct_with`, take `id = sink.reserve_run_id()`, call `claim_run(id)` before the first `run_pass`, and `release_run(id)` after the last pass has joined (or in `Finisher::drop`). In `broker_run`, keep the claim only as a fallback for a lone hand or autopilot run: if the press holds the key, `claim_run` already fails and the leg does not release. Alternatively, thread the press id into the legs and emit with `emit_for_run`, as `sweeprun` already does (sweeprun.rs:1371, 1613).

## Pass-1 verification (findings that touch this slice)

| Pass-1 item | Verdict | Reason from the code |
|---|---|---|
| autopilot-1 (pause in the pre-fetch window is ignored) | CONFIRMED | autopilot.rs:2643 checks `is_paused()` (Relaxed). server.rs:7819 later captures `epoch()` (Relaxed). `pause()` (autopilot.rs:2034-2035) stores `paused` and then does `fetch_add` on the epoch. A pause between those two reads bumps the epoch before it is captured, so `stopped(epoch)` (server.rs:7840) stays false for the whole run. On ordering: as written, both are Relaxed on two different atomics. So even with the suggested reorder (capture, then check paused), a reader could see the new epoch and still read `paused == false`. Nothing orders the two locations. The pass-1 fix note is correct that the bump must be Release and the capture Acquire (or both SeqCst). |
| telemetry-1 / cli1-5 / xcut-1 (`reserve_run_id` duplicates across processes) | CONFIRMED (the atomic part) | `reserved_run` is a per-process `AtomicU64` seeded from `resume_point` (sink.rs:746, 788). `fetch_update` (sink.rs:1032-1039) is exact only inside one process. Two processes that share a log dir start from the same seed and hand out the same ids. |
| hunt-conc-8 (`SWEEPS_SHARING_THIS_MACHINE` is process-global, and the last guard drop resets it to 1) | CONFIRMED (info, as reported) | cli/lib.rs:16529-16540: `these` is `store(count)` and `Drop` is `store(1)`, both Relaxed, with no counting. Overlapping guards would clobber each other. Only batch.rs:356 and lib.rs:15218 create guards, and the server's admission serialises them. Relaxed is otherwise sufficient: rayon's job injection orders the store before worker reads. |
| pass-1 telemetry "claim_run/release_run sound" (concurrency.md:753) | PARTLY REFUTED | The CAS pair is correct as a primitive. The production caller holds it at the wrong scope; see atomics-1. |
| pass-1 server/runs "pullrun checkpoints Relaxed OK" (concurrency.md:286) | CONFIRMED | `clean` flags (pullrun.rs:760, 785, 855) and `attempted` (776, 889, 895) are read only after `chain.await`. A JoinHandle await synchronizes-with the task's completion. The reset at 855 runs before the spawn. |
| sweep_evidence atomics (concurrency.md:1412) | CONFIRMED | `acknowledged_*` use Release/Acquire and are read in `seal`. `ranked_published.swap(AcqRel)` is a one-shot. The `FORGOTTEN` epoch is bumped while holding the `FLUSHED` lock (sweep_evidence.rs:1141-1144), and re-read under that lock before insert (1125-1128), so a refusal that lands during a flush keeps the chain unremembered. The epoch is a u64 counter that cannot wrap in practice. |
| index-stop scheduler (concurrency.md:1974, hunt-conc) | CONFIRMED | index_stop_search_progress.rs:180-188: the Relaxed `fetch_add` only hands out indices, and `assemble` refuses gaps and duplicates. `cancelled` is Release/Acquire and only advisory, because dropping the receiver also fails `send`. Overshoot past `len` is at most one per lane. |
| capture slots (hunt-conc) | CONFIRMED | capture.rs:315, 423: `fetch_update` with a `< PER_SLOT` bound is the whole decision. It publishes no data, and file names are created with `create_new`. |
| lake `stranded` (concurrency.md:893) | CONFIRMED | page.rs:550 stores and reader.rs:645 loads on the same thread, after `read_records` drains the boxed page reader. |
| GridProgress (concurrency.md:1282) | CONFIRMED | cli/lib.rs:18381-18388: the counter only decides whether to print. |
| `Assets::missing` (concurrency.md:171), `rate::ABSORBED_MICROS` (998/1133), telemetry floors (752) | CONFIRMED | All are display or gating counters with no data published through them. `floors` is one packed u16, so the global and fast floors cannot tear. |

## Checked and clean (not findings)

- **autopilot seats** (autopilot.rs:2103, 2131, 2246, 2260): `fetch_or` for one seat, CAS 0→ALL for every seat, `fetch_and(!bit, Release)` and `store(0, Release)` to drop. A loser changes nothing. `AllSeats::drop`'s blanket `store(0)` is safe because no single seat can be taken while all are held. `ALL_SEATS` covers every `seat_bit`, with a const assert that `FEED_COUNT < 8`.
- **autopilot epoch otherwise**: one u64 with coherent modification order. A pause after capture is always eventually observed by `stopped`. `resume` correctly does not bump it.
- **server `Slots`** (server.rs:16801-16824) and **detail permits** (detail.rs:59-82): `fetch_update` with a cap check, and a decrement on Drop. `Permit::owed` deliberately exceeds the cap and is counted. There is no lost wake-up, because `Notify::notify_one` stores a permit.
- **calendar_of `waiting`** (calendar_of.rs:2086-2104): a gauge that no production code reads.
- **telemetry**: `rotation_broken` is read and written only under the `inner` mutex. `reported` is a one-shot CAS. `reserve_run_id` uses checked_add with a JS-safe cap and refuses rather than wrapping. `written`, `dropped` and `rotations` are counters.
- **re-entrancy guards**: `boolean_observation_file::ReadLease` (CAS AcqRel; field order drops the flock before clearing the flag), `index_stop_source_context::Projection` (CAS Acquire / store Release), and `index_stop_search_reader::Projection` plus the `require_current` Acquire check. All are try-guards that refuse rather than wait, and no data is published through them.
- **engine, runner, indicators, vocab, core, store, costs, greeks**: no production atomics. The only hits are in test modules.
