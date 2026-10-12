<script>
  /**
   * ALL SOURCES — one instrument, every feed, side by side (WP-58; D-3339,
   * with D-3323 and D-3324 for the labels; DB-079, DB-G41, DB-G58, DB-G99).
   *
   * `/db` shows ONE feed at a time, by design. This page is where the feeds
   * meet, for one instrument:
   *
   *   1. the rung table   — every bar size each feed holds, stored, derived
   *                         or folded on request (`/sources.json`);
   *   2. one timeline     — a chosen bar size, one tagged row per feed in
   *                         every bucket, the server's comparison under them
   *                         (`/sources/bars.json`);
   *   3. the day table    — Zerodha's official day bar, the fold of its
   *                         minutes and the fold of GDFL's seconds, per day
   *                         (`/sources/day.json`).
   *
   * STORAGE STAYS PER VENDOR. Nothing here is merged: every row names its
   * feed and its tag, and every source row links back to that feed's own
   * stored rows on /db.
   *
   * THE ADDRESS IS THE SELECTION. Instrument, rung, day, direction, cursor
   * and month are read from the address and nowhere else, so a link from /db
   * (or a pasted one) opens exactly what it names, and every control is a
   * navigation. A field the address spells wrongly is named on the page and
   * left at its default, never coerced.
   *
   * EVERY NUMBER IS THE SERVER'S. The differences, flags, factors and
   * tolerances are computed in `crates/api/src/sources.rs` with the one fold
   * authority (`pull::fold`); `$lib/sources.js` only words them. A refusal is
   * drawn as the server's sentence, a failure as a failure, and neither as an
   * empty table.
   *
   * THREE REQUESTS, EACH BOUNDED BY ITS PAGE: the rung table is O(rungs ×
   * months held) census probes, a timeline page reads one page of buckets
   * through `.tix`, and the day table is at most 31 days, kept per day by the
   * server. Each section shows how long its own request took.
   */
  import { onDestroy, untrack } from 'svelte';
  import { afterNavigate, goto } from '$app/navigation';
  import { ask } from '$lib/ask.js';
  import { createPageRequests } from '$lib/page-requests.js';
  import {
    SOURCES_DEFAULTS,
    decodeSources,
    sourcesHref,
    instrumentOf,
    rungsUrl,
    barsUrl,
    dayUrl,
    timelineMode,
    shiftMonth,
    outcomeOf,
    rungTable,
    timeline,
    dayTable,
    toleranceLine,
    rungSeconds,
    countText
  } from '$lib/sources.js';

  /* ── The address ──────────────────────────────────────────────────────── */
  let search = $state(typeof location === 'undefined' ? '' : location.search);
  afterNavigate(() => {
    search = location.search;
  });

  const decoded = $derived(decodeSources(search));
  const key = $derived(decoded.sel.key ?? SOURCES_DEFAULTS.key);
  const rung = $derived(decoded.sel.rung ?? SOURCES_DEFAULTS.rung);
  const dir = $derived(decoded.sel.dir ?? SOURCES_DEFAULTS.dir);
  const cursor = $derived(decoded.sel.cursor ?? '');
  const instrument = $derived(instrumentOf(key));

  /**
   * Navigate to this page with some fields changed. A change of instrument,
   * rung, day or direction drops the cursor: a cursor is a place in ONE
   * timeline and means nothing in another.
   *
   * @param {Record<string, string | undefined>} change
   */
  function hrefWith(change) {
    const next = { key, rung, dir, day: decoded.sel.day, month: decoded.sel.month, cursor: decoded.sel.cursor, ...change };
    if (!('cursor' in change)) next.cursor = undefined;
    return sourcesHref(/** @type {any} */ (next));
  }
  /** @param {Record<string, string | undefined>} change */
  const go = (change) => goto(hrefWith(change), { keepFocus: true, noScroll: true });

  /* ── One request per section, each its own owner ──────────────────────── */
  /**
   * @typedef {{phase: 'idle'|'loading'|'ok'|'refused'|'failed', url: string, body?: any, why?: string, status?: number, ms?: number}} Read
   */
  /** @type {Read} */
  const IDLE = { phase: 'idle', url: '' };
  let rungsRead = $state.raw(IDLE);
  let barsRead = $state.raw(IDLE);
  let daysRead = $state.raw(IDLE);

  const owners = { rungs: createPageRequests(), bars: createPageRequests(), days: createPageRequests() };
  onDestroy(() => {
    for (const owner of Object.values(owners)) owner.dispose();
  });

  /**
   * Read one route. The owner revokes whatever it had in flight first, so a
   * late answer to an earlier question can never be drawn under a later one.
   *
   * @param {{cancel: () => void, run: (work: (ticket: {signal: AbortSignal, current: () => boolean}) => Promise<void>) => Promise<void>}} owner
   * @param {string | null} url
   * @param {(read: Read) => void} set
   */
  function read(owner, url, set) {
    owner.cancel();
    if (!url) {
      set(IDLE);
      return;
    }
    set({ phase: 'loading', url });
    void owner.run(async (ticket) => {
      const started = performance.now();
      try {
        const response = await ask(url, { signal: ticket.signal });
        /** @type {unknown} */
        let body = null;
        try {
          body = await response.json();
        } catch {
          body = null;
        }
        if (!ticket.current()) return;
        set({ ...outcomeOf(response.status, body), url, ms: performance.now() - started });
      } catch (error) {
        if (!ticket.current()) return;
        set({ phase: 'failed', url, status: 0, why: error instanceof Error ? error.message : String(error), ms: performance.now() - started });
      }
    });
  }

  /* ── 1. The rung table ────────────────────────────────────────────────── */
  const rungsAsk = $derived(instrument.ok ? rungsUrl(key) : null);
  $effect(() => {
    const url = rungsAsk;
    untrack(() => read(owners.rungs, url, (r) => (rungsRead = r)));
  });
  const rungs = $derived(rungsRead.phase === 'ok' && rungsRead.url === rungsAsk ? rungTable(rungsRead.body) : null);

  /** The rungs the answer names, finest first: the timeline's choices come from the data. */
  const rungChoices = $derived.by(() => {
    const seen = new Set(rungs ? rungs.rows.map((/** @type {any} */ r) => r.rung) : []);
    seen.add(rung);
    return [...seen].sort((a, b) => rungSeconds(a) - rungSeconds(b));
  });

  /* ── 2. The timeline ──────────────────────────────────────────────────── */
  /** The day shown: the address's, else the newest day every holding feed has (from the rung table). */
  const day = $derived(decoded.sel.day ?? rungs?.lastDay ?? '');
  const month = $derived(decoded.sel.month ?? (day ? day.slice(0, 7) : ''));

  /* AT THE DAY RUNG THE TIMELINE IS THE DAY TABLE. `/sources/bars.json` at
     the day rung folds a month of seconds on every request and keeps none of
     it; the day table holds the same days, tagged the same way, kept per day
     by the server. So the day rung asks nothing here.

     THE DAY RUNG IS THE ONE THE RUNG TABLE NAMES, never any spelling that
     ends in "day": `rung=2day` or `999day` is asked of the server like any
     other rung and its refusal is drawn (`timelineMode`). While the rung
     table has not answered, a one-day-long rung waits for it. */
  const rungsAnswered = $derived(rungsRead.url === rungsAsk && (rungsRead.phase === 'ok' || rungsRead.phase === 'refused' || rungsRead.phase === 'failed'));
  const mode = $derived(timelineMode(rung, rungs, rungsAnswered));
  const dayRung = $derived(mode === 'day');
  const barsAsk = $derived(instrument.ok && day && mode === 'ask' ? barsUrl({ key, rung, day, dir, cursor }) : null);
  $effect(() => {
    const url = barsAsk;
    untrack(() => read(owners.bars, url, (r) => (barsRead = r)));
  });
  const bars = $derived(barsRead.phase === 'ok' && barsRead.url === barsAsk ? timeline(barsRead.body, rungs) : null);

  /* ── 3. The day table ─────────────────────────────────────────────────── */
  const daysAsk = $derived(instrument.ok && month ? dayUrl(key, month) : null);
  $effect(() => {
    const url = daysAsk;
    untrack(() => read(owners.days, url, (r) => (daysRead = r)));
  });
  const days = $derived(daysRead.phase === 'ok' && daysRead.url === daysAsk ? dayTable(daysRead.body, rungs) : null);

  /* ── The instrument box ───────────────────────────────────────────────── */
  let typed = $state('');
  let typedWhy = $state('');
  $effect(() => {
    typed = key;
    typedWhy = '';
  });
  /** @param {SubmitEvent} event */
  function openTyped(event) {
    event.preventDefault();
    const asked = instrumentOf(typed);
    if (!asked.ok) {
      typedWhy = asked.why;
      return;
    }
    typedWhy = '';
    void goto(sourcesHref({ key: asked.key, rung, dir }), { keepFocus: true, noScroll: true });
  }

  /** @param {number | undefined} ms */
  const took = (ms) => (typeof ms === 'number' ? `${ms < 10 ? ms.toFixed(1) : Math.round(ms)} ms` : '');
