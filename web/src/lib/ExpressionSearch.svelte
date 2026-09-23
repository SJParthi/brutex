<script>
  import { ask } from './ask.js';
  import { fetchExpressionSearch } from './expression-search.js';
  import CandidateTrades from './CandidateTrades.svelte';
  let { vocabulary = /** @type {any} */ (null) } = $props();
  let identity = $state('');
  let limit = $state(8);
  let loaded = $state(/** @type {any} */ ({phase:'idle',body:null,why:''}));
  let selected = $state(/** @type {any} */ (null));
  let generation = 0;
  /** @param {any} selection */
  async function load(selection) {
    const ticket = ++generation;
    selected = null;
    loaded = {phase:'loading',body:null,why:''};
    try {
      const body = await fetchExpressionSearch({...selection, limit}, (url) => ask(url, {cache:'no-store'}));
      if (ticket === generation) loaded = {phase:'ready',body,why:''};
    } catch (why) {
      if (ticket === generation) loaded = {phase:'failed',body:null,why:why instanceof Error ? why.message : String(why)};
    }
  }
</script>
<details class="expression-search">
  <summary>Explore an expression search by its saved search ID</summary>
  <p>Paste the search ID printed by the expression-search or expression-backtest command. This reads saved progress and opens each expression’s own trade capture.</p>
  <form onsubmit={(event) => {event.preventDefault(); void load({identity:identity.trim()});}}><label>Search ID <input bind:value={identity} placeholder="64-character search identity" autocomplete="off" spellcheck="false" /></label><label>Checkpoint links per page <select bind:value={limit}><option value={1}>1</option><option value={4}>4</option><option value={8}>8</option><option value={32}>32</option><option value={256}>256</option></select></label><button>Open saved search</button></form>
  {#if loaded.phase === 'loading'}<p role="status">Verifying checkpoint links and exact child receipts…</p>
  {:else if loaded.phase === 'failed'}<p role="alert">Search evidence unavailable: {loaded.why}</p>
  {:else if loaded.phase === 'ready'}
    {@const body = loaded.body}
    {#if body.status === 'missing'}<p>{body.why}</p>
    {:else}
      <h3>{body.state === 'exhausted-observed' ? 'Language exhaustion recorded' : body.state === 'writer-observed' ? 'Writer observed at snapshot time' : body.state === 'paused-or-stopped' ? 'Saved checkpoint; writer not observed' : 'Awaiting first completed checkpoint'}</h3>
      <p class="scope">This is an observed snapshot, not continuous process monitoring. Saved receipt and grammar checks do not reconstruct the original market inputs or establish institutional admission.</p>
      <div class="facts"><span>Evaluated expressions <b>{body.candidates}</b></span><span>Recorded support qualifiers <b>{body.qualifying}</b></span><span>Grammar work <b>{body.work}</b></span><span>Saved signal observations <b>{body.signal_rows}</b></span><span>Unfinished reservations <b>{body.interrupted_reservations}</b></span></div>
      <div class="scroll"><table><caption>Verified expression children, newest first</caption><thead><tr><th>Candidate</th><th>Exact predicate</th><th>True</th><th>False</th><th>Unknown</th><th>Saved pricing</th></tr></thead><tbody>{#each body.rows as row (`${row.identity}-${row.attempt}`)}<tr><td>{row.ordinal}</td><td>{row.expression}<details><summary>Exact child identity</summary><code>{row.identity}</code><br />Attempt {row.attempt}</details></td><td>{row.signals.hits}</td><td>{row.signals.misses}</td><td>{row.signals.unknown}</td><td>{#if row.capture_digest}<button onclick={() => selected = row}>Explore exact trades</button>{:else}No sealed price capture{/if}</td></tr>{/each}</tbody></table></div>
      {#if body.rows.length === 0}<p>No expression child occurred in these {body.links} checked links. {body.next ? 'More checkpoint history remains.' : 'This snapshot has no earlier links.'}</p>{/if}
      <div class="paging"><span>{body.links} checkpoint links checked on this page</span><button disabled={body.next === null} onclick={() => load({identity:body.identity,snapshot:body.snapshot,cursor:body.next})}>Earlier expression receipts</button><button onclick={() => load({identity:body.identity})}>Refresh latest snapshot</button></div>
      {#if selected}{#key `${selected.identity}-${selected.attempt}`}<CandidateTrades identity={selected.identity} attempt={selected.attempt} model="expression" initialDigest={selected.capture_digest} {vocabulary} />{/key}{/if}
    {/if}
  {/if}
</details>
<style>
  .expression-search{border:1px solid #566d7955;border-radius:8px;padding:1rem;margin:1rem 0}summary{cursor:pointer;font-weight:600}p,.scope{font-size:.85rem;max-width:95ch;line-height:1.5}.scope{opacity:.8}form,.facts,.paging{display:flex;align-items:center;gap:1rem;flex-wrap:wrap}.facts{font-size:.85rem;margin:1rem 0}.facts span{display:flex;gap:.4rem}form label{flex:1}input{width:min(100%,40rem);margin:.4rem .6rem}input,button{font:inherit;color:inherit;background:transparent;border:1px solid #506879;border-radius:5px;padding:.5rem .7rem}button{cursor:pointer}button:disabled{opacity:.4;cursor:default}input:focus-visible,button:focus-visible{outline:2px solid #8cd8e5}.scroll{overflow:auto}table{width:100%;border-collapse:collapse;text-align:left;font-size:.8rem}caption{text-align:left;padding:.8rem 0}th,td{padding:.65rem;border-bottom:1px solid #566d7933;vertical-align:top}td:nth-child(2){min-width:24rem;max-width:48rem;overflow-wrap:anywhere}code{overflow-wrap:anywhere;font-size:.72rem}.paging{margin:.8rem 0;font-size:.8rem}[role=alert]{color:#ef998e}
</style>
