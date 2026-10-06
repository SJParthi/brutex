# PARK — L1 observability lens (2026-10-06 12:35 UTC)

- **Branch:** `attack/observability`
- **Head:** `202a351` (pushed). Base `origin/final/all-fixes` 969493e, merged; no newer base commit at 12:15.
- **Ready to merge:** NOT YET — Gate 18 pre-run on the combined diff is still running (see below). Everything else on 202a351 is validated.

## Pushed and validated on 202a351
Rounds 1 and 2, eleven findings, each with a test recorded failing against the pre-fix code and green after:

| ID | Finding | Where |
|---|---|---|
| OBSV-01 F-ECBE65 | merged /logs page claimed reached_oldest after its limit cut read records | api logs.rs `merged` |
| OBSV-02 F-24EDBD | /masters/status.json reported an unreadable master as absent | api mastersrun.rs + /mapping |
| OBSV-03 F-8AEE77 | cash-session cache reinstall reported durable on the strength of a read | pull cash_session_cache.rs |
| OBSV-04 F-08BBA7 | failed census append named requested, not landed, count | pull ingest.rs `append_locked` |
| OBSV-05 F-6EF086 | live sweep progress refused on log history older than the run; `/logs.json since=` | api logs.rs + web live-progress |
| OBSV-06 F-C27563 | sweep-all refused months left no log event | cli batch.rs |
| OBSV-07 F-B26F07 | refused sweep-stored left no reason in the log | cli lib.rs |
| OBSV-08 F-E995C5 | refused command's `command finished` carried no reason | cli lib.rs `run_with_sink` |
| OBSV-09 F-21160A | /ingest called a taken-but-unpersisted stop undelivered, dropped its warning | web ingest page |
| OBSV-10 F-FC80BC | /audit counted an unreadable older page as held | web audit page |
| OBSV-11 F-DC087B | one refused census row dropped every other row of a rolling batch | pull ingest.rs `record_all` |

Docs: D-3200..D-3210, docs/04-invariants OBSV-01..11, docs/11-findings "Observability lens L1" narrative (REFUTED items with reasons).
Evidence: `cargo fmt --check` clean; `cargo clippy --workspace --all-targets -- -D warnings` exit 0; static gates (language-purity, all but 1e) fail=0; test suites green non-root: api (62d3e3e, unchanged since), pull, cli lib (1947 passed, 0 failed, 1 ignored), core (findings.rs as root: git ownership); web `npm test` 870 pass 0 fail, `npm run check` 0 errors.

