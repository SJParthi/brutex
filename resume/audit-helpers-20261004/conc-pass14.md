# conc-pass14: operations that interfere with each other, at 1f4de71

**Verdict (head 1f4de71, final/all-fixes-zero, read-only checkout /home/claude/wt/zero3):** 2 new findings (0 high, 0 medium, 2 low): conc14-1 and conc14-2. No pair of operations can interleave bytes inside one month file, and every bar a sweep uses is the bar its `data_digest` names (the digest is computed over the in-memory vectors that are swept). No double-submit starts two runs on any POST except the known `/masters/refresh` queue (conc6-3). The new defects are (1) the scrub reports a live append as a counter/disk disagreement, and (2) the archive ingest is the one landing that holds a Tokio worker and cannot be stopped. 13 known findings on this theme re-checked: all NOT FIXED.

Method: source reading only (no cargo; no medium finding). Read conc-pass6 first, then the finding headings of concurrency.md, conc-pass2..13 and the triage TSV, so no ID below repeats one. Files read: `api/src/server.rs` (`spot_pull_held`, `spot_answer`, `local_answer`, `run_local`, `pull_run_stop`, `detached_pull`, `off_the_workers`, `verify_json`, `gaps_json`, `audit_span`, `audit_one`), `api/src/{verify,mastersrun,recovery,ingest,autopilot}.rs`, `pull/src/{scrub,ingest,archive}.rs`, `store/src/file.rs` (`open_or_create`, `open_existing`, `lock_fault`), `cli/src/{lib.rs (run_durable, sweep_stored, stored_sweep_inputs, stored_executed_digest), stored.rs (load), batch.rs (sweep_all), fold_audit.rs, execution_lease.rs, expression_search.rs, candidate_trades.rs}`.

Operations: **P** api pull (hand `/pull/spot` broker, `/pull/fno`, `/pull/run` press) · **A** autopilot · **I** archive ingest (`/pull/spot` with a TrueData/GDFL feed and a folder) · **M** `/masters/refresh` · **S** api-launched sweep (`/backtest/run`, `/backtest/descend`, `/backtest/command`, Boolean and index-stop launches) · **C** cli `sweep-stored`, `sweep-all`, `ledger-all`, `ledger-v6` · **R** recovery · **G** gap audit (`/gaps.json`, recovery `assess`) · **V** verify/scrub (`/verify.json`, cli `fold-audit`, `checksum-audit-stored`).

Exclusion mechanisms that exist: per-feed seat (`Control::take_seat`, `fetch_or`), all-seats CAS for the autopilot round, `site.run` slot (press and recovery), `ADMISSION` + `site.sweep` slot (api engine work), store-wide execution lease `.sweep-execution-v1.lock` (cli sweep verbs via `run_durable` lib.rs:2105-2116, and api `claim_execution`), per-vendor `CensusLock` (held across a whole `from_members_inner`, ingest.rs:805-936), per-month flock (writer exclusive `try_lock` file.rs:1417, reader shared `try_lock_shared` file.rs:1594), `REFRESH` async mutex, `serve.lock` (one api per store).

---

## Pair matrix

Legend: **X** excluded (refused, reason named) · **Q** serialised (waits) · **S** safe by design · **K** known open finding · **N** new finding here.

