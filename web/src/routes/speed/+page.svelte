<script>
  /**
   * SPEED — how fast this server has answered, said by the server (R3-15).
   *
   * # What this closes
   *
   * R3-14 made the server keep, for every route it answers and every import
   * stage it counts, p50, p99 and the max since the start, over the last
   * whole minute and the last hour, and the first (cold) request after the
   * start: `/speed.json`. Nothing drew it, so the only speed sheet anyone read
   * was a hand-run script outside the app, a build behind. This page draws
   * the running build's own figures, stamped with the build that answered
   * (`x-brutex-build`, DB-B71).
   *
   * # What it asks, and when
   *
   * `/speed.json` once when it opens and again only when Read again is
   * pressed. No poll: an idle tab asks for nothing but the header's
   * header-only up check (R3-15 goal 4), and this page is held to that too.
   *
   * # Honest figures
   *
   * Every figure is the server's: a p50 or p99 is the top of the time bucket
   * it falls in (the report's own resolution sentence, printed below), a
   * window that holds no request prints a dash and never 0 µs, and a report
   * missing a field is refused by name (`$lib/speed.js` readSpeedReport).
   */
  import { onMount } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { buildOf, buildLabel } from '$lib/runtime-inspection.js';
  import { stampLabel } from '$lib/dates.js';
  import {
    SPEED_ROUTE, WINDOWS, SORTS, PAGE_TARGETS, MEASURED_PAGES,
    readSpeedReport, speedRows, sortRows, microsText, speedTile, pageSelf
  } from '$lib/speed.js';

  /** @type {{phase: 'reading' | 'ready' | 'failed', report: import('$lib/speed.js').SpeedReport | null,
   *   build: string | null, why: string, at: number}} */
  let read = $state({ phase: 'reading', report: null, build: null, why: '', at: 0 });
  let windowKey = $state('last_hour');
  /** @type {'all' | 'route' | 'stage'} */
  let kind = $state('all');
  let sortKey = $state('order');
  /** @type {ReturnType<typeof pageSelf> | null} */
  let self = $state(null);
  /** @type {AbortController | null} */
  let inFlight = null;

  async function load() {
    inFlight?.abort();
    const controller = new AbortController();
    inFlight = controller;
    read = { ...read, phase: 'reading', why: '' };
    try {
      const r = await ask(SPEED_ROUTE, { cache: 'no-store', signal: controller.signal });
      const ct = r.headers.get('content-type') ?? '';
      if (!ct.includes('json')) throw new Error(`${SPEED_ROUTE} answered ${ct || 'no content-type'}, not JSON — the API is not behind this route`);
      const body = await r.json();
      if (!r.ok) throw new Error(`${SPEED_ROUTE} answered HTTP ${r.status}: ${body?.error ?? 'no reason given'}`);
      const report = readSpeedReport(body);
      if (inFlight !== controller) return;
      read = { phase: 'ready', report, build: buildOf(r.headers), why: '', at: Date.now() };
    } catch (why) {
      if (controller.signal.aborted && inFlight !== controller) return;
      read = { ...read, phase: 'failed', why: String(/** @type {{message?: unknown}} */ (why)?.message ?? why) };
    } finally {
      if (inFlight === controller) inFlight = null;
    }
  }

  onMount(() => {
    void load();
    // THIS PAGE'S OWN LOAD, read once the browser has painted it: the same
    // targets the harness holds every page to, as far as a page can see them.
    const settle = setTimeout(() => (self = pageSelf(typeof performance === 'undefined' ? null : /** @type {any} */ (performance))), 0);
    return () => {
      clearTimeout(settle);
      inFlight?.abort();
    };
  });

  const rows = $derived(read.report ? sortRows(speedRows(read.report, windowKey, kind), sortKey) : []);
  const tile = $derived(read.report ? speedTile(read.report) : null);
  const windowWhy = $derived(WINDOWS.find((w) => w.key === windowKey)?.why ?? '');
  /** @param {number} ms */
  const ago = (ms) => {
    const s = Math.max(0, Math.round(ms / 1000));
    if (s < 120) return `${s} s`;
    if (s < 7200) return `${Math.round(s / 60)} min`;
    return `${(s / 3600).toFixed(1)} h`;
  };
  /** @param {number | null} b */
  const kb = (b) => (b === null ? 'not reported by this browser' : `${(b / 1000).toFixed(1)} KB`);
