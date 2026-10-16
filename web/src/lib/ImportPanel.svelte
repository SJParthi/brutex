<script>
  /**
   * THE IMPORT PANEL (GDFL today) — /ingest and /audit draw the same one (WP-61;
   * DB-G07, DB-G22, DB-A51).
   *
   * Kinds done and where each stands, the options progress with its measured
   * pace and an ETA LABELLED as an extrapolation, every not-done day with its
   * stage and reason, the days the census refused, the census capacity used
   * and left, and the import runs. Every figure is the server's: the fold is
   * `$lib/import-panel.js`, and this file only lays it out.
   */
  import { censusRefusedSentence, dayRow, importPanel, span } from '$lib/import-panel.js';
  import { group } from '$lib/money.js';
  import { stampLabel } from '$lib/dates.js';

  /**
   * @type {{
   *   summary: any,
   *   store?: any,
   *   archiveTo?: string | null,
   *   optionsLeft?: number | null,
   *   notDone?: any[] | null,
   *   notDoneError?: string | null,
   *   now?: number
   * }}
   */
  let {
    summary,
    store = null,
    archiveTo = null,
    optionsLeft = null,
    notDone = null,
    notDoneError = null,
    now = Date.now()
  } = $props();

  const panel = $derived(importPanel(summary, store, { now, archiveTo, optionsLeft }));
  const days = $derived((notDone ?? []).map(dayRow));
</script>