| pair | verdict | reason / where |
|---|---|---|
| P × P (same feed, two tabs or double-submit) | X | seat `fetch_or`; 409 "another pull already holds {feed}'s seat … If it is the autopilot, pause it" (server.rs:11300-11325). Press leg 409 = Retry |
| P × P (two `/pull/run` presses) | X | `pull_run` checks and installs under one `site.run` take; 409. runs-1 K (Finisher can finish the next run) |
| P × A | X | hand pull refuses while round holds all seats (same 409 text); round stands off on any held seat "a hand-made pull holds the pull seat" (autopilot.rs:3248-3249). conc6-2 K (autopilot Stop truncates hand walks) |
| P × I | S | different feeds, different seats, different vendor censuses and directories (`store_vendor`, vendor.rs:3704-3718) |
| P × M | S | masters live outside the store; `reparse` swaps the universe under `reload_lock`; a walk keeps the snapshot it read. Token read only, never minted |
| P × S, P × C | K | writer exclusive vs reader shared month flock: whichever comes second is refused. Reader side names the lock (stored.rs:2324-2347). Writer side says "another writer holds" for a reader and a derived rung refused this way is never re-derived: barflow-1 K. No torn view possible |
| P × R | X/K | same `site.run` slot (press) 409; hand pull vs recovery seat: recovery BLOCKED and charged: recauto-1/recovery-1 K |
| P × G | K | `/gaps.json` opens months shared; a busy month renders as `absent_file` naming the lock (honest). Cash-day cache exclusive lock refuses a live equity ingest: equity-2 K |
| P × V (`/verify.json`) | **N conc14-1** | a month appended during the scrub (or committed and not yet counted) is reported as a disagreement |
| P × V (`fold-audit`) | S (noted) | transient behind-rung mismatch, judged not a finding in conc-pass2/press.md:114 |
| A × A | S | one `fly` task; resume/stop races are conc6-1, CE-46 K |
| A × I | X | an archive ingest holds its feed's seat, so `take_every_seat` fails and the round stands off; text says "a hand-made pull holds the pull seat" (accurate) |
| A × M, A × S, A × C, A × G, A × V | as P × same | autopilot is a P writer under all seats; same rows apply (barflow-1, equity-2, conc14-1) |
| A × R | X/K | round stands off on `site.run`; recovery started mid-tick is BLOCKED: recauto-1 K; text names "hand-made pull" for a recovery: conc6-4 K |
| I × I (double-submit, same archive feed) | X | seat 409 |
| I × I (TrueData and GDFL at once) | **N conc14-2** | each holds a Tokio worker for the whole walk and write; neither can be stopped |
| I × S / C / G / V | as P × same | same month-lock and census windows; `from_dir` holds the CensusLock for the whole folder, so conc14-1's window is the whole ingest |
| M × M (double-submit) | Q, unbounded | conc6-3 K: every press queues another full credentialed refresh |
| M × S / C / R / G / V | S | none of them reads the masters directory mid-write except through the parsed universe |
| S × S (two tabs, double-submit) | X | `admit` under `ADMISSION`: 409 Busy; execution lease. runs-4 / log-1 K (busy 429 / permits parked) |
| S × C, C × C (any two of sweep-stored, sweep-all, ledger-all, ledger-v6) | X | store execution lease, "another sweep owns this store's execution lease; no new work was queued". Status GET takes the exclusive lease: runs-2/cli1-3 K; index try-lock busy: cli1-2 K; killed cli leaves its marker and refuses browser launches: sweep-1 K; a ROOT shared by two stores: pop2-7 K |
| S/C × data_digest | S | `stored_executed_digest` (lib.rs:2639-2649) binds signal, exact-minute, daily and execution vectors, i.e. exactly what is swept. Multi-read snapshot mixing in range/pool/ledger: rangeall-1, rangeall-2, ledgerall-3 K |
| S/C × R, S/C × G, S/C × V | S | readers only (shared locks); `checksum-audit-stored` is not a sweep verb and takes no lease, its publish races are replay-3 K |
| R × R (double-submit) | X | `claim` 409 or journal flock 503 (conc6 table C) |
| R × G | S | `assess` reads 1day then 1min under separate shared locks; the later read is never older, a busy month refuses with the lock text |
| stop × start (press) | S | `pull_run_stop` with no running slot answers `{"stopping":false}`; recovery `claim` clears STOP only for an explicit start (conc6 table C) |
| stop × start (sweep) | S | no stop route; shutdown `cancel::request`; `TaskFinisher` writes ABNORMAL_END (conc6 table D) |
| stop × start (autopilot) | K | CE-46 / autopilot-3, conc6-1 |
| stop × archive ingest | **N conc14-2** | `run_local` never checks the autopilot epoch that 06-limits names as the hand-pull stop |

---

## New findings

### conc14-1 (low): `/verify.json` reports a month that a live pull is appending to as "the disk says otherwise", because the census snapshot it checks against is older than the file

