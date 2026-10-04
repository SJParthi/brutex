# conc-pass15: the autopilot as a state machine, at 1f4de71

**Verdict (head 1f4de71, final/all-fixes-zero, read-only checkout /home/claude/wt/zero3):** 6 new findings (0 high, 1 medium, 5 low): conc15-1 to conc15-6. The new findings come from one structural fact: **per-feed state is held per rung (`Place`, D-0949), but the loop, the verdicts and the published status are global.** `fly` alternates day and minute rounds, and each round's "nothing chosen" verdict, nap, retry timing and report table describe only its own rung. This pass found no state that has no exit and is undocumented. Every terminal state (credential or configuration halt, store probe allowance spent, clock wait spent, stall allowance spent) is named on the page and documented as needing a restart. The known items conc11-2, conc12-1, conc13-4 and CE-84 are each confirmed by the table below. All 13 known rows re-checked here are in the same state as before: 2 FIXED and 11 NOT FIXED.

Method: source reading only. No cargo was run. Only conc15-1 is medium, and its premise is the plain control flow of `fly` and `round`. Files read: `api/src/autopilot.rs` (all production code, :1-4075), `server.rs` `broker_run` (:7900-8047), the `fly` spawn and shutdown (:19160-19268), `ingest.rs` `status_json`/`waiting_json`/`blocked_by` (:1945-2120), and `web/src/routes/autopilot/+page.svelte` (the fields it reads). Before filing, I checked every finding against concurrency.md, conc-pass1/autopilot.md, conc-pass2/{clock,recauto,press,lifecycle}.md, conc-pass6, 11, 12, 13 and 14, crash-edge*.md, tests-docs-security*.md and triage-20261004.tsv.

---

## 1. States

### Global (one `fly` task; `Control.paused`, `Control.epoch`, `Control.seats`, `Status.phase`)

| state | how it is represented | published word |
|---|---|---|
| G0 boot-paused | `Control::serving` → `pause()` unless `BRUTEX_AUTOPILOT=run` (autopilot.rs:2113-2119) | starting (Default) until `fly` reaches the loop, then paused |
| G1 pre-loop returned | `fly` returns: no live broker :2587-2596, clock allowance spent :2612-2625, no day directory :2664-2669 | halted, `feeds` empty |
| G2 clock wait (alive) | :2626-2634 loop of `IDLE_POLL_SECS` sleeps | halted, `feeds` empty (autopilot-3/CE-46) |
| G3 grace | `grace` :2766-2786 | starting |
| G4 operator-paused | `paused == true`; `dwell_paused` :2752-2763 every 1 s | paused |
| G5 standing off | `round` :3244-3250 → `stand_off` :3182-3194, 1 s | **paused** (same word as G4) |
| G6 ticking | `round` chosen branch :3332-3386, `broker_run` sets `now` | running |
| G7 backoff nap | `settle` Wait :3397-3411 → `nap(secs)` | waiting |
| G8 idle (rung caught up) | `round` idle branch :3264-3330, `Settled::Complete` | complete |
| G9 idle, every feed halted / no universe | :3317-3321 | halted |
| G10 reconsidering | idle branch with `reconsider` → `carry_on` | complete, then wait 0 |

### Per feed, per rung (`FeedState` + `parked: Place`)

| state | fields | exit |
|---|---|---|
| F-live | `halted None`, `attempts < 3`, `dry < 2` | observe |
| F-backoff | `attempts 1..2`, `backoff n` | next same-rung tick |
| F-store-1 | `store_refused = true` (one store refusal) | progress, complete or a second refusal |
| F-stalled-month | `stalls[(month,rung)]`, `retried < 2` | `reconsider` from an idle branch, ≥6 h (wall clock, CE-84) |
| F-stall-spent | `retried == STALL_RETRIES` | restart only (documented) |
| H-credential / H-configuration | `halt_kind` Credential/Configuration, `probe None` | restart only (documented, §8) |
| H-store | `halt_kind Store`, `probe made < 8` | `probe_store_halts` success → `revive` |
| H-store-spent | `probe.made == 8` | restart only (documented) |
| H-census | `halt_kind Census` | `survey` sees `Census::Held` → `revive` |

