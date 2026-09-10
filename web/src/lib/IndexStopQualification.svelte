<script>
 import {onDestroy} from 'svelte';
 import {ask} from './ask.js';
 import {createPageRequests} from './page-requests.js';
 import {createIndexStopQualificationReader,indexStopQualificationContext,indexStopQualificationInitialRow,fetchIndexStopFolds} from './index-stop-qualification.js';
 import {indexStopPoints} from './index-stop-results.js';
 import {indexConsistencyLabel,indexConsistencyReasons} from './index-consistency.js';
 import {catalogDay} from './boolean-catalog.js';
 import IndexConsistency from './IndexConsistency.svelte';
 import IndexStopResults from './IndexStopResults.svelte';
 let {initialIdentity='',initialCompletion='',initialOffset='0',initialSetting='',autoLoad=false}=$props();
 let input=$state(''),report=$state.raw(/** @type {any} */({phase:'idle',body:null,why:'',selection:null})),selected=$state.raw(/** @type {any} */(null)),tradePeriod=$state('later');
 let folds=$state.raw(/** @type {any} */({phase:'idle',body:null,why:'',offset:'0'}));
 let initialWhy=$state(''),expectedInitial=/** @type {any} */(null);
 const foldReads=createPageRequests(),reader=createIndexStopQualificationReader(value=>{report=value;if(value.phase==='ready'&&expectedInitial){try{choose(indexStopQualificationInitialRow(value.body,expectedInitial));}catch(why){initialWhy=why instanceof Error?why.message:String(why);}expectedInitial=null;}});
 $effect(()=>{input=initialIdentity;selected=null;initialWhy='';expectedInitial=initialSetting?{identity:initialIdentity,completion:initialCompletion,offset:initialOffset,setting:initialSetting}:null;foldReads.cancel();folds={phase:'idle',body:null,why:'',offset:'0'};reader.close();if(autoLoad&&initialIdentity)void reader.open({identity:initialIdentity,completion:initialCompletion||null,offset:initialOffset});});
 onDestroy(()=>{reader.dispose();foldReads.dispose();});
 /** @param {string} offset */function paginate(offset){selected=null;initialWhy='';expectedInitial=null;foldReads.cancel();folds={phase:'idle',body:null,why:'',offset:'0'};return reader.open({identity:report.body.identity,completion:report.body.completion,offset});}
 /** @param {any} row */function choose(row){selected=row;tradePeriod='later';foldReads.cancel();folds={phase:'idle',body:null,why:'',offset:'0'};}
 /** @param {string} [offset] */async function openFolds(offset='0'){
  foldReads.cancel();const body=structuredClone(report.body),row=structuredClone(selected);folds={phase:'loading',body:null,why:'',offset};
  await foldReads.run(async ticket=>{try{const page=await fetchIndexStopFolds(body,row,offset,url=>ask(url,{cache:'no-store',signal:ticket.signal}));if(ticket.current())folds={phase:'ready',body:page,why:'',offset};}catch(why){if(ticket.current())folds={phase:'failed',body:null,why:String(why),offset};}});
 }
 /** @param {string} value */const verdict=value=>({admitted:'Passed',rejected:'Failed',refused:'Refused',unmeasured:'Unmeasured'})[/** @type {'admitted'|'rejected'|'refused'|'unmeasured'} */(value)];
 /** @param {any} summary */const winningShare=summary=>{const eligible=BigInt(summary.eligible_days);if(!eligible)return 'Unavailable';const hundredths=BigInt(summary.winning_days)*10000n/eligible;return `${hundredths/100n}.${String(hundredths%100n).padStart(2,'0')}%`;};
