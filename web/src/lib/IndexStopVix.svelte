<script>
 import {onDestroy} from 'svelte';
 import {createIndexStopVixReader,indexStopVixBoundaries} from './index-stop-vix.js';
 import {candidateTime} from './candidate-trades.js';
 import {indexStopPoints} from './index-stop-results.js';
 let {identity='',completion='',selected=/** @type {any} */(null),trade=/** @type {any} */(null)}=$props();
 let report=$state.raw(/** @type {any} */({phase:'idle',body:null,selection:null,why:''}));
 const reader=createIndexStopVixReader(value=>{report=value;});
 $effect(()=>{const chosen=selected,savedTrade=trade;reader.close();if(chosen&&savedTrade)void reader.open({identity,completion,selected:chosen,trade:savedTrade});});
 onDestroy(()=>reader.dispose());
</script>

<section class="vix" aria-label="Original VIX reference candles">
 <h5>Original VIX reference</h5>
 <p>These saved one-minute candles describe market context. Each full candle becomes known after its minute completes;
  its high, low and close are not an instantaneous VIX value known at the entry open.
  VIX does not change this setup, its trades or its ranking.</p>
 {#if report.phase==='loading'}<p role="status">Reading this trade’s original VIX reference snapshot…</p>
 {:else if report.phase==='failed'}
  <p role="alert">{report.why}</p><p>The original trade and source-candle panels remain available independently. Current VIX data will not replace missing saved evidence.</p>
  {#if report.selection}<button onclick={()=>reader.open(report.selection)}>Retry this saved reference</button>{/if}
 {:else if report.phase==='ready'}
  {@const body=report.body}{@const row=body.rows[0]}
  <p><b>NSE-INDIAVIX · 1min · {body.feed}</b> · Saved trade {row.trade_index}.
   Reference month: {row.month.year}-{String(row.month.month).padStart(2,'0')}.</p>
  <div class="scroll"><table>
   <caption>Original candles covering the entry minute and exit minute · VIX index points</caption>
   <thead><tr><th>Boundary</th><th>Minute starts · IST</th><th>Saved evidence</th><th>Open</th><th>High</th><th>Low</th><th>Close</th><th>Volume</th><th>Open interest</th></tr></thead>
   <tbody>{#each indexStopVixBoundaries(body) as boundary (boundary.label)}<tr>
    <th>{boundary.label}</th><td>{candidateTime(boundary.micros)}</td><td>{boundary.availability}</td>
    {#if boundary.state==='exact'}
     <td>{indexStopPoints(boundary.candle.open_paisa)}</td><td>{indexStopPoints(boundary.candle.high_paisa)}</td><td>{indexStopPoints(boundary.candle.low_paisa)}</td><td>{indexStopPoints(boundary.candle.close_paisa)}</td><td>{boundary.candle.volume}</td><td>{boundary.candle.open_interest==='-9223372036854775808'?'Not recorded':boundary.candle.open_interest}</td>
    {:else}<td colspan="6" class="reason">{boundary.reason}</td>{/if}
   </tr>{/each}</tbody>
  </table></div>
  {#if row.original_trade.exit_reason==='forced_1510_close'}
   <p>The forced trade close is {candidateTime(row.exit_from_micros)}. The exit reference is the candle starting at {candidateTime(row.exit_bar_micros)}, covering the minute that ends at that close.</p>
  {:else}
   <p>The stop occurred within {candidateTime(row.exit_from_micros)} to {candidateTime(row.exit_until_micros)} (end excluded).
    The exit reference covers that minute; the exact intraminute stop time and instantaneous VIX are unknown.</p>
  {/if}
  <details><summary>Saved reference receipts and availability</summary>
   <p>Across this saved catalog: {body.summary.trades} trades, {body.summary.exact_stamps} exact reference candles,
    {body.summary.absent_stamps} absent minutes and {body.summary.unavailable_stamps} unavailable references.
    Each trade has two reference boundaries; these are not additional trades.</p>
   <dl><dt>Catalog completion</dt><dd>{body.catalog_completion}</dd><dt>Reference identity</dt><dd>{body.reference.identity}</dd><dt>Publication</dt><dd>{body.reference.publication_id}</dd><dt>Reference completion</dt><dd>{body.reference.completion}</dd><dt>Original trade digest</dt><dd>{row.original_trade_digest}</dd><dt>Original month snapshot</dt><dd>{row.month.snapshot_digest??'Unavailable'}</dd><dt>Original month records</dt><dd>{row.month.records??'Unmeasured'}</dd><dt>Saved refusal category</dt><dd>{row.month.unavailable_code??'None'}</dd><dt>Read admission</dt><dd>{body.admitted_bytes} encoded bytes admitted within {body.observation_byte_limit} bytes for this reader; this is not process memory usage.</dd></dl>
  </details>
 {:else if selected?.trades_count==='0'}<p>This saved setting has no completed trade to annotate.</p>
 {:else}<p>Choose “Show candles and VIX” beside a saved trade to inspect its original reference candles.</p>{/if}
</section>

<style>
 .vix{margin:1rem 0;padding:.8rem;border:1px solid var(--line,#cbd6df);border-radius:6px;min-width:0}h5{font-size:.9rem;margin:.3rem 0}p,button,summary,table,dl{font-size:.8rem;line-height:1.6}button{font:inherit;background:transparent;border:1px solid var(--line,#cbd6df);border-radius:5px;color:inherit;padding:.5rem .7rem;cursor:pointer}summary{cursor:pointer}button:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#34849e);outline-offset:2px}.scroll{overflow:auto;max-width:100%}table{border-collapse:collapse;width:100%;text-align:left;font-variant-numeric:tabular-nums}th,td{padding:.45rem;border-bottom:1px solid var(--line,#cbd6df);white-space:nowrap;vertical-align:top}.reason{white-space:normal;min-width:180px;overflow-wrap:anywhere}caption{text-align:left;font-weight:600}dl{display:grid;grid-template-columns:160px minmax(0,1fr);gap:.5rem}dd{margin:0;overflow-wrap:anywhere}[role=alert]{color:var(--down,#ad4944);overflow-wrap:anywhere}@media(max-width:520px){dl{grid-template-columns:1fr}.vix{padding:.5rem}}
</style>
