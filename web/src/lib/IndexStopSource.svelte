<script>
 import {onDestroy} from 'svelte';
 import {createIndexStopSourceReader} from './index-stop-source.js';
 import {catalogDay} from './boolean-catalog.js';
 import IndexStopChart from './IndexStopChart.svelte';
 let {identity='',completion='',selected=/** @type {any} */(null),trade=/** @type {any} */(null)}=$props();
 let report=$state.raw(/** @type {any} */({phase:'idle',body:null,selection:null,why:''}));
 const reader=createIndexStopSourceReader(value=>{report=value;});
 $effect(()=>{
  const chosen=selected,savedTrade=trade;reader.close();if(!chosen)return;
  // Context is only a visible page request. It cannot alter trade execution.
  const before=savedTrade?String(BigInt(savedTrade.entry_bar)<BigInt(chosen.timeframe.slice(0,-3))?BigInt(savedTrade.entry_bar):BigInt(chosen.timeframe.slice(0,-3))):'0';
  void reader.open({identity,completion,selected:chosen,trade:savedTrade,before,after:'0'});
 });
 onDestroy(()=>reader.dispose());
</script>

<section class="source" aria-label="Original entry rules and source candles">
 <h5>Original entry rules and source</h5>
 {#if report.phase==='loading'}<p role="status">Verifying the saved source snapshot and its original condition names…</p>
 {:else if report.phase==='failed'}<p role="alert">{report.why}</p><p>The saved trade table remains available. This panel requires the original source evidence before it can show names or draw candles.</p>{#if report.selection}<button onclick={()=>reader.open(report.selection)}>Retry this exact source</button>{/if}
 {:else if report.phase==='ready'}
  {@const body=report.body}
  <p class="rule"><b>Entry rule:</b> {body.namedRule}</p>
  <p>{body.instrument} · {body.direction} · {body.timeframe} signal · {body.feed} data.
   Original source: {catalogDay(body.source_first_day)}–{catalogDay(body.source_last_day)}.
   Measured period: {catalogDay(body.measurement_first_day)}–{catalogDay(body.measurement_last_day)}.</p>
  {#if body.kind==='candles'}<IndexStopChart candles={body.candles} trade={body.trade} direction={body.direction}/>
  {:else if selected.trades_count==='0'}<p>This setting has no saved trade to chart. Its original entry rules are still shown above.</p>
  {:else}<p>Choose “Show candles and VIX” beside a saved trade to inspect its original one-minute OHLCV.</p>{/if}
  <details><summary>Original build, vocabulary and source receipts</summary>
   <dl><dt>Build</dt><dd>{body.original_build_commit}</dd><dt>Vocabulary version</dt><dd>{body.vocabulary_version}</dd><dt>Source context</dt><dd>{body.source_context_identity}</dd><dt>Source completion</dt><dd>{body.source_context_completion}</dd><dt>Saved source</dt><dd>{body.source_id}</dd><dt>Saved run</dt><dd>{body.run_id}</dd></dl>
   <table><caption>Condition names retained by the original source</caption><thead><tr><th>Saved bit</th><th>Original name</th></tr></thead><tbody>{#each body.condition_names as condition (condition.bit)}<tr><td>{condition.bit}</td><td>{condition.name}</td></tr>{/each}</tbody></table>
  </details>
 {/if}
</section>
<style>
 .source{margin:1rem 0;padding:.8rem;border:1px solid var(--line,#cbd6df);border-radius:6px;min-width:0}h5{font-size:.9rem;margin:.3rem 0}p,button,summary,table,dl{font-size:.8rem;line-height:1.6}.rule{overflow-wrap:anywhere}button{font:inherit;background:transparent;border:1px solid var(--line,#cbd6df);border-radius:5px;color:inherit;padding:.5rem .7rem;cursor:pointer}summary{cursor:pointer}button:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#34849e);outline-offset:2px}dl{display:grid;grid-template-columns:140px minmax(0,1fr);gap:.5rem}dd{margin:0;overflow-wrap:anywhere}table{border-collapse:collapse;width:100%;text-align:left}th,td{padding:.45rem;border-bottom:1px solid var(--line,#cbd6df);overflow-wrap:anywhere}caption{text-align:left;font-weight:600}[role=alert]{color:var(--down,#ad4944);overflow-wrap:anywhere}@media(max-width:520px){dl{grid-template-columns:1fr}.source{padding:.5rem}}
</style>
