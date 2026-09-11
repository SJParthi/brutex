<script>
  import { ask } from './ask.js';
  import { watchVisible } from './page-requests.js';
  import { fetchInvocationAudit, auditTime } from './invocation-audit.js';
  let expanded = $state(false), before = $state(/** @type {string|null} */ (null));
  let loaded = $state(/** @type {any} */ ({ phase: 'idle', body: null, why: '' }));
  let updated = $state('');
  let refresh = /** @type {(()=>void)|null} */ (null);
  $effect(() => {
    const cursor = before;
    if (!expanded) return;
    loaded = { phase: 'loading', body: null, why: '' };
    const stop = watchVisible(async ticket => {
      try {
        const body = await fetchInvocationAudit(cursor, url => ask(url, { signal: ticket.signal, cache: 'no-store' }));
        if (ticket.current()) { loaded = { phase: 'ready', body, why: '' }; updated = new Date().toLocaleTimeString('en-IN'); }
      } catch (why) { if (ticket.current()) loaded = { phase: 'failed', body: null, why: String(why) }; }
    }, 5000, { visible: () => document.visibilityState === 'visible', listen: wake => { document.addEventListener('visibilitychange', wake); return () => document.removeEventListener('visibilitychange', wake); } });
    refresh = stop.refresh;
    return () => { refresh = null; stop(); };
  });
</script>

<details class="invocations" bind:open={expanded}>
  <summary>Saved run and request history</summary>
  <p>This history survives a process restart. A completed request or command is separate from a completed search and a passing strategy. “Unconfirmed” means a start or progress record exists without a confirmed ending; it does not prove the process is still running.</p>
  <div class="toolbar"><button onclick={() => { before = null; refresh?.(); }}>Latest history</button><button onclick={() => refresh?.()}>Refresh</button>{#if updated}<span>Last read {updated} · refreshes while this panel is visible</span>{/if}</div>
  {#if loaded.phase === 'loading'}<p role="status">Reading at most 32 saved invocations…</p>
  {:else if loaded.phase === 'failed'}<p role="alert">History unavailable: {loaded.why} No empty history or completion is substituted.</p>
  {:else if loaded.phase === 'ready'}
    <div class="scroll"><table><caption>Newest recorded invocations first · timestamps in IST</caption><thead><tr><th>Exact invocation</th><th>Started by</th><th>Operation</th><th>Last recorded outcome</th><th>Time of record</th><th>Recorded boundaries</th><th>HTTP status</th></tr></thead><tbody>{#each loaded.body.records as record (record.invocation)}<tr><th><a href={'/backtest/audit.json?invocation=' + record.invocation}>{record.invocation}</a></th><td>{record.origin === 'browser' ? 'Backtest task' : record.origin === 'cli' ? 'CLI command' : 'Web request'}</td><td>{record.operation}</td><td>{record.status === 'completed' ? (record.origin === 'http' ? 'Response prepared' : 'Command returned') : record.status === 'unconfirmed' ? 'Ending unconfirmed' : record.status}</td><td>{auditTime(record.at_millis)}</td><td>{record.completed_boundaries}</td><td>{record.response_status ?? 'Not an HTTP response'}</td></tr>{/each}</tbody></table></div>
    {#if loaded.body.records.length === 0}<p>No durable invocation records on this page. Older telemetry logs are a separate record.</p>{/if}
    <button disabled={loaded.body.next_before === null} onclick={() => { before = loaded.body.next_before; }}>Earlier history</button>
    <p>{loaded.body.claim}</p>
  {/if}
</details>

<style>
 .invocations{margin:1rem 0;padding:1rem;border:1px solid var(--line,#c9d4df);border-radius:8px}summary{font-weight:600;cursor:pointer}p{font-size:.82rem;line-height:1.6}.toolbar{display:flex;gap:.5rem;align-items:center;flex-wrap:wrap;margin:1rem 0}.toolbar span{font-size:.75rem;opacity:.8}button{background:transparent;color:inherit;border:1px solid var(--line,#c9d4df);border-radius:5px;padding:.5rem .7rem;cursor:pointer}button:disabled{opacity:.4;cursor:default}.scroll{overflow:auto}table{width:100%;border-collapse:collapse;text-align:left;font-size:.78rem;font-variant-numeric:tabular-nums}caption{text-align:left;padding:.6rem 0}th,td{padding:.65rem;border-bottom:1px solid var(--line,#dce3eb);white-space:nowrap}button:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#33839b);outline-offset:3px}
</style>
