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
   * This page asks the absolute question. `/gaps.json` runs `pull::gaps`, which
   * consults `pull::calendar` per day and classifies every absent minute:
   *
   *   · `closed`         — the exchange did not trade. Not a loss.
   *   · `outside-window` — it traded, and this minute was outside its windows.
   *                        Muhurat is one hour in the afternoon; a
   *                        disaster-recovery Saturday has a two-hour hole in
   *                        the middle by design. Not a loss.
   *   · `vendor-hole`    — inside a window the exchange traded, and the bar is
   *                        not there. **The only variant that is a loss.**
   *   · `unmeasured`     — the calendar does not cover this day, so no claim is
   *                        made. Deliberately not folded into `closed`: "the
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
   * shown as `no file` rather than as a hole of some size — the size is
   * unknowable and claiming one would be an invention. `/gaps.json` walks past
   * it rather than refusing, which is the whole reason the ranged form exists.
   *
   * # One request for the whole range
   *
   * The endpoint takes `month` and `to` and answers every month between them in
   * one response. Fanning out one request per month is the shape
   * `/bars/window.json` was built to refuse — measured there at 2,187 requests,
   * past what a browser will open.
   */
  import { untrack } from 'svelte';
  import { feeds } from '$lib/feeds.svelte.js';
  import { ask } from '$lib/ask.js';
  import { store, syncStore } from '$lib/store.svelte.js';
  import { parseKey } from '$lib/instrument.js';

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
   *   absent_file: string | null,
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
   *   calendar: { first: string | null, last: string | null, stale: boolean },
   *   month: MonthVerdict[]
   * }} Answer
   */

  /* THE FORM, SEEDED FROM THE CENSUS AND NEVER FROM A LITERAL. A hardcoded
     default span once threw away 40 of 121 months and reported 81/81 with no
     hole — the answer looked complete because the question was small. Every
     default below is read off `/store.json`. */
  let symbol = $state('');
  let rung = $state('');
  let from = $state('');
  let to = $state('');

  /** @type {{ phase: 'idle' | 'loading' | 'done' | 'failed', body: Answer | null, why: string }} */
  let result = $state({ phase: 'idle', body: null, why: '' });

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
    const seen = new Set();
    for (const row of store.rows ?? []) seen.add(row.instrument);
    return [...seen].sort();
  });

  /* Every rung the census holds. Read rather than listed: the store ships nine
     and the engine sweeps eight, and a page that named them would be a second
     list free to disagree with the first. */
  const rungs = $derived.by(() => {
    const seen = new Set();
    for (const row of store.rows ?? []) if (row.instrument === symbol) seen.add(row.timeframe);
    return [...seen].sort();
  });

  /* The span the census actually holds for the chosen series — the widest true
     answer, so the form opens on the whole of it rather than on a guess. */
  const span = $derived.by(() => {
    let lo = null;
    let hi = null;
    for (const row of store.rows ?? []) {
      if (row.instrument !== symbol) continue;
      if (rung && row.timeframe !== rung) continue;
      if (lo === null || row.month < lo) lo = row.month;
      if (hi === null || row.month > hi) hi = row.month;
    }
    return { lo, hi };
  });

  /* SEEDING, NOT BINDING. The fields are the operator's once they touch them,
     so each is filled only while it is empty — a census that reloads must not
     silently move a range somebody narrowed on purpose. */
  $effect(() => {
    const first = symbols[0];
    if (!symbol && first) untrack(() => (symbol = first));
  });
  $effect(() => {
    const first = rungs[0];
    if (!rung && first) untrack(() => (rung = first));
  });
  $effect(() => {
    const { lo, hi } = span;
    untrack(() => {
      if (!from && lo) from = lo;
      if (!to && hi) to = hi;
    });
  });

  async function run() {
    if (!feeds.active || !symbol || !rung || !from || !to) return;
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
      const response = await ask(url, { cache: 'no-store', ms: 60_000 });
      /* TEXT FIRST, THEN PARSE. Calling `.json()` on a refusal is how a 404
         becomes `SyntaxError: Unexpected token '<'` — measured, against a
         server binary older than this page: the route was not registered, the
         body was not JSON, and the operator was shown a parser error about a
         character instead of the fact that their build is stale. */
      const text = await response.text();
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
      result = { phase: 'failed', body: null, why: error instanceof Error ? error.message : String(error) };
    }
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

  /** Only the runs that are a LOSS. The other three reasons are why a healthy
      store has thousands of absent minutes, and listing them would bury the
      one that matters.
      @param {MonthVerdict} m
      @returns {Run[]} */
  const losses = (m) => (m.gaps ?? []).filter((g) => g.reason === 'vendor-hole');

  /** One month's verdict word. Absence is its own state, never a zero.
      @param {MonthVerdict} m
      @returns {{ word: string, kind: 'ok' | 'bad' | 'absent' | 'quiet' }} */
  function verdict(m) {
    if (m.absent_file) return { word: 'no file', kind: 'absent' };
    if (m.expected === 0) return { word: 'nothing owed', kind: 'quiet' };
    if (m.lost_minutes === 0) return { word: 'whole', kind: 'ok' };
    return { word: 'short', kind: 'bad' };
  }
