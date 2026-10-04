# Crash and edge-input audit, pass 12: the web front end's own logic

**Head:** `final/all-fixes-zero` at 1f4de71, read at /home/claude/wt/zero3. This was an audit only. Nothing was edited, and no cargo run was needed.

**Verdict:** 4 new findings (1 medium, 3 low), CE-70 to CE-73. None of them crashes the engine, and all four are in web/src.
- **CE-70 (medium):** wrong arithmetic in the backtest Performance tab's per-period P&L chart.
- **CE-71 (low):** an older poll reply can overwrite a newer one.
- **CE-72 (low):** the browser's own zone or locale leaks onto the screen.
- **CE-73 (low):** a second rounding rule, the one bps.js was written to remove.

No new instance turned up of a validator stricter or looser than its Rust producer (the CE-42/47/48 class) on the routes pass 6 did not cover: see "Sites checked".

**Method:** source read. CE-70 was reproduced under Node 22 with a verbatim copy of the function (ran). CE-71 to CE-73 were not run; they are read from source.

---

## New findings

### CE-70 (medium): the "Daily / Weekly PnL" holding chart measures the wrong interval, and its daily key has no year
- **Site:** web/src/routes/backtest/+page.svelte:4978-5004 (`holdPeriods`):
  ```
  if (periodScale === 'daily') return `${d.getUTCDate()}/${d.getUTCMonth() + 1}`;
  ...
  return `${monday.getUTCDate()}/${monday.getUTCMonth() + 1}`;
  ...
  if (at) at.last = b.c;
  else buckets.set(k, { label: k, first: b.c, last: b.c });
  ...
  return [...buckets.values()].map((x) => ({ label: x.label, v: x.last - x.first }));
  ```
