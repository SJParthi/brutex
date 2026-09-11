<script>
 import {onDestroy} from 'svelte';
 import {createIndexStopReader,indexStopReasonLabel} from './index-stop-results.js';
 import {candidateTime} from './candidate-trades.js';
 import {indexStopPoints} from './index-stop-results.js';
 import {catalogDay} from './boolean-catalog.js';
 import IndexStopSource from './IndexStopSource.svelte';
 import IndexStopVix from './IndexStopVix.svelte';
 let {initialIdentity='',initialCompletion='',initialSelected=/** @type {any} */(null),autoLoad=false}=$props();
 let input=$state('');let report=$state.raw(/** @type {any} */ ({phase:'idle',body:null,why:'',selection:null}));
 let overview=$state.raw(/** @type {any} */ (null));
 let chartTrade=$state.raw(/** @type {any} */(null));
 const reader=createIndexStopReader(value=>{report=value;if(value.phase==='ready'&&value.body.kind==='settings')overview=value.body;});
 $effect(()=>{input=initialIdentity;overview=null;chartTrade=null;reader.close();if(autoLoad&&initialIdentity)void reader.open({identity:initialIdentity,completion:initialCompletion||null,...(initialSelected?{kind:'trades',setting:initialSelected.index,selected:initialSelected}:{})});});
 onDestroy(()=>reader.dispose());
 /** @param {any} body @param {string} offset */
 const paginate=(body,offset)=>{chartTrade=null;return reader.open({identity:body.identity,completion:body.completion,kind:body.kind,setting:body.setting,selected:body.selected,offset,limit:body.limit});};
 /** @param {any} setting @param {string} kind */
 const inspect=(setting,kind)=>{chartTrade=null;const saved=overview??report.body;if(saved)return reader.open({identity:saved.identity,completion:saved.completion,kind,setting:setting.index,selected:setting});};
 const words=indexStopReasonLabel;
