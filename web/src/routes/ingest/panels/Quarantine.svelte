<script module>
  /* THE PANEL CONTRACT (`$lib/ingest-panels.js`): an order and a title. */
  export const order = 40;
  export const title = 'Quarantine of bars that are not the share’s own';
</script>

<script>
  /*
   * THE QUARANTINE PANEL (R4-18; ING-01, RT-DATA-02, Z5, Z6). `cli
   * series-identity` (R4-17) writes a plan naming the bars a Zerodha share
   * holds that are not its own: rows from before it listed, a zero-price day,
   * minutes of another price basis. This panel reads what the quarantine
   * would do with them (`GET /pull/quarantine.json`, nothing written) and
   * sends ONE request per press (`POST /pull/quarantine`) with the reason the
   * ledger keeps beside every month it sets aside. A month named whole is
   * recorded and answered by name; any other is served as a new generation
   * without the named bars, the old one kept byte for byte.
   *
   * Zerodha only: the plan is the Zerodha series-identity scan's. Another
   * feed is said in one line.
   */
  import { ask } from '$lib/ask.js';
  import { answerLine } from '$lib/ingest-panels.js';
  import { CAUSES, MAX_REASON_BYTES, SEGMENTS, answerOf, createQuarantineGate, planUrl, questionFault, reasonFault } from '$lib/quarantine.js';

  /** @type {{ ctx: { feed: string | null, row: any } }} */
  let { ctx } = $props();

  const feed = $derived(ctx?.feed ?? '');

  let symbol = $state('IRFC');
  let segment = $state('CASH');
  let cause = $state('before_listing');
  let from = $state('2018-01-01');
  let to = $state('2026-09-30');
  let reason = $state('');

  /** @type {{ state: 'idle' | 'reading' | 'ready' | 'failed', line: string, report: string }} */
  let plan = $state({ state: 'idle', line: '', report: '' });
  let answer = $state({ ok: false, line: '', report: '' });
  let holder = $state('');

  const question = $derived({ symbol, from, to, cause, segment });
  const questionWhy = $derived(questionFault(question));
  const reasonBytes = $derived(new TextEncoder().encode(reason.trim()).length);
  const reasonWhy = $derived(reason === '' ? null : reasonFault(reason));

  async function readPlan() {
    if (questionWhy) {
      plan = { state: 'failed', line: questionWhy, report: '' };
      return;
    }
    const url = planUrl(question);
    plan = { state: 'reading', line: `Reading ${url}`, report: '' };
    try {
      const r = await ask(url, { ms: 120_000 });
      const got = answerOf(r.status, await r.text());
      plan = { state: got.report ? 'ready' : 'failed', line: got.line, report: got.report };
    } catch (why) {
      plan = { state: 'failed', line: answerLine('GET /pull/quarantine.json', 0, why instanceof Error ? why.message : String(why)), report: '' };
    }
  }

  const gate = createQuarantineGate(async (body) => {
    const r = await ask('/pull/quarantine', {
      ms: 1_800_000,
      method: 'POST',
      headers: { 'content-type': 'application/x-www-form-urlencoded' },
      body
    });
    return { status: r.status, text: await r.text() };
  });

  async function press() {
    holder = `Quarantine of ${symbol.toUpperCase()} ${from}..${to} (${cause}) sent from this tab; the press is held until the server answers.`;
    const done = await gate.press(question, reason);
    holder = '';
    answer = { ok: done.ok, line: done.line, report: done.report };
    if (done.sent) await readPlan();
  }
</script>

<section class="quarantine" aria-label={title}>
  <h3>{title}</h3>
  {#if feed !== 'zerodha'}
    <p class="line">
      The quarantine acts on the Zerodha series-identity plan; {feed || 'no feed'} is selected. Choose Zerodha to plan one.
    </p>
  {:else}
    <form class="ask" onsubmit={(e) => { e.preventDefault(); void readPlan(); }}>
      <label title="The share's store symbol (or ALL), e.g. IRFC, KFINTECH, MAZDOCK, PIIND.">
        Symbol <input bind:value={symbol} maxlength="24" title="The share's store symbol, or ALL" />
      </label>
      <label title="CASH for a share, INDEX for an index.">
        Segment
        <select bind:value={segment} title="CASH for a share, INDEX for an index">
          {#each SEGMENTS as s (s)}<option value={s}>{s}</option>{/each}
        </select>
      </label>
      <label title="The cause the plan measured: before_listing, foreign_level or zero_price.">
        Cause
        <select bind:value={cause} title="The cause the plan measured">
          {#each CAUSES as c (c)}<option value={c}>{c}</option>{/each}
        </select>
      </label>
      <label title="First day, inclusive.">From <input type="date" bind:value={from} title="First day" /></label>
      <label title="Last day, inclusive.">To <input type="date" bind:value={to} title="Last day" /></label>
      <button type="submit" disabled={plan.state === 'reading'} title="Read the plan: GET /pull/quarantine.json. Nothing is written.">Plan</button>
    </form>
    {#if questionWhy}<p class="line bad">{questionWhy}</p>{/if}
    {#if plan.state === 'idle'}
      <p class="line dim">Press Plan to read, per month, the bars the quarantine would set aside and why. Nothing is written.</p>
    {:else}
      <p class="line" class:bad={plan.state === 'failed'}>{plan.line}</p>
    {/if}
    {#if plan.report}<pre class="report" aria-label="The plan">{plan.report}</pre>{/if}
    <label class="why" title={`Why: one line, at most ${MAX_REASON_BYTES} bytes, kept in the ledger beside every month.`}>
      Reason
      <input bind:value={reason} placeholder="why these bars are set aside" title={`One line, at most ${MAX_REASON_BYTES} bytes`} />
      <span class="mono" class:bad={reasonBytes > MAX_REASON_BYTES}>{reasonBytes}/{MAX_REASON_BYTES} bytes</span>
    </label>
    {#if reasonWhy}<p class="line bad">{reasonWhy}</p>{/if}
    <button
      type="button"
      disabled={holder !== '' || plan.state !== 'ready' || reason === '' || reasonWhy !== null || questionWhy !== null}
      title={holder !== '' ? `Held: ${holder}` : plan.state !== 'ready' ? 'Read the plan first' : reason === '' ? 'Name the reason first' : 'POST /pull/quarantine: one request; the old generations are kept'}
      onclick={press}>Apply the quarantine</button
    >
    {#if holder}<p class="line warn" aria-live="polite">{holder}</p>{/if}
    {#if answer.line}<p class="line mono" class:bad={!answer.ok} aria-live="polite">{answer.line}</p>{/if}
    {#if answer.report}<pre class="report" aria-label="What the press did">{answer.report}</pre>{/if}
  {/if}
</section>

<style>
  .quarantine {
    border: 1px solid var(--line);
    border-radius: var(--r2, 8px);
    padding: 12px;
    background: var(--panel-2, transparent);
    color: var(--ink-2, inherit);
    max-width: 100%;
    min-width: 0;
  }
  .ask {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: end;
  }
  .ask label,
  .why {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: 0.85em;
  }
  .why {
    margin-top: 8px;
  }
  .report {
    max-height: 320px;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-size: 0.8em;
    border: 1px solid var(--line);
    padding: 6px;
  }
  .line {
    margin: 6px 0;
  }
  .bad {
    color: var(--bad, #b00020);
  }
  .warn {
    color: var(--warn, #8a5a00);
  }
  .dim {
    opacity: 0.75;
  }
  .mono {
    font-family: var(--mono, ui-monospace, monospace);
  }
</style>
