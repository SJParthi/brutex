# Concurrency and state audit, pass 9: the telemetry and logging pipeline

**Verdict (head 1f4de71, final/all-fixes-zero, read-only checkout /home/claude/wt/zero3):** 3 new findings: conc9-1 (medium, reproduced with cargo), conc9-2 (low), conc9-3 (low). Another pass is needed. The writer core in `crates/telemetry` holds up under this pass's attacks:
- one sink per directory (`events.lock`);
- `ms` clamped inside the lock;
- a torn tail is terminated at open and on ENOSPC;
- the reader walks by FileId across a roll;
- the scan budget and the overlong-line cap are exact;
- no fsync on the emit path.

All three defects are at the edges: one emitter, one reader policy, and one surface.

Method: I read the source. I ran cargo once, for conc9-1 only. That was one throwaway test in a scratch worktree (`/home/claude/wt/scratch-conc9`, CARGO_BUILD_JOBS=2), and the worktree and its target directory have been removed. Known ids are not re-reported: telemetry-1/xcut-1, cli1-5, CE-5, CE-36, CE-41, CE-45, CE-51, CE-11, the conc-pass2 stale-marker wedge, and the crash-edge-pass6 grid-progress attempt token.

## Writer/reader tables

### W1 cli sink vs W2 cli sink (two cli commands, one `<store>/logs/cli`)
| attack | result at 1f4de71 | evidence |
|---|---|---|
| both append, both roll | impossible: the second `Sink::open` is refused | sink.rs:1508-1528 `hold_directory` `try_lock`, held in `Sink::held_lock` |
| what the refused command does | runs with no log and prints the refusal **only after the command finishes** | **conc9-2** |
| seq / run-id fork | gone with the lock | sink.rs:784-787 |

### W (api sink `<store>/logs`) vs W (cli sink `<store>/logs/cli`)
| attack | result | evidence |
|---|---|---|
| shared files | none: separate directories and separate locks | logs.rs:285 `cli_half`; server.rs:19110 `served_log_dir` |
| `BRUTEX_LOG_DIR` vs `BRUTEX_LOGS` mismatch | known as CE-45, still NOT FIXED | cli lib.rs:3098-3103 |
| one in-process sink, many threads | one mutex; clock read and clamped inside it; seq is lock order | sink.rs:1166-1183 |