- **Where:** `crates/pull/src/scrub.rs:243-249` (`one`), `crates/api/src/verify.rs:229-246` (`vendor` walks `manifest.newest()` from one census snapshot), `crates/api/src/server.rs:4812-4821` (`verify_json` takes `census_now` once, then scrubs every entry). Writer side: `crates/pull/src/ingest.rs:2632-2679` (`write_and_count` commits the month and drops its lock) and `:936` (`install_census` runs only after the whole member loop).
- **Code:**
  ```rust
  let held = file.records();
  if held != entry.rows {
      return Finding::Rows {
          counted: entry.rows,
          held,
      };
  }
  ```
- **Why it is wrong:** the scrub's census is read once before a walk that opens every held month (thousands of files). A month the autopilot or a hand pull appends to during that walk (the census itself records a backfilled month growing day by day, verify.rs:196-201) is opened after its append and compared against the pre-append entry, so `held > counted`. A second window exists even with a fresh census: `from_members_inner` commits each member's month and releases the month lock, and the census that counts it is installed only after every member (and its derived rungs) is done. For an archive ingest that is the whole folder walk. In both windows the month's lock is free, so the `Busy` class that W1-api6-7/D-1501 added for exactly this situation ("A LIVE WRITER IS NOT A FAULT EITHER") does not fire. The answer is `verified:false` with "these month(s) read as held while the disk says otherwise", the one sentence this surface reserves for a store that lies. The bars are sound and the counter is merely behind an append-only file.
- **Repro (not run):** with the autopilot backfilling dhan, poll `GET /verify.json?feed=dhan` during a round. The month in progress shows up as `"rows":1` with "the counter claims N, the file holds N+k", and the next poll after the round agrees.
- **Minimal fix:** files are append-only, so a prefix check is exact. In `scrub::one`, when `held > entry.rows`, read record `entry.rows - 1` (and record 0) and, if they carry `entry.last_ts_micros` and `entry.first_ts_micros`, return `Finding::Busy` (or a new `Ahead`, "the file has grown past the counter; scrub again once the pull finishes") instead of `Rows`. Keep `Rows` for `held < counted` and for a prefix that does not match.

### conc14-2 (low): the archive ingest runs the whole folder walk and every write on a Tokio worker with no `block_in_place`, and it cannot be stopped; 06-limits says the opposite of both

- **Where:** `crates/api/src/server.rs:6336` (`local_answer(&asked, &folder, now, site, &journal, facts)`, a synchronous call inside `async fn spot_answer`), `:10685-10708` (`local_answer` calls `run_local` directly), `:10985` (`pull::ingest::from_dir(...)`). Compare `:7651-7661` (`off_the_workers`, used by every broker landing). Doc: `docs/06-limits.md:14157-14163` (D-1589, D-1581).
- **Code:**
  ```rust
  facts.push(("Store root", site.store_root.display().to_string()));
  local_answer(&asked, &folder, now, site, &journal, facts)
  ```
- **Why it is wrong:** `detached_pull` puts the hand pull on a `tokio::spawn` task, and for an archive feed that task then runs `from_dir` inline: a walk of up to `archive::MAX_MEMBERS` = 50,000 members of up to `MAX_MEMBER_BYTES` = 256 MiB each, all decoded and held (archive.rs:36-48), then one locked append and fsync per instrument-month, all under the vendor's `CensusLock`. The worker that runs it is held for that whole time. TrueData and GDFL have separate seats, so both can run at once. On a 2-core host (main.rs builds the default multi-thread runtime, one worker per core) the two block every worker, and the autopilot, `/pull/run.json`, `/autopilot/control` and `/pull/run/stop` stop answering until one ends. On a larger host they still take workers away from the rest of the api. `run_local` also checks nothing between members, neither `site.autopilot.stopped(epoch)` nor `cli::cancel`. So the only stop is killing the process, which is the unaudited mid-write case. 06-limits states "A pull's landing uses `block_in_place` (D-1589)… so the HTTP surface keeps answering" and "The stop control for a hand pull remains the per-instrument pause of the autopilot epoch". Neither is true for this kind of pull.
- **Repro (not run):** on a 2-vCPU host, POST `/pull/spot` with `vendor=truedata` and a large folder, and at the same time POST with `vendor=gdfl` and another. While both run, `GET /autopilot.json` hangs until one walk returns, and POST `/autopilot/control action=stop` has no effect on either.
- **Minimal fix:** wrap the call: `off_the_workers(|| local_answer(...))`, the helper the broker path already uses. Then pass a stop predicate into `from_dir` and check it per member (the epoch check `broker_run` uses), or amend 06-limits to state that an archive ingest is unstoppable.