## 2. Inputs

tick success (complete / partial `stored>0`) · empty reply (`stored 0`, no reason) · vendor error (reason, classified by text) · store error (reason containing a STORE marker) · credential verdict (`run.credential_stop`) · breaker or operator stop (`run.stopped`) · operator stop / start / resume (`act`) · restart · disk probe result · census load result · manual pull (seat or `site.run`) · clock (`yesterday_ist`, `epoch_secs`) · masters reparse (`SeriesCache` generation).

## 3. Transition table (from code)

| from | input | to | where | note |
|---|---|---|---|---|
| any rung | `credential = SameValue` | H-credential, Halt | observe :1356-1370 | first arm, outranks progress |
| any rung | `Unreadable(Configuration)` | H-configuration | :1371-1384 | |
| any rung | `Unreadable(Transport)` | falls through to the transport arm | :1385-1387 | intended |
| any rung | `run.stopped` (operator **or breaker**) | Retry, no attempt | :1389-1391, outcome_of :3631 | autopilot-2, NOT FIXED |
| any | `complete` | unstall, clear, `months_done+1`, Advance | :1392-1397 | conc15-6 |
| any | `stored > 0` | clear_month, Retry (0 s) | :1398-1401 | |
| F-store-1 | store reason | H-store | :1403-1413 | **conc15-3**: `store_refused` survives `revive` on the parked rung |
| F-live | store / transport reason | attempts+1, Wait backoff | :1415-1428 | class by text: **conc11-2 confirmed** |
| attempts = 3 | reason | stall recorded, Stall, frontier+1 | :1417-1425, settle :3412-3424 | settle sets no phase (cosmetic) |
| F-live | nothing stored, no reason | dry+1; at 2 Advance | :1430-1436 | **conc12-1 confirmed**: per rung, restart rebuilds at the floor (:2673) |
| Wait n | — | global `nap(n)`, then the **other** rung's round | fly :2711-2728 | **conc15-2** |
| Retry | — | 0 s, then the **other** rung's round | fly :2711-2722 | **conc15-2** |
| G6 | this rung has nothing chosen | G8 (complete) and nap 60 s, or G10 | round :3264-3330 | **conc15-1** |
| H-store | probe due and OK | revive (live rung only) | :2985-2999 | conc15-3; timer is wall clock: **CE-84 confirmed** (:2974) |
| H-store | probe fails ×8 | H-store-spent | :3001-3027, store_due :541-551 | documented terminal |
| H-census | census Held | revive | survey :2849-2858 | Absent does not revive (intended) |
| F-stalled-month | idle branch, 6 h (wall clock) | frontier back on that rung | reconsider :1483-1537 | CE-84 confirmed (:3280); see conc15-1 |
| Halt (settle) | — | phase Halted, `feeds` stale, nap 60 | :3425-3431 | conc6-1 NOT FIXED; not logged: **conc13-4 confirmed** |
| any | operator stop | `paused`, epoch+1; phase Paused unless Halted | stop :4037-4055 | then `dwell_paused` overwrites Halted within 1 s (conc6-1) |
| G4/G5/G8/G9 | operator resume | `admit_resume` reads only per-feed halts | :3757-3800, start :4057-4072 | **conc15-5** |
| G2 | operator resume | refused "has returned" | :3767-3778 | autopilot-3/CE-46 NOT FIXED |
| loop top | stop before `broker_run` epoch capture | the month is fetched anyway | fly :2703, server.rs:7917 | autopilot-1 NOT FIXED |
| G6 | Ctrl-C | `flying.abort()` mid-tick | server.rs:19267 | autopilot-4 NOT FIXED |
| G6 | tick ends | journal record; `journal_error` overwritten | tick :3567, round :3373-3377 | **conc15-4** |
| restart | — | every in-memory state lost; pause re-derived from env | module doc :62-68 | documented; stalls and halts lost: conc13-4 |
| round | clock unusable mid-life | `return IDLE_POLL_SECS`, no publish | :3251-3253 | `ist_day` fails only for a clock before 1970 or past 9999; not filed |

