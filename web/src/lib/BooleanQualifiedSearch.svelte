<script>
 import {ask} from './ask.js';
 import {fetchQualifiedSearch} from './boolean-qualified-search.js';
 import {CAMPAIGN_RUNGS} from './boolean-campaign.js';
 import {createCampaignMonitor} from './campaign-monitor.js';
 import QualificationDetails from './QualificationDetails.svelte';
 import BooleanEvidence from './BooleanEvidence.svelte';
 import ResearchTester from './ResearchTester.svelte';
 let {initialIdentity='',refreshKey=0,autoLoad=false,vocabulary=/** @type {any} */ (null)}=$props();
 let input=$state(''),batch=$state('0'),rung=$state('0');
 let loaded=$state(/** @type {any} */ ({phase:'idle',body:null,why:''})),detail=$state(/** @type {any} */ ({phase:'idle',body:null,why:''})),original=$state(/** @type {any} */ (null));
 let generation=0,abort=/** @type {AbortController|null} */ (null);
 let testerSelection=$state(/** @type {any} */ (null));
 let jumpOffset=$state('0');
 const monitor=createCampaignMonitor((selection,signal)=>fetchQualifiedSearch(selection,url=>ask(url,{cache:'no-store',signal})),state=>{loaded=state;});
 $effect(()=>{if(loaded.phase==='ready'&&!loaded.body.rows.some((/** @type {any} */ row)=>row.rung_index===rung))rung=loaded.body.rows[0].rung_index;});
 $effect(()=>{void refreshKey;input=initialIdentity;monitor.stop();cancelDetail();loaded={phase:'idle',body:null,why:''};if(autoLoad&&initialIdentity)load({identity:initialIdentity});return()=>{monitor.stop();cancelDetail();};});
 function cancelDetail(){generation+=1;abort?.abort();abort=null;detail={phase:'idle',body:null,why:''};original=null;testerSelection=null;}
 /** @param {any} selection */
 function load(selection){cancelDetail();try{monitor.start(selection,selection.pin==null);}catch(why){loaded={phase:'failed',body:null,why:String(why)};}}
 /** @param {any} selection */
 async function open(selection){
  const ticket=++generation;abort?.abort();const controller=new AbortController();abort=controller;detail={phase:'loading',body:null,why:''};original=null;testerSelection=null;
  try{const body=await fetchQualifiedSearch(selection,url=>ask(url,{cache:'no-store',signal:controller.signal}));if(ticket===generation){detail={phase:'ready',body,why:''};jumpOffset=body.source.offset;}}
  catch(why){if(ticket===generation)detail={phase:'failed',body:null,why:why instanceof Error?why.message:String(why)};}
 }
 /** @param {any} body @param {string} offset */
 const page=(body,offset)=>({identity:body.identity,pin:body.pin,batch:body.batch,rung:body.rung,timeframes:body.timeframes,completion:body.completion,offset,limit:body.source.limit});
 /** @param {string} value */
 const label=value=>value.replaceAll('_',' ').replaceAll('-',' ');