---

## Checked and clean (this pass)

- **Torn or mixed view of one month:** impossible. A reader's shared `try_lock_shared` and the writer's exclusive `try_lock` (file.rs:1417/1594) exclude each other, and `stored::load` decodes the whole month under that one handle (stored.rs:2337-2366). (When `.lock` is absent there is no lock at all: store1-1 K.)
- **`data_digest` vs swept bars:** `sweep_stored` loads signal, 1min execution, daily context and exact-minute context in four separate reads. `stored_executed_digest` hashes all four vectors as used, so the identity names exactly what was swept. A pull between reads changes the identity and does not falsify it. Write order (minute, then derived) means the later 1min read is never older than the signal read. `candidate_trades` and `expression_pricing` hash the same in-memory slices.
- **Double-submit:** `/pull/spot`, `/pull/fno` and archive (seat), `/pull/run` (slot), recovery (slot plus flock), `/backtest/*` (`ADMISSION` plus lease) all refuse the second with 409/503 and a named reason. `/ingest/queue` never queues. Only `/masters/refresh` queues (conc6-3 K).
- **Stuck "running":** every slot has a Drop guard or a terminal write (conc6 tables B-D). A stop with nothing running answers `stopping:false`.
- **Gap audit vs writer:** a busy month comes back as `absent_file` carrying the lock text, not as silent zero coverage.

## Verification of known findings on this theme (state at 1f4de71)

| id | state | evidence |
|---|---|---|
| barflow-1 | NOT FIXED | store/src/file.rs:1417 still `Flock::try_lock`, `lock_fault` :3601-3606 maps WouldBlock to `Locked`, displayed "another writer holds" :988 |
| runs-1 | NOT FIXED | api/src/pullrun.rs:684-708 `Finisher::drop` tests only `finished.is_none()` |
| runs-2 / cli1-3 | NOT FIXED | cli/src/execution_lease.rs:133-143 `probe` takes and releases the exclusive lock |
| cli1-2 | NOT FIXED | operation_audit index still non-blocking try-lock (triage row, unchanged head) |
| sweep-1 | NOT FIXED | api/src/sweeprun.rs:2705-2719 stale "command started" marker still refuses launch |
| store1-1 | NOT FIXED | store/src/file.rs:1586-1597 `lock = None` when `.lock` is absent |
| equity-2 | NOT FIXED | pull/src/cash_session_cache.rs:390 `Flock::try_lock` (exclusive) reached by read-only audits |
| rangeall-1 | NOT FIXED | cli/src/lib.rs `one_rung` sizes min_hits from one `load_span` and sweeps a second |
| rangeall-2 | NOT FIXED | cli/src/pool.rs:853-866 `price_all` re-runs `load_span` with no digest comparison |
| ledgerall-3 | NOT FIXED | cli/src/ledger_all.rs:1400-1407 min_hits from `load_span` before the families reload |
| recauto-1 | NOT FIXED | recovery.rs reserve-then-seat order unchanged (fix only on unmerged zero/conc-api 02cdb0d) |
| conc6-2 | NOT FIXED | server.rs:7917 `broker_run` still captures `site.autopilot.epoch()` for hand walks |
| conc6-3 | NOT FIXED | api/src/mastersrun.rs:552 `REFRESH.lock().await` inside the detached task, no `try_lock` at the door |

Tally: 13 checked, 0 FIXED, 13 NOT FIXED.
