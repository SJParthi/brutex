<script module>
  /* THE PANEL CONTRACT (`$lib/ingest-panels.js`): an order and a title. */
  export const order = 40;
  export const title = 'Feed checks: reconcile, tick-to-second and excluded days';
</script>

<script>
  /*
   * THE DATA CHECKS (DATA-1w, D-5190..D-5194). Three read-only routes, each
   * the append-only output of one check:
   *
   *  - /feed-checks.json   DATA-1 `cli feed-reconcile`: Zerodha minutes vs
   *                        the Zerodha day bar vs GDFL, per stock-day: status,
   *                        exact factor pair, adjusted, ex-date, minute flags;
   *  - /tick-checks.json   DATA-2 `cli tick-to-second-check`: every stored
   *                        GDFL share-day rebuilt under rule T and rule A;
   *  - /calendar-days.json CAL-1: the days the tradable-day rule excludes.
   *
   * Plain comparison tables. A month leads to its shares, a share to its days,
   * and every day to the source row of the file it came from. Read once when
   * the panel opens and when Reload is pressed: the checks change only when a
   * check run writes a new version, so nothing here polls.
   */
  import { onMount } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { calendarRows, monthRows, readAll, read, address, share, stockRows, tickMonthsOf, words, yearsOf } from '$lib/feed-checks.js';

  /** @type {{ ctx?: any }} */
  let { ctx } = $props();
  void ctx;

  const get = (/** @type {string} */ url) => ask(url, { ms: 15_000 });

  let busy = $state(false);
  /** @type {string[]} */
  let notes = $state([]);
  /** @type {any} */
  let feedHead = $state(null);
  /** @type {any[]} */
  let feedMonths = $state([]);
  /** @type {Record<string, any>} */
  let tickMonths = $state({});
  /** @type {string | null} */
  let tickDir = $state(null);
  /** @type {any} */
  let calendar = $state(null);

  /** @type {string | null} */
  let month = $state(null);
  /** @type {any[]} */
  let stocks = $state([]);
  /** @type {string | null} */
  let symbol = $state(null);
  /** @type {any} */
  let feedDays = $state(null);
  /** @type {any} */
  let tickDays = $state(null);
  /** @type {any} */
  let source = $state(null);
  /** @type {number | null} */
  let calYear = $state(null);
  /** @type {string | null} */
  let calReason = $state(null);
  /** @type {any[]} */
  let calDays = $state([]);

  let rows = $derived(monthRows(feedMonths, tickMonths));
  let cal = $derived(calendarRows(calendar?.years ?? []));

  async function load() {
    busy = true;
    /** @type {string[]} */
    const said = [];
    const feed = await readAll(get, '/feed-checks.json', {});
    if (feed.ok) {
      feedHead = feed.body;
      feedMonths = feed.body.months ?? [];
    } else {
      feedHead = feed.body;
      feedMonths = [];
      said.push(feed.error ?? 'feed checks unread');
    }
    const ticks = await readAll(get, '/tick-checks.json', {});
    /** @type {Record<string, any>} */
    const byMonth = {};
    if (ticks.ok) {
      tickDir = ticks.body.dir ?? null;
      for (const y of ticks.body.years ?? []) {
        if (y?.error) {
          said.push(`tick-to-second ${y.year}: ${y.error}`);
          continue;
        }
        const year = await readAll(get, '/tick-checks.json', { year: y.year });
        if (year.ok) Object.assign(byMonth, tickMonthsOf(year.body));
        else said.push(year.error ?? `tick-to-second ${y.year} unread`);
      }
    } else {
      tickDir = ticks.body?.dir ?? null;
      said.push(ticks.error ?? 'tick checks unread');
    }
    tickMonths = byMonth;
    const days = await readAll(get, '/calendar-days.json', {});
    calendar = days.ok ? days.body : null;
    if (!days.ok) said.push(days.error ?? 'calendar unread');
    notes = said;
    busy = false;
  }

  /** @param {string} m */
  async function openMonth(m) {
    month = m;
    symbol = null;
    feedDays = null;
    tickDays = null;
    source = null;
    const said = [];
    const feed = await readAll(get, '/feed-checks.json', { month: m });
    const tick = await readAll(get, '/tick-checks.json', { month: m });
    if (!feed.ok) said.push(feed.error);
    if (!tick.ok) said.push(tick.error);
    stocks = stockRows(feed.ok ? feed.body.stocks : [], tick.ok ? tick.body.stocks : []);
    notes = said.filter((x) => typeof x === 'string');
  }

  /** @param {string} s */
  async function openStock(s) {
    if (!month) return;
    symbol = s;
    source = null;
    const feed = await readAll(get, '/feed-checks.json', { month, symbol: s });
    const tick = await readAll(get, '/tick-checks.json', { month, symbol: s });
    feedDays = feed;
    tickDays = tick;
  }

  /**
   * @param {'feed' | 'tick'} which
   * @param {number} line
   */
  async function openLine(which, line) {
    if (!month) return;
    const r =
      which === 'feed'
        ? await read(get, address('/feed-checks.json', { month, version: feedDays?.body?.month?.version, line }))
        : await read(get, address('/tick-checks.json', { year: month.slice(0, 4), version: tickDays?.body?.year?.version, line }));
    source = r;
  }

  /**
   * @param {number} y
   * @param {string | null} reason
   */
  async function openYear(y, reason) {
    calYear = y;
    calReason = reason;
    const r = await readAll(get, '/calendar-days.json', { year: y, reason });
    calDays = r.ok ? (r.body.days ?? []) : [];
    if (!r.ok) notes = [r.error ?? `calendar ${y} unread`];
  }

  onMount(() => {
    load();
  });

  const STATUSES = ['matched', 'gdfl_only', 'zerodha_only', 'no_overlap', 'not_aligned', 'read_failed'];
