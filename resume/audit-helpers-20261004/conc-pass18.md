# Concurrency pass 18 (tag conc18): request races in the web front end

Head: /home/claude/wt/zero3 at 1f4de71 (origin/final/all-fixes-zero). Audit only. No source edits.

Theme: for every page under web/src/routes and every shared store under web/src/lib, I found each place where a fetch result is written into state. For each one I checked six things: older-over-newer, a response shown under a selection the user has left, two pollers writing one store, whether an error clears or keeps stale data, optimistic UI left wrong after a failed POST, and double-click guards. Not re-reported: CE-71, CE-79, CE-81. All three are still NOT FIXED, see Verification.

Method: source read of every `ask(` / `request(` / `ask_(` site, 9 routes and 17 lib readers. Two medium findings and one low finding were replayed under Node 22 (ran). For each, the page's own function text is pasted verbatim into a harness. The harness uses the real `lib/store.svelte.js`, `store-census.js`, `ask.js` and `page-requests.js`, with runes stripped to plain values. The transport is a mock that holds and releases responses in a chosen order. Harness: scratchpad `conc18/gen.mjs`, `run-*.mjs`, `ap-run.mjs`.

## Summary

| ID | Sev | Where | One line |
|---|---|---|---|
| conc18-1 | medium | routes/ingest/+page.svelte:6669-6766, 6901-6917, 5444, 8561 | Pull's double-click guard is `phase === 'running'`, but phase is set only after `await snapshot()`. Two presses both proceed. Result: either a false pre-run error, or a second POST that the server refuses, which turns the page to `done` while the run continues. |
| conc18-2 | medium | routes/ingest/+page.svelte:5504-5513 (`netError`, `pollError`, `aborted`) | Three error states appear on 16, 11 and 7 script lines (comments included) and are rendered 0 times. A refused `/pull/run`, a failed Stop POST, a failed run-status poll and a failed pre-run census are all invisible. |
| conc18-3 | low | routes/autopilot/+page.svelte:1267-1323 vs 748-805 | The Pause/Resume POST does not revoke the in-flight 2 s poll. An older GET that lands after the POST reverts the state and the button, and writes a false "state moved from paused to running" into the page trail. |
| conc18-4 | low | routes/backtest/+page.svelte:303-385, 2479, 7247-7320 | `fetchLiveTop` has no token and no run binding. It ignores the `identity` that `/live.json` sends, is never cleared, and an overlapping slower reply can overwrite a newer one. Another run's heap is rendered under the running sweep as "for this run". |
| conc18-5 | low | routes/mapping/+page.svelte:242-292, 700-785 | The constituents crawl result is feed-specific and never cleared on a feed change, and its `feed` field is never rendered. Feed A's join is shown under feed B. |

## Findings

### conc18-1 (medium): the /ingest Pull button accepts a second press during the pre-run snapshot

