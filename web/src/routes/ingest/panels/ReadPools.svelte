<script module>
  /* THE PANEL CONTRACT (`$lib/ingest-panels.js`): an order and a title. */
  export const order = 90;
  export const title = 'Read pools: answered, busy, being written, cancelled';
</script>

<script>
  /*
   * THE READ POOLS (R3-08: NEW-04, NEW-21; RA-020, RA-130). Every route that
   * reads a file runs in a named pool with a short wait; a request that finds
   * every slot taken past it is refused 429 with Retry-After, one that meets a
   * writer on its month past its patience 503 "being written, ask again", and
   * one whose reader left is counted cancelled. `GET /pools.json` serves those
   * counts from memory; this panel shows them as two plain tables, once every
   * two seconds at most and never while the page is hidden.
   *
   * FEED-AGNOSTIC: the pools are the server's, not a feed's, so the selected
   * feed changes nothing here.
   */
  import { onMount } from 'svelte';
  import { ask } from '$lib/ask.js';
  import { answerLine, createPoller } from '$lib/ingest-panels.js';
  import { poolTables } from '$lib/read-pools.js';

  /** @type {{ ctx?: any }} */
  let { ctx } = $props();
  void ctx;

  /** @type {ReturnType<typeof poolTables> | null} */
  let tables = $state(null);
  let line = $state('Reading /pools.json');

  const poll = createPoller({
    every: 2000,
    hidden: () => typeof document !== 'undefined' && document.hidden,
    run: async () => {
      try {
        const r = await ask('/pools.json', { ms: 5_000 });
        const text = await r.text();
        /** @type {any} */
        let body = null;
        try {
          body = JSON.parse(text);
        } catch {
          body = null;
        }
        if (!r.ok) {
          line = answerLine('GET /pools.json', r.status, body?.error ?? text.slice(0, 200));
          return;
        }
        const read = poolTables(body);
        tables = read;
        line = read.fault ?? '';
      } catch (why) {
        line = answerLine('GET /pools.json', 0, why instanceof Error ? why.message : String(why));
      }
    }
  });

  onMount(() => {
    const wake = () => poll.wake();
    poll.start();
    poll.wake();
    document.addEventListener('visibilitychange', wake);
    return () => {
      document.removeEventListener('visibilitychange', wake);
      poll.stop();
    };
  });
</script>

<div class="pools">
  {#if line}<p class="line" class:bad={tables?.fault}>{line}</p>{/if}
  {#if tables && !tables.fault}
    <p class="line dim">
      {tables.cores} cores; a full pool is waited on for {tables.waitMs} ms{#if tables.patienceMs !== null}, a
        writer on a month for {tables.patienceMs} ms{/if}. Counts since this server started.
    </p>
    <div class="scroll">
      <table aria-label="Pools">
        <thead>
          <tr>
            <th scope="col">Pool</th>
            <th scope="col" title="Slots on this machine">Slots</th>
            <th scope="col" title="Running now">Running</th>
            <th scope="col" title="Waiting for a slot now">Waiting</th>
            <th scope="col" title="Work run to its end for a reader still there, whatever the route then said: a page, a refusal or being written. The routes below split it by what was said.">Ran</th>
            <th scope="col" title="Refused 429 after the wait, with Retry-After">Busy</th>
            <th scope="col" title="The reader left before its answer">Cancelled</th>
          </tr>
        </thead>
        <tbody>
          {#each tables.pools as p (p.pool)}
            <tr>
              <td class="mono">{p.pool}</td>
              <td>{p.slots}</td>
              <td>{p.running}</td>
              <td>{p.waiting}</td>
              <td>{p.answered}</td>
              <td class:warn={p.busy > 0}>{p.busy}</td>
              <td>{p.cancelled}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
    {#if tables.routes.length === 0}
      <p class="line dim">No route has been asked since this server started.</p>
    {:else}
      <div class="scroll">
        <table aria-label="Routes">
          <thead>
            <tr>
              <th scope="col">Route</th>
              <th scope="col">Answered</th>
              <th scope="col" title="429: every slot stayed taken for the wait">Busy</th>
              <th scope="col" title="503: a writer held the month past the read's patience">Being written</th>
              <th scope="col" title="The reader left before its answer">Cancelled</th>
              <th scope="col" title="Refused as asked (other 4xx)">Refused</th>
              <th scope="col" title="Every other 5xx">Failed</th>
            </tr>
          </thead>
          <tbody>
            {#each tables.routes as r (r.route)}
              <tr>
                <td class="mono">{r.route}</td>
                <td>{r.answered}</td>
                <td class:warn={r.busy > 0}>{r.busy}</td>
                <td class:warn={r.being_written > 0}>{r.being_written}</td>
                <td>{r.cancelled}</td>
                <td>{r.refused}</td>
                <td class:bad={r.failed > 0}>{r.failed}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
    {/if}
  {/if}
</div>

<style>
  .pools {
    max-width: 100%;
    min-width: 0;
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
  }
</style>