</script>

<div class="checks">
  <p class="line">
    <button type="button" title="Read the three check routes again" onclick={load} disabled={busy}>Reload</button>
    {#if busy}<span class="dim">Reading /feed-checks.json, /tick-checks.json, /calendar-days.json</span>{/if}
  </p>
  {#each notes as n}<p class="line warn mono">{n}</p>{/each}
  {#if feedHead?.dir}<p class="line dim">Feed reconcile read from <span class="mono">{feedHead.dir}</span> ({feedHead.dir_from}); tick-to-second from <span class="mono">{tickDir ?? '–'}</span>.</p>{/if}

  <h4>Per month</h4>
  {#if rows.length === 0}
    <p class="line dim">No month of either check is held yet.</p>
  {:else}
    <div class="scroll">
      <table aria-label="Feed checks per month">
        <thead>
          <tr>
            <th scope="col">Month</th>
            <th scope="col" title="Shares with a row in the newest full feed-reconcile version">Stocks</th>
            <th scope="col" title="Stock-days, every status">Stock-days</th>
            {#each STATUSES as s}<th scope="col" title={`Stock-days whose status is ${s}`}>{s}</th>{/each}
            <th scope="col" title="Shares with at least one day flagged adjusted (a factor other than 1)">Adjusted stocks</th>
            <th scope="col" title="Days flagged ex_date: consecutive matched days with disjoint factor intervals; every one unsourced">Ex-date days</th>
            <th scope="col" title="Days whose Zerodha day bar exceeds its minute sum by more than GDFL explains: measured, not judged (no ENV-1 envelope)">Zerodha-minute-short days</th>
            <th scope="col" title="Tick-to-second share-days counted (one per stored share-day)">Tick share-days</th>
            <th scope="col" title="Share-days whose store equals rule T (trades only) exactly">Rule T exact</th>
            <th scope="col" title="Zerodha minutes whose high the rule-A seconds match exactly, raw basis">Rule A minute-high match</th>
          </tr>
        </thead>
        <tbody>
          {#each rows as r (r.month)}
            <tr class:picked={r.month === month}>
              <td><button type="button" class="link mono" title={`Open the shares of ${r.month}`} onclick={() => openMonth(r.month)}>{r.month}</button></td>
              {#if r.feedError}
                <td class="bad mono" colspan={STATUSES.length + 5}>{r.feedError}</td>
              {:else}
                <td>{r.stocks ?? '–'}</td>
                <td>{r.stockDays ?? '–'}</td>
                {#each STATUSES as s}<td class:warn={s === 'read_failed' && (r.byStatus?.[s] ?? 0) > 0}>{r.byStatus ? (r.byStatus[s] ?? 0) : '–'}</td>{/each}
                <td>{r.adjustedStocks ?? '–'}</td>
                <td>{r.exDateDays ?? '–'}</td>
                <td>{r.shortDays ?? '–'}</td>
              {/if}
              <td>{r.tickShareDays ?? '–'}</td>
              <td>{r.ruleT}</td>
              <td>{r.ruleAHigh}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if month}
    <h4>Shares of {month}</h4>
    <div class="scroll">
      <table aria-label={`Shares of ${month}`}>
        <thead>
          <tr>
            <th scope="col">Share</th>
            <th scope="col" title="Feed-reconcile stock-days">Days</th>
            <th scope="col" title="Feed-reconcile stock-days by status">Statuses</th>
            <th scope="col" title="Days flagged adjusted">Adjusted</th>
            <th scope="col" title="Days flagged ex_date">Ex-date</th>
            <th scope="col" title="Zerodha-minute-short days (measured, not judged)">Short</th>
            <th scope="col" title="Every flag the month's rows carry, with its count">Flags</th>
            <th scope="col" title="Tick-to-second share-days by status">Tick statuses</th>
            <th scope="col" title="Rule T exact share-days">Rule T exact</th>
            <th scope="col" title="Rule A minute-high match, raw basis">Rule A minute-high</th>
          </tr>
        </thead>
        <tbody>
          {#each stocks as s (s.symbol)}
            {@const c = s.feed?.counts}
            {@const t = s.tick?.counts}
            <tr class:picked={s.symbol === symbol}>
              <td><button type="button" class="link mono" title={`Open the days of ${s.symbol} in ${month}`} onclick={() => openStock(s.symbol)}>{s.symbol}</button></td>
              <td>{c?.stock_days ?? '–'}</td>
              <td class="mono">{words(c?.by_status)}</td>
              <td>{c?.adjusted_days ?? '–'}</td>
              <td>{c?.ex_date_days ?? '–'}</td>
              <td>{c?.zerodha_minute_short_days ?? '–'}</td>
              <td class="mono">{words(c?.flags)}</td>
              <td class="mono">{words(t?.by_status)}</td>
              <td>{t ? share(t.rule_t_exact?.share_days, t.rule_t_exact?.of) : '–'}</td>
              <td>{t ? share(t.minute_match?.a?.high_match, t.minute_match?.a?.compared) : '–'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}

  {#if symbol}
    <h4>{symbol} in {month}</h4>
    {#if feedDays && !feedDays.ok}<p class="line warn mono">{feedDays.error}</p>{/if}
    {#if feedDays?.ok}
      <div class="scroll">
        <table aria-label={`Feed reconcile days of ${symbol}`}>
          <thead>
            <tr>
              <th scope="col">Day</th>
              <th scope="col" title="The calendar's reason the day takes no trade; empty when it is tradable">Excluded</th>
              <th scope="col">Status</th>
              <th scope="col" title="Zerodha paisa / GDFL paisa of the day's median matched minute">Factor pair</th>
              <th scope="col">Adjusted</th>
              <th scope="col">Ex-date</th>
              <th scope="col">Flags</th>
              <th scope="col" title="Shares; measured, not judged">Short volume</th>
              <th scope="col" title="The row in the version file">Source row</th>
            </tr>
          </thead>
          <tbody>
            {#each feedDays.body.days as d (d.line)}
              <tr>
                <td class="mono">{d.day}</td>
                <td class:warn={d.tradable}>{d.tradable ?? ''}</td>
                <td class="mono">{d.status}</td>
                <td class="mono">{d.factor ? `${d.factor.zerodha_paisa} / ${d.factor.gdfl_paisa}` : '–'}</td>
                <td>{d.adjusted ? 'yes' : ''}</td>
                <td>{d.ex_date}</td>
                <td class="mono">{(d.flags ?? []).join(' ')}</td>
                <td>{d.short_volume ?? '–'}</td>
                <td><button type="button" class="link mono" title="Show this row of the feed-reconcile file" onclick={() => openLine('feed', d.line)}>line {d.line}</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
    {#if tickDays && !tickDays.ok}<p class="line warn mono">{tickDays.error}</p>{/if}
    {#if tickDays?.ok}
      <div class="scroll">
        <table aria-label={`Tick-to-second days of ${symbol}`}>
          <thead>
            <tr>
              <th scope="col">Day</th>
              <th scope="col">Role</th>
              <th scope="col" title="Whether this row is the one the year's index counts for the share-day">Counted</th>
              <th scope="col">Status</th>
              <th scope="col">Compare</th>
              <th scope="col" title="Rule A: Zerodha minutes compared / high matched / low matched">Rule A minutes</th>
              <th scope="col" title="Stored seconds compared / worse under rule A">Fill</th>
              <th scope="col" title="The row in the version file">Source row</th>
            </tr>
          </thead>
          <tbody>
            {#each tickDays.body.days as d (d.line)}
              <tr>
                <td class="mono">{d.day}</td>
                <td>{d.role}</td>
                <td>{d.counted ? 'yes' : 'no'}</td>
                <td class="mono">{d.status}</td>
                <td class="mono">{d.compare}</td>
                <td class="mono">{(d.minutes?.a ?? []).join(' / ')}</td>
                <td class="mono">{(d.fill ?? []).join(' / ')}</td>
                <td><button type="button" class="link mono" title="Show this row of the tick-to-second file" onclick={() => openLine('tick', d.line)}>line {d.line}</button></td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}

  {#if source}
    {#if !source.ok}
      <p class="line warn mono">{source.error}</p>
    {:else}
      <h4>Source row: <span class="mono">{source.body.source.file}</span> line {source.body.source.line}</h4>
      <div class="scroll">
        <table aria-label="Source row">
          <tbody>
            {#each source.body.source.cells as [name, value]}
              <tr><th scope="row" class="mono">{name}</th><td class="mono">{value}</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}

  <h4>Excluded special days per year</h4>
  {#if calendar}
    <p class="line dim">{calendar.rule}</p>
    <div class="scroll">
      <table aria-label="Excluded days per year">
        <thead>
          <tr>
            <th scope="col">Year</th>
            <th scope="col" title="Days the rule admits">Tradable</th>
            <th scope="col" title="Sessions the exchange traded that the rule excludes">Excluded sessions</th>
            {#each cal.columns as c}<th scope="col" title={`Days excluded as ${c}`}>{c}</th>{/each}
            <th scope="col" title="Ordinary closed weekends: counted, never listed">Weekends</th>
            <th scope="col" title="Days no table explains">Unexplained</th>
            <th scope="col" title="How the year's holidays are on record">Holidays</th>
          </tr>
        </thead>
        <tbody>
          {#each cal.rows as y (y.year)}
            <tr class:picked={y.year === calYear}>
              <td><button type="button" class="link mono" title={`List the excluded days of ${y.year}`} onclick={() => openYear(y.year, null)} disabled={!!y.refused}>{y.year}</button></td>
              {#if y.refused}
                <td class="warn" colspan={cal.columns.length + 5}>{y.refused}</td>
              {:else}
                <td>{y.tradable}</td>
                <td>{y.excluded}</td>
                {#each cal.columns as c, i}
                  <td>{#if y.counts[i] > 0}<button type="button" class="link" title={`List the ${c} days of ${y.year}`} onclick={() => openYear(y.year, c)}>{y.counts[i]}</button>{:else}0{/if}</td>
                {/each}
                <td>{y.weekends}</td>
                <td class:bad={y.unexplained > 0}>{y.unexplained}</td>
                <td class="mono">{y.holidays}</td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  {#if calYear !== null && calDays.length > 0}
    <h4>{calReason ?? 'Excluded days'} in {calYear}</h4>
    <div class="scroll">
      <table aria-label={`Excluded days of ${calYear}`}>
        <thead>
          <tr>
            <th scope="col">Day</th>
            <th scope="col">Weekday</th>
            <th scope="col">Reason</th>
            <th scope="col">What it was</th>
            <th scope="col" title="The notice, circular or evidence the day rests on">Source</th>
          </tr>
        </thead>
        <tbody>
          {#each calDays as d (d.day)}
            <tr>
              <td class="mono">{d.day}</td>
              <td>{d.weekday}</td>
              <td class:bad={d.unexplained}>{d.reason}</td>
              <td>{d.label}{#if d.holiday}; {d.holiday.name}{/if}</td>
              <td class="mono">
                {#if d.notice_url}<a href={d.notice_url} rel="noreferrer" title="The exchange's notice">{d.notice}</a>
                {:else}{d.notice ?? d.holiday?.circular ?? d.special_evidence ?? d.halt?.evidence ?? '–'}{/if}
              </td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</div>

<style>
  .checks {
    max-width: 100%;
    min-width: 0;
  }
  h4 {
    margin: 12px 0 4px;
    font-size: var(--fs-sm, 0.9rem);
  }
  .line {
    margin: 6px 0;
    font-size: var(--fs-xs, 0.8rem);
  }
  .bad {
    color: var(--bad, var(--down));
  }
  .warn {
    color: var(--warn);
  }
  .dim {
    color: var(--faint);
  }
  .mono {
    font-family: var(--mono, monospace);
  }
  .scroll {
    overflow-x: auto;
    max-width: 100%;
    margin: 6px 0;
  }
  table {
    border-collapse: collapse;
    font-size: var(--fs-xs, 0.8rem);
  }
  th,
  td {
    padding: 2px 8px;
    border-bottom: 1px solid var(--line-soft, var(--line));
    text-align: left;
    vertical-align: top;
  }
  tr.picked td {
    background: var(--hover, transparent);
  }
  button.link {
    background: none;
    border: 0;
    padding: 0;
    color: var(--accent, inherit);
    text-decoration: underline;
    cursor: pointer;
    font: inherit;
  }
</style>
