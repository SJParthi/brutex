STATUS: DONE

# AG-06b (cloud): pull runs never start unjournalled, and no tokio worker waits on the journal lock

Branch `cloud/db-AG-06b-20261024`, cut from `origin/feat/db-fix` e851da1ca and merged with the
newest `origin/feat/db-fix` 7b397bbe (Merge CLOUD-MUTCLI) before the final gates (the three
append-only ledgers conflicted and were resolved as both sides, in merge order; nothing deleted).
Spec: `reports/db-specs-20261024/AG-06b.md` on `origin/cloud/db-base-20261024`.

Holds: HOLD 09:16 IST (seen 09:52, lifted by 12:37); HOLD 12:56 IST (seen 13:25, lifted by
18:05); HOLD 18:16 IST does not apply to this part: `GO-AG-06b` (19:47 IST) says "build now (HOLD
does not apply to this part)". Every step was committed and pushed at its save point.

Prerequisites: none hard. IA-10c is a FILES_ONLY link, not merged at 7b397bbe: the merge-back
waits for it (notes below).

## Findings

| Id | Status | How |
|---|---|---|
| AG06B-1 | fixed | `BrokerRun::opened` answers the blocked shape (`503`, `journal_unavailable: ...` carrying the journal's own refusal: bound, wait, attempts, record kind and outcome, holder in `holder_note`'s words) when `RunLedger::open`'s `open_run` is refused, and keeps the ledger (so `opened_refused` still reaches the unchanged autopilot's `journal_error`). `broker_run_by` returns it before its first vendor request, through `BrokerRun::refused_at_opening`, which writes one `pull.run` `refused` event (Error, audited) and releases the run key (`note_run_refused`). `broker_answer` and `recovery_spot` append nothing more for such a run; the receipt carries the refusal, `Recorded: NO` and retry class `transient` (so `/pull/run` retries the leg and files the refusal as the feed's `lastError` on `/pull/run.json`). `fno_walk`: `fno_opened` is a precondition (`fno_unjournalled`), same sentence and lines. D-7100. |
| AG06B-2 | fixed | `RunLedger::open`, `close`, `flush` and `rest` (inside the ledger, so every caller including `autopilot.rs` is covered and no pinned call text moves), `fno_opened`'s append, `FnoPage::say_counted`'s record, and the blocked-arm appends of `broker_answer` and `recovery_spot` go through `off_the_workers`. No `spawn_blocking` added. D-7101. |
| AG06B-3 | fixed | `audit_lock_wait_measure.rs`: one `one_run` shared by the ignored measurement and the new test; the close is `Record::of_run` with the opening's source and window. |
| AG06B-4 | done | `docs/05` D-7100, D-7101; `docs/04` AG06B-01..08; `docs/06` L-AG06b-01..07 with the measurements. L-AG06-03/-04 and L-AG06-07 (and L-AG06-08's worker half) are closed there by name, with tests and figures; AG-06's own rows are not edited. |

FM-162 (R4-11's): this proves the run-open half on the api's pull paths
(`a_run_whose_opening_is_refused_after_the_bound_asks_the_vendor_nothing_and_lands_nothing`,
`an_fno_walk_whose_opening_is_refused_asks_nothing_and_appends_nothing`); the ingest command's half
is R4-11's. Matrix rows FM-629 and FM-630: their tests exist and pass.

One existing test changed, with its reason: the last block of
`an_expired_fno_walk_cut_short_is_seen_at_the_next_start` asserted that a walk the journal refused
at its opening still answered "Opened in the journal: NO"; that is the behaviour D-7100 replaces, so
it asserts the refusal, and keeps "nothing was appended". Never weakened:
`a_run_whose_opening_record_was_refused_closes_behind_it_written_late` is unchanged and green, and
the source pins in `the_sweep_withholds_per_instrument_probes_the_store_and_says_the_horizon_once` and
`every_record_of_a_run_names_one_source_and_a_refusal_before_it_closes_none` are untouched and green.

## Gates (final tree: the merge with 7b397bbe plus the last test and docs/04 edit)

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | clean |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | EXIT 0 (`Finished dev profile`), run after the last code edit |
| touched api tests | `cargo test -p api --locked --lib -- journal_unavailable_tests audit::lock_wait emitted:: an_expired_fno_walk_cut_short_is_seen_at_the_next_start the_sweep_withholds_per_instrument_probes_the_store_and_says_the_horizon_once every_record_of_a_run_names_one_source_and_a_refusal_before_it_closes_none a_run_whose_opening_record_was_refused_closes_behind_it_written_late the_run_events_and_the_request_event_reach_the_installed_sink` | **ok. 41 passed; 0 failed; 3 ignored** (the ignored three are the measurements) |
| api lib, whole | `cargo test --workspace --locked` (stops after api lib) | 2,477 passed, 9 failed, 15 ignored. **Every failure is a base red**: the eight that finish were re-run one by one on the base build (e851da1ca + only this test file) and each fails there too: `detail::tests::a_refused_admission_names_what_its_bound_and_why`, `pullrun::restart_tests::the_journals_own_extremes`, `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted` ("this host reads a 000 directory (root?)"), and five `server::head_deadline_body_tests` slow-reader cuts; the ninth, `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`, hangs (its child on a futex, 0 % CPU) and hangs the same on the base build (killed by `timeout 300`, no result) |
| rest of workspace | `cargo test --workspace --exclude api --locked --no-fail-fast` | 243 test binaries; 50 failures in `cli` (lib), `cli` `tests/gdfl_run_logs.rs`, `pull`, `store`, `telemetry` (libs): root ignoring mode bits, the store's free-space floor on this 91 %-full volume, timing, and one base scan of a DATA-2 test file. **Not mine by construction**: none of those crates depends on `api`, this diff changes no file in them, and none of the failing modules reads `docs/04`/`05`/`06` (the one that reads `docs/06`, `selection_v6_source`, passed). Not re-run on a base build here (disk): NEEDS THE MAC |
| api integration + doc | `cargo test -p api --locked --no-fail-fast --test '*'`; `cargo test -p api --locked --doc` | `binary`, `logs_r3_24`, `pool_load`, `state_stream_emits`, `store_step_forms` all ok; `cloud_mutants_bars`: `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held` hung (0 % CPU, 16 min) and was killed: a store read-pool saturation test like the base hang above, on a path this diff does not touch (not run on base here: NEEDS THE MAC). Doc tests EXIT 0 |
| deny | `cargo deny check` (cargo-deny installed here) | **advisories FAILED** on RUSTSEC-2026-0386 (tokio `AsyncFd::new` unsound); bans, licenses, sources ok. **Red identically on the unmodified base** (`cargo deny check advisories` in a pristine 7b397bbe worktree: same id). `Cargo.toml` and `Cargo.lock` left alone (17 Oct cloud rule): NEEDS THE MAC |
| CI shell gates | every `run:` step of `ci.yml`'s `language-purity` job, in order, on a pristine 7b397bbe worktree and on this branch, compared | **Same verdicts on both**: 24 pass; 9 fail, the nine `CONTEXT.md` 4.10 lists as base reds (1c, 1d, 10, 11, 12, 19, 23, 27b, 30). Log diffs base against branch: only counts move (more files, tests, headings), except **gate 23**, which also lists this branch's `api/src/journal_unavailable_tests.rs:println!` (now one site, the measurement print). Gate 23's declared list is in `.github/workflows/ci.yml` (not this part's; BASE-REDS-1023 repairs gate 23): it needs `api/src/journal_unavailable_tests.rs println! 1` declared. Gate 1e (a full workspace build and test in a copy) was not run here (Node is installed in this container, so it would not be the no-front-end premise anyway; this diff adds no build script and no dependency): NEEDS THE MAC |
| coverage, mutants | `cargo llvm-cov --branch`, `cargo mutants --in-diff` | **NOT RUN.** An instrumented build of the workspace does not fit the remaining disk (3.8 GB free of this session's allowance after two full builds), and `cargo mutants` rebuilds the api test target once per mutant (about 8 minutes each here). By hand: every branch the diff adds has a test that fails if it is removed or inverted (the refusal, the precondition in each driver, the facts, the event and the key release, the walk's refusal, the four off-worker sites); `RunLedger::fact`'s `opened_refused` text is now reached only by a ledger that no production path runs. NEEDS THE MAC |

## Failing first (each red before the change, shown in a log)

On the base build (e851da1ca with only this package's test file added), 13 of 13 server tests
failed, each for its own reason: the refused-opening test "a run whose opening the journal refused
is itself refused" (no refusal); both stall tests "a GET waited 956.9 ms" and "861.8 ms"; the shape
test "a ledger method calls the journal on a worker"; the extremes "refused" (no refusal, the
vendor asked); the autopilot test `vendor.asked()` 1 not 0; the F&O test no `journal_unavailable`
on its page; the `/pull/run.json` test HTTP 502 not 503; the read-only test, re-run as uid 65534
from a world-readable copy, "refused" (not refused). The harness test failed on the branch with the
old close: "a harness run reads as unclosed: INTERRUPTED: a pull run for 2025-07-01..2025-07-01 was
opened at journal record 8 ... and has no closing record". Two tests were not run red:
`the_run_key_a_refused_run_claimed_is_released` calls the new `note_run_refused`, so before the
change it does not compile; `a_recovery_unit_refused_at_its_opening_fails_by_name_and_appends_nothing`
was added after the base build was removed for disk (on the base its unit is not refused: it asks
the vendor, and its close is the bound's refusal, not `journal_unavailable`; reasoned, not run).

## Measurements (n = 200 each; method and full table in docs/06, AG-06b section)

Test builds (workspace test profile) in this 4-core container, fresh temp journals of invented
records, never a clone of the operator's.

| Operation | Before p50 / p99 / max (ms) | After p50 / p99 / max (ms) |
|---|---|---|
| `RunLedger::open` uncontended, 3 quiet pairs | 0.302-0.318 / 0.783-0.864 / 1.36-3.18 | 0.244-0.359 / 0.551-1.127 / 1.61-5.73 |
| `RunLedger::flush` (one failure) | 0.153-0.165 / 0.465-0.516 / 0.51-0.65 | 0.128-0.182 / 0.335-0.678 / 0.41-1.45 |
| `RunLedger::close` | 0.298-0.328 / 0.534-1.178 / 0.71-1.22 | 0.245-0.360 / 0.575-1.074 / 0.90-6.73 |
| `Journal::open_run` / `append_member` / `close_run`, 1 writer (AG-06 harness) | 0.319/0.871/2.20, 0.149/0.308/0.34, 0.288/0.596/0.97 | 0.353/0.660/1.44, 0.175/0.294/0.40, 0.349/0.697/1.09 |
| `Journal::open_run`, 8 writers | 4.87 / 5.73 / 5.85, 0 refused | 4.28 / 5.57 / 5.86, 0 refused |
| GET `/feeds.json`, no holder (one-worker runtime, load 3) | 1.24 / 177.7 / 326.4 | 1.57 / 169.2 / 348.5 |
| GET `/feeds.json`, a holder keeps the append lock while runs open | **2,002.5 / 3,110.6 / 3,132.1** | **1.27 / 48.6 / 126.0** |

After/before p50 of the uncontended calls is 0.77 to 1.18 (inside one build's own spread). Every
cost left above O(1) is a docs/06 row: the bound a refused run still waits (L-AG06b-01), the
`off_the_workers` hand-off (L-AG06b-02), and the appends still on a worker outside this region
(L-AG06b-03). Real-store and shadow figures: NOT MEASURED here.

## Extreme cases (each a test; red before, green after, except as stated above)

1. zero wait: refused at the first attempt (the journal's path is a directory), at once
2. asked again at once: refused, then lands and asks the vendor when the holder lets go
3. journal missing and cannot be made (`audit/` is a file)
4. journal read-only (as uid 65534, where mode bits bind)
5. current-thread runtime: every ledger call inline, a refused opening still refuses
6. COMBINATION: held past the bound on a one-worker runtime + 20 concurrent GETs: no stall AND refused
7. COMBINATION: the autopilot driver + a refused opening + the unchanged autopilot's own reading and classifier
8. one shared path: the F&O walk refused the same way, nothing asked, nothing appended
9. the third driver: a recovery unit refused at its opening fails by name, nothing appended
10. OUT OF THE BOX: the journal replaced while the opening waits; neither file gains a record
11. the run key a refused run claimed is released
12. the refusal end to end: on the receipt and as `lastError` on `/pull/run.json` through `/pull/run`

## Files outside ownership

- `crates/api/src/emitted.rs`: lib emit sites 96 -> 97, `REACHED_IN_SERVER_TESTS` 32 -> 33, for the
  one new `pull.run refused` site (read back by two server tests).
- `crates/api/src/journal_unavailable_tests.rs`: new test file of this package's own, declared by one
  `#[cfg(test)] #[path]` mod line inside the owned region of `server.rs`.
- In `server.rs` beyond the RunLedger / `opened()` / journal-call region, kept minimal:
  `broker_answer`'s and `recovery_spot`'s blocked arms (journal calls of async callers of
  RunLedger, owned per the spec), `fno_opened` (now `Result`), the new `fno_unjournalled`,
  `FnoPage::say_counted`'s one line, the new `BrokerRun::refused_at_opening`,
  `journal_unavailable`, `JOURNAL_UNAVAILABLE`, `note_run_refused`, `journal_unavailable_facts`, and
  the one block of the existing F&O test named above.
- Not touched: `audit.rs`, `pullrun.rs`, `autopilot.rs`, `store/src/flock.rs`, `.github/`.

## D-numbers used

D-7100, D-7101 (block D-7100..D-7109). D-7102..D-7109 unused.

## Notes for the merge-back

- **IA-10c merges first.** On IA-10c the run key is claimed inside `open_spot_landing` and returned
  as `(landing, claimed_run)`, before the journal opening. Keep that, and pass its `claimed_run` to
  `out.refused_at_opening(asked, claimed_run)`; the `LandingRun` then drops at that `return` and
  releases the vendor's landing claim. A merge keeps both sides.
- `broker_run_by` is at clippy's `too_many_lines` ceiling with little headroom (107 with the refusal
  inline, under 100 now). If IA-10c's or R4-13's lines push it over at the merge, move their match
  into a helper rather than adding an allow.
- Gate 23 needs `api/src/journal_unavailable_tests.rs println! 1` in its declared list (BASE-REDS-1023's file).
- The `started` event of a refused spot run stays and is counted (L-AG06b-04, D-7100 item 2).

## NEEDS THE MAC

1. Everything in the spec's section 6, as written there: gates in pool 5 with the lock held once;
   `cargo test -p store --test cited_commits --locked`; the release build; the shadow on 8515
   (GET only) with the listener's pid checked; real journal size and mtime unchanged before and
   after every step; `/pull/run.json`'s `restart` unchanged on the cloned journal; GET
   `/feeds.json` n >= 200 with no holder and with a holder plus a run opening, before and after
   (report what the shadow shows; the served runtime has one worker per core); the AG-06 harness on
   a `cp -cR` clone of the shadow's audit dir with `BRUTEX_AG06_MEASURE_ROOT`, then the restart
   scan of that clone finding no open harness run; the refusal on an owned temp root
   (`journal_unavailable`, 1000 ms, attempts, record kind, holder; 0 vendor requests; store
   byte-identical; one `api.audit` Error and one `pull.run refused` event) seen on `/logs.json` and
   `/pull/run.json`; NIFTY first, then BANKNIFTY, one F&O stock and one F&O walk.
2. `cargo deny check`: RUSTSEC-2026-0386 is red on the base too; the lock bump is the Mac's.
3. Coverage (`cargo llvm-cov --branch`) and `cargo mutants --in-diff` on `crates/api/src/server.rs`'s
   touched region, `audit_lock_wait_measure.rs` and `journal_unavailable_tests.rs`.
4. Gate 1e, and the CI shell gates with BASE-REDS-1023 merged.
5. The 50 failures outside `api` and the `cloud_mutants_bars` hang, re-run on the base build to
   confirm they are base reds (argued above by construction, not run here).
6. The ignored measurements, on the Mac's build:
   `cargo test -p api --locked --lib measure_the_ledger_calls_uncontended -- --ignored --nocapture` and
   `cargo test -p api --locked --lib measure_feeds_json_while_a_holder_keeps_the_append_lock -- --ignored --nocapture`.

`store_writes_needed`: none.

## Log

- 09:20 IST: read README, CONTEXT, spec; branch created.
- 09:45 IST: 14 of 15 new tests pass; the harness test red before its fix.
- 09:52 IST: ef43189b pushed; HOLD; HELD.
- 12:37 IST: HOLD gone; base build: 13 of 13 red.
- 12:43 IST: 21e75da3 (ledger rows, line-ceiling fix). 12:55 clippy clean.
- 13:15-13:23 IST: measurements.
- 13:25 IST: a134f6ee; HOLD (12:56 note); HELD.
- 18:05 IST: HOLD gone; merged 7b397bbe (60e8549c); CI shell gates on base and branch: same verdicts.
- 18:14-20:10 IST: disk ran out once (gate-runner temp, 21 GB) and was freed; workspace tests,
  deny, api integration and doc tests; one print site for gate 23; recovery-unit test added.
- 20:20 IST: final clippy and touched tests green; GO-AG-06b says the 18:16 HOLD does not apply;
  STATUS: DONE.
