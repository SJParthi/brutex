<script module>
  /* THE PANEL CONTRACT (`$lib/ingest-panels.js`): an order and a title. */
  export const order = 30;
  export const title = 'Versioned repair of a broker series';
</script>

<script>
  /*
   * THE REPAIR PANEL (R4-01, Z6). `POST /pull/repair` (D-3350) publishes a new
   * generation of a month — held bars kept, offered bars added — and no page
   * could start it. This panel reads the read-only plan
   * (`GET /pull/repair.json`), lists what each month owes, and sends exactly
   * one request per press, with the reason the server journals beside every
   * repaired month. Every body is the server's own template; the panel fills
   * in the reason and nothing else (`$lib/zerodha-repair.js`).
   *
   * FEED-AGNOSTIC: it plans for the feed selected on the page. An archive feed
   * is imported, not pulled, and the panel says so in one line.
   */
  import { onMount } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { stampLabel } from '$lib/dates.js';
  import { RECEIPT_HEADER, notAReceipt } from '$lib/receipt.js';
  import { answerLine, createPoller } from '$lib/ingest-panels.js';
  import { projectionLine } from '$lib/pull-plan.js';
  import { MAX_REASON_BYTES, createRepairGate, planRows, planUrl, reasonFault } from '$lib/zerodha-repair.js';

  /** @type {{ ctx: { feed: string | null, row: any } }} */
  let { ctx } = $props();

  const row = $derived(ctx?.row ?? null);
  const feed = $derived(ctx?.feed ?? '');
  const display = $derived(row?.display ?? feed ?? 'this feed');
  const archive = $derived(row?.archive != null);
  const rungs = $derived([
    ...(Array.isArray(row?.rungs) ? row.rungs : []),
    ...(Array.isArray(row?.derived_rungs) ? row.derived_rungs : [])
  ]);

  let instrument = $state('NIFTY');
  let segment = $state('INDEX');
  let rung = $state('1min');
  let from = $state('');
  let to = $state('');
  let reason = $state('');
  let showAll = $state(false);

  /** @type {{ state: 'idle' | 'reading' | 'ready' | 'failed', line: string, plan: ReturnType<typeof planRows> | null, body: any }} */
  let plan = $state({ state: 'idle', line: '', plan: null, body: null });
  /** The last press's answer, verbatim. */
  let answer = $state('');
  let answerGood = $state(false);
  /** What holds the press, while one runs. */
  let holder = $state('');
  /** The server's pull seat, read while a repair runs. */
  let seat = $state('');

  const reasonBytes = $derived(new TextEncoder().encode(reason.trim()).length);
  const reasonWhy = $derived(reason === '' ? null : reasonFault(reason));
  const projection = $derived(plan.body ? projectionLine(plan.body) : null);
  const shown = $derived(
    (plan.plan?.rows ?? []).filter((r) => showAll || r.owed > 0 || r.repair)
  );

  /* THE WINDOW STARTS AS THE FEED'S OWN (`/feeds.json` `window`), never a
     date written here; a person may narrow it. */
  $effect(() => {
    if (from === '' && typeof row?.window?.from === 'string') from = row.window.from;
    if (to === '' && typeof row?.window?.to === 'string') to = row.window.to;
    if (rungs.length > 0 && !rungs.includes(rung)) rung = rungs.includes('1min') ? '1min' : rungs[0];
  });

  async function readPlan() {
    if (archive || !feed) return;
    const url = planUrl({ feed, instrument: instrument.trim().toUpperCase(), segment, rung, from, to });
    plan = { ...plan, state: 'reading', line: `Reading ${url}` };
    try {
      const r = await ask(url, { ms: 30_000 });
      const text = await r.text();
      /** @type {any} */
      let body = null;
      try {
        body = JSON.parse(text);
      } catch {
        body = null;
      }
      if (!r.ok) {
        plan = { state: 'failed', line: answerLine('GET /pull/repair.json', r.status, body?.error ?? body?.why ?? text.slice(0, 300)), plan: null, body: null };
        return;
      }
      const rows = planRows(body);
      plan = rows.fault
        ? { state: 'failed', line: rows.fault, plan: null, body: null }
        : { state: 'ready', line: '', plan: rows, body };
    } catch (why) {
      plan = { state: 'failed', line: answerLine('GET /pull/repair.json', 0, why instanceof Error ? why.message : String(why)), plan: null, body: null };
    }
  }

  /** ONE request per press; the receipt is read as the page reads every receipt. */
  const gate = createRepairGate(async (body) => {
    const r = await ask('/pull/repair', {
      ms: 600_000,
      method: 'POST',
      headers: { 'content-type': 'application/x-www-form-urlencoded' },
      body
    });
    const html = await r.text();
    const wrong = notAReceipt({ html, status: r.status, ctype: r.headers.get('content-type'), marker: r.headers.get(RECEIPT_HEADER), hasVerdict: true });
    if (wrong) return { status: r.status, verdict: wrong.verdict, reason: wrong.reason };
    const doc = new DOMParser().parseFromString(html, 'text/html');
    const badge = doc.querySelector('.badge');
    const halt = doc.querySelector('.halt');
    return {
      status: r.status,
      verdict: badge?.textContent?.trim() || `HTTP ${r.status}`,
      reason: halt?.textContent?.trim() ?? ''
    };
  });

  const seatPoll = createPoller({
    every: 1000,
    hidden: () => typeof document !== 'undefined' && document.hidden,
    run: async () => {
      try {
        const r = await ask('/ingest/status.json', { ms: 5_000 });
        const j = r.ok ? await r.json() : null;
        seat = r.ok
          ? `The server's pull seat: ${typeof j?.pull_seat === 'string' ? j.pull_seat : 'not stated'}.`
          : answerLine('GET /ingest/status.json', r.status);
      } catch (why) {
        seat = answerLine('GET /ingest/status.json', 0, why instanceof Error ? why.message : String(why));
      }
    }
  });

  /** @param {import('$lib/zerodha-repair.js').PlanRow} m */
  async function press(m) {
    const what = `${plan.plan?.instrument ?? instrument} ${m.month} at ${plan.plan?.rung ?? rung} (${m.from}..${m.to})`;
    holder = `Repair of ${what}, sent from this tab at ${stampLabel(new Date())} IST. The press is held until the server answers.`;
    answer = '';
    seatPoll.start();
    const done = await gate.press(m.body, reason, what, m.member);
    seatPoll.stop();
    holder = '';
    answer = done.line;
    answerGood = done.status !== null && done.status >= 200 && done.status < 300;
    if (done.sent) await readPlan();
  }

  onMount(() => {
    const wake = () => seatPoll.wake();
    document.addEventListener('visibilitychange', wake);
    // THE PLAN IS READ ON A PRESS, NOT ON EVERY LOAD: it is O(days asked) on
    // the server (p50 29 ms, p99 0.3-0.8 s over the whole window, measured on
    // the R4-01 shadow), and most visits to /ingest never open this panel.
    return () => {
      seatPoll.stop();
      document.removeEventListener('visibilitychange', wake);
    };
  });