</script>
<details class="search" open={autoLoad}>
 <summary>Follow the full declared search and compare every saved setting</summary>
 <p>Recorded grammar work, the selected timeframes, and the additional correction for testing successive batches. This is cost-excluded research, with no Selection V6 authority, live-trading approval or future-return guarantee.</p>
 <form onsubmit={event=>{event.preventDefault();load({identity:input.trim()});}}><label>Search ID <input bind:value={input} placeholder="64-character saved search identity" autocomplete="off" spellcheck="false" /></label><button>Read saved search</button></form>
 {#if loaded.phase==='loading'}<p role="status">Checking acknowledged search history within the configured byte and replay-node allowances…</p>
 {:else if loaded.phase==='failed'}<p role="alert"><b>Search unavailable.</b> {loaded.why}</p>
 {:else if loaded.phase==='ready'}
  {@const body=loaded.body}
  <p><b>Saved timeframes:</b> {body.timeframes.join(' · ')}. Each retains its original position in the eight-timeframe statistical allowance; unselected positions remain unspent.</p>
  <div class="facts"><b>{body.state==='completed'?'Declared grammar exhausted':body.state==='refused'?'Recorded refusal':body.phase==='reserved'?'Batch reserved':body.owner_active?'Recorded batch complete; owner observed':'Paused at an acknowledged batch'}</b><span>{body.grammar_work} grammar node choices</span><span>{body.programs} declared programs through this batch</span><span>{body.completed_batches} completed batches</span></div>
  {#if body.projection_version===1}<p role="status"><b>Historical calculation.</b> Version 1 retained individual probability limits. These results have not passed the newer shared-limit comparison.</p>{/if}
  <p>Current batch {body.batch} (zero-based), checkpoint {body.sequence}. {body.owner_active?'An owner lock was observed; this is not a heartbeat.':'No owner lock was observed in this snapshot; process liveness is not inferred.'} {loaded.watching?'Following new snapshots automatically, one request at a time.':'Automatic watching is stopped for this saved state.'}</p>
  {#if body.reason}<p role="alert">Recorded reason: {body.reason}</p>{/if}
  {#if body.planned_campaign}<p><a href={'/backtest?boolean_qualified_campaign='+body.planned_campaign}>View this batch’s saved timeframe progress</a>. This identity comes from the declared plan; the link does not prove the batch started or completed. An absent progress journal stays unavailable.</p>{/if}
  {#if body.node_only}<p><b>Grammar work only in this batch.</b> No programs were emitted, so no qualification children or passing outcomes are invented.</p>{/if}
  {#if loaded.why}<p role="alert">Refresh failed: {loaded.why}. The table retains the previous acknowledged snapshot.</p>{/if}
  <button onclick={()=>load({identity:body.identity})}>Refresh latest</button><button onclick={()=>load({identity:body.identity,pin:body.pin})}>Recheck exact checkpoint</button>
  <div class="scroll"><table><caption>Latest batch: existing institutional checks by declared timeframe</caption><thead><tr><th>Timeframe</th><th>Retained settings</th><th>Admitted</th><th>Rejected</th><th>Unmeasured</th><th>Refused</th><th>Detail</th></tr></thead><tbody>{#each body.rows as row (row.rung)}<tr><th>{row.rung}</th><td>{row.count}</td><td>{row.counts.admitted}</td><td>{row.counts.rejected}</td><td>{row.counts.unmeasured}</td><td>{row.counts.refused}</td><td>{#if row.qualification}<button onclick={()=>open({identity:body.identity,pin:body.pin,batch:body.batch,rung:row.rung_index,timeframes:body.timeframes,completion:row.qualification.completion})}>Every check and later fold</button>{:else}No completed projection{/if}</td></tr>{/each}</tbody></table></div>
  <p>These counts retain the existing institutional verdict. Open a setting to see its separate index day/week assessment and combined result; older results remain unassessed under the new rule.</p>
  <form onsubmit={event=>{event.preventDefault();void open({identity:body.identity,pin:body.pin,batch:batch.trim(),rung,timeframes:body.timeframes});}}><label>Earlier batch <input bind:value={batch} inputmode="numeric" /></label><label>Timeframe <select bind:value={rung}>{#each body.rows as row (row.rung_index)}<option value={row.rung_index}>{row.rung}</option>{/each}</select></label><button>Open exact recorded batch</button></form>
  <p>Reserved and refused slots keep their error allocation. A completed node-only batch has no candidate detail. No estimated total or completion percentage is invented. Opening a detail authenticates its full saved child evidence; the overview alone does not authenticate those bodies or reread current raw-market files.</p>
  <details><summary>Exact snapshot and resource limits</summary><p>Serialized admission {body.admitted_bytes} of {body.observation_byte_limit} bytes; grammar replay allowances charged {body.replay_nodes_charged} of {body.replay_node_limit} nodes. These are bounds for this request, not total memory or elapsed time.</p><pre>{JSON.stringify(body,null,2)}</pre></details>
 {/if}
 {#if detail.phase==='loading'}<p role="status">Authenticating the selected batch, child and full comparison…</p>
 {:else if detail.phase==='failed'}<p role="alert"><b>Detail unavailable.</b> {detail.why} No partial or replacement page is displayed.</p>
 {:else if detail.phase==='ready'}
  {@const d=detail.body}{@const s=d.source}{@const a=d.allocation}
  <section><h3>Batch {d.batch} · {CAMPAIGN_RUNGS[Number(d.rung)]} · search-wide comparison</h3>
   <p>This detail stays on its opened parent and child pins while overview watching can advance. Exact threshold {a.threshold.numerator} / {a.threshold.denominator}; original alpha {a.alpha_ppm} ppm. Rule: {a.rule}.</p>
   <p>{s.qualification.procedure.draws} later bootstrap draws. {a.minimum_draws===null?'Zero alpha is unreachable by a positive bootstrap probability':`Minimum ${a.minimum_draws} draws for this slot`}. <b>{a.draw_resolution_met?'Numerically reachable; this does not establish a passing strategy':'Not enough bootstrap draws to meet this slot’s probability limit'}</b>.</p>
   <button onclick={()=>{original={identity:s.identity,completion:s.completion};}}>Open the unmodified original qualification</button>
   <details><summary>All 39 saved common policy settings</summary><p>{d.projection_version>=2?'The shared search allowance can tighten the four probability limits. Every other setting stays unchanged.':'Historical version 1 keeps every original limit. Its White and SPA checks do not apply the newer shared ceiling.'} Original qualification remains available separately.</p><table><thead><tr><th>Setting and unit</th><th>Original value</th><th>Effective search value</th></tr></thead><tbody>{#each s.policy.values as row,index (row.name)}<tr><th>{label(row.name)}</th><td>{String(row.value)}</td><td>{String(d.search_policy.values[index].value)}</td></tr>{/each}</tbody></table></details>
   {#each d.rows as projected,index (projected.comparison.index)}
    {@const row=s.rows[index]}{@const c=projected.comparison}
    <details class="setting"><summary>Setting {row.index} · {row.statistics.source.instrument} · {row.statistics.side} · {label(c.status)}</summary>
     <p>Original qualification: <b>{label(row.status)}</b>. Additional search-wide comparison: <b>{label(c.status)}</b>. Missing or refused evidence is retained.</p>
     <button onclick={()=>{testerSelection={detail:d,index:row.index};}}>Inspect in strategy tester</button>
     <table><caption>Exact search-corrected later probabilities</caption><tbody>{#each ['romano','white','spa'] as key}<tr><th>{label(key)}</th><td>{projected.probabilities[key].numerator} / {projected.probabilities[key].denominator}</td></tr>{/each}</tbody></table>
     <QualificationDetails body={s} {row} consistency={c.index_consistency} institutionalStatus={c.status} />
     <div class="scroll"><table><caption>All 44 search-wide policy checks</caption><thead><tr><th>Check</th><th>Outcome</th></tr></thead><tbody>{#each c.checks as check (check.index)}<tr><th>{label(check.name)}</th><td>{label(check.state)}</td></tr>{/each}</tbody></table></div>
     <div class="scroll"><table><caption>All 44 exact projected evidence values</caption><thead><tr><th>Evidence and unit</th><th>Availability</th><th>Value</th></tr></thead><tbody>{#each c.values as value (value.name)}<tr><th>{label(value.name)}</th><td>{label(value.state)}</td><td>{value.value??'Unavailable'}</td></tr>{/each}</tbody></table></div>
    </details>
   {/each}
   <div class="facts"><span>{d.rows.length} of {s.total} settings; page starts at {s.offset}</span><button disabled={s.offset==='0'} onclick={()=>open(page(d,String(BigInt(s.offset)>BigInt(s.limit)?BigInt(s.offset)-BigInt(s.limit):0n)))}>Previous</button><button disabled={s.next===null} onclick={()=>open(page(d,s.next))}>Next</button></div>
   <form onsubmit={event=>{event.preventDefault();void open(page(d,jumpOffset.trim()));}}><label>Start at setting <input bind:value={jumpOffset} inputmode="numeric" /></label><button>Open setting page</button></form>
   {#if original}<BooleanEvidence initialIdentity={original.identity} initialCompletion={original.completion} initialModel="qualification" autoLoad={true} />{/if}
  </section>
 {/if}
</details>
<ResearchTester selection={testerSelection} {vocabulary} />
<style>
 .search{border:1px solid #566d7955;border-radius:10px;padding:1rem;margin:1rem 0}summary{font-weight:650;cursor:pointer}p{font-size:.85rem;line-height:1.55}form,.facts{display:flex;gap:.7rem;align-items:center;flex-wrap:wrap;margin:1rem 0}label{display:flex;align-items:center;gap:.5rem}input{min-width:180px;flex:1}input,button,select{background:transparent;color:inherit;border:1px solid #647985;border-radius:5px;padding:.55rem}button{cursor:pointer}button:disabled{opacity:.4}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid #79cad8}.scroll{overflow:auto}table{width:100%;border-collapse:collapse;font-size:.82rem;text-align:left}th,td{padding:.6rem;border-bottom:1px solid #61717e44;vertical-align:top}caption{text-align:left;font-weight:650;margin:.5rem 0}pre{max-height:30rem;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font-size:.75rem}.setting{border-left:3px solid #6bb8c4;padding:.7rem 1rem;margin:.8rem 0}section{margin-top:1.5rem}h3{font-size:1rem}
</style>
