<script>
  import { onDestroy } from 'svelte';
  import { ask } from './ask.js';
  import { createReceiptBatch, receiptPlan, MAX_RECEIPT_JOBS } from './receipt-batch.js';

  let {
    feed = '', symbols = /** @type {string[]} */ ([]), rungs = /** @type {string[]} */ ([]),
    supportedRungs = /** @type {string[]} */ ([]), from = '', to = '',
    knobs = /** @type {Record<string,string>} */ ({}), blockedReason = '',
    onbusy = /** @type {(busy:boolean)=>void} */ (() => {}),
    oncomplete = () => {}
  } = $props();
  let minHits = $state('');
  let localWhy = $state('');
  let page = $state(0);
  let checking = $state(false);
  let batchState = $state(/** @type {import('./receipt-batch.js').BatchState} */ ({ phase: 'idle', jobs: [], why: '' }));
  const controller = createReceiptBatch({ request: ask, changed: next => {
    batchState = next;
    onbusy(next.jobs.some(job => ['starting', 'running', 'unknown'].includes(job.phase)));
    if (next.phase === 'done' || next.phase === 'failed' || next.phase === 'stopped') oncomplete();
  } });
  onDestroy(() => { controller.dispose(); onbusy(false); });

  const preparation = $derived.by(() => {
    try { return { plan: receiptPlan({ feed, symbols, rungs, supportedRungs, from, to, minHits, knobs }), why: '' }; }
    catch (error) { return { plan: null, why: error instanceof Error ? error.message : String(error) }; }
  });
  const busy = $derived(batchState.jobs.some(job => ['starting', 'running', 'unknown'].includes(job.phase)));
  const completed = $derived(batchState.jobs.filter(job => job.phase === 'done').length);
  const unresolved = $derived(batchState.jobs.find(job => ['running', 'unknown'].includes(job.phase)));
  const visible = $derived(batchState.jobs.slice(page * 64, (page + 1) * 64));
  const titles = { idle: 'Receipt-checked research', running: 'Receipt-checked batch in progress', done: 'Every queued job completed', failed: 'Batch stopped on refusal', unknown: 'Batch stopped — status unconfirmed', stopped: 'Queue stopped' };
  const labels = { queued: 'Waiting', starting: 'Submitting', running: 'Running', done: 'Completed', failed: 'Refused', unknown: 'Unconfirmed', 'not-started': 'Not started' };

  async function start() {
    if (!preparation.plan || blockedReason || busy) return;
    page = 0;
    localWhy = '';
    try { await controller.start(preparation.plan); }
    catch (error) { localWhy = error instanceof Error ? error.message : String(error); }
  }
  async function recheck() {
    checking = true;
    try { await controller.recheck(); } finally { checking = false; }
  }
</script>