</script>

<section class="repair" aria-label={title}>
  <h3>{title}</h3>
  {#if !feed}
    <p class="line">No feed is selected.</p>
  {:else if archive}
    <p class="line">
      {display} is imported from its archive, not pulled, so a versioned repair (which re-asks a broker for a
      month) does not apply to it. Choose a pulled feed to plan one.
    </p>
  {:else}
    <form class="ask" onsubmit={(e) => { e.preventDefault(); void readPlan(); }}>
      <label title="The store symbol of the series to plan, e.g. NIFTY or PIIND.">
        Instrument <input bind:value={instrument} title="The store symbol of the series to plan" maxlength="32" />
      </label>
      <label title="INDEX for an index, CASH for a share: the two segments a repair plan reads.">
        Segment
        <select bind:value={segment} title="INDEX for an index, CASH for a share">
          <option value="INDEX">INDEX</option>
          <option value="CASH">CASH</option>
        </select>
      </label>
      <label title={`The rungs ${display} files, from /feeds.json.`}>
        Rung
        <select bind:value={rung} title={`The rungs ${display} files, from /feeds.json`}>
          {#each rungs as r (r)}<option value={r}>{r}</option>{/each}
        </select>
      </label>
      <label title="First day of the plan (at most 3,653 days per plan).">
        From <input type="date" bind:value={from} title="First day of the plan" />
      </label>
      <label title="Last day of the plan.">
        To <input type="date" bind:value={to} title="Last day of the plan" />
      </label>
      <button type="submit" disabled={plan.state === 'reading'} title="Read the plan: GET /pull/repair.json. Read-only; nothing is asked of any vendor.">
        Plan
      </button>
    </form>

    {#if plan.state === 'idle'}
      <p class="line dim">Press Plan to read what a repair of this series would do: GET /pull/repair.json, read-only, nothing asked of any vendor.</p>
    {:else if plan.state === 'reading' || plan.state === 'failed'}
      <p class="line" class:bad={plan.state === 'failed'}>{plan.line}</p>
    {/if}

    {#if plan.plan}
      {@const t = plan.plan.totals}
      <p class="line">
        {plan.plan.instrument} at {plan.plan.rung}, {plan.plan.from} to {plan.plan.to}:
        {t ? `${t.held} of ${t.sessions} session(s) hold a bar; ${t.missing} missing, ${t.repair_sessions} of them only a repair fills, in ${t.repair_months} month(s).` : 'the plan sent no totals.'}
      </p>
      {#if projection}
        <p class="line" class:bad={projection.refused || projection.fault}>{projection.line}</p>
      {/if}
      <label class="all" title="Every month of the window, not only those that owe a session.">
        <input type="checkbox" bind:checked={showAll} title="Show every month of the window" /> every month
      </label>
      <label class="why" title={`Why this repair is run: one line, at most ${MAX_REASON_BYTES} bytes. Journaled with every repaired month in audit/<feed>.repairs.`}>
        Reason
        <input
          bind:value={reason}
          placeholder="why this repair is run"
          title={`One line, at most ${MAX_REASON_BYTES} bytes`}
        />
        <span class="mono" class:bad={reasonBytes > MAX_REASON_BYTES}>{reasonBytes}/{MAX_REASON_BYTES} bytes</span>
      </label>
      {#if reasonWhy}<p class="line bad">{reasonWhy}</p>{/if}
      {#if plan.plan.bodyFaultLine}<p class="line bad">{plan.plan.bodyFaultLine}</p>{/if}
      {#if holder}<p class="line warn" aria-live="polite">{holder} {seat}</p>{/if}
      {#if answer}<p class="line mono" class:bad={!answerGood} aria-live="polite">{answer}</p>{/if}
      <div class="scroll">
        <table>
          <thead>
            <tr>
              <th title="The month file">Month</th>
              <th title="Sessions of record that hold a bar, of the sessions of record">Held</th>
              <th title="Missing sessions between the month's held bars: only a repair fills them">Interior</th>
              <th title="Missing sessions before the month's first held bar: only a repair fills them">Front</th>
              <th title="Every session of record in the month without a bar">Sessions owed</th>
              <th title="The generation the month is served from">Gen</th>
              <th title="One POST /pull/repair for this month, with the reason above">Repair</th>
            </tr>
          </thead>
          <tbody>
            {#each shown as m (m.month)}
              <tr>
                <td class="mono">{m.month}</td>
                <td class="mono">{m.held}/{m.sessions}</td>
                <td class="mono">{m.interior}</td>
                <td class="mono">{m.front}</td>
                <td class="mono">{m.owed}</td>
                <td class="mono">{m.generation ?? '—'}</td>
                <td>
                  {#if m.repair && m.bodyFault !== null}
                    <!-- REFUSED BEFORE IT IS PRESSED: the server's template for
                         this month is not the eight fields for this member. -->
                    <span class="warn mono" title={m.bodyFault}>refused</span>
                  {:else if m.repair}
                    <button
                      type="button"
                      disabled={holder !== '' || reason === '' || reasonWhy !== null}
                      title={holder !== ''
                        ? `Held: ${holder}`
                        : reason === ''
                          ? 'Name the reason first'
                          : `POST /pull/repair for ${m.from}..${m.to}: a new generation of the month, held bars kept`}
                      onclick={() => press(m)}>Repair</button
                    >
                  {:else}
                    <span class="dim" title={m.owed > 0 ? 'What this month owes an ordinary pull fills' : 'Nothing owed'}>—</span>
                  {/if}
                </td>
              </tr>
            {:else}
              <tr><td colspan="7" class="dim">No month in the window owes a session. Tick “every month” to list them all.</td></tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</section>

<style>
  .repair {
    border: 1px solid var(--line);
    border-radius: var(--r2, 8px);
    padding: 12px;
    background: var(--panel-2, transparent);
    color: var(--ink-2, inherit);
    max-width: 100%;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  h3 {
    margin: 0 0 8px;
    font-size: var(--fs-sm, 0.9rem);
  }
  .ask {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: end;
  }
  label {
    display: flex;
    flex-direction: column;
    gap: 2px;
    font-size: var(--fs-xs, 0.8rem);
    min-width: 0;
  }
  .all,
  .why {
    flex-direction: row;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
    margin: 6px 0;
  }
  .why input {
    flex: 1 1 12rem;
    min-width: 0;
  }
  input,
  select {
    max-width: 100%;
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
  }
</style>
