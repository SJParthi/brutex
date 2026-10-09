<script>
  /**
   * THE SCRUB PANEL — the census checked against the files it counts, for the
   * feed `/db` is showing (sobs-10, D-4454).
   *
   * Run on request only, never polled: a scrub opens one bar file per entry
   * the counter holds. It never repairs anything; it reports. Each run is also
   * written to the event log as `api.verify` "scrub", which this panel says so
   * an operator knows where the answer is kept.
   */
  import { ask } from './ask.js';
  import { fetchScrub, verdictHeading, SCRUB_MS } from './store-scrub.js';

  let { feed = '' } = $props();
  let shown = $state(/** @type {any} */ ({ phase: 'idle', result: null, why: '', feed: '' }));
  let at = $state('');
  const stale = $derived(shown.feed !== '' && shown.feed !== feed);

  async function scrub() {
    const asked = feed;
    shown = { phase: 'running', result: null, why: '', feed: asked };
    try {
      const result = await fetchScrub(asked, (url) => ask(url, { cache: 'no-store', ms: SCRUB_MS }));
      if (shown.feed === asked) {
        shown = { phase: 'ready', result, why: '', feed: asked };
        at = new Date().toLocaleTimeString('en-IN');
      }
    } catch (why) {
      if (shown.feed === asked) shown = { phase: 'failed', result: null, why: String(why), feed: asked };
    }
  }
</script>

<details class="scrub">
  <summary>Scrub: check the counter against its files</summary>
  <p>
    Every count on this page comes from the census. A scrub opens each bar file the census counts
    and checks that it exists, holds the counted rows, and starts and ends where the census says. It
    changes nothing. Each scrub is also written to the event log (<code>api.verify</code>), so its
    result can be read in <a href="/logs?target=api.verify" data-sveltekit-reload>the log</a> later.
  </p>
  <div class="toolbar">
    <button disabled={!feed || shown.phase === 'running'} onclick={scrub}>
      {shown.phase === 'running' ? 'Scrubbing…' : `Scrub ${feed || 'this feed'} now`}
    </button>
    {#if at && shown.phase === 'ready'}<span>Scrubbed {shown.feed} at {at}</span>{/if}
  </div>
  {#if stale}
    <p role="status">This result is for {shown.feed}, not {feed}. Scrub again to check {feed}.</p>
  {/if}
  {#if shown.phase === 'running'}
    <p role="status">Opening every bar file {shown.feed} counts. This can take a while on a large store.</p>
  {:else if shown.phase === 'failed'}
    <p role="alert">Scrub unavailable: {shown.why} No result is substituted.</p>
  {:else if shown.phase === 'ready'}
    {@const result = shown.result}
    <h3 class={result.verdict}>{verdictHeading(result.verdict)}</h3>
    <p>{result.say}</p>
    {#if result.counts.length}
      <table>
        <caption>Counts for {result.feed || shown.feed}</caption>
        <tbody>
          {#each result.counts as count (count.key)}
            <tr><th scope="row">{count.label}</th><td>{count.n.toLocaleString('en-IN')}</td></tr>
          {/each}
        </tbody>
      </table>
    {/if}
    {#if result.findings.length}
      <ol class="findings">
        {#each result.findings as finding, i (i)}<li>{finding}</li>{/each}
      </ol>
    {/if}
    {#if result.undrawn > 0}
      <p>{result.undrawn.toLocaleString('en-IN')} more disagreement(s) were counted and are not listed here.</p>
    {/if}
  {/if}
</details>

<style>
  .scrub{margin:1rem 0;padding:1rem;border:1px solid var(--line,#c9d4df);border-radius:8px}
  summary{font-weight:600;cursor:pointer}
  p{font-size:.82rem;line-height:1.6}
  h3{font-size:.9rem;margin:.8rem 0 .2rem}
  h3.disagrees,h3.refused{color:var(--bad,#b4282e)}
  .toolbar{display:flex;gap:.5rem;align-items:center;flex-wrap:wrap;margin:1rem 0}
  .toolbar span{font-size:.75rem;opacity:.8}
  button{background:transparent;color:inherit;border:1px solid var(--line,#c9d4df);border-radius:5px;padding:.5rem .7rem;cursor:pointer}
  button:disabled{opacity:.4;cursor:default}
  table{border-collapse:collapse;font-size:.78rem;font-variant-numeric:tabular-nums}
  caption{text-align:left;padding:.4rem 0}
  th,td{padding:.4rem .65rem;border-bottom:1px solid var(--line,#dce3eb);text-align:left}
  td{text-align:right}
  .findings{font-size:.78rem;line-height:1.5;max-height:16rem;overflow:auto}
  button:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#33839b);outline-offset:3px}
</style>
