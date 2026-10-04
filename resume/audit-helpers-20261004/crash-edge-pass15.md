# Crash/edge pass 15 (tag ce15): the api to web/ JSON contract

Checkout: /home/claude/wt/zero3 at 1f4de71 (read only). This was an audit only, and cargo was not run.
Theme: for each JSON route web/ fetches, I compared the fields crates/api serialises with the fields web/ reads. I looked for renamed or dropped fields, type, unit and >2^53 mismatches, enum gaps, null handling, and refusal or error shapes.
I did not re-report these earlier findings: CE-42, CE-47, CE-48, CE-49 (frontier, ledger, live), CE-60 (bars i64 numbers), CE-71 (calendar token race), CE-72 (locale), CE-74 (rows_now `.sum()` overflow) and CE-25/26 (masters page).

## Routes checked

| Route | api source | web reader | Result |
|---|---|---|---|
| /frontier.json | frontierjson.rs:399 | lib/frontier-analytics.js | clean: mask_words are strings, every number is checked with isSafeInteger, a refusal is shown |
| /trades.json | trades.rs:208,304 | lib/trade-analytics.js | clean: integer exactness and reconciliation are refused loudly |
| /backtest.json mask_words | backtest.rs:701 | lib/mask.js | clean: decimal strings with a u64 check |
| /candidate-trades.json, /expression-search.json | candidatejson.rs:349, expressionsearchjson.rs:169 | lib/candidate-trades.js, expression-search.js | clean: words and counts are strings |
| /vocab.json | server.rs | backtest validateVocabEnvelope | clean |
| /logs.json | logs.rs:200-237 | backtest +page:1970, live-progress.ts | run_key is a string, which is clean. The `error` text is dropped on a non-200 (CE-83) |
| /engine/top.json | topjson.rs:133,211 | backtest +page:2145 | clean: the refusal is shown |
| /store.json (GET, selected feed) | server.rs:4911-5065, census_headers 4495 | lib/store-census.js, store.svelte.js, backtest loadCatalog | CE-77, CE-83 |
| /store.json (HEAD, survey) | same | lib/feed-summary.js | clean: it reads the state, note and degraded headers |
| /autopilot.json | autopilot.rs:1754-1850 | routes/autopilot +page:536-660 | the state enum matches all six words, which is clean. journal_error is never read (CE-78) |
| /ingest/status.json | ingest.rs:1968-2070 | routes/ingest +page:4645 | CE-79 |
| /pull/run.json | pullrun.rs:287-345, 608 | routes/ingest +page:6629,6799,8753-8830 | the camelCase names match. CE-81 |
| /masters/status.json | mastersrun.rs:694-760 | routes/mapping +page:109-137 | CE-82 |
| /indexmap.json | server.rs:34160-34230 | routes/mapping +page:321 | it reads `.error`. The unknown-feed 400 sends `refused` (CE-83) |
| /calendar.json | server.rs:34570, calendar_of.rs:2290,2598 | routes/ingest +page:1057 | the key names match. A refusal shows only its status (CE-83) |
| /gaps.json | server.rs:2531, 2828 | routes/gaps +page:242, lib/gap-verdict.js | the error is shown, which is clean. calendar.unreadable is ignored (CE-80) |
| /audit.json | audit_json.rs | routes/audit +page:324 | clean: the refusal text is shown |
| /feeds.json | server.rs:1633-1690 | +layout probe, ingest pairFloor | clean: `kind` covers fixed, rolling and unknown, and contested and null are handled |

## Findings

### CE-77 (medium): the selected-feed census readers ignore `x-brutex-census-degraded`, so a recovered (older-generation) census is drawn as the current store on /db, /markets, /terminal and the /backtest form

- web/src/lib/store-census.js:57 `if (!result.ok) return { ok: false, status: result.status, body: null, headers: result.headers };` and on a 200 it returns `{ ok: true, status, body, headers }`. No other code reads the headers.
- web/src/lib/store.svelte.js:486-490 `.then((r) => (r.ok ? r.body : ...)) ... Object.assign(store, fold(body));` This code never reads `x-brutex-census-state` or `-degraded`. The word `degraded` appears 0 times in routes/db, markets and terminal.
- api: crates/api/src/server.rs:4518-4525 stamps `x-brutex-census-degraded` with `VendorCensus::degraded()`. census.rs:221-234 says what that means: "the file is damaged and this census is what could still be recovered from it ... a month a later generation had committed is not in them". Such a census is `Held`, so `census_is_unreadable` is false and the answer is **200** (server.rs:5014-5018). The doc at server.rs:4488-4493 says "What a page must read".
- Why it is wrong: the only reader of the header is the HEAD survey (feed-summary.js:12). On /db that survey only shows "not read" in the feed picker's detail and tooltip (db +page:10125-10146). The grid, the counts, /markets, /terminal and the backtest span form all fold the stale generation as the current store, with no mark. The page shows committed months as "never pulled". This is the D-0036 silent fallback, moved one layer up into the browser (CLAUDE.md §4).
- Repro (not run): make a manifest whose newest header slot fails its checksum (pull::manifest falls back to the previous generation), then GET /store.json?feed=dhan. The answer is 200 with a non-empty `x-brutex-census-degraded`, and /db renders the rows with no notice.
- Minimal fix: have the census loader return `state`, `note` and `degraded` from the headers. In store.svelte.js, a non-empty `degraded` should set a named `store.degraded` that each page renders, or the read should be refused the way feed-summary.js refuses it.

