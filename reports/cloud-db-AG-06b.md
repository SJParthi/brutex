STATUS: HELD

# AG-06b (cloud): pull runs never start unjournalled, and no tokio worker waits on the journal lock

Branch `cloud/db-AG-06b-20261024`, cut from `origin/feat/db-fix` e851da1ca (Merge DATA-2;
still the head at 13:25 IST). Spec: `reports/db-specs-20261024/AG-06b.md` on
`origin/cloud/db-base-20261024`.

Holds: the first HOLD (09:16 IST) was seen at the 09:52 save point and lifted by 12:37; the
second (12:56 IST, "parts without a GO-<NAME> file ... wait until this file is gone, at the latest
after 17:30 IST") was seen at the 13:25 save point. Work in 12:37-13:25 began before it. Every
step is committed and pushed (tip a134f6ee before this report).

Prerequisites: none hard. IA-10c is a FILES_ONLY link (not merged at e851da1ca): the start is not
blocked, the merge-back waits for it.

## Done (code complete; ledgers written; measured)

- **AG06B-1** (`crates/api/src/server.rs`, owned region): `BrokerRun::opened` returns the blocked
  shape (`503`, `journal_unavailable: ...` carrying the journal's own refusal: bound, wait,
  attempts, record kind and outcome, holder in `holder_note`'s words) when `RunLedger::open`'s
  `open_run` is refused, and keeps the ledger (so `opened_refused` still reaches the autopilot's
  unchanged `journal_error`). `broker_run_by` returns it before its first vendor request through
  `BrokerRun::refused_at_opening`, which writes one `pull.run` `refused` event (Error, audited)
  and releases the run key (`note_run_refused`). `broker_answer` and `recovery_spot` append
  nothing more for such a run; the receipt carries the refusal, a `Recorded: NO` line and a
  `transient` retry class (so `/pull/run` retries the leg and files it as `lastError`).
  `fno_walk`: `fno_opened` is a precondition (`fno_unjournalled`), same sentence, same lines.
- **AG06B-2**: `RunLedger::open`, `close`, `flush`, `rest`, `fno_opened`'s append and
  `FnoPage::say_counted`'s record, plus the blocked-arm appends of `broker_answer` and
  `recovery_spot`, go through `off_the_workers`. No `spawn_blocking` added.
- **AG06B-3** (`audit_lock_wait_measure.rs`): one `one_run` shared by the ignored measurement and
  the new test; its close is `Record::of_run` with the opening's source and window.
- **AG06B-4**: `docs/05` D-7100, D-7101; `docs/04` AG06B-01..08; `docs/06` L-AG06b-01..07 with
  the measurements.
- One existing test changed, with its reason: the last block of
  `an_expired_fno_walk_cut_short_is_seen_at_the_next_start` asserted the walk ran on after a refused
  opening ("Opened in the journal: NO"); it now asserts the refusal, and keeps "nothing was
  appended" (D-7100 item 6). `a_run_whose_opening_record_was_refused_closes_behind_it_written_late`
  is unchanged.

## Gates so far

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | clean (13:20 IST) |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | EXIT 0, `Finished` (12:55 IST, before the docs-only and test-assertion edits since) |
| new tests, after | `cargo test -p api --locked --lib -- journal_unavailable_tests the_measure_harness_closes_every_run_it_opens` | 15 passed (09:45 + 12:50 runs); to be re-run in the final gate run |
| new tests, before | base e851da1ca + only the test file (scratch worktree), lib test binary, filter `journal_unavailable_tests` | **0 passed; 13 failed** (each for its own reason, below); the read-only one re-run as uid 65534 from a world-readable copy: FAILED with `refused` (not refused on the base) |
| harness test, before its fix | the branch with the old windowless close | FAILED: "a harness run reads as unclosed: INTERRUPTED: a pull run for 2025-07-01..2025-07-01 was opened at journal record 8 ... and has no closing record" |
| workspace tests, deny, CI shell gates, coverage, mutants | | NOT RUN YET (see NEXT) |

Red-before reasons (base build): the refused-opening test "a run whose opening the journal
refused is itself refused" (no refusal); both stall tests "a GET waited 956.9 ms / 861.8 ms";
the shape test "a ledger method calls the journal on a worker"; the extremes "refused" (no
refusal) or the run reached the vendor; the autopilot test `vendor.asked()` 1 not 0; the F&O test
no `journal_unavailable` on the page; the `/pull/run.json` test HTTP 502 not 503. The run-key test
calls the new `note_run_refused`, so before the change it does not compile (stated, not hidden).

## Measurements (n = 200 each; method and full table in docs/06 AG-06b section)

- Ledger calls uncontended, quiet machine, three pairs: before p50 open 0.302-0.318 ms / flush
  0.153-0.165 / close 0.298-0.328; after 0.244-0.359 / 0.128-0.182 / 0.245-0.360 (after/before
  0.77-1.18, within the spread of one build's own pairs).
- AG-06 harness on empty temp roots: 1 writer open_run 0.319/0.871/2.20 ms before, 0.353/0.660/1.44
  after; 8 writers 0 refused both.
- GET /feeds.json, one-worker runtime, holder keeps the lock while runs open: before p50/p99/max
  **2,002.5 / 3,110.6 / 3,132.1 ms**; after **1.27 / 48.6 / 126.0 ms**. No holder: 1.24/177.7/326.4
  before, 1.57/169.2/348.5 after (load average 3, includes cache warm-up).

## Extreme cases (each a test; all red before, green after)

1. zero wait: refused at the first attempt (journal path is a directory), at once
2. asked again at once: refused, then lands when the holder lets go
3. journal missing and cannot be made (`audit/` is a file)
4. journal read-only (as uid 65534, where mode bits bind)
5. current-thread runtime: ledger calls inline, refusal still refuses
6. COMBINATION: held past the bound on a one-worker runtime + 20 concurrent GETs: no stall AND refused
7. COMBINATION: autopilot driver + refused opening + the unchanged autopilot's reading and classifier
8. one shared path: the F&O walk refused the same way
9. OUT OF THE BOX: the journal replaced while the opening waits; neither file gains a record
10. the run key a refused run claimed is released (compile-fail before: new function)
11. the refusal is on the receipt and on `/pull/run.json` (end to end through `/pull/run`)

## Files outside ownership

- `crates/api/src/emitted.rs`: lib emit sites 96 -> 97, `REACHED_IN_SERVER_TESTS` 32 -> 33 (the
  one new `pull.run refused` site, read back by two server tests).
- `crates/api/src/journal_unavailable_tests.rs`: new test file of this package's own, declared by
  one `#[cfg(test)] #[path]` mod line inside the owned region of `server.rs`.
- In `server.rs` outside the RunLedger/opened/journal-call region: `broker_answer`'s and
  `recovery_spot`'s blocked arms (journal calls of async callers of RunLedger: owned per the spec),
  `fno_opened` (now `Result`), the new `fno_unjournalled`, `FnoPage::say_counted`'s one line, the
  new `BrokerRun::refused_at_opening`, `journal_unavailable`, `JOURNAL_UNAVAILABLE`,
  `note_run_refused`, `journal_unavailable_facts`, and the one changed block of the existing F&O
  test above.

## D-numbers used

D-7100, D-7101 (block D-7100..D-7109). D-7102..D-7109 unused.

## Notes for the merge-back

- `broker_run_by` is at clippy's 100-line ceiling minus headroom (it was 107 with the refusal
  inline; the refusal moved into `BrokerRun::refused_at_opening`). IA-10c adds about 3 lines there:
  if clippy's `too_many_lines` fires after that merge, move IA-10c's landing match into a helper.
- IA-10c: on its branch the run key is claimed inside `open_spot_landing`; keep its
  `(landing, claimed_run)` and pass `claimed_run` to `refused_at_opening`; the `LandingRun` drops at
  that `return` and releases the landing claim.
- FM-162 (R4-11's): this proves the api pull paths' run-open half
  (`a_run_whose_opening_is_refused_after_the_bound_asks_the_vendor_nothing_and_lands_nothing`,
  `an_fno_walk_whose_opening_is_refused_asks_nothing_and_appends_nothing`).
- Matrix rows FM-629 / FM-630: tests exist and pass.

## NEEDS THE MAC (spec section 6, unchanged)

Every command and expected answer in the spec's section 6 (gates in pool 5, the shadow on 8515,
real journal size and mtime unchanged, `/pull/run.json` restart word unchanged on the clone, GET
`/feeds.json` under a held lock before/after on the shadow, the harness on a cloned audit dir and
the scan of it finding no open harness run, the refusal on an owned temp root and on `/logs.json`,
NIFTY first then BANKNIFTY, one F&O stock and one F&O walk). Also: coverage
(`cargo llvm-cov --branch`) and `cargo mutants --in-diff` if they cannot be installed here.

## NEXT

When HOLD is gone: (1) run the CI shell gates on the pristine base worktree and on this branch and
compare (`scratchpad/gates/run-gates.sh`); (2) merge the newest `origin/feat/db-fix`; (3) final
gates: fmt, clippy, `cargo test --workspace --locked`, `cargo deny check` (install), coverage and
mutants on the touched modules (install, or NEEDS THE MAC); (4) STATUS: DONE, push.

## Log

- 09:20 IST: read README, CONTEXT, spec; branch created.
- 09:45 IST: 14 of 15 new tests pass; the harness test red before its fix (above).
- 09:52 IST: ef43189b pushed; HOLD; HELD.
- 12:37 IST: HOLD gone; before build run: 13 of 13 red.
- 12:43 IST: 21e75da3 (ledger rows, line-ceiling fix). 12:55 clippy clean.
- 13:15-13:23 IST: measurements (above).
- 13:25 IST: a134f6ee (docs/06); HOLD again (12:56 IST note); gate runs not started; HELD.
