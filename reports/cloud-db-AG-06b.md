STATUS: HELD

# AG-06b (cloud): pull runs never start unjournalled, and no tokio worker waits on the journal lock

Branch `cloud/db-AG-06b-20261024`, cut from `origin/feat/db-fix` e851da1ca (Merge DATA-2).
Spec: `reports/db-specs-20261024/AG-06b.md` on `origin/cloud/db-base-20261024`. GO seen
2026-10-24 09:08 IST. HOLD (pushed 09:16 IST, "usage window near its limit, expected lift soon
after 12:30 IST") was first seen at the 09:52 IST save point: the work between 09:16 and 09:52
(the code and tests below) was done before this session checked for it. Committed, pushed, held.

Prerequisites: none hard. IA-10c is a FILES_ONLY link (not merged at e851da1ca): the start is not
blocked, the merge-back waits for it.

## Done so far (commit ef43189b, pushed)

- **AG06B-1** (`crates/api/src/server.rs`, owned region): `BrokerRun::opened` returns the blocked
  shape (`503`, `journal_unavailable: ...` carrying the journal's own refusal: bound, wait,
  attempts, record kind and outcome, holder in `holder_note`'s words) when `RunLedger::open`'s
  `open_run` is refused, and keeps the ledger (so `opened_refused` still reaches the autopilot's
  unchanged `journal_error`). `broker_run_by` returns it before its first vendor request, after
  one `pull.run` `refused` event (Error, audited) that also releases the run key
  (`note_run_refused`). `broker_answer` and `recovery_spot` append nothing more for such a run;
  the receipt carries the refusal, a `Recorded: NO` line and a `transient` retry class.
  `fno_walk`: `fno_opened` is now a precondition (`fno_unjournalled`), same sentence, same lines.
- **AG06B-2**: `RunLedger::open`, `close`, `flush`, `rest`, `fno_opened`'s append and
  `FnoPage::say_counted`'s record, plus the blocked-arm appends of `broker_answer` and
  `recovery_spot`, go through `off_the_workers` (no `spawn_blocking` added).
- **AG06B-3** (`audit_lock_wait_measure.rs`): the harness run is one function `one_run`, shared by
  the ignored measurement and the new test; its close is `Record::of_run` with the opening's source
  and window.
- `crates/api/src/emitted.rs` (outside ownership): lib emit sites 96 -> 97,
  `REACHED_IN_SERVER_TESTS` 32 -> 33, for the one new `pull.run refused` site.
- Tests (`crates/api/src/journal_unavailable_tests.rs`, new): the four failing-first tests the spec
  names and 11 extremes; all 15 pass on this branch (log: 14 of 15 at 09:45 IST, the 15th, the
  harness, red then by design and fixed after).

## Left (NEXT)

- NEXT: (1) when HOLD lifts, run the before build (scratch worktree at e851da1ca + the test file,
  building now) and record each failing-first test red; (2) re-run the branch tests; (3) docs/04,
  docs/05 (D-7100, D-7101, maybe D-7102), docs/06 L-AG06b rows; (4) measurements p50/p99/max
  (uncontended open_run/append_member/close_run before/after, GET /feeds.json under a held lock
  before/after on a temp root); (5) full gates; (6) report DONE.

## Log

### 2026-10-24 09:20 IST
- Read README, CONTEXT, AG-06b spec in full. Branch created. First background build of `api` tests
  started (10 m 18 s, EXIT 0).

### 2026-10-24 09:45 IST
- `cargo test -p api --locked --lib -- journal_unavailable_tests the_measure_harness_closes_every_run_it_opens`:
  `test result: FAILED. 14 passed; 1 failed` — the one failure is the harness test, red before its
  fix: "a harness run reads as unclosed: INTERRUPTED: a pull run for 2025-07-01..2025-07-01 was
  opened at journal record 8 ... and has no closing record".

### 2026-10-24 09:52 IST
- WIP commit ef43189b pushed. HOLD found. STATUS: HELD.
- NEXT: wait for HOLD to go (poll as README says), then the NEXT list above.