<div class="receipt-batch">
  <h3>{titles[batchState.phase]}</h3>
  <p>Uses checksum receipts for the signal bars, actual one-minute execution bars, and daily context.
    Missing, changed, or invalid required evidence refuses the job. The server owns the receipt location
    and physical input limits; this form does not configure them.</p>
  <p><strong>Intraday only. Forced exit at 15:10 IST.</strong> Daily bars provide context, never a swept
    timeframe. This is cost-excluded research, not institutional admission or a profitability assurance.
    Institutional Selection V6 and its later replay use separate commands.</p>
  <div class="controls">
    <label>Minimum matching bars per job
      <input type="text" inputmode="numeric" bind:value={minHits} disabled={busy}
        autocomplete="off" placeholder="Enter an absolute hit count" />
    </label>
    <button onclick={start} disabled={busy || !!blockedReason || preparation.plan === null}>Run receipt-checked batch</button>
    {#if batchState.phase === 'running'}<button onclick={() => controller.stop()}>Stop after current job</button>{/if}
    {#if batchState.phase === 'unknown' && unresolved?.attempt}
      <button onclick={recheck} disabled={checking}>{checking ? 'Checking…' : 'Recheck this exact attempt'}</button>
    {/if}
  </div>
  {#if preparation.plan}
    <p>{preparation.plan.length} jobs selected: {symbols.length} instruments × {rungs.length} intraday timeframes.
      One job runs at a time with the same absolute hit floor; it is not a common percentage across timeframes.</p>
  {:else}<p class="hint">{preparation.why}</p>{/if}
  {#if blockedReason}<p role="alert">{blockedReason}</p>{/if}
  {#if localWhy}<p role="alert">{localWhy}</p>{/if}
  <p class="hint">Selections and explicit settings are captured when you press Run. A refusal or uncertain
    status stops the remaining jobs; an accepted request is never automatically resent. Closing this page
    stops its queue, while an accepted server job continues. Saved server evidence remains available.
    The browser queue holds at most {MAX_RECEIPT_JOBS} jobs.</p>
  {#if batchState.jobs.length}
    <p role="status">{completed} of {batchState.jobs.length} jobs confirmed complete. {batchState.why}</p>
    <!-- svelte-ignore a11y_no_noninteractive_tabindex (The overflowing region needs keyboard focus for horizontal scrolling.) -->
    <div class="table-scroll" tabindex="0" role="region" aria-label="Receipt-checked batch results; scroll horizontally">
      <table>
        <caption>Captured request and exact attempt for each job</caption>
        <thead><tr><th scope="col">Instrument</th><th scope="col">Timeframe</th><th scope="col">State</th><th scope="col">Audit attempt</th><th scope="col">Evidence</th></tr></thead>
        <tbody>{#each visible as job (`${job.request.underlying}-${job.request.rung}`)}
          <tr><th scope="row">{job.request.underlying}</th><td>{job.request.rung}</td><td>{labels[job.phase]}</td>
            <td>{job.attempt ?? 'Not confirmed'}</td><td>
              <details><summary>Request and result</summary>
                <p>{job.request.feed} · {job.request.from_year}-{String(job.request.from_month).padStart(2, '0')} through
                  {job.request.to_year}-{String(job.request.to_month).padStart(2, '0')} · minimum hits {job.request.min_hits}</p>
                <pre aria-label="Exact submitted request">{JSON.stringify(job.request, null, 2)}</pre>
                {#if job.why}<p role="alert">{job.why}</p>{/if}
                {#if job.report}<pre aria-label="Server result report">{job.report}</pre>{/if}
                {#if !job.report && !job.why}<p>No terminal report has been observed for this job.</p>{/if}
              </details>
            </td></tr>
        {/each}</tbody>
      </table>
    </div>
    {#if batchState.jobs.length > 64}<div class="controls"><button disabled={page === 0} onclick={() => page -= 1}>Previous jobs</button>
      <span>Jobs {page * 64 + 1}–{Math.min((page + 1) * 64, batchState.jobs.length)} of {batchState.jobs.length}</span>
      <button disabled={(page + 1) * 64 >= batchState.jobs.length} onclick={() => page += 1}>Next jobs</button></div>{/if}
  {/if}
</div>

<style>
  .receipt-batch{width:100%;padding:.8rem 0;border-top:1px solid #566d7944}h3{font-size:1rem;margin:.3rem 0}p{font-size:.85rem;line-height:1.5;max-width:110ch}.hint{opacity:.8}.controls{display:flex;align-items:end;gap:.75rem;flex-wrap:wrap}label{display:grid;gap:.35rem;font-size:.85rem}input,button{font:inherit;color:inherit;background:transparent;border:1px solid #506879;border-radius:5px;padding:.5rem .7rem}button{cursor:pointer}button:disabled,input:disabled{opacity:.5;cursor:not-allowed}input:focus-visible,button:focus-visible,summary:focus-visible,.table-scroll:focus-visible{outline:2px solid #8cd8e5;outline-offset:2px}.table-scroll{overflow:auto}table{width:100%;border-collapse:collapse;text-align:left;font-size:.8rem}caption{text-align:left;padding:.8rem 0}th,td{padding:.65rem;border-bottom:1px solid #566d7933;vertical-align:top}pre{white-space:pre-wrap;overflow-wrap:anywhere;max-height:24rem;overflow:auto;font-size:.75rem}summary{cursor:pointer}[role=alert]{color:#ef998e}
</style>
