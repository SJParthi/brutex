<script>
  /**
   * SELECTION V6 — the per-rung winners `ledger-v6` committed. D-1578.
   *
   * # What this closes
   *
   * `ledger-v6` commits one sealed Selection V6 block per rung and printed it
   * once, as text, while it ran. No route served the file and no page drew
   * it, so the authoritative chain was terminal-only (audit-20261003
   * gaps-10). `/selection-v6.json` reads the blocks; this draws them.
   *
   * # What is drawn, and what is said beside it
   *
   * Each rung is one of three things and the page says which: absent (the
   * path read is shown), refused (the reason is shown, and no winner from
   * that file), or saved (every block, its counters and its Top-25, the first
   * ten marked). A sealed stored record is not a fresh Selection V6
   * capability, and the server's own scope sentence says so above the table.
   *
   * # Index families only
   *
   * The family filter offers NIFTY and BANKNIFTY and nothing else. Selection
   * V6 holds no equity, and CLAUDE.md §1 forbids one entering it; the server's
   * equity sentence is printed on the page, not left in the payload.
   */
  import { ask } from '$lib/ask.js';
  import { fetchSelectionV6, FAMILIES } from '$lib/selection-v6.js';

  /** @type {string} */
  let family = $state('');
  /** @type {{phase:'idle'|'loading'|'ready'|'failed', body:any, why:string}} */
  let result = $state({ phase: 'idle', body: null, why: '' });

  async function load() {
    result = { phase: 'loading', body: null, why: '' };
    try {
      const body = await fetchSelectionV6(family === '' ? null : family, (url) =>
        ask(url, { cache: 'no-store' })
      );
      result = { phase: 'ready', body, why: '' };
    } catch (error) {
      result = { phase: 'failed', body: null, why: error instanceof Error ? error.message : String(error) };
    }
  }

  $effect(() => {
    void family;
    load();
  });

  /** @param {string} digest */
  const short = (digest) => digest.slice(0, 12);
  /** @param {string|null} ppm */
  const perMille = (ppm) => (ppm === null ? 'undefined' : ppm);
</script>

<svelte:head><title>Selection V6 · brutex</title></svelte:head>

<div class="page">
  <header class="head">
    <h1>Selection V6</h1>
    <p class="sub">
      The per-rung Top-25 that <code>ledger-v6</code> committed under the dashboard's store root,
      read from each sealed block. Index families only.
    </p>
  </header>

  <div class="controls">
    <label class="f">
      <span class="fk">Family</span>
      <select bind:value={family}>
        <option value="">Both</option>
        {#each FAMILIES as name (name)}<option value={name}>{name}</option>{/each}
      </select>
    </label>
    <button class="go" onclick={load} disabled={result.phase === 'loading'}>
      {result.phase === 'loading' ? 'Reading…' : 'Reload'}
    </button>
  </div>

  {#if result.phase === 'failed'}
    <p class="refused" role="alert">{result.why}</p>
  {:else if result.phase === 'ready'}
    <p class="scope">{result.body.scope}</p>
    <p class="equities"><b>Equities excluded.</b> {result.body.equities}</p>
    {#each result.body.rungs as rung (rung.rung)}
      <section class="rung">
        <h2>{rung.rung} <span class="state {rung.status}">{rung.status}</span></h2>
        {#if rung.status === 'absent'}
          <p class="whence">No Selection V6 file at <code>{rung.path}</code>.</p>
        {:else if rung.status === 'refused'}
          <p class="refused">{rung.refusal}</p>
        {:else}
          {#each rung.records as record (record.identity)}
            <div class="record">
              <p class="ids">
                block <code>{short(record.identity)}</code> · population
                <code>{short(record.population)}</code> · execution
                <code>{short(record.execution_completion)}</code> · horizon {record.horizon_bars} bars
              </p>
              <p class="counts">
                considered {record.considered} · admitted {record.admitted} · refused {record.refused}
                · unmeasured {record.unmeasured} · winners {record.winner_count}
                {#each record.families as f (f.family)}
                  · {f.family}: {f.terminal}, {f.candidates} candidates{/each}
              </p>
              {#if record.winners.length === 0}
                <p class="whence">No winner in this block for the family shown.</p>
              {:else}
                <table>
                  <thead>
                    <tr>
                      <th>rank</th><th>family</th><th>side</th><th>score</th><th>pessimistic</th>
                      <th>drawdown</th><th>worst loss</th><th>wins</th><th>losses</th>
                      <th>reward/risk ppm</th><th>strategy</th>
                    </tr>
                  </thead>
                  <tbody>
                    {#each record.winners as w (w.rank)}
                      <tr class:top={w.top_ten}>
                        <td>{w.rank + 1}{w.top_ten ? ' ·10' : ''}</td><td>{w.family}</td>
                        <td>{w.direction}</td><td class="n">{w.score}</td>
                        <td class="n">{w.pessimistic_profit}</td><td class="n">{w.drawdown}</td>
                        <td class="n">{w.worst_loss}</td><td class="n">{w.winning_trades}</td>
                        <td class="n">{w.losing_trades}</td>
                        <td class="n">{perMille(w.reward_to_risk_ppm)}</td>
                        <td><code>{short(w.strategy)}</code></td>
                      </tr>
                    {/each}
                  </tbody>
                </table>
              {/if}
            </div>
          {/each}
        {/if}
      </section>
    {/each}
  {:else}
    <p class="whence">Reading…</p>
  {/if}
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
  .scope,
  .equities,
  .whence,
  .ids,
  .counts {
    font-size: var(--fs-sm);
    color: var(--n9);
    line-height: 1.55;
    max-width: 90ch;
  }
  .refused {
    color: var(--red, #c33);
    font-size: var(--fs-sm);
  }
  .controls {
    display: flex;
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
  .state {
    font-size: var(--fs-xs, 11px);
    text-transform: uppercase;
    margin-left: var(--s2);
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
  }
  td.n {
    text-align: right;
    font-family: var(--mono);
  }
  tr.top td:first-child {
    font-weight: var(--w-semi);
  }
  code {
    font-family: var(--mono);
    color: var(--n11);
  }
</style>