</script>
<section class="index-stop-results" aria-label="Single-stop saved research results">
 <header><div><span>SAVED SINGLE-STOP RESEARCH</span><h3>Compare every saved program and direction</h3></div><b>Costs excluded</b></header>
 <p>One completed signal candle fixes the stop: its low for a long trade, its high for a short trade.
  Entry is the immediate next one-minute open. Exit is that fixed stop or the accepted 15:10 IST close.
  There is no target, trailing stop or holding-horizon alternative in this model.</p>
 <form onsubmit={event=>{event.preventDefault();overview=null;void reader.open({identity:input.trim()});}}>
  <label>Saved result identity <input bind:value={input} autocomplete="off" spellcheck="false" placeholder="Exact saved result ID" /></label><button>Read saved results</button>
 </form>
 {#if overview}
  <p><b>{overview.total} direction settings</b> ({String(BigInt(overview.total)/2n)} programs) in this saved artifact.
   Each program has a long and short evaluation. Pessimistic and optimistic are two readings of those same trades.
   These counts do not describe the full search population.</p>
  <div class="scroll"><table><caption>Exact recorded order; both directions and both fill readings</caption>
   <thead><tr><th>Setting / program</th><th>Instrument / timeframe</th><th>Direction</th><th>Trades</th><th>Pessimistic index points</th><th>Optimistic index points</th><th>Drawdown points</th><th>Inspect</th></tr></thead>
   <tbody>{#each overview.rows as row (row.index)}<tr><th>{row.index}<small>Program {row.program_index}</small></th><td>{row.instrument}<small>{row.timeframe} · {catalogDay(row.first_day)}–{catalogDay(row.last_day)}</small></td><td>{row.direction}</td><td>{row.metrics.trades}</td><td>{indexStopPoints(row.metrics.pessimistic_paisa)}</td><td>{indexStopPoints(row.metrics.optimistic_paisa)}</td><td>{indexStopPoints(row.metrics.drawdown_paisa)}</td><td><button onclick={()=>inspect(row,'trades')}>Trades and evidence</button></td></tr>{/each}</tbody>
  </table></div>
  <div class="pager"><span>{overview.rows.length} settings on this page · starts at {overview.offset}</span><button disabled={overview.offset==='0'} onclick={()=>paginate(overview,String(BigInt(overview.offset)>BigInt(overview.limit)?BigInt(overview.offset)-BigInt(overview.limit):0n))}>Previous settings</button><button disabled={overview.next===null} onclick={()=>paginate(overview,overview.next)}>Next settings</button></div>
 {/if}
 {#if report.phase==='loading'}<p role="status">Authenticating the saved result and reading one bounded page…</p>
 {:else if report.phase==='failed'}<p role="alert">Saved results unavailable: {report.why}</p>{#if report.selection}<button onclick={()=>reader.open(report.selection)}>Retry this exact page</button>{/if}
 {:else if report.phase==='ready'&&report.body.kind!=='settings'}
  {@const body=report.body}{@const row=body.selected}
  <section class="setting"><h4>Setting {row.index} · {row.instrument} · {row.timeframe} · {row.direction}</h4>
   <p><b>Full saved expression:</b> <code>{row.expression}</code>. Numbers identify the saved condition bits; no current indicator names are substituted.</p>
   {#key `${body.identity}:${body.completion}:${row.index}`}<IndexStopSource identity={body.identity} completion={body.completion} selected={row} trade={chartTrade}/><IndexStopVix identity={body.identity} completion={body.completion} selected={row} trade={chartTrade}/>{/key}
   <p>{row.truth.evaluated} evaluated signal bars: {row.truth.hits} definite signals, {row.truth.misses} false,
    {row.truth.unknown} unknown. {row.metrics.unreachable} lacked an immediate entry; {row.metrics.too_late} were too late;
    {row.metrics.gap_invalid} opened at or beyond the stop; {row.metrics.while_open} encountered an occupied position.
    Entry/path/close refusals: {row.metrics.entry_refused}/{row.metrics.path_refused}/{row.metrics.closing_refused}.</p>
   <nav aria-label="Single-stop evidence pages">{#each [['trades','Trades'],['events','Every signal'],['days','Observed days']] as [kind,name]}<button aria-pressed={body.kind===kind} onclick={()=>inspect(row,kind)}>{name}</button>{/each}</nav>
   <div class="scroll"><table>
    {#if body.kind==='trades'}
     <caption>Both saved fill bounds; stop time is a one-minute window, not an invented tick</caption><thead><tr><th>Trade</th><th>Entry · IST</th><th>Entry price</th><th>Fixed stop</th><th>Exit window · IST</th><th>Reason</th><th>Pessimistic points</th><th>Optimistic points</th><th>Original context</th></tr></thead>
     <tbody>{#each body.rows as trade (trade.index)}<tr><th>{trade.index}</th><td>{candidateTime(trade.entry_micros)}</td><td>{indexStopPoints(trade.entry_paisa)}</td><td>{indexStopPoints(trade.stop_paisa)}</td><td>{candidateTime(trade.exit_from_micros)}{#if trade.exit_from_micros!==trade.exit_until_micros}<small>until {candidateTime(trade.exit_until_micros)} (exclusive)</small>{/if}</td><td>{words(trade.exit_reason)}{#if trade.gapped}<small>Opening gap</small>{/if}</td><td>{indexStopPoints(trade.pessimistic_paisa)}</td><td>{indexStopPoints(trade.optimistic_paisa)}</td><td><button aria-pressed={chartTrade?.index===trade.index} onclick={()=>{chartTrade=trade;}}>Show candles and VIX</button></td></tr>{/each}</tbody>
    {:else if body.kind==='events'}
     <caption>Every definite signal, including skipped or refused entries</caption><thead><tr><th>Signal</th><th>Candle closes · IST</th><th>Fixed stop</th><th>Disposition</th><th>Saved trade</th></tr></thead><tbody>{#each body.rows as event (event.index)}<tr><th>{event.signal_bar}</th><td>{candidateTime(event.signal_close_micros)}</td><td>{indexStopPoints(event.stop_paisa)}</td><td>{words(event.reason)}</td><td>{event.trade_index??'No completed trade'}</td></tr>{/each}</tbody>
    {:else}
     <caption>Actually observed days; missing dates are not created as zero-trade days</caption><thead><tr><th>Day</th><th>Trades</th><th>Winning trades</th><th>Pessimistic points</th><th>Optimistic points</th><th>Unavailable minutes</th><th>Closing minute</th></tr></thead><tbody>{#each body.rows as day (day.index)}<tr><th>{catalogDay(day.day)}</th><td>{day.trades}</td><td>{day.wins}</td><td>{indexStopPoints(day.pessimistic_paisa)}</td><td>{indexStopPoints(day.optimistic_paisa)}</td><td>{day.unavailable_minutes}</td><td>{day.close_verified?'Verified':'Unverified'}</td></tr>{/each}</tbody>
    {/if}
   </table></div>
   {#if body.total==='0'}<p>This exact setting records no {body.kind}. A recorded zero does not mean an evidence read failed.</p>{/if}
   <div class="pager"><span>{body.rows.length} of {body.total} records · starts at {body.offset}</span><button disabled={body.offset==='0'} onclick={()=>paginate(body,String(BigInt(body.offset)>BigInt(body.limit)?BigInt(body.offset)-BigInt(body.limit):0n))}>Previous records</button><button disabled={body.next===null} onclick={()=>paginate(body,body.next)}>Next records</button></div>
   <details><summary>Exact source and observation identities</summary><dl><dt>Run</dt><dd>{row.run_id}</dd><dt>Source</dt><dd>{row.source_id}</dd><dt>Observation</dt><dd>{row.evaluation_digest}</dd><dt>Feed label</dt><dd>Unavailable in this saved projection; source identity retained</dd><dt>Completion</dt><dd>{body.completion}</dd></dl></details>
  </section>
 {/if}
 <p class="scope">This panel verifies saved execution observations. Institutional and daily/weekly qualification are separate saved checks; no approval or ranking is inferred here. Returns are gross index points per underlying unit, without an assumed account balance.</p>
</section>
<style>
 .index-stop-results{border:1px solid var(--line,#cad6df);border-radius:9px;margin:1rem 0;padding:1rem;color:var(--n11,#23374c)}header{display:flex;justify-content:space-between;gap:1rem;align-items:center}header span{font-size:.7rem;letter-spacing:.07em}header b{font-size:.75rem}h3{font-size:1.1rem;margin:.4rem 0}h4{font-size:1rem;margin:.4rem 0}p{font-size:.82rem;line-height:1.6}form,label,.pager,nav{display:flex;gap:.6rem;flex-wrap:wrap;align-items:center;margin:.8rem 0}label{flex:1;font-size:.82rem}input{min-width:180px;flex:1}button,input{background:transparent;color:inherit;border:1px solid var(--line,#cad6df);border-radius:5px;padding:.5rem .7rem;font:inherit;font-size:.8rem}button{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:focus-visible,input:focus-visible,summary:focus-visible{outline:2px solid var(--acc,#34879e);outline-offset:2px}button[aria-pressed=true]{background:var(--acc-soft,#e8f2f7)}.scroll{overflow:auto;max-width:100%}table{width:100%;border-collapse:collapse;text-align:left;font-size:.8rem;font-variant-numeric:tabular-nums}caption{text-align:left;font-weight:600;padding:.7rem 0}th,td{padding:.65rem;border-bottom:1px solid var(--line,#cad6df);white-space:nowrap;vertical-align:top}thead{background:var(--subtle,#f5f8fb)}small{display:block;font-size:.72rem;font-weight:400;margin-top:.3rem}.pager span{margin-right:auto;font-size:.8rem}.setting{border-left:3px solid var(--acc,#34879e);padding:.7rem 1rem;margin:1rem 0}code,dd{overflow-wrap:anywhere}dl{display:grid;grid-template-columns:100px minmax(0,1fr);gap:.6rem;font-size:.75rem}dd{margin:0}summary{cursor:pointer;font-size:.8rem}[role=alert]{color:var(--down,#b04a42)}.scope{color:var(--n8,#5c7287)}@media(max-width:520px){.index-stop-results{padding:.7rem}.setting{padding:.5rem}header{align-items:flex-start}dl{grid-template-columns:1fr}}
</style>