- **Where:** web/src/routes/ingest/+page.svelte:6669-6672
  ```js
  async function start(e) {
    e?.preventDefault?.();
    showProblems = true;
    if (problems.length > 0 || phase === 'running') return;
  ```
  Then :6719-6735 runs `baseline = await snapshot();` (snapshot is :5611, `await refreshStore()`, a full `/store.json` census with a 30 s ceiling and busy retries). `phase = 'running';` is set only afterwards. The submit button is `disabled={phase === 'running'}` (:8561), so it stays live for the whole snapshot. The source comments measure the census at about 375 ms and 377 KB. `runPull` (:6901-6922, used by each census row's "Pull N" button, :5444/:9626) has the identical shape.
- **Why it is wrong:** two presses inside the snapshot window both get past the guard. Each calls `refreshStore()`, which bumps `store.generation`, so the two census reads are distinct flights. Two outcomes depend only on which census reply lands first:
  1. **First reply first:** the first press's read is superseded (`asked !== key`) and returns without settling `store.state`. `snapshot()` then sees `'reading'` and throws `/store.json could not be read, and it named no reason`. Press 1 sets `netError` ("The store could not be read before starting…"), and press 2 runs normally. The page now holds a false pre-run failure for a run that is running. It is invisible only because of conc18-2.
  2. **Second reply first** (for example, the second read is a fast 304 by etag): both snapshots pass and **two `POST /pull/run` are sent**. The server refuses one (`pullrun.rs:391`/`:421` AlreadyRunning). The loser's branch at :6748-6757 sets `phase = 'done'` and `finishedAt`, while the winner's `watchRun()` keeps polling a running run. `pollRunCurrent` writes `phase` only when `running === false` (:6849), so the page stays `done` for the entire run. The Pull button is re-enabled, the "Clear the result" button appears, and the Stop button disappears (it is gated on `phase === 'running'`). The loser also overwrote `controller` and re-armed `releaseWatch`.
- **Repro (ran):** `node run-first-first.mjs` and `node run-second-first.mjs` (two `start()` calls back to back, one held census each):
  ```
  first-first : phase 'running', posts 1, netError 'The store could not be read before starting … named no reason'
  second-first: phase 'done',    posts 2, serverRunning true, netError 'A run is already in flight on this server.'
  ```
- **Fix:** take a synchronous latch before the first `await` in both `start` and `runPull` (for example, `if (pressing) return; pressing = true; … finally pressing = false`), and disable the button and the row buttons on it. A refusal of `AlreadyRunning` must not set `phase = 'done'` while a watch is live. Better still, route it to `resumeRun()`.

### conc18-2 (medium): /ingest's run errors are written and never rendered

- **Where:** web/src/routes/ingest/+page.svelte:5504 `let netError = $state(null);`, :5513 `let pollError = $state(null);`, :5505 `let aborted = $state(false);`. The template starts at :7418. `awk 'NR>7418' … | grep -c 'netError\|pollError\|aborted'` gives **0**, and no other file names them.
- **Writers that therefore say nothing:**
  - :6753 `netError = answer?.why ?? …` for a refused `/pull/run`. The page shows `phase = 'done'` with `receipt = null`, which looks like a finished run that pulled nothing.
  - :6761 "The run could not be started…".
  - :6726 and :6924 "The store could not be read before starting…". The press appears to do nothing.
  - :7047 a dropped `/pull/spot` leg. The comment at :7063 says "The error is recorded in `netError` and shown", which is false.
  - :6812 `pollError = 'Run status is unknown…'`. The card keeps showing the last good `runState` with no stale label.
  - :7168 `pollError = 'Stop could not be delivered, so the run may still be going…'` and :7178 "Nothing was stopped…". After a failed Stop POST, `stopAsked` reverts to `false`, so the button silently reads "Stop" again with no reason. This is the optimistic-revert case with the label missing.
  - :5646 the census poll error during a run.
- **Why it is wrong:** these are the exact refusals CLAUDE.md §4 requires to be degraded loudly. The handlers were written to name them, but the render was never wired.
- **Repro (ran):** `grep -n 'netError\|pollError\|aborted' web/src/routes/ingest/+page.svelte`. Every hit is at or before :7210 (script); none is in markup. `git log -S'{#if netError'` and `-S'{#if pollError'` show no commit ever rendered them.
- **Fix:** render `netError` and `pollError` in the run card (role="alert") whenever they are non-null, regardless of `phase`. When `pollError` is set, label the progress block as last-known.

### conc18-3 (low): an older /autopilot.json poll overwrites the control POST's newer state

- **Where:** web/src/routes/autopilot/+page.svelte:1267 `async function send(action)`. It POSTs to `/autopilot/control` and at :1311-1314 calls `adopt(parsed.value)` from the POST's `status`. It never cancels or refreshes the `watchVisible(tick, TICK_MS)` reader (:809). `tick` (:748) checks only `ticket.current()`, which is still true for the poll already in flight.
- **Why it is wrong:** suppose the GET was answered by the server before the control was applied, but its body resolves after the POST's. Then `adopt` runs with the older state. The offer button flips back (Pause becomes Resume and back again), and `adopt` writes a false transition into the page's own trail, which is that page's "what happened while I was away" record. It self-heals on the next 2 s tick, which writes a second false transition.
- **Repro (ran):** `node ap-run.mjs` (verbatim `readState`, `note`, `adopt`, `setLink`, `tick`, `send`, plus the real `createPageRequests`):
  ```
  after POST: paused | receipt: done
  after older poll lands: running
  trail: 'state moved from paused to running.', 'state moved from running to paused — paused by operator', …
  ```
- **Fix:** in `send`, after the POST settles, revoke the current poll and start a fresh one (keep the handle that `watchVisible` returns and call its `.refresh`). Alternatively, drop any GET whose request started before the last POST's response.

### conc18-4 (low): the live "Best combinations so far" panel is unbound to the run and untokened

- **Where:** web/src/routes/backtest/+page.svelte:303-385 `fetchLiveTop()`. It is called fire-and-forget on every running poll (:2479), and nothing awaits it before the next poll is scheduled 2 s later. It has no sequence and no gate. `liveTop` is assigned only inside this function and is never reset when a sweep starts or ends. It picks `fresh[0]`, the smallest `idle_secs` across **all** runs in `/live.json`, and ignores each run's `identity` (crates/api/src/livejson.rs:179 sends `"identity"`). The comment at :338-340 chooses this on purpose: "an older run with real rows sat right behind it".
- **Why it is wrong:**
  - A new sweep's panel opens with the previous `liveTop` (another run's rows). It then keeps showing whichever run's heap is freshest. Once the new heap is non-empty that is usually this run, but until then it is an older one, possibly another instrument. Meanwhile the verdict at :7314 says "They beat the evidence bar … **for this run**".
  - A `/live.json` reply slower than 2 s (the ceiling is 15 s) overlaps the next call, and the older reply can land last.
  - It is labelled only by "updated Ns ago" and a "not moving" pill that starts at 120 s.