</script>
<section class="index-stop-comparison" aria-label="Saved single-stop qualification comparison">
 <h3>Saved index comparisons</h3>
 <p>Compare original training with later results, then inspect the exact daily, weekly and trade evidence.
  These are cost-excluded research observations. Passing requires both the institutional checks and the index consistency rule.</p>
 <form onsubmit={event=>{event.preventDefault();selected=null;initialWhy='';expectedInitial=null;foldReads.cancel();void reader.open({identity:input.trim()});}}><label>Saved comparison identity <input bind:value={input} placeholder="Exact saved qualification ID" spellcheck="false" autocomplete="off"/></label><button>Read saved comparison</button></form>
 {#if initialWhy}<p role="alert">{initialWhy}</p>{/if}
 {#if report.phase==='loading'}<p role="status">Authenticating this comparison and reading one bounded settings page…</p>
 {:else if report.phase==='failed'}<p role="alert">This comparison is unavailable: {report.why}</p>{#if report.selection}<button onclick={()=>reader.open(report.selection)}>Retry this exact comparison</button>{/if}
 {:else if report.phase==='ready'}
  {@const body=report.body}
  <div class="summary"><span><b>Saved settings</b>{body.total}</span><span><b>Passed both checks</b>{body.summary.combined_qualified}</span><span><b>Institutional passed</b>{body.summary.institutional_admitted}</span><span><b>Rejected / refused / unmeasured</b>{body.summary.rejected} / {body.summary.refused} / {body.summary.unmeasured}</span></div>
  <p><b>Saved batch {body.batch}</b> (numbering starts at 0). All counts and ranks here belong to this one saved batch and timeframe.
   They do not establish the full search population or the strongest 0.1% of all possible setups.
   {#if body.summary.combined_qualified==='0'}<b>No setting in this saved comparison passed both requirements.</b>{/if}</p>
  <div class="scroll"><table><caption>Ordered by later pessimistic index points; optimistic is comparison only</caption><thead><tr><th>Rank / setting</th><th>Program / direction</th><th>Training trades</th><th>Training pessimistic / optimistic</th><th>Later trades</th><th>Later pessimistic / optimistic</th><th>Winning days / eligible</th><th>Full weeks passed / failed</th><th>Longest losing-day streak</th><th>Institutional</th><th>Index day/week rule</th><th>Combined</th><th>Inspect</th></tr></thead>
   <tbody>{#each body.rows as row (row.index)}{@const summary=row.index_consistency.periods.full.summary}<tr><th>{row.rank}<small>Setting {row.index}</small></th><td>{row.original.instrument}<small>{row.original.timeframe} · program {row.original.program_index} · {row.original.direction}</small></td><td>{row.original.metrics.trades}</td><td>{indexStopPoints(row.original.metrics.pessimistic_paisa)}<small>{indexStopPoints(row.original.metrics.optimistic_paisa)}</small></td><td>{row.later.metrics.trades}</td><td>{indexStopPoints(row.later.metrics.pessimistic_paisa)}<small>{indexStopPoints(row.later.metrics.optimistic_paisa)}</small></td><td>{winningShare(summary)}<small>{summary.winning_days} / {summary.eligible_days} days · full span</small></td><td>{summary.passing_weeks} / {summary.failing_weeks}<small>{summary.complete_weeks} complete five-session weeks</small></td><td>{summary.longest_losing_streak}</td><td>{verdict(row.institutional.status)}</td><td>{indexConsistencyLabel(row.index_consistency.state)}</td><td>{row.index_consistency.combined_qualifies?'Both passed':'Does not qualify'}<small>{indexConsistencyReasons(row.index_consistency).slice(0,2).join(' ')}</small></td><td><button onclick={()=>choose(row)}>Details and trades</button></td></tr>{/each}</tbody>
  </table></div>
  <div class="controls"><span>{body.rows.length} of {body.total} settings · page starts at {body.offset}</span><button disabled={body.offset==='0'} onclick={()=>paginate(String(BigInt(body.offset)>16n?BigInt(body.offset)-16n:0n))}>Previous settings</button><button disabled={body.next===null} onclick={()=>paginate(body.next)}>Next settings</button></div>
  {#if selected}
   <section class="selected"><h4>Setting {selected.index} · {selected.original.instrument} · {selected.original.timeframe} · {selected.original.direction}</h4>
    <p>Training: {catalogDay(selected.original.first_day)}–{catalogDay(selected.original.last_day)}. Later evaluation: {catalogDay(selected.later.first_day)}–{catalogDay(selected.later.last_day)}.</p>
    <IndexConsistency value={selected.index_consistency} context={indexStopQualificationContext(body,selected)} model="index-stop-qualification"/>
    <details><summary>All institutional checks and recorded evidence</summary><div class="scroll"><table><caption>Native check outcomes retained separately from the day/week rule</caption><thead><tr><th>Check</th><th>Outcome</th></tr></thead><tbody>{#each selected.institutional.checks as check (check.index)}<tr><th>{check.name.replaceAll('_',' ')}</th><td>{check.state}</td></tr>{/each}</tbody></table></div>
     <div class="scroll"><table><caption>Exact recorded measurements; missing values remain unavailable</caption><thead><tr><th>Measurement</th><th>State</th><th>Recorded value</th></tr></thead><tbody>{#each selected.institutional.values as value (value.name)}<tr><th>{value.name.replaceAll('_',' ')}</th><td>{value.state}</td><td>{value.value??'Unavailable'}</td></tr>{/each}</tbody></table></div>
     <div class="scroll"><table><caption>Acceptance thresholds saved with this comparison</caption><thead><tr><th>Rule</th><th>Required value</th></tr></thead><tbody>{#each body.policy.values as field (field.name)}<tr><th>{field.name.replaceAll('_',' ')}</th><td>{String(field.value)}</td></tr>{/each}</tbody></table></div>
    </details>
    <button onclick={()=>openFolds()}>Inspect later evaluation folds</button>
    {#if folds.phase==='loading'}<p role="status">Reading this setting’s saved fold outcomes…</p>{:else if folds.phase==='failed'}<p role="alert">{folds.why}</p><button onclick={()=>openFolds(folds.offset)}>Retry these folds</button>{:else if folds.phase==='ready'}
     <div class="scroll"><table><caption>Saved later-period fold outcomes</caption><thead><tr><th>Fold</th><th>Dates</th><th>Observed days</th><th>Trades / wins</th><th>Pessimistic index points</th><th>Refusals</th><th>Decided</th></tr></thead><tbody>{#each folds.body.rows as fold (fold.index)}<tr><th>{fold.index}</th><td>{catalogDay(fold.first_day)}–{catalogDay(fold.last_day)}</td><td>{fold.observed_days}</td><td>{fold.trades} / {fold.wins}</td><td>{indexStopPoints(fold.pessimistic_paisa)}</td><td>{fold.refused}</td><td>{fold.decided?'Yes':'No'}</td></tr>{/each}</tbody></table></div>
     <div class="controls"><button disabled={folds.body.offset==='0'} onclick={()=>openFolds(String(BigInt(folds.body.offset)>16n?BigInt(folds.body.offset)-16n:0n))}>Previous folds</button><button disabled={folds.body.next===null} onclick={()=>openFolds(folds.body.next)}>Next folds</button></div>
    {/if}
    <div class="controls" aria-label="Choose saved trade period"><button aria-pressed={tradePeriod==='training'} onclick={()=>{tradePeriod='training';}}>Original training trades</button><button aria-pressed={tradePeriod==='later'} onclick={()=>{tradePeriod='later';}}>Later trades</button></div>
    <IndexStopResults initialIdentity={tradePeriod==='training'?body.original.identity:body.later.identity} initialCompletion={tradePeriod==='training'?body.original.completion:body.later.completion} initialSelected={tradePeriod==='training'?selected.original:selected.later} autoLoad={true}/>
   </section>
  {/if}
  <details><summary>Exact saved identities</summary><p>Qualification: <code>{body.identity}</code></p><p>Completion: <code>{body.completion}</code></p><p>Search: <code>{body.search_identity}</code></p></details>
 {/if}
</section>
<style>
 .index-stop-comparison{border:1px solid var(--line,#cad6df);border-radius:8px;padding:1rem;margin:1rem 0;color:var(--n11,#23374c)}h3{font-size:1.1rem;margin:.2rem 0}h4{font-size:1rem}p{font-size:.82rem;line-height:1.6}form,label,.summary,.controls{display:flex;align-items:center;gap:.7rem;flex-wrap:wrap;margin:.8rem 0}label{flex:1;font-size:.82rem}input{flex:1;min-width:180px}input,button{font:inherit;font-size:.8rem;color:inherit;background:transparent;border:1px solid var(--line,#cad6df);border-radius:5px;padding:.55rem .7rem}button,summary{cursor:pointer}button:disabled{opacity:.4;cursor:default}button:focus-visible,summary:focus-visible,input:focus-visible{outline:2px solid var(--acc,#34849e);outline-offset:2px}.summary span{padding:.65rem;border:1px solid var(--line,#cad6df);border-radius:5px;font-size:.8rem}.summary b{display:block;margin-bottom:.3rem}.scroll{overflow:auto;max-width:100%}table{width:100%;border-collapse:collapse;text-align:left;font-size:.8rem;font-variant-numeric:tabular-nums}caption{text-align:left;font-weight:600;padding:.7rem 0}th,td{padding:.65rem;border-bottom:1px solid var(--line,#cad6df);white-space:nowrap;vertical-align:top}thead{background:var(--subtle,#f4f8fa)}small{display:block;font-size:.72rem;font-weight:400;margin-top:.25rem}.controls span{font-size:.8rem;margin-right:auto}.selected{border-left:3px solid var(--acc,#34849e);padding:.8rem;margin:1rem 0}details{font-size:.82rem;margin:.8rem 0}code{overflow-wrap:anywhere}[aria-pressed=true]{background:var(--acc-soft,#e6f1f5)}[role=alert]{color:var(--down,#ad4944)}@media(max-width:520px){.index-stop-comparison{padding:.6rem}.selected{padding:.5rem}}
</style>