<section class="imp" aria-label="Import journal" data-import-panel={panel.state}>
  <header>
    <h3>
      Import
      <span class="src">{String(summary?.journal?.path ?? 'no journal path stated').split('/').slice(-2).join('/')}{summary?.route ? ` · ${summary.route}` : ''}</span>
    </h3>
    {#if panel.state === 'active'}
      <span class="tag up" title={panel.why}>importing</span>
    {:else if panel.state === 'quiet'}
      <span class="tag warn" title={panel.why}>no import event recently</span>
    {:else if panel.state === 'error'}
      <span class="tag down">journal unreadable</span>
    {:else}
      <span class="tag">{panel.state}</span>
    {/if}
  </header>
  <p class="why">{panel.why}</p>

  {#if panel.kinds.length > 0}
    <table>
      <thead>
        <tr>
          <th scope="col">Kind</th>
          <th scope="col">Newest day</th>
          <th scope="col">Its outcome</th>
          <th scope="col">Pace (p50 · p90 per day)</th>
          <th scope="col">Newest event</th>
        </tr>
      </thead>
      <tbody>
        {#each panel.kinds as k (k.kind)}
          <tr>
            <td>{k.kind}</td>
            <td class="mono">{k.lastDay ?? '—'}</td>
            <td>{k.lastVerb ?? '—'}</td>
            <td class="mono">{k.p50 === null ? '—' : `${span(k.p50)} · ${span(k.p90 ?? Number.NaN)}`}</td>
            <td title={k.newestAt === null ? 'no event in the log the server reads' : `${stampLabel(k.newestAt)} IST`}
              >{k.age === null ? '—' : `${span(k.age)} ago`}</td
            >
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}

  {#if panel.options}
    <p class="line">
      <b>Options</b> reach <span class="mono">{panel.options.lastDay ?? '—'}</span>{#if panel.archiveTo}
        of an archive ending <span class="mono">{panel.archiveTo}</span>{/if}.
      {#if panel.optionsLeft !== null}
        {group(/** @type {number} */ (panel.optionsLeft))} archive day(s) after it are journalled for the indices and not yet for options.
      {/if}
      {#if panel.eta}<span class="extra">{panel.eta.text}.</span>{/if}
    </p>
  {/if}

  <div class="cells">
    {#each Object.entries(panel.counts ?? {}) as [outcome, n] (outcome)}
      <span class="cell" class:bad={outcome !== 'done' && n > 0}><b>{group(Number(n))}</b> {outcome.replace(/_/g, ' ')}</span>
    {/each}
  </div>

  {#if panel.censusRefused}
    <p class="line" class:bad={panel.censusRefused.count > 0}>{censusRefusedSentence(panel.censusRefused)}</p>
  {/if}

  {#if panel.capacity}
    <p class="line" class:bad={panel.capacity.full}>
      <b>Census capacity</b> ({panel.capacity.segment ?? 'segment'}): {group(panel.capacity.committed)} of
      {group(panel.capacity.limit)} entries used, {group(panel.capacity.headroom)} left{panel.capacity.full
        ? ' — FULL: the next landing in this segment is refused'
        : ''}.
    </p>
  {/if}

  {#if panel.newestNotDone}
    <p class="line bad">
      <b>Newest day not done:</b> {panel.newestNotDone.kind} {panel.newestNotDone.day} —
      {panel.newestNotDone.outcome}{panel.newestNotDone.stage ? `, at stage ${panel.newestNotDone.stage}` : ''}{panel
        .newestNotDone.reason
        ? `, reason ${panel.newestNotDone.reason}`
        : ''}.
    </p>
  {/if}

  {#if notDoneError}
    <p class="line bad">The not-done days could not be read: {notDoneError}</p>
  {:else if days.length > 0}
    <details open={days.length <= 12}>
      <summary>{group(days.length)} day(s) not done, with the importer's reasons</summary>
      <ul class="days">
        {#each days as d (d.key)}
          <li>
            <span class="mono">{d.day}</span> {d.kind} · <b>{d.outcome}</b>{d.stage ? ` · stage ${d.stage}` : ''}{d.reason
              ? ` · ${d.reason}`
              : ''}{d.unsettled ? ' · stage unrecorded (a line written before D-3317)' : ''}
            {#each d.words as w, i (i)}
              <div class="said">{w}</div>
            {/each}
          </li>
        {/each}
      </ul>
    </details>
  {:else if notDone !== null}
    <p class="line">Every journalled day is done.</p>
  {/if}

  {#if panel.runs && panel.runs.length > 0}
    <details>
      <summary>{group(panel.runsTotal)} import run(s) in the journal</summary>
      <ul class="days">
        {#each panel.runs as run (run.id)}
          <li>
            <span class="mono">run {run.id}</span> · line {run.line}
            {#if run.fields?.source} · {run.fields.source}{/if}
            {#if run.fields?.from} · from {run.fields.from}{/if}
            {#if run.fields?.started} · started {stampLabel(Number(run.fields.started) * 1000)} IST{/if}
          </li>
        {/each}
      </ul>
    </details>
  {/if}

  {#each panel.notes ?? [] as note, i (i)}
    <p class="note">{note}</p>
  {/each}
</section>

<style>
  .imp {
    border: 1px solid var(--line);
    border-radius: var(--r2, 8px);
    padding: 12px 14px;
    margin: 12px 0;
    background: var(--panel-2, transparent);
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }
  h3 {
    margin: 0;
    font-size: var(--fs-sm, 13px);
  }
  .src {
    font-weight: 400;
    color: var(--faint);
    margin-left: 6px;
    font-family: var(--mono);
    font-size: var(--fs-xs, 11px);
  }
  .tag {
    font-size: var(--fs-xs, 11px);
    padding: 2px 8px;
    border-radius: var(--r-full, 999px);
    border: 1px solid var(--line);
  }
  .tag.up {
    color: var(--up);
  }
  .tag.warn {
    color: var(--warn);
  }
  .tag.down {
    color: var(--down);
  }
  .why,
  .note {
    color: var(--ink-2, inherit);
    font-size: var(--fs-xs, 12px);
    margin: 6px 0;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: var(--fs-xs, 12px);
    margin: 6px 0;
  }
  th,
  td {
    text-align: left;
    padding: 3px 6px;
    border-bottom: 1px solid var(--line-soft, var(--line));
  }
  .mono {
    font-family: var(--mono);
  }
  .line {
    font-size: var(--fs-xs, 12px);
    margin: 6px 0;
  }
  .bad {
    color: var(--down);
  }
  .extra {
    color: var(--warn);
  }
  .cells {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    font-size: var(--fs-xs, 12px);
    margin: 6px 0;
  }
  .cell.bad {
    color: var(--down);
  }
  .days {
    margin: 4px 0;
    padding-left: 18px;
    font-size: var(--fs-xs, 12px);
    max-height: 320px;
    overflow: auto;
  }
  .said {
    color: var(--faint);
    word-break: break-word;
  }
</style>