### Writer vs reader (`telemetry::tail`, /logs, /logs.json, /backtest/run.json, live-progress)
| attack | result | evidence |
|---|---|---|
| roll renames the file mid-walk (duplicate) | no duplicate: keyed by (dev, ino) from the open handle's fstat | tail.rs:380, 420-432 |
| roll mid-walk (gap) | no gap: the skipped repeat moves on to the next path, which now holds the older file | tail.rs:425-432 |
| `events.ndjson` absent between rename and reopen | NotFound is skipped; `.1` becomes the first file read | tail.rs:396-399 |
| partial last line while the writer is mid-`write(2)` | `partial_tail=true` and the fragment is skipped. That is correct for /logs. **The sweep admission reads it as damage and refuses a launch** | **conc9-1b** (folded into conc9-2's table as a note, see below) |
| torn or non-JSON line | counted in `malformed` and stepped over; lines after it are still returned | tail.rs:595-600 |
| line wider than any buffer | the writer caps it (widest line about 41 KB). The reader refuses runs over 64 KiB as one malformed line | tail.rs:83-109, 562-571 |
| `since` early exit | sound within one sink: the `ms` clamp, plus a resumed floor at open | sink.rs:597-601, 784-799 |
| merge of the two halves | sorted by (ms desc, seq desc), limit applied to the union, `missing` summed | logs.rs:361-418 |
| clock stepped backwards | clamped and counted in `Health::clock_held`. **Nothing renders the count** | **conc9-3** |
| seq order equals causal order? | yes for one thread's emits. **No for counters a worker computes before taking the lock** | **conc9-1** |
| disk full mid-line | the fragment is terminated, or `torn` makes the next event lead with `\n` | sink.rs:1235-1285 |
| per-event cost | lock, format, one `write_all`; no fsync, no stat. A roll is at most 2*keep syscalls once per 8 MiB | sink.rs:1166-1240, 1333-1368 |
| gate 17 | vocab, engine, indicators and runner have no telemetry dependency and no print/log tokens | the four Cargo.toml files; grep is clean |

## New findings

### conc9-1 (medium): exit-grid progress events reach the file out of order, and the live view refuses the healthy run
- **Site:** crates/cli/src/lib.rs:18823-18832, `GridProgress::tick`, which is called from `par_iter` at lib.rs:12391:
  ```
  let done = self.counted.fetch_add(1, Relaxed) + 1;
  if done.is_multiple_of(self.stride) || done == self.total {
      note_grid_progress(self.recording, done, self.total);
  ```
  The reader is web/src/lib/live-progress.ts:426-447:
  ```
  priced < held.priced ) { return refused(`Rung ${rung} reported inconsistent exit-grid progress.`, ...)
  ```
- **Why it is wrong:**
  - `done` is fixed at the `fetch_add`.
  - `seq` and file position are fixed later, when that thread takes the sink mutex.
  - A worker that crosses decile d1 can lose the mutex to a worker that crosses d2 > d1. The file then holds `priced=d2` before `priced=d1`.
  - The fold treats any decrease as a refusal, and `refused` returns `phase:'failed'`.
  - The pair stays in the window, so every poll for the rest of the rung shows the run as failed.
  - The doc on `tick` says a race "costs a log line and nothing else". That is false given this reader.
  - conc-pass1 and pass2 confirmed this site on the same wrong premise.
- **Repro (ran):** a throwaway test in cli lib installed a sink in a temp dir and ran `GridProgress::over(N, None)` with `(0..N).into_par_iter().for_each(|_| tick())`. It then counted, round by round, how often `priced` decreased in `events.ndjson` file order:
  - N=10, 300 rounds: **585 inversions**.
  - N=365 (stride 37, 10 events per round), 1000 rounds: **1429 inversions**.

  Real pricing work per candidate is heavier, which narrows the window but does not close it.

  This is independent of crash-edge-pass6, the attempt-token refusal on the same event. Once that one is fixed, this one is the refusal the operator sees.
- **Minimal fix (Rust side):** emit under an order that matches the count. Either:
  - (a) keep a `Mutex<usize>` "last spoken" and emit only while holding it, only if `done` is greater than the last value spoken; or
  - (b) emit from `tick` only `max(done, last_spoken)` with a `fetch_max`, skipping a stale decile. Either way the file is monotone.

  Alternatively, relax the JS to ignore a lower `priced` rather than refuse. That loses fail-closed behaviour, so (a) is better. Fix the doc on `tick` too.

### conc9-2 (low): a cli command refused by `events.lock` runs unlogged, and the operator is told only after it ends
- **Site:** crates/cli/src/main.rs:69 and :82-84:
  ```
  let where_events_went = cli::install_log();
  ...
  let shown = format!("{where_events_went}\n{out}");
  ```
  together with telemetry sink.rs:1520-1524 (the D-1537 refusal).
- **Why it is wrong:**
  - Since D-1537, any second cli process on `<store>/logs/cli` gets "events are NOT being recorded: another sink ... holds this telemetry directory". For example, a sweep started while a `cli report` or `verify` is still running.
  - That line is put in front of the report and delivered when the command exits.
  - A multi-hour sweep therefore writes no `command started` marker, no rung events and no progress. Meanwhile:
    - `/logs` shows nothing;
    - `/backtest/run.json` reports the PREVIOUS sweep's marker, e.g. "completed", from `observe_elsewhere` (sweeprun.rs:2650-2748) while the lease says busy;
    - the terminal is silent until the end.
  - `install_log`'s own doc (lib.rs:3027-3029) promises the operator "is told why on the same screen rather than discovering an empty log later". In practice they are told later.
- **Repro:** not run. Hold `<store>/logs/cli/events.lock` with any running cli verb, start `cli sweep-stored ...` in a second shell, and watch /logs and stderr during the run.

  A related reader note, also low and also not run: `tail_fault_unless_answered` (sweeprun.rs:2583-2592) counts `partial_tail` as damage. A cli writer whose line straddles a page boundary is briefly visible with `i_size` mid-line. While a non-sweep cli command is logging, a browser Run (`claim_execution`, sweeprun.rs:2009-2014) or a status poll can therefore get a transient "external sweep log is incomplete ... partial tail true". `events.lock` now answers the question directly: a `try_lock` on it that returns WouldBlock means the fragment is a live writer in the act, not a tear.
- **Minimal fix:**
  - In `main`, `eprintln!` the `install_log` line immediately when it starts "events are NOT being recorded", and keep it in the final output as well.
  - In `tail_fault_unless_answered`, ignore `partial_tail` while the cli `events.lock` is held by a live writer.

### conc9-3 (low): `Health::clock_held` is counted and never shown, so a clock that stepped back or jumped forward stamps every event at a frozen instant while /logs says "Sink healthy"
- **Site:**
  - telemetry sink.rs:519-527 (`clock_held`);
  - sink.rs:530-534 `is_loud`, which is `dropped > 0 || rotation_failures > 0` and ignores `clock_held`;
  - api logs.rs:541-561 `sink_json`, which emits no `clock_held`;
  - api logs.rs:858-870 `health_banner`, which shows `last_error` only when loud.
- **Why it is wrong:**
  - A clock reading in the future, even once (e.g. an NTP correction), raises `last_at`.
  - Every later event is held at that stamp until the real clock passes it. That is up to the whole size of the jump, and it carries across restarts through `resume_point`.
  - D-1538 added the counter "so it is not silent", but no surface reads it.
  - The open-time notice ("Sink::health().clock_held counts them") goes into `last_error`, which the HTML banner hides while `is_loud()` is false.
  - So `/logs` reads "Sink healthy" while every row's time is wrong.
  - Every row's age in `observe_elsewhere` is then 0, so a killed CLI sweep's marker stays `running` past `STALE_AFTER_MILLIS`.
- **Repro:** not run. The clamp is already proved by `a_backward_clock_cannot_move_ms_backwards`. `grep -rn clock_held crates/api/src` finds only a test literal (logs.rs:2014).
- **Minimal fix:**
  - Add `"clock_held"` to `sink_json`.
  - Render a banner line when `clock_held > 0`, or include it in `is_loud`.
  - Show `last_error` in the banner whenever it is set.

## Verification of earlier rows touching this pipeline
| id | state | evidence |
|---|---|---|
| telemetry-1 / xcut-1 / cli1-5 | FIXED | sink.rs:1508-1528 `events.lock` EX `try_lock`, held for the sink's life (D-1537) |
| CE-5 | FIXED | cli lib.rs:3016 `knob::folder("BRUTEX_LOG_DIR", ..)` |
| CE-36 | FIXED | api server `log_dir_from` via `knob::folder` (as in crash-edge-pass5) |
| CE-41 | PARTIAL | banner fixed (logs.rs:880-893). sink.rs:1207-1213 still sets `rotation_broken` once and never retries |
| CE-45 | NOT FIXED | cli lib.rs:3098-3103 still promises /logs while `BRUTEX_LOG_DIR` points elsewhere |
| CE-51 | NOT FIXED | sink.rs:1457-1466 `resume_point` unchanged: an undecodable newest block restarts seq and the floor at 0 |
| CE-11 | FIXED | sweeprun.rs:2626-2642 re-walks to the marker so damage behind it no longer blocks |
