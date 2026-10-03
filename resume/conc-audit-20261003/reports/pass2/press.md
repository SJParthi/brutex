Verdict: no high or medium defect found in the Pull press slice. 2 findings, both low. Every pass-1 finding that touches the slice is still present (11 CONFIRMED, 0 REFUTED).

Slice: the api Pull press end to end at 331b05c. Covered: the `/pull/spot`, `/pull/fno`, `/pull/run`, `/pull/run.json` and `/pull/run/stop` handlers (crates/api/src/server.rs:10884-11125, 14833-14895); `pullrun.rs` (claim, `conduct_with`, `run_pass`, `run_chain`, `Finisher`, ticker, `legs_from`, `by_feed`, `rows_now`); `broker_answer`, `broker_run`, `land_broker_member`, `land_spot`, `prepare_cash_schedule`, `land_bodies_observed`, `await_budget`, `credential_halts`, `recovery_spot`, `fno_walk`, `read_month_bars`; the seats (`take_seat`, `take_every_seat`, `Seat`/`AllSeats` Drop, autopilot.rs:2097-2268); `audit::Journal::append`/`appended`/`page` (audit.rs:1143-1335); `recorded_fact` and `recorded_with_failures` (server.rs:10616-10690); `recovery_control::stop`/`persist`; and how a `cli` process on the same store meets these paths (`cli::stored` load, `fold_audit`, `pull::ingest::derive_all`, `store::file::BarFile::open_existing`/`open_or_create` locks). I read the source only and did not run cargo.

Already filed elsewhere, so not counted here: lifecycle-1 in pass 2 (a press is not awaited at shutdown, so the receipt of its in-flight leg is lost). I had drafted it independently and dropped it. tests-docs-security (`legs_from` does not check the envelope vendor/dir against the payload). The concurrency consequence of that one is noted under "Checked" below.

---

## press-1 (low): writes by other pullers count as press growth, so a hand pull running beside a press holds off its idle stop, forces back-to-back full re-pulls and can spend the 400-pass ceiling