- **Repro:** not run. The control flow is read from source, and the `identity` field was confirmed in livejson.rs:179.
- **Fix:** keep a `liveTopSeq` like `liveSeq` and drop replies that are not the latest. Filter `runs` to `identity === sweep.run.identity`, or when the identity is unknown, render the chosen run's identity and drop "for this run". Reset `liveTop` in `invalidateLive(true)`.

### conc18-5 (low): the constituents crawl result outlives a feed change, unlabelled

- **Where:** web/src/routes/mapping/+page.svelte:243-292 `resolveUniverse()` captures `feed = feeds.active` and stores `crawl = { phase: 'done', body }`. Nothing clears `crawl` when `feeds.active` changes; the only effect on the feed is `fetchJoin` (:376-380). The render (:705-785) shows published names, unlinked, buckets, publishable, digest, key, identity and day, but never `crawl.body.feed`. The server does send it (server.rs, the `"feed":{}` in the `/universe/resolve` success line).
- **Why it is wrong:** the section says this crawl "checks every published name against this feed's master". After a switch, the previous feed's join counts and its publishable verdict sit under the new feed. A switch made during the 5-minute crawl lands the old feed's result straight into the new feed's view.
- **Repro:** not run (source read).
- **Fix:** reset `crawl` to idle in the feed effect. Alternatively, keep the asked feed and render `crawl.body.feed`, and hide the result when it is not `feeds.active`.

## Fetch-site table

Key to the verdicts: **ok** means token or abort plus identity checks (or an effect-cleanup `live` flag), so a late or left-behind reply cannot publish.