### CE-78 (medium): `/autopilot.json` sends `journal_error` and the /autopilot page never reads it, but still tells the operator the journal is the durable record

- api: crates/api/src/autopilot.rs:1759/1765 emits `"journal_error":{}`. It is filled at 3373-3377 from `site.journal().append(&record).err()` (3567). The doc at 1610-1617 reads: "A path is not a proof ... This is that answer, and `/autopilot.json` carries it as `journal_error`."
- web: routes/autopilot/+page.svelte has 0 occurrences of `journal_error`, and its `readState` (536-660) does not copy it. The page renders `<code>{ap.journal ?? JOURNAL}</code> ... A failure here and not there was never written down, and that is a defect in the journal` at 2182-2183, and "durable record is ..." at 2203.
- Why it is wrong: when the append fails (a full disk, permissions, a torn file), the server knows the record was not written, but the only page that reads this route still points the operator at the journal as the proof. The api field exists only to make this failure loud (§4), and its single consumer drops it. /ingest/status.json (ingest.rs:2013) does not carry it either.
- Repro (not run): make `<store>/audit/pull.journal` unwritable, for example with chmod 0444 on its directory, and let one autopilot tick run. `/autopilot.json` then has a non-empty `journal_error`, and /autopilot shows nothing.
- Minimal fix: read `journal_error` in `readState`, and when it is non-empty render it as a bad-tone banner next to the journal path.

### CE-79 (low): /ingest drops every refusal from `/ingest/status.json`. `pilot.error` is never rendered, and a failed read keeps the previous in-flight and halt state

- web/src/routes/ingest/+page.svelte:4648-4661: `if (!r.ok) throw new Error(...)` ... `catch (why) { pilot = { ...pilot, busy: false, error: String(why) }; }`. `pilot.error` has no render site: grep finds no `pilot.error` outside the assignments. On failure the spread keeps the old `inFlight` and `feeds`, so `named` (4884) and `feedHalt` (4680, 4984) keep deciding the 'retry' and 'fail' verdicts from a stale answer.
- web reads only `in_flight`, `waiting_on` and `state`. It never reads `surveyed`, `blocked_by` (ingest.rs:2074-2085 calls it "the one sentence that answers the question the route is named for"), or the 503 body's `error` (ingest.rs:1990: "the autopilot's status lock is poisoned ... Restart the server").
- Why it is wrong: `surveyed:false` with `waiting_on:[]` is the api's explicit "nothing surveyed, not nothing outstanding" (ingest.rs:2074-2080). web folds it to `feeds: []`, which means no halt, so a halted feed's missing months read as "short" or "never" instead of "fail", and a poisoned lock is invisible.
- Repro (not run): request /ingest before the autopilot finishes its first round, or after a publisher panic. The page shows no notice.
- Minimal fix: render `pilot.error`, clear `inFlight` and `feeds` on failure, and show `blocked_by` and `surveyed:false` beside the census table.

### CE-80 (low): the /gaps page never reads `calendar.unreadable`, so peers that could not be read are not named and do not block the "whole" verdict

- api: server.rs:2820-2828 emits `"calendar":{..."voted_by":[...],"unreadable":[...]}`. PeerCalendar doc 2864-2874: "without this list a damaged counter fell back to the table with `voted_by: []` -- the same answer as a store with no peers at all, which is the failure `CLAUDE.md` §4 bans."
- web: routes/gaps/+page.svelte:414-421 prints `voted_by` or "the typed calendar table" and never prints `unreadable`. lib/gap-verdict.js:65-79 `isWholeAudit` checks `answer.calendar?.covers_span === true` and does not check `calendar.unreadable.length === 0`.
- Why it is wrong: when every peer is unreadable, the page says "Measured against the typed calendar table" and can paint "none missing in measured windows". Those are exactly the indistinguishable answers the api added the field to separate.
- Repro (not run): damage one bar file of each peer series that its census still lists, then run GET /gaps.json for a third series. The answer has `source:"table"` and a non-empty `unreadable`, and the page names neither.
- Minimal fix: render `calendar.unreadable` and require it to be empty in `isWholeAudit`.