### Hunt checklist

- **State with no exit:** none that is undocumented. The credential, configuration, probe-spent, stall-spent and clock-spent states are terminal by design and named.
- **Input falling to a default:** text classification defaults to Transport (conc11-2). A breaker stop is read as an operator stop (autopilot-2). `Next::Stall` publishes no phase (cosmetic, not filed).
- **Two inputs in one tick, where order matters:** a stop and completion. `stopped` outranks `complete` (:1389 before :1392), so a month that finished while a stop landed is Retry, not Advance. This is benign because the next survey skips it. A probe-revive and the rung parity give conc15-3.
- **Operator vs machine:** a resume is admitted over the global halts that are not per-feed (NoUniverse), and over a machine standoff shown as "paused" (conc15-5). `dwell_paused` overwrites a machine halt with Paused (conc6-1). No machine event clears `paused`: `resume()` is called only from `start`.
- **Counters:** `months_done` is counted once per rung and again every day for the current month (conc15-6). `store_refused`, `attempts` and `backoff` survive `revive` on the parked rung (conc15-3). `journal_error` is reset too often (conc15-4).
- **Persisted vs memory:** nothing is persisted, and the module doc (:62-68) says so. Restart loses stalls and halts (conc13-4) and re-fetches months that end on a non-session day (conc12-1).
- **Journal vs /autopilot.json:** conc15-4. Halt, stall and backoff decisions reach neither the journal nor telemetry (conc13-4).

---

## New findings

### conc15-1 (medium): a round whose own rung has nothing missing publishes "complete … The store is complete through the newest finished day" and naps 60 s, while the other rung still owes months. While one rung lags, this alternates with every tick of the rung still backfilling

- **Where:** `crates/api/src/autopilot.rs:2711-2728` (`fly`: one rung per iteration, then `nap(waited)`), `:3262-3330` (`round` idle branch decides over `series` and `state.frontier` of the live rung only), `:3119-3123` (`Settled::Complete` sentence), `:3317-3321` (phase Idle), `:3330` (`IDLE_POLL_SECS`). Doc claim: `:1454-1457`.
- **Code:**
  ```rust
  let rung = rung_for(next_rung);
  next_rung = next_rung.wrapping_add(1);
  ...
  let waited = round(&site, &mut feeds, rung_series, rung).await;
  ...
  if waited > 0 { nap(&site, waited).await; }
  ```
  ```rust
  "nothing is missing that any feed can still be asked for, across \
   {instruments} tracked instrument(s). The store is complete through the \
   newest finished day; ..."
  ...
  return if carry_on { 0 } else { IDLE_POLL_SECS };
  ```
- **Why it is wrong:** `survey` receives the live rung's series and frontier only (`enter(granularity)` at :3206). When the day rung is caught up and the minute rung is not, every day round takes the idle branch. In that case:
  1. The page shows `state:"complete"` with the sentence above. `/ingest/status.json` `waiting_on` shows the day rung's rows (`behind 0`). Neither `FeedReport` nor that row carries a rung field, and `target.timeframe` still names the last ticked rung, because the idle branch does not set `target`.
  2. It returns 60 s, so `fly` naps a full minute before the next minute tick. The minute backfill therefore pays a 60 s idle gap per tick, which is a large fraction of a tick that asks ~765 instruments.
  3. `reconsider` runs from that "nothing is missing" branch. It can move the minute rung's frontier back (`state.parked = Place::at(month)`, :1528) while the minute rung has forward work. This contradicts :1454-1457 ("it can never delay forward progress").

  This is the CE-23 class of claim, "complete" while work is owed, reached through the rung split rather than the frontier clamp. A lagging day rung is the mirror case.