| Site | Writes | Verdict |
|---|---|---|
| lib/store.svelte.js:463-520 `read` (shared census) | `store.*` | ok. `asked` key and `flightToken`. The error clears the value and sets `state='error'` (labelled). One poller (`watchStore`, a single `tick` loop); `/ingest` and `/autopilot` share it, and the effective period is the finest holder's. |
| lib/store.svelte.js:826-880 `surveyStores` | `survey.*` | ok for publishing (`surveyAsked` key). The unconditional `.finally(() => surveyFlight = null)` lets an older flight null a newer one, so a same-key re-call resolves early with `state 'reading'`. Harmless today because `/db:800` does not await it. |
| lib/store-census.js `createCensusLoader` | cache | ok. An older flight is not aborted when superseded (bandwidth only). |
| lib/feed-startup.js `loadNow` | `feeds.all/active/error` | ok. `selectionRevision` guards the default pick. A failed forced reload keeps the old list with `error` set. |
| lib/catalogue-loader.js | `catalogue.*` | ok. `active !== current` plus abort. The error clears. |
| lib/runtime-inspection.js | inspection | ok. Single flight, one-shot. |
| lib/terminal.svelte.js `quote`/`monthBars` + routes/terminal:640-850 | `days`, `times`, `quotes` | ok. Each effect has a `live` flag and abort, and `quotes` is cleared on a window change. A failure result is cached in `inflight`/`months` until `store.generation` moves (labelled with its `why`, but no retry while mounted). |
| routes/+layout.svelte:640 probe | `api` | ok (`watchVisible` ticket). |
| routes/gaps:242 | `result` | ok. Ticket plus `asked === questionKey`, reset on a question change. |
| routes/audit:328/420 | `payload`, `samples`, `older` | ok. Ticket plus `feeds.active === feed`, and a feed change resets everything (:541-551). The error keeps `payload` with `load.state='error'` (labelled). |
| routes/markets:1028 | `rawBars` | ok. `chartRequests` ticket. Loading shows the 'wait' reason, not old bars. |
| routes/db:4534/4652 bar grid | `barState` | ok (`barToken` plus `dead`). Old files are kept while loading by design. A feed change empties the plan first. |
| routes/mapping:111 masters, :321 indexmap | `onDisk`, `load` | ok (tickets plus feed check). The swallowed 503 is CE-82. |
| routes/mapping:253 crawl POST | `crawl` | **conc18-5**. The double press is guarded (`phase==='running'`). |
| routes/mapping:153 refresh POST | `masters` | ok. The double press is guarded. |
| routes/autopilot:751 poll | `ap`, `link` | ok against itself (ticket). See conc18-3 versus the POST. |
| routes/autopilot:1271 control POST | `ap`, `receipt` | **conc18-3**. The double press is guarded by `disabled={control.busy}`, which is set synchronously. |
| routes/ingest:1061 calendar | `calendar` | CE-71 (not re-reported, still open). |
| routes/ingest:1650 folder | `folderReach` | ok (effect `live`). |
| routes/ingest:4100 roster | `roster` | ok. The `busy` latch and `roster.key` are compared by `rosterFresh`. |
| routes/ingest:4649 pilot | `pilot` | CE-79 (still open). |
| routes/ingest:6629 resume, :6799 run poll | `runState`, `phase` | ok (`resumeRequests` and `watchVisible` tickets). The error goes to `pollError` (conc18-2). |
| routes/ingest:6740 POST /pull/run, :7013 /pull/spot | `phase`, `receipt(s)`, `netError` | **conc18-1**, **conc18-2**. |
| routes/ingest:7163 stop POST | `stopAsked`, `pollError` | Optimistic `stopAsked` reverts on failure, but the reason is unrendered (conc18-2). |
| routes/backtest:305 /live.json | `liveTop` | **conc18-4**. |
| routes/backtest:1085 board, :1292 trades, combos | `board`, `tradeList` | ok (`boardSeq` + abort, `tradeGate`, `comboGate`). |
| routes/backtest:1971 /logs.json | `live` | ok (`liveSeq` + `liveRunKey`). |
| routes/backtest:2074 /vocab.json | `vocab` | One-shot on mount; no competing writer. |
| routes/backtest:2145 /engine/top.json | `top` | No token. Only called on a `done` poll for that run's feed and underlying. An older run's slow reply could replace a newer run's only if it outlasts a whole second sweep. Not filed: there is no realistic window, and the report text carries its own banner. |
| routes/backtest:2179 /backtest.json | `load` | ok (`ledgerSeq` + abort). |
| routes/backtest:2387/2435/2611 status polls | `sweep`, `launchAdmission` | ok (`statusRequests` single flight, tickets). `pollSweep` honours `unconfirmedSubmission`. |
| routes/backtest:2552/2846 launch and descend POSTs | `sweep` | ok. `unconfirmedSubmission = true` and `phase 'starting'` are set before the await, `launchStop` blocks a second press, and status reads are cancelled first. |
| routes/backtest:3314 census, :3417 universes | `catalog`, `sweptSurface` | ok (`catalogGate` keyed on the feed). |
| routes/backtest:4609 series, :4770 rungs, :4875 bench | `series`, `storeRungs`, `bench` | ok (`seriesSeq`, `rungsSeq`, `benchSeq`). They are cleared before reload (:4672-4705). |
| lib/SweepEvidence, CandidateTrades, ExpressionSearch, BooleanCatalog, BooleanEvidence, BooleanLater | `loaded`, `trades` | ok (generation counters, effect cleanup). |
| lib/InvocationAudit, QualifiedCampaign, BooleanCampaign, BooleanQualifiedSearch (campaign-monitor) | `loaded`, `detail` | ok. The monitor checks `active !== ticket` and `sequence` monotonicity. The error keeps `latest` with `why` (labelled). |
| lib/boolean-launch.js, index-stop-launch.js, receipt-batch.js POSTs | launch state | ok. `busy`/`postAbort`/`active` latch synchronously and an unconfirmed response blocks a resend. |
| lib/index-stop-{results,source,vix,ranking,qualification}.js, index-consistency.js, research-tester.js | readers | ok (`createPageRequests` tickets or a generation, plus abort). |
| lib/BooleanLaunch.svelte:74, IndexStopLaunch.svelte:43 config | `config` | ok (generation plus abort). |

## Verification of the excluded rows (still open at 1f4de71)

| Row | Status | Evidence |
|---|---|---|
| CE-71 | NOT FIXED | routes/ingest/+page.svelte:1058-1061 `loadCalendar(feed)` still fetches with no token or signal and writes `calendar` unconditionally. |
| CE-79 | NOT FIXED | routes/ingest/+page.svelte:4661 `pilot = { ...pilot, busy: false, error: String(why) };`. `pilot.error` still has no render site. |
| CE-81 | NOT FIXED | routes/ingest/+page.svelte:8789 `class:up={f.finished \|\| (f.legsDone ?? 0) >= (f.legs ?? 0)}` is unchanged. |