### CE-81 (low): the /ingest run card draws a halted feed's partial bar in the success colour, never shows `skipped`, and subtracts the row counters with no guard

- web/src/routes/ingest/+page.svelte:8789 `class:up={f.finished || (f.legsDone ?? 0) >= (f.legs ?? 0)}`. The `f.finished ||` term only changes the result when `legsDone < legs`, which is the case pullrun.rs:897-904 marks `finished = true` with `skipped = legs - legs_done` (a Permanent or Credential halt, or a dead chain, 1171). So 1/5 is drawn in `--up` with '—' in the doing cell. The comment at 6821-6829 says `skipped` "WERE MISSING HERE" and was added. It was added only to a type annotation and is rendered nowhere.
- 8756 `n(runState.rowsNow - runState.rowsAtStart)`. The api `rows_now` (pullrun.rs:608-615) `filter_map`s out any vendor whose census is Unreadable, and a degraded reload counts an older generation. Either can make `rowsNow < rowsAtStart`, which prints a negative "bar(s) landed". The api's own summary saturates the same subtraction (`current_rows.saturating_sub(started_rows)`, ~1110), so the two surfaces disagree.
- Repro (not run): use a feed whose second leg returns a permanent refusal. The card shows the feed at 1/N with a green fill.
- Minimal fix: use `class:up={(f.legsDone ?? 0) >= (f.legs ?? 0) && !f.skipped}` and render `f.skipped`. Show the delta only when it is >= 0, and otherwise name the census drop.

### CE-82 (low): /mapping swallows the `/masters/status.json` 503 refusal and keeps a stale restart flag

- web/src/routes/mapping/+page.svelte:113 `if (!response.ok) return;`. The 503 body `{"masters":[],"refusal":"neither BRUTEX_MASTERS nor HOME is set ..."}` (mastersrun.rs:697-702) is never read. `onDisk` keeps its old value (initially `[]`, so the whole four-file table at 602 is hidden), and `restartNeeded` (129) keeps the previous answer. The catch at 130-137 sets `onDisk = []` and also leaves `restartNeeded` stale.
- Why it is wrong: the table disappears with no reason, and a "restart required" banner from an earlier read can stay up after the server has stopped answering. That is stale data with no notice (§4).
- Repro (not run): start the api with BRUTEX_MASTERS and HOME unset, then open /mapping. No file list and no reason are shown.
- Minimal fix: on `!response.ok`, parse the body, show `refusal`, and reset `restartNeeded` to unknown.

### CE-83 (low): four readers print only the HTTP status and drop the api's named reason, and the unknown-feed refusal uses a second key, `refused`, that no reader reads

- lib/store-census.js:57 drops the body, so store.svelte.js:486 shows `HTTP 503 from /store.json`. The api put the reason in the body `{"error":...}` (server.rs:4929, 5036) or in `x-brutex-census-note` (an unreadable census, 4511-4517, doc 4490-4493). A 429 `census read unavailable: Saturated` reads the same way.
- routes/ingest/+page.svelte:1074 `/calendar.json answered ${response.status}`. This drops `{"error":"calendar derivation not admitted ...; retry"}` (server.rs:34605) and the unreadable or unopened refusals.
- routes/backtest/+page.svelte:1981 `/logs.json answered ${response.status}`. This drops `{"error":"logging is not installed ..."}` (logs.rs:227).
- server.rs:1444-1449 `no_such_feed_json` answers `{"refused":...,"feed":...}` for /instruments, /universes, /verify, /store, /indexmap and /calendar, while every other refusal uses `error`. mapping +page:334 `if (body?.error) why = String(body.error);` then prints `/indexmap.json answered 400.`
- Why it is wrong: these failures are loud, but they are not named. The operator gets a status code where the server gave the fix. /audit (audit +page:338-345) and /gaps (gaps +page:259-266) already show the body, so the pattern exists.
- Repro (not run): request /calendar.json while more than MAX_CALENDAR_CONCURRENT derivations are running. /ingest shows only "answered 429".
- Minimal fix: one helper that reads `error ?? refused ?? refusal`, plus the census note header for /store.json, used at each of these sites. Alternatively, rename `refused` to `error` in `no_such_feed_json`.

## Verification rows

None were requested in this pass: 0 FIXED, 0 PARTIAL, 0 NOT FIXED, 0 WRONG FIX.