- **When it happens:** whenever the day rung is ahead of the minute rung. That happens when a minute month needs more than one tick (any failed instrument, a stop, or partial progress), after a restart where daily history is already held (the module's own note says operators pull daily months by hand), or when the minute rung stalls.
- **Repro (not run):** use a store whose dhan 1day files are held through yesterday and whose 1min directory is empty. Run `BRUTEX_AUTOPILOT=run api serve` and poll `/autopilot.json` every 2 s. After grace, the first round (Day1, `rung_for(0)`) publishes `"state":"complete"` with "The store is complete through the newest finished day", and `due_unix` counts 60 s. Then a minute tick runs (`"running"`), then "complete" for 60 s again, and so on.
- **Minimal fix:** make the idle verdict global. In `round`'s idle branch, return 0 (not `IDLE_POLL_SECS`) and do not publish `Complete` unless the parked rung is also caught up. For example, keep a per-feed `caught_up: [bool; 2]` set by the round of each rung, publish Idle and nap only when both rungs are caught up, and call `reconsider` only then. Add `"timeframe"` to `FeedReport` and its JSON.

### conc15-2 (low): `Next::Wait` and `Next::Retry` are per-rung decisions that a global loop carries out. The wait blocks every feed and both rungs, and the "same month" is retried only after the other rung's whole tick, not "in {secs}s"

- **Where:** autopilot.rs:3397-3410 (`settle` Wait publishes "retrying the same month in {secs}s", `due_unix = now + secs`), :840 (`Retry`: "Ask again for the same month, straight away"), and fly :2711-2728.
- **Code:** `"{month} did not complete. Attempt {attempts} of {MAX_MONTH_ATTEMPTS}; retrying the same month in {secs}s."` followed by `nap(&site, waited)`, after which `rung_for(next_rung)` picks the other rung.
- **Why it is wrong:** after the nap, the next round is the other rung's. It surveys the oldest month on that rung across all feeds and runs a full tick, which can take minutes. The backed-off month is asked again only on the round after that. The countdown therefore points at an action that does not happen at that time. The other rung's independent work is held up by a backoff it did not earn.
- **Repro (not run):** make Groww's day month M fail once with a transport error while the minute rung owes months. `/autopilot.json` shows `waiting`, "retrying the same month in 30s" and `due_unix` T+30. At T+30, `now` shows a 1min instrument on another month. M's 1day is asked only after that tick ends.
- **Minimal fix:** after a Wait, record `retry_at` on the rung's `Place` and let `fly` keep alternating with no global nap. A round skips a feed or rung whose `retry_at` is in the future (monotonic `Instant`), and the loop sleeps only when every rung is waiting. Alternatively, reword the detail to "retried on this rung's next round, at the earliest in {secs}s".

### conc15-3 (low): `revive` clears only the live rung's counters, and the store halt always revives on the other rung. The halting rung keeps `store_refused = true`, so its first refusal after revival halts again with the false sentence "the store refused the same write twice"

- **Where:** autopilot.rs:1285-1290 (`revive` → `clear_month`, live rung only), :1249-1254 (`clear_month`), :1177-1200 (`enter` swaps `store_refused` in and out of `parked`), :1403-1415 (observe), :3425-3431 (Halt → `IDLE_POLL_SECS`), :2985-2999 (probe success → revive).
- **Code:**
  ```rust
  fn revive(&mut self) {
      self.halted = None; self.halt_kind = None; self.probe = None;
      self.clear_month();
  }
  ...
  if store && self.store_refused {   // "the store refused the same write twice"
  ```
- **Why it is wrong:** the halt happens on rung X with `store_refused = true`. Observe returns before resetting it. `settle` returns 60 s, and the next round is rung Y. `round` calls `enter(Y)` for every feed, which parks X's `Place` with `store_refused: true, attempts: 1, backoff: 1`. The first probe is due at once (`due_unix: 0`), so it runs in this Y round, and `revive` clears Y's counters only. On X's next round, `enter(X)` restores `store_refused = true`, and **one** new store refusal halts the feed again with "refused the same write twice". This is the defect Z1-slice11-F2/D-1762 fixed (:1082-1087), reached again through rung parity. The effect is bounded, because the re-halt arms a fresh 8-probe allowance, but the sentence is false and the second-chance rule is skipped. A Census revive (`survey` :2851) has the same parity for `attempts` and `backoff`.
- **Repro (not run; a pure unit test can drive it):** `let mut s = FeedState::new(Feed::Groww, v, m); s.enter(Day1);` observe a store refusal twice (→ Halt), `s.enter(Minute1); s.revive(); s.enter(Day1);` then observe one store refusal: `Next::Halt` with "refused the same write twice".
- **Minimal fix:** in `revive`, also reset the parked place's counters: `self.parked = Place::at(self.parked.frontier);`.

### conc15-4 (low): `journal_error` is overwritten with "" by the next tick whose record lands. A hole in the audit journal then disappears from `/autopilot.json`, which contradicts the field's own contract

- **Where:** autopilot.rs:1610-1617 (doc: "Empty when every record this process wrote landed."), :3369-3377 (`status.journal_error = journal_error;`, unconditional), tick :3567.
- **Code:**
  ```rust
  let journal_error = out.journal_error.clone().unwrap_or_default();
  site.autopilot.publish(move |status| {
      status.bars_stored = status.bars_stored.saturating_add(stored);
      status.journal_error = journal_error;
  });
  ```
- **Why it is wrong:** the journal's view and the status's view diverge. Suppose tick N's record is refused (for example, the `try_lock` collision with a recovery receipt, recauto-2, or a transient EIO) and tick N+1's lands. `/autopilot.json` then carries `"journal_error":""`, which by its own documentation means every record landed. Tick N's bars are on disk, `/audit` has no row for them, and `ticks`/`bars_stored` count a tick the journal never saw. Nothing durable or visible records the gap, and the halt/stall logs that could stand in for it do not exist (conc13-4). The comment at :3369-3372 chose "current state" deliberately, but the field doc and the page's claim that "the journal is the durable record" were not changed to match.
- **Repro (not run):** hold `<store>/audit/pull.journal`'s flock from another process for one tick, then release it. During the next tick, `/autopilot.json` shows `journal_error` non-empty. After that tick, it is `""`, and `/audit` lacks the first tick's row.
- **Minimal fix:** keep a sticky `journal_lost: u64` count, and the first lost tick's source and reason, beside the current error. Never clear it in-process. Emit both in the JSON, and fix the doc at :1610-1611.

### conc15-5 (low): `admit_resume` reads only per-feed halts, so Resume is admitted, and "resumed. The next unit is whatever the store is missing" is painted, over the global no-universe halt, over a "complete" idle and over a machine standoff shown as "paused"

- **Where:** autopilot.rs:3757-3800 (`admit_resume`: with `feeds` non-empty, only `feed.halted` is consulted), :4057-4072 (`start` publishes `Phase::Running`), :3317-3324 (NoUniverse: phase Halted, `status.feeds = reports` with no feed halted, then a 60 s nap), :3182-3194 (`stand_off` publishes `Phase::Paused`).
- **Why it is wrong:** for the empty-masters halt, the feed rows are present and none is halted, so Resume returns 200 with "started". `start` sets phase Running (shown as `waiting`) with the "resumed" sentence over the "NOT COMPLETE — NOTHING IS TRACKED" reason. `nap` ignores a resume (it checks only `is_paused`), so that false text stands for up to 60 s, until the next round republishes Halted. `RESUME_CANNOT_CLEAR` exists to refuse exactly such a resume, one that changes nothing. In the same way, while a hand pull holds a seat, the page says `paused` (G5) and Resume answers "started" while the next round stands off again. This is not conc6-1: there `feeds` is stale after a per-feed halt. Here `feeds` is fresh and the halt is global.
- **Repro (not run):** start with an unreadable masters directory and `BRUTEX_AUTOPILOT=run`. After grace, `/autopilot.json` shows `halted` "NOTHING IS TRACKED". POST `/autopilot/control action=resume` gets 200 `accepted:true` "started. The next unit is whatever the store is missing". `/autopilot.json` then says `waiting`/"resumed" until the 60 s nap ends.
- **Minimal fix:** in `admit_resume`, return `Refused` when `!is_paused && status.phase == Phase::Halted`, whatever `feeds` holds. When not paused, answer "not paused; nothing to resume" without publishing. Give the standoff its own phase or flag, so that it is not `paused`.

### conc15-6 (low): `months_done` ("Months retired since this process started", against `months_total`) counts each month once per rung and again every day for the current month, so it passes `months_total`

- **Where:** autopilot.rs:1134 and :1673 (docs), :1392-1396 and :1430-1434 (`months_done += 1` on complete and on dry Advance; the field is on `FeedState`, not on `Place`, so both rungs add to it), :2515-2520 (`months_total` counts each month once). Emitted at :1820ff and in `/ingest/status.json` `waiting_on` (ingest.rs:2054-2065).
- **Why it is wrong:** a full backfill of N months ends with `months_done ≈ 2N`. Each day after that, both rungs complete the current month again (`frontier` clamps back to yesterday's month, :2483-2489), adding 2 per day. The pair reads as progress (`done` of `total`) and cannot be one. A dry Advance also counts as "retired". No page renders the pair today (web reads `feed`, `vendor` and `halted` only), so the effect is limited to the API contract.
- **Repro (not run):** let the autopilot catch one feed up and leave it for a day. `/autopilot.json` `feeds[].months_done` exceeds `months_total`.
- **Minimal fix:** move `months_done` into `Place`, so it is per rung, and count only an Advance whose month was not completed before. Alternatively, drop the field and report `store_months` as the progress fact.

---

## Verification of known rows (state at 1f4de71)

| id | state | evidence |
|---|---|---|
| autopilot-1 | NOT FIXED | fly :2703 `is_paused()`; server.rs:7917 `let epoch = site.autopilot.epoch();` captured after the census and ladder; all Relaxed (:2072-2073) |
| autopilot-2 | NOT FIXED | :3631 `stopped: run.stopped.is_some() && run.credential_stop.is_none()`; breaker sets `out.stopped` at server.rs:8030-8036; observe :1389 Retry |
| autopilot-3 / CE-46 | NOT FIXED | clock wait publishes Halted with empty feeds :2626-2634; `admit_resume` :3766-3778 refuses as "returned" |
| autopilot-4 | NOT FIXED | server.rs:19267 `flying.abort();`, with no pause or bounded await |
| conc6-1 | NOT FIXED | settle Halt :3425-3431 sets phase and detail only; `dwell_paused` :2754 overwrites Halted |
| conc6-2 | NOT FIXED | server.rs:7917 hand walks capture the autopilot epoch |
| conc6-4 | NOT FIXED | :3244-3245 "a hand-made pull is running" for any `site.run` holder |
| CE-23 | FIXED | `frontier` clamps to yesterday's month at :2483-2489 and :2501-2507 |
| CE-24 | FIXED | `stop` keeps Halted at :4040-4042 (in the alive-task case, `dwell_paused` still overwrites it: conc6-1) |
| conc11-2 (confirm) | CONFIRMED, NOT FIXED | `classify` text markers :385-399; observe :1403 |
| conc12-1 (confirm) | CONFIRMED, NOT FIXED | dry arm :1430-1436 per rung; restart rebuilds at the floor via `drivable` :2673 |
| conc13-4 (confirm) | CONFIRMED, NOT FIXED | the only `telemetry::` calls before :4075 are pause and resume (:2078, :2089); settle, stall and backoff emit nothing |
| CE-84 / clock-3 (confirm) | CONFIRMED, NOT FIXED | `probe_store_halts` :2974 and the `reconsider` stamp :3280 use `SystemTime` epoch seconds |

Tally: 13 checked: 2 FIXED (CE-23, CE-24), 11 NOT FIXED. That is 7 NOT FIXED rows plus the 4 known items confirmed.