</script>

<svelte:head><title>Completeness · brutex</title></svelte:head>

<div class="page">
  <header class="head">
    <h1>Completeness</h1>
    <p class="sub">
      Whether a stored series is <b>whole</b>, measured against the exchange calendar rather than
      against its neighbours. <code>/db</code> compares each instrument-month to the fullest peer at
      the same rung, which cannot see a gap that hit every instrument at once. This asks
      <code>pull::gaps</code> what the calendar owed, day by day, and only a minute inside a window
      the exchange actually traded counts as a loss.
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
      <select bind:value={rung}>
        {#each rungs as r (r)}<option value={r}>{r}</option>{/each}
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
    <button class="go" onclick={run} disabled={result.phase === 'loading'}>
      {result.phase === 'loading' ? 'Reading…' : 'Audit'}
    </button>
  </div>

  {#if result.phase === 'idle'}
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
    <div class="census">
      <div class="cell {b.lost_minutes === 0 && b.months_absent === 0 ? 'p' : 'x'}">
        <span class="n">{fmt(b.lost_minutes)}</span>
        <span class="k">minutes lost</span>
        <span class="s">inside a window the exchange traded</span>
      </div>
      <div class="cell {b.months_absent === 0 ? 'p' : 'x'}">
        <span class="n">{fmt(b.months_absent)}</span>
        <span class="k">months with no file</span>
        <span class="s">of {fmt(b.months)} looked at</span>
      </div>
      <div class="cell i">
        <span class="n">{fmt(b.held)}</span>
        <span class="k">bars held</span>
        <span class="s">against {fmt(b.expected)} the calendar owed</span>
      </div>
    </div>

    {#if b.calendar?.stale}
      <div class="refusal">
        <span class="rlabel">The calendar has run out</span>
        <p>
          <code>pull::calendar</code> knows {b.calendar.first} to
          <b>{b.calendar.last}</b>, and today is past it. Every day after that is
          <b>unmeasured</b> — not a loss and not a clean bill, because this build has no holiday
          list for it. Those days add nothing to <em>owed</em> while their bars still count as
          <em>held</em>, which is why <em>held</em> can exceed <em>owed</em> above.
        </p>
        <p class="rhint">
          {fmt(b.unmeasured_minutes)} minutes in this answer are unclaimed for that reason. Nothing
          else in the workspace notices this: <code>LAST_DAY</code> appears outside
          <code>pull::calendar</code> exactly once, in that module's own test. Extending the table
          is an exchange fact and belongs in <code>docs/00-charter.md</code> — this build will not
          invent a trading day.
        </p>
      </div>
    {:else if b.unmeasured_minutes > 0}
      <div class="refusal">
        <span class="rlabel">Partly unclaimed</span>
        <p>
          {fmt(b.unmeasured_minutes)} minutes in this span are <b>unmeasured</b> — outside
          {b.calendar?.first} to {b.calendar?.last}, or a session whose length this build does not
          know. They are neither a loss nor a clean bill, and they add nothing to <em>owed</em>.
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
          <th class="num">Lost</th>
          <th>Verdict</th>
          <th>Where</th>
        </tr>
      </thead>
      <tbody>
        {#each b.month as m (m.month)}
          {@const v = verdict(m)}
          <tr class={v.kind}>
            <td class="mono">{m.month}</td>
            <td class="num mono">{fmt(m.held)}</td>
            <td class="num mono">{fmt(m.expected)}</td>
            <td class="num mono">{m.lost_minutes ? fmt(m.lost_minutes) : '—'}</td>
            <td><span class="tag {v.kind}">{v.word}</span></td>
            <td class="where">
              {#if m.absent_file}
                <span class="dim">{m.absent_file}</span>
              {:else}
                {#each losses(m).slice(0, 6) as g (`${g.day}-${g.from}`)}
                  <span class="run">
                    {dayLabel(g.day)}
                    <span class="dim">{clock(g.from)}–{clock(g.to)}</span>
                    <span class="dim">· {g.minutes}m</span>
                  </span>
                {/each}
                {#if losses(m).length > 6}
                  <span class="dim">and {losses(m).length - 6} more runs</span>
                {/if}
                {#if m.unreadable_records}
                  <span class="run bad">
                    {m.unreadable_records} record(s) unreadable — counted as holes, but the fault is
                    the FILE, not the vendor
                  </span>
                {/if}
                {#if m.truncated}
                  <span class="run bad">this month's run list hit its ceiling and is not complete</span>
                {/if}
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
