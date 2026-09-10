<script>
  /**
   * COMPLETENESS — is this series WHOLE, measured against the exchange calendar.
   *
   * # The question no page could answer
   *
   * `/db` has a completeness column and it is **peer-relative**:
   * `$lib/completeness.js` takes each row's denominator from `fullest`, the
   * largest row count among instruments at the same (month, rung). That is the
   * right measure for the question it answers — "is this instrument short
   * against its neighbours" — and it is structurally unable to answer a
   * different one.
   *
   * A gap that hit EVERY instrument in the same month reads as 100% there.
   * The denominator moves with the data, so a systemic loss — one pull that
   * failed for the whole universe, one vendor month that never landed for
   * anybody — lowers the bar and the score along with it, and every row agrees
   * with every other row about a store that is missing the same week. That
   * page's own header is honest about the neighbouring case: a sole row at a
   * (month, rung) is its own denominator, so "short by zero" is a tautology and
   * it prints `unverified` rather than a verdict.
   *
   * `/gaps.json` runs a minute audit against dated session evidence. NSE cash
   * additionally needs the exact stock's dated closing-auction eligibility;
   * a generic peer cannot establish that schedule or the stock's listing life.
   *
   *   · `closed`         — the exchange did not trade. Not a loss.
   *   · `outside-window` — it traded, and this minute was outside its windows.
   *                        Muhurat is one hour in the afternoon; a
   *                        disaster-recovery Saturday has a two-hour hole in
   *                        the middle by design. Not a loss.
   *   · `vendor-hole`    — an expected minute is absent. This wire name does
   *                        not prove provider fault or that the stock traded.
   *   · `unmeasured`     — required calendar or session evidence is missing,
   *                        so no claim is made. Not folded into `closed`: "the
   *                        exchange was shut" and "nobody has looked" are
   *                        different facts.
   *
   * # Why the arithmetic answer is not offered anywhere on this page
   *
   * "375 bars a day times the trading days" is wrong in both directions, which
   * is why counting was never enough. It calls Muhurat a loss, it calls the
   * DR-Saturday hole a loss, and it calls five pre-2025 Diwali sessions of a
   * length this build does not know a loss. A number that is wrong three ways
   * for a healthy store cannot be trusted when it is non-zero.
   *
   * # A month with NO FILE is the loudest row here
   *
   * A backfill that stopped in March 2021 leaves no file at all, and it is
   * shown as `no file`. Known session evidence can still establish expected
   * and missing minutes; missing evidence remains unmeasured. `/gaps.json`
   * continues through the range and retains both findings.
   *
   * # One request for the whole range
   *
   * The endpoint takes `month` and `to` and answers every month between them in
   * one response. Fanning out one request per month is the shape
   * `/bars/window.json` was built to refuse — measured there at 2,187 requests,
   * past what a browser will open.
   */
  import { onDestroy, untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  import { ask } from '$lib/ask.js';
  import { createPageRequests } from '$lib/page-requests.js';
  import { store, syncStore } from '$lib/store.svelte.js';
  import { parseKey } from '$lib/instrument.js';
  import { monthVerdict, isWholeAudit } from '$lib/gap-verdict.js';

  /** @typedef {{ day: number, from: number, to: number, minutes: number, reason: string }} Run */
  /**
   * @typedef {{
   *   month: string,
   *   expected: number,
   *   held: number,
   *   lost_minutes: number,
   *   absent_minutes: number,
   *   truncated: boolean,
   *   unreadable_records: number,
   *   invalid_timestamps?: number,
   *   absent_file: string | null,
   *   evidence_error?: string | null,
   *   unmeasured_minutes: number,
   *   gaps: Run[]
   * }} MonthVerdict
   */
  /**
   * @typedef {{
   *   expected: number,
   *   held: number,
   *   lost_minutes: number,
   *   unmeasured_minutes: number,
   *   months: number,
   *   months_absent: number,
   *   truncated: boolean,
   *   calendar: {
   *     first: string | null,
   *     last: string | null,
   *     days: number,
   *     source: 'peers' | 'table',
   *     stale: boolean,
   *     covers_span: boolean,
   *     voted_by: string[]
   *   },
   *   month: MonthVerdict[]
   * }} Answer
   */

  /* The instrument and span are seeded from the census. A hardcoded
     default span once threw away 40 of 121 months and reported 81/81 with no
     hole — the answer looked complete because the question was small.
     The endpoint audits only source minutes, so its timeframe is fixed. */
  let symbol = $state('');
  const rung = '1min';
  let from = $state('');
  let to = $state('');

  /** @type {{ phase: 'idle' | 'loading' | 'done' | 'failed', body: Answer | null, why: string }} */
  let result = $state({ phase: 'idle', body: null, why: '' });
  const auditRequests = createPageRequests();
  const questionKey = $derived(JSON.stringify([feeds.active, symbol, rung, from, to]));
  onDestroy(() => auditRequests.dispose());
  $effect(() => {
    questionKey;
    auditRequests.cancel();
    result = { phase: 'idle', body: null, why: '' };
    return () => auditRequests.cancel();
  });

  $effect(() => {
    if (feeds.active) syncStore(feeds.active);
  });
  /**
   * The census key, decomposed — **not** the instrument master.
   *
   * `/store.json` keys a row `NSE-INDEX-NIFTY`, which already carries the
   * exchange, the segment and the underlying, plus a contract tail for a
   * future or an option. Resolving those three through `$lib/place.js` instead
   * would ask the master a question the census has already answered, and would
   * answer `null` for every expired contract — a master lists what is TRADEABLE
   * and an expired option is by definition not, which is exactly the series an
   * audit is most needed for.
   *
   * `parseKey` names a key it cannot read rather than coercing it, so an
   * unreadable one refuses by name here too.
   */
  const parsed = $derived(symbol ? parseKey(symbol) : null);

  /**
   * The contract segment below the symbol, or `''` for a spot series.
   *
   * **Sliced off the key rather than re-rendered from the parsed parts.** A
   * strike arrives as a paisa INTEGER, and formatting it back into digits is a
   * round trip that can lose a trailing zero — at which point the request names
   * a contract one hundredth of the one asked for and the store answers "no
   * such month" for a file that is there. The key is `EXCHANGE-SEGMENT-
   * UNDERLYING` followed by the tail, so the tail is what is left after that
   * prefix and no digit is ever rewritten.
   *
   * @param {ReturnType<typeof parseKey>} at
   * @returns {string}
   */
  function contractTail(at) {
    if (!at.exchange || !at.segment || !at.underlying) return '';
    const prefix = `${at.exchange}-${at.segment}-${at.underlying}`;
    return at.key.length > prefix.length ? at.key.slice(prefix.length + 1) : '';
  }

  /* Every symbol the census holds for this feed, in one pass. */
  const symbols = $derived.by(() => {
    if (store.feed !== feeds.active) return [];
    const seen = new Set();
    for (const row of store.rows ?? []) seen.add(row.instrument);
    return [...seen].sort();
  });

  /* Stored minute months seed the span. A census entry establishes neither
     historical listing dates nor the provider's available history. */
  const span = $derived.by(() => {
    let lo = null;
    let hi = null;
    if (store.feed !== feeds.active) return { lo, hi };
    for (const row of store.rows ?? []) {
      if (row.instrument !== symbol) continue;
      if (row.timeframe !== rung) continue;
      if (lo === null || row.month < lo) lo = row.month;
      if (hi === null || row.month > hi) hi = row.month;
    }
    return { lo, hi };
  });
  const hasMinuteSource = $derived(span.lo !== null);

  /* SEEDING, NOT BINDING. The fields are the operator's once they touch them,
     so each is filled only while it is empty — a census that reloads must not
     silently move a range somebody narrowed on purpose. */
  $effect(() => {
    const first = symbols[0];
    if (!symbol && first) untrack(() => (symbol = first));
  });
  $effect(() => {
    const { lo, hi } = span;
    untrack(() => {
      if (!from && lo) from = lo;
      if (!to && hi) to = hi;
    });
  });

  /** @param {import('$lib/page-requests.js').ReadTicket} ticket @param {string} asked */
  async function runCurrent(ticket, asked) {
    if (!ticket.current() || asked !== questionKey) return;
    if (!feeds.active || !symbol || !hasMinuteSource || !from || !to) return;
    const at = parsed;
    if (!at || !at.exchange || !at.segment || !at.underlying) {
      /* A HALF-KNOWN PLACE IS NOT A PLACE. Asking the route for `undefined`
         would be a request made of a value nobody supplied. */
      result = {
        phase: 'failed',
        body: null,
        why:
          `${symbol} does not read as EXCHANGE-SEGMENT-UNDERLYING, so there is no path to ` +
          `audit. ${at?.why ?? 'The census key could not be decomposed.'} That is a defect in ` +
          `the key rather than in the bars.`
      };
      return;
    }
    result = { phase: 'loading', body: null, why: '' };
    /* THE CONTRACT IS ITS OWN PARAMETER, exactly as `/bars.json` requires.
       Concatenating it onto the symbol is what sent the store a 31-byte name
       against a 24-byte cap; the tail below the symbol is the contract and the
       head is the underlying. */
    const tail = contractTail(at);
    const url =
      `/gaps.json?feed=${encodeURIComponent(feeds.active)}` +
      `&exchange=${encodeURIComponent(at.exchange)}&segment=${encodeURIComponent(at.segment)}` +
      `&symbol=${encodeURIComponent(at.underlying)}&timeframe=${encodeURIComponent(rung)}` +
      (tail ? `&contract=${encodeURIComponent(tail)}` : '') +
      `&month=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`;
    try {
      /* 60 s rather than the default 15. This walks every minute of every month
         asked for — ~44,640 per month against at most ~11,625 stored bars — and
         121 months is a real ask. Giving up on a read that is working would
         report a wedged server that is not one. */
      const response = await ask(url, { cache: 'no-store', ms: 60_000, signal: ticket.signal });
      if (!ticket.current() || asked !== questionKey) return;
      /* TEXT FIRST, THEN PARSE. Calling `.json()` on a refusal is how a 404
         becomes `SyntaxError: Unexpected token '<'` — measured, against a
         server binary older than this page: the route was not registered, the
         body was not JSON, and the operator was shown a parser error about a
         character instead of the fact that their build is stale. */
      const text = await response.text();
      if (!ticket.current() || asked !== questionKey) return;
      let body = null;
      try {
        body = JSON.parse(text);
      } catch {
        body = null;
      }
      if (!response.ok) {
        result = {
          phase: 'failed',
          body: null,
          why:
            body?.error ??
            (response.status === 404
              ? `The server answered 404 for /gaps.json. This page is newer than the binary ` +
                `serving it — the route exists in crates/api and the running process was built ` +
                `before it. Restart the server; nothing is wrong with the store.`
              : `The server answered ${response.status} and the body was not JSON: ` +
                `${text.slice(0, 200)}`)
        };
        return;
      }
      if (!body) {
        result = {
          phase: 'failed',
          body: null,
          why: `The server answered 200 and the body did not parse as JSON: ${text.slice(0, 200)}`
        };
        return;
      }
      result = { phase: 'done', body, why: '' };
    } catch (error) {
      if (!ticket.current() || asked !== questionKey) return;
      result = { phase: 'failed', body: null, why: error instanceof Error ? error.message : String(error) };
    }
  }

  function run() {
    const asked = questionKey;
    auditRequests.cancel();
    return auditRequests.run((ticket) => runCurrent(ticket, asked));
  }

  /** @param {number | null | undefined} n */
  const fmt = (n) => (typeof n === 'number' ? n.toLocaleString('en-IN') : '—');

  /* Days since the epoch back to a date, for a gap run's label. The store's own
     unit, so nothing is converted twice. */
  /** @param {number} day */
  const dayLabel = (day) => new Date(day * 86_400_000).toISOString().slice(0, 10);

  /** Minute-of-day as HH:MM, which is how a session is read.
      @param {number} m */
  const clock = (m) =>
    `${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`;

  /** Expected absences, with the backend's exact dates and minute boundaries.
      The wire name does not establish that the provider caused the absence.
      @param {MonthVerdict} m
      @returns {Run[]} */
  const losses = (m) => (m.gaps ?? []).filter((g) => g.reason === 'vendor-hole');
</script>

<svelte:head><title>Completeness · brutex</title></svelte:head>

<div class="page">
  <header class="head">
    <h1>Completeness</h1>
    <p class="sub">
      Whether a stored <b>1min</b> series is whole against the available dated session evidence.
      <code>/db</code> compares each instrument-month to the fullest peer, which cannot see a gap
      shared by every instrument. Here, missing means an expected minute is absent; it does not
      prove provider fault. NSE cash needs the exact stock's dated closing-auction eligibility.
      A stored dataset does not establish the instrument's full listing history or the source's
      available history.
    </p>
  </header>

  <div class="controls">
    <label class="f">
      <span class="fk">Instrument</span>
      <select bind:value={symbol}>
        {#each symbols as s (s)}<option value={s}>{s}</option>{/each}
      </select>
    </label>
    <label class="f">
      <span class="fk">Rung</span>
      <select value={rung} disabled aria-describedby="minute-audit-note">
        <option value="1min">1min</option>
      </select>
    </label>
    <label class="f">
      <span class="fk">From</span>
      <input bind:value={from} placeholder="YYYY-MM" size="8" />
    </label>
    <label class="f">
      <span class="fk">To</span>
      <input bind:value={to} placeholder="YYYY-MM" size="8" />
    </label>
    <button
      class="go"
      onclick={run}
      disabled={result.phase === 'loading' || !feeds.active || !hasMinuteSource || !from || !to}
      aria-describedby="minute-audit-note"
    >
      {result.phase === 'loading' ? 'Reading…' : 'Audit'}
    </button>
  </div>

  <p class="whence" id="minute-audit-note">
    This is a source-minute audit. Daily and coarser bars cannot prove their source minutes.
  </p>

  {#if !hasMinuteSource}
    <p class="state">
      {#if store.error}
        Minute audit unavailable: the store census could not be read. {store.error}
      {:else if !feeds.active || store.feed !== feeds.active}
        Waiting for the selected feed's store census before a minute audit can be offered.
      {:else if symbol}
        No 1min source is recorded for {symbol} in this feed's store census. Audit is disabled;
        select a series with stored 1min bars. Other timeframes cannot substitute for them.
      {:else}
        No stored series is available to select for a minute audit.
      {/if}
    </p>
  {:else if result.phase === 'idle'}
    <p class="state">
      Pick a series and a span. The whole range is one request — the endpoint answers every month
      between the two, so nothing here fans out per month.
    </p>
  {:else if result.phase === 'loading'}
    <p class="state">
      Walking every minute the calendar owes for {symbol} · {rung} · {from} to {to}.
    </p>
  {:else if result.phase === 'failed'}
    <div class="refusal">
      <span class="rlabel">Refused</span>
      <p>{result.why}</p>
    </div>
  {:else if result.body}
    {@const b = result.body}
    {@const whole = isWholeAudit(b)}
    <div class="census">
      <div class="cell {whole ? 'p' : b.lost_minutes > 0 ? 'x' : ''}">
        <span class="n">{fmt(b.lost_minutes)}</span>
        <span class="k">minutes missing</span>
        <span class="s">
          {whole ? 'none missing in measured windows' : 'expected absences; see month verdicts'}
        </span>
      </div>
      <div class="cell {whole ? 'p' : b.months_absent > 0 ? 'x' : ''}">
        <span class="n">{fmt(b.months_absent)}</span>
        <span class="k">months with no file</span>
        <span class="s">of {fmt(b.months)} looked at</span>
      </div>
      <div class="cell i">
        <span class="n">{fmt(b.held)}</span>
        <span class="k">bars held</span>
        <span class="s">against {fmt(b.expected)} expected from available evidence</span>
      </div>
    </div>

    <p class="whence">
      Measured against
      {#if b.calendar?.source === 'peers'}
        <b>{b.calendar.voted_by.length} peer reading{b.calendar.voted_by.length === 1 ? '' : 's'}</b>
        — {b.calendar.voted_by.join(', ')} — agreed by union, covering
        {b.calendar.first} to {b.calendar.last}. The series being audited is not among them: a
        calendar derived from it would read every hole as the edge of a trading window and answer
        <em>no losses</em> for any input.
      {:else}
        the <b>typed calendar table</b>, {b.calendar?.first} to {b.calendar?.last}.
      {/if}
      NSE cash uses the typed calendar and exact dated stock session metadata. Generic peers
      cannot supply a stock's closing-auction eligibility. None of these readings establishes
      its full listing lifetime or guarantees that a provider can supply every expected minute.
    </p>

    {#if b.calendar?.stale}
      <div class="refusal">
        <span class="rlabel">The table has run out</span>
        <p>
          The typed calendar covers {b.calendar.first} to <b>{b.calendar.last}</b>, and today is
          past it. It cannot verify sessions beyond that bound. A historical span fully inside
          the bound still requires complete session evidence and readable, valid records.
        </p>
        <p class="rhint">
          Calendar extensions require dated exchange evidence. Peer data cannot replace missing
          cash-session metadata for the exact stock and date.
        </p>
      </div>
    {/if}
    {#if b.unmeasured_minutes > 0 || b.calendar?.covers_span === false}
      <div class="refusal">
        <span class="rlabel">Partly unclaimed</span>
        <p>
          {fmt(b.unmeasured_minutes)} minutes in this span are <b>unmeasured</b>.
          Calendar coverage or required dated session metadata is incomplete; the month rows
          name missing evidence. Unknown minutes are excluded from <em>owed</em>, so zero
          measured absences cannot establish completeness and <em>held</em> may exceed <em>owed</em>.
        </p>
      </div>
    {/if}

    {#if b.truncated}
      <div class="refusal">
        <span class="rlabel">Truncated</span>
        <p>
          The walk stopped at its ceiling before it reached the end of the range, so the months
          after it were never looked at. They are not reported as clean — they are not reported at
          all. Narrow the span and ask again.
        </p>
      </div>
    {/if}

    <table class="months">
      <thead>
        <tr>
          <th>Month</th>
          <th class="num">Held</th>
          <th class="num">Owed</th>
          <th class="num">Missing</th>
          <th>Verdict</th>
          <th>Where</th>
        </tr>
      </thead>
      <tbody>
        {#each b.month as m (m.month)}
          {@const v = monthVerdict(m)}
          <tr class={v.kind}>
            <td class="mono">{m.month}</td>
            <td class="num mono">{fmt(m.held)}</td>
            <td class="num mono">{fmt(m.expected)}</td>
            <td class="num mono">{m.lost_minutes ? fmt(m.lost_minutes) : '—'}</td>
            <td><span class="tag {v.kind}">{v.word}</span></td>
            <td class="where">
              {#if m.absent_file != null}
                <span class="dim">{m.absent_file || 'The month file is unavailable.'}</span>
              {/if}
              {#if m.evidence_error != null}
                <span class="run bad">
                  Dated session evidence unavailable: {m.evidence_error || 'No reason supplied.'}
                </span>
              {/if}
              {#if m.unmeasured_minutes > 0}
                <span class="run bad">
                  {fmt(m.unmeasured_minutes)} minutes unmeasured — calendar or dated session
                  evidence is incomplete; zero measured absences does not mean whole
                </span>
              {/if}
              {#if m.invalid_timestamps}
                <span class="run bad">
                  {fmt(m.invalid_timestamps)} invalid timestamp(s) — off-grid, duplicate or
                  backward stamps; this month's totals are unverified
                </span>
              {/if}
              {#each losses(m) as g (`${g.day}-${g.from}`)}
                <span class="run">
                  {dayLabel(g.day)}
                  <span class="dim">{clock(g.from)}–{clock(g.to)}</span>
                  <span class="dim">· {g.minutes}m</span>
                </span>
              {/each}
              {#if m.unreadable_records}
                <span class="run bad">
                  {fmt(m.unreadable_records)} record(s) unreadable — the file could not supply
                  those bars; this does not establish provider fault
                </span>
              {/if}
              {#if m.truncated}
                <span class="run bad">this month's run list hit its ceiling and is not complete</span>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</div>

<style>
  .page {
    padding: var(--s5) var(--s5) var(--s8);
    max-width: 1180px;
    margin: 0 auto;
    /* THE SHELL DELIBERATELY DOES NOT SCROLL. `theme.css`'s `.main` is
       `overflow: hidden`, so every page provides its own scroller; `/mapping`
       and `/backtest` were both measured clipped for want of these two lines,
       at 4,325px and 248px. `min-height: 0` is load-bearing beside it: without
       it a flex child floors at its content height and `auto` has nothing to
       scroll. */
    min-height: 0;
    overflow-y: auto;
  }
  .head {
    margin-bottom: var(--s5);
  }
  h1 {
    font: var(--w-semi) var(--fs-xl) / 1.15 var(--sans);
    letter-spacing: -0.015em;
    color: var(--n11);
    margin: 0 0 var(--s2);
  }
  .sub {
    font-size: var(--fs-sm);
    color: var(--n9);
    line-height: 1.55;
    max-width: 78ch;
    margin: 0;
  }
  code {
    font-family: var(--mono);
    color: var(--n11);
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--s3);
    margin: var(--s5) 0;
  }
  .f {
    display: flex;
    flex-direction: column;
    gap: var(--s1, 4px);
  }
  .fk {
    font-size: var(--fs-xs, 11px);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--n8);
  }
  select,
  input {
    font: var(--fs-sm) / 1.2 var(--sans);
    color: var(--n11);
    background: var(--n1, transparent);
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: var(--s2) var(--s3);
  }
  .go {
    font: var(--w-semi) var(--fs-sm) / 1.2 var(--sans);
    color: var(--n11);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: var(--s2) var(--s4);
    cursor: pointer;
  }
  .go:disabled {
    opacity: 0.55;
    cursor: default;
  }
  .state {
    font-size: var(--fs-sm);
    color: var(--n9);
    padding: var(--s5) 0;
    max-width: 78ch;
    line-height: 1.55;
  }
  .refusal {
    border: 1px solid var(--n6);
    border-left: 3px solid var(--warn);
    border-radius: var(--r1);
    padding: var(--s3) var(--s4);
    margin: var(--s4) 0;
    max-width: 78ch;
  }
  .rlabel {
    font: var(--w-semi) var(--fs-xs, 11px) / 1.2 var(--sans);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--warn);
  }
  .refusal p {
    font-size: var(--fs-sm);
    color: var(--n9);
    line-height: 1.55;
    margin: var(--s2) 0 0;
  }
  .refusal .rhint {
    font-size: var(--fs-xs, 11px);
    color: var(--n8);
  }
  .whence {
    font-size: var(--fs-xs, 11px);
    color: var(--n8);
    line-height: 1.6;
    max-width: 78ch;
    margin: 0 0 var(--s4);
    padding-left: var(--s3);
    border-left: 2px solid var(--n3);
  }
  .census {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3);
    margin: var(--s4) 0 var(--s5);
  }
  .cell {
    display: flex;
    flex-direction: column;
    gap: 2px;
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: var(--s3) var(--s4);
    min-width: 200px;
  }
  .cell .n {
    font: var(--w-semi) var(--fs-xl) / 1.1 var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }
  .cell .k {
    font-size: var(--fs-sm);
    color: var(--n9);
  }
  .cell .s {
    font-size: var(--fs-xs, 11px);
    color: var(--n8);
  }
  .cell.p .n {
    color: var(--up);
  }
  .cell.x .n {
    color: var(--down);
  }
  .cell.i .n {
    color: var(--acc);
  }
  .months {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-sm);
  }
  .months th {
    text-align: left;
    font: var(--w-semi) var(--fs-xs, 11px) / 1.2 var(--sans);
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--n8);
    border-bottom: 1px solid var(--n6);
    padding: var(--s2) var(--s3);
  }
  .months td {
    border-bottom: 1px solid var(--n3);
    padding: var(--s2) var(--s3);
    color: var(--n9);
    vertical-align: top;
  }
  .num {
    text-align: right;
  }
  .mono {
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    color: var(--n11);
  }
  .tag {
    font: var(--w-semi) var(--fs-xs, 11px) / 1.2 var(--sans);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: 1px 6px;
    white-space: nowrap;
  }
  .tag.ok {
    color: var(--up);
    border-color: var(--up);
  }
  .tag.bad {
    color: var(--down);
    border-color: var(--down);
  }
  .tag.absent {
    color: var(--warn);
    border-color: var(--warn);
  }
  .where {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s2);
    max-width: 60ch;
  }
  .run {
    font-family: var(--mono);
    font-size: var(--fs-xs, 11px);
    color: var(--n9);
    white-space: nowrap;
  }
  .run.bad {
    color: var(--down);
    white-space: normal;
  }
  .dim {
    color: var(--n8);
  }
</style>