</script>

<svelte:head><title>Speed · brutex</title></svelte:head>

<div class="page">
  <header class="head">
    <h1>Speed</h1>
    <p class="sub">
      How fast this server has answered each route, kept by the server itself
      (<code>{SPEED_ROUTE}</code>): requests, how they ended, and p50, p99 and the max.
    </p>
  </header>

  <div class="controls">
    <div class="f" role="group" aria-label="Window">
      <span class="fk">Window</span>
      <div class="seg">
        {#each WINDOWS as w (w.key)}
          <button class:on={windowKey === w.key} aria-pressed={windowKey === w.key} title={w.why} onclick={() => (windowKey = w.key)}>{w.label}</button>
        {/each}
      </div>
    </div>
    <label class="f">
      <span class="fk">Rows</span>
      <select bind:value={kind} title="Routes are HTTP routes and the front end; stages are import and checking stages.">
        <option value="all">Routes and stages</option>
        <option value="route">Routes</option>
        <option value="stage">Stages</option>
      </select>
    </label>
    <label class="f">
      <span class="fk">Order</span>
      <select bind:value={sortKey} title="The order the rows are listed in.">
        {#each SORTS as s (s.key)}<option value={s.key}>{s.label}</option>{/each}
      </select>
    </label>
    <button class="go" onclick={load} disabled={read.phase === 'reading'} title="Ask {SPEED_ROUTE} again. This page never asks on its own after it opens.">
      {read.phase === 'reading' ? 'Reading…' : 'Read again'}
    </button>
  </div>

  {#if read.phase === 'failed'}
    <p class="refused" role="alert" data-speed-failed>{read.why}</p>
  {/if}

  {#if read.report}
    {@const report = read.report}
    <p class="stamp" data-speed-build>
      <b class:warn={read.build !== null && read.build.length !== 40}>{buildLabel(read.build)}</b>
      · started {stampLabel(report.started_ms)} ({ago(report.now_ms - report.started_ms)} ago)
      · read {stampLabel(read.at)}
      {#if report.over_cap > 0}
        · <span class="warn">{report.over_cap} requests counted over the route cap, under one row</span>
      {/if}
    </p>
    {#if tile}
      <p class="tile {tile.state}" data-speed-tile title="The health page's speed tile, from this same report.">{tile.line}</p>
    {/if}
    <p class="whence">{windowWhy}</p>

    {#if rows.length === 0}
      <p class="whence">
        No {kind === 'all' ? 'route or stage' : kind} has a row yet: a row exists from the first time its route is asked
        in this server process.
      </p>
    {:else}
      <div class="scroll">
        <table>
          <thead>
            <tr>
              <th title="The matched route, the front end, or stage:&lt;name&gt;">route</th>
              <th class="n" title="Requests in this window, every outcome">n</th>
              <th class="n" title="Answered below 400">answered</th>
              <th class="n" title="Refused as asked: a 4xx that is not 429">refused</th>
              <th class="n" title="Busy: a 429, or a 503 with Retry-After">busy</th>
              <th class="n" title="Failed: any other 5xx">failed</th>
              <th class="n" title="Cancelled: the client left before the answer">cancelled</th>
              <th class="n" title="The median, the top of its time bucket">p50</th>
              <th class="n" title="The 99th percentile, the top of its time bucket">p99</th>
              <th class="n" title="The slowest request in this window">max</th>
              <th class="n" title="The first request this row took after the start: the cold one">first after start</th>
            </tr>
          </thead>
          <tbody>
            {#each rows as r (r.route)}
              <tr class:slow={r.slow} class:failing={r.failed > 0} data-speed-row={r.route}>
                <td><code>{r.route}</code>{#if r.kind === 'stage'} <span class="kind">stage</span>{/if}</td>
                <td class="n">{r.n}</td>
                <td class="n">{r.answered}</td>
                <td class="n">{r.refused}</td>
                <td class="n">{r.busy}</td>
                <td class="n" class:bad={r.failed > 0}>{r.failed}</td>
                <td class="n">{r.cancelled}</td>
                <td class="n">{microsText(r.p50, r.n)}</td>
                <td class="n" title={r.slow ? `Past the ${PAGE_TARGETS.firstContentP99Ms} ms first-content target on its own: a page waiting on it cannot draw in time.` : undefined}>{microsText(r.p99, r.n)}</td>
                <td class="n">{microsText(r.max, r.n)}</td>
                <td class="n">{r.cold === null ? '—' : microsText(r.cold)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if windowKey === 'last_hour'}
        <p class="whence">The last hour holds the minutes each row was frozen for; a row first asked under an hour ago holds fewer.</p>
      {/if}
    {/if}
    <p class="whence resolution">{report.resolution}</p>
  {:else if read.phase === 'reading'}
    <p class="whence">Reading {SPEED_ROUTE}…</p>
  {/if}

  <section class="targets">
    <h2>Page targets</h2>
    <p class="whence">
      Every page is held to these, measured in a browser by <code>web/ci/browser-harness.mjs</code>
      (n = 200, cold and warm): first content p99 under {PAGE_TARGETS.firstContentP99Ms} ms, no pause over
      {PAGE_TARGETS.longestPauseMs} ms, memory under {PAGE_TARGETS.heapBytes / 1_000_000} MB and under
      {PAGE_TARGETS.wireBytes / 1000} KB on the wire — for {MEASURED_PAGES.join(', ')}. What each page ships
      is budgeted at build time (<code>vite.config.js</code>), and every route is timed after each deploy by
      <code>web/ci/speed-sheet.mjs</code>.
    </p>
    {#if self}
      <p class="whence" data-speed-self>
        This page's own load, from this browser:
        first content {self.firstContentMs === null ? 'not reported by this browser' : `${self.firstContentMs} ms`}
        · {kb(self.wireBytes)} on the wire over {self.resources} fetches
        · script heap {kb(self.heapBytes)}.
      </p>
    {/if}
  </section>
</div>

<style>
  .page {
    padding: var(--s5) var(--s5) var(--s8);
    max-width: 1180px;
    margin: 0 auto;
    min-height: 0;
    overflow-y: auto;
  }
  h1 {
    font: var(--w-semi) var(--fs-xl) / 1.15 var(--sans);
    color: var(--n11);
    margin: 0 0 var(--s2);
  }
  h2 {
    font: var(--w-semi) var(--fs-md, 15px) / 1.2 var(--sans);
    color: var(--n11);
    margin: var(--s5) 0 var(--s2);
  }
  .sub,
  .whence,
  .stamp,
  .tile {
    font-size: var(--fs-sm);
    color: var(--n9);
    line-height: 1.55;
    max-width: 90ch;
  }
  .stamp b {
    color: var(--n11);
  }
  .warn {
    color: var(--warn);
  }
  .refused,
  .bad {
    color: var(--down);
  }
  .refused {
    font-size: var(--fs-sm);
  }
  .tile {
    border-left: 3px solid var(--up);
    padding-left: var(--s2);
  }
  .tile.slow {
    border-left-color: var(--warn);
  }
  .tile.failing {
    border-left-color: var(--down);
  }
  .tile.empty {
    border-left-color: var(--n6, #999);
  }
  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: var(--s3);
    align-items: flex-end;
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
  }
  .seg {
    display: flex;
  }
  .seg button {
    border-radius: 0;
  }
  .seg button.on {
    background: var(--acc-soft);
    color: var(--n11);
  }
  .scroll {
    overflow-x: auto;
  }
  table {
    border-collapse: collapse;
    font-size: var(--fs-sm);
    width: 100%;
  }
  th,
  td {
    padding: 2px var(--s2);
    text-align: left;
    border-bottom: 1px solid var(--n4, #ddd);
    white-space: nowrap;
  }
  th.n,
  td.n {
    text-align: right;
    font-family: var(--mono);
  }
  tr.slow td {
    background: var(--warn-soft);
  }
  tr.failing td:first-child {
    box-shadow: inset 3px 0 0 var(--down);
  }
  .kind {
    font-size: var(--fs-xs, 11px);
    text-transform: uppercase;
    color: var(--n9);
  }
  .resolution {
    margin-top: var(--s3);
  }
  code {
    font-family: var(--mono);
    color: var(--n11);
  }
</style>