- **Where:** crates/api/src/pullrun.rs:509-516 (`rows_now` sums every vendor's census) and :966-983 (`conduct_with`), together with :853-857 (`run_pass` clears every checkpoint when no feed is retrying).
- **Code:**
  ```rust
  pub(crate) fn rows_now(site: &Site) -> u64 {
      let (censuses, _) = crate::server::census_now(site);
      censuses.iter().filter_map(census::VendorCensus::counters)
          .map(|(_months, rows, _entries)| rows).sum()
  }
  ...
  let before = rows_now(&site);
  run_pass(&site, &groups, &mut outcomes, &checkpoints, &request).await;
  ...
  let after = rows_now(&site);
  ...
  if after > before {
      clean_empty = 0;
      continue;            // no RETRY_WAIT, straight into the next pass
  }
  ```
  and in `run_pass`: `if !retry_cycle { for leg in clean.iter() { leg.store(false, ..) } }`.
- **Why it is wrong:** The press uses growth of the whole store across all five vendors as its proof that "this pass found new bars". Other writers can add rows while a press runs: `pull_spot` and `pull_fno` check only their own feed's seat and never `site.run`. Their rows land in the same sum. So a pass in which the press itself added nothing reads `after > before`. The loop then:
  - resets `clean_empty`;
  - skips the `RETRY_WAIT` sleep;
  - starts a fresh pass with every checkpoint cleared.

  Every leg of every feed in the basket is asked of the vendor again, back to back, for as long as the foreign writer keeps landing rows. Three things follow:
  - **Vendor quota.** A full basket is re-requested on each such pass, over one shared token.
  - **The pass ceiling.** It is spent on passes that are not the press's own work. Archive-feed legs (TrueData/GDFL) re-read a local folder and finish in seconds, so a long hand walk can burn most of `MAX_PASSES = 400`. The run then ends with "Reached the 400-pass ceiling … This is a runaway stop" (`summary_of`), which is a false verdict about the press.
  - **Misattributed counts.** The press summary's "N bar(s) added to the store census" (`current_rows - started_rows`) includes bars the hand pull wrote.

  conc-pass2/census.md covers the opposite error: a stale census cache making `after == before` when the press did add rows. This one inflates instead.
- **Repro:** Press Pull with one feed, dhan 1day over the tracked universe, for a month already fully on disk, so each pass lands 0 rows. While that runs, POST `/pull/fno` for groww, a walk that "can run for half an hour" and lands rows steadily, or a `/pull/spot` on groww for a missing month. In each press pass, groww's manifest grows, so `after > before` holds. `clean_empty` stays 0, no sleep follows, and pass N+1 clears the checkpoints and re-requests all ~800 dhan instruments. The press cannot reach its 3-clean-pass idle stop until the groww walk finishes. With an archive-feed press instead of dhan, passes are short enough that `passes` climbs toward 400. `/pull/run.json` shows `passes` rising and `rowsNow - rowsAtStart` growing by groww's rows.
- **Minimal fix:** Measure only the press's own feeds. Have `rows_now` take the vendor set of `groups` (`Feed::store_vendor` of each group key) and sum only those censuses. Better, count the press's own receipts: sum `bars_stored` out of each leg's `Ingested`, which `request_leg` would have to return. The rows of a feed the press does not touch then cannot reset its idle counter. A same-feed hand pull between two legs is still counted; refusing `/pull/spot`/`/pull/fno` while `site.run` is running for that feed would close that too.

## press-2 (low): the pull journal's first record is reported "Recorded: yes" before the new `audit/` directory and `pull.journal` directory entries are durable

- **Where:** crates/api/src/audit.rs:1172-1220 (`Journal::appended`).
- **Code:**
  ```rust
  if let Some(dir) = self.path.parent() {
      match std::fs::create_dir(dir) { Ok(()) => {} ... }
  }
  let mut file = Flock::try_lock(std::fs::OpenOptions::new()
      .read(true).append(true).create(true).open(&self.path)..., ...)?;
  ...
  file.write_all(&record.image())...;
  file.sync_all()...;      // the file's inode and data only
  drop(file);
  Ok(())
  ```
  Nothing in audit.rs (non-test, lines 1-1576) opens or syncs a directory: `grep sync audit.rs` finds only the file's `sync_all`.
- **Why it is wrong:** The module promises "written where a restart cannot lose it … `fsync`-ed before the answer is rendered" (audit.rs:1-12), and `recorded_fact` renders "Recorded: yes — appended to …" on that basis. POSIX makes a newly created file reachable after a crash only once its parent directory is fsynced. Here two new entries are created and neither parent is synced: `audit/` in the store root, and `pull.journal` in `audit/`.

  On the first record ever written to a store, a power loss before the filesystem's next metadata commit can leave the synced data blocks unreachable. The next boot sees `Log::Absent` ("never pulled"), and the bars of that run are on disk without their receipt. That is the state the module calls "a defect in the journal".

  The sibling writer that shares the directory gets this right: `recovery_control::persist` syncs `audit/` and the store root after creating children (recovery_control.rs:148-168). So does `recovery_journal::open`. The window is one-time per store: once the entries have been committed, later appends only extend an existing inode, and `sync_all` covers them.
- **Repro:** Fresh store root with no `audit/`. Press Pull (or a hand `/pull/spot`) on one instrument. `appended` runs `create_dir(audit)`, then `open(create)` of `pull.journal`, `write_all` and `sync_all`, and the page says "Recorded: yes". Cut power within the filesystem's commit interval (ext4 default 5 s), before any other metadata sync. After reboot, `audit/pull.journal` (or `audit/` itself) can be missing. `/audit` reports no journal while the manifest holds the run's rows. Not executed: this needs a power cut, and the claim rests on POSIX directory-entry durability, not a measurement.
- **Minimal fix:** After a successful `create_dir(dir)`, fsync the store root. When the `open` actually created the file (check `bytes == 0` under the lock, or use `create_new` first), fsync `dir` before returning `Ok`. Both run once per store lifetime, so the steady-state append cost is unchanged.

---

## Pass-1 verification

- **server1-1 (two parallel feed legs refuse each other's audit records): CONFIRMED.**
  - audit.rs:1190-1205 still takes `Flock::try_lock` and maps `WouldBlock` to "cannot take the journal append lock … refused".
  - `run_pass` still spawns one `run_chain` per feed (pullrun.rs:871). Each leg reaches `broker_answer` → `landed_answer` → `recorded_with_failures` (server.rs:10655-10690), which appends once per failed member, each with its own fsync.
  - No in-process mutex exists around `appended`.
- **server1-2 (Ctrl-C during a hand walk does not stop it; a second Ctrl-C does nothing): CONFIRMED in code.**
  - main.rs still passes `Box::pin(tokio::signal::ctrl_c())`.
  - The serve arm (server.rs:18541-18556) only awaits `serve(..)`, then `flying.abort()`.
  - `pull_spot` and `pull_fno` still run the whole walk inside the request.
  - Nothing on the shutdown path bumps `autopilot.epoch`, which `broker_run` checks (server.rs:7840).
  - Same caveat as the original: the axum drain behaviour is from its documentation, not its source.
- **runs-1 (Finisher and ticker write into the next run's progress): CONFIRMED.**
  - `let _finisher = Finisher {..}` is still the first local in `conduct_with` (pullrun.rs:929), so it drops after `groups`, `checkpoints`, `outcomes` and the ticker handle.
  - `Finisher::drop` (590-605) still checks only `finished.is_none()`.
  - `Progress` still has no generation.
  - `ticker.abort()` (1005) is still followed by the final write, with no join.
  - The "next run" can equally be a recovery. `recovery::claim` (recovery.rs:560-576) installs `Progress::claimed()` on the same `!running()` test. Its `drive` then writes `finished` without an identity check either (recovery.rs:832).
- **runs-3 / recovery-5 (`pull_run_stop` holds `site.run` across fsyncs): CONFIRMED, with one narrowing.**
  - server.rs:10981-10995 still calls `recovery_control::stop(&site)` while `held` is alive.
  - For a plain press, `recovery_active` is `None` and `stop` returns `Ok(())` with no I/O (recovery_control.rs:84-92). The fsync stall needs an active recovery, as both reports' repros already assume.
- **recovery-1 (a seat refusal is charged as a vendor attempt): CONFIRMED.** recovery.rs:1550 `append_attempt(..)?` still precedes `server::recovery_spot(site, &asked).await?` (1552), and `recovery_spot` still takes the seat on its first line (server.rs:8022-8025).
- **recovery-4 (the audit journal refuses instead of waiting between in-process writers): CONFIRMED.** Same code as server1-1. `recovery_spot` still appends with `journal.append(&record)?` after `broker_run` has landed bars.
- **autopilot-4 (Ctrl-C mid-tick leaves bars with no record): CONFIRMED.** `flying.abort()` is still at server.rs:18555. The receipt append still follows `broker_run` (autopilot.rs:3443, 3482).
- **hunt-api-1 (a hand pull cancelled by client disconnect loses its record): CONFIRMED.** `broker_run` (server.rs:7737-7948) still has no Drop guard between `note_run_started` and `note_run_finished`/`publish(now = None)`. The journal append is still in the caller after the await. A press leg is not exposed: it runs inside a spawned chain, not a connection future.
- **pull2-4 (exclusive try-lock on cash-session cache reads): CONFIRMED on the press path.** `prepare_cash_schedule` (server.rs:7422-7470) calls `prepare_local_observed` and then `prepare_observed` for every NSE cash minute body. Two equity feeds of one press run in parallel chains, so they reach the same day locks at the same time. That is the ordinary trigger, not an edge case.
- **server1 "Checked: seats" and autopilot "Checked: seat bitmask": CONFIRMED sound.**
  - `fetch_or` (AcqRel) with a lone-bit check. `compare_exchange(0, ALL)`.
  - `AllSeats::drop` uses `store(0)`. That is safe because no `Seat` can be live while the mask is ALL (`fetch_or` on a set bit returns `None`).
  - `Seat::drop` uses `fetch_and(!bit)`.
  - Every store writer in the api is under a seat: `pull_spot` (local and broker), `pull_fno`, `recovery_spot` and the autopilot round. `land_*`, `fno_land` and `run_local` are reachable only from those.
  - `Feed::store_vendor` is 1:1, so one seat stands for exactly one vendor census.
- **server1 "Checked: `pull_run` check-and-claim": CONFIRMED.** server.rs:10901-10914: the `running()` test and `*held = Some(Progress::claimed())` sit under one guard, before `tokio::spawn`.

## Checked and clean (no finding)

- **Lock order across the press.** `site.run` (std) is taken only inside closures (`with_progress`, `stopping`, `halted_feeds`) or short handler scopes, and never across an `.await`. The compiler would refuse it, because the futures are spawned `Send`. `site.run` → `recovery_active` is the only nesting. `site.budgets` → `governor` is released before the sleep in `await_budget`. The `Seat` is an atomic held across awaits and not a lock, so it has no ordering hazard.
- **Chain atomics.** `Checkpoints` and `attempted` are Relaxed and carry no other data. Every chain is joined (`chain.await`) before the conductor reads them, and the join gives the happens-before. `note_dead_chain` reads `attempted` after a failed join, which also synchronizes.
- **Panicking leg.** `Seat` and the `Flock` guards release on unwind. `run_pass` turns a `JoinError` into `note_dead_chain` plus `Retry`. The press slot is released by `Finisher`, with the runs-1 caveat.
- **Same-feed writers inside one api process.** Legs of one feed run sequentially in one chain. The seat refuses a concurrent hand pull on that feed with 409, which the press classifies as `Retry` (`leg_outcome`). The autopilot stands off on `site.run` and then CASes all seats. Its TOCTOU is bounded, as pass 1 noted. `pull::ingest::derive_all`'s read-minutes-then-append-derived gap, which store1 noted out of slice, is therefore not reachable inside the api: every writer of a vendor's months holds that vendor's seat.
- **The cli on the same store.** No cli path writes bars, manifests or `audit/pull.journal`. `grep` of `crates/cli/src` finds only `pull::{calendar,session,fold,vendor}` and `BarFile::open_existing`. Two kinds of contention remain, and neither is a finding:
  - A cli read and an api landing of the same month refuse each other promptly through the month lock: a shared try-lock against an exclusive try-lock. `cli::stored` names the lock, and the press retries the leg. A refused `derive_all` rung is re-derived from the whole minute month on the next pass's ingest of that month, so nothing is permanently short.
  - `cli fold-audit` run during a landing can transiently report a coarse rung behind its minutes, because the minute commit and the derived commits are separate locks. The module says a mismatch can mean a stale coarse file, the command only prints, and a rerun clears it.
- **`fno_walk` / `read_month_bars`.** These read the underlying's spot bars from the same vendor (`wire.store_vendor`), which is under the walk's own seat. No cross-feed read.
- **Feed parsing.** `pull_spot`/`pull_fno` take the seat from `param(body, "vendor")`, and `parse_spot`/`parse_fno` read the same first-match `param`. The seat and the written vendor therefore cannot diverge for a repeated `vendor=`.
- **`legs_from` envelope vs payload (filed as tests-docs-security).** The concurrency consequence: two groups whose payloads name the same feed become two parallel chains on one seat. They 409 each other and retry within `MAX_PASSES`. The seat still serializes their writes, so the cost is retries and mislabelled progress, not interleaved bytes.
- **`audit::Journal` torn tail.** `appended` refuses a length that is not a multiple of 256 under the lock, and never repairs it. This is documented and deliberate (module doc, 06-limits J-04). Records are 256-aligned and never cross a filesystem block, so an ENOSPC short write cannot tear one in practice.
- **Display-only cross-talk, not reported.** Concurrent `broker_run`s on different feeds share `autopilot.status.now` and `failures`, so the first to finish publishes `now = None` while another still walks. `Status::json` documents that `pull_active` "does not … claim to count every concurrent pull", and no decision reads `now`.