## Open
1. **Gate 18 pre-run** on `git diff origin/final/all-fixes...202a351 -- crates`, in worktree /tmp/claude-0/mut, tests filtered to touched modules, order pull → cli → api (api's 6 mutants done before the pause excluded: 5 caught, 1 unviable). Result appended below when it lands.
2. **Round 3 is NOT a zero round.** Survivors not yet refuted/fixed (parked, not started):
   - Web pages that drop the server's stated reason on a non-2xx or render unknown as zero: backtest `matched:null` shown as 0 (coverage.rs counted_from "no master"); catalogue-loader blames the master when the census note names the failure (also ingest measureRoster, feed-summary); frontier-pages, /live.json, sweep-evidence, run.json?attempt= `running.why`, BooleanLaunch/IndexStopLaunch 503 `refusal`. A refuter was running at park time; its verdicts are appended below if it lands.
   - Refuted in round 3: telemetry `Sink::sync` after a roll (no production caller; non-durability documented).
   - The api/pull round-3 audit was still running at park time.

## Exact next step to resume
1. Read the mutation results (`/tmp/claude-0/mutout-{pull,cli,api}/mutants.out/missed.txt`); kill any MISSED with a real test.
2. Fix the round-3 web survivors that the refuter upheld, through `web/src/lib/refusal.js` `refusalFrom`/`refusalOf` (one helper; extend `refusalOf` for nested `running.why` if upheld), each with a failing node test first; D-3211.., OBSV-12...
3. Round 4 with fresh read-only agents; repeat until a zero round; write RESULT-observability.md.

## Landed after park (12:4x UTC)
- Gate 18 partial: pull 5 caught, 2 unviable, 0 missed. cli and api still running.
- Round-3 api/pull audit candidates (NOT yet refuted by a separate agent, NOT fixed; next session refutes first):
  1. /pull/run coordinator (pullrun.rs) emits no event for leg failures, halted feeds, dead chains, retry passes or the final verdict; legs refused before broker_run (seat 409, unknown feed 400, clock 500) bypass `note_request` too, since pullrun calls `server::pull_spot`/`pull_fno` directly.
  2. `broker_run` refusals before `note_run_started` (unreachable broker server.rs~7937, mapping block ~7956, ladder out-of-order ~7999) emit nothing; the autopilot tick journals them but logs nothing.
  3. Autopilot `fly` exits (broker not Live ~2657, clock allowance spent ~2681, no day dir ~2735) and Halt/Stall verdicts are status-only; the spawned task's JoinHandle (server.rs~19973) is never checked, so a panic leaves status frozen "Running".
  4. Expired F&O walk: `FnoLanded::record_refusal` (server.rs~11789) does not emit (its spot peer does); `walk_months` partial-month failures unlogged; `pull::http` `Discovery::get`/`post_json` skip `note_answer`.
  5. Transport-level retries (`with_retry`, `laddered`, http `TransportFailed`) unlogged.
  6. `recovery::activate_durable` BLOCKED reason not logged.
  7. `sweeprun` `descend_with`/`command_with_configuration` stamp refusal and `environment_budget_refusal` not emitted (run_with does emit).

## Refuter verdicts on the round-3 web candidates (landed after park): ALL SEVEN STAND
Not fixed (parked). Fix plan, one shared path where possible; extend D-1789 with a new D-32xx entry:
- W1 backtest `loadSurface`/`coverNote` (~3498, ~3694): keep `matched:null` and `counted_from`; say "swept count not measured (no master)" instead of "0 of N". Test via the parse-and-extract harness of web/tests/live-top-binding.test.js.
- W2 census-unreadable 503 shown as a master message: one header helper (e.g. `censusRefusal(headers)` in web/src/lib/refusal.js) used by catalogue-loader.js:48-50, feed-summary.js:9, ingest +page.svelte:4025. Tests: catalogue-loader.test.js, feed-summary.test.js.
- W3 frontier-pages.js:43-44: `refusalFrom('/frontier.json', response)`; change frontier-pages.test.js:103-107 (it pins the old behaviour) and give the stub `text()`.
- W4 backtest /live.json 503 (~323-328): `refusalFrom('/live.json', response)`; test in live-top-binding.test.js.
- W5 sweep-evidence.js:43: `detailRefusal(response, …)`; test sweep-evidence.test.js.
- W6 run.json?attempt= 503 `running.why`: teach `refusalOf` the `{running:{status:'unknown',why}}` shape, then `refusalFrom` in receipt-batch.js:120, boolean-launch.js:354, index-stop-launch.js:145, backtest pollSweep ~2498. Tests: refusal.test.js, receipt-batch.test.js, boolean-launch.test.js, index-stop-launch.test.js.
- W7 BooleanLaunch.svelte:75, IndexStopLaunch.svelte:43: `refusalFrom(...)` + " No launch was attempted." Tests: index-stop-launch.test.js harness; copy it into boolean-launch-page.test.js.

## Gate 18 pre-run stopped at the 2 h background limit (14:27 UTC)
Combined diff 202a351, tests filtered to touched modules, run in worktree /tmp/claude-0/mut (reset clean afterwards; never committed from).
- pull: caught=5 missed=0 timeout=0 unviable=2
- cli: caught=11 missed=1 timeout=0 unviable=1
  - MISSED: crates/cli/src/lib.rs:2403:32: replace == with != in run_with_sink
- api: caught=5 missed=0 timeout=0 unviable=0
- api before the pause (6 of 22): 5 caught, 1 unviable, 0 missed.
- Not run: the remaining api mutants not reached above. Resume: same command for api with --exclude-re for every mutant already in caught/unviable, with a longer timeout or in chunks (`--shard k/n`).

## Survivor killed (14:39 UTC)
- `crates/cli/src/lib.rs:2403:32 replace == with != in run_with_sink` (MISSED above) is now caught: the OBSV-08 test asserts `phase=refused`. Proof: `cargo mutants --baseline skip --in-place -p cli --file crates/cli/src/lib.rs --re "replace == with != in run_with_sink" -- --lib -- tests::a_refused_command` -> "1 mutant tested in 6m: 1 caught". fmt clean, clippy -p cli -D warnings clean.
- **New head: `838f5e6`** (pushed). Ready to merge: still NO, only because the api Gate 18 pre-run is incomplete (11 of 22 tested: 10 caught, 1 unviable, 0 missed) and round 3 has open survivors (W1-W7 web, seven api/pull candidates).