</script>

<svelte:head><title>All sources · {key} · brutex</title></svelte:head>

{#snippet status(/** @type {Read} */ r, /** @type {string} */ what)}
  {#if r.phase === 'loading'}
    <p class="state" role="status">Reading {what}…</p>
  {:else if r.phase === 'refused'}
    <p class="state refusal" role="alert"><b>Refused ({r.status}).</b> {r.why} <a class="raw" href={r.url} target="_blank" rel="noopener">the request</a></p>
  {:else if r.phase === 'failed'}
    <p class="state failure" role="alert"><b>Failed{r.status ? ` (${r.status})` : ''}.</b> {r.why} <a class="raw" href={r.url} target="_blank" rel="noopener">the request</a></p>
  {/if}
{/snippet}

{#snippet chips(/** @type {Array<{code: string, word: string, tone: string, why: string}>} */ flags)}
  {#each flags as f, i (i)}
    <span class="chip t-{f.tone}" title={f.why} data-flag={f.code}>{f.word}</span>
  {/each}
{/snippet}

{#snippet cmpRow(/** @type {any} */ c, /** @type {boolean} */ oi)}
  <tr class="cmp">
    <th scope="row" class="src">{c.label}</th>
    {#each [c.o, c.h, c.l, c.c, c.v] as cell, i (i)}
      <td class="s-num t-{cell.tone}" title={cell.title}>{cell.text}{#if cell.note}<i>{cell.note}</i>{/if}</td>
    {/each}
    {#if oi}<td></td>{/if}
    <td></td>
  </tr>
{/snippet}

<div class="page" id="sources">
  <header class="head">
    <h1>All sources <span class="of">{key}</span></h1>
    <p class="s-sub">
      Every feed that holds this instrument, side by side. Nothing is merged: each row names its feed and whether it is
      stored, derived or folded on request, and links to that feed's own rows on <a href="/db">DB</a>. Differences, flags
      and factors are computed by the server.
    </p>
  </header>

  <form class="controls" onsubmit={openTyped}>
    <label class="f">
      <span class="fk">Instrument key</span>
      <input name="key" bind:value={typed} size="38" spellcheck="false" autocomplete="off" aria-describedby="key-help" aria-invalid={typedWhy ? 'true' : undefined} />
    </label>
    <button class="go" type="submit">Open</button>
    <span class="hint" id="key-help">EXCHANGE-SEGMENT-SYMBOL, like NSE-CASH-RELIANCE; one option contract adds -YYYY-MM-DD-STRIKEPAISA-CE. Or open it from DB with “All sources”.</span>
  </form>
  {#if typedWhy}<p class="state refusal" role="alert"><b>Not opened.</b> {typedWhy}.</p>{/if}
  {#if !instrument.ok}<p class="state refusal" role="alert"><b>No instrument.</b> {instrument.why}.</p>{/if}
  {#each decoded.refused as r, i (i)}
    <p class="state refusal" role="alert"><b>The address’s {r.key}={JSON.stringify(r.value)} was not used.</b> {r.why}.</p>
  {/each}

  <!-- ══ 1. THE RUNG TABLE ══════════════════════════════════════════════════ -->
  <section aria-labelledby="h-rungs" data-section="rungs">
    <div class="sec-head">
      <h2 id="h-rungs">Bar sizes held, by feed</h2>
      {#if rungsRead.phase === 'ok'}<span class="took" title="how long this page's request to /sources.json took">{took(rungsRead.ms)}</span>{/if}
      {#if rungsAsk}<a class="raw" href={rungsAsk} target="_blank" rel="noopener">JSON</a>{/if}
    </div>
    {@render status(rungsRead, 'the rung table')}
    {#if rungs}
      <ul class="s-feeds">
        {#each rungs.feeds as f, i (i)}
          <li class="s-feed">
            <b>{f.name}</b>
            <span class="chip t-{f.census.tone}" title={f.census.note}>census {f.census.state}</span>
            <span>{f.rungsHeld === null ? 'rungs not sent' : `${f.rungsHeld} rung${f.rungsHeld === 1 ? '' : 's'} held`}</span>
            {#if f.archive}<span title={f.archive.source}>archive {f.archive.from} → {f.archive.to}</span>{/if}
            {#if f.window}<span title={f.window.source}>your window {f.window.from} → {f.window.to}</span>{/if}
            {#if f.census.behindDays}<span class="t-warn">census {f.census.behindDays} day{f.census.behindDays === 1 ? '' : 's'} behind{f.census.behindFrom ? ` from ${f.census.behindFrom}` : ''}</span>{/if}
            {#if f.census.degraded}<span class="t-warn">degraded: {f.census.degraded}</span>{/if}
          </li>
        {/each}
      </ul>
      {#if rungs.empty}
        <p class="state refusal">{rungs.empty}</p>
      {/if}
      {#if rungs.rows.length > 0}
        <div class="scroll">
          <table class="s-grid rungs">
            <caption class="vh">Every bar size each feed holds for {rungs.key}</caption>
            <thead>
              <tr>
                <th scope="col">Source</th>
                <th scope="col">State</th>
                <th scope="col">First → last day</th>
                <th scope="col" class="s-num">Months</th>
                <th scope="col">Sessions not held</th>
                <th scope="col" class="s-num">Bars counted</th>
                <th scope="col">Price · volume · close</th>
              </tr>
            </thead>
            <tbody>
              {#each rungs.lines as r, i (i)}
                <tr class="k-{r.kind}" data-tag={r.tag} data-rung={r.rung} data-state={r.state.code}>
                  <th scope="row" class="src">
                    <span class="s-tag">{r.tagWord}</span>
                    <span class="s-why links">
                      {#if r.rungs.length > 1}
                        to {#each r.rungs as one, j (j)}<a href={hrefWith({ rung: one.rung })} title="the combined timeline at {one.rung}">{one.rung}</a>{' '}{/each}
                      {:else}
                        {#if r.db}<a href={r.db} title="this feed's stored rows on DB">DB rows</a>{' '}{/if}
                        <a href={hrefWith({ rung: r.rung })} title="the combined timeline at this bar size">timeline</a>
                      {/if}
                    </span>
                  </th>
                  <td>
                    <span class="chip t-{r.state.tone}" title={r.kind === 'fold' ? r.state.why.join(' ') : undefined}>{r.state.word}</span>
                    {#if r.kind !== 'fold'}{#each r.state.why as line, j (j)}<span class="s-why">{line}</span>{/each}{/if}
                  </td>
                  <td class="span">
                    {#if r.outside}
                      <span class="oow" title="held on disk, hidden on DB by default, never deleted (D-3338)">
                        {r.outside.from} → {r.outside.to}
                        <i>{r.outside.months === null ? '' : `${r.outside.months} months · `}{r.outside.label}</i>
                        {#if r.outside.db}<a href={r.outside.db}>show on DB</a>{/if}
                      </span>
                    {/if}
                    <span>{r.insideFrom || '—'} → {r.last || '—'}</span>
                  </td>
                  <td class="s-num">
                    {countText(r.monthsHeld)}
                    {#if r.monthsAbsent}<span class="s-why t-warn">{r.monthsAbsent} absent: {r.monthHoles.text}</span>{/if}
                  </td>
                  <td>
                    {#if r.sessions.notHeld === null}
                      <span class="t-quiet">{r.sessions.why || (r.kind === 'fold' ? 'as its seconds' : 'not sent')}</span>
                    {:else if r.sessions.notHeld === 0}
                      <span class="t-ok">none of {countText(r.sessions.owed)} owed</span>
                    {:else}
                      <span class="t-warn">{countText(r.sessions.notHeld)} of {countText(r.sessions.owed)} owed</span>
                      <span class="s-why">{r.sessions.holes.text}{r.sessions.holes.all ? '' : ' (not all are named here)'}</span>
                    {/if}
                    {#if r.sessions.calendar}<a class="raw" href={r.sessions.calendar} target="_blank" rel="noopener">calendar</a>{/if}
                  </td>
                  <td class="s-num">{r.kind === 'fold' ? 'none filed' : countText(r.rows)}</td>
                  <td class="basis">
                    <span class="t-{r.basis.tone}" title={r.basis.source || r.basis.unverified}>{r.basis.price}</span>
                    <span class="t-quiet">{r.basis.volume}</span>
                    <span class="t-{r.basis.close === 'official close' ? 'info' : 'quiet'}">{r.basis.close}</span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
      {#each rungs.notes as note, i (i)}<p class="s-note">{note}</p>{/each}
      {#if rungs.tolerances}<p class="s-note" title={rungs.tolerances.rule}>{toleranceLine(rungs.tolerances)}</p>{/if}
    {/if}
  </section>

  <!-- ══ 2. THE COMBINED TIMELINE ═══════════════════════════════════════════ -->
  <section aria-labelledby="h-bars" data-section="timeline">
    <div class="sec-head">
      <h2 id="h-bars">One timeline, every source tagged</h2>
      {#if barsRead.phase === 'ok'}<span class="took" title="how long this page's request to /sources/bars.json took">{took(barsRead.ms)}</span>{/if}
      {#if barsAsk}<a class="raw" href={barsAsk} target="_blank" rel="noopener">JSON</a>{/if}
    </div>
    <div class="controls">
      <label class="f">
        <span class="fk">Bar size</span>
        <select value={rung} onchange={(e) => go({ rung: e.currentTarget.value })}>
          {#each rungChoices as choice, i (i)}<option value={choice}>{choice}</option>{/each}
        </select>
      </label>
      {#if mode === 'ask'}
        <label class="f">
          <span class="fk">Day</span>
          <input type="date" value={day} onchange={(e) => e.currentTarget.value && go({ day: e.currentTarget.value })} />
        </label>
        <label class="f">
          <span class="fk">Order</span>
          <select value={dir} onchange={(e) => go({ dir: e.currentTarget.value })}>
            <option value="desc">newest first</option>
            <option value="asc">oldest first</option>
          </select>
        </label>
        {#if cursor}<a class="go" href={hrefWith({})}>First page</a>{/if}
        {#if bars?.nextCursor}<a class="go" href={hrefWith({ cursor: bars.nextCursor })}>{dir === 'asc' ? 'Later' : 'Earlier'} page →</a>{/if}
      {/if}
    </div>
    {#if dayRung}
      <p class="state" data-day-rung>
        At {rung} the timeline is the <a href="#h-days">day table below</a>: every day of the month, one tagged row per source. The server
        keeps each day's fold, so a month of seconds is not folded again on every visit.
      </p>
    {:else if !day && instrument.ok && rungs}
      <p class="state" data-no-day="none-held">No day to show: the address names none and no feed holds a stored bar of {key}.</p>
    {:else if !day && instrument.ok && rungsAnswered}
      <p class="state refusal" data-no-day="table-unread">
        No day to show: the address names none, and the rung table that would name the newest day every feed holds was not read (its
        reason is above). That is not a statement about what the feeds hold.
      </p>
    {/if}
    {@render status(barsRead, 'the timeline')}
    {#if bars}
      {#each bars.summary as line, i (i)}<p class="sum">{line}</p>{/each}
      {#if bars.empty}
        <p class="state refusal" data-empty={bars.beyondCursor ? 'beyond-cursor' : 'none-held'}>
          {bars.empty}
          {#if bars.beyondCursor}<a href={hrefWith({})}>First page</a>{/if}
        </p>
      {:else}
        <div class="scroll">
          <table class="s-grid lines">
            <caption class="vh">{bars.rung} buckets of {bars.key}, one row per feed</caption>
            <thead>
              <tr>
                <th scope="col">Bucket (IST)</th>
                <th scope="col">Source</th>
                <th scope="col" class="s-num">Open</th>
                <th scope="col" class="s-num">High</th>
                <th scope="col" class="s-num">Low</th>
                <th scope="col" class="s-num">Close</th>
                <th scope="col" class="s-num">Volume</th>
                {#if bars.showOi}<th scope="col" class="s-num">Open interest</th>{/if}
                <th scope="col">Read from</th>
                {#if bars.showFactor}<th scope="col" class="s-num" title="GDFL ÷ Zerodha on one instant: shown where the price bases differ, in place of a disagreement">Factor</th>{/if}
                <th scope="col">Flags</th>
              </tr>
            </thead>
            {#each bars.groups as g, gi (gi)}
              <tbody data-bucket={g.label}>
                {#each g.rows as r, i (i)}
                  <tr class:absent={Boolean(r.absent)} data-feed={r.feed}>
                    {#if i === 0}<th scope="rowgroup" class="when" rowspan={g.rows.length + (g.cmp ? 1 : 0)}>{g.label}</th>{/if}
                    <th scope="row" class="src">
                      {#if r.db}<a class="s-tag" href={r.db} title="this feed's stored rows for this day on DB">{r.tag}</a>{:else}<span class="s-tag">{r.tag}</span>{/if}
                      {#if r.close}<i>{r.close}</i>{/if}
                    </th>
                    {#if r.absent}
                      <td class="none" colspan={bars.showOi ? 7 : 6}>{r.absent}</td>
                    {:else}
                      <td class="s-num">{r.o}</td>
                      <td class="s-num">{r.h}</td>
                      <td class="s-num">{r.l}</td>
                      <td class="s-num">{r.c}</td>
                      <td class="s-num">{r.v}</td>
                      {#if bars.showOi}<td class="s-num">{r.oi}</td>{/if}
                      <td class="dim">{r.count}{r.count && r.from ? ' from ' : ''}{r.from}</td>
                    {/if}
                    {#if i === 0}
                      {#if bars.showFactor}<td class="s-num t-{g.cmp?.factor.tone ?? 'quiet'}" rowspan={g.rows.length + (g.cmp ? 1 : 0)} title={g.cmp?.factor.title ?? ''}>{g.cmp?.factor.text ?? ''}</td>{/if}
                      <td class="flags" rowspan={g.rows.length + (g.cmp ? 1 : 0)}>
                        {@render chips(g.flags)}
                        {#if g.only}<span class="chip t-warn" title="only this feed holds a bar in this bucket">only {g.only}</span>{/if}
                      </td>
                    {/if}
                  </tr>
                {/each}
                {#if g.cmp}{@render cmpRow(g.cmp, bars.showOi)}{/if}
              </tbody>
            {/each}
          </table>
        </div>
        <p class="s-note">
          {bars.complete ? 'This is the last page of the range.' : 'More buckets follow in this range.'}
          {#if bars.recordsRead !== null}The server read {countText(bars.recordsRead)} stored records for this page.{/if}
          {#if days && days.days.some((/** @type {any} */ d) => d.id === day)}Marks for {day} (halts, refused rows, gaps) are in the day table below.{/if}
        </p>
      {/if}
      {#each bars.notes as note, i (i)}<p class="s-note">{note}</p>{/each}
    {/if}
  </section>

  <!-- ══ 3. THE DAY RECONCILIATION ══════════════════════════════════════════ -->
  <section aria-labelledby="h-days" data-section="days">
    <div class="sec-head">
      <h2 id="h-days">Day by day: official bar, minutes folded, seconds folded</h2>
      {#if daysRead.phase === 'ok'}<span class="took" title="how long this page's request to /sources/day.json took">{took(daysRead.ms)}</span>{/if}
      {#if daysAsk}<a class="raw" href={daysAsk} target="_blank" rel="noopener">JSON</a>{/if}
    </div>
    <div class="controls">
      <label class="f">
        <span class="fk">Month</span>
        <input type="month" value={month} onchange={(e) => e.currentTarget.value && go({ month: e.currentTarget.value, cursor })} />
      </label>
      {#if month}
        {#if shiftMonth(month, -1)}<a class="go" href={hrefWith({ month: shiftMonth(month, -1), cursor })}>← {shiftMonth(month, -1)}</a>{/if}
        {#if shiftMonth(month, 1)}<a class="go" href={hrefWith({ month: shiftMonth(month, 1), cursor })}>{shiftMonth(month, 1)} →</a>{/if}
      {/if}
    </div>
    {@render status(daysRead, 'the day table')}
    {#if days}
      {#each days.summary as line, i (i)}<p class="sum">{line}</p>{/each}
      {#if days.empty}
        <p class="state refusal">{days.empty}</p>
      {:else}
        <div class="scroll">
          <table class="s-grid lines">
            <caption class="vh">Day reconciliation of {days.key}, {days.from} to {days.to}</caption>
            <thead>
              <tr>
                <th scope="col">Day</th>
                <th scope="col">Source</th>
                <th scope="col" class="s-num">Open</th>
                <th scope="col" class="s-num">High</th>
                <th scope="col" class="s-num">Low</th>
                <th scope="col" class="s-num">Close</th>
                <th scope="col" class="s-num">Volume</th>
                <th scope="col">First → last print</th>
                {#if days.showFactor}<th scope="col" class="s-num" title="GDFL ÷ Zerodha on one instant: shown where the price bases differ, in place of a disagreement">Factor</th>{/if}
                <th scope="col">Flags and marks</th>
              </tr>
            </thead>
            {#each days.days as d, di (di)}
              <tbody data-day={d.id} class:equal={d.closeEqual}>
                {#each d.rows as r, i (i)}
                  <tr class:absent={Boolean(r.absent)} data-feed={r.feed}>
                    {#if i === 0}
                      <th scope="rowgroup" class="when" rowspan={d.rows.length + d.cmps.length}>
                        {d.label}
                        {#if d.sub}<i>{d.sub}</i>{/if}
                        <a href={d.timeline} title="the combined 1min timeline of this day">timeline</a>
                      </th>
                    {/if}
                    <th scope="row" class="src">
                      {#if r.db}<a class="s-tag" href={r.db} title="this feed's stored rows for this day on DB">{r.tag}</a>{:else}<span class="s-tag">{r.tag}</span>{/if}
                      {#if r.close}<i>{r.close}</i>{/if}
                      {#if r.outside}<i class="oow-day" title="held on disk, hidden on DB by default, never deleted (D-3338); the link asks DB to show it">{r.outside}</i>{/if}
                    </th>
                    {#if r.absent}
                      <td class="none" colspan="6">{r.absent}</td>
                    {:else}
                      <td class="s-num">{r.o}</td>
                      <td class="s-num">{r.h}</td>
                      <td class="s-num">{r.l}</td>
                      <td class="s-num">{r.c}</td>
                      <td class="s-num">{r.v}</td>
                      <td class="dim">{r.from}{r.from && r.to ? ' → ' : ''}{r.to}{r.count ? ` · ${r.count}` : ''}</td>
                    {/if}
                    {#if i === 0}
                      {#if days.showFactor}
                        <td class="s-num factor t-{d.factor?.tone ?? 'quiet'}" rowspan={d.rows.length + d.cmps.length} title={d.factor?.title ?? ''}>
                          {d.factor?.text ?? ''}
                          {#if d.factor?.volume}<span class="s-why">{d.factor.volume}</span>{/if}
                          {#if d.factor?.minute}<span class="s-why">{d.factor.minute}</span>{/if}
                        </td>
                      {/if}
                      <td class="flags" rowspan={d.rows.length + d.cmps.length}>
                        {@render chips(d.flags)}
                        {#each d.marks as m, j (j)}<span class="s-why t-{m.tone}">{m.text}</span>{/each}
                        {#if d.auction}<span class="s-why">{d.auction}</span>{/if}
                      </td>
                    {/if}
                  </tr>
                {/each}
                {#each d.cmps as c, j (j)}{@render cmpRow(c, false)}{/each}
              </tbody>
            {/each}
          </table>
        </div>
        <p class="s-note">
          {#if days.pages > 1}Page {days.page + 1} of {days.pages}. {/if}
          {#if days.recordsRead !== null}The server read {countText(days.recordsRead)} stored records for this page{days.fromCache ? `, ${days.fromCache} day${days.fromCache === 1 ? '' : 's'} from its kept answers` : ''}.{/if}
        </p>
      {/if}
      {#each days.notes as note, i (i)}<p class="s-note">{note}</p>{/each}
    {/if}
  </section>
</div>

<style>
  .page {
    padding: var(--s5) var(--s5) var(--s8);
    max-width: 1480px;
    margin: 0 auto;
    /* The shell does not scroll (`theme.css` `.main` is `overflow: hidden`),
       so every page provides its own scroller; `min-height: 0` lets it. */
    min-height: 0;
    overflow-y: auto;
  }
  .head {
    margin-bottom: var(--s4);
  }
  h1 {
    font: var(--w-semi) var(--fs-xl) / 1.15 var(--sans);
    letter-spacing: -0.015em;
    color: var(--n11);
    margin: 0 0 var(--s2);
  }
  h1 .of {
    font-family: var(--mono);
    font-weight: 400;
    color: var(--n9);
    margin-left: var(--s2);
    overflow-wrap: anywhere;
  }
  h2 {
    font: var(--w-semi) var(--fs-md, 15px) / 1.2 var(--sans);
    color: var(--n11);
    margin: 0;
  }
  section {
    margin-top: var(--s6);
  }
  .sec-head {
    display: flex;
    align-items: baseline;
    flex-wrap: wrap;
    gap: var(--s3);
    margin-bottom: var(--s2);
  }
  .took {
    font: var(--fs-xs, 11px) var(--mono);
    color: var(--n8);
  }
  .s-sub,
  .s-note,
  .sum,
  .state {
    font-size: var(--fs-sm);
    line-height: 1.55;
    max-width: 110ch;
    margin: var(--s2) 0;
    color: var(--n9);
    overflow-wrap: anywhere;
  }
  .sum {
    color: var(--n11);
  }
  .s-note {
    font-size: var(--fs-xs, 11px);
  }
  a {
    color: var(--info);
  }
  .raw {
    font-size: var(--fs-xs, 11px);
    color: var(--n8);
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    align-items: flex-end;
    gap: var(--s3);
    margin: var(--s3) 0;
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
  .hint {
    font-size: var(--fs-xs, 11px);
    color: var(--n8);
    max-width: 62ch;
    align-self: center;
  }
  select,
  input {
    font: var(--fs-sm) / 1.2 var(--sans);
    color: var(--n11);
    background: var(--n2);
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: var(--s2) var(--s3);
    max-width: 100%;
  }
  input[name='key'] {
    font-family: var(--mono);
  }
  .go {
    font: var(--w-semi) var(--fs-sm) / 1.2 var(--sans);
    color: var(--n11);
    background: var(--n3);
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    padding: var(--s2) var(--s4);
    cursor: pointer;
    text-decoration: none;
  }
  .refusal,
  .failure {
    border: 1px solid var(--n6);
    border-left: 3px solid var(--warn);
    border-radius: var(--r1);
    padding: var(--s3) var(--s4);
    color: var(--n11);
  }
  .failure {
    border-left-color: var(--down);
  }
  .s-feeds {
    list-style: none;
    margin: var(--s2) 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--s1, 4px);
  }
  .s-feed {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--s2) var(--s4);
    font-size: var(--fs-sm);
    color: var(--n9);
  }
  .s-feed b {
    color: var(--n11);
    min-width: 14ch;
  }
  .scroll {
    overflow-x: auto;
    border: 1px solid var(--n6);
    border-radius: var(--r1);
    background: var(--n3);
  }
  .s-grid {
    border-collapse: collapse;
    width: 100%;
    font-size: var(--fs-sm);
    color: var(--n11);
  }
  .s-grid th,
  .s-grid td {
    padding: var(--s2) var(--s3);
    text-align: left;
    vertical-align: top;
    border-bottom: 1px solid var(--n5);
    font-weight: 400;
  }
  .s-grid thead th {
    position: sticky;
    top: 0;
    background: var(--n2);
    font-size: var(--fs-xs, 11px);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--n8);
    white-space: nowrap;
    border-bottom: 1px solid var(--n6);
  }
  .s-grid .s-num {
    text-align: right;
    font-family: var(--mono);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
  .s-grid .src {
    white-space: nowrap;
  }
  .s-tag {
    font-family: var(--mono);
    color: var(--n11);
  }
  .src i,
  .when i,
  .oow i {
    display: block;
    font-style: normal;
    font-size: var(--fs-xs, 11px);
    color: var(--n8);
  }
  .when {
    font-family: var(--mono);
    white-space: nowrap;
    background: var(--n2);
  }
  .when a {
    display: block;
    font: var(--fs-xs, 11px) var(--sans);
  }
  .lines tbody {
    border-bottom: 2px solid var(--n6);
  }
  .lines tr.cmp th,
  .lines tr.cmp td {
    background: var(--n4);
    font-size: var(--fs-xs, 11px);
    color: var(--n9);
  }
  .lines tr.cmp td.s-num {
    white-space: normal;
    min-width: 12ch;
  }
  /* The comparison's label names both sources and may wrap: left on one line
     it sets the Source column's width and pushes Flags off the screen. */
  .lines tr.cmp th.src {
    white-space: normal;
    max-width: 26ch;
  }
  .lines td.dim {
    white-space: nowrap;
  }
  .s-grid tr.absent td.none,
  .dim {
    color: var(--n8);
    font-size: var(--fs-xs, 11px);
  }
  .s-grid tr.k-fold th,
  .s-grid tr.k-fold td {
    color: var(--n9);
  }
  .s-why {
    display: block;
    font: var(--fs-xs, 11px) / 1.45 var(--sans);
    color: var(--n9);
    white-space: normal;
    max-width: 46ch;
    overflow-wrap: anywhere;
  }
  .span span {
    display: block;
    white-space: nowrap;
  }
  /* THE PART OF A RUNG OUTSIDE THE OPERATOR'S WINDOW IS SHADED, never dropped
     (D-3338): hatched, labelled, with its own link to the rows. */
  .oow {
    background: repeating-linear-gradient(135deg, var(--n4), var(--n4) 6px, var(--n0) 6px, var(--n0) 12px);
    border: 1px dashed var(--n7);
    border-radius: var(--r1);
    padding: var(--s1, 4px) var(--s2);
    margin-bottom: var(--s1, 4px);
    color: var(--n9);
  }
  /* A DAY OUTSIDE THE WINDOW carries the same label on its own source row in
     the day table, in the same hatching, so it is never read as an ordinary
     day of the window (the operator's answer 6). */
  .src i.oow-day {
    display: inline-block;
    background: repeating-linear-gradient(135deg, var(--n4), var(--n4) 6px, var(--n0) 6px, var(--n0) 12px);
    border: 1px dashed var(--n7);
    border-radius: var(--r1);
    padding: 0 var(--s1, 4px);
    color: var(--n9);
  }
  .basis span {
    display: block;
    font-size: var(--fs-xs, 11px);
    line-height: 1.5;
    min-width: 26ch;
  }
  .links a {
    margin-right: var(--s1, 4px);
  }
  .lines tr.cmp td i {
    display: block;
    font-style: normal;
    white-space: nowrap;
  }
  .chip {
    display: inline-block;
    font: var(--fs-xs, 11px) / 1.5 var(--sans);
    border: 1px solid var(--n6);
    border-radius: 999px;
    padding: 0 var(--s2);
    margin: 0 var(--s1, 4px) var(--s1, 4px) 0;
    white-space: nowrap;
  }
  .flags {
    min-width: 20ch;
    max-width: 40ch;
  }
  /* One flag a line: side by side they widen the table past the screen. */
  .flags .chip {
    display: block;
    width: fit-content;
    white-space: normal;
    border-radius: var(--r1);
  }
  .t-ok {
    color: var(--up);
  }
  .t-bad {
    color: var(--down);
  }
  .t-warn {
    color: var(--warn);
  }
  .t-info {
    color: var(--info);
  }
  .t-quiet {
    color: var(--n9);
  }
  .chip.t-ok {
    background: var(--up-soft);
    border-color: transparent;
  }
  .chip.t-bad {
    background: var(--down-soft);
    border-color: transparent;
  }
  .chip.t-warn {
    background: var(--warn-soft);
    border-color: transparent;
  }
  .chip.t-info {
    background: var(--info-soft);
    border-color: transparent;
  }
  .vh {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  @media (max-width: 720px) {
    .page {
      padding: var(--s4) var(--s3) var(--s8);
    }
    .hint {
      max-width: 100%;
    }
  }
</style>