- **Why it is wrong:**
  1. **Wrong interval.** Each bucket is the bucket's LAST close minus its own FIRST BAR's close. It is not the change from the previous bucket's close. So every bar is dropped from the period that holds it:
     - the move of the period's first bar;
     - the gap between periods (overnight, or over the weekend).

     The chart is captioned "Per-period P&L of HOLDING one unit" (:9423), and that is a close-to-previous-close quantity. The buckets do not sum to the holding P&L. On a 1day series every daily bucket holds one bar, so every day reads exactly 0.
  2. **No year in the key.** The `daily` key is `d/m` with no year (the weekly key has the same shape). The series is the newest `MAX_WINDOW_LIMIT` = 1000 bars (:4346), and on a `1day` run (legacy ledger rows; the switcher always offers the run's own rung, :4797-4803) that is about 4 years. So 5 Mar 2023 and 5 Mar 2024 fold into ONE bucket whose value is a year-over-year change, drawn as one day. The comment above it says buckets are "keyed by a STRING derived from the bar's IST date, so a week that straddles a month or a year stays one bucket". That holds for straddling, but not for different years sharing a label.
- **Repro (ran):** scratchpad p12/hold.mjs, with the function copied verbatim.
  - 1000 weekday 1day bars, +100 paisa per day: `daily` gives 366 buckets, not 1000. Bucket "5/3" has v = 26,100 paisa where the true value is 100. The maximum v is 78,400.
  - 60min bars, 7 per session, +10 per bar and a +500 overnight gap: each day shows v = 60 against a real close-to-close of 570. The buckets sum to 900, while holding from first to last was 8,040.
- **Fix:**
  - Carry the previous bucket's last close and use it as the base of the next bucket's v. The first bucket can use its own first open, or be marked partial.
  - Include the year in the `daily` and `weekly` keys and labels (for example `YYYY-MM-DD`, as `istDay` in lib/dates.js already produces).

### CE-71 (low): /ingest's exchange calendar has no request token, so a slower reply for the previous feed replaces the current feed's calendar
- **Site:** web/src/routes/ingest/+page.svelte:1058-1132:
  ```
  async function loadCalendar(feed) {
    if (!feed) return;
    try {
      const response = await request(`/calendar.json?feed=${encodeURIComponent(feed)}`, ...);
      ...
      calendar = { first: ..., owed, indexOwed, withheld: ..., ... };
  ...
  $effect(() => { const feed = feeds.active; untrack(() => loadCalendar(feed)); });
  ```
- **Why it is wrong:**
  - There is no generation, ticket or abort, and nothing checks after either `await` that `feed === feeds.active`.
  - Switch feed A to B while A's `/calendar.json` is still in flight. If A answers last, the page shows A's span, owed counts and withheld days under B.
  - Separately, choosing "no feed" returns early and leaves the previous feed's calendar standing.
  - While B loads, nothing is cleared, so A's calendar is shown under B with no loading mark.
- **Which numbers are wrong:** that calendar is the denominator for:
  - `isSession` / `holidaysKnownFor` (:1143-1170): days marked "NSE holiday";
  - the month span (:1213-1223);
  - `foldMinuteOwed` (:4725): owed minute counts in the coverage meter.

  Every other per-feed loader on these pages is already gated: backtest `catalogGate` (:3304-3315), mapping `joinRequests` (:360-373), gaps `auditRequests` (:271-275). This loader is the one left out.
- **Repro:** not run; read from source.
- **Fix:** run it through `createPageRequests`, or a `seq`, the way `loadRungs` does (:4766-4774). Discard a reply whose ticket or feed is no longer current. Reset `calendar` to the empty "not loaded" state both on a feed change and on a null feed.

### CE-72 (low): four render sites use the browser's own locale or time zone, against lib/money.js's and lib/dates.js's pinned rule
- **Sites:**
  - web/src/routes/mapping/+page.svelte:616: `new Date(file.modified_unix_millis).toLocaleString()`. This uses the host zone and locale with no zone label, so a London browser shows a master file's modification time 5h30m off IST, unlabelled.
  - mapping :614 and :627: `.toLocaleString()` on byte counts.
  - backtest +page.svelte:7170, 7171, 7175 and 7188: `r.bars / r.minHits / r.candidates / r.priced .toLocaleString()`, with no locale argument.
- **Why it is wrong:** money.js:14-15 says *"Pinned, never inherited. The host's locale is not this product's."* An en-US browser prints 87,828,617 where the rest of the page prints 8,78,28,617. dates.js:93-99 likewise names `Asia/Kolkata` explicitly. This is the same class as P3 "zone is named, not inherited". InvocationAudit.svelte:16 (`toLocaleTimeString('en-IN')`, no `timeZone`) is the same shape, but it stamps the browser's own read time, so it is cosmetic.
- **Repro:** not run (it needs a non-IN browser); read from source.
- **Fix:** use `stampLabel(ms)` from lib/dates.js and `group`/`exact` from lib/money.js.

### CE-73 (low): the per-bar return histogram rounds half toward +Infinity, which is the rule bps.js was written to remove
- **Site:** web/src/routes/backtest/+page.svelte:5027: `rets.push(Math.round(((b.c - b.o) / b.o) * 10_000));`
- **Why it is wrong:** lib/bps.js:6-20 records that `Math.round` on a bp ratio made +312.5 into 313 and -312.5 into -312, and that `Math.round(-0.4)` is `-0`. It replaced that rule with `basisPoints`, which rounds half away from zero exactly as `api::server::basis_points` does. This histogram (and its `avgLoss` / `avgGain`, :5047) still uses the float-and-`Math.round` path. So on one page a mirror-image gain and loss land in asymmetric bins, and a sub-half-bp loss counts as `flat`, not as a loser (`r < 0` is false for `-0`).
- **Repro:** not run. The arithmetic is the one bps.js documents.
- **Fix:** `basisPoints(b.o, b.c)` from lib/bps.js. Skip the bar where it returns `null`, and say so.

---

## Sites checked, and why each holds

**Money and number formatting**
- **lib/money.js `rupee`:** integer quotient and remainder, guarded by `isSafeInteger`.
- **`exact` / `whole` / `oneDp`:** give an em dash on a non-finite value.
- **lib/bps.js `basisPoints` / `bpsText`:** integer-exact, and match the Rust half-away rule.
- **db `paisaText` / `strikeText` (:1033, :4800):** `%` plus a subtraction. A negative strike is unreachable.
- **candidate-trades.js `candidateMoney`:** BigInt exact.
- **index-stop-chart.js:** refuses a price above 2^53 before dividing by 100.
- **comparison.js:600-614:** BigInt round with a safe-range refusal.

**Dates and IST**
- **lib/dates.js:** `istFields` pins `Asia/Kolkata` with h23. A date-only ISO string parses as UTC midnight, which is the same IST day.
- **trade-analytics.js `periodKeyOf`:**
  - shifts +05:30 once before deriving the day;
  - week key `floor((days+3)/7)` gives Monday as `7k-3`, checked;
  - weekday 0 is Monday, and `fill` reorders it to Sun..Sat consistently;
  - `civilDay` refuses a value outside Date's range.
- **The `+IST_OFFSET` helpers:**
  - ingest :939/:1245;
  - autopilot :378/:884;
  - db :4766;
  - markets :1466;
  - index-stop-vix :42;
  - `indexStopServerMonth`.

  All of them shift before reading UTC fields, which is correct in any host zone.
- **boolean-catalog.js `catalogDay`:** a civil-day number formatted in UTC, which is correct.
- **invocation-audit.js `auditTime`, candidate-trades.js `candidateTime`, backtest `when`/`tradeWhen`/`istLabel`, IndexStopChart:** all pass `timeZone: 'Asia/Kolkata'`.

**Polling and overlapping requests**
- **lib/page-requests.js:** single flight. `revoke` bumps the generation, and `ticket.current()` gates every write.
- **`watchVisible`:** reschedules only while the ticket is current and the page is visible.
- **store.svelte.js:**
  - `read` uses a key and a `flightToken`;
  - `tick` awaits, so reads do not stack;
  - the visibility wake is registered once.
- **store-census.js:** per-key flights; `retainedKey` stops a late answer from re-caching; the 304 path checks the ETag.
- **index-stop-launch.js:** epoch, `postAbort`, a backoff capped at 30 s, and an ambiguous POST is never resent.
- **backtest:**
  - `startSweep` is blocked by the `launchStop` derived from `unconfirmedSubmission`;
  - `loadSeries` uses `seriesSeq`;
  - `loadRungs` uses `rungsSeq`;
  - `loadCatalog`/`loadSurface` use `catalogGate`;
  - `/live.json` is a documented last-tick-wins.
- **mapping `fetchJoin`, gaps `runCurrent`, audit, autopilot, layout `/feeds.json`:** all ticketed.
- **InvocationAudit:** `watchVisible`, guarded by `ticket.current()`.

**Validators against their Rust producers (routes pass 6 did not cover)**
- **`/boolean-candidates.json` (booleanjson.rs:226-421) against boolean-catalog.js:**
  - `next_offset` matches JS `end<total`;
  - the grid summary fields, and `forced_stop` arms that line up with each other;
  - u64::MAX ratio and ceiling fields pass `uint`/`sint`;
  - `operator-rule` is served by Rust and refused by JS, but every production policy is `GuaranteedFloor` (cli ledger_all.rs:284), so this is latent and not reported.
- **`/candidate-trades.json` (candidatejson.rs:182-352) against candidate-trades.js:**
  - candidates total = `evaluated*2`;
  - trades total = `cell.trades` (0 when the cell is null);
  - the empty-capture arm and the missing arm match;
  - the 20-character signed and unsigned bounds admit i64::MIN and u64::MAX.
- **`/backtest/audit.json` (operation_audit.rs:237-352, cli operation_audit.rs:167-180) against invocation-audit.js:**
  - the label charset is no more than 96 bytes and has no control characters;
  - the status is 0 or 100..=599;
  - `total_boundaries` is always null;
  - `next_before` uses the same `> ID_BASE+1` rule.
- **`/bars/window.json` (server.rs:3406-3527):** `total` is always present, so terminal.svelte.js's `bars.length` fallback is unreachable.
- **Boolean launch metadata (booleanlaunch.rs:430-441):** the field set, `request_bytes`, `horizon_bars_max` and `max_points_max` are byte-equal to the JS constants.

**Mask decoding**
- **lib/mask.js:** exactly six canonical u64 strings, otherwise refused whole.
- **backtest `fetchVocab`:** refuses a malformed table and names the reason.
- **Unknown bits** render as "Condition bit N" or `cunk`, never as a wrong name.

**Forms and POSTs**
- **backtest sweep / descend:** a lost response becomes `unknown`, and no resend is made.
- **mapping refresh:** see conc-pass6 for its queue; not re-reported here.
- **ingest `/pull/run` and `/pull/run/stop`, autopilot control, receipt-batch, boolean-launch:** single-flight latches.
