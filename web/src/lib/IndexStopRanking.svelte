<script>
 import {onDestroy,untrack} from 'svelte';
 import {CAMPAIGN_RUNGS} from './boolean-campaign.js';
 import {createIndexStopRankingReader,indexStopRankingDetail} from './index-stop-ranking.js';
 import {indexStopPoints} from './index-stop-results.js';
 import {indexConsistencyLabel,indexConsistencyReasons} from './index-consistency.js';
 import IndexStopQualification from './IndexStopQualification.svelte';
 let {searchIdentity='',timeframes=[],refreshKey='',autoLoad=false}=$props();
 let timeframe=$state(''),filter=$state('qualified'),selected=$state.raw(/** @type {any} */(null)),report=$state.raw(/** @type {any} */({phase:'idle',body:null,why:'',selection:null}));
 const reader=createIndexStopRankingReader(value=>{report=value;if(value.phase==='ready')selected=null;});
 const allowed=$derived(CAMPAIGN_RUNGS.filter(label=>timeframes.includes(label)));
 $effect(()=>{const identity=searchIdentity,labels=allowed,key=refreshKey,start=autoLoad;untrack(()=>{if(!labels.includes(timeframe))timeframe=labels[0]??'';if(start&&identity&&timeframe){void key;void reader.open({identity,timeframe,filter});}else reader.close();});});
 onDestroy(()=>reader.dispose());
 /** @param {string} label */function chooseTimeframe(label){timeframe=label;selected=null;void reader.open({identity:searchIdentity,timeframe,filter});}
 /** @param {string} next */function chooseFilter(next){filter=next;selected=null;const body=report.body;void reader.open({identity:searchIdentity,timeframe,filter,checkpoint:body?.timeframe===timeframe?body.checkpoint:null});}
 /** @param {string} [offset] */function paginate(offset='0'){const body=report.body;selected=null;void reader.open({identity:body.identity,timeframe:body.timeframe,filter:body.filter,checkpoint:body.checkpoint,offset});}
 /** @param {any} row */function inspect(row){selected={...indexStopRankingDetail(row),batch:row.batch};}
 /** @param {any} summary */function share(summary){const total=BigInt(summary.eligible_days);if(total===0n)return 'Unavailable';const value=BigInt(summary.winning_days)*10000n/total;return `${value/100n}.${String(value%100n).padStart(2,'0')}%`;}
 /** @param {string} value */const verdict=value=>({admitted:'Passed',rejected:'Failed',refused:'Refused',unmeasured:'Unmeasured'})[/** @type {'admitted'|'rejected'|'refused'|'unmeasured'} */(value)];
</script>

<section class="cumulative-results" aria-label="Best results across acknowledged batches">
 <h3>Best saved results across batches</h3>
 <p>Compare every setting saved for one timeframe, ordered by later pessimistic index points. Optimistic points remain a comparison reading. Every result uses the signal-candle risk stop and the 15:10 exit; costs are excluded.</p>
 <div class="controls" aria-label="Cumulative result filters">
  <div class="timeframes" aria-label="Saved result timeframe">{#each allowed as label (label)}<button class:active={timeframe===label} aria-pressed={timeframe===label} onclick={()=>chooseTimeframe(label)}>{label}</button>{/each}</div>
  <label>Show <select value={filter} onchange={event=>chooseFilter(event.currentTarget.value)}><option value="qualified">Passed both checks</option><option value="all">All outcomes, including failures</option></select></label>
  <button disabled={!searchIdentity||!timeframe} onclick={()=>reader.open({identity:searchIdentity,timeframe,filter})}>Refresh saved results</button>
 </div>
 {#if report.phase==='loading'}<p role="status">Checking the complete saved scope for {report.selection.timeframe}. {report.body?'The previous verified table remains below with its original scope.':'The latest-batch comparison remains available while this read is checked.'}</p>{/if}
 {#if report.phase==='failed'}<p role="alert">The requested cumulative comparison is unavailable: {report.why} {report.body?'Showing the previous verified scope below.':'Individual saved batch comparisons remain available.'}</p>{/if}
 {#if report.body}
  {@const body=report.body}
  <p class="scope"><b>{body.timeframe} · {body.scope.completed_batches} acknowledged batches</b> · {body.all_total} settings across {body.scope.included_families} saved families.
   {#if body.scope.first_batch!==null}Included nonempty batches: {body.scope.first_batch}–{body.scope.last_batch} (numbering starts at 0).{/if}
   {#if body.scope.pending_next_batch}<b>The next batch is pending and is excluded from this table.</b>{/if}
   {#if body.scope.exhausted}The saved checkpoint marks this declared grammar exhausted.{:else}<b>The declared search is not exhausted.</b>{/if}
   These ranks cover this verified saved scope. They do not establish the strongest 0.1% of all possible setups or future profitability.</p>
  <div class="summary"><span><b>Passed both checks</b>{body.summary.combined_qualified}</span><span><b>Institutional passed</b>{body.summary.institutional_admitted}</span><span><b>Failed</b>{body.summary.rejected}</span><span><b>Refused / unmeasured</b>{body.summary.refused} / {body.summary.unmeasured}</span></div>
  {#if body.rows.length===0}<p class="empty">{body.filter==='qualified'?'No setting in this verified scope passed both requirements. Choose All outcomes to inspect the recorded reasons.':'This checkpoint has no acknowledged candidate results for this timeframe.'}</p>{:else}
   <div class="table-scroll" role="region" aria-label="Cumulative saved result comparison"><table><caption>{body.timeframe}: {body.filter==='qualified'?'settings that passed both checks':'all outcomes'} · later pessimistic order across all included batches</caption><thead><tr><th>Rank / saved batch</th><th>Setup / direction</th><th>Later pessimistic / optimistic</th><th>Training pessimistic / optimistic</th><th>Winning days</th><th>Full weeks passed / failed</th><th>Longest losing streak</th><th>Institutional</th><th>Daily / weekly rule</th><th>Combined</th><th>Inspect</th></tr></thead>
    <tbody>{#each body.rows as row (row.batch+':'+row.index)}{@const daily=row.index_consistency.periods.full.summary}<tr><th>{row.rank}<small>Batch {row.batch} · setting {row.index}</small></th><td>{row.original.instrument}<small>Program {row.original.program_index} · {row.original.direction}</small></td><td>{indexStopPoints(row.later.metrics.pessimistic_paisa)}<small>{indexStopPoints(row.later.metrics.optimistic_paisa)} · {row.later.metrics.trades} trades</small></td><td>{indexStopPoints(row.original.metrics.pessimistic_paisa)}<small>{indexStopPoints(row.original.metrics.optimistic_paisa)}</small></td><td>{share(daily)}<small>{daily.winning_days} / {daily.eligible_days} eligible days</small></td><td>{daily.passing_weeks} / {daily.failing_weeks}<small>{daily.complete_weeks} complete weeks</small></td><td>{daily.longest_losing_streak} days</td><td>{verdict(row.institutional.status)}</td><td>{indexConsistencyLabel(row.index_consistency.state)}</td><td>{row.index_consistency.combined_qualifies?'Both passed':'Does not qualify'}<small>{indexConsistencyReasons(row.index_consistency).slice(0,2).join(' ')}</small></td><td><button onclick={()=>inspect(row)}>Details and trades</button></td></tr>{/each}</tbody>
   </table></div>
  {/if}
  <div class="controls"><span>{body.rows.length} of {body.total} in this filter · page starts at {body.offset}</span><button disabled={body.offset==='0'} onclick={()=>paginate(String(BigInt(body.offset)>16n?BigInt(body.offset)-16n:0n))}>Previous results</button><button disabled={body.next===null} onclick={()=>paginate(body.next)}>Next results</button></div>
  <details><summary>Exact saved scope and read limits</summary><p>Search <code>{body.identity}</code>. Checkpoint {body.checkpoint.sequence}: <code>{body.checkpoint.completion}</code>.</p><p>{body.scope.feed} · {body.scope.instrument}. Acknowledged programs: {body.scope.completed_programs}; syntax work: {body.scope.completed_work}. Retained serialized evidence: {body.read_work.retained_serialized_bytes} bytes. Cold verification grows with saved history and sorts the whole admitted scope before paging. A warm page checks retained file generations and copies at most {body.limit} rows. Storage latency and total work are not O(1).</p></details>
  {#if selected}<section class="detail"><h4>Exact saved batch {selected.batch} · setting {selected.setting}</h4><IndexStopQualification initialIdentity={selected.identity} initialCompletion={selected.completion} initialOffset={selected.offset} initialSetting={selected.setting} autoLoad={true}/></section>{/if}
 {:else if report.phase==='idle'}<p>Saved results appear after an acknowledged batch is available.</p>{/if}
</section>

<style>
 .cumulative-results{margin:1rem 0;padding:1rem;border:1px solid var(--line,#ccd8e4);border-radius:.75rem;background:var(--panel,#fff);min-width:0}.cumulative-results h3{margin-top:0}.controls,.timeframes{display:flex;flex-wrap:wrap;align-items:center;gap:.5rem}.controls{margin:.75rem 0}.controls label{display:flex;align-items:center;gap:.5rem;max-width:100%}button,select{font:inherit;padding:.5rem .7rem;max-width:100%}.active{outline:2px solid var(--accent,#147691);outline-offset:1px}.scope{padding:.75rem;border-left:3px solid var(--accent,#147691);background:var(--soft,#edf5f8)}.summary{display:flex;flex-wrap:wrap;gap:1rem;margin:1rem 0}.summary span{min-width:8rem}.summary b,small{display:block}small{font-size:.8rem;margin-top:.25rem;color:var(--muted,#5c6c80)}.table-scroll{max-width:100%;overflow:auto}table{border-collapse:collapse;width:100%;font-size:.9rem}caption{text-align:left;font-weight:600;padding:.5rem 0}th,td{padding:.65rem;border-bottom:1px solid var(--line,#ccd8e4);text-align:left;vertical-align:top;min-width:7rem}.empty{padding:1rem;background:var(--soft,#edf5f8)}details{margin-top:1rem}code{overflow-wrap:anywhere}.detail{margin-top:1rem}p{line-height:1.5}@media(max-width:480px){.cumulative-results{padding:.7rem}.controls label{align-items:flex-start;flex-direction:column}.controls select{width:100%}}
</style>
